// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The invariants an element chain has to hold (`chainChecks.js`), and the
//! closing of what misses by less than a tolerance (`chainSnap.js`).

use rail_across_js::{get, number, strict_eq};
use serde_json::{Value, json};

use crate::element::{Utm, curved_values, resolve_end_bearing, straight_values};
use crate::reconstruct::reconstruct_elements;
use crate::tolerances::NODE_TOL;

/// Two tangents are the same tangent below this [°].
pub const BEARING_TOL: f64 = 1e-6;

fn node(el: &Value, key: &str) -> Option<[f64; 2]> {
    let a = el.get(key)?.as_array()?;
    Some([
        a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ])
}

fn bearing_delta(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs()
}

/// Where an element does not end where the next begins: (index, gap) (`nodeGaps`).
pub fn node_gaps(elements: &[Value], tol: f64) -> Vec<(usize, f64)> {
    let mut out = Vec::new();
    for i in 0..elements.len().saturating_sub(1) {
        let (Some(e), Some(s)) = (node(&elements[i], "endNode"), node(&elements[i + 1], "startNode")) else {
            continue;
        };
        let gap = (e[0] - s[0]).hypot(e[1] - s[1]);
        if !(gap < tol) {
            out.push((i, gap));
        }
    }
    out
}

/// Where an element leaves on another tangent than the one before arrived on (`tangentBreaks`).
pub fn tangent_breaks(elements: &[Value], epsg: i64, tol: f64) -> Vec<(usize, f64)> {
    let mut out = Vec::new();
    for i in 0..elements.len().saturating_sub(1) {
        let end = resolve_end_bearing(&elements[i], epsg);
        let start = get(&elements[i + 1], "bearing").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let delta = bearing_delta(end, start);
        if !(delta < tol) {
            out.push((i, delta));
        }
    }
    out
}

/// Where the stated length is not the element's own geometry's (`untrueLengths`): (index, length, expected).
pub fn untrue_lengths(elements: &[Value], tol: f64) -> Vec<(usize, f64, f64)> {
    let mut out = Vec::new();
    for (i, el) in elements.iter().enumerate() {
        let (Some(s), Some(e)) = (node(el, "startNode"), node(el, "endNode")) else {
            continue;
        };
        let chord = (e[0] - s[0]).hypot(e[1] - s[1]);
        let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
        if let Some(r) = get(el, "radius").and_then(Value::as_f64) {
            let abs_r = r.abs();
            let expected = 2.0 * abs_r * (chord / (2.0 * abs_r)).min(1.0).asin();
            if !((length - expected).abs() < 0.5e-3) {
                out.push((i, length, expected));
            }
        } else if el.get("elementType").and_then(Value::as_f64) == Some(2.0) {
            if !(length >= chord - tol) {
                out.push((i, length, chord));
            }
        } else if !((length - chord).abs() < 0.5e-6) {
            out.push((i, length, chord));
        }
    }
    out
}

/// Elements stating another CRS than their track's (`epsgMismatches`).
pub fn epsg_mismatches(track: &Value) -> Vec<(usize, Value)> {
    let epsg = track.get("epsg");
    track
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, el)| match el.get("epsg") {
            Some(v) if !strict_eq(Some(v), epsg) => Some((i, v.clone())),
            _ => None,
        })
        .collect()
}

/// The default tolerance of `nodeGaps` and `untrueLengths`.
pub const JOINT_TOL: f64 = NODE_TOL;

// ── Closing ──────────────────────────────────────────────────────────────────

/// An element whose node was moved, its values taken from its nodes again (`refitToNodes`).
pub fn refit_to_nodes(el: &Value, epsg: i64, length_change: f64) -> Value {
    let (Some(s), Some(e)) = (node(el, "startNode"), node(el, "endNode")) else {
        return el.clone();
    };
    let su = Utm { e: s[0], n: s[1], zone: epsg };
    let eu = Utm { e: e[0], n: e[1], zone: epsg };
    let mut out = el.as_object().cloned().unwrap_or_default();
    if let Some(r) = get(el, "radius").and_then(Value::as_f64) {
        let (length, bearing, end_bearing) = curved_values(su, eu, r);
        if length > 0.0 {
            out.insert("length".into(), number(length));
            out.insert("bearing".into(), number(bearing));
            if get(el, "endBearing").is_some() {
                out.insert("endBearing".into(), number(end_bearing));
            }
        }
    } else if el.get("elementType").and_then(Value::as_f64) != Some(2.0) {
        let (length, bearing) = straight_values(su, eu);
        out.insert("length".into(), number(length));
        out.insert("bearing".into(), number(bearing));
    } else if length_change != 0.0 && !length_change.is_nan() {
        let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
        out.insert("length".into(), number(length + length_change));
    }
    let out = Value::Object(out);
    if epsg != 0 { reconstruct_elements(&[out], epsg).remove(0) } else { out }
}

