// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The JavaScript core's calls replayed on the module (decision 0020): every
//! vector file under `crates/*/tests/vectors/` (written by `npm run replay` in
//! packages/core) is a list of `{ op, input, output | error }`, and each call
//! has to come out as the JavaScript's did — members in any order, a refusal
//! with the same code and details, numbers by value: alike to 4e-9 — two units
//! in the last place of a coordinate of 10⁷ m, which a difference of two such
//! numbers keeps — or to four units in their own last place, since `sin`,
//! `cos` and `atan2` may differ in the last digit between V8 and Rust's libm.

use rail_across_core_wasm::dispatch;
use serde_json::{Value, json};

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
    let crates = format!("{}/..", env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<_> = std::fs::read_dir(&crates)
        .unwrap()
        .filter_map(|c| std::fs::read_dir(c.ok()?.path().join("tests/vectors")).ok())
        .flatten()
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
            let mut request = json!({ "op": op, "input": case["input"] });
            if let Some(ids) = case.get("ids") {
                request["ids"] = ids.clone();
            }
            let answer: Value = serde_json::from_str(&dispatch(&request.to_string())).unwrap();
            calls += 1;
            let tol = case.get("tol").and_then(Value::as_f64).unwrap_or(TOL);
            let verdict = if let Some(want) = case.get("output") {
                match answer.get("ok") {
                    Some(got) => same_to(got, want, "", tol),
                    None => Err(format!("refused: {}", answer)),
                }
            } else {
                let got = answer.get("data").cloned().unwrap_or(Value::Null);
                match answer.get("ok") {
                    Some(_) => Err("taken, but the JavaScript refused it".into()),
                    None => same(&got, &case["error"], "error"),
                }
            };
            if let Err(why) = verdict {
                failures.push(format!("{} #{i} {op}: {why}", file.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{} of {calls} calls differ:\n{}", failures.len(), failures.join("\n"));
    assert!(calls > 0, "no vectors");
}
