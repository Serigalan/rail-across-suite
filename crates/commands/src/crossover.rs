// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The search of the extended switch connection: its request, and an answer
//! adopted into the dialog (`crossover.js`).

use rail_across_js::{js_number, js_to_fixed, number};
use rail_across_rules::cant::max_switch_cant_def;
use rail_across_rules::element_length::{comparison_radius_limits, min_element_length};
use rail_across_rules::physics::CANT_DEFICIENCY_COEFF;
use rail_across_switch::connection::connection_branch_ends;
use rail_across_switch::mid_chain::{Mid, MidKind, mid_unknowns};
use serde_json::{Value, json};

use crate::scurve::{Ask, connection_remnants, settle_connection, stems};

const SAMPLE_STEP: f64 = 0.5;
const SEARCH_REACH: f64 = 300.0;
const CANT_TOL: f64 = 1.0;

/// The request of the search (`crossoverRequest`).
pub fn crossover_request(picks: &[Value], speed: f64, form: Option<&Value>) -> Value {
    let (g1, g2) = stems(picks);
    let ends = connection_branch_ends(&g1, &g2, speed, SAMPLE_STEP, SEARCH_REACH, form);
    let flipped2 = ends["flipped2"].clone();
    let forms = ends["forms"].as_array().cloned().unwrap_or_default();
    let origin = forms
        .iter()
        .flat_map(|f| f["first"].as_array().cloned().unwrap_or_default())
        .next()
        .unwrap_or_else(|| json!({ "x": 0, "y": 0 }));
    let (ox, oy) = (js_number(origin.get("x")), js_number(origin.get("y")));
    let remnants_ok = |k: usize, s: f64, f: &Value| -> bool {
        let res = json!({ "s2": if k == 1 { s } else { 0.0 }, "flipped2": flipped2, "throughLength": f["through"], "throughLength2": f["through2"] });
        !connection_remnants(picks, if k == 0 { s } else { 0.0 }, &res, speed)
            .iter()
            .any(|r| r.track == k && r.short)
    };
    let sample = |k: usize, f: &Value, p: &Value| -> Value {
        let s = js_number(p.get("s"));
        json!({
            "s": p["s"], "x": number(js_number(p.get("x")) - ox), "y": number(js_number(p.get("y")) - oy), "psi": p["psi"], "kappa": p["kappa"],
            "tail": p["tail"], "cantLds": p["cantLds"], "ok": p["ok"] == json!(true) && remnants_ok(k, s, f),
        })
    };
    let rw = comparison_radius_limits(speed);
    json!({
        "speed": number(speed),
        "limits": {
            "lMin": number(min_element_length(speed).unwrap_or(0.0)),
            "rwReg": rw.map(|r| number(r.0)), "rwDiscretion": rw.map(|r| number(r.1)),
            "maxCantDef": number(max_switch_cant_def()), "cantDefCoeff": number(CANT_DEFICIENCY_COEFF), "cantTol": number(CANT_TOL),
        },
        "forms": forms.iter().map(|f| json!({
            "label": f["sw"]["label"], "label2": f["sw2"]["label"],
            "first": f["first"].as_array().into_iter().flatten().map(|p| sample(0, f, p)).collect::<Vec<_>>(),
            "second": f["second"].as_array().into_iter().flatten().map(|p| sample(1, f, p)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// One solution of the service adopted into the dialog (`adoptCrossover`): { shift, mids, seed } or None.
pub fn adopt_crossover(picks: &[Value], speed: f64, solution: &Value, form: Option<&Value>, lenient: bool, exact_first: bool) -> Option<Value> {
    let s1 = js_number(solution.get("s1"));
    let s2 = js_number(solution.get("s2"));
    let pieces = solution["pieces"].as_array().cloned().unwrap_or_default();
    let exact: Vec<Mid> = pieces
        .iter()
        .map(|p| {
            let length = Some(js_number(p.get("length")));
            match p["type"].as_str() {
                Some("arc") => {
                    let k = js_number(p.get("kappa"));
                    Mid {
                        kind: MidKind::Arc,
                        length,
                        radius: Some(1.0 / k.abs()),
                        side: Some(-js_sign(k)),
                    }
                }
                Some("straight") => Mid {
                    kind: MidKind::Straight,
                    length,
                    radius: None,
                    side: None,
                },
                _ => Mid {
                    kind: MidKind::Transition,
                    length,
                    radius: None,
                    side: None,
                },
            }
        })
        .collect();
    let round = |v: f64, d: usize| js_number(Some(&json!(js_to_fixed(v, d))));
    let rounded: Vec<Mid> = exact
        .iter()
        .map(|m| Mid {
            length: m.length.map(|l| round(l, 3)),
            radius: m.radius.map(|r| round(r, 1)),
            ..*m
        })
        .collect();
    let order: [&Vec<Mid>; 2] = if exact_first { [&exact, &rounded] } else { [&rounded, &exact] };
    for full in order {
        if let Some(a) = adopt_spec(picks, speed, form, lenient, s1, s2, &pieces, full) {
            return Some(a);
        }
    }
    None
}

fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

fn adopt_spec(picks: &[Value], speed: f64, form: Option<&Value>, lenient: bool, s1: f64, s2: f64, pieces: &[Value], full: &[Mid]) -> Option<Value> {
    // (element, is length)
    let slots: Vec<(usize, bool)> = full
        .iter()
        .enumerate()
        .flat_map(|(i, m)| std::iter::once((i, true)).chain((m.kind == MidKind::Arc).then_some((i, false))))
        .collect();
    let mut pairs: Vec<[(usize, bool); 2]> = Vec::new();
    for a in 0..slots.len() {
        for b in a + 1..slots.len() {
            pairs.push([slots[a], slots[b]]);
        }
    }
    pairs.sort_by_key(|p| p.iter().filter(|u| !u.1).count());
    for pair in pairs {
        let mids: Vec<Mid> = full
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let mut m = *m;
                if pair.iter().any(|u| u.0 == i && u.1) {
                    m.length = None;
                }
                if pair.iter().any(|u| u.0 == i && !u.1) {
                    m.radius = None;
                    m.side = None;
                }
                m
            })
            .collect();
        let values: Vec<f64> = mid_unknowns(&mids)
            .iter()
            .map(|u| {
                if u.length {
                    js_number(pieces[u.i].get("length"))
                } else {
                    -js_number(pieces[u.i].get("kappa"))
                }
            })
            .collect();
        let ask = Ask {
            picks,
            speed,
            mids: Some(mids.clone()),
            seed: Some((s2 - s1, values.clone())),
            form,
            lenient: false,
        };
        let settled = settle_connection(&ask, s1);
        let r = &settled.result;
        if r.get("valid") == Some(&json!(true)) || (lenient && r.get("overridable") == Some(&json!(true))) {
            return Some(json!({
                "shift": number(settled.shift),
                "mids": mids.iter().map(|m| mid_json(m)).collect::<Vec<_>>(),
                "seed": { "ds": number(s2 - s1), "values": values.into_iter().map(number).collect::<Vec<_>>() },
            }));
        }
    }
    None
}

/// A spec entry as the JavaScript writes it: `side` left out where it was taken away.
fn mid_json(m: &Mid) -> Value {
    m.to_json()
}
