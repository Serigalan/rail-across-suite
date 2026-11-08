// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The project record as it is kept and the door to it (`persistenceUtils.js`):
//! oltjson-1.1 files and core 1.0 documents in, the record with its derived
//! data stripped and its numbers kept to what they mean.

use rail_across_geom::cant::stored_track_cants;
use rail_across_geom::element::derived_end_bearing;
use rail_across_geom::tolerances::POSITION_DIGITS;
use rail_across_js::{Object, get, js_round, js_string, js_trim, number, truthy};
use serde_json::{Value, json};

use crate::door::{FORMAT as DOCUMENT_FORMAT, from_document, to_document};
use crate::error::{PayloadError, Result};

const FORMAT: &str = "oltjson";
const FORMAT_VERSION: &str = "1.1";
/// The format's name as it is spoken of: oltjson-1.1.
pub const FORMAT_NAME: &str = "oltjson-1.1";

/// A file of `records` (already dehydrated): { format, version, projects }.
pub fn projects_payload(records: Vec<Value>) -> Value {
    json!({ "format": FORMAT, "version": FORMAT_VERSION, "projects": records })
}

fn is_port(p: &Value) -> bool {
    p.is_null() || p.is_object()
}

const ELEMENT_ROLES: &[&str] = &["follow", "intermediate"];

fn kept_role_and_reasons(el: &Value) -> bool {
    let role_ok = match get(el, "role") {
        None => true,
        Some(r) => r.as_str().is_some_and(|r| ELEMENT_ROLES.contains(&r)),
    };
    let reasons_ok = match get(el, "reasons") {
        None => true,
        Some(Value::Object(o)) => o.values().all(Value::is_string),
        Some(_) => false,
    };
    role_ok && reasons_ok
}

/// `isModelledSwitch`: a switch with its id, its kind and its form version.
pub fn is_modelled_switch(sw: &Value) -> bool {
    truthy(sw.get("switchId")) && truthy(sw.get("kind")) && get(sw, "formVersion").is_some()
}

fn items(v: &Value, key: &str) -> Result<Vec<Value>> {
    match get(v, key) {
        None => Ok(Vec::new()),
        Some(Value::Array(a)) => Ok(a.clone()),
        Some(_) => Err(PayloadError::new("invalid_payload")),
    }
}

/// The records of a payload — a core 1.0 document, or an oltjson-1.1 file —
/// refused where it is not one of these (`parseProjectsPayload`).
pub fn parse_projects_payload(data: &Value) -> Result<Vec<Value>> {
    if data.get("format").and_then(Value::as_str) == Some(DOCUMENT_FORMAT) {
        return Ok(vec![from_document(data)?]);
    }
    let Some(projects) = data.get("projects").and_then(Value::as_array) else {
        return Err(PayloadError::new("invalid_payload"));
    };
    if data.get("format").and_then(Value::as_str) != Some(FORMAT) || data.get("version").and_then(Value::as_str) != Some(FORMAT_VERSION) {
        return Err(PayloadError::new("unsupported_version"));
    }
    for p in projects {
        for sw in items(p, "switches")? {
            if !is_modelled_switch(&sw) {
                return Err(PayloadError::new("invalid_payload"));
            }
            if let Some(ports) = get(&sw, "ports") {
                let ok = match ports {
                    Value::Object(o) => o.values().all(is_port),
                    Value::Array(a) => a.iter().all(is_port),
                    _ => false,
                };
                if !ok {
                    return Err(PayloadError::new("invalid_payload"));
                }
            }
        }
        for track in items(p, "tracks")? {
            for el in items(&track, "elements")? {
                if truthy(el.get("switchBranch")) && !truthy(el.get("switchId")) {
                    return Err(PayloadError::new("invalid_payload"));
                }
                if !kept_role_and_reasons(&el) {
                    return Err(PayloadError::new("invalid_payload"));
                }
            }
        }
    }
    projects
        .iter()
        .map(|p| {
            let kept = dehydrate_project(p);
            from_document(&to_document(&kept, false, true)?)
        })
        .collect()
}

