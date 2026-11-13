// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Track ends: buffer stops and boundaries (`trackEndMarks.js`).

use std::collections::HashSet;

use rail_across_js::{generate_id, get, js_number, js_string, js_trim, number, truthy};
use serde_json::{Value, json};

use crate::switch_model::{port_end, port_track, ports_of};
use crate::util::{obj, text, with};

pub const BUFFER_STOP: &str = "buffer_stop";
pub const BOUNDARY: &str = "boundary";
pub const BUFFER_STOP_TYPES: [i64; 4] = [4, 6, 8, 10];
pub const DEFAULT_BUFFER_STOP_TYPE: i64 = 4;
pub const BUFFER_STOP_LENGTH: f64 = 2.2;

pub fn default_brake_length(t: &Value) -> f64 {
    js_number(Some(t))
}

/// `${trackId}|${endpoint}` (`endKey`).
pub fn end_key(track_id: Option<&Value>, endpoint: Option<&Value>) -> String {
    format!("{}|{}", js_string(track_id), js_string(endpoint))
}

fn flip(e: &Value) -> Value {
    match e.as_str() {
        Some("BEGIN") => json!("END"),
        Some("END") => json!("BEGIN"),
        _ => e.clone(),
    }
}

/// A buffer stop at a track end (`newBufferStop`).
pub fn new_buffer_stop(track_id: &Value, endpoint: &Value, kind: Option<&Value>, brake: Option<f64>) -> Value {
    let t = kind.cloned().unwrap_or(json!(DEFAULT_BUFFER_STOP_TYPE));
    let brake = brake.unwrap_or_else(|| default_brake_length(&t));
    json!({ "id": generate_id(), "kind": BUFFER_STOP, "trackId": track_id, "endpoint": endpoint, "type": t, "brakeLength": number(brake) })
}

/// A boundary at a track end (`newBoundary`).
pub fn new_boundary(track_id: &Value, endpoint: &Value, speed: Option<&Value>) -> Value {
    let mut o = rail_across_js::Object::new();
    o.insert("id".into(), json!(generate_id()));
    o.insert("kind".into(), json!(BOUNDARY));
    o.insert("trackId".into(), track_id.clone());
    o.insert("endpoint".into(), endpoint.clone());
    if let Some(s) = speed.filter(|s| !s.is_null()) {
        o.insert("speed".into(), s.clone());
    }
    Value::Object(o)
}

/// What a mark is called (`endMarkName`), its track's station the name of
/// `point`, the operational point the track names.
pub fn end_mark_name(mark: &Value, track: &Value, point: Option<&Value>) -> String {
    let own = js_trim(&get(mark, "name").map(|v| js_string(Some(v))).unwrap_or_default()).to_string();
    if !own.is_empty() {
        return own;
    }
    let parts: Vec<String> = [
        point.and_then(|p| get(p, "name")).filter(|v| truthy(Some(v))).map(|v| js_string(Some(v))),
        get(track, "trackNumber")
            .filter(|v| truthy(Some(v)))
            .map(|v| format!("Gl. {}", js_string(Some(v)))),
    ]
    .into_iter()
    .flatten()
    .collect();
    let station = parts.join(" ");
    let name = js_trim(&get(track, "name").map(|v| js_string(Some(v))).unwrap_or_default()).to_string();
    let where_ = if !name.is_empty() {
        name
    } else if !station.is_empty() {
        station
    } else {
        get(mark, "trackId").map(|v| js_string(Some(v))).unwrap_or_default().chars().take(8).collect()
    };
    let kind = if text(mark, "kind") == Some(BOUNDARY) { "Grenze" } else { "Prellbock" };
    let end = if text(mark, "endpoint") == Some("BEGIN") { "Anfang" } else { "Ende" };
    format!("{kind} {where_} {end}")
}

/// Does a buffer stop with this brake length fit on the track (`bufferStopFits`)?
pub fn buffer_stop_fits(track_length: f64, brake: f64) -> bool {
    brake.is_finite() && brake >= 0.0 && track_length + 1e-9 >= brake + BUFFER_STOP_LENGTH
}

/// Where a buffer stop's face, body and brake lie (`bufferStopStations`): (face, body, brake).
pub fn buffer_stop_stations(track_length: f64, mark: &Value) -> (f64, [f64; 2], [f64; 2]) {
    let clamp = |s: f64| track_length.min(0f64.max(s));
    let b = js_number(mark.get("brakeLength").or(Some(&Value::Null)));
    let brake = if b.is_nan() || b == 0.0 { 0.0 } else { 0f64.max(b) };
    if text(mark, "endpoint") == Some("BEGIN") {
        return (
            clamp(brake + BUFFER_STOP_LENGTH),
            [clamp(brake), clamp(brake + BUFFER_STOP_LENGTH)],
            [0.0, clamp(brake)],
        );
    }
    (
        clamp(track_length - brake - BUFFER_STOP_LENGTH),
        [clamp(track_length - brake - BUFFER_STOP_LENGTH), clamp(track_length - brake)],
        [clamp(track_length - brake), track_length],
    )
}

