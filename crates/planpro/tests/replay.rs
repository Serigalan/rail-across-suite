// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The reader's vectors, recorded from its JavaScript before it went (decision
//! 0005), replayed on the crate: the reader is no part of the
//! WebAssembly module any more — only the check of the export — so this test,
//! not `crates/core-wasm/tests/replay.rs`, replays them. Each case is
//! `{ op, input, output | error }`, compared as that test compares them.

use rail_across_planpro::{planpro, xml};
use serde_json::{Value, json};

/// One unit in the last place of `x`.
fn ulp(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        f64::MIN_POSITIVE
    } else {
        2f64.powi(x.log2().floor() as i32 - 52)
    }
}

const TOL: f64 = 4e-9;

fn same(a: &Value, b: &Value, path: &str, tol: f64) -> Result<(), String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            let d = (x - y).abs();
            if x == y || d <= tol || d <= 4.0 * ulp(x.abs().max(y.abs())) {
                Ok(())
            } else {
                Err(format!("{path}: {x} ≠ {y} (Δ {d:e})"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} items ≠ {}", x.len(), y.len()));
            }
            x.iter().zip(y).enumerate().try_for_each(|(i, (p, q))| same(p, q, &format!("{path}/{i}"), tol))
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => same(p, q, &format!("{path}/{k}"), tol)?,
                    (p, q) => return Err(format!("{path}/{k}: {p:?} ≠ {q:?}")),
                }
            }
            Ok(())
        }
        _ if a == b => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {b}")),
    }
}

/// A recorded call on the crate, as the module answered it.
fn run(op: &str, input: &Value) -> Result<Value, String> {
    let text = input[0].as_str().unwrap_or("");
    let opt = |k: &str| input.get(1).and_then(|o| o.get(k)).and_then(Value::as_str);
    match op {
        "planpro.parseXml" => xml::parse_xml(text).map(|el| el.to_json()),
        "planpro.readPlanPro" => xml::parse_xml(text).and_then(|root| planpro::read_planpro(&root).map(|m| m.to_json())),
        "planpro.planproRecord" => planpro::planpro_record(text, opt("id"), opt("title")),
        _ => panic!("no such operation: {op}"),
    }
}

#[test]
fn every_recorded_call_comes_out_as_the_javascripts() {
    let file = format!("{}/tests/vectors/planpro.json", env!("CARGO_MANIFEST_DIR"));
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let op = case["op"].as_str().unwrap();
        let tol = case.get("tol").and_then(Value::as_f64).unwrap_or(TOL);
        let verdict = match (run(op, &case["input"]), case.get("output")) {
            (Ok(got), Some(want)) => same(&got, want, "", tol),
            (Err(e), Some(_)) => Err(format!("refused: {e}")),
            (Ok(_), None) => Err("taken, but the JavaScript refused it".into()),
            (Err(e), None) => same(&json!({ "message": e }), &case["error"], "error", tol),
        };
        if let Err(why) = verdict {
            failures.push(format!("#{i} {op}: {why}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} calls differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
