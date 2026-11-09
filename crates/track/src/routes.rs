// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Named runs of tracks (`routes.js`).

use std::collections::HashMap;

use rail_across_js::{js_string, number, strict_eq};
use serde_json::{Value, json};

use crate::heights::track_length;
use crate::platforms::{point_at_station, station_on_track};
use crate::topology::tracks_on_from;
use crate::util::{list, num, obj, with};

fn other(end: &str) -> &'static str {
    if end == "BEGIN" { "END" } else { "BEGIN" }
}

fn links(a: &Value, b: &Value, tracks: &[Value], switches: &[Value]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for exit in ["END", "BEGIN"] {
        for (t, e) in tracks_on_from(a, exit, tracks, switches) {
            if strict_eq(Some(&t), Some(b)) {
                out.push((exit, e));
            }
        }
    }
    out
}

/// A route laid on the tracks (`resolveRoute`).
pub fn resolve_route(route: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    let by_id: HashMap<String, &Value> = tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
    let mut gaps: Vec<(i64, Value)> = Vec::new();
    let mut present: Vec<Value> = Vec::new();
    for id in list(route, "trackIds") {
        if by_id.contains_key(&js_string(Some(id))) {
            present.push(id.clone());
        } else {
            let after = present.len() as i64 - 1;
            gaps.push((after, json!({ "after": after, "kind": "missing", "trackId": id })));
        }
    }
    let mut entry: Vec<Option<String>> = vec![None; present.len()];
    let mut exit: Vec<Option<&str>> = vec![None; present.len()];
    for i in 0..present.len().saturating_sub(1) {
        let found = links(&present[i], &present[i + 1], tracks, switches);
        let pick = found.iter().find(|(ex, _)| entry[i].as_deref().is_none_or(|en| *ex == other(en)));
        let Some((ex, en)) = pick else {
            if !gaps.iter().any(|(a, _)| *a == i as i64) {
                gaps.push((i as i64, json!({ "after": i, "kind": "disconnected" })));
            }
            continue;
        };
        exit[i] = Some(ex);
        entry[i + 1] = Some(en.clone());
    }
    let mut parts = Vec::new();
    let mut offset = 0.0;
    for (i, id) in present.iter().enumerate() {
        let track = by_id[&js_string(Some(id))];
        let reversed = match (&entry[i], exit[i]) {
            (Some(en), _) => en == "END",
            (None, Some(ex)) => ex == "BEGIN",
            _ => false,
        };
        let length = track_length(track);
        parts.push(json!({ "trackId": id, "track": track, "reversed": reversed, "offset": number(offset), "length": number(length) }));
        offset += length;
    }
    gaps.sort_by_key(|(a, _)| *a);
    let ok = !parts.is_empty() && gaps.is_empty();
    json!({ "route": route, "parts": parts, "length": number(offset), "gaps": gaps.into_iter().map(|(_, g)| g).collect::<Vec<_>>(), "ok": ok })
}

/// A part's own station at `s` along the route (`partStation`).
pub fn part_station(part: &Value, s: f64) -> f64 {
    if part["reversed"].as_bool().unwrap_or(false) {
        num(part, "length").unwrap_or(f64::NAN) - s
    } else {
        s
    }
}

/// Where `s` along a resolved route lies (`routeAt`).
pub fn route_at(resolved: &Value, s: f64) -> Value {
    let parts = list(resolved, "parts");
    if parts.is_empty() {
        return Value::Null;
    }
    let total = num(resolved, "length").unwrap_or(f64::NAN);
    let at = 0f64.max(total.min(s));
    let index = parts
        .iter()
        .position(|p| at < num(p, "offset").unwrap_or(0.0) + num(p, "length").unwrap_or(0.0))
        .unwrap_or(parts.len() - 1);
    let part = &parts[index];
    let local = 0f64.max(num(part, "length").unwrap_or(0.0).min(at - num(part, "offset").unwrap_or(0.0)));
    json!({ "part": part, "index": index, "trackId": part["trackId"], "station": number(part_station(part, local)) })
}

