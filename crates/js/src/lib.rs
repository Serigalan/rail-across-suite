// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! JSON values with the meaning JavaScript gives them — what the core's crates
//! need to answer exactly as the JavaScript core they replace (decision 0020):
//! a number written as ECMAScript writes it, a string escaped as
//! `JSON.stringify` escapes it, canonical JSON after RFC 8785, keys compared by
//! their UTF-16 code units, and the reading of a member that may be absent,
//! `null` or there (`??`, `!= null`, `Object.hasOwn`, truthiness, `String()`).
//!
//! Objects keep the order their members were made in (serde_json's
//! `preserve_order`), as JavaScript objects do for names that are no index.

use std::cmp::Ordering;

use serde_json::{Map, Number, Value};

mod number;

pub use number::{es_number, number_value};

/// An object, as serde_json holds one.
pub type Object = Map<String, Value>;

/// Member `key` of `value` where it is there and not `null` — JavaScript's `value[key] ?? …`.
pub fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value.get(key) {
        Some(Value::Null) | None => None,
        found => found,
    }
}

/// Whether `value` has a member `key` of its own, `null` or not — `Object.hasOwn`.
pub fn has(value: &Value, key: &str) -> bool {
    value.as_object().is_some_and(|o| o.contains_key(key))
}

/// Member `key` as a number, where it is one.
pub fn num(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

/// Member `key` as a string, where it is one.
pub fn str_of<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// JavaScript's truthiness of a value that may be absent.
pub fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `String(value)` — `undefined` for an absent one.
pub fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => es_number(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| if v.is_null() { String::new() } else { js_string(Some(v)) })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// JavaScript's `===` on two values that may be absent: numbers by value,
/// strings, booleans and `null` alike; an array or an object only equals itself,
/// which two values handed over never are.
pub fn strict_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Value::Null), Some(Value::Null)) => true,
        (Some(Value::Bool(x)), Some(Value::Bool(y))) => x == y,
        (Some(Value::Number(x)), Some(Value::Number(y))) => x.as_f64() == y.as_f64(),
        (Some(Value::String(x)), Some(Value::String(y))) => x == y,
        _ => false,
    }
}

/// Whether `c` is white space as `String.prototype.trim` takes it (WhiteSpace and LineTerminator).
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// `s.trim()` as JavaScript trims.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `Math.round(x)`: to the nearest integer, a half up (toward +∞).
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// JavaScript's `a < b` on two values (strings by UTF-16, numbers by value; otherwise false).
pub fn js_less(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(Value::String(x)), Some(Value::String(y))) => cmp_utf16(x, y) == Ordering::Less,
        (Some(Value::Number(x)), Some(Value::Number(y))) => as_f64(x) < as_f64(y),
        _ => false,
    }
}

/// Two strings compared by their UTF-16 code units — JavaScript's `<` and `sort()`.
pub fn cmp_utf16(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// A string as `JSON.stringify` writes it, quotes included.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Why a value has no JSON text.
#[derive(Debug, Clone, PartialEq)]
pub struct NoJson(pub String);

/// The canonical JSON text of `value` (RFC 8785): members sorted by their
/// names' UTF-16 code units, numbers as ECMAScript writes them (`-0` as `0`),
/// strings escaped as `JSON.stringify` escapes them, no whitespace.
pub fn canonical_json(value: &Value) -> Result<String, NoJson> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(value: &Value, out: &mut String) -> Result<(), NoJson> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if !x.is_finite() {
                return Err(NoJson(format!("no JSON for {}", es_number(x))));
            }
            out.push_str(&es_number(x));
        }
        Value::String(s) => out.push_str(&json_string(s)),
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(v, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| cmp_utf16(a, b));
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(k));
                out.push(':');
                write_canonical(&map[k.as_str()], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// A value as `JSON.stringify` writes it (no whitespace): members in their
/// order, numbers as ECMAScript writes them, a number that is not finite `null`.
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_plain(value, &mut out);
    out
}

fn write_plain(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            out.push_str(&if x.is_finite() { es_number(x) } else { "null".to_string() });
        }
        Value::String(s) => out.push_str(&json_string(s)),
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_plain(v, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(k));
                out.push(':');
                write_plain(v, out);
            }
            out.push('}');
        }
    }
}

/// A number as a JSON value — an integer where it is one, as JavaScript would print it.
pub fn number(x: f64) -> Value {
    number_value(x)
}

/// `obj` without its members that are `null` — `withoutNull` of the JavaScript.
pub fn without_null(obj: Object) -> Object {
    obj.into_iter().filter(|(_, v)| !v.is_null()).collect()
}

/// A JSON number's value as JavaScript holds it.
pub fn as_f64(n: &Number) -> f64 {
    n.as_f64().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_are_written_as_ecmascript_writes_them() {
        for (x, text) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (100.0, "100"),
            (0.1, "0.1"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123456.789, "123456.789"),
            (-2.5e-10, "-2.5e-10"),
            (5600000.1234568, "5600000.1234568"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
        ] {
            assert_eq!(es_number(x), text, "{x}");
        }
    }

    #[test]
    fn canonical_json_sorts_by_utf16_and_escapes_as_json_stringify() {
        let v = json!({ "b": 1, "a": [true, null, "x\u{1}\n\"é"], "\u{e9}": 0.5, "Z": -0.0 });
        assert_eq!(
            canonical_json(&v).unwrap(),
            "{\"Z\":0,\"a\":[true,null,\"x\\u0001\\n\\\"é\"],\"b\":1,\"é\":0.5}"
        );
    }

    #[test]
    fn strings_compare_by_utf16_code_units() {
        // U+FF5E sorts after U+1F600 in UTF-8 but before it in UTF-16.
        assert_eq!(cmp_utf16("\u{1F600}", "\u{FF5E}"), Ordering::Less);
        assert_eq!(cmp_utf16("a", "b"), Ordering::Less);
    }

    #[test]
    fn string_coercion_is_javascripts() {
        assert_eq!(js_string(Some(&json!(12.0))), "12");
        assert_eq!(js_string(Some(&json!(0.30000000000000004))), "0.30000000000000004");
        assert_eq!(js_string(None), "undefined");
    }
}
