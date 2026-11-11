// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What infra-data-2.0's layers ask of the stopping places (decision 0025): they
//! are the core's, a platform (civil) names those at it, and a station boundary is
//! marked by a stopping place where it was marked by a platform. The door's step
//! from 1.0 and the record's way into a document both go through here.

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
    let Some(Value::Array(boundaries)) = signalling.get("stationBoundaries").cloned() else { return };
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
                    let Some(pid) = m.get("platformId").map(|v| js_string(Some(v))) else { return m.clone() };
                    let mut at_it: Vec<&Value> = stopping.iter().filter(|h| get(h, "platformId").map(|v| js_string(Some(v))) == Some(pid.clone())).collect();
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

/// The stopping places in the core's form: no `platformId`.
pub fn core_stopping_places(stopping: &[Value]) -> Vec<Value> {
    stopping
        .iter()
        .map(|h| {
            let mut o = obj(h);
            o.shift_remove("platformId");
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
                get(p, "stoppingPlaceIds").and_then(Value::as_array).is_some_and(|ids| ids.iter().any(|x| js_string(Some(x)) == id))
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
