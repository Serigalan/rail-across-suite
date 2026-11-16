// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Stationing and the vertical alignment (`heightUtils.js`): points along a
//! track, height points with their vertical curves, splitting and joining
//! them, the tracks joined at an end and the joint groups of height points.

use rail_across_geom::kind::is_transition;
use std::collections::{HashMap, HashSet, VecDeque};

use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, end_point_curved, end_point_straight, node_utm};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{is_bloss, transition_point_at};
use rail_across_js::{get, has, js_round, number, strict_eq};
use serde_json::{Value, json};

use crate::ids::{HEIGHT_POINT_SPACING, HEIGHT_SPLIT_MIN};
use crate::switch_model::{port_end, port_track, ports_of};
use crate::util::{elements, epsg, list, num, obj, text, with};

const STATION_TOL: f64 = 1e-6;
const JOINT_TOL: f64 = 0.01;

/// The sum of a track's element lengths (`trackLength`).
pub fn track_length(track: &Value) -> f64 {
    elements(track).iter().map(|el| num(el, "length").unwrap_or(0.0)).sum()
}

/// The stations height points go at along a length (`heightStations`).
pub fn height_stations(length: f64) -> Vec<f64> {
    if !(length > 0.0) {
        return vec![];
    }
    if length <= HEIGHT_SPLIT_MIN {
        return vec![0.0, length];
    }
    let n = (length / HEIGHT_POINT_SPACING).ceil() as usize;
    (0..=n).map(|i| if i == n { length } else { length * i as f64 / n as f64 }).collect()
}

