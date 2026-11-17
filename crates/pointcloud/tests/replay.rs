// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The JavaScript point clouds' calls replayed on the crate (decision 0005): every vector file under `tests/vectors/` (written by `npm run
//! replay pointcloud` in packages/core) is a list of `{ op, input, output |
//! error }`, compared as `crates/core-wasm/tests/replay.rs` compares them —
//! which leaves this crate's vectors to this test, the crate being no part
//! of the WebAssembly module. Files are referred to from the repository root.

use rail_across_pointcloud::ops::run;
use serde_json::Value;

/// One unit in the last place of `x`.
fn ulp(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        f64::MIN_POSITIVE
    } else {
        2f64.powi(x.log2().floor() as i32 - 52)
    }
}

/// The tolerance a case states (`tol`: what it derives from two converted
/// points, a bearing or a ratio, amplifies their last digit), else 4e-9.
fn same(a: &Value, b: &Value, path: &str) -> Result<(), String> {
    same_to(a, b, path, TOL)
}

const TOL: f64 = 4e-9;

fn same_to(a: &Value, b: &Value, path: &str, tol: f64) -> Result<(), String> {
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
            x.iter()
                .zip(y)
                .enumerate()
                .try_for_each(|(i, (p, q))| same_to(p, q, &format!("{path}/{i}"), tol))
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => same_to(p, q, &format!("{path}/{k}"), tol)?,
                    (p, q) => return Err(format!("{path}/{k}: {p:?} ≠ {q:?}")),
                }
            }
            Ok(())
        }
        _ if a == b => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {b}")),
    }
}

#[test]
fn every_recorded_call_comes_out_as_the_javascripts() {
    let dir = format!("{}/tests/vectors", env!("CARGO_MANIFEST_DIR"));
    std::env::set_current_dir(format!("{}/../..", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|f| f.ok().map(|f| f.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut calls = 0;
    let mut failures = Vec::new();
    for file in &files {
        let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        for (i, case) in cases.iter().enumerate() {
            let op = case["op"].as_str().unwrap();
            calls += 1;
            let tol = case.get("tol").and_then(Value::as_f64).unwrap_or(TOL);
            let verdict = match (run(op, &case["input"]), case.get("output")) {
                (Ok(got), Some(want)) => same_to(&got, want, "", tol),
                (Err(e), Some(_)) => Err(format!("refused: {}", e.message())),
                (Ok(_), None) => Err("taken, but the JavaScript refused it".into()),
                (Err(e), None) => same(&serde_json::json!({ "message": e.message() }), &case["error"], "error"),
            };
            if let Err(why) = verdict {
                failures.push(format!("{} #{i} {op}: {why}", file.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{} of {calls} calls differ:\n{}", failures.len(), failures.join("\n"));
    assert!(calls > 0, "no vectors");
}
