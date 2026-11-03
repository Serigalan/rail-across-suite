// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reconnecting existing elements, held to what the Python service found for
//! the same round trips (tests/data/reconnect_python.json, the cases of its
//! verify_reconnect.py, recorded 2026-11-03): an ideal chain built forwards,
//! its axis taken every centimetre as the app takes it, the elements between
//! two neighbours handed over as the stretch to replace.

mod common;

use rail_across_optimizer::geometry::{Profile, Pt, arc_forward, transition_end};
use rail_across_optimizer::reconnect::{chain_polyline, reconnect_payload};
use rail_across_optimizer::rules::Rules;
use rail_across_optimizer::splice::{El, Role, arc, straight, transition};
use serde_json::{Value, json};

const P0: Pt = (500000.0, 5600000.0);

enum S {
    Straight(f64),
    Arc(f64, f64),
    Transition(f64, Option<f64>, Option<f64>),
}

/// An ideal chain from P0 heading east.
fn build(specs: &[S], profile: Profile) -> Vec<El> {
    let (mut p, mut b) = (P0, 90.0_f64);
    let mut els = Vec::new();
    for s in specs {
        match *s {
            S::Straight(l) => {
                let q = (p.0 + l * b.to_radians().sin(), p.1 + l * b.to_radians().cos());
                els.push(straight(p, q, Role::New));
                p = q;
            }
            S::Arc(l, r) => {
                let (q, nb) = arc_forward(p.0, p.1, b, r, l / r.abs());
                els.push(arc(p, q, r, Role::New).unwrap());
                (p, b) = (q, nb);
            }
            S::Transition(l, r1, r2) => {
                let (e, n, nb) = transition_end(p.0, p.1, b, l, r1, r2, profile);
                els.push(transition(p, b, l, r1, r2, profile, Some((e, n)), Some(nb)));
                (p, b) = ((e, n), nb);
            }
        }
    }
    els
}

/// An element as the app hands it over — or, `virtual_`, the point it ends in.
fn as_pick(el: &El, join_at: &str, virtual_: bool) -> Value {
    if virtual_ {
        let at = if join_at == "end" { el.end } else { el.start };
        let r = if el.kind == 2 {
            if join_at == "end" { el.r2 } else { el.r1 }
        } else {
            el.radius
        };
        let b = if join_at == "end" { el.end_bearing() } else { el.bearing };
        return json!({"start": [at.0, at.1], "end": [at.0, at.1], "bearing": b, "radius": r, "length": 0, "joinAt": join_at});
    }
    json!({"start": [el.start.0, el.start.1], "end": [el.end.0, el.end.1], "bearing": el.end_bearing(), "radius": el.radius, "length": el.length, "joinAt": join_at})
}