fn coords(el: &Value) -> &[Value] {
    el.get("geometry")
        .and_then(|g| g.get("coordinates"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The plane point at `s` along an element (`pointAtStationUtm`).
pub fn point_at_station_utm(el: &Value, s: f64, epsg: i64) -> Option<Utm> {
    let c = coords(el);
    let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
    if s >= length && get(el, "endNode").is_some() {
        return node_utm(el.get("endNode"), c.last(), epsg).ok();
    }
    let start = node_utm(el.get("startNode"), c.first(), epsg).ok()?;
    if s <= 0.0 {
        return Some(start);
    }
    let bearing = el.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN);
    if is_transition(el) && has(el, "r1") {
        let [e, n] = transition_point_at(
            start.e,
            start.n,
            bearing,
            length,
            num(el, "r1"),
            num(el, "r2"),
            is_bloss(text(el, "transitionType")),
            s,
        );
        return Some(Utm { e, n, zone: start.zone });
    }
    if let Some(r) = num(el, "radius") {
        return Some(end_point_curved(start, bearing, s, r));
    }
    Some(end_point_straight(start, bearing, s))
}

/// The element a station falls on, and how far along it (`elementAtStation`): (index, s).
pub fn element_at_station(els: &[Value], station: f64) -> Option<(usize, f64)> {
    let mut start = 0.0;
    for (i, el) in els.iter().enumerate() {
        let length = num(el, "length").unwrap_or(0.0);
        if station <= start + length + STATION_TOL {
            return Some((i, 0f64.max(station - start)));
        }
        start += length;
    }
    let last = els.len().checked_sub(1)?;
    Some((last, num(&els[last], "length").unwrap_or(0.0)))
}

/// `elementAtStation` as the JavaScript returns it.
pub fn element_at_station_json(els: &[Value], station: f64) -> Value {
    match element_at_station(els, station) {
        Some((i, s)) => json!({ "el": els[i], "elIdx": i, "s": number(s) }),
        None => Value::Null,
    }
}

/// Points of a track at stations, in WGS 84 (`trackSamplePoints`).
pub fn track_sample_points(track: &Value, stations: &[f64]) -> Value {
    let e = epsg(track);
    Value::Array(
        stations
            .iter()
            .map(|&station| {
                let p = element_at_station(elements(track), station).and_then(|(i, s)| point_at_station_utm(&elements(track)[i], s, e));
                let ll = p
                    .and_then(|p| plane_to_wgs84(p.e, p.n, e).ok())
                    .map(|(a, b)| json!([number(a), number(b)]))
                    .unwrap_or(Value::Null);
                json!({ "station": number(station), "lngLat": ll })
            })
            .collect(),
    )
}

fn st(p: &Value) -> f64 {
    p.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN)
}
fn z(p: &Value) -> f64 {
    p.get("z").and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// The height at a station, linear between the points (`heightAt`).
pub fn height_at(heights: &[Value], station: f64) -> Option<f64> {
    let first = heights.first()?;
    if station <= st(first) {
        return Some(z(first));
    }
    for i in 1..heights.len() {
        let (a, b) = (&heights[i - 1], &heights[i]);
        if station <= st(b) {
            let t = if st(b) == st(a) { 0.0 } else { (station - st(a)) / (st(b) - st(a)) };
            return Some(z(a) + (z(b) - z(a)) * t);
        }
    }
    Some(z(&heights[heights.len() - 1]))
}

/// The grades either side of point `i` (`pointGrades`).
pub fn point_grades(heights: &[Value], i: i64) -> Value {
    let at = |k: i64| if k >= 0 { heights.get(k as usize) } else { None };
    let (p, a, b) = (at(i), at(i - 1), at(i + 1));
    let grade = |u: Option<&Value>, v: Option<&Value>| match (u, v) {
        (Some(u), Some(v)) if st(v) > st(u) => number((z(v) - z(u)) / (st(v) - st(u))),
        _ => Value::Null,
    };
    json!({ "before": if p.is_some() { grade(a, p) } else { Value::Null }, "after": if p.is_some() { grade(p, b) } else { Value::Null } })
}

/// A height kept to 0.1 mm (`roundHeight`).
pub fn round_height(z: f64) -> f64 {
    js_round(z * 1e4) / 1e4
}

/// A height point from two of station, height and the grades either side (`solveHeightPoint`).
pub fn solve_height_point(heights: &[Value], index: usize, given: &Value, length: f64, min_gap: f64) -> Value {
    let a = index.checked_sub(1).and_then(|k| heights.get(k));
    let b = heights.get(index + 1);
    let g = |k: &str| get(given, k).and_then(Value::as_f64).filter(|x| x.is_finite());
    let err = |e: &str| json!({ "error": e });
    if (g("gb").is_some() && a.is_none()) || (g("ga").is_some() && b.is_none()) {
        return err("missing");
    }
    let (s, zz);
    match (g("s"), g("z"), g("gb"), g("ga")) {
        (Some(s0), Some(z0), _, _) => {
            s = s0;
            zz = z0;
        }
        (Some(s0), None, Some(gb), _) => {
            s = s0;
            zz = z(a.unwrap()) + gb * (s - st(a.unwrap()));
        }
        (Some(s0), None, None, Some(ga)) => {
            s = s0;
            zz = z(b.unwrap()) - ga * (st(b.unwrap()) - s);
        }
        (None, Some(z0), Some(gb), _) => {
            if gb.abs() < 1e-9 {
                return err("flat");
            }
            zz = z0;
            s = st(a.unwrap()) + (zz - z(a.unwrap())) / gb;
        }
        (None, Some(z0), None, Some(ga)) => {
            if ga.abs() < 1e-9 {
                return err("flat");
            }
            zz = z0;
            s = st(b.unwrap()) - (z(b.unwrap()) - zz) / ga;
        }
        (None, None, Some(gb), Some(ga)) => {
            if (gb - ga).abs() < 1e-9 {
                return err("parallel");
            }
            let (a, b) = (a.unwrap(), b.unwrap());
            s = (z(b) - z(a) + gb * st(a) - ga * st(b)) / (gb - ga);
            zz = z(a) + gb * (s - st(a));
        }
        _ => return err("missing"),
    }
    let station = js_round(s * 1000.0) / 1000.0;
    if !station.is_finite()
        || station < -1e-9
        || station > length + 1e-9
        || a.is_some_and(|a| station < st(a) + min_gap - 1e-9)
        || b.is_some_and(|b| station > st(b) - min_gap + 1e-9)
    {
        return err("order");
    }
    json!({ "station": number(station), "z": number(round_height(zz)) })
}

/// A height point inserted at a station, on the line between its neighbours (`insertHeightPoint`).
pub fn insert_height_point(heights: &[Value], station: f64, min_gap: f64) -> Value {
    if heights.len() < 2 {
        return Value::Null;
    }
    let Some(i) = heights.iter().position(|p| st(p) > station).filter(|i| *i > 0) else {
        return Value::Null;
    };
    if station - st(&heights[i - 1]) < min_gap || st(&heights[i]) - station < min_gap {
        return Value::Null;
    }
    let zz = round_height(height_at(heights, station).unwrap_or(f64::NAN));
    let mut out = heights[..i].to_vec();
    out.push(json!({ "station": number(station), "z": number(zz) }));
    out.extend_from_slice(&heights[i..]);
    json!({ "heights": out, "index": i })
}

fn at_station(p: &Value, station: f64) -> Value {
    with(p, "station", number(station))
}

/// Two halves of a vertical alignment, none where a side has none.
pub type Halves = (Option<Vec<Value>>, Option<Vec<Value>>);

/// The heights of a track parted at `sj` (`splitHeights`): the two halves, none where a side has none.
pub fn split_heights(heights: Option<&[Value]>, sj: f64) -> Halves {
    let Some(h) = heights.filter(|h| !h.is_empty()) else { return (None, None) };
    if sj < st(&h[0]) - STATION_TOL {
        return (None, Some(h.iter().map(|p| at_station(p, st(p) - sj)).collect()));
    }
    if sj > st(&h[h.len() - 1]) + STATION_TOL {
        return (Some(h.to_vec()), None);
    }
    if let Some(curve) = split_curve_at(h, sj) {
        return curve;
    }
    let at = h
        .iter()
        .find(|p| (st(p) - sj).abs() <= STATION_TOL)
        .cloned()
        .unwrap_or_else(|| json!({ "z": height_at(h, sj).map(number).unwrap_or(Value::Null) }));
    let mut a: Vec<Value> = h.iter().filter(|p| st(p) < sj - STATION_TOL).cloned().collect();
    a.push(at_station(&at, sj));
    let mut b = vec![at_station(&at, 0.0)];
    b.extend(h.iter().filter(|p| st(p) > sj + STATION_TOL).map(|p| at_station(p, st(p) - sj)));
    (Some(a), Some(b))
}

fn split_curve_at(h: &[Value], sj: f64) -> Option<Halves> {
    for i in 1..h.len().saturating_sub(1) {
        let t = tangent_length(h, i);
        let p = &h[i];
        let x = sj - st(p);
        let Some(t) = t.filter(|t| *t != 0.0) else { continue };
        if x.abs() >= t - STATION_TOL {
            continue;
        }
        let (before, after) = gradients(h, i)?;
        let delta = after - before;
        let s_in = st(p) - t;
        let s_out = st(p) + t;
        let z_in = z(p) - before * t;
        let z_cut = z(p) + before * x + (x + t).powf(2.0) * delta / (4.0 * t);
        let g_cut = before + (x + t) * delta / (2.0 * t);
        let cut = json!({ "station": number(sj), "z": number(z_cut) });
        let rv = p.get("rv").cloned();
        let mut a: Vec<Value> = h[..i].to_vec();
        if sj - s_in > STATION_TOL {
            let mut o = json!({ "station": number((s_in + sj) / 2.0), "z": number(z_in + before * (sj - s_in) / 2.0) });
            if let Some(rv) = &rv {
                o["rv"] = rv.clone();
            }
            a.push(o);
        }
        a.push(cut.clone());
        let mut b = vec![at_station(&cut, 0.0)];
        if s_out - sj > STATION_TOL {
            let mut o = json!({ "station": number((s_out - sj) / 2.0), "z": number(z_cut + g_cut * (s_out - sj) / 2.0) });
            if let Some(rv) = &rv {
                o["rv"] = rv.clone();
            }
            b.push(o);
        }
        b.extend(h[i + 1..].iter().map(|q| at_station(q, st(q) - sj)));
        return Some((Some(a), Some(b)));
    }
    None
}

/// The heights of two tracks joined end to end (`joinHeights`).
pub fn join_heights(a: Option<&[Value]>, b: Option<&[Value]>, a_length: f64) -> Option<Vec<Value>> {
    let left = a.unwrap_or(&[]).to_vec();
    let right: Vec<Value> = b.unwrap_or(&[]).iter().map(|p| at_station(p, st(p) + a_length)).collect();
    if left.is_empty() && right.is_empty() {
        return None;
    }
    let last = left.last().map(st);
    let mut points = left.clone();
    points.extend(right.into_iter().filter(|p| last.is_none_or(|l| st(p) > l + STATION_TOL)));
    Some(mend_curve_at(points, left.len() as i64 - 1))
}

fn rv(p: Option<&Value>) -> Option<f64> {
    p.and_then(|p| get(p, "rv")).and_then(Value::as_f64).filter(|r| *r != 0.0 && !r.is_nan())
}

fn mend_curve_at(points: Vec<Value>, j: i64) -> Vec<Value> {
    if j < 1 {
        return points;
    }
    let j = j as usize;
    let (a, c, b) = (points.get(j - 1), points.get(j), points.get(j + 1));
    let (Some(arv), Some(brv)) = (rv(a), rv(b)) else { return points };
    if rv(c).is_some() || (arv.abs() - brv.abs()).abs() > 1e-9 * arv.abs() {
        return points;
    }
    let (a, c, b) = (a.unwrap(), c.unwrap(), b.unwrap());
    let (Some(ta), Some(tb)) = (
        tangent_length(&points, j - 1).filter(|t| *t != 0.0),
        tangent_length(&points, j + 1).filter(|t| *t != 0.0),
    ) else {
        return points;
    };
    if (ta - (st(c) - st(a))).abs() > STATION_TOL || (tb - (st(b) - st(c))).abs() > STATION_TOL {
        return points;
    }
    let Some(g) = gradients(&points, j) else { return points };
    if (g.1 - g.0).abs() > 1e-9 {
        return points;
    }
    let (Some(g_in), Some(g_out)) = (gradients(&points, j - 1).map(|g| g.0), gradients(&points, j + 1).map(|g| g.1)) else {
        return points;
    };
    if (g_out - g_in).abs() < 1e-12 {
        return points;
    }
    let station = (z(b) - z(a) + g_in * st(a) - g_out * st(b)) / (g_in - g_out);
    let zz = z(a) + g_in * (station - st(a));
    let mut out = points[..j - 1].to_vec();
    out.push(json!({ "station": number(station), "z": number(zz), "rv": a["rv"] }));
    out.extend_from_slice(&points[j + 2..]);
    out
}

/// The heights up to `at` (`truncateHeights`).
pub fn truncate_heights(heights: Option<&[Value]>, at: f64) -> Option<Vec<Value>> {
    if heights.is_none_or(|h| h.is_empty()) || !(at > 0.0) {
        return None;
    }
    split_heights(heights, at).0.filter(|k| k.len() >= 2)
}

/// The heights between two stations, restationed from the first (`sliceHeights`).
pub fn slice_heights(heights: Option<&[Value]>, from: f64, to: f64) -> Option<Vec<Value>> {
    let (head, _) = split_heights(heights, to);
    let (_, piece) = split_heights(head.as_deref(), from);
    piece.filter(|p| p.len() >= 2)
}

/// The heights of a track turned around (`reverseHeights`).
pub fn reverse_heights(heights: Option<&Value>, length: f64) -> Option<Value> {
    let h = heights?;
    match h.as_array() {
        Some(list) if !list.is_empty() => Some(Value::Array(list.iter().rev().map(|p| at_station(p, length - st(p))).collect())),
        _ => Some(h.clone()),
    }
}

/// The profile of a track: its height points and element boundaries (`trackProfile`).
pub fn track_profile(track: &Value) -> Value {
    let mut station = 0.0;
    let boundaries: Vec<Value> = elements(track)
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let b = json!({ "station": number(station), "elIdx": i, "el": el });
            station += num(el, "length").unwrap_or(0.0);
            b
        })
        .collect();
    let points: Vec<Value> = list(track, "heights").iter().enumerate().map(|(i, p)| with(p, "index", json!(i))).collect();
    json!({ "points": points, "boundaries": boundaries, "length": number(station) })
}

