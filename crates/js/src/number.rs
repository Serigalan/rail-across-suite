// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Numbers as ECMAScript writes them (`Number.prototype.toString`, which
//! `JSON.stringify` and RFC 8785 use): the shortest digits that read back as
//! the same double — Rust's own shortest formatting gives them — laid out by
//! the rules of ECMA-262 §6.1.6.1.20.

use serde_json::{Number, Value};

/// `x` as ECMAScript writes it: `100`, `0.1`, `1e-7`, `1e+21`, `-0` as `0`.
pub fn es_number(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // "d.ddde±x": the shortest round-trip digits and their exponent.
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().expect("exponent") + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let e_sign = if e < 0 { "-" } else { "+" };
        if k == 1 {
            format!("{digits}e{e_sign}{}", e.abs())
        } else {
            format!("{}.{}e{e_sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    format!("{sign}{body}")
}

/// `x` as a JSON number: an integer where it is one that serde_json holds
/// exactly, so it is written without a fraction — as JavaScript would. A
/// number that is not finite becomes `null`, as JSON has no other.
pub fn number_value(x: f64) -> Value {
    if !x.is_finite() {
        return Value::Null;
    }
    if x == x.trunc() && x.abs() < 9.007_199_254_740_992e15 {
        let i = x as i64;
        if i == 0 {
            return Value::Number(Number::from(0));
        }
        return Value::Number(Number::from(i));
    }
    Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null)
}
