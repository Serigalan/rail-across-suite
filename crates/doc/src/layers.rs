// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What infra-data-2.0's layers ask of the stopping places (decision 0025): they
//! are the core's, a platform (civil) names those at it, passenger stops by that,
//! and a station boundary is marked by a stopping place where it was marked by a
//! platform. The door's steps and the record's way into a document go through here.

use rail_across_js::{Object, get, js_string};
use serde_json::{Value, json};

use crate::canonical::derived_id;

/// The signalling domain's collections that must be there whenever it is (signalling 2.0).
pub const SIGNALLING_COLLECTIONS: &[&str] = &[
    "signals",
    "detectionPoints",
    "trackSections",
    "switchElements",
    "routes",
    "overlaps",
    "flankProtection",
    "balises",
    "stationBoundaries",
    "itineraries",
];

fn obj(v: &Value) -> Object {
    v.as_object().cloned().unwrap_or_default()
}

/// The id of the stopping place a boundary made of platform `platform` is marked by, where none is at it yet.
fn made_id(platform: &str) -> String {
    derived_id(&format!("stopping place at platform {platform}"))
}

/// The marks of the station boundaries in `signalling` (its collections) that name a
/// platform, marked by the stopping place at it instead: the first of `stopping` that
/// names it (`platformId`, the record's form), or one made over the platform where
/// none does — pushed onto `stopping`. `platforms` are in the document's form (`at`,
/// `operationalPoint`, `lifespan`).
pub fn boundaries_by_stopping_place(signalling: &mut Object, stopping: &mut Vec<Value>, platforms: &[Value]) {
    let Some(Value::Array(boundaries)) = signalling.get("stationBoundaries").cloned() else {
        return;
    };
    let mut changed = false;
    let boundaries: Vec<Value> = boundaries
        .iter()
        .map(|b| {
            let Some(Value::Array(marks)) = b.get("marks") else { return b.clone() };
            if !marks.iter().any(|m| m.get("platformId").is_some()) {
                return b.clone();
            }
            changed = true;
            let marks: Vec<Value> = marks
                .iter()
                .map(|m| {
                    let Some(pid) = m.get("platformId").map(|v| js_string(Some(v))) else {
                        return m.clone();
                    };
                    let mut at_it: Vec<&Value> = stopping
                        .iter()
                        .filter(|h| get(h, "platformId").map(|v| js_string(Some(v))) == Some(pid.clone()))
                        .collect();
                    at_it.sort_by_key(|h| js_string(h.get("id")));
                    let id = match at_it.first() {
                        Some(h) => js_string(h.get("id")),
                        None => {
                            let id = made_id(&pid);
                            let platform = platforms.iter().find(|p| js_string(p.get("id")) == pid);
                            let mut h = Object::new();
                            h.insert("id".into(), json!(id));
                            if let Some(p) = platform {
                                for (k, to) in [("operationalPoint", "operationalPointId"), ("at", "at"), ("lifespan", "lifespan")] {
                                    if let Some(v) = get(p, k) {
                                        h.insert(to.into(), v.clone());
                                    }
                                }
                            }
                            h.insert("platformId".into(), json!(pid));
                            stopping.push(Value::Object(h));
                            id
                        }
                    };
                    json!({ "stoppingPlaceId": id })
                })
                .collect();
            let mut o = obj(b);
            o.insert("marks".into(), Value::Array(marks));
            Value::Object(o)
        })
        .collect();
    if changed {
        signalling.insert("stationBoundaries".into(), Value::Array(boundaries));
    }
}

/// The platforms (document's form) each naming the stopping places at it, from the
/// stopping places' `platformId` (the record's form).
pub fn platforms_naming(platforms: &[Value], stopping: &[Value]) -> Vec<Value> {
    platforms
        .iter()
        .map(|p| {
            let pid = js_string(p.get("id"));
            let mut ids: Vec<String> = stopping
                .iter()
                .filter(|h| get(h, "platformId").map(|v| js_string(Some(v))) == Some(pid.clone()))
                .map(|h| js_string(h.get("id")))
                .collect();
            if ids.is_empty() {
                return p.clone();
            }
            ids.sort();
            let mut o = obj(p);
            o.insert("stoppingPlaceIds".into(), json!(ids));
            Value::Object(o)
        })
        .collect()
}

/// The stopping places in the core's form: no `platformId`, and one that named
/// a platform a passenger stop.
pub fn core_stopping_places(stopping: &[Value]) -> Vec<Value> {
    stopping
        .iter()
        .map(|h| {
            let mut o = obj(h);
            if o.shift_remove("platformId").is_some() {
                o.insert("passengerStop".into(), json!(true));
            }
            Value::Object(o)
        })
        .collect()
}

/// The stopping places (core's form) with each that one of `platforms` (document's
/// form) names a passenger stop.
pub fn passenger_stops(stopping: &[Value], platforms: &[Value]) -> Vec<Value> {
    let named: std::collections::HashSet<String> = platforms
        .iter()
        .flat_map(|p| get(p, "stoppingPlaceIds").and_then(Value::as_array).cloned().unwrap_or_default())
        .map(|x| js_string(Some(&x)))
        .collect();
    stopping
        .iter()
        .map(|h| {
            if !named.contains(&js_string(h.get("id"))) {
                return h.clone();
            }
            let mut o = obj(h);
            o.insert("passengerStop".into(), json!(true));
            Value::Object(o)
        })
        .collect()
}

