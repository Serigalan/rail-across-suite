// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Platforms, and points along a track by station (`platformUtils.js`).

use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, bearing_after, end_point_curved, end_point_straight, node_utm, project_on_arc};
use rail_across_geom::kind::is_transition;
use rail_across_geom::precision::{SAGITTA_ELEMENT, STRAIGHT_VERTEX_SPACING};
use rail_across_geom::transition::{is_bloss, transition_bearing_at, transition_point_at};
use rail_across_js::{get, js_number, number, strict_eq, truthy};
use serde_json::{Value, json};

use crate::heights::height_at;
use crate::util::{elements, epsg, list, num, obj, text, with, without};

const DEG: f64 = std::f64::consts::PI / 180.0;
pub const PLATFORM_FRONT_OFFSET: f64 = 1.68;
pub const PLATFORM_WIDTH: f64 = 3.0;
pub const PLATFORM_HEIGHTS: [f64; 3] = [380.0, 550.0, 760.0];
pub const DEFAULT_PLATFORM_HEIGHT: f64 = 550.0;
pub const PLATFORM_CODE_MAX: i64 = 4;

/// Each element with the stations it runs between (`elementStations`): (index, start, end).
pub fn element_stations(track: &Value) -> Vec<(usize, f64, f64)> {
    let mut station = 0.0;
    elements(track)
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let start = station;
            station += num(el, "length").unwrap_or(0.0);
            (i, start, station)
        })
        .collect()
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    hi.min(lo.max(v))
}

/// A point and its bearing at `s` along an element (`pointOnElement`).
pub fn point_on_element(el: &Value, epsg: i64, s: f64) -> Option<(Utm, f64)> {
    let first = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0));
    let start = node_utm(el.get("startNode"), first, epsg).ok()?;
    let bearing = el.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
    if is_transition(el) {
        let bloss = is_bloss(text(el, "transitionType"));
        let [e, n] = transition_point_at(start.e, start.n, bearing, length, num(el, "r1"), num(el, "r2"), bloss, s);
        return Some((
            Utm { e, n, zone: epsg },
            transition_bearing_at(bearing, length, num(el, "r1"), num(el, "r2"), bloss, s),
        ));
    }
    if let Some(r) = num(el, "radius") {
        return Some((end_point_curved(start, bearing, s, r), bearing_after(bearing, s, Some(r))));
    }
    Some((end_point_straight(start, bearing, s), bearing))
}

/// `pointOnElement` as the JavaScript returns it.
pub fn point_json(p: Option<(Utm, f64)>) -> Value {
    match p {
        Some((utm, bearing)) => json!({ "utm": utm.to_json(), "bearing": number(bearing) }),
        None => Value::Null,
    }
}

/// The point at a station of a track (`pointAtStation`).
pub fn point_at_station(track: &Value, station: f64) -> Option<(Utm, f64)> {
    let rows = element_stations(track);
    if rows.is_empty() || !truthy(track.get("epsg")) {
        return None;
    }
    let total = rows[rows.len() - 1].2;
    let s = clamp(station, 0.0, total);
    let row = rows.iter().find(|r| s <= r.2 + 1e-9).copied().unwrap_or(rows[rows.len() - 1]);
    point_on_element(&elements(track)[row.0], epsg(track), s - row.1)
}

/// The station nearest a plane point (`stationOnTrack`): (station, dist, bearing).
pub fn station_on_track(track: &Value, utm: Utm) -> Option<(f64, f64, f64)> {
    let mut best: Option<(f64, f64, f64)> = None;
    for (i, _, _) in element_stations(track) {
        let Some(s) = station_from_click(track, i, utm) else { continue };
        let Some((p, bearing)) = point_at_station(track, s) else { continue };
        let d = (p.e - utm.e).hypot(p.n - utm.n);
        if best.is_none_or(|b| d < b.1) {
            best = Some((s, d, bearing));
        }
    }
    best
}