/// The route station of a track station (`routeStationOf`).
pub fn route_station_of(resolved: &Value, track_id: &Value, station: f64) -> Option<f64> {
    let part = list(resolved, "parts").iter().find(|p| strict_eq(p.get("trackId"), Some(track_id)))?;
    let local = 0f64.max(num(part, "length").unwrap_or(0.0).min(station));
    Some(num(part, "offset").unwrap_or(0.0) + part_station(part, local))
}

/// The point at `s` along a resolved route (`routePointAt`).
pub fn route_point_at(resolved: &Value, s: f64) -> Value {
    let at = route_at(resolved, s);
    if at.is_null() {
        return Value::Null;
    }
    let station = at["station"].as_f64().unwrap_or(f64::NAN);
    let Some((utm, bearing)) = point_at_station(&at["part"]["track"], station) else {
        return Value::Null;
    };
    let reversed = at["part"]["reversed"].as_bool().unwrap_or(false);
    json!({ "utm": utm.to_json(), "bearing": number(if reversed { (bearing + 180.0) % 360.0 } else { bearing }), "trackId": at["trackId"], "station": number(station) })
}

/// The shortest run of tracks from one to another (`routePath`).
pub fn route_path(tracks: &[Value], switches: &[Value], from: &Value, exits: &[&str], to: &Value) -> Option<Vec<Value>> {
    let by_id: HashMap<String, &Value> = tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
    struct Node {
        cost: f64,
        prev: Option<String>,
        track: Value,
    }
    let mut best: HashMap<String, Node> = HashMap::new();
    let mut queue: Vec<(String, f64)> = Vec::new();
    let push = |best: &mut HashMap<String, Node>, queue: &mut Vec<(String, f64)>, track: Value, entry: &str, cost: f64, prev: Option<String>| {
        let key = format!("{}|{entry}", js_string(Some(&track)));
        if best.get(&key).is_some_and(|n| n.cost <= cost) {
            return;
        }
        best.insert(key.clone(), Node { cost, prev, track });
        queue.push((key, cost));
    };
    for exit in exits {
        for (t, e) in tracks_on_from(from, exit, tracks, switches) {
            push(&mut best, &mut queue, t, &e, 0.0, None);
        }
    }
    while !queue.is_empty() {
        queue.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let (key, cost) = queue.remove(0);
        let (node_cost, node_track) = {
            let n = &best[&key];
            (n.cost, n.track.clone())
        };
        if node_cost < cost {
            continue;
        }
        if strict_eq(Some(&node_track), Some(to)) {
            let mut path = Vec::new();
            let mut k = Some(key);
            while let Some(kk) = k {
                let n = &best[&kk];
                path.insert(0, n.track.clone());
                k = n.prev.clone();
            }
            return Some(path);
        }
        if strict_eq(Some(&node_track), Some(from)) {
            continue;
        }
        let Some(track) = by_id.get(&js_string(Some(&node_track))) else { continue };
        let entry = key[key.rfind('|').map(|p| p + 1).unwrap_or(0)..].to_string();
        let next = cost + track_length(track);
        for (t, e) in tracks_on_from(&node_track, other(&entry), tracks, switches) {
            push(&mut best, &mut queue, t, &e, next, Some(key.clone()));
        }
    }
    None
}

