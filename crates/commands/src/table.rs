// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What the element table shows: one track, or a route (`tableView.js`).

use std::collections::HashMap;

use rail_across_geom::element::reverse_element;
use rail_across_js::{get, js_string, number, truthy};
use rail_across_track::model::{flip_switch_endpoints, reverse_track};
use rail_across_track::routes::resolve_route;
use serde_json::{Map, Value, json};

use crate::roles::element_roles;

/// The parts a table shows (`tableParts`).
pub fn table_parts(o: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    let Some(route) = get(o, "route") else {
        let parts = match get(o, "trackId").filter(|v| truthy(Some(v))) {
            Some(t) => json!([{ "trackId": t, "reversed": false }]),
            None => json!([]),
        };
        return json!({ "parts": parts, "gaps": [] });
    };
    let res = resolve_route(route, tracks, switches);
    let parts: Vec<Value> = res["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| json!({ "trackId": p.get("trackId"), "reversed": p.get("reversed") }))
        .collect();
    json!({ "parts": parts, "gaps": res["gaps"] })
}

fn by_id(tracks: &[Value]) -> HashMap<String, &Value> {
    tracks.iter().map(|t| (js_string(t.get("id")), t)).collect()
}

fn els(t: Option<&Value>) -> &[Value] {
    t.and_then(|t| t.get("elements")).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The rows of those parts (`tableRows`).
pub fn table_rows(parts: &[Value], tracks: &[Value]) -> Value {
    let by = by_id(tracks);
    let (mut elements, mut rows, mut stations) = (Vec::new(), Vec::new(), Vec::new());
    let mut at = 0.0;
    for part in parts {
        let tid = part.get("trackId").cloned().unwrap_or(Value::Null);
        let track = by.get(&js_string(Some(&tid))).copied();
        let es = els(track);
        let reversed = truthy(part.get("reversed"));
        let order: Vec<usize> = if reversed { (0..es.len()).rev().collect() } else { (0..es.len()).collect() };
        for index in order {
            let el = if reversed { reverse_element(&es[index]) } else { es[index].clone() };
            let len = get(&el, "length").and_then(Value::as_f64).unwrap_or(0.0);
            rows.push(json!({ "trackId": tid, "index": index, "reversed": reversed, "epsg": track.and_then(|t| t.get("epsg")) }));
            stations.push(json!({ "start": number(at), "end": number(at + len) }));
            at += len;
            elements.push(el);
        }
    }
    json!({ "elements": elements, "rows": rows, "stations": stations, "length": number(at) })
}

/// The row of a track element in the view (`rowOf`).
pub fn row_of(rows: &[Value], track_id: &Value, index: usize) -> i64 {
    rows.iter()
        .position(|r| rail_across_js::strict_eq(r.get("trackId"), Some(track_id)) && r.get("index").and_then(Value::as_u64) == Some(index as u64))
        .map(|i| i as i64)
        .unwrap_or(-1)
}

fn run_index(row: &Value, count: usize) -> usize {
    let index = row.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
    if truthy(row.get("reversed")) { count - 1 - index } else { index }
}

/// What the elements may take, per row (`tableRoles`).
pub fn table_roles(rows: &[Value], tracks: &[Value]) -> Vec<Value> {
    let by = by_id(tracks);
    let mut per_track: HashMap<String, Vec<Value>> = HashMap::new();
    rows.iter()
        .map(|r| {
            let tid = js_string(r.get("trackId"));
            let es = els(by.get(&tid).copied());
            let reversed = truthy(r.get("reversed"));
            let roles = per_track.entry(tid).or_insert_with(|| {
                element_roles(&if reversed {
                    es.iter().rev().cloned().collect::<Vec<_>>()
                } else {
                    es.to_vec()
                })
            });
            let Some(role) = roles.get(run_index(r, es.len())).cloned() else {
                return Value::Null;
            };
            let Some(by_idx) = role.get("by").and_then(Value::as_u64) else { return role };
            let index = if reversed { es.len() - 1 - by_idx as usize } else { by_idx as usize };
            let mut out = role;
            out["by"] = json!(row_of(rows, r.get("trackId").unwrap_or(&Value::Null), index));
            out
        })
        .collect()
}

/// An edit of a row's track done in the view's direction (`inRun`).
pub fn in_run(row: &Value, tracks: &[Value], switches: &[Value], edit: &dyn Fn(&[Value], &[Value], usize) -> Value) -> Value {
    let tid = row.get("trackId").cloned().unwrap_or(Value::Null);
    let Some(track) = tracks.iter().find(|t| rail_across_js::strict_eq(t.get("id"), Some(&tid))) else {
        return Value::Null;
    };
    let index = run_index(row, els(Some(track)).len());
    if !truthy(row.get("reversed")) {
        return edit(tracks, switches, index);
    }
    let turned: Vec<Value> = tracks
        .iter()
        .map(|t| {
            if rail_across_js::strict_eq(t.get("id"), Some(&tid)) {
                reverse_track(t)
            } else {
                t.clone()
            }
        })
        .collect();
    let mut out = edit(&turned, &flip_switch_endpoints(switches, &tid), index);
    if let Some(ts) = out.get("tracks").and_then(Value::as_array) {
        let back: Vec<Value> = ts
            .iter()
            .map(|t| {
                if rail_across_js::strict_eq(t.get("id"), Some(&tid)) {
                    reverse_track(t)
                } else {
                    t.clone()
                }
            })
            .collect();
        out["tracks"] = Value::Array(back);
    }
    out
}

/// A value typed into a row as its track element takes it (`trackEdit`).
pub fn track_edit(row: &Value, key: &str, value: &Value) -> Value {
    if !truthy(row.get("reversed")) {
        return json!({ "key": key, "value": value });
    }
    let neg = |v: &Value| match v.as_f64().filter(|x| x.is_finite()) {
        Some(x) => number(-x),
        None => v.clone(),
    };
    match key {
        "radius" | "cant" => json!({ "key": key, "value": neg(value) }),
        "cantStart" => json!({ "key": "cantEnd", "value": neg(value) }),
        "cantEnd" => json!({ "key": "cantStart", "value": neg(value) }),
        _ => json!({ "key": key, "value": value }),
    }
}

const GEOMETRY: [&str; 6] = ["geometry", "renderCoords", "startNode", "endNode", "bearing", "endBearing"];

fn diff_fields(old: &Value, next: &Value) -> Map<String, Value> {
    next.as_object()
        .into_iter()
        .flatten()
        .filter(|(k, v)| !GEOMETRY.contains(&k.as_str()) && !rail_across_js::strict_eq(old.get(k.as_str()), Some(v)))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// The tracks with the view's elements written back (`writeRows`).
pub fn write_rows(tracks: &[Value], rows: &[Value], elements: &[Value]) -> Vec<Value> {
    let mut edits: HashMap<String, HashMap<usize, Value>> = HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        let el = if truthy(r.get("reversed")) {
            reverse_element(&elements[i])
        } else {
            elements[i].clone()
        };
        edits
            .entry(js_string(r.get("trackId")))
            .or_default()
            .insert(r.get("index").and_then(Value::as_u64).unwrap_or(0) as usize, el);
    }
    tracks
        .iter()
        .map(|t| {
            let Some(mine) = edits.get(&js_string(t.get("id"))) else { return t.clone() };
            let mut changed = false;
            let new_els: Vec<Value> = els(Some(t))
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let Some(next) = mine.get(&i) else { return e.clone() };
                    let diff = diff_fields(e, next);
                    if diff.is_empty() {
                        return e.clone();
                    }
                    changed = true;
                    let mut o = e.clone();
                    for (k, v) in diff {
                        o[k] = v;
                    }
                    o
                })
                .collect();
            if changed {
                let mut out = t.clone();
                out["elements"] = Value::Array(new_els);
                out
            } else {
                t.clone()
            }
        })
        .collect()
}