/// A chain closed where its nodes miss by no more than `tol` (`snapNodeGaps`).
pub fn snap_node_gaps(elements: &[Value], epsg: i64, tol: f64) -> (Vec<Value>, Vec<(usize, f64)>) {
    let mut out = elements.to_vec();
    let mut snapped = Vec::new();
    for i in 0..out.len().saturating_sub(1) {
        let (Some(end), Some(start)) = (node(&out[i], "endNode"), node(&out[i + 1], "startNode")) else {
            continue;
        };
        if node(&out[i], "startNode").is_none() {
            continue;
        }
        let gap = (end[0] - start[0]).hypot(end[1] - start[1]);
        if !(gap > 0.0 && gap <= tol) {
            continue;
        }
        let mut moved = out[i].as_object().cloned().unwrap_or_default();
        moved.insert("endNode".into(), json!([number(start[0]), number(start[1])]));
        out[i] = refit_to_nodes(&Value::Object(moved), epsg, 0.0);
        snapped.push((i, gap));
    }
    (out, snapped)
}

/// `track` with the node at `endpoint` moved to `node` (`moveTrackEnd`).
pub fn move_track_end(track: &Value, endpoint: &str, to: [f64; 2]) -> Value {
    let mut elements = track.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    if elements.is_empty() {
        return track.clone();
    }
    let k = if endpoint == "BEGIN" { 0 } else { elements.len() - 1 };
    let key = if endpoint == "BEGIN" { "startNode" } else { "endNode" };
    let mut el = elements[k].as_object().cloned().unwrap_or_default();
    el.insert(key.into(), json!([number(to[0]), number(to[1])]));
    let epsg = track.get("epsg").and_then(Value::as_f64).unwrap_or(0.0) as i64;
    elements[k] = refit_to_nodes(&Value::Object(el), epsg, 0.0);
    let mut out = track.as_object().cloned().unwrap_or_default();
    out.insert("elements".into(), Value::Array(elements));
    Value::Object(out)
}

/// Tracks whose ends meet within `tol` meeting at one node (`snapTrackJoints`).
pub fn snap_track_joints(tracks: &[Value], tol: f64) -> (Vec<Value>, Vec<(Value, String, f64)>) {
    let mut out = tracks.to_vec();
    let mut snapped = Vec::new();
    let node_of = |t: &Value, endpoint: &str| -> Option<[f64; 2]> {
        let els = t.get("elements").and_then(Value::as_array)?;
        if endpoint == "BEGIN" {
            node(els.first()?, "startNode")
        } else {
            node(els.last()?, "endNode")
        }
    };
    let mut anchors: Vec<(usize, f64, [f64; 2])> = Vec::new();
    for endpoint in ["BEGIN", "END"] {
        #[allow(clippy::needless_range_loop)]
        for i in 0..out.len() {
            let Some(p) = node_of(&out[i], endpoint) else { continue };
            let epsg = out[i].get("epsg").and_then(Value::as_f64).unwrap_or(f64::NAN);
            let anchor = anchors
                .iter()
                .find(|(j, e, a)| *j != i && *e == epsg && (a[0] - p[0]).hypot(a[1] - p[1]) <= tol)
                .copied();
            let Some((_, _, a)) = anchor else {
                anchors.push((i, epsg, p));
                continue;
            };
            let gap = (a[0] - p[0]).hypot(a[1] - p[1]);
            if !(gap > 0.0) {
                continue;
            }
            out[i] = move_track_end(&out[i], endpoint, a);
            snapped.push((out[i].get("id").cloned().unwrap_or(Value::Null), endpoint.to_string(), gap));
        }
    }
    (out, snapped)
}