fn same_node(a: Option<&Value>, b: Option<&Value>) -> bool {
    let (Some(a), Some(b)) = (a.and_then(Value::as_array), b.and_then(Value::as_array)) else {
        return false;
    };
    let f = |v: &Vec<Value>, i: usize| v.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
    (f(a, 0) - f(b, 0)).abs() < NODE_TOL && (f(a, 1) - f(b, 1)).abs() < NODE_TOL
}

/// Tracks joined to `track` at its end `end`: through a switch, or end to end (`adjacentTracks`).
pub fn adjacent_tracks<'a>(tracks: &'a [Value], switches: &[Value], track: &Value, end: &str) -> Vec<(&'a Value, String)> {
    let mut found: Vec<(&'a Value, String)> = Vec::new();
    let id = track.get("id");
    let mut add = |other_id: Option<&Value>, endpoint: Option<&str>| {
        if strict_eq(other_id, id) {
            return;
        }
        let Some(other) = tracks.iter().find(|t| strict_eq(t.get("id"), other_id)) else {
            return;
        };
        if !found.iter().any(|(t, _)| strict_eq(t.get("id"), other_id)) {
            found.push((other, endpoint.unwrap_or("").to_string()));
        }
    };
    for sw in switches {
        let ports = ports_of(sw);
        let here = ports.iter().any(|(p, _)| strict_eq(port_track(sw, p), id) && port_end(sw, p) == Some(end));
        if !here {
            continue;
        }
        for (p, _) in &ports {
            if port_track(sw, p).is_some() && !strict_eq(port_track(sw, p), id) {
                add(port_track(sw, p), port_end(sw, p));
            }
        }
    }
    let els = elements(track);
    let node = if end == "BEGIN" {
        els.first().and_then(|e| e.get("startNode"))
    } else {
        els.last().and_then(|e| e.get("endNode"))
    };
    for other in tracks {
        if strict_eq(other.get("id"), id) || epsg(other) != epsg(track) {
            continue;
        }
        let oe = elements(other);
        if same_node(oe.first().and_then(|e| e.get("startNode")), node) {
            add(other.get("id"), Some("BEGIN"));
        } else if same_node(oe.last().and_then(|e| e.get("endNode")), node) {
            add(other.get("id"), Some("END"));
        }
    }
    found.truncate(2);
    found
}

/// The first two height points of a joined track from the joint (`neighbourStub`).
pub fn neighbour_stub(track: &Value, endpoint: &str) -> Value {
    let h = list(track, "heights");
    if h.len() < 2 {
        return json!([]);
    }
    if endpoint == "BEGIN" {
        return json!([{ "d": 0, "z": h[0]["z"] }, { "d": number(st(&h[1]) - st(&h[0])), "z": h[1]["z"] }]);
    }
    let (a, b) = (&h[h.len() - 1], &h[h.len() - 2]);
    json!([{ "d": 0, "z": a["z"] }, { "d": number(st(a) - st(b)), "z": b["z"] }])
}

fn end_index(track: &Value, end: &str) -> Option<usize> {
    let h = list(track, "heights");
    if h.is_empty() {
        return None;
    }
    if end == "BEGIN" {
        return if st(&h[0]) <= JOINT_TOL { Some(0) } else { None };
    }
    if track_length(track) - st(&h[h.len() - 1]) <= JOINT_TOL {
        Some(h.len() - 1)
    } else {
        None
    }
}

/// Which end a height point sits at (`endOfIndex`).
pub fn end_of_index(track: &Value, index: usize) -> Option<&'static str> {
    if index == 0 && end_index(track, "BEGIN") == Some(0) {
        return Some("BEGIN");
    }
    if index > 0 && Some(index) == end_index(track, "END") {
        return Some("END");
    }
    None
}

