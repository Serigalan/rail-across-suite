// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Carrying the locations of a domain across a change of the net (`carryLocations.js`).

use std::collections::HashSet;

use rail_across_js::{get, js_number, js_round, js_string, number, truthy};
use rail_across_track::platforms::{point_at_station, station_on_track};
use rail_across_track::routes::repair_route_with;
use serde_json::{Map, Value, json};

const ON_TRACK: f64 = 0.05;

fn round(s: f64) -> f64 {
    js_round(s * 1000.0) / 1000.0
}

fn bearing_gap(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs()
}

fn is_location(v: &Value) -> bool {
    v.get("anchor").and_then(Value::as_str) == Some("station")
        && v.get("trackId").is_some_and(Value::is_string)
        && v.get("station").and_then(Value::as_f64).is_some_and(f64::is_finite)
}

fn is_range(v: &Value) -> bool {
    v.get("anchor").and_then(Value::as_str) == Some("station")
        && v.get("trackId").is_some_and(Value::is_string)
        && v.get("startStation").and_then(Value::as_f64).is_some_and(f64::is_finite)
        && v.get("endStation").and_then(Value::as_f64).is_some_and(f64::is_finite)
}

fn is_path(v: &Value) -> bool {
    v.get("trackIds").is_some_and(Value::is_array) && v.as_object().is_some_and(|o| o.len() == 1)
}

fn flip_side(s: &str) -> Option<&'static str> {
    match s {
        "left" => Some("right"),
        "right" => Some("left"),
        _ => None,
    }
}

fn flip_direction(s: &str) -> Value {
    match s {
        "up" => json!("down"),
        "down" => json!("up"),
        _ => Value::Null,
    }
}

/// What a carry is asked with (`carryDomain`'s ctx).
pub struct Ctx<'a> {
    pub present: HashSet<String>,
    pub changed: &'a dyn Fn(&str) -> bool,
    pub old_track: &'a dyn Fn(&str) -> Option<Value>,
    pub candidates: &'a dyn Fn(&str) -> Vec<Value>,
    pub tracks: Vec<Value>,
    pub switches: Vec<Value>,
}

fn epsg_eq(a: &Value, b: &Value) -> bool {
    rail_across_js::strict_eq(a.get("epsg"), b.get("epsg"))
}

fn carry_location(loc: &Value, ctx: &Ctx) -> Value {
    let tid = js_string(loc.get("trackId"));
    let Some(old) = (ctx.old_track)(&tid) else { return loc.clone() };
    let Some((p, pb)) = point_at_station(&old, js_number(loc.get("station"))) else {
        return loc.clone();
    };
    let mut hit: Option<(Value, (f64, f64, f64))> = None;
    for track in (ctx.candidates)(&tid) {
        if !epsg_eq(&track, &old) {
            continue;
        }
        let Some(on) = station_on_track(&track, p).filter(|o| o.1 <= ON_TRACK) else {
            continue;
        };
        let is_same = js_string(track.get("id")) == tid;
        let better = match &hit {
            None => true,
            Some((ht, hon)) => {
                let hit_same = js_string(ht.get("id")) == tid;
                (is_same && !hit_same) || (on.1 < hon.1 - 1e-6 && !hit_same)
            }
        };
        if better {
            hit = Some((track, on));
        }
    }
    let Some((track, on)) = hit else {
        if ctx.present.contains(&tid) {
            return loc.clone();
        }
        let down = loc.get("direction").and_then(Value::as_str) == Some("down");
        let bearing = if down { (pb + 180.0) % 360.0 } else { pb };
        let mut out = json!({ "anchor": "plane", "easting": number(round(p.e)), "northing": number(round(p.n)), "epsg": old.get("epsg") });
        if truthy(loc.get("direction")) {
            out["bearing"] = number(js_round(bearing * 1e6) / 1e6);
        }
        return out;
    };
    let reversed = bearing_gap(on.2, pb) > 90.0;
    let station = round(on.0);
    let new_id = js_string(track.get("id"));
    if new_id == tid && !reversed && (station - js_number(loc.get("station"))).abs() < 1e-3 {
        return loc.clone();
    }
    let mut out = loc.clone();
    out["trackId"] = track.get("id").cloned().unwrap_or(Value::Null);
    out["station"] = number(station);
    if reversed && let Some(s) = loc.get("side").and_then(Value::as_str).and_then(flip_side) {
        out["side"] = json!(s);
    }
    if reversed && let Some(d) = loc.get("direction").filter(|v| truthy(Some(v))) {
        out["direction"] = flip_direction(d.as_str().unwrap_or(""));
    }
    out
}

