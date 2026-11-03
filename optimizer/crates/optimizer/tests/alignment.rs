// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The alignment fit held to what the Python service answered for the same
//! points: the round trips of its verify_align.py (tests/data/
//! align_python.json, recorded 2026-11-03) and the answer the app's own tests
//! read (packages/core/test/fixtures/align_answer.json).

mod common;

use rail_across_optimizer::alignment::align_payload;
use serde_json::Value;
use std::time::Instant;

fn close(a: &Value, b: &Value, tol: f64) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() <= tol,
        _ => a == b,
    }
}

fn compare(got: &Value, want: &Value) -> Vec<String> {
    let mut why = Vec::new();
    for key in ["error", "errorParams", "merged"] {
        if got.get(key).unwrap_or(&Value::Null) != want.get(key).unwrap_or(&Value::Null) {
            why.push(format!(
                "{key} {} want {}",
                got.get(key).unwrap_or(&Value::Null),
                want.get(key).unwrap_or(&Value::Null)
            ));
        }
    }
    let straights = |v: &Value| v["straights"].as_array().map(|a| a.len()).unwrap_or(0);
    if straights(got) != straights(want)
        || got["straights"]
            .as_array()
            .unwrap()
            .iter()
            .zip(want["straights"].as_array().unwrap())
            .any(|(g, w)| !close(&g["from"], &w["from"], 0.01) || !close(&g["to"], &w["to"], 0.01))
    {
        why.push(format!("straights {} want {}", got["straights"], want["straights"]));
    }
    let (gc, wc) = (got["curves"].as_array().unwrap(), want["curves"].as_array().unwrap());
    if gc.len() != wc.len() {
        why.push(format!("{} curves want {}", gc.len(), wc.len()));
    } else {
        for (g, w) in gc.iter().zip(wc) {
            let r = w["radius"].as_f64().unwrap().abs();
            if g["kind"] != w["kind"]
                || g["notes"] != w["notes"]
                || !close(&g["radius"], &w["radius"], (r * 2e-3).max(0.5))
                || !close(&g["l1"], &w["l1"], 0.5)
                || !close(&g["l2"], &w["l2"], 0.5)
            {
                why.push(format!(
                    "curve {} {} {} {} {} want {} {} {} {} {}",
                    g["kind"], g["radius"], g["l1"], g["l2"], g["notes"], w["kind"], w["radius"], w["l1"], w["l2"], w["notes"]
                ));
            }
        }
    }
    if want["error"].is_null() {
        if !close(&got["rms"], &want["rms"], 0.0005) || !close(&got["max"], &want["max"], 0.001) {
            why.push(format!("rms/max {}/{} want {}/{}", got["rms"], got["max"], want["rms"], want["max"]));
        }
        let elements = got["elements"].as_array().map(Vec::len);
        if elements != want["elements"].as_u64().map(|n| n as usize) {
            why.push(format!("{elements:?} elements want {}", want["elements"]));
        }
        if got["elementCants"] != want["elementCants"] {
            why.push(format!("cants {} want {}", got["elementCants"], want["elementCants"]));
        }
        let heights = |v: &Value| v.as_array().cloned().unwrap_or_default();
        let (gh, wh) = (heights(&got["gradient"]["heights"]), heights(&want["gradient"]));
        if gh.len() != wh.len()
            || gh
                .iter()
                .zip(&wh)
                .any(|(g, w)| !close(&g["station"], &w["station"], 0.5) || !close(&g["z"], &w["z"], 0.002) || g.get("rv") != w.get("rv"))
        {
            why.push(format!("gradient {} want {}", got["gradient"]["heights"], want["gradient"]));
        }
    }
    why
}

#[test]
fn the_fit_finds_what_the_service_found() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("data/align_python.json")).unwrap();
    let mut failures = Vec::new();
    for (k, case) in cases.iter().enumerate() {
        let t0 = Instant::now();
        let got = align_payload(&case["request"]).unwrap();
        let curves: Vec<String> = got["curves"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| format!("{} R {} L {}/{}", c["kind"], c["radius"], c["l1"], c["l2"]))
            .collect();
        println!(
            "#{k}: {} points, {:.2} s, {:?} {:?}",
            case["request"]["points"].as_array().unwrap().len(),
            t0.elapsed().as_secs_f64(),
            got.get("error"),
            curves
        );
        let why = compare(&got, &case["python"]);
        if !why.is_empty() {
            failures.push(format!("#{k}: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_apps_fixture() {
    let fixture = common::constraints("../test/fixtures/align_answer.json");
    let got = align_payload(&fixture["request"]).unwrap();
    let want = &fixture["answer"];
    let mut summary = want.clone();
    summary["elements"] = Value::from(want["elements"].as_array().unwrap().len());
    summary["gradient"] = want["gradient"]["heights"].clone();
    let why = compare(&got, &summary);
    assert!(why.is_empty(), "{}", why.join("\n"));
    // Every element where the service had it, to the millimetre.
    for (g, w) in got["elements"].as_array().unwrap().iter().zip(want["elements"].as_array().unwrap()) {
        for node in ["startNode", "endNode"] {
            let d = (g[node][0].as_f64().unwrap() - w[node][0].as_f64().unwrap()).hypot(g[node][1].as_f64().unwrap() - w[node][1].as_f64().unwrap());
            assert!(d < 0.005, "{node} {d}");
        }
    }
}
