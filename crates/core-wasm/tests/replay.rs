// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The JavaScript core's calls replayed on the module (decision 0020): every
//! vector file under `crates/*/tests/vectors/` (written by `npm run replay` in
//! packages/core) is a list of `{ op, input, output | error }`, and each call
//! has to come out as the JavaScript's did — numbers by value, members in any
//! order, a refusal with the same code and details.

use rail_across_core_wasm::dispatch;
use serde_json::{Value, json};

fn same(a: &Value, b: &Value, path: &str) -> Result<(), String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if x == y { Ok(()) } else { Err(format!("{path}: {x} ≠ {y}")) }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} items ≠ {}", x.len(), y.len()));
            }
            x.iter().zip(y).enumerate().try_for_each(|(i, (p, q))| same(p, q, &format!("{path}/{i}")))
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => same(p, q, &format!("{path}/{k}"))?,
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
            let answer: Value = serde_json::from_str(&dispatch(&json!({ "op": op, "input": case["input"] }).to_string())).unwrap();
            calls += 1;
            let verdict = if let Some(want) = case.get("output") {
                match answer.get("ok") {
                    Some(got) => same(got, want, ""),
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