fn carry_range(range: &Value, ctx: &Ctx) -> Value {
    let tid = js_string(range.get("trackId"));
    let Some(old) = (ctx.old_track)(&tid) else { return range.clone() };
    let (Some((p0, b0)), Some((p1, _))) = (
        point_at_station(&old, js_number(range.get("startStation"))),
        point_at_station(&old, js_number(range.get("endStation"))),
    ) else {
        return range.clone();
    };
    let mut hit: Option<(Value, (f64, f64, f64), (f64, f64, f64))> = None;
    for track in (ctx.candidates)(&tid) {
        if !epsg_eq(&track, &old) {
            continue;
        }
        let (Some(a), Some(b)) = (station_on_track(&track, p0), station_on_track(&track, p1)) else {
            continue;
        };
        if a.1 > ON_TRACK || b.1 > ON_TRACK {
            continue;
        }
        if hit.is_none() || js_string(track.get("id")) == tid {
            hit = Some((track, a, b));
        }
    }
    let Some((track, a, b)) = hit else { return range.clone() };
    let reversed = bearing_gap(a.2, b0) > 90.0;
    let (s0, s1) = (round(a.0), round(b.0));
    let mut out = range.clone();
    out["trackId"] = track.get("id").cloned().unwrap_or(Value::Null);
    out["startStation"] = number(s0.min(s1));
    out["endStation"] = number(s0.max(s1));
    if reversed && let Some(s) = range.get("side").and_then(Value::as_str).and_then(flip_side) {
        out["side"] = json!(s);
    }
    let same = js_string(out.get("trackId")) == tid
        && (js_number(out.get("startStation")) - js_number(range.get("startStation"))).abs() < 1e-3
        && (js_number(out.get("endStation")) - js_number(range.get("endStation"))).abs() < 1e-3
        && out.get("side") == range.get("side");
    if same { range.clone() } else { out }
}

fn carry_path(path: &Value, ctx: &Ctx) -> Value {
    let old = |id: &Value| (ctx.old_track)(&js_string(Some(id)));
    let cands = |id: &Value| (ctx.candidates)(&js_string(Some(id)));
    let next = repair_route_with(path, &ctx.tracks, &ctx.switches, &old, &cands);
    if next == *path {
        path.clone()
    } else {
        json!({ "trackIds": next["trackIds"] })
    }
}

/// A moved location.
pub struct Moved {
    pub collection: String,
    pub id: Value,
    pub field: String,
    pub from: String,
    pub to: String,
}