/// The height points joined to one at a track end (`jointGroup`): (trackId, index).
pub fn joint_group(tracks: &[Value], switches: &[Value], track_id: &Value, index: usize) -> Vec<(Value, usize)> {
    let key = |t: &Value, i: usize| format!("{}|{i}", rail_across_js::js_string(Some(t)));
    let mut group: Vec<(String, (Value, usize))> = Vec::new();
    let Some(first) = tracks.iter().find(|t| strict_eq(t.get("id"), Some(track_id))) else {
        return vec![];
    };
    if list(first, "heights").is_empty() {
        return vec![];
    }
    group.push((key(track_id, index), (track_id.clone(), index)));
    let Some(start_end) = end_of_index(first, index) else {
        return group.into_iter().map(|(_, v)| v).collect();
    };
    let mut queue: VecDeque<(&Value, String)> = VecDeque::from([(first, start_end.to_string())]);
    let mut seen: HashSet<String> = HashSet::from([format!("{}|{start_end}", rail_across_js::js_string(first.get("id")))]);
    while let Some((track, end)) = queue.pop_front() {
        for (n, endpoint) in adjacent_tracks(tracks, switches, track, &end) {
            let mark = format!("{}|{endpoint}", rail_across_js::js_string(n.get("id")));
            if !seen.insert(mark) {
                continue;
            }
            let Some(i) = end_index(n, &endpoint) else { continue };
            let id = n.get("id").cloned().unwrap_or(Value::Null);
            let k = key(&id, i);
            if !group.iter().any(|(g, _)| *g == k) {
                group.push((k, (id, i)));
            }
            queue.push_back((n, endpoint));
        }
    }
    group.into_iter().map(|(_, v)| v).collect()
}

