// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The stations of a record by their ids (`stations.js`): a station or halt is
//! a point of the operational point register, and its id is what names it.

use rail_across_js::{Object, js_string, js_trim};
use serde_json::{Value, json};

use crate::core_format::{Register, Source, operational_register};

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The register of a record (`stationRegister`).
pub fn station_register(record: &Value) -> Register {
    operational_register(
        record.get("id"),
        list(record, "tracks"),
        list(record, "platforms"),
        list(record, "operationalPoints"),
    )
}

/// `stationRegister(record)` as the face answers it: the points, and the point of each track and platform.
pub fn station_register_json(record: &Value) -> Value {
    let r = station_register(record);
    let tracks: Vec<Value> = (0..list(record, "tracks").len())
        .map(|i| r.point_of(Source::Track(i)).map(|s| json!(s)).unwrap_or(Value::Null))
        .collect();
    let platforms: Vec<Value> = (0..list(record, "platforms").len())
        .map(|i| r.point_of(Source::Platform(i)).map(|s| json!(s)).unwrap_or(Value::Null))
        .collect();
    json!({ "points": r.points, "tracks": tracks, "platforms": platforms })
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

/// The record with the station `id` renamed, keeping its id (`renamedStation`):
/// { tracks, platforms, operationalPoints }, or null where it has no such station.
pub fn renamed_station(record: &Value, id: &str, name: Option<&Value>, uic: Option<Option<&Value>>) -> Value {
    let register = station_register(record);
    let Some(point) = register.points.iter().find(|p| p.get("id").and_then(Value::as_str) == Some(id)) else {
        return Value::Null;
    };
    let mut next = Object::new();
    next.insert("id".into(), json!(id));
    put(&mut next, "name", text(name));
    put(&mut next, "rl100", point.get("rl100").cloned());
    let uic = match uic {
        None => point.get("uic").cloned(),
        Some(u) => text(u),
    };
    put(&mut next, "uic", uic);
    let next = Value::Object(next);
    let tracks: Vec<Value> = list(record, "tracks")
        .iter()
        .enumerate()
        .map(|(i, tr)| {
            if register.point_of(Source::Track(i)) != Some(id) {
                return tr.clone();
            }
            let mut o = tr.as_object().cloned().unwrap_or_default();
            put(&mut o, "stationName", next.get("name").cloned());
            put(&mut o, "uicStation", next.get("uic").cloned());
            Value::Object(o)
        })
        .collect();
    let platforms: Vec<Value> = list(record, "platforms")
        .iter()
        .enumerate()
        .map(|(i, pf)| {
            if register.point_of(Source::Platform(i)) != Some(id) {
                return pf.clone();
            }
            let mut o = pf.as_object().cloned().unwrap_or_default();
            put(&mut o, "stationName", next.get("name").cloned());
            Value::Object(o)
        })
        .collect();
    let kept = list(record, "operationalPoints");
    let ops: Vec<Value> = if kept.iter().any(|p| p.get("id").and_then(Value::as_str) == Some(id)) {
        kept.iter()
            .map(|p| {
                if p.get("id").and_then(Value::as_str) == Some(id) {
                    next.clone()
                } else {
                    p.clone()
                }
            })
            .collect()
    } else {
        let mut v = kept.to_vec();
        v.push(next.clone());
        v
    };
    json!({ "tracks": tracks, "platforms": platforms, "operationalPoints": ops })
}
