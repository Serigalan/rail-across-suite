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

mod ids;
mod number;

pub use ids::{generate_id, queue_ids, seed_ids};
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

/// `x.toFixed(digits)`: the exact decimal value of the double rounded to
/// `digits` places, a tie to the larger magnitude (ECMA-262 §21.1.3.3) —
/// not to even, as Rust's own formatting rounds. Past 1e21 as `String(x)`.
pub fn js_to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return es_number(x);
    }
    if x.abs() >= 1e21 {
        return es_number(x);
    }
    let negative = x < 0.0;
    // Exact digits well past the place rounded at: a tie is a 5 and zeros.
    let exact = format!("{:.*}", digits + 40, x.abs());
    let (int_part, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let kept = &frac[..digits];
    let rest = &frac[digits..];
    let up = rest.as_bytes().first().is_some_and(|d| *d >= b'5');
    let mut all: Vec<u8> = format!("{int_part}{kept}").into_bytes();
    if up {
        let mut i = all.len();
        loop {
            if i == 0 {
                all.insert(0, b'1');
                break;
            }
            i -= 1;
            if all[i] == b'9' {
                all[i] = b'0';
            } else {
                all[i] += 1;
                break;
            }
        }
    }
    let text = String::from_utf8(all).expect("digits");
    let (i, f) = text.split_at(text.len() - digits);
    let i = if i.is_empty() { "0" } else { i };
    let body = if digits == 0 { i.to_string() } else { format!("{i}.{f}") };
    let zero = body.chars().all(|c| c == '0' || c == '.');
    if negative && !zero { format!("-{body}") } else { body }
}

/// `Number(value)` — JavaScript's conversion: a string trimmed and read as a
/// decimal (or `0x…`, `Infinity`), empty as 0; `null` and `false` 0, `true`
/// 1; anything else, or an absent value, NaN.
pub fn js_number(value: Option<&Value>) -> f64 {
    match value {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => {
            let t = js_trim(s);
            if t.is_empty() {
                return 0.0;
            }
            match t {
                "Infinity" | "+Infinity" => return f64::INFINITY,
                "-Infinity" => return f64::NEG_INFINITY,
                _ => {}
            }
            if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                return u64::from_str_radix(hex, 16).map(|v| v as f64).unwrap_or(f64::NAN);
            }
            let ok = t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'));
            if !ok || t.contains("inf") || t.contains("nan") {
                return f64::NAN;
            }
            t.parse::<f64>().unwrap_or(f64::NAN)
        }
        Some(Value::Array(a)) => match a.len() {
            0 => 0.0,
            1 => js_number(Some(&Value::String(js_string(Some(&a[0]))))),
            _ => f64::NAN,
        },
        Some(Value::Object(_)) => f64::NAN,
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

/// `Math.hypot(...values)` as V8 computes it: each value divided by the
/// largest, the squares summed with Kahan's compensation, the root scaled
/// back — not the libm `hypot`, which may differ in the last place.
pub fn js_hypot(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut max = 0.0f64;
    let mut nan = false;
    for v in values {
        if v.is_nan() {
            nan = true;
        } else if v.abs() > max {
            max = v.abs();
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0f64, 0.0f64);
    for v in values {
        let n = v.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
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

    #[test]
    fn to_fixed_takes_the_larger_of_a_tie() {
        assert_eq!(js_to_fixed(0.0625, 3), "0.063");
        assert_eq!(js_to_fixed(1.005, 2), "1.00");
        assert_eq!(js_to_fixed(-2.5, 0), "-3");
        assert_eq!(js_to_fixed(123.456, 1), "123.5");
        assert_eq!(js_to_fixed(-0.0001, 3), "0.000");
    }
}