/// The core 1.0 document of a project record (`projectDocument`).
pub fn project_document(record: &Value, carry_schemas: bool, check: bool) -> Result<Value> {
    to_document(&dehydrate_project(record), carry_schemas, check)
}

fn is_point(p: &Value) -> bool {
    p.as_array()
        .is_some_and(|a| a.len() >= 2 && a.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)))
}

/// The tracks of an exported tracks file, refused where they are not (`parseTracksPayload`).
pub fn parse_tracks_payload(data: &Value, switch_ids: &[String]) -> Result<Vec<Value>> {
    let tracks: Vec<Value> = match data {
        Value::Array(a) => a.clone(),
        other => vec![other.clone()],
    };
    if tracks.is_empty() {
        return Err(PayloadError::new("tracks_empty"));
    }
    let invalid = || PayloadError::new("invalid_payload");
    for track in &tracks {
        if !track.is_object() {
            return Err(invalid());
        }
        if !track.get("id").and_then(Value::as_str).is_some_and(|s| !s.is_empty()) {
            return Err(invalid());
        }
        let Some(elements) = track.get("elements").and_then(Value::as_array).filter(|e| !e.is_empty()) else {
            return Err(invalid());
        };
        for el in elements {
            if !el.is_object() {
                return Err(invalid());
            }
            let t = el.get("elementType").and_then(Value::as_f64);
            if !matches!(t, Some(x) if x == 0.0 || x == 1.0 || x == 2.0) {
                return Err(invalid());
            }
            if !is_point(&el["startNode"]) || !is_point(&el["endNode"]) {
                return Err(invalid());
            }
            if !el.get("length").and_then(Value::as_f64).is_some_and(|l| l.is_finite() && l >= 0.0) {
                return Err(invalid());
            }
            if truthy(el.get("switchBranch")) && !truthy(el.get("switchId")) {
                return Err(invalid());
            }
            if truthy(el.get("switchId")) && !switch_ids.contains(&js_string(el.get("switchId"))) {
                return Err(PayloadError::new("switch_missing"));
            }
            if !kept_role_and_reasons(el) {
                return Err(invalid());
            }
        }
    }
    Ok(tracks)
}

// ── Numbers as they are kept ─────────────────────────────────────────────────

fn at(digits: i32) -> impl Fn(&Value) -> Value {
    let f = 10f64.powi(digits);
    move |v: &Value| match v.as_f64() {
        Some(x) if x.is_finite() => number(js_round(x * f) / f),
        _ => v.clone(),
    }
}

const SAME_END_BEARING: f64 = 1e-7;

fn turn_between(a: f64, b: f64) -> f64 {
    ((((a - b) % 360.0) + 540.0) % 360.0 - 180.0).abs()
}

fn kept_reasons(el: Object) -> Object {
    let mut rest = el;
    let reasons = rest.shift_remove("reasons");
    let mut kept = Object::new();
    if let Some(Value::Object(r)) = reasons {
        for (id, text) in r {
            let t = js_trim(&if text.is_null() { String::new() } else { js_string(Some(&text)) }).to_string();
            if !t.is_empty() {
                kept.insert(id, Value::String(t));
            }
        }
    }
    if !kept.is_empty() {
        rest.insert("reasons".into(), Value::Object(kept));
    }
    rest
}