fn patched_point(q: &Value, entry: &Value) -> Value {
    let mut out = obj(q);
    if let Some(v) = entry.get("z") {
        out.insert("z".into(), v.clone());
    }
    for k in ["rv", "la", "reason", "locked"] {
        match entry.get(k) {
            None => {}
            Some(Value::Null) => {
                out.shift_remove(k);
            }
            Some(v) => {
                out.insert(k.into(), v.clone());
            }
        }
    }
    Value::Object(out)
}

/// The heights of every track a joint edit reaches (`jointHeightUpdates`): trackId → heights.
pub fn joint_height_updates(tracks: &[Value], switches: &[Value], entries: &[Value]) -> Vec<(Value, Vec<Value>)> {
    let mut by_track: Vec<(Value, Vec<Value>)> = Vec::new();
    for entry in entries {
        let index = entry.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        for (track_id, i) in joint_group(tracks, switches, entry.get("trackId").unwrap_or(&Value::Null), index) {
            let current = by_track.iter().find(|(id, _)| strict_eq(Some(id), Some(&track_id))).map(|(_, h)| h.clone());
            let heights = current.or_else(|| {
                tracks
                    .iter()
                    .find(|t| strict_eq(t.get("id"), Some(&track_id)))
                    .map(|t| list(t, "heights").to_vec())
            });
            let Some(heights) = heights.filter(|h| h.get(i).is_some()) else { continue };
            let next: Vec<Value> = heights
                .iter()
                .enumerate()
                .map(|(k, q)| if k == i { patched_point(q, entry) } else { q.clone() })
                .collect();
            match by_track.iter_mut().find(|(id, _)| strict_eq(Some(id), Some(&track_id))) {
                Some(slot) => slot.1 = next,
                None => by_track.push((track_id, next)),
            }
        }
    }
    by_track
}

