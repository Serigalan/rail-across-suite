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

/// Does the element state its cant at its two ends rather than as one `cant` (`hasCantEnds`)?
pub fn has_cant_ends(el: &Value) -> bool {
    is_transition(el) || get(el, "cantStart").is_some() || get(el, "cantEnd").is_some()
}

/// An element with its cant at one end set (`withEndCant`).
fn with_end_cant(el: &Value, start: bool, value: f64) -> Value {
    if is_transition(el) {
        let mut o = el.as_object().cloned().unwrap_or_default();
        o.insert(if start { "cantStart" } else { "cantEnd" }.into(), number(value));
        return Value::Object(o);
    }
    if !has_cant_ends(el) {
        let mut o = el.as_object().cloned().unwrap_or_default();
        o.insert("cant".into(), number(value));
        return Value::Object(o);
    }
    let own_start = get(el, "cantStart").and_then(Value::as_f64).unwrap_or_else(|| member_or(el, "cant", 0.0));
    let own_end = get(el, "cantEnd").and_then(Value::as_f64).unwrap_or_else(|| member_or(el, "cant", 0.0));
    let s = if start { value } else { own_start };
    let e = if !start { value } else { own_end };
    with_cant_ramp(el, s, e)
}

/// Carry the cant `v` over a joint into `out`, from `from` on in direction `step` (`carryCant`).
fn carry_cant(out: &mut [Value], from: isize, step: isize, v: f64) {
    let mut j = from;
    while j >= 0 && (j as usize) < out.len() {
        let own = has_cant_ends(&out[j as usize]);
        out[j as usize] = with_end_cant(&out[j as usize], step > 0, v);
        if own {
            return;
        }
        j += step;
    }
}

/// The elements with the cant `v` at the track's end `endpoint` (`withCantAtEnd`).
pub fn with_cant_at_end(elements: &[Value], endpoint: &str, v: f64) -> Vec<Value> {
    if elements.is_empty() {
        return elements.to_vec();
    }
    let mut out = elements.to_vec();
    if endpoint == "END" {
        let last = out.len() as isize - 1;
        carry_cant(&mut out, last, -1, v);
    } else {
        carry_cant(&mut out, 0, 1, v);
    }
    following_cants(out)
}

/// The elements with the cant of the run `from`–`to` set at both ends of each (`withCantsOver`).
pub fn with_cants_over(elements: &[Value], from: usize, to: usize, ends: &[(f64, f64)]) -> Vec<Value> {
    let mut out = elements.to_vec();
    for i in from..=to {
        let (start, end) = ends[i - from];
        let el = &out[i];
        out[i] = if is_transition(el) {
            let mut o = el.as_object().cloned().unwrap_or_default();
            o.insert("cantStart".into(), number(start));
            o.insert("cantEnd".into(), number(end));
            Value::Object(o)
        } else {
            with_cant_ramp(el, start, end)
        };
    }
    carry_cant(&mut out, from as isize - 1, -1, ends[0].0);
    carry_cant(&mut out, to as isize + 1, 1, ends[ends.len() - 1].1);
    following_cants(out)
}

/// The elements with one cant typed in the element table (`withCantEdit`).
pub fn with_cant_edit(elements: &[Value], i: usize, key: &str, value: f64) -> Vec<Value> {
    let mut out = elements.to_vec();
    let Some(el) = out.get(i).cloned() else { return elements.to_vec() };
    let i = i as isize;
    if key == "cant" {
        if has_cant_ends(&el) {
            return elements.to_vec();
        }
        out[i as usize] = with_end_cant(&el, true, value);
        carry_cant(&mut out, i - 1, -1, value);
        carry_cant(&mut out, i + 1, 1, value);
    } else {
        if !has_cant_ends(&el) {
            return elements.to_vec();
        }
        let start = key == "cantStart";
        out[i as usize] = with_end_cant(&el, start, value);
        if start {
            carry_cant(&mut out, i - 1, -1, value);
        } else {
            carry_cant(&mut out, i + 1, 1, value);
        }
    }
    following_cants(out)
}