/// The stopping places in the record's form: each with the platform that names it.
pub fn record_stopping_places(stopping: &[Value], platforms: &[Value]) -> Vec<Value> {
    stopping
        .iter()
        .map(|h| {
            let id = js_string(h.get("id"));
            let platform = platforms.iter().find(|p| {
                get(p, "stoppingPlaceIds")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| ids.iter().any(|x| js_string(Some(x)) == id))
            });
            let mut o = obj(h);
            if let Some(p) = platform {
                o.insert("platformId".into(), p.get("id").cloned().unwrap_or(Value::Null));
            }
            Value::Object(o)
        })
        .collect()
}

/// Whether a domain's fields and collections hold anything — one that holds nothing is written as missing.
pub fn holds_anything(fields: &Object, collections: &Object) -> bool {
    !fields.is_empty() || collections.values().any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
}

/// How far apart two stations may be and still be one [m].
const TOL: f64 = 0.01;

fn num(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// Whether the stopping place `h` (record's form) lies at the platform `p` (record's form): on its track, overlapping it.
fn at_platform(h: &Value, p: &Value) -> bool {
    let at = &h["at"];
    if at.get("trackId").is_none() || at.get("trackId") != p.get("trackId") {
        return false;
    }
    let (a, b) = (num(p.get("startStation")), num(p.get("endStation")));
    let overlap = num(at.get("endStation")).min(a.max(b)) - num(at.get("startStation")).max(a.min(b));
    overlap > TOL
}

/// The record (its form) with the stopping places at the platform `platform_id` as
/// the Civil-Tool sets them where it puts, moves or deletes a platform (decision
/// 0025): every stopping place at it a passenger stop, named by it — unless another
/// platform names it already —; one over the platform, of its station and stage,
/// called `name`, where none is at it yet; one it named that is no longer at it
/// named by the platform it is at now, else neither named nor a passenger stop. A
/// stopping place is never deleted: it is one for every train, the
/// Signalling-Tool's as much (decision 0024).
pub fn with_platform_stops(record: &Value, platform_id: &str, name: Option<&str>) -> Value {
    let platforms: Vec<Value> = record.get("platforms").and_then(Value::as_array).cloned().unwrap_or_default();
    let ids: Vec<String> = platforms.iter().map(|p| js_string(p.get("id"))).collect();
    let platform = platforms.iter().position(|p| js_string(p.get("id")) == platform_id);
    let mut places: Vec<Value> = record["signalling"]["collections"]
        .get("stoppingPlaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;
    let mut at_it = false;
    for h in places.iter_mut() {
        let names = get(h, "platformId").map(|v| js_string(Some(v)));
        let mine = platform.is_some_and(|i| at_platform(h, &platforms[i]));
        let mut o = obj(h);
        if mine {
            at_it = true;
            o.insert("passengerStop".into(), json!(true));
            // Named by another platform there is, it stays so.
            if names.as_deref() != Some(platform_id) && !names.as_ref().is_some_and(|n| ids.contains(n)) {
                o.insert("platformId".into(), json!(platform_id));
            }
        } else if names.as_deref() == Some(platform_id) {
            match platforms.iter().find(|p| js_string(p.get("id")) != platform_id && at_platform(h, p)) {
                Some(other) => {
                    o.insert("platformId".into(), other.get("id").cloned().unwrap_or(Value::Null));
                }
                None => {
                    o.shift_remove("platformId");
                    o.shift_remove("passengerStop");
                }
            }
        }
        let next = Value::Object(o);
        if next != *h {
            *h = next;
            changed = true;
        }
    }
    if let Some(i) = platform.filter(|_| !at_it) {
        let p = &platforms[i];
        let (a, b) = (num(p.get("startStation")), num(p.get("endStation")));
        let pid = js_string(record.get("id"));
        let final_id = js_string(crate::core_format::default_stages(&pid)[1].get("id"));
        let point = crate::stations::station_register(record)
            .point_of(crate::core_format::Source::Platform(i))
            .map(str::to_string);
        let mut h = Object::new();
        h.insert("id".into(), json!(rail_across_js::generate_id()));
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            h.insert("name".into(), json!(n));
        }
        if let Some(point) = point {
            h.insert("operationalPointId".into(), json!(point));
        }
        h.insert(
            "at".into(),
            json!({ "anchor": "station", "trackId": p.get("trackId"), "startStation": a.min(b), "endStation": a.max(b) }),
        );
        h.insert("platformId".into(), json!(platform_id));
        h.insert("passengerStop".into(), json!(true));
        let lifespan = crate::core_format::lifespan_of(p.get("status"), &final_id);
        if !lifespan.is_null() {
            h.insert("lifespan".into(), lifespan);
        }
        places.push(Value::Object(h));
        changed = true;
    }
    if !changed {
        return record.clone();
    }
    let mut signalling = obj(record.get("signalling").unwrap_or(&Value::Null));
    let mut cols = obj(signalling.get("collections").unwrap_or(&Value::Null));
    cols.insert("stoppingPlaces".into(), Value::Array(places));
    signalling.entry("fields").or_insert(json!({}));
    signalling.insert("collections".into(), Value::Object(cols));
    let mut out = obj(record);
    out.insert("signalling".into(), Value::Object(signalling));
    Value::Object(out)
}
