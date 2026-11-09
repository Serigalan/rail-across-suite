// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track model: pure functions on tracks and switch records (`trackModel.js`).

use rail_across_geom::element::{resolve_end_bearing, reverse_element};
use rail_across_js::{generate_id, get, js_string, number, strict_eq, truthy};
use serde_json::{Value, json};

use crate::heights::{join_heights, reverse_heights, track_length};
use crate::switch_model::{port_end, port_track, ports_of, with_port};
use crate::util::{elements, epsg, list, num, obj, text, without};

/// The tracks a switch's ports name (`portTracks`).
pub fn port_tracks(sw: &Value) -> Vec<Value> {
    ports_of(sw)
        .iter()
        .filter_map(|(p, _)| port_track(sw, p).filter(|v| truthy(Some(v))).cloned())
        .collect()
}

/// Does a switch name the track (`referencesTrack`)?
pub fn references_track(sw: &Value, track_id: &Value) -> bool {
    ports_of(sw).iter().any(|(p, _)| strict_eq(port_track(sw, p), Some(track_id)))
}

fn flip(e: Option<&str>) -> Value {
    match e {
        Some("BEGIN") => json!("END"),
        Some("END") => json!("BEGIN"),
        Some(o) => json!(o),
        None => Value::Null,
    }
}

/// The switches after tracks were split, joined or folded in backwards (`remapSwitches`).
pub fn remap_switches(switches: &[Value], remap: &[Value]) -> Vec<Value> {
    switches
        .iter()
        .map(|sw| {
            let mut updated = sw.clone();
            for (p, _) in ports_of(sw) {
                let Some(entry) = remap.iter().find(|r| strict_eq(r.get("oldId"), port_track(sw, p))) else {
                    continue;
                };
                let end = port_end(sw, p);
                updated = if let Some(ids) = entry.get("newId").and_then(Value::as_array) {
                    let id = if end == Some("BEGIN") { ids.first() } else { ids.get(1) };
                    with_port(&updated, p, &json!({ "trackId": id.cloned().unwrap_or(Value::Null) }))
                } else {
                    let mut patch = json!({ "trackId": entry.get("newId").cloned().unwrap_or(Value::Null) });
                    if truthy(entry.get("flip")) {
                        patch["endpoint"] = flip(end);
                    }
                    with_port(&updated, p, &patch)
                };
            }
            updated
        })
        .collect()
}

/// BEGIN and END swapped on every port naming `track_id` (`flipSwitchEndpoints`).
pub fn flip_switch_endpoints(switches: &[Value], track_id: &Value) -> Vec<Value> {
    switches
        .iter()
        .map(|sw| {
            if !references_track(sw, track_id) {
                return sw.clone();
            }
            let mut updated = sw.clone();
            for (p, _) in ports_of(sw) {
                if strict_eq(port_track(sw, p), Some(track_id)) {
                    updated = with_port(&updated, p, &json!({ "endpoint": flip(port_end(sw, p)) }));
                }
            }
            updated
        })
        .collect()
}

/// A track's polyline from its elements (`rebuildCoords`).
pub fn rebuild_coords(els: &[Value]) -> Vec<Value> {
    let mut coords: Vec<Value> = Vec::new();
    for (i, el) in els.iter().enumerate() {
        let c: &[Value] = get(el, "renderCoords")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .or_else(|| {
                el.get("geometry")
                    .and_then(|g| g.get("coordinates"))
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
            })
            .unwrap_or(&[]);
        if i == 0 {
            coords = c.to_vec();
        } else {
            coords.extend(c.iter().skip(1).cloned());
        }
    }
    coords
}

/// The first `prefix.001`… not taken (`nextTrackName`).
pub fn next_track_name(prefix: &str, existing: &[String]) -> String {
    for i in 1..=999 {
        let c = format!("{prefix}.{i:03}");
        if !existing.contains(&c) {
            return c;
        }
    }
    format!("{prefix}.{}", generate_id())
}