fn sample_step(el: &Value, dist: f64) -> f64 {
    let step_for = |radius: f64| -> Option<f64> {
        let r = radius.abs();
        if !(r > 0.5) {
            return None;
        }
        Some(0.5f64.max(r * 2.0 * clamp(1.0 - SAGITTA_ELEMENT / r, -1.0, 1.0).acos()))
    };
    if let Some(r) = num(el, "radius") {
        return step_for(r - dist).unwrap_or(STRAIGHT_VERTEX_SPACING);
    }
    if is_transition(el) {
        let steps: Vec<f64> = ["r1", "r2"]
            .iter()
            .filter_map(|k| num(el, k).filter(|r| r.is_finite()))
            .map(|r| r - dist)
            .filter_map(step_for)
            .collect();
        return if steps.is_empty() {
            5.0
        } else {
            steps.iter().copied().fold(f64::INFINITY, f64::min)
        };
    }
    STRAIGHT_VERTEX_SPACING
}

/// A polyline `dist` beside a track between two stations, in WGS 84 (`offsetEdgeCoords`).
pub fn offset_edge_coords(track: &Value, from: f64, to: f64, dist: f64) -> Option<Vec<Value>> {
    let e = epsg(track);
    let rows = element_stations(track);
    if !truthy(track.get("epsg")) || rows.is_empty() {
        return None;
    }
    let total = rows[rows.len() - 1].2;
    let a = clamp(from.min(to), 0.0, total);
    let b = clamp(from.max(to), 0.0, total);
    if !(b - a > 1e-6) {
        return None;
    }
    let mut out: Vec<[f64; 2]> = Vec::new();
    for (i, start, end) in rows {
        let el = &elements(track)[i];
        let f = a.max(start);
        let t = b.min(end);
        if t - f <= 1e-9 {
            continue;
        }
        let n = 1usize.max(((t - f) / sample_step(el, dist)).ceil() as usize);
        for k in 0..=n {
            let s = f - start + (t - f) * k as f64 / n as f64;
            let Some((utm, bearing)) = point_on_element(el, e, s) else { continue };
            let rad = bearing * DEG;
            let Ok((lng, lat)) = plane_to_wgs84(utm.e + dist * rad.cos(), utm.n - dist * rad.sin(), e) else {
                continue;
            };
            if out.last().is_none_or(|l| l[0] != lng || l[1] != lat) {
                out.push([lng, lat]);
            }
        }
    }
    if out.len() >= 2 {
        Some(out.into_iter().map(|[a, b]| json!([number(a), number(b)])).collect())
    } else {
        None
    }
}

/// The signed offsets of a platform's front and back edge (`edgeOffsets`).
pub fn edge_offsets(platform: &Value) -> (f64, f64) {
    let sign = if text(platform, "side") == Some("left") { -1.0 } else { 1.0 };
    let front = num(platform, "frontOffset").unwrap_or(PLATFORM_FRONT_OFFSET);
    let back = num(platform, "backOffset").unwrap_or(front + PLATFORM_WIDTH);
    (sign * front, sign * back)
}

/// The height of a platform's edge at a station (`platformEdgeElevation`).
pub fn platform_edge_elevation(platform: &Value, track: &Value, station: f64) -> Option<f64> {
    let zz = height_at(list(track, "heights"), station)?;
    let height = js_number(platform.get("height"));
    if height.is_finite() { Some(zz + height / 1000.0) } else { None }
}

/// The outline of a platform (`platformRing`).
pub fn platform_ring(platform: &Value, track: &Value) -> Option<Vec<Value>> {
    let (front, back) = edge_offsets(platform);
    let s = num(platform, "startStation").unwrap_or(f64::NAN);
    let t = num(platform, "endStation").unwrap_or(f64::NAN);
    let f = offset_edge_coords(track, s, t, front)?;
    let mut b = offset_edge_coords(track, s, t, back)?;
    b.reverse();
    let mut ring = f.clone();
    ring.extend(b);
    ring.push(f[0].clone());
    Some(ring)
}

/// A platform with its outline derived again (`rebuildPlatformSymbol`).
pub fn rebuild_platform_symbol(platform: &Value, tracks: &[Value]) -> Value {
    let rest = Value::Object(without(platform, &["coords"]));
    let Some(track) = tracks.iter().find(|t| strict_eq(t.get("id"), platform.get("trackId"))) else {
        return rest;
    };
    match platform_ring(&rest, track) {
        Some(ring) => with(&rest, "coords", Value::Array(ring)),
        None => rest,
    }
}

