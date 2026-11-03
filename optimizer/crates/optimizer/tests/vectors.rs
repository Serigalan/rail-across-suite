// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The shared test vectors (packages/core/constraints/tests/) against the Rust
//! side. The app (ruleExpr.js, trassierungCheck.js, rules/transitionLength.js)
//! and this crate implement the same expression language, the same catalogue
//! check and the same shortest transition lengths twice; the vectors are one
//! set of cases both have to answer alike. The app's half is
//! packages/core/src/rules/*.vectors.test.js.

mod common;

use rail_across_optimizer::element::Element;
use rail_across_optimizer::expr::{Basic, Value, eval_expr};
use rail_across_optimizer::grenzen::{Grenzen, Stufe, transition_lengths};
use rail_across_optimizer::pruefung::{Context, check_track};
use serde_json::Value as Json;

fn same_number(got: f64, want: f64) -> bool {
    (got - want).abs() <= 1e-9 * want.abs().max(1.0)
}

#[test]
fn expressions() {
    let data = common::constraints("tests/expressions.json");
    let cases = data["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let src = case["src"].as_str().unwrap();
        let scope: Vec<(&str, Value)> = case
            .get("scope")
            .and_then(Json::as_object)
            .map(|o| o.iter().map(|(k, v)| (k.as_str(), Value::from_json(v).unwrap())).collect())
            .unwrap_or_default();
        let got = eval_expr(src, &scope, &Basic);
        let ok = if case.get("error").and_then(Json::as_bool) == Some(true) {
            got.is_err()
        } else {
            match (&got, Value::from_json(&case["value"]).unwrap()) {
                (Ok(Value::Num(g)), Value::Num(w)) => same_number(*g, w),
                (Ok(g), w) => *g == w,
                (Err(_), _) => false,
            }
        };
        if !ok {
            failures.push(format!("{src}: want {}, got {got:?}", case.get("value").unwrap_or(&Json::Null)));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(cases.len() > 30);
}

#[test]
fn checks() {
    let data = common::constraints("tests/checks.json");
    assert_eq!(data["catalogue"], "db-ril-800-0110");
    let katalog = common::katalog();
    let chains = data["chains"].as_array().unwrap();
    let mut failures = Vec::new();
    for chain in chains {
        let els: Vec<Element> = serde_json::from_value(chain["elements"].clone()).unwrap();
        let mut got: Vec<[String; 3]> = check_track(&katalog, &els, None, Context::default())
            .unwrap()
            .into_iter()
            .map(|f| [f.at, f.rule, f.severity])
            .collect();
        let mut want: Vec<[String; 3]> = serde_json::from_value(chain["findings"].clone()).unwrap();
        got.sort();
        want.sort();
        if got != want {
            let missing: Vec<_> = want.iter().filter(|f| !got.contains(f)).take(5).collect();
            let extra: Vec<_> = got.iter().filter(|f| !want.contains(f)).take(5).collect();
            failures.push(format!("{}: missing {missing:?}, extra {extra:?}", chain["name"]));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(!chains.is_empty());
}

#[test]
fn lengths() {
    let data = common::constraints("tests/transition_lengths.json");
    let katalog = common::katalog();
    let reg = Grenzen::new(&katalog, Stufe::Reg).unwrap();
    let discretion = Grenzen::new(&katalog, Stufe::Discretion).unwrap();
    let element = |v: &Json| -> Option<Element> { (!v.is_null()).then(|| serde_json::from_value(v.clone()).unwrap()) };
    let mut failures = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let name = &case["name"];
        let got = transition_lengths(
            element(&case["prev"]).as_ref(),
            element(&case["next"]).as_ref(),
            case["r1"].as_f64(),
            case["type"].as_str().unwrap(),
            case["speed"].as_f64().unwrap(),
            &reg,
            &discretion,
        )
        .unwrap();
        for (key, length, by) in [("regular", got.regular, &got.regular_by), ("minimum", got.minimum, &got.minimum_by)] {
            let want = case[key].as_f64();
            let same = match (length, want) {
                (None, None) => true,
                (Some(g), Some(w)) => (g - w).abs() < 1e-9,
                _ => false,
            };
            if !same {
                failures.push(format!("{name} {key}: want {want:?}, got {length:?}"));
            }
            let rows = case[format!("{key}By")].as_array().unwrap();
            let same_rows = rows.len() == by.len()
                && rows.iter().zip(by).all(|(w, g)| {
                    w[0].as_str() == Some(g.id.as_str()) && (w[1].as_f64().unwrap() - g.length).abs() <= 1e-6 && w[2].as_bool() == Some(g.binding)
                });
            if !same_rows {
                failures.push(format!("{name} {key}By: want {rows:?}, got {by:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
