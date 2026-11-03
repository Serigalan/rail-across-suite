// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The optimizer on the test tracks of the service's verify.py, against what
//! the Python service found for them (tests/data/optimize_python.json,
//! recorded 2026-11-03). The existing speed and the per-curve baseline are
//! exact searches and must come out the same; the joint search draws its
//! numbers from another generator than scipy's, so of its result the test
//! holds what has to hold of any: never below the baseline, ends fixed, the
//! chain closed, inside the corridor, every rule of the level kept.

mod common;

use rail_across_optimizer::element::Element;
use rail_across_optimizer::geometry::{Pt, max_dist_to_polyline};
use rail_across_optimizer::optimize::optimize_payload;
use rail_across_optimizer::pruefung::{Context, check_track};
use rail_across_optimizer::rules::Rules;
use rail_across_optimizer::track::element_ref_points;
use serde_json::Value;
use std::time::Instant;

fn pt(v: &Value) -> Pt {
    (v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

/// The elements a run proposed: not an element of the old chain passed through.
fn new_indices(old: &[Value], new: &[Value]) -> Vec<usize> {
    let key = |el: &Value| {
        let (s, e) = (pt(&el["startNode"]), pt(&el["endNode"]));
        format!(
            "{} {} {} {} {} {} {}",
            (s.0 / 1e-6).round(),
            (s.1 / 1e-6).round(),
            (e.0 / 1e-6).round(),
            (e.1 / 1e-6).round(),
            el["elementType"],
            el.get("radius").unwrap_or(&Value::Null),
            el.get("speed").unwrap_or(&Value::Null)
        )
    };
    let old: std::collections::HashSet<String> = old.iter().map(key).collect();
    (0..new.len()).filter(|&i| !old.contains(&key(&new[i]))).collect()
}

fn polyline(els: &[Value]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for el in els {
        let part = element_ref_points(el);
        let skip = usize::from(!out.is_empty());
        out.extend(part.into_iter().skip(skip));
    }
    out
}

#[test]
fn the_optimizer_on_the_services_test_tracks() {
    let data: Value = serde_json::from_str(include_str!("data/optimize_python.json")).unwrap();
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let mut failures = Vec::new();
    for run in data["runs"].as_array().unwrap() {
        let mut request = run["request"].clone();
        let name = request["track"].as_str().unwrap().to_string();
        let track = data["tracks"][&name].clone();
        request["track"] = track.clone();
        let label = format!("{name} {}", run["request"]);
        let t0 = Instant::now();
        let got = optimize_payload(&rules, &request).unwrap();
        let took = t0.elapsed().as_secs_f64();
        let want = &run["python"];
        let num = |v: &Value, k: &str| v[k].as_f64().unwrap();
        println!(
            "{label}: Bestand {:.2} | baseline {:.2} (py {:.2}) | neu {:.2} (py {:.2}) | {took:.1} s (py {} s)",
            num(&got, "vBestand"),
            num(&got, "vBaseline"),
            num(want, "vBaseline"),
            num(&got, "vNeu"),
            num(want, "vNeu"),
            run["seconds"]
        );
        let mut fail = |why: String| failures.push(format!("{label}: {why}"));
        for key in ["vBestand", "vBaseline"] {
            if (num(&got, key) - num(want, key)).abs() > 1e-6 {
                fail(format!("{key} {} want {}", got[key], want[key]));
            }
        }
        if got["variant"] != want["variant"] {
            fail(format!("variant {} want {}", got["variant"], want["variant"]));
        }
        if got["skipped"] != want["skipped"] {
            fail(format!("skipped {} want {}", got["skipped"], want["skipped"]));
        }
        if num(&got, "vNeu") < num(&got, "vBaseline") - 1e-9 {
            fail("below its baseline".into());
        }
        // Within a few km/h of what scipy's search found: another generator, the same search.
        if num(&got, "vNeu") < num(want, "vNeu") - 3.0 {
            fail(format!("vNeu {:.2} far below Python's {:.2}", num(&got, "vNeu"), num(want, "vNeu")));
        }
        let old = track["elements"].as_array().unwrap();
        let new = got["elements"].as_array().unwrap();
        if new.len() != want["elements"].as_u64().unwrap() as usize {
            fail(format!("{} elements want {}", new.len(), want["elements"]));
        }
        // Ends fixed, the chain closed and tangent.
        if (pt(&new[0]["startNode"]).0 - pt(&old[0]["startNode"]).0).abs() > 1e-6
            || (pt(&new[new.len() - 1]["endNode"]).1 - pt(&old[old.len() - 1]["endNode"]).1).abs() > 1e-6
        {
            fail("ends moved".into());
        }
        for w in new.windows(2) {
            let (a, b) = (pt(&w[0]["endNode"]), pt(&w[1]["startNode"]));
            let kink =
                ((w[0].get("endBearing").unwrap_or(&w[0]["bearing"]).as_f64().unwrap() - w[1]["bearing"].as_f64().unwrap() + 540.0) % 360.0 - 180.0).abs();
            if (a.0 - b.0).hypot(a.1 - b.1) > 1e-6 || kink > 1e-6 {
                fail(format!("chain open or kinked: gap {:.2e}, kink {kink:.2e}°", (a.0 - b.0).hypot(a.1 - b.1)));
                break;
            }
        }
        // Inside the corridor, measured independently.
        let corridor = request["corridorCm"].as_f64().unwrap() / 100.0;
        let offset = max_dist_to_polyline(&polyline(new), &polyline(old));
        if offset > corridor + 0.01 {
            fail(format!("offset {offset:.3} m beyond the corridor"));
        }
        // Every rule kept at the level asked for, by what the run proposed.
        let level = request["grenzwert"].as_str().unwrap();
        let grenzen = if level == "reg" { &rules.reg } else { &rules.discretion };
        let els: Vec<Element> = new.iter().map(|e| serde_json::from_value(e.clone()).unwrap()).collect();
        let idx = new_indices(old, new);
        let bad: Vec<_> = check_track(&katalog, &els, Some(&idx), Context::default())
            .unwrap()
            .into_iter()
            .filter(|f| katalog.rank(&f.severity) > grenzen.max_rank && !grenzen.toleriert.contains(&f.rule))
            .collect();
        if !bad.is_empty() {
            fail(format!("findings above the level: {bad:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn what_a_run_cannot_take_is_said() {
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let straight = |a: [f64; 2], b: [f64; 2]| serde_json::json!({"elementType": 0, "startNode": a, "endNode": b, "length": 1.0});
    let arc = |r: f64| serde_json::json!({"elementType": 1, "radius": r, "startNode": [1, 1], "endNode": [2, 2], "length": 1.0});
    let transition = serde_json::json!({"elementType": 2, "startNode": [1, 1], "endNode": [2, 2], "length": 1.0});
    for els in [
        vec![straight([0.0, 0.0], [1.0, 1.0]), transition, straight([2.0, 2.0], [3.0, 3.0])],
        vec![
            straight([0.0, 0.0], [1.0, 1.0]),
            arc(700.0),
            arc(-700.0),
            arc(700.0),
            straight([2.0, 2.0], [3.0, 3.0]),
        ],
    ] {
        let got = optimize_payload(&rules, &serde_json::json!({"track": {"elements": els, "epsg": 25832}})).unwrap();
        assert_eq!(got["error"], "unsupported_topology", "{got}");
        assert!(got["message"].as_str().unwrap().contains("Keine optimierbaren Bögen"), "{got}");
    }
    let got = optimize_payload(&rules, &serde_json::json!({"track": {"elements": [], "epsg": 25832}, "uebergang": "bloss"})).unwrap();
    assert!(got["message"].as_str().unwrap().contains("LP.UB.02"), "{got}");
}