/// The station on element `el_idx` nearest a click (`stationFromClick`).
pub fn station_from_click(track: &Value, el_idx: usize, click: Utm) -> Option<f64> {
    let (_, start, _) = *element_stations(track).get(el_idx)?;
    if !truthy(track.get("epsg")) {
        return None;
    }
    let el = &elements(track)[el_idx];
    let e = epsg(track);
    let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let along = if is_transition(el) {
        let dist_at = |s: f64| {
            point_on_element(el, e, s)
                .map(|(p, _)| (p.e - click.e).hypot(p.n - click.n))
                .unwrap_or(f64::INFINITY)
        };
        let n = 64;
        let mut best = f64::INFINITY;
        let mut along = 0.0;
        for i in 0..=n {
            let s = length * f64::from(i) / f64::from(n);
            let d = dist_at(s);
            if d < best {
                best = d;
                along = s;
            }
        }
        let mut lo = 0f64.max(along - length / f64::from(n));
        let mut hi = length.min(along + length / f64::from(n));
        let mut i = 0;
        while i < 40 && hi - lo > 1e-4 {
            let m1 = lo + (hi - lo) / 3.0;
            let m2 = hi - (hi - lo) / 3.0;
            if dist_at(m1) < dist_at(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
            i += 1;
        }
        (lo + hi) / 2.0
    } else {
        let first = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0));
        let s = node_utm(el.get("startNode"), first, e).ok()?;
        project_on_arc(s, click, el.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN), num(el, "radius")).0
    };
    Some(start + clamp(along, 0.0, length))
}

/// The length of a platform (`platformLength`).
pub fn platform_length(platform: &Value) -> f64 {
    (num(platform, "endStation").unwrap_or(0.0) - num(platform, "startStation").unwrap_or(0.0)).abs()
}

const CARRY_TOL: f64 = 1e-6;

/// Platforms carried onto the tracks that took over their stretch (`carryPlatforms`).
pub fn carry_platforms(platforms: &Value, carries: &[Value]) -> Value {
    let Some(list) = platforms.as_array().filter(|l| !l.is_empty()) else {
        return platforms.clone();
    };
    if carries.is_empty() {
        return platforms.clone();
    }
    let mut out = Vec::new();
    for pl in list {
        let mine: Vec<&Value> = carries.iter().filter(|c| strict_eq(c.get("oldId"), pl.get("trackId"))).collect();
        if mine.is_empty() {
            out.push(pl.clone());
            continue;
        }
        let a = num(pl, "startStation").unwrap_or(0.0);
        let b = num(pl, "endStation").unwrap_or(0.0);
        let (lo, hi) = (a.min(b), a.max(b));
        let Some(c) = mine
            .iter()
            .find(|c| lo >= num(c, "from").unwrap_or(f64::NAN) - CARRY_TOL && hi <= num(c, "to").unwrap_or(f64::NAN) + CARRY_TOL)
        else {
            continue;
        };
        let flip = truthy(c.get("flip"));
        let offset = num(c, "offset").unwrap_or(f64::NAN);
        let at = |s: f64| if flip { offset - s } else { offset + s };
        let (start, end) = if flip { (at(b), at(a)) } else { (at(a), at(b)) };
        let mut o = obj(pl);
        o.insert("trackId".into(), c.get("newId").cloned().unwrap_or(Value::Null));
        o.insert("startStation".into(), number(start));
        o.insert("endStation".into(), number(end));
        if flip && truthy(pl.get("side")) {
            o.insert("side".into(), json!(if text(pl, "side") == Some("left") { "right" } else { "left" }));
        }
        out.push(Value::Object(o));
    }
    Value::Array(out)
}

/// Platforms of a track turned around (`flipPlatforms`).
pub fn flip_platforms(platforms: &Value, track_id: &Value, length: f64) -> Value {
    carry_platforms(
        platforms,
        &[json!({ "oldId": track_id, "newId": track_id, "from": 0, "to": number(length), "offset": number(length), "flip": true })],
    )
}

/// `get` re-exported for the crate's callers.
pub fn member<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    get(v, k)
}
