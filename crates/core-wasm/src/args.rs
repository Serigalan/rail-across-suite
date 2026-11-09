// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The arguments of an operation called as the JavaScript function is
//! called: `input` is the list of its arguments.

use rail_across_geom::element::Utm;
use rail_across_js::{get, js_string, number};
use serde_json::{Value, json};

/// The arguments of one call.
pub struct Args<'a>(pub &'a [Value]);

impl<'a> Args<'a> {
    pub fn new(input: &'a Value) -> Args<'a> {
        Args(input.as_array().map(Vec::as_slice).unwrap_or(&[]))
    }
    /// Argument `i`, null where it was not given.
    pub fn v(&self, i: usize) -> &'a Value {
        self.0.get(i).unwrap_or(&Value::Null)
    }
    /// Argument `i` where it was given and is not null.
    pub fn opt(&self, i: usize) -> Option<&'a Value> {
        self.0.get(i).filter(|v| !v.is_null())
    }
    pub fn f(&self, i: usize) -> f64 {
        self.v(i).as_f64().unwrap_or(f64::NAN)
    }
    pub fn of(&self, i: usize) -> Option<f64> {
        self.opt(i).and_then(Value::as_f64)
    }
    pub fn s(&self, i: usize) -> &'a str {
        self.v(i).as_str().unwrap_or("")
    }
    pub fn os(&self, i: usize) -> Option<&'a str> {
        self.opt(i).and_then(Value::as_str)
    }
    pub fn u(&self, i: usize) -> usize {
        self.v(i).as_f64().unwrap_or(0.0) as usize
    }
    pub fn i(&self, i: usize) -> i64 {
        rail_across_js::js_number(Some(self.v(i))) as i64
    }
    #[allow(dead_code)]
    pub fn b(&self, i: usize) -> bool {
        rail_across_js::truthy(self.opt(i))
    }
    /// Argument `i` as a list (an object's values for a `byId` map).
    pub fn list(&self, i: usize) -> Vec<Value> {
        match self.v(i) {
            Value::Array(a) => a.clone(),
            Value::Object(o) => o.values().cloned().collect(),
            _ => Vec::new(),
        }
    }
    pub fn strings(&self, i: usize) -> Vec<String> {
        self.list(i).iter().map(|v| js_string(Some(v))).collect()
    }
    pub fn utm(&self, i: usize) -> Utm {
        Utm::from_json(self.v(i))
    }
    /// Option `key` of an options object at `i`.
    pub fn option(&self, i: usize, key: &str) -> Option<&'a Value> {
        get(self.v(i), key)
    }
}

pub fn opt_num(x: Option<f64>) -> Value {
    x.map(number).unwrap_or(Value::Null)
}

pub fn opt_str<S: AsRef<str>>(x: Option<S>) -> Value {
    x.map(|s| json!(s.as_ref())).unwrap_or(Value::Null)
}

pub fn opt_list(x: Option<Vec<Value>>) -> Value {
    x.map(Value::Array).unwrap_or(Value::Null)
}
