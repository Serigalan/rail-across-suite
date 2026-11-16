// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The bands of curvature, cant and speed along a track (`alignmentBands.js`),
//! and the buffer stops and boundaries as the map draws them
//! (`bufferStopGeometry.js`).

use rail_across_geom::kind::is_transition;
use std::collections::HashMap;

use rail_across_geom::cant::{cant_ends, transition_cant_ends, with_cant_edit};
use rail_across_geom::coords::plane_to_wgs84;
use rail_across_js::{get, js_string, number, strict_eq};
use rail_across_rules::bundled::worst_severity;
use rail_across_rules::cant::{cant_limit, cant_sign, clamp_cant};
use regex::Regex;
use serde_json::{Value, json};

use crate::end_marks::{BOUNDARY, BUFFER_STOP, buffer_stop_stations};
use crate::heights::track_length;
use crate::platforms::{offset_edge_coords, point_at_station};
use crate::util::{epsg, list, num, obj, text};

const BLOSS_STEPS: usize = 16;

/// Each element with the stations it spans (`elementSpans`): (index, from, to).
pub fn element_spans(els: &[Value]) -> Vec<(usize, f64, f64)> {
    let mut s = 0.0;
    els.iter()
        .enumerate()
        .map(|(i, el)| {
            let from = s;
            s += num(el, "length").unwrap_or(0.0);
            (i, from, s)
        })
        .collect()
}

fn k(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 && !r.is_nan() => 1000.0 / r,
        _ => 0.0,
    }
}

fn pt(s: f64, v: Option<f64>) -> Value {
    json!({ "s": number(s), "v": v.map(number) })
}

/// The curvature band [1/km] (`curvatureBand`).
pub fn curvature_band(els: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for (i, from, to) in element_spans(els) {
        let el = &els[i];
        if is_transition(el) {
            let (k1, k2) = (k(num(el, "r1")), k(num(el, "r2")));
            if text(el, "transitionType") == Some("bloss") {
                for n in 0..=BLOSS_STEPS {
                    let t = n as f64 / BLOSS_STEPS as f64;
                    out.push(pt(from + t * (to - from), Some(k1 + (k2 - k1) * (3.0 * t * t - 2.0 * t * t * t))));
                }
            } else {
                out.push(pt(from, Some(k1)));
                out.push(pt(to, Some(k2)));
            }
        } else {
            let kk = k(num(el, "radius"));
            out.push(pt(from, Some(kk)));
            out.push(pt(to, Some(kk)));
        }
    }
    out
}

/// The cant band [mm] (`cantBand`).
pub fn cant_band(els: &[Value]) -> Vec<Value> {
    element_spans(els)
        .into_iter()
        .flat_map(|(i, from, to)| {
            let (s, e) = cant_ends(els, i);
            [pt(from, Some(s)), pt(to, Some(e))]
        })
        .collect()
}

/// The speed band [km/h] (`speedBand`).
pub fn speed_band(els: &[Value]) -> Vec<Value> {
    element_spans(els)
        .into_iter()
        .flat_map(|(i, from, to)| {
            let v = num(&els[i], "speed").filter(|v| *v > 0.0);
            [pt(from, v), pt(to, v)]
        })
        .collect()
}

fn bs(p: &Value) -> f64 {
    p["s"].as_f64().unwrap_or(f64::NAN)
}
fn bv(p: &Value) -> Option<f64> {
    p["v"].as_f64()
}

/// A band's value at `s` (`bandValueAt`).
pub fn band_value_at(band: &[Value], s: f64) -> Option<f64> {
    if band.is_empty() || s < bs(&band[0]) || s > bs(&band[band.len() - 1]) {
        return None;
    }
    for i in (1..band.len()).rev() {
        let (a, b) = (&band[i - 1], &band[i]);
        if s < bs(a) {
            continue;
        }
        let (av, bvv) = (bv(a)?, bv(b)?);
        if bs(b) - bs(a) <= 0.0 {
            return Some(bvv);
        }
        return Some(av + (bvv - av) * (s - bs(a)) / (bs(b) - bs(a)));
    }
    bv(&band[0])
}

/// The runs of one value along a band (`bandRuns`).
pub fn band_runs(band: &[Value], eps: f64) -> Vec<Value> {
    let mut runs: Vec<(f64, f64, f64)> = Vec::new();
    for i in 1..band.len() {
        let (a, b) = (&band[i - 1], &band[i]);
        let (Some(av), Some(bvv)) = (bv(a), bv(b)) else { continue };
        if bs(b) - bs(a) <= 0.0 || (av - bvv).abs() > eps {
            continue;
        }
        match runs.last_mut() {
            Some(last) if (last.1 - bs(a)).abs() < 1e-6 && (last.2 - av).abs() <= eps => last.1 = bs(b),
            _ => runs.push((bs(a), bs(b), av)),
        }
    }
    runs.into_iter()
        .map(|(f, t, v)| json!({ "from": number(f), "to": number(t), "v": number(v) }))
        .collect()
}