fn gradients(points: &[Value], i: usize) -> Option<(f64, f64)> {
    let (p, a, b) = (points.get(i)?, points.get(i.checked_sub(1)?)?, points.get(i + 1)?);
    let before = (z(p) - z(a)) / (st(p) - st(a));
    let after = (z(b) - z(p)) / (st(b) - st(p));
    if before.is_finite() && after.is_finite() {
        Some((before, after))
    } else {
        None
    }
}

/// The tangent length of the vertical curve at point `i` (`tangentLength`).
pub fn tangent_length(points: &[Value], i: usize) -> Option<f64> {
    let r = rv(points.get(i))?;
    let (before, after) = gradients(points, i)?;
    Some(r.abs() * (after - before).abs() / 2.0)
}

/// The vertical curve at point `i`, sampled (`verticalCurve`).
pub fn vertical_curve(points: &[Value], i: usize, steps: usize) -> Option<Value> {
    let t = tangent_length(points, i).filter(|t| *t != 0.0)?;
    let (before, after) = gradients(points, i)?;
    let p = &points[i];
    let delta = after - before;
    Some(Value::Array(
        (0..=steps)
            .map(|k| {
                let x = -t + 2.0 * t * k as f64 / steps as f64;
                json!({ "station": number(st(p) + x), "z": number(z(p) + before * x + (x + t).powf(2.0) * delta / (4.0 * t)) })
            })
            .collect(),
    ))
}

