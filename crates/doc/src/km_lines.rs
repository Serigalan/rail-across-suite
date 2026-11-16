// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Km lines as core 4.0 keeps them (decision 0029, phase 2): in one plane, as runs from km jump to km jump,
//! each with its kilometrage at the start and the way it counts — the run itself design elements, or the points
//! a source gave where it gave no elements, held as a blob. The record keeps a km line as before, in WGS 84 with
//! the kilometrage per point (`crs`, `clip`, `runs` as lists of points), until it follows the document
//! (decision 0029, phase 6); this converts both ways.
//!
//! A line of before is written in the UTM plane of ETRS89 its first point lies in: a plane without a grid, so
//! that every device writes the same numbers. Its points go into the plane to the tenth of a millimetre and come
//! back to the seventh decimal of a degree, the precision the record keeps them in.

use rail_across_geom::coords::{epsg_for_lng_lat, plane_to_wgs84, wgs84_to_plane};
use rail_across_geom::element::element_points;
use rail_across_js::{Object, without_null};
use serde_json::{Value, json};

use crate::error::{PayloadError, Result};

/// How finely the points of a run that has design elements are given back, in metres.
const STEP: f64 = 10.0;

fn round(x: f64, decimals: i32) -> f64 {
    let f = 10f64.powi(decimals);
    let r = (x * f).round() / f;
    if r == 0.0 { 0.0 } else { r }
}

/// A number as JSON states it: a whole one as an integer, as JavaScript would read it back.
fn num(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
        return json!(v as i64);
    }
    serde_json::Number::from_f64(v).map(Value::Number).unwrap_or(Value::Null)
}

fn point(p: &Value) -> Option<(f64, f64, f64)> {
    let a = p.as_array()?;
    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?))
}

/// The plane a line of the record is written in: the UTM plane of its first point.
fn plane_of(line: &Value) -> Option<i64> {
    let runs = line.get("runs")?.as_array()?;
    let first = runs.iter().filter_map(Value::as_array).flatten().find_map(point)?;
    Some(epsg_for_lng_lat(first.0, first.1))
}

/// A km line of the record as core 4.0 states it, its points put into `blob` (which gives back what names them).
pub fn to_document(line: &Value, blob: &mut impl FnMut(&Value) -> Value) -> Value {
    let mut out = Object::new();
    for k in ["id", "lineNumber"] {
        if let Some(v) = line.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    let epsg = plane_of(line);
    if let Some(e) = epsg {
        out.insert("epsg".into(), json!(e));
    }
    let mut runs = Vec::new();
    for run in line.get("runs").and_then(Value::as_array).into_iter().flatten() {
        let points: Vec<(f64, f64, f64)> = run.as_array().into_iter().flatten().filter_map(point).collect();
        let (Some(first), Some(last), Some(epsg)) = (points.first(), points.last(), epsg) else {
            continue;
        };
        let Ok(planar) = points
            .iter()
            .map(|(lng, lat, km)| wgs84_to_plane(*lng, *lat, epsg).map(|(e, n)| json!([num(round(e, 4)), num(round(n, 4)), num(*km)])))
            .collect::<std::result::Result<Vec<Value>, _>>()
        else {
            continue;
        };
        let counting = if last.2 < first.2 { "down" } else { "up" };
        runs.push(json!({ "kmStart": num(first.2), "counting": counting, "points": blob(&Value::Array(planar)) }));
    }
    out.insert("runs".into(), Value::Array(runs));
    if let Some(v) = line.get("source") {
        out.insert("source".into(), v.clone());
    }
    Value::Object(without_null(out))
}

/// The points of a run of core 4.0 in its plane, [easting, northing, km] each: those it holds, or its design
/// elements sampled, the kilometrage counted from its start.
fn run_points(run: &Value, epsg: i64, blobs: &Value) -> Result<Vec<(f64, f64, f64)>> {
    if let Some(hash) = run.get("points").and_then(Value::as_str) {
        let held = blobs
            .get(hash)
            .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/blobs: lacks {hash}")]))?;
        return Ok(held.as_array().into_iter().flatten().filter_map(point).collect());
    }
    let start = run.get("kmStart").and_then(Value::as_f64).unwrap_or(0.0);
    let sign = if run.get("counting").and_then(Value::as_str) == Some("down") {
        -1.0
    } else {
        1.0
    };
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    let mut along = 0.0;
    for el in run.get("elements").and_then(Value::as_array).into_iter().flatten() {
        let length = el.get("length").and_then(Value::as_f64).unwrap_or(0.0);
        let pts = element_points(el, epsg, STEP).unwrap_or_default();
        let n = pts.len().saturating_sub(1).max(1) as f64;
        for (i, [e, north]) in pts.iter().enumerate() {
            if i == 0 && !out.is_empty() {
                continue;
            }
            out.push((*e, *north, start + sign * (along + length * i as f64 / n)));
        }
        along += length;
    }
    Ok(out)
}

/// A km line of core 4.0 as the record keeps it: in WGS 84, the kilometrage per point, the box it covers.
pub fn to_record(line: &Value, blobs: &Value) -> Result<Value> {
    let mut out = Object::new();
    for k in ["id", "lineNumber"] {
        if let Some(v) = line.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    out.insert("crs".into(), json!("EPSG:4326"));
    let epsg = line.get("epsg").and_then(Value::as_i64);
    let mut runs = Vec::new();
    let mut bounds = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for run in line.get("runs").and_then(Value::as_array).into_iter().flatten() {
        let Some(epsg) = epsg else { continue };
        let mut points = Vec::new();
        for (e, n, km) in run_points(run, epsg, blobs)? {
            let Ok((lng, lat)) = plane_to_wgs84(e, n, epsg) else { continue };
            let (lng, lat) = (round(lng, 7), round(lat, 7));
            bounds = [bounds[0].min(lng), bounds[1].min(lat), bounds[2].max(lng), bounds[3].max(lat)];
            points.push(json!([num(lng), num(lat), num(km)]));
        }
        runs.push(Value::Array(points));
    }
    if bounds[0].is_finite() {
        out.insert("clip".into(), json!(bounds.iter().map(|b| num(*b)).collect::<Vec<_>>()));
    }
    out.insert("runs".into(), Value::Array(runs));
    if let Some(v) = line.get("source") {
        out.insert("source".into(), v.clone());
    }
    Ok(Value::Object(out))
}
