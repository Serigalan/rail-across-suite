// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What infra-data-2.0's layers ask of the stopping places (decision 0008): they
//! are the core's, a platform (civil) names those at it, passenger stops by that,
//! and a station boundary is marked by a stopping place where it was marked by a
//! platform. The record holds them as the document does (decision 0006); the
//! door's step from infra-data 1.0 and the Civil-Tool's platforms go through here.

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
/// `operationalPointId`, `lifespan`).
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
                                // A platform of infra-data 1.0 named its point by `operationalPoint`.
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

/// Whether a domain's fields and collections hold anything — one that holds nothing is written as missing.
pub fn holds_anything(fields: &Object, collections: &Object) -> bool {
    !fields.is_empty() || collections.values().any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
}

/// How far apart two stations may be and still be one [m].
const TOL: f64 = 0.01;

fn num(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// Whether the stopping place `h` lies at the platform `p` (the record's): on its track, overlapping it.
fn at_platform(h: &Value, p: &Value) -> bool {
    let at = &h["at"];
    if at.get("trackId").is_none() || at.get("trackId") != p.get("trackId") {
        return false;
    }
    let (a, b) = (num(p.get("startStation")), num(p.get("endStation")));
    let overlap = num(at.get("endStation")).min(a.max(b)) - num(at.get("startStation")).max(a.min(b));
    overlap > TOL
}

/// The ids of the stopping places a platform names.
fn named_by(p: &Value) -> Vec<String> {
    get(p, "stoppingPlaceIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|x| js_string(Some(x)))
        .collect()
}

/// The record with the stopping places at the platform `platform_id` as the
/// Civil-Tool sets them where it puts, moves or deletes a platform (decision
/// 0008): every stopping place at it a passenger stop, named by it
/// (`stoppingPlaceIds`) — unless another platform names it already —; one over
/// the platform, of its station and stage, called `name`, where none is at it
/// yet; one it named that is no longer at it named by the platform it is at now,
/// else no passenger stop — which holds for every passenger stop no platform
/// names, as after a platform was deleted. A stopping place is never deleted: it
/// is one for every train, the Signalling-Tool's as much (decision 0008).
pub fn with_platform_stops(record: &Value, platform_id: &str, name: Option<&str>) -> Value {
    let mut platforms: Vec<Value> = record.get("platforms").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut places: Vec<Value> = record.get("stoppingPlaces").and_then(Value::as_array).cloned().unwrap_or_default();
    let platform = platforms.iter().position(|p| js_string(p.get("id")) == platform_id);
    let before = (platforms.clone(), places.clone());
    let set_ids = |p: &mut Value, ids: Vec<String>| {
        let o = p.as_object_mut().expect("a platform");
        if ids.is_empty() {
            o.shift_remove("stoppingPlaceIds");
        } else {
            o.insert("stoppingPlaceIds".into(), json!(ids));
        }
    };
    let mut at_it = false;
    for h in places.iter_mut() {
        let id = js_string(h.get("id"));
        let namer = platforms.iter().position(|p| named_by(p).contains(&id));
        if platform.is_some_and(|i| at_platform(h, &platforms[i])) {
            at_it = true;
            h["passengerStop"] = json!(true);
            // Named by another platform there is, it stays so.
            if namer.is_none() {
                let i = platform.expect("the platform");
                let mut ids = named_by(&platforms[i]);
                ids.push(id);
                set_ids(&mut platforms[i], ids);
            }
            continue;
        }
        if namer.is_some_and(|i| Some(i) == platform) {
            let i = platform.expect("the platform");
            let ids: Vec<String> = named_by(&platforms[i]).into_iter().filter(|x| *x != id).collect();
            set_ids(&mut platforms[i], ids);
        } else if namer.is_some() || get(h, "passengerStop") != Some(&json!(true)) {
            continue;
        }
        // A passenger stop no platform names: the platform it lies at names it, else it is none.
        match platforms.iter().position(|p| js_string(p.get("id")) != platform_id && at_platform(h, p)) {
            Some(j) => {
                let mut ids = named_by(&platforms[j]);
                ids.push(id);
                set_ids(&mut platforms[j], ids);
            }
            None => {
                if let Some(o) = h.as_object_mut() {
                    o.shift_remove("passengerStop");
                }
            }
        }
    }
    if let Some(i) = platform.filter(|_| !at_it) {
        let p = &platforms[i];
        let (a, b) = (num(p.get("startStation")), num(p.get("endStation")));
        let point = crate::stations::point_of(record, p)
            .and_then(|o| o.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let id = rail_across_js::generate_id();
        let mut h = Object::new();
        h.insert("id".into(), json!(id));
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
        h.insert("passengerStop".into(), json!(true));
        if let Some(lifespan) = get(p, "lifespan") {
            h.insert("lifespan".into(), lifespan.clone());
        }
        places.push(Value::Object(h));
        let mut ids = named_by(&platforms[i]);
        ids.push(id);
        set_ids(&mut platforms[i], ids);
    }
    if (platforms.clone(), places.clone()) == before {
        return record.clone();
    }
    let mut out = obj(record);
    out.insert("platforms".into(), Value::Array(platforms));
    out.insert("stoppingPlaces".into(), Value::Array(places));
    Value::Object(out)
}