/// A route extended to a track along the shortest way (`extendRoute`).
pub fn extend_route(track_ids: &[Value], track_id: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    if track_ids.is_empty() {
        return json!({ "trackIds": [track_id], "added": [track_id] });
    }
    let last = &track_ids[track_ids.len() - 1];
    if strict_eq(Some(last), Some(track_id)) {
        return json!({ "trackIds": track_ids, "added": [] });
    }
    let resolved = resolve_route(&json!({ "trackIds": track_ids }), tracks, switches);
    let parts = list(&resolved, "parts");
    let exits: Vec<&str> = match parts.last() {
        None => vec![],
        Some(_) if parts.len() == 1 => vec!["END", "BEGIN"],
        Some(p) => vec![if p["reversed"].as_bool().unwrap_or(false) { "BEGIN" } else { "END" }],
    };
    match route_path(tracks, switches, last, &exits, track_id) {
        None => json!({ "trackIds": track_ids, "added": [] }),
        Some(path) => {
            let mut ids = track_ids.to_vec();
            ids.extend(path.iter().cloned());
            json!({ "trackIds": ids, "added": path })
        }
    }
}

/// A route's gaps closed along the shortest ways (`closeGaps`).
pub fn close_gaps(track_ids: &[Value], tracks: &[Value], switches: &[Value]) -> Value {
    let resolved = resolve_route(&json!({ "trackIds": track_ids }), tracks, switches);
    let parts = list(&resolved, "parts");
    let gaps = list(&resolved, "gaps");
    let ids: Vec<Value> = parts.iter().map(|p| p["trackId"].clone()).collect();
    let gap_at = |i: i64, kind: Option<&str>| {
        gaps.iter()
            .any(|g| g["after"].as_i64() == Some(i) && kind.is_none_or(|k| g["kind"].as_str() == Some(k)))
    };
    let (mut out, mut closed, mut open) = (Vec::new(), 0, 0);
    for (i, id) in ids.iter().enumerate() {
        out.push(id.clone());
        if !gap_at(i as i64, Some("disconnected")) {
            continue;
        }
        let entered = i > 0 && !gap_at(i as i64 - 1, None);
        let exits: Vec<&str> = if entered {
            vec![if parts[i]["reversed"].as_bool().unwrap_or(false) { "BEGIN" } else { "END" }]
        } else {
            vec!["END", "BEGIN"]
        };
        match route_path(tracks, switches, id, &exits, &ids[i + 1]) {
            None => open += 1,
            Some(path) => {
                out.extend(path[..path.len() - 1].iter().cloned());
                closed += 1;
            }
        }
    }
    json!({ "trackIds": out, "closed": closed, "open": open })
}

const ON_TRACK: f64 = 0.05;
const SAMPLE_EVERY: f64 = 5.0;

/// The tracks that took over a stretch of an old one, in order along it (`piecesAlong`).
pub fn pieces_along(old: &Value, candidates: &[Value]) -> Vec<Value> {
    let length = track_length(old);
    if !(length > 0.0) {
        return vec![];
    }
    let cands: Vec<&Value> = candidates
        .iter()
        .filter(|c| strict_eq(c.get("epsg"), old.get("epsg")) && !list(c, "elements").is_empty())
        .collect();
    if cands.is_empty() {
        return vec![];
    }
    let mut stations = Vec::new();
    let mut s = 0.05;
    while s < length - 0.05 {
        stations.push(s);
        s += SAMPLE_EVERY;
    }
    stations.push(0f64.max(length - 0.05));
    for c in &cands {
        if let Some((mid, _)) = point_at_station(c, track_length(c) / 2.0)
            && let Some((st, d, _)) = station_on_track(old, mid)
            && d < ON_TRACK
        {
            stations.push(st);
        }
    }
    stations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<Value> = Vec::new();
    for s in stations {
        let Some((p, _)) = point_at_station(old, s) else { continue };
        let mut hit: Option<(Value, f64)> = None;
        for c in &cands {
            if let Some((_, d, _)) = station_on_track(c, p)
                && d < ON_TRACK
                && hit.as_ref().is_none_or(|h| d < h.1)
            {
                hit = Some((c["id"].clone(), d));
            }
        }
        if let Some((id, _)) = hit
            && !out.last().is_some_and(|l| strict_eq(Some(l), Some(&id)))
        {
            out.push(id);
        }
    }
    out
}

