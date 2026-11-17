// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The stations of a record (`stations.js`; decision 0006): a station or halt
//! is a point of the record's operational point register, an object with an id
//! of its own, and a track or platform names its point by `operationalPointId`.
//! Its name and codes are the point's, stated once.

use rail_across_js::{Object, js_string, js_trim};
use serde_json::{Value, json};

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The point a track or platform of `record` names, where the register has it.
pub fn point_of<'a>(record: &'a Value, obj: &Value) -> Option<&'a Value> {
    let id = obj.get("operationalPointId").and_then(Value::as_str)?;
    list(record, "operationalPoints")
        .iter()
        .find(|p| p.get("id").and_then(Value::as_str) == Some(id))
}

/// `stationRegister(record)` as the face answers it: the points, and the point of each track and platform.
pub fn station_register_json(record: &Value) -> Value {
    let of = |key: &str| -> Vec<Value> {
        list(record, key)
            .iter()
            .map(|o| point_of(record, o).and_then(|p| p.get("id")).cloned().unwrap_or(Value::Null))
            .collect()
    };
    json!({ "points": list(record, "operationalPoints"), "tracks": of("tracks"), "platforms": of("platforms") })
}

/// `text` of stations.js: trimmed, or nothing where empty.
fn text(v: Option<&Value>) -> Option<Value> {
    match v {
        None | Some(Value::Null) => None,
        Some(x) => {
            let s = js_trim(&js_string(Some(x))).to_string();
            if s.is_empty() { None } else { Some(Value::String(s)) }
        }
    }
}

fn put(o: &mut Object, key: &str, v: Option<Value>) {
    match v {
        Some(v) => {
            o.insert(key.into(), v);
        }
        None => {
            o.shift_remove(key);
        }
    }
}

/// The register with the station `id` renamed — `name`, and its `uic` number
/// where given —, keeping its id (`renamedStation`): { operationalPoints }, or
/// null where the record has no such station. Its tracks and platforms name it
/// by its id, so they stay as they are.
pub fn renamed_station(record: &Value, id: &str, name: Option<&Value>, uic: Option<Option<&Value>>) -> Value {
    let points = list(record, "operationalPoints");
    if !points.iter().any(|p| p.get("id").and_then(Value::as_str) == Some(id)) {
        return Value::Null;
    }
    let renamed: Vec<Value> = points
        .iter()
        .map(|p| {
            if p.get("id").and_then(Value::as_str) != Some(id) {
                return p.clone();
            }
            let mut o = p.as_object().cloned().unwrap_or_default();
            put(&mut o, "name", text(name));
            if let Some(u) = uic {
                put(&mut o, "uic", text(u));
            }
            Value::Object(o)
        })
        .collect();
    json!({ "operationalPoints": renamed })
}

/// The point called `name` (`stationPoint`): the record's where it has one of that
/// name whose codes do not say otherwise, else a new one with the codes given —
/// { operationalPoints, id }, the register with it. `None` where nothing names a point.
pub fn station_point(record: &Value, name: Option<&Value>, uic: Option<&Value>, rl100: Option<&Value>) -> Value {
    let (name, uic, rl100) = (text(name), text(uic), text(rl100));
    if name.is_none() && uic.is_none() && rl100.is_none() {
        return Value::Null;
    }
    let points = list(record, "operationalPoints");
    let fits = |p: &Value, k: &str, v: &Option<Value>| v.is_none() || text(p.get(k)).is_none() || text(p.get(k)) == *v;
    let found = points.iter().find(|p| {
        let same = match &name {
            Some(n) => text(p.get("name")).is_some_and(|m| js_string(Some(&m)).to_lowercase() == js_string(Some(n)).to_lowercase()),
            None => uic.is_some() && text(p.get("uic")) == uic || rl100.is_some() && text(p.get("rl100")) == rl100,
        };
        same && fits(p, "uic", &uic) && fits(p, "rl100", &rl100)
    });
    if let Some(p) = found {
        return json!({ "operationalPoints": points, "id": p.get("id") });
    }
    let id = rail_across_js::generate_id();
    let mut o = Object::new();
    o.insert("id".into(), json!(id));
    put(&mut o, "name", name);
    put(&mut o, "rl100", rl100);
    put(&mut o, "uic", uic);
    let mut all = points.to_vec();
    all.push(Value::Object(o));
    json!({ "operationalPoints": all, "id": id })
}
