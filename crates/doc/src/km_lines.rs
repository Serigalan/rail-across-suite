// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Km lines as core 6.0 keeps them (decision 0006): in one plane, as runs from km jump to km jump,
//! each with its kilometrage at the start and the way it counts — the run itself its plan by column
//! (`horizontalAlignment`, as a track's without cant and speed), or the points a source gave where it gave no
//! elements, held as a blob. The record keeps a km line in WGS 84 with the kilometrage per point (`crs`, `clip`,
//! `runs` as lists of points), which every tool draws and reads the kilometrage from, and beside it, where a
//! source gave design elements, the line as the document has it (`design`: `epsg` and its runs, each with its
//! `elements` by row or its `points` [easting, northing, km] in the plane); this converts both ways, the points
//! of a run with elements sampled from them.
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

/// The runs of a record's `design` as the document states them: each by column, or its points as a blob.
/// Refused where a run's elements do not hold together (`alignment::to_km_columns`).
fn design_runs(design: &Value, blob: &mut impl FnMut(&Value) -> Value) -> std::result::Result<Vec<Value>, Vec<String>> {
    let mut runs = Vec::new();
    let mut apart = Vec::new();
    for (k, run) in design.get("runs").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let mut out = Object::new();
        out.insert("kmStart".into(), run.get("kmStart").cloned().unwrap_or(json!(0)));
        out.insert("counting".into(), run.get("counting").cloned().unwrap_or(json!("up")));
        if let Some(rows) = run.get("elements").and_then(Value::as_array) {
            match crate::alignment::to_km_columns(rows) {
                Ok(Some(columns)) => {
                    out.insert("horizontalAlignment".into(), columns);
                }
                Ok(None) => continue,
                Err(found) => {
                    apart.extend(found.into_iter().map(|f| format!("run {k}, {f}")));
                    continue;
                }
            }
        } else if let Some(points) = run.get("points").filter(|p| p.is_array()) {
            out.insert("points".into(), blob(points));
        } else {
            continue;
        }
        runs.push(Value::Object(out));
    }
    if apart.is_empty() { Ok(runs) } else { Err(apart) }
}

/// A km line of the record as core 6.0 states it: from its `design` where it has one, else from its points in
/// the plane of its first one, put into `blob` (which gives back what names them). Refused where the design's
/// elements do not hold together.
pub fn to_document(line: &Value, blob: &mut impl FnMut(&Value) -> Value) -> Result<Value> {
    let mut out = Object::new();
    for k in ["id", "lineNumber"] {
        if let Some(v) = line.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    if let Some(design) = line.get("design").filter(|d| d.get("epsg").is_some_and(Value::is_i64)) {
        let runs = design_runs(design, blob).map_err(|found| {
            let name = rail_across_js::js_string(line.get("lineNumber").or(line.get("id")));
            PayloadError::with("invalid_payload", found.into_iter().map(|f| format!("/kmLines: {name}, {f}")).collect())
        })?;
        out.insert("epsg".into(), design["epsg"].clone());
        out.insert("runs".into(), Value::Array(runs));
        if let Some(v) = line.get("source") {
            out.insert("source".into(), v.clone());
        }
        return Ok(Value::Object(without_null(out)));
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
    Ok(Value::Object(without_null(out)))
}

/// The points a run of core 6.0 holds, [easting, northing, km] each in its plane, or none for a run by column.
fn held_points(run: &Value, blobs: &Value) -> Result<Option<Vec<Value>>> {
    let Some(hash) = run.get("points").and_then(Value::as_str) else {
        return Ok(None);
    };
    let held = blobs
        .get(hash)
        .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/blobs: lacks {hash}")]))?;
    Ok(Some(held.as_array().cloned().unwrap_or_default()))
}

/// The points of a run of core 6.0 in its plane, [easting, northing, km] each: those it holds, or its design
/// elements sampled, the kilometrage counted from its start.
fn run_points(run: &Value, epsg: i64, blobs: &Value) -> Result<Vec<(f64, f64, f64)>> {
    if let Some(held) = held_points(run, blobs)? {
        return Ok(held.iter().filter_map(point).collect());
    }
    let start = run.get("kmStart").and_then(Value::as_f64).unwrap_or(0.0);
    let sign = if run.get("counting").and_then(Value::as_str) == Some("down") {
        -1.0
    } else {
        1.0
    };
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    let mut along = 0.0;
    let rows = run.get("horizontalAlignment").map(crate::alignment::to_rows).unwrap_or_default();
    for el in &rows {
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

/// A km line of core 6.0 as the record keeps it: in WGS 84, the kilometrage per point, the box it covers — and,
/// where a run states its plan by column, the line as the document has it (`design`).
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
    let given = line.get("runs").and_then(Value::as_array).cloned().unwrap_or_default();
    if let (Some(epsg), true) = (epsg, given.iter().any(|r| r.get("horizontalAlignment").is_some())) {
        let mut design_runs = Vec::new();
        for run in &given {
            let mut o = Object::new();
            o.insert("kmStart".into(), run.get("kmStart").cloned().unwrap_or(json!(0)));
            o.insert("counting".into(), run.get("counting").cloned().unwrap_or(json!("up")));
            match (run.get("horizontalAlignment"), held_points(run, blobs)?) {
                (Some(a), _) => o.insert("elements".into(), Value::Array(crate::alignment::to_rows(a))),
                (None, Some(points)) => o.insert("points".into(), Value::Array(points)),
                (None, None) => continue,
            };
            design_runs.push(Value::Object(o));
        }
        out.insert("design".into(), json!({ "epsg": epsg, "runs": design_runs }));
    }
    if let Some(v) = line.get("source") {
        out.insert("source".into(), v.clone());
    }
    Ok(Value::Object(out))
}
