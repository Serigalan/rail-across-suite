// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The splice held against what the optimizer service answered for the very
//! same requests — random geometries asked in every way a user can ask them,
//! with and without transitions, at the Regellänge (tests/data/
//! splice_answers.json, written by the service's splice_answers_fixture.py;
//! the app's solveSplice.js was held to the same answers before this
//! replaced it).

mod common;

use rail_across_optimizer::rules::Rules;
use rail_across_optimizer::splice::splice_payload;
use serde_json::Value;

const NODE_TOL: f64 = 1e-6;

fn close(got: &Value, want: &Value, tol: f64) -> bool {
    match (got.as_f64(), want.as_f64()) {
        (_, None) if want.is_null() => got.is_null(),
        (Some(g), Some(w)) => (g - w).abs() <= tol,
        _ => false,
    }
}

fn same_sides(got: &Value, want: &Value, why: &mut Vec<String>) {
    let (g, w) = (got.as_array().unwrap(), want.as_array().unwrap());
    if g.len() != w.len() {
        why.push("sides".into());
        return;
    }
    for (g, w) in g.iter().zip(w) {
        if g["mode"] != w["mode"] {
            why.push(format!("mode {} {}", g["mode"], w["mode"]));
        }
        for key in ["length", "regular", "minimum"] {
            let gv = g.get(key).cloned().unwrap_or(Value::Null);
            let wv = w.get(key).cloned().unwrap_or(Value::Null);
            if !close(&gv, &wv, 1e-6) {
                why.push(format!("{key} {gv} {wv}"));
            }
        }
    }
}

fn same_answer(got: &Value, want: &Value) -> Vec<String> {
    let mut why = Vec::new();
    if let Some(error) = want.get("error") {
        if got.get("error") != Some(error) {
            return vec![format!("error {:?} want {error}", got.get("error"))];
        }
        for (key, tol) in [("rMax", 1.0), ("rMin", 1.0), ("lMax", 0.1)] {
            let w = &want["params"][key];
            let g = got["params"].get(key);
            let ok = if w.is_null() {
                g.is_none()
            } else {
                g.and_then(Value::as_f64).is_some_and(|g| (g - w.as_f64().unwrap()).abs() <= tol + 1e-9)
            };
            if !ok {
                why.push(format!("{key} {g:?} want {w}"));
            }
        }
        if let Some(lengths) = want.get("lengths") {
            same_sides(&got["lengths"], lengths, &mut why);
        }
        return why;
    }
    if got.get("error").is_some() {
        return vec![format!("error {}", got["error"])];
    }
    let shape = |a: &Value| -> Vec<String> {
        a["solutions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| format!("{} {} {}", s["ends"], s["alternative"], s.get("arcJoin").unwrap_or(&Value::Null)))
            .collect()
    };
    if shape(got) != shape(want) {
        return vec![format!("solutions {:?} want {:?}", shape(got), shape(want))];
    }
    match (want.get("requested"), got.get("requested")) {
        (Some(w), Some(g)) if w["error"] == g["error"] => {}
        (None, None) => {}
        (w, g) => why.push(format!("requested {g:?} want {w:?}")),
    }
    for (g, w) in got["solutions"].as_array().unwrap().iter().zip(want["solutions"].as_array().unwrap()) {
        for key in ["depPick", "reverseDep", "reverseArr", "worst", "judged"] {
            if g[key] != w[key] {
                why.push(format!("{key} {} want {}", g[key], w[key]));
            }
        }
        if !close(&g["rebuilt"], &w["rebuilt"], 1e-4) {
            why.push(format!("rebuilt {} want {}", g["rebuilt"], w["rebuilt"]));
        }
        let findings: Vec<Value> = g["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| serde_json::json!([f["at"], f["id"], f["severity"]]))
            .collect();
        if Value::Array(findings.clone()) != w["findings"] {
            why.push(format!("findings {findings:?} want {}", w["findings"]));
        }
        if let Some(lengths) = w.get("lengths") {
            same_sides(&g["lengths"], lengths, &mut why);
        }
        let (ge, we) = (g["elements"].as_array().unwrap(), w["elements"].as_array().unwrap());
        let kinds = |els: &[Value]| els.iter().map(|e| format!("{}{}", e["elementType"], e["role"])).collect::<Vec<_>>();
        if kinds(ge) != kinds(we) {
            why.push(format!("elements {:?} want {:?}", kinds(ge), kinds(we)));
            continue;
        }
        for (ge, we) in ge.iter().zip(we) {
            for node in ["startNode", "endNode"] {
                let d = (ge[node][0].as_f64().unwrap() - we[node][0].as_f64().unwrap()).hypot(ge[node][1].as_f64().unwrap() - we[node][1].as_f64().unwrap());
                if d >= NODE_TOL {
                    why.push(format!("{node} off by {d}"));
                }
            }
            for key in ["radius", "r1", "r2", "length"] {
                if let Some(w) = we.get(key).filter(|w| !w.is_null())
                    && !close(&ge[key], w, 1e-6)
                {
                    why.push(format!("{key} {} want {w}", ge[key]));
                }
            }
        }
    }
    why
}

#[test]
fn the_splice_finds_what_the_service_found() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("data/splice_answers.json")).unwrap();
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let mut failures = Vec::new();
    for case in &cases {
        let got = splice_payload(&rules, &case["request"]).unwrap();
        let why = same_answer(&got, &case["answer"]);
        if !why.is_empty() {
            failures.push(format!("{}: {}", case["name"], why[..why.len().min(4)].join("; ")));
        }
    }
    assert!(failures.is_empty(), "{} of {} differ:\n{}", failures.len(), cases.len(), failures.join("\n"));
    assert_eq!(cases.len(), 243);
}