fn meet(a: &Value, b: &Value, tracks: &[Value], switches: &[Value]) -> bool {
    !links(a, b, tracks, switches).is_empty()
}

/// A route whose tracks were replaced, on the tracks that took them over (`repairRoute`).
pub fn repair_route(route: &Value, tracks: &[Value], switches: &[Value], old_track: &dyn Fn(&Value) -> Option<Value>, candidates: &[Value]) -> Value {
    repair_route_with(route, tracks, switches, old_track, &|_| candidates.to_vec())
}

/// The same with the tracks each one may have become looked up per track (`repairRoute`'s `candidates(id)`).
pub fn repair_route_with(
    route: &Value,
    tracks: &[Value],
    switches: &[Value],
    old_track: &dyn Fn(&Value) -> Option<Value>,
    candidates: &dyn Fn(&Value) -> Vec<Value>,
) -> Value {
    let present: Vec<String> = tracks.iter().map(|t| js_string(t.get("id"))).collect();
    let ids = list(route, "trackIds");
    if ids.iter().all(|id| present.contains(&js_string(Some(id)))) {
        return route.clone();
    }
    let mut out: Vec<Value> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        if present.contains(&js_string(Some(id))) {
            out.push(id.clone());
            continue;
        }
        let mut pieces = old_track(id).map(|old| pieces_along(&old, &candidates(id))).unwrap_or_default();
        if pieces.len() > 1 {
            let prev = out.last().cloned();
            let next = ids[i + 1..].iter().find(|x| present.contains(&js_string(Some(x)))).cloned();
            let forward = match (&prev, &next) {
                (Some(p), _) => meet(p, &pieces[0], tracks, switches),
                (None, Some(n)) => meet(&pieces[pieces.len() - 1], n, tracks, switches),
                _ => true,
            };
            if !forward {
                pieces.reverse();
            }
        }
        for p in pieces {
            if !out.last().is_some_and(|l| strict_eq(Some(l), Some(&p))) {
                out.push(p);
            }
        }
    }
    with(route, "trackIds", Value::Array(out))
}

/// Every route of `after` repaired where tracks it named were replaced (`repairRoutes`).
pub fn repair_routes(before: &Value, after: &Value) -> Value {
    let routes = list(after, "routes");
    if routes.is_empty() {
        return after.clone();
    }
    let tracks = list(after, "tracks");
    let switches = list(after, "switches");
    let present: Vec<String> = tracks.iter().map(|t| js_string(t.get("id"))).collect();
    if routes
        .iter()
        .all(|r| list(r, "trackIds").iter().all(|id| present.contains(&js_string(Some(id)))))
    {
        return after.clone();
    }
    let old: Vec<&Value> = list(before, "tracks").iter().collect();
    let touched: Vec<Value> = tracks
        .iter()
        .filter(|t| old.iter().find(|o| strict_eq(o.get("id"), t.get("id"))).is_none_or(|o| *o != *t))
        .cloned()
        .collect();
    let old_track = |id: &Value| old.iter().find(|t| strict_eq(t.get("id"), Some(id))).map(|t| (*t).clone());
    let next: Vec<Value> = routes.iter().map(|r| repair_route(r, tracks, switches, &old_track, &touched)).collect();
    if next.iter().zip(routes).all(|(a, b)| a == b) {
        return after.clone();
    }
    let mut o = obj(after);
    o.insert("routes".into(), Value::Array(next));
    Value::Object(o)
}

/// `Route n` not taken (`nextRouteName`).
pub fn next_route_name(routes: &[Value], base: &str) -> String {
    let taken: Vec<String> = routes.iter().map(|r| js_string(r.get("name"))).collect();
    let mut n = routes.len() + 1;
    while taken.contains(&format!("{base} {n}")) {
        n += 1;
    }
    format!("{base} {n}")
}