/// The least and greatest value of a band (`bandRange`).
pub fn band_range(band: &[Value]) -> Option<(f64, f64)> {
    let vs: Vec<f64> = band.iter().filter_map(bv).collect();
    if vs.is_empty() {
        return None;
    }
    Some((
        vs.iter().copied().fold(f64::INFINITY, f64::min),
        vs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ))
}

fn bands_of_rule(id: &str) -> &'static [&'static str] {
    let rules: [(&str, &[&str]); 7] = [
        (r"^LP\.ALL\.", &["speed"]),
        (r"^LP\.KB\.0[26]$", &["cant", "speed"]),
        (r"^LP\.KB\.", &["cant"]),
        (r"^LP\.UB\.0[1278]$", &["cant"]),
        (r"^LP\.UB\.", &["curvature"]),
        (r"^LP\.EL\.", &["curvature"]),
        (r"^LP\.KS\.", &["curvature"]),
    ];
    rules
        .iter()
        .find(|(re, _)| Regex::new(re).expect("pattern").is_match(id))
        .map(|(_, b)| *b)
        .unwrap_or(&[])
}

/// The findings of a check by band (`bandFindings`).
pub fn band_findings(check: &Value) -> Value {
    let bands = ["curvature", "cant", "speed"];
    let mut out: HashMap<&str, (Vec<Value>, Vec<Value>)> = bands.iter().map(|b| (*b, (Vec::new(), Vec::new()))).collect();
    let mut add = |joint: bool, index: &Value, results: &[Value]| {
        for band in bands {
            let own: Vec<Value> = results
                .iter()
                .filter(|r| r["severity"].as_str().is_some_and(|s| !s.is_empty() && s != "ok") && bands_of_rule(r["id"].as_str().unwrap_or("")).contains(&band))
                .cloned()
                .collect();
            if !own.is_empty() {
                let sev = worst_severity(own.iter().map(|r| r["severity"].as_str())).map(str::to_string);
                let entry = json!({ "index": index, "results": own, "severity": sev });
                let slot = out.get_mut(band).expect("band");
                if joint { slot.1.push(entry) } else { slot.0.push(entry) }
            }
        }
    };
    let ramps = list(check, "ramps");
    for e in list(check, "elements") {
        let mut results = list(e, "results").to_vec();
        if let Some(r) = ramps.iter().find(|r| strict_eq(r.get("index"), e.get("index"))) {
            results.extend(list(r, "results").iter().cloned());
        }
        add(false, &e["index"], &results);
    }
    for b in list(check, "boundaries") {
        add(true, &b["index"], list(b, "results"));
    }
    let mut o = rail_across_js::Object::new();
    for band in bands {
        let (spans, joints) = out.remove(band).expect("band");
        o.insert(band.into(), json!({ "spans": spans, "joints": joints }));
    }
    Value::Object(o)
}

fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// The elements with one band's value typed for some of them (`withBandValue`).
pub fn with_band_value(els: &[Value], indices: &[usize], band: &str, value: Option<f64>) -> Vec<Value> {
    if band == "cant" {
        let mut out = els.to_vec();
        for &i in indices {
            let Some(el) = out.get(i).cloned() else { continue };
            if is_transition(&el) {
                continue;
            }
            let flat = Value::Object(crate::util::without(&el, &["cantStart", "cantEnd"]));
            let side = if num(&el, "radius").is_some_and(|r| r != 0.0) {
                1.0
            } else {
                let s = js_sign(num(&el, "cant").unwrap_or(0.0));
                if s == 0.0 || s.is_nan() { 1.0 } else { s }
            };
            let typed = side * value.unwrap_or(0.0).abs();
            let c = clamp_cant(typed, &el);
            let c = if c == 0.0 || c.is_nan() { 0.0 } else { c };
            let mut next = out.clone();
            next[i] = flat;
            out = with_cant_edit(&next, i, "cant", c);
        }
        let same = |a: &Value, b: &Value| ["cant", "cantStart", "cantEnd"].iter().all(|k| strict_eq(a.get(*k), b.get(*k)));
        return out
            .into_iter()
            .enumerate()
            .map(|(i, el)| if el == els[i] || !same(&el, &els[i]) { el } else { els[i].clone() })
            .collect();
    }
    els.iter()
        .enumerate()
        .map(|(i, el)| {
            if !indices.contains(&i) {
                return el.clone();
            }
            match value {
                None => Value::Object(crate::util::without(el, &[band])),
                Some(v) => {
                    let mut o = obj(el);
                    o.insert("speed".into(), number(v.max(0.0)));
                    Value::Object(o)
                }
            }
        })
        .collect()
}

