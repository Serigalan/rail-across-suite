// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Runs of elements that are one element (`mergeElements.js`).

use std::collections::BTreeSet;

use rail_across_geom::element::{Utm, curved_values, resolve_end_bearing, straight_values};
use rail_across_geom::reconstruct::reconstruct_elements;
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::curvature_of;
use rail_across_js::{get, has, number, strict_eq, truthy};
use serde_json::{Value, json};

use crate::util::{num, obj, text, without};

const BEARING_TOL: f64 = 1e-6;
const RADIUS_TOL: f64 = 1e-6;
const EQUAL_RADIUS_TOL: f64 = 1e-3;
const CANT_TOL: f64 = 1e-6;
const PLACEMENT: &[&str] = &[
    "startNode",
    "endNode",
    "bearing",
    "endBearing",
    "length",
    "geometry",
    "renderCoords",
    "epsg",
    "radius",
    "r1",
    "r2",
    "cantStart",
    "cantEnd",
];

fn bearing_delta(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs()
}

fn same_fields(a: &Value, b: &Value) -> bool {
    let keys: BTreeSet<&String> = a
        .as_object()
        .into_iter()
        .flatten()
        .chain(b.as_object().into_iter().flatten())
        .map(|(k, _)| k)
        .filter(|k| !PLACEMENT.contains(&k.as_str()))
        .collect();
    // An object a copy shares with its original is one object in the
    // JavaScript; across JSON only its value is left to compare.
    keys.into_iter().all(|k| strict_eq(a.get(k), b.get(k)) || a.get(k) == b.get(k))
}

fn cant_pair(el: &Value) -> (f64, f64) {
    let c = num(el, "cant");
    (num(el, "cantStart").or(c).unwrap_or(0.0), num(el, "cantEnd").or(c).unwrap_or(0.0))
}

fn same_kind(a: &Value, b: &Value) -> bool {
    if num(a, "elementType").unwrap_or(0.0) != num(b, "elementType").unwrap_or(0.0) {
        return false;
    }
    if num(a, "elementType") == Some(2.0) {
        return text(a, "transitionType").unwrap_or("clothoid") == text(b, "transitionType").unwrap_or("clothoid");
    }
    let ((a0, a1), (b0, b1)) = (cant_pair(a), cant_pair(b));
    (a0 - a1).abs() < CANT_TOL && (b0 - b1).abs() < CANT_TOL && (a0 - b0).abs() < CANT_TOL
}

fn curvature_rate(el: &Value) -> f64 {
    match num(el, "length") {
        Some(l) if l > 0.0 => (curvature_of(num(el, "r2")) - curvature_of(num(el, "r1"))) / l,
        _ => 0.0,
    }
}

/// Is this run of elements one element (`mergeableRun`)?
pub fn mergeable_run(els: &[Value], epsg: i64, equal: bool) -> bool {
    if els.len() < 2 {
        return false;
    }
    if els
        .iter()
        .any(|el| !el.is_object() || truthy(el.get("switchBranch")) || !truthy(el.get("startNode")) || !truthy(el.get("endNode")))
    {
        return false;
    }
    let first = &els[0];
    let alike = |e: &Value| if equal { same_kind(first, e) } else { same_fields(first, e) };
    if !els.iter().all(alike) {
        return false;
    }
    let radius_tol = if equal { EQUAL_RADIUS_TOL } else { RADIUS_TOL };
    if num(first, "elementType") == Some(2.0) {
        if text(first, "transitionType") == Some("bloss") || !has(first, "r1") {
            return false;
        }
        let rate = curvature_rate(first);
        if els.iter().any(|el| (curvature_rate(el) - rate).abs() > 1e-9 * rate.abs().max(1e-9)) {
            return false;
        }
        for w in els.windows(2) {
            let k1 = curvature_of(num(&w[0], "r2"));
            let k2 = curvature_of(num(&w[1], "r1"));
            if if equal { (k1 - k2).abs() > 1e-12 } else { k1 != k2 } {
                return false;
            }
            let (c1, c2) = (get(&w[0], "cantEnd"), get(&w[1], "cantStart"));
            let differ = match (equal, c1.and_then(Value::as_f64), c2.and_then(Value::as_f64)) {
                (true, Some(x), Some(y)) => (x - y).abs() >= CANT_TOL,
                _ => !strict_eq(c1.or(Some(&Value::Null)), c2.or(Some(&Value::Null))),
            };
            if differ {
                return false;
            }
        }
    } else if let Some(r0) = num(first, "radius") {
        if els.iter().any(|el| num(el, "radius").is_none_or(|r| (r - r0).abs() > radius_tol)) {
            return false;
        }
        let sweep = els.iter().map(|el| num(el, "length").unwrap_or(0.0)).sum::<f64>() / r0.abs();
        if !(sweep < std::f64::consts::PI) {
            return false;
        }
    } else if els.iter().any(|el| num(el, "radius").is_some()) {
        return false;
    }
    for w in els.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (e, s) = (&a["endNode"], &b["startNode"]);
        let f = |v: &Value, i: usize| v.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
        if (f(e, 0) - f(s, 0)).hypot(f(e, 1) - f(s, 1)) > NODE_TOL {
            return false;
        }
        if bearing_delta(resolve_end_bearing(a, epsg), b.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN)) > BEARING_TOL {
            return false;
        }
    }
    true
}