/// The height at a station with the vertical curves (`gradientAt`).
pub fn gradient_at(heights: &[Value], station: f64) -> Option<f64> {
    let zz = height_at(heights, station)?;
    for i in 1..heights.len().saturating_sub(1) {
        let Some(t) = tangent_length(heights, i).filter(|t| *t != 0.0) else {
            continue;
        };
        let p = &heights[i];
        let x = station - st(p);
        if x.abs() >= t {
            continue;
        }
        let (before, after) = gradients(heights, i)?;
        return Some(z(p) + before * x + (x + t).powf(2.0) * (after - before) / (4.0 * t));
    }
    Some(zz)
}

/// Every vertical curve, sampled (`verticalCurves`).
pub fn vertical_curves(points: &[Value], steps: usize) -> Value {
    Value::Array((0..points.len()).filter_map(|i| vertical_curve(points, i, steps)).collect())
}

/// Where two vertical curves overlap (`verticalCurveOverlaps`).
pub fn vertical_curve_overlaps(points: &[Value]) -> Value {
    let spans: Vec<(usize, f64, f64)> = (0..points.len())
        .filter_map(|i| {
            tangent_length(points, i)
                .filter(|t| *t != 0.0)
                .map(|t| (i, st(&points[i]) - t, st(&points[i]) + t))
        })
        .collect();
    let mut out = Vec::new();
    for j in 0..spans.len() {
        for k in j + 1..spans.len() {
            let from = spans[j].1.max(spans[k].1);
            let to = spans[j].2.min(spans[k].2);
            if to - from <= 1e-6 {
                continue;
            }
            let mut zs = Vec::new();
            for i in [spans[j].0, spans[k].0] {
                let (before, after) = gradients(points, i).unwrap_or((f64::NAN, f64::NAN));
                let p = &points[i];
                let t = tangent_length(points, i).unwrap_or(f64::NAN);
                for n in 0..9 {
                    let x = from + (to - from) * f64::from(n) / 8.0 - st(p);
                    zs.push(z(p) + before * x + (x + t).powf(2.0) * (after - before) / (4.0 * t));
                }
            }
            let min = zs.iter().copied().fold(f64::INFINITY, f64::min);
            let max = zs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            out.push(json!({ "a": spans[j].0, "b": spans[k].0, "from": number(from), "to": number(to), "zMin": number(min), "zMax": number(max) }));
        }
    }
    Value::Array(out)
}

/// The heights of a track by id, after `jointHeightUpdates`, as a JSON object.
pub fn updates_json(updates: Vec<(Value, Vec<Value>)>) -> Value {
    let mut m: HashMap<String, Value> = HashMap::new();
    let mut order = Vec::new();
    for (id, h) in updates {
        let k = rail_across_js::js_string(Some(&id));
        order.push(k.clone());
        m.insert(k, Value::Array(h));
    }
    let mut o = rail_across_js::Object::new();
    for k in order {
        if let Some(v) = m.remove(&k) {
            o.insert(k, v);
        }
    }
    Value::Object(o)
}
