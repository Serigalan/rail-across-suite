// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Carrying an alignment from one projected plane into another
//! (`planeTransform.js`): the nodes are transformed, every other number is
//! refitted to them — scaled by the ratio of the chords, turned by the
//! difference of their directions.

use rail_across_js::{get, number};
use serde_json::{Value, json};

use crate::coords::transform_plane_point;
use crate::reconstruct::reconstruct_elements;

const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;
const MIN_CHORD: f64 = 0.1;
const PROBE: f64 = 10.0;

fn norm360(deg: f64) -> f64 {
    ((deg % 360.0) + 360.0) % 360.0
}
fn bearing_delta(a: f64, b: f64) -> f64 {
    ((a - b + 540.0) % 360.0) - 180.0
}
fn azimuth(de: f64, dn: f64) -> f64 {
    norm360(de.atan2(dn) * RAD2DEG)
}

fn tpp(e: f64, n: f64, from: i64, to: i64) -> (f64, f64) {
    transform_plane_point(e, n, from, to).unwrap_or((f64::NAN, f64::NAN))
}

fn local_fit(e: f64, n: f64, bearing: f64, from: i64, to: i64) -> (f64, f64) {
    let rad = bearing * std::f64::consts::PI / 180.0;
    let (e1, n1) = tpp(e, n, from, to);
    let (e2, n2) = tpp(e + PROBE * rad.sin(), n + PROBE * rad.cos(), from, to);
    let (de, dn) = (e2 - e1, n2 - n1);
    let len = de.hypot(dn);
    if !(len > 0.0) {
        return (1.0, 0.0);
    }
    (len / PROBE, bearing_delta(azimuth(de, dn), bearing))
}

fn node(el: &Value, key: &str) -> Option<[f64; 2]> {
    let a = get(el, key)?.as_array()?;
    Some([
        a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ])
}

/// One element in the other plane, fitted to its own transformed nodes (`transformElement`).
pub fn transform_element(el: &Value, from: i64, to: i64) -> Option<Value> {
    let s = node(el, "startNode")?;
    let e = node(el, "endNode")?;
    let (se, sn) = tpp(s[0], s[1], from, to);
    let (ee, en) = tpp(e[0], e[1], from, to);
    if ![se, sn, ee, en].iter().all(|x| x.is_finite()) {
        return None;
    }
    let bearing0 = get(el, "bearing").and_then(Value::as_f64).unwrap_or(0.0);
    let d = (e[0] - s[0]).hypot(e[1] - s[1]);
    let d2 = (ee - se).hypot(en - sn);
    let (k, turn) = if d >= MIN_CHORD && d2 > 0.0 {
        (d2 / d, bearing_delta(azimuth(ee - se, en - sn), azimuth(e[0] - s[0], e[1] - s[1])))
    } else {
        local_fit(s[0], s[1], bearing0, from, to)
    };
    let mut out = el.as_object().cloned().unwrap_or_default();
    out.insert("startNode".into(), json!([number(se), number(sn)]));
    out.insert("endNode".into(), json!([number(ee), number(en)]));
    out.insert("bearing".into(), number(norm360(bearing0 + turn)));
    out.insert("length".into(), number(get(el, "length").and_then(Value::as_f64).unwrap_or(0.0) * k));
    if let Some(b) = get(el, "endBearing").and_then(Value::as_f64) {
        out.insert("endBearing".into(), number(norm360(b + turn)));
    }
    for key in ["radius", "r1", "r2"] {
        if let Some(r) = get(el, key).and_then(Value::as_f64) {
            out.insert(key.into(), number(r * k));
        }
    }
    Some(Value::Object(out))
}

fn joint_gap(els: &[Value], i: usize) -> f64 {
    let b = |el: &Value, k: &str| get(el, k).and_then(Value::as_f64);
    let next = b(&els[i + 1], "bearing").unwrap_or(f64::NAN);
    let end = b(&els[i], "endBearing").or_else(|| b(&els[i], "bearing")).unwrap_or(f64::NAN);
    bearing_delta(next, end).abs()
}

/// A whole track in the other plane (`transformTrackToPlane`): { track, scale, tangentGap, refused }.
pub fn transform_track_to_plane(track: &Value, to: f64) -> Value {
    let from = track
        .get("epsg")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(f64::NAN);
    if !from.is_finite() || !to.is_finite() {
        return Value::Null;
    }
    if from == to {
        return json!({ "track": track, "scale": { "min": 1, "max": 1 }, "tangentGap": 0, "refused": 0 });
    }
    let (from, to_i) = (from as i64, to as i64);
    let source = track.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut moved = Vec::new();
    let mut refused = 0;
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    for el in &source {
        let Some(next) = transform_element(el, from, to_i) else {
            refused += 1;
            continue;
        };
        let length = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        let k = if length > 0.0 {
            next["length"].as_f64().unwrap_or(f64::NAN) / length
        } else {
            1.0
        };
        min = min.min(k);
        max = max.max(k);
        moved.push(next);
    }
    let mut tangent_gap: f64 = 0.0;
    if moved.len() == source.len() {
        for i in 0..moved.len().saturating_sub(1) {
            tangent_gap = tangent_gap.max((joint_gap(&moved, i) - joint_gap(&source, i)).abs());
        }
    }
    let elements = reconstruct_elements(&moved, to_i);
    let scale = json!({ "min": number(if min.is_finite() { min } else { 1.0 }), "max": number(if max.is_finite() { max } else { 1.0 }) });
    let mut out = track.as_object().cloned().unwrap_or_default();
    out.insert("epsg".into(), number(to));
    out.insert("elements".into(), Value::Array(elements));
    if let Some(h) = scale_heights(track.get("heights"), &moved, &source) {
        out.insert("heights".into(), h);
    }
    json!({ "track": Value::Object(out), "scale": scale, "tangentGap": number(tangent_gap), "refused": refused })
}

fn scale_heights(heights: Option<&Value>, moved: &[Value], source: &[Value]) -> Option<Value> {
    let hs = heights?.as_array().filter(|h| h.len() >= 2)?;
    let sum = |els: &[Value]| els.iter().map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum::<f64>();
    let (before, after) = (sum(source), sum(moved));
    if !(before > 0.0) || !(after > 0.0) {
        return heights.cloned();
    }
    let k = after / before;
    Some(Value::Array(
        hs.iter()
            .map(|h| {
                let mut o = h.as_object().cloned().unwrap_or_default();
                let station = h.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN);
                o.insert("station".into(), number(station * k));
                if let Some(rv) = get(h, "rv").and_then(Value::as_f64) {
                    o.insert("rv".into(), number(rv * k));
                }
                Value::Object(o)
            })
            .collect(),
    ))
}