/// The axis every centimetre, packed as the app sends it.
fn points_of(els: &[El]) -> Value {
    let poly = chain_polyline(els);
    let mut st = vec![0.0];
    for w in poly.windows(2) {
        st.push(st.last().unwrap() + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
    }
    let total = *st.last().unwrap();
    let mut s: Vec<f64> = (0..).map(|i| f64::from(i) * 0.01).take_while(|s| *s < total).collect();
    s.push(total);
    let mut j = 0;
    let pts: Vec<Pt> = s
        .iter()
        .map(|&s| {
            while j + 2 < st.len() && st[j + 1] < s {
                j += 1;
            }
            let t = if st[j + 1] > st[j] {
                ((s - st[j]) / (st[j + 1] - st[j])).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (poly[j].0 + t * (poly[j + 1].0 - poly[j].0), poly[j].1 + t * (poly[j + 1].1 - poly[j].1))
        })
        .collect();
    let (e0, n0) = pts[0];
    json!({
        "e0": e0, "n0": n0,
        "de": pts.iter().map(|p| ((p.0 - e0) * 1000.0).round_ties_even() as i64).collect::<Vec<_>>(),
        "dn": pts.iter().map(|p| ((p.1 - n0) * 1000.0).round_ties_even() as i64).collect::<Vec<_>>(),
    })
}

#[allow(clippy::too_many_arguments)]
fn ask(rules: &Rules, els: &[El], dep: usize, arr: usize, tol: f64, speed: f64, virtual_: (bool, bool), extra: Value) -> Value {
    let lo = dep + usize::from(virtual_.0);
    let hi = arr - usize::from(virtual_.1);
    let mut req = json!({
        "dep": as_pick(&els[dep], "end", virtual_.0), "arr": as_pick(&els[arr], "start", virtual_.1),
        "points": points_of(&els[lo..=hi]), "tolerance": tol, "speed": speed,
        "cantModel": {"coeff": 6.5, "defCoeff": 11.8, "defMin": 60, "max": 160, "step": 5},
        "lMin": 0.4 * speed,
    });
    for (k, v) in extra.as_object().unwrap() {
        req[k] = v.clone();
    }
    reconnect_payload(rules, &req).unwrap()
}

#[test]
fn reconnecting_finds_what_the_service_found() {
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let c = Profile::Clothoid;
    let curve = build(
        &[
            S::Straight(200.0),
            S::Transition(60.0, None, Some(800.0)),
            S::Arc(300.0, 800.0),
            S::Transition(60.0, Some(800.0), None),
            S::Straight(200.0),
        ],
        c,
    );
    let long_bloss = build(
        &[
            S::Straight(435.0),
            S::Transition(188.0, None, Some(-1890.0)),
            S::Arc(659.0, -1890.0),
            S::Transition(188.0, Some(-1890.0), None),
            S::Straight(980.0),
        ],
        Profile::Bloss,
    );
    let plain = build(&[S::Straight(150.0), S::Arc(200.0, -1900.0), S::Straight(150.0)], c);
    let arcs = build(
        &[
            S::Arc(150.0, 600.0),
            S::Transition(40.0, Some(600.0), None),
            S::Straight(100.0),
            S::Transition(40.0, None, Some(900.0)),
            S::Arc(150.0, 900.0),
        ],
        c,
    );
    let s_curve = build(&[S::Straight(100.0), S::Arc(120.0, 500.0), S::Arc(120.0, -500.0), S::Straight(100.0)], c);
    let no = (false, false);
    let got: Vec<(&str, Value)> = vec![
        ("curve", ask(&rules, &curve, 0, 4, 0.10, 100.0, no, json!({}))),
        ("tight", ask(&rules, &curve, 0, 4, 0.0005, 160.0, no, json!({}))),
        ("fixed radius", ask(&rules, &curve, 0, 4, 0.50, 100.0, no, json!({"radius": 900}))),
        ("long bloss", ask(&rules, &long_bloss, 0, 4, 0.05, 160.0, no, json!({"transition": "bloss"}))),
        ("plain arc", ask(&rules, &plain, 0, 2, 0.05, 60.0, no, json!({}))),
        ("fixed neighbours", ask(&rules, &curve, 0, 4, 0.15, 100.0, (true, true), json!({}))),
        ("one circle", ask(&rules, &curve, 1, 3, 0.10, 100.0, (true, true), json!({}))),
        ("arc and straight", ask(&rules, &arcs, 0, 2, 0.10, 80.0, no, json!({}))),
        ("two arcs", ask(&rules, &arcs, 0, 4, 0.10, 80.0, no, json!({}))),
        ("reverse curve", ask(&rules, &s_curve, 0, 3, 0.05, 60.0, no, json!({}))),
    ];
    let want: Vec<Value> = serde_json::from_str(include_str!("data/reconnect_python.json")).unwrap();
    let mut failures = Vec::new();
    for ((name, got), want) in got.iter().zip(&want) {
        assert_eq!(*name, want["name"]);
        let want = &want["answer"];
        if want.get("error").is_some() {
            if got.get("error") != want.get("error") {
                failures.push(format!("{name}: {:?} want {}", got.get("error"), want["error"]));
            }
            continue;
        }
        let sols = got["solutions"].as_array().unwrap();
        let summary = |s: &Value| {
            let r = &s["reconnect"];
            format!(
                "{} R={} u={} within={} max={} worst={}",
                r["variant"], r["radius"], r["cant"], r["within"], r["max"], s["worst"]
            )
        };
        let wants = want.as_array().unwrap();
        let ok = sols.len() == wants.len()
            && sols.iter().zip(wants).all(|(g, w)| {
                let r = &g["reconnect"];
                r["variant"]["lengths"] == w["variant"]["lengths"]
                    && r["variant"]["arcJoin"] == w["variant"]["arcJoin"]
                    && match (r["radius"].as_f64(), w["radius"].as_f64()) {
                        (Some(a), Some(b)) => (a - b).abs() <= 1.0,
                        (None, None) => true,
                        _ => false,
                    }
                    && r["within"] == w["within"]
                    && (r["max"].as_f64().unwrap() - w["max"].as_f64().unwrap()).abs() <= 0.002
                    && g["worst"] == w["worst"]
            });
        if !ok {
            failures.push(format!(
                "{name}:\n  got  {}\n  want {}",
                sols.iter().map(summary).collect::<Vec<_>>().join("\n       "),
                wants
                    .iter()
                    .map(|w| format!(
                        "{} R={} u={} within={} max={} worst={}",
                        w["variant"], w["radius"], w["cant"], w["within"], w["max"], w["worst"]
                    ))
                    .collect::<Vec<_>>()
                    .join("\n       ")
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
