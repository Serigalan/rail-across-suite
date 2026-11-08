// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The cant of elements (`clothoidUtils.js`, Entscheidungen 355, 369, 371):
//! `cant` where it is the same at both ends, `cantStart` + `cantEnd` where it
//! changes along an element, never both; a transition takes its ends from its
//! neighbours unless it states its own.

use rail_across_js::{Object, get, num, number, truthy};
use serde_json::Value;

/// Two cants this close are one [mm] — the thousandth a record keeps.
pub const SAME_CANT: f64 = 0.001;

fn is_transition(el: &Value) -> bool {
    num(el, "elementType") == Some(2.0)
}

/// `a ?? b` on a number member.
fn member_or(el: &Value, key: &str, otherwise: f64) -> f64 {
    get(el, key).and_then(Value::as_f64).unwrap_or(otherwise)
}

/// The cant an element states at `end` (`start` or not) itself — its own end,
/// else its one `cant`; 0 for none, or no element (`endCant`).
pub fn end_cant(el: Option<&Value>, start: bool) -> f64 {
    let Some(el) = el else { return 0.0 };
    let own = get(el, if start { "cantStart" } else { "cantEnd" }).and_then(Value::as_f64);
    own.unwrap_or_else(|| member_or(el, "cant", 0.0))
}

/// Cant at the two ends of the transition `elements[i]` (`transitionCantEnds`).
pub fn transition_cant_ends(elements: &[Value], i: usize) -> (f64, f64) {
    let el = &elements[i];
    let prev = if i > 0 { elements.get(i - 1) } else { None };
    let start = get(el, "cantStart").and_then(Value::as_f64).unwrap_or_else(|| end_cant(prev, false));
    let end = get(el, "cantEnd")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| end_cant(elements.get(i + 1), true));
    (start, end)
}

/// Cant at the two ends of any element (`cantEnds`).
pub fn cant_ends(elements: &[Value], i: usize) -> (f64, f64) {
    let el = &elements[i];
    if is_transition(el) {
        return transition_cant_ends(elements, i);
    }
    (end_cant(Some(el), true), end_cant(Some(el), false))
}

fn without(el: &Value, keys: &[&str]) -> Object {
    let mut out = el.as_object().cloned().unwrap_or_default();
    for k in keys {
        out.shift_remove(*k);
    }
    out
}

/// `x || 0` for a number.
fn or_zero(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() { 0.0 } else { x }
}

/// An arc or a straight with the cant `start` at its start and `end` at its end (`withCantRamp`).
pub fn with_cant_ramp(el: &Value, start: f64, end: f64) -> Value {
    let mut rest = without(el, &["cantStart", "cantEnd", "cant"]);
    let radius = get(el, "radius");
    if (start - end).abs() <= SAME_CANT {
        if truthy(Some(&number(start))) || truthy(radius) {
            rest.insert("cant".into(), number(or_zero(start)));
        }
        return Value::Object(rest);
    }
    let r = radius.and_then(Value::as_f64).unwrap_or(f64::NAN);
    let side = if r < 0.0 { -1.0 } else { 1.0 };
    let cant = if truthy(radius) && side * end < side * start { end } else { start };
    rest.insert("cant".into(), number(cant));
    rest.insert("cantStart".into(), number(start));
    rest.insert("cantEnd".into(), number(end));
    Value::Object(rest)
}

/// An element's cant as a record keeps it (`storedCants`).
fn stored_cants(el: &Value) -> Value {
    let rest = without(el, &["cant"]);
    let has_start = get(el, "cantStart").is_some();
    let has_end = get(el, "cantEnd").is_some();
    if is_transition(el) {
        return if has_start || has_end { Value::Object(rest) } else { el.clone() };
    }
    if !has_start || !has_end {
        return el.clone();
    }
    let cant_start = member_or(el, "cantStart", f64::NAN);
    let cant_end = member_or(el, "cantEnd", f64::NAN);
    // `(cantStart - cantEnd) > SAME_CANT` is false for NaN, as in JavaScript.
    if (cant_start - cant_end).abs() > SAME_CANT {
        return Value::Object(rest);
    }
    let mut bare = rest;
    let start_value = bare.shift_remove("cantStart");
    bare.shift_remove("cantEnd");
    if truthy(start_value.as_ref()) || truthy(get(el, "radius")) {
        bare.insert("cant".into(), start_value.unwrap_or(Value::Null));
    }
    Value::Object(bare)
}

/// A track's elements with their cant as a record keeps it (`storedTrackCants`).
pub fn stored_track_cants(elements: &[Value]) -> Vec<Value> {
    elements
        .iter()
        .enumerate()
        .map(|(i, el)| {
            if !is_transition(el) {
                return stored_cants(el);
            }
            let mut rest = without(el, &["cant"]);
            let (start, end) = transition_cant_ends(elements, i);
            rest.insert("cantStart".into(), number(or_zero(start)));
            rest.insert("cantEnd".into(), number(or_zero(end)));
            Value::Object(rest)
        })
        .collect()
}

/// An element's cant as the model reads it from a record (`loadedCants`).
fn loaded_cants(el: &Value) -> Value {
    if is_transition(el) || get(el, "cant").is_some() || get(el, "cantStart").is_none() || get(el, "cantEnd").is_none() {
        return el.clone();
    }
    with_cant_ramp(el, member_or(el, "cantStart", 0.0), member_or(el, "cantEnd", 0.0))
}

/// A transition's ends left out where they are what its neighbours give it anyway (`followingCants`).
pub fn following_cants(elements: Vec<Value>) -> Vec<Value> {
    let mut out = elements;
    for i in 0..out.len() {
        if !is_transition(&out[i]) {
            continue;
        }
        let mut next = out[i].clone();
        let prev_end = end_cant(if i > 0 { out.get(i - 1) } else { None }, false);
        if let Some(s) = get(&next, "cantStart").and_then(Value::as_f64)
            && (s - prev_end).abs() <= SAME_CANT
        {
            next = Value::Object(without(&next, &["cantStart"]));
        }
        let next_start = end_cant(out.get(i + 1), true);
        if let Some(e) = get(&next, "cantEnd").and_then(Value::as_f64)
            && (e - next_start).abs() <= SAME_CANT
        {
            next = Value::Object(without(&next, &["cantEnd"]));
        }
        out[i] = next;
    }
    out
}

/// A track's elements as the model reads them from a record (`loadedTrackCants`).
pub fn loaded_track_cants(elements: &[Value]) -> Vec<Value> {
    following_cants(
        elements
            .iter()
            .map(|el| {
                if is_transition(el) {
                    let mut o = el.as_object().cloned().unwrap_or_default();
                    let s = get(el, "cantStart").cloned().unwrap_or(number(0.0));
                    let e = get(el, "cantEnd").cloned().unwrap_or(number(0.0));
                    o.insert("cantStart".into(), s);
                    o.insert("cantEnd".into(), e);
                    Value::Object(o)
                } else {
                    loaded_cants(el)
                }
            })
            .collect(),
    )
}