fn merge_run(els: &[Value], epsg: i64) -> Value {
    let (first, last) = (&els[0], &els[els.len() - 1]);
    let f = |v: &Value, k: &str, i: usize| v[k].get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let s = Utm {
        e: f(first, "startNode", 0),
        n: f(first, "startNode", 1),
        zone: epsg,
    };
    let e = Utm {
        e: f(last, "endNode", 0),
        n: f(last, "endNode", 1),
        zone: epsg,
    };
    let mut merged = without(first, &["endBearing", "geometry", "renderCoords"]);
    if num(first, "elementType") == Some(2.0) {
        merged.insert("r1".into(), get(first, "r1").cloned().unwrap_or(Value::Null));
        merged.insert("r2".into(), get(last, "r2").cloned().unwrap_or(Value::Null));
        merged.insert("endNode".into(), last["endNode"].clone());
        merged.insert("length".into(), number(els.iter().map(|el| num(el, "length").unwrap_or(0.0)).sum()));
        if let Some(eb) = get(last, "endBearing") {
            merged.insert("endBearing".into(), eb.clone());
        }
        if has(first, "cantStart") || has(last, "cantEnd") {
            merged.insert("cantStart".into(), get(first, "cantStart").cloned().unwrap_or(Value::Null));
            merged.insert("cantEnd".into(), get(last, "cantEnd").cloned().unwrap_or(Value::Null));
        }
    } else if let Some(r) = num(first, "radius") {
        let (length, bearing, end_bearing) = curved_values(s, e, r);
        merged.insert("endNode".into(), json!([number(e.e), number(e.n)]));
        merged.insert("bearing".into(), number(bearing));
        merged.insert("endBearing".into(), number(end_bearing));
        merged.insert("length".into(), number(length));
        merged.insert("radius".into(), number(r));
    } else {
        let (length, bearing) = straight_values(s, e);
        merged.insert("endNode".into(), json!([number(e.e), number(e.n)]));
        merged.insert("bearing".into(), number(bearing));
        merged.insert("length".into(), number(length));
        if let Some(eb) = get(last, "endBearing") {
            merged.insert("endBearing".into(), eb.clone());
        }
    }
    reconstruct_elements(&[Value::Object(merged)], epsg).remove(0)
}

/// The elements with every run that is one element merged (`mergeChain`).
pub fn merge_chain(elements: &[Value], epsg: i64, equal: bool) -> Value {
    let mut out = Vec::new();
    let (mut from, mut into) = (0, 0);
    let mut i = 0;
    while i < elements.len() {
        let mut j = i + 1;
        while j < elements.len() && mergeable_run(&elements[i..=j], epsg, equal) {
            j += 1;
        }
        if j - i > 1 {
            out.push(merge_run(&elements[i..j], epsg));
            from += j - i;
            into += 1;
        } else {
            out.push(elements[i].clone());
        }
        i = j;
    }
    let _ = obj;
    json!({ "elements": out, "mergedFrom": from, "mergedInto": into })
}