/// The elements with a transition's cant ends typed (`withCantEnds`).
pub fn with_cant_ends(els: &[Value], indices: &[usize], ends: &Value) -> Vec<Value> {
    let mut out = els.to_vec();
    for &i in indices {
        let Some(el) = out.get(i).cloned() else { continue };
        if !is_transition(&el) {
            continue;
        }
        for (key, field) in [("start", "cantStart"), ("end", "cantEnd")] {
            let Some(value) = ends.get(key) else { continue };
            if value.is_null() {
                out[i] = Value::Object(crate::util::without(&out[i], &[field]));
                continue;
            }
            let r = if key == "start" {
                num(&el, "r1").or(num(&el, "r2"))
            } else {
                num(&el, "r2").or(num(&el, "r1"))
            };
            let current = {
                let (s, e) = transition_cant_ends(&out, i);
                if key == "start" { s } else { e }
            };
            let mut side = js_sign(current);
            if side == 0.0 || side.is_nan() {
                side = match r.filter(|r| *r != 0.0) {
                    Some(r) => cant_sign(r),
                    None => 1.0,
                };
            }
            let v = value.as_f64().unwrap_or(f64::NAN);
            let mut c = side * cant_limit(&el).min(rail_across_js::js_round(v.abs() * 1000.0) / 1000.0);
            if c == 0.0 || c.is_nan() {
                c = 0.0;
            }
            out = with_cant_edit(&out, i, field, c);
        }
    }
    out
}

// ── Buffer stops and boundaries on the map ──────────────────────────────────

pub const FACE_HALF_WIDTH: f64 = 1.75;
pub const BOUNDARY_HALF_WIDTH: f64 = 3.0;

fn bar_at(track: &Value, station: f64, half: f64) -> Option<Vec<Value>> {
    let (utm, bearing) = point_at_station(track, station)?;
    let rad = bearing * std::f64::consts::PI / 180.0;
    let (dx, dy) = (half * rad.cos(), -half * rad.sin());
    let e = epsg(track);
    let p = |x: f64, y: f64| plane_to_wgs84(x, y, e).map(|(a, b)| json!([number(a), number(b)])).unwrap_or(Value::Null);
    Some(vec![p(utm.e - dx, utm.n - dy), p(utm.e + dx, utm.n + dy)])
}

/// The features a buffer stop is drawn with (`bufferStopFeatures`).
pub fn buffer_stop_features(tracks: &[Value], marks: &[Value]) -> Vec<Value> {
    let mut features = Vec::new();
    for mark in marks {
        if text(mark, "kind") != Some(BUFFER_STOP) {
            continue;
        }
        let Some(track) = tracks.iter().find(|t| strict_eq(t.get("id"), mark.get("trackId"))) else {
            continue;
        };
        let total = track_length(track);
        if !(total > 0.0) || !rail_across_js::truthy(track.get("epsg")) {
            continue;
        }
        let (face, body, brake) = buffer_stop_stations(total, mark);
        let parts = [
            ("face", bar_at(track, face, FACE_HALF_WIDTH)),
            ("body", offset_edge_coords(track, body[0], body[1], 0.0)),
            ("brake", offset_edge_coords(track, brake[0], brake[1], 0.0)),
        ];
        for (part, coords) in parts {
            if let Some(c) = coords.filter(|c| c.len() >= 2) {
                features.push(json!({ "type": "Feature", "properties": { "markId": mark.get("id"), "trackId": mark.get("trackId"), "part": part }, "geometry": { "type": "LineString", "coordinates": c } }));
            }
        }
    }
    features
}

/// The features a boundary is drawn with (`boundaryFeatures`).
pub fn boundary_features(tracks: &[Value], marks: &[Value]) -> Vec<Value> {
    let mut features = Vec::new();
    for mark in marks {
        if text(mark, "kind") != Some(BOUNDARY) {
            continue;
        }
        let Some(track) = tracks.iter().find(|t| strict_eq(t.get("id"), mark.get("trackId"))) else {
            continue;
        };
        if !rail_across_js::truthy(track.get("epsg")) {
            continue;
        }
        let total = track_length(track);
        if !(total > 0.0) {
            continue;
        }
        if let Some(bar) = bar_at(track, if text(mark, "endpoint") == Some("BEGIN") { 0.0 } else { total }, BOUNDARY_HALF_WIDTH) {
            features.push(json!({ "type": "Feature", "properties": { "markId": mark.get("id"), "trackId": mark.get("trackId"), "part": "boundary" }, "geometry": { "type": "LineString", "coordinates": bar } }));
        }
    }
    let _ = (get, js_string);
    features
}