fn walk(value: &Value, field: &str, ctx: &Ctx, note: &mut dyn FnMut(&str, String, String)) -> Value {
    match value {
        Value::Array(a) => Value::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| walk(v, &if field.is_empty() { i.to_string() } else { format!("{field}.{i}") }, ctx, note))
                .collect(),
        ),
        Value::Object(o) => {
            if is_location(value) {
                let tid = js_string(value.get("trackId"));
                if !(ctx.changed)(&tid) {
                    return value.clone();
                }
                let next = carry_location(value, ctx);
                if next != *value {
                    let to = if next.get("anchor").and_then(Value::as_str) == Some("plane") {
                        "plane".to_string()
                    } else {
                        js_string(next.get("trackId"))
                    };
                    note(field, tid, to);
                }
                return next;
            }
            if is_range(value) {
                let tid = js_string(value.get("trackId"));
                if !(ctx.changed)(&tid) {
                    return value.clone();
                }
                let next = carry_range(value, ctx);
                if next != *value {
                    note(field, tid, js_string(next.get("trackId")));
                }
                return next;
            }
            if is_path(value) {
                let ids: Vec<String> = value["trackIds"].as_array().into_iter().flatten().map(|v| js_string(Some(v))).collect();
                if !ids.iter().any(|id| (ctx.changed)(id)) {
                    return value.clone();
                }
                let next = carry_path(value, ctx);
                if next != *value {
                    let from = ids.iter().filter(|id| !ctx.present.contains(*id)).cloned().collect::<Vec<_>>().join(",");
                    let to = next["trackIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|v| js_string(Some(v)))
                        .collect::<Vec<_>>()
                        .join(",");
                    note(field, from, to);
                }
                return next;
            }
            let mut out = Map::new();
            for (k, v) in o {
                out.insert(
                    k.clone(),
                    walk(v, &if field.is_empty() { k.clone() } else { format!("{field}.{k}") }, ctx, note),
                );
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}

/// A domain carried from the tracks `before` onto a project's tracks and switches —
/// what the Signalling-Tool's taking of its own side over a merged net builds its
/// context from: a track changed where it is gone or no longer the same.
pub fn carry_domain_onto(domain: &Value, before: &[Value], tracks: &[Value], switches: &[Value]) -> (Value, Vec<Moved>) {
    let old: Map<String, Value> = before.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
    let now: Map<String, Value> = tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
    let changed = |id: &str| old.contains_key(id) && old.get(id) != now.get(id);
    let old_track = |id: &str| old.get(id).cloned();
    let candidates = |_: &str| tracks.to_vec();
    let ctx = Ctx {
        present: now.keys().cloned().collect(),
        changed: &changed,
        old_track: &old_track,
        candidates: &candidates,
        tracks: tracks.to_vec(),
        switches: switches.to_vec(),
    };
    carry_domain(domain, &ctx)
}

impl Moved {
    pub fn to_json(&self) -> Value {
        json!({ "collection": self.collection, "id": self.id, "field": self.field, "from": self.from, "to": self.to })
    }
}

/// The domain with every location on a changed track carried (`carryDomain`).
pub fn carry_domain(domain: &Value, ctx: &Ctx) -> (Value, Vec<Moved>) {
    let mut carried = Vec::new();
    let Some(collections) = domain.get("collections").and_then(Value::as_object) else {
        return (domain.clone(), carried);
    };
    let mut any = false;
    let mut out = Map::new();
    for (name, list) in collections {
        let Some(items) = list.as_array() else {
            out.insert(name.clone(), list.clone());
            continue;
        };
        let mut changed = false;
        let next: Vec<Value> = items
            .iter()
            .map(|obj| {
                let id = get(obj, "id").cloned().unwrap_or(Value::Null);
                let mut note = |field: &str, from: String, to: String| {
                    carried.push(Moved {
                        collection: name.clone(),
                        id: id.clone(),
                        field: field.to_string(),
                        from,
                        to,
                    })
                };
                let w = walk(obj, "", ctx, &mut note);
                if w != *obj {
                    changed = true;
                }
                w
            })
            .collect();
        if changed {
            any = true;
            out.insert(name.clone(), Value::Array(next));
        } else {
            out.insert(name.clone(), list.clone());
        }
    }
    if !any {
        return (domain.clone(), carried);
    }
    let mut d = domain.clone();
    d["collections"] = Value::Object(out);
    (d, carried)
}

/// The layers whose locations every write of the net carries (R4): the signalling and the electrification (decision 0025).
pub const CARRIED_DOMAINS: [&str; 2] = ["signalling", "electrification"];

/// A record after one write of the net, its domains' locations carried (`carryAfterWrite`).
pub fn carry_after_write(before: &Value, after: &Value) -> Value {
    if after.is_null() || before.get("tracks") == after.get("tracks") {
        return after.clone();
    }
    let domains: Vec<&str> = CARRIED_DOMAINS
        .into_iter()
        .filter(|n| after.get(*n).and_then(|d| d.get("collections")).is_some_and(|c| truthy(Some(c))))
        .collect();
    // The civil structures the record keeps at its top are carried as a domain of their own (decision 0025).
    let structures: Map<String, Value> = STRUCTURES
        .iter()
        .filter_map(|n| {
            after
                .get(*n)
                .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
                .map(|v| ((*n).to_string(), v.clone()))
        })
        .collect();
    if domains.is_empty() && structures.is_empty() {
        return after.clone();
    }
    let old: Map<String, Value> = before
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|t| (js_string(t.get("id")), t.clone()))
        .collect();
    let now: Map<String, Value> = after
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|t| (js_string(t.get("id")), t.clone()))
        .collect();
    let tracks: Vec<Value> = after.get("tracks").and_then(Value::as_array).cloned().unwrap_or_default();
    let touched: Vec<Value> = tracks.iter().filter(|t| old.get(&js_string(t.get("id"))) != Some(*t)).cloned().collect();
    let changed = |id: &str| old.contains_key(id) && old.get(id) != now.get(id);
    let old_track = |id: &str| old.get(id).cloned();
    let candidates = |_: &str| touched.clone();
    let ctx = Ctx {
        present: now.keys().cloned().collect(),
        changed: &changed,
        old_track: &old_track,
        candidates: &candidates,
        tracks: tracks.clone(),
        switches: after.get("switches").and_then(Value::as_array).cloned().unwrap_or_default(),
    };
    let mut out = after.clone();
    for name in domains {
        let (domain, _) = carry_domain(&after[name], &ctx);
        if domain != after[name] {
            out[name] = domain;
        }
    }
    if !structures.is_empty() {
        let (carried, _) = carry_domain(&json!({ "collections": structures }), &ctx);
        for (name, list) in carried["collections"].as_object().into_iter().flatten() {
            out[name.as_str()] = list.clone();
        }
    }
    out
}

/// The civil structures a record keeps at its top, carried with the net as a domain's locations are.
pub const STRUCTURES: [&str; 3] = ["bridges", "retainingWalls", "noiseBarriers"];