fn kept_element(el: &Value) -> Value {
    let position = at(POSITION_DIGITS);
    let angle = at(9);
    let cant = at(3);
    let mut base = el.as_object().cloned().unwrap_or_default();
    base.shift_remove("renderCoords");
    base.shift_remove("geometry");
    let mut out = kept_reasons(base);
    match out.get("endBearing").and_then(Value::as_f64).filter(|x| x.is_finite()) {
        None => {
            out.shift_remove("endBearing");
        }
        Some(end) => {
            let probe = Value::Object(out.clone());
            let derived = derived_end_bearing(&probe).or_else(|| get(&probe, "bearing").and_then(Value::as_f64));
            if let Some(d) = derived
                && turn_between(end, d) < SAME_END_BEARING
            {
                out.shift_remove("endBearing");
            }
        }
    }
    for k in ["startNode", "endNode"] {
        if let Some(Value::Array(p)) = out.get(k) {
            let p: Vec<Value> = p.iter().map(&position).collect();
            out.insert(k.into(), Value::Array(p));
        }
    }
    for k in ["length", "radius", "r1", "r2"] {
        if let Some(v) = out.get(k).filter(|v| !v.is_null()) {
            let v = position(v);
            out.insert(k.into(), v);
        }
    }
    for k in ["bearing", "endBearing"] {
        if let Some(v) = out.get(k).filter(|v| !v.is_null()) {
            let v = angle(v);
            out.insert(k.into(), v);
        }
    }
    for k in ["cant", "cantStart", "cantEnd"] {
        if let Some(v) = out.get(k).filter(|v| !v.is_null()) {
            let v = cant(v);
            out.insert(k.into(), v);
        }
    }
    Value::Object(out)
}

fn kept_height(h: &Value) -> Value {
    let position = at(POSITION_DIGITS);
    let mm10 = at(4);
    let mut out = h.as_object().cloned().unwrap_or_default();
    for k in ["station", "rv", "la"] {
        if let Some(v) = out.get(k).filter(|v| !v.is_null()) {
            let v = position(v);
            out.insert(k.into(), v);
        }
    }
    if let Some(z) = out.get("z").filter(|v| !v.is_null()) {
        let z = mm10(z);
        out.insert("z".into(), z);
    }
    Value::Object(out)
}

/// One record as it is kept: derived data stripped, numbers rounded.
pub fn dehydrate_project(p: &Value) -> Value {
    let has_tracks = truthy(p.get("tracks"));
    let has_switches = truthy(p.get("switches"));
    let has_platforms = truthy(p.get("platforms"));
    if !has_tracks && !has_switches && !has_platforms {
        return p.clone();
    }
    let position = at(POSITION_DIGITS);
    let mut out = p.as_object().cloned().unwrap_or_default();
    if has_tracks && let Some(Value::Array(tracks)) = p.get("tracks") {
        let tracks: Vec<Value> = tracks
            .iter()
            .map(|t| {
                let mut track = t.as_object().cloned().unwrap_or_default();
                for k in ["coordinates", "regelwerk", "regelwerkGrenzwert"] {
                    track.shift_remove(k);
                }
                let elements: Vec<Value> = match get(t, "elements") {
                    Some(Value::Array(e)) => stored_track_cants(e).iter().map(kept_element).collect(),
                    _ => Vec::new(),
                };
                track.insert("elements".into(), Value::Array(elements));
                if let Some(Value::Array(h)) = t.get("heights").filter(|h| truthy(Some(h))) {
                    track.insert("heights".into(), Value::Array(h.iter().map(kept_height).collect()));
                }
                Value::Object(track)
            })
            .collect();
        out.insert("tracks".into(), Value::Array(tracks));
    }
    if has_switches && let Some(Value::Array(sws)) = p.get("switches") {
        let sws: Vec<Value> = sws
            .iter()
            .map(|sw| {
                let mut o = sw.as_object().cloned().unwrap_or_default();
                for k in ["fillCoords", "lcsCoords", "labelCoords", "bodyCentre", "bauform"] {
                    o.shift_remove(k);
                }
                Value::Object(o)
            })
            .collect();
        out.insert("switches".into(), Value::Array(sws));
    }
    if has_platforms && let Some(Value::Array(pfs)) = p.get("platforms") {
        let pfs: Vec<Value> = pfs
            .iter()
            .map(|pf| {
                let mut o = pf.as_object().cloned().unwrap_or_default();
                o.shift_remove("coords");
                for k in ["startStation", "endStation"] {
                    if let Some(v) = get(pf, k) {
                        o.insert(k.into(), position(v));
                    }
                }
                Value::Object(o)
            })
            .collect();
        out.insert("platforms".into(), Value::Array(pfs));
    }
    Value::Object(out)
}

/// The records as they are kept (`dehydrateProjects`).
pub fn dehydrate_projects(projects: &[Value]) -> Vec<Value> {
    projects.iter().map(dehydrate_project).collect()
}
