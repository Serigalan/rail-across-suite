// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The project record as it is kept and the door to it (`persistenceUtils.js`):
//! infra-data-1.0 documents in (decision 0021: the one format), the record with
//! its derived data stripped and its numbers kept to what they mean.

use rail_across_geom::cant::stored_track_cants;
use rail_across_geom::element::derived_end_bearing;
use rail_across_geom::tolerances::POSITION_DIGITS;
use rail_across_js::{Object, get, js_round, js_string, js_trim, number, truthy};
use serde_json::Value;

use crate::door::{from_document, to_document};
use crate::error::{PayloadError, Result};

/// The records of a payload — an infra-data-1.0 document, one project —
/// refused where it is not one (`parseProjectsPayload`): `unsupported_version`
/// for another format or version (an oltjson file among them, decision 0021),
/// `invalid_payload` where it does not hold what it states.
pub fn parse_projects_payload(data: &Value) -> Result<Vec<Value>> {
    Ok(vec![from_document(data)?])
}

/// The infra-data-1.0 document of a project record (`projectDocument`).
pub fn project_document(record: &Value, carry_schemas: bool, check: bool) -> Result<Value> {
    to_document(&dehydrate_project(record), carry_schemas, check)
}

/// The domains of a project record's document in the words and names the record keeps (decision 0029, phase 1):
/// what a tool reads of the core beside its own layer until the record follows the document (phase 6). Not
/// checked: a tool reads it while it works.
pub fn record_domains(record: &Value) -> Result<Value> {
    let doc = to_document(&dehydrate_project(record), false, false)?;
    let mut domains = doc["domains"].as_object().cloned().unwrap_or_default();
    crate::words::back(&mut domains);
    Ok(Value::Object(domains))
}

/// The tracks of an exported tracks file — an infra-data-1.0 document holding
/// them —, refused where it is not one or holds none (`tracks_empty`), and where
/// a track belongs to a switch the project does not have (`switch_missing`):
/// its branch would point at nothing (`parseTracksPayload`).
pub fn parse_tracks_payload(data: &Value, switch_ids: &[String]) -> Result<Vec<Value>> {
    let record = from_document(data)?;
    let tracks: Vec<Value> = record.get("tracks").and_then(Value::as_array).cloned().unwrap_or_default();
    if tracks.is_empty() {
        return Err(PayloadError::new("tracks_empty"));
    }
    for track in &tracks {
        for el in track.get("elements").and_then(Value::as_array).into_iter().flatten() {
            if truthy(el.get("switchId")) && !switch_ids.contains(&js_string(el.get("switchId"))) {
                return Err(PayloadError::new("switch_missing"));
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
