// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reading the records as the JavaScript reads them.

use rail_across_js::{Object, get};
use serde_json::Value;

/// An object's members, cloned (none for anything else).
pub fn obj(v: &Value) -> Object {
    v.as_object().cloned().unwrap_or_default()
}

/// `v[key] ?? []` as a list.
pub fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    get(v, key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// `v[key]` as a number where it is one (null and absent: none).
pub fn num(v: &Value, key: &str) -> Option<f64> {
    get(v, key).and_then(Value::as_f64)
}

/// `v[key]` as a string where it is one.
pub fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    get(v, key).and_then(Value::as_str)
}

/// `{ ...v, key: value }`.
pub fn with(v: &Value, key: &str, value: Value) -> Value {
    let mut o = obj(v);
    o.insert(key.to_string(), value);
    Value::Object(o)
}

/// `v` without `keys`.
pub fn without(v: &Value, keys: &[&str]) -> Object {
    let mut o = obj(v);
    for k in keys {
        o.shift_remove(*k);
    }
    o
}

/// A node `[e, n]` where both are numbers.
pub fn node(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v?.as_array()?;
    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])
}

/// `Number(track.epsg)` as the conversions take it.
pub fn epsg(track: &Value) -> i64 {
    rail_across_js::js_number(track.get("epsg")) as i64
}

/// `els.length` elements of a track.
pub fn elements(track: &Value) -> &[Value] {
    list(track, "elements")
}