/// `bufferStopStations` as the JavaScript returns it.
pub fn buffer_stop_stations_json(track_length: f64, mark: &Value) -> Value {
    let (face, body, brake) = buffer_stop_stations(track_length, mark);
    json!({ "face": number(face), "body": [number(body[0]), number(body[1])], "brake": [number(brake[0]), number(brake[1])] })
}

/// The station a mark stands at (`endMarkStation`).
pub fn end_mark_station(track_length: f64, mark: &Value) -> f64 {
    if text(mark, "kind") == Some(BUFFER_STOP) {
        return buffer_stop_stations(track_length, mark).0;
    }
    if text(mark, "endpoint") == Some("BEGIN") { 0.0 } else { track_length }
}

/// The marks after tracks were split, joined or turned (`remapEndMarks`).
pub fn remap_end_marks(marks: &Value, remap: &[Value], consumed: &[Value]) -> Value {
    let Some(list) = marks.as_array().filter(|l| !l.is_empty()) else {
        return marks.clone();
    };
    let gone: HashSet<String> = consumed.iter().map(|e| end_key(e.get("trackId"), e.get("endpoint"))).collect();
    Value::Array(
        list.iter()
            .filter(|m| !gone.contains(&end_key(m.get("trackId"), m.get("endpoint"))))
            .map(|m| {
                let Some(entry) = remap.iter().find(|r| rail_across_js::strict_eq(r.get("oldId"), m.get("trackId"))) else {
                    return m.clone();
                };
                if let Some(ids) = entry.get("newId").and_then(Value::as_array) {
                    let id = if text(m, "endpoint") == Some("BEGIN") { ids.first() } else { ids.get(1) };
                    return with(m, "trackId", id.cloned().unwrap_or(Value::Null));
                }
                let mut o = obj(m);
                o.insert("trackId".into(), entry.get("newId").cloned().unwrap_or(Value::Null));
                let e = m.get("endpoint").cloned().unwrap_or(Value::Null);
                o.insert("endpoint".into(), if truthy(entry.get("flip")) { flip(&e) } else { e });
                Value::Object(o)
            })
            .collect(),
    )
}

/// The marks of a track turned around (`flipEndMarks`).
pub fn flip_end_marks(marks: &Value, track_id: &Value) -> Value {
    let Some(list) = marks.as_array().filter(|l| !l.is_empty()) else {
        return marks.clone();
    };
    Value::Array(
        list.iter()
            .map(|m| {
                if rail_across_js::strict_eq(m.get("trackId"), Some(track_id)) {
                    with(m, "endpoint", flip(m.get("endpoint").unwrap_or(&Value::Null)))
                } else {
                    m.clone()
                }
            })
            .collect(),
    )
}

fn port_ends(switches: &[Value]) -> HashSet<String> {
    let mut held = HashSet::new();
    for sw in switches {
        for (p, _) in ports_of(sw) {
            if let (Some(t), Some(e)) = (port_track(sw, p).filter(|v| truthy(Some(v))), port_end(sw, p)) {
                held.insert(end_key(Some(t), Some(&json!(e))));
            }
        }
    }
    held
}

/// The marks that still stand: on a track, at a free end, one per end, of a known kind (`pruneEndMarks`).
pub fn prune_end_marks(marks: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    let Some(list) = marks.as_array().filter(|l| !l.is_empty()) else {
        return marks.clone();
    };
    let ids: HashSet<String> = tracks.iter().map(|t| js_string(t.get("id"))).collect();
    let held = port_ends(switches);
    let mut seen = HashSet::new();
    let kept: Vec<Value> = list
        .iter()
        .filter(|m| {
            let key = end_key(m.get("trackId"), m.get("endpoint"));
            if !ids.contains(&js_string(m.get("trackId"))) || held.contains(&key) || seen.contains(&key) {
                return false;
            }
            if !matches!(text(m, "kind"), Some(BUFFER_STOP | BOUNDARY)) {
                return false;
            }
            seen.insert(key);
            true
        })
        .cloned()
        .collect();
    if kept.len() == list.len() { marks.clone() } else { Value::Array(kept) }
}