/// A track from a base record and elements (`makeTrack`).
pub fn make_track(base: &Value, els: Vec<Value>, id: Option<&Value>, heights: Option<&Value>) -> Value {
    let mut out = without(base, &["elements", "coordinates", "id", "heights"]);
    out.insert("id".into(), id.filter(|v| !v.is_null()).cloned().unwrap_or_else(|| json!(generate_id())));
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&els)));
    out.insert("elements".into(), Value::Array(els));
    if let Some(h) = heights.filter(|h| h.as_array().is_some_and(|a| !a.is_empty())) {
        out.insert("heights".into(), h.clone());
    }
    Value::Object(out)
}

/// A track running the other way (`reverseTrack`).
pub fn reverse_track(track: &Value) -> Value {
    let els: Vec<Value> = elements(track).iter().rev().map(reverse_element).collect();
    let heights = reverse_heights(get(track, "heights"), track_length(track));
    let mut out = obj(track);
    out.insert("elements".into(), Value::Array(els.clone()));
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&els)));
    if let Some(h) = heights.filter(|h| h.as_array().is_some_and(|a| !a.is_empty())) {
        out.insert("heights".into(), h);
    }
    Value::Object(out)
}

/// Two tracks joined where `head` ends and `tail` begins (`joinTracks`).
pub fn join_tracks(head: &Value, tail: &Value) -> Value {
    let mut els = elements(head).to_vec();
    els.extend_from_slice(elements(tail));
    let heights = join_heights(
        get(head, "heights").map(|_| list(head, "heights")),
        get(tail, "heights").map(|_| list(tail, "heights")),
        track_length(head),
    );
    let mut out = without(head, &["heights"]);
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&els)));
    out.insert("elements".into(), Value::Array(els));
    if let Some(h) = heights.filter(|h| !h.is_empty()) {
        out.insert("heights".into(), Value::Array(h));
    }
    Value::Object(out)
}

/// Where a track can be continued (`trackEndAnchor`).
pub fn track_end_anchor(track: &Value) -> Value {
    let els = elements(track);
    let Some(last) = els.last() else { return Value::Null };
    let c = last.get("geometry").and_then(|g| g.get("coordinates")).and_then(Value::as_array);
    let end_wgs = c.and_then(|c| c.last()).cloned().unwrap_or(Value::Null);
    let Some(end) = get(last, "endNode").and_then(Value::as_array) else {
        return Value::Null;
    };
    if !truthy(track.get("epsg")) {
        return Value::Null;
    }
    json!({
        "endUtm": { "easting": end.first(), "northing": end.get(1), "zone": track.get("epsg") },
        "endWgs": end_wgs,
        "bearing": number(resolve_end_bearing(last, epsg(track))),
        "lastEl": last,
        "lastIndex": els.len() - 1,
    })
}

/// The speed a stretch appended at the end takes by default (`inheritedSpeed`).
pub fn inherited_speed(track: &Value, switches: &[Value]) -> Option<f64> {
    let els = elements(track);
    let branch_speed = |el: &Value| -> Option<f64> {
        if !truthy(el.get("switchBranch")) || text(el, "switchRoute") != Some("branch") || !truthy(el.get("switchId")) {
            return None;
        }
        let sw = switches.iter().find(|sw| strict_eq(sw.get("id"), el.get("switchId")))?;
        num(sw, "speed").filter(|v| *v > 0.0)
    };
    for el in els.iter().rev() {
        let own = num(el, "speed").filter(|v| *v > 0.0);
        let branch = branch_speed(el);
        if own.is_some() || branch.is_some() {
            return Some(own.unwrap_or(f64::INFINITY).min(branch.unwrap_or(f64::INFINITY)));
        }
    }
    None
}

/// How a dialog names a track (`trackLabel`).
pub fn track_label(track: Option<&Value>) -> String {
    let Some(t) = track else { return "–".into() };
    let parts: Vec<String> = ["lineNumber", "trackNumber"]
        .iter()
        .filter_map(|k| t.get(*k).filter(|v| truthy(Some(v))).map(|v| js_string(Some(v))))
        .collect();
    let joined = parts.join(" / ");
    if !joined.is_empty() {
        return joined;
    }
    if let Some(name) = t.get("name").filter(|v| truthy(Some(v))) {
        return js_string(Some(name));
    }
    js_string(t.get("id")).chars().take(8).collect()
}
