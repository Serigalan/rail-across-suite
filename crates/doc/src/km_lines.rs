// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Km lines as core 6.0 keeps them (decision 0006): in one plane, as runs from km jump to km jump, each with its
//! kilometrage at the start and the way it counts — the run itself its plan by column (`horizontalAlignment`, as a
//! track's without cant and speed), or the points a source gave where it gave no elements, held as a blob.
//!
//! The record keeps a km line as the document does (`rail_across_geom::km_line` reads it): in its plane, its runs
//! each with its `elements` by row or its `points` [easting, northing, km]. A record of before is read too: its
//! line in WGS 84 with the kilometrage per point (`crs`, `clip`, `runs` as lists of points), written in the UTM
//! plane of ETRS89 its first point lies in — a plane without a grid, so that every device writes the same numbers —,
//! or carrying the design elements a source gave beside its points (`design`), which are then the line.

use rail_across_geom::coords::{epsg_for_lng_lat, wgs84_to_plane};
use rail_across_geom::km_line::plane_of as held_in_plane;
use rail_across_js::{Object, js_string, without_null};
use serde_json::{Value, json};

use crate::error::{PayloadError, Result};

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

/// The plane a line of WGS 84 is written in: the UTM plane of its first point.
fn utm_of(line: &Value) -> Option<i64> {
    let runs = line.get("runs")?.as_array()?;
    let first = runs.iter().filter_map(Value::as_array).flatten().find_map(point)?;
    Some(epsg_for_lng_lat(first.0, first.1))
}

/// Runs in a plane as the document states them: each by column, or its points as a blob. Refused where a run's
/// elements do not hold together (`alignment::to_km_columns`).
fn plane_runs(runs: &[Value], blob: &mut impl FnMut(&Value) -> Value) -> std::result::Result<Vec<Value>, Vec<String>> {
    let mut out = Vec::new();
    let mut apart = Vec::new();
    for (k, run) in runs.iter().enumerate() {
        let mut o = Object::new();
        o.insert("kmStart".into(), run.get("kmStart").cloned().unwrap_or(json!(0)));
        o.insert("counting".into(), run.get("counting").cloned().unwrap_or(json!("up")));
        // A run the document already states — by column, or its points a blob — stays as it is.
        if let Some(columns) = run.get("horizontalAlignment") {
            o.insert("horizontalAlignment".into(), columns.clone());
        } else if let Some(hash) = run.get("points").filter(|p| p.is_string()) {
            o.insert("points".into(), hash.clone());
        } else if let Some(rows) = run.get("elements").and_then(Value::as_array) {
            match crate::alignment::to_km_columns(rows) {
                Ok(Some(columns)) => {
                    o.insert("horizontalAlignment".into(), columns);
                }
                Ok(None) => continue,
                Err(found) => {
                    apart.extend(found.into_iter().map(|f| format!("run {k}, {f}")));
                    continue;
                }
            }
        } else if let Some(points) = run.get("points").filter(|p| p.as_array().is_some_and(|a| a.len() >= 2)) {
            o.insert("points".into(), blob(points));
        } else {
            continue;
        }
        out.push(Value::Object(o));
    }
    if apart.is_empty() { Ok(out) } else { Err(apart) }
}

/// A km line of the record as core 6.0 states it: from its plane and runs, or from the design a record of before
/// carried beside its points, or from its points of WGS 84 in the plane of the first one; the points put into
/// `blob` (which gives back what names them). Refused where a run's elements do not hold together.
pub fn to_document(line: &Value, blob: &mut impl FnMut(&Value) -> Value) -> Result<Value> {
    let mut out = Object::new();
    for k in ["id", "lineNumber"] {
        if let Some(v) = line.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    let in_plane = held_in_plane(line).map(|epsg| (epsg, line)).or_else(|| {
        let design = line.get("design")?;
        held_in_plane(design).map(|epsg| (epsg, design))
    });
    if let Some((epsg, held)) = in_plane {
        let runs = held.get("runs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        let runs = plane_runs(runs, blob).map_err(|found| {
            let name = js_string(line.get("lineNumber").or(line.get("id")));
            PayloadError::with("invalid_payload", found.into_iter().map(|f| format!("/kmLines: {name}, {f}")).collect())
        })?;
        out.insert("epsg".into(), json!(epsg));
        out.insert("runs".into(), Value::Array(runs));
    } else if let Some(epsg) = utm_of(line) {
        out.insert("epsg".into(), json!(epsg));
        let mut runs = Vec::new();
        for run in line.get("runs").and_then(Value::as_array).into_iter().flatten() {
            let points: Vec<(f64, f64, f64)> = run.as_array().into_iter().flatten().filter_map(point).collect();
            let (Some(first), Some(last)) = (points.first(), points.last()) else {
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
    }
    if let Some(v) = line.get("source") {
        out.insert("source".into(), v.clone());
    }
    Ok(Value::Object(without_null(out)))
}

/// A km line of core 6.0 as the record keeps it: in its plane, each run with its elements by row or its points.
pub fn to_record(line: &Value, blobs: &Value) -> Result<Value> {
    let mut out = Object::new();
    for k in ["id", "lineNumber", "epsg"] {
        if let Some(v) = line.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    let mut runs = Vec::new();
    for run in line.get("runs").and_then(Value::as_array).into_iter().flatten() {
        let mut o = Object::new();
        o.insert("kmStart".into(), run.get("kmStart").cloned().unwrap_or(json!(0)));
        o.insert("counting".into(), run.get("counting").cloned().unwrap_or(json!("up")));
        if let Some(a) = run.get("horizontalAlignment") {
            o.insert("elements".into(), Value::Array(crate::alignment::to_rows(a)));
        } else if let Some(hash) = run.get("points").and_then(Value::as_str) {
            let held = blobs
                .get(hash)
                .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/blobs: lacks {hash}")]))?;
            o.insert("points".into(), held.clone());
        } else {
            continue;
        }
        runs.push(Value::Object(o));
    }
    out.insert("runs".into(), Value::Array(runs));
    if let Some(v) = line.get("source") {
        out.insert("source".into(), v.clone());
    }
    Ok(Value::Object(out))
}
