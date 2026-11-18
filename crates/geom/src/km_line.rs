// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Km lines read from either form a project holds them in (decision 0006): in a plane (`epsg`), its runs each
//! with its kilometrage at the start (`kmStart`), the way it counts (`counting`) and its design elements by row
//! (`elements`) or its points `[easting, northing, km]` (`points`) — as the document has a km line; or, as a
//! record of before and the line files shipped with the app have it, in WGS 84, its runs lists of
//! `[lon, lat, km]`. A run of elements gives its kilometre exactly: the start value plus or minus the station
//! along them; a run of points between its points.

use rail_across_js::{num, str_of};
use serde_json::Value;

use crate::coords::{plane_to_wgs84, transform_plane_point, wgs84_to_plane};
use crate::element::{Utm, element_points, end_point_curved, end_point_straight, project_on_arc};
use crate::transition::{is_bloss, project_on_transition};

/// How finely a run of elements is drawn [m].
pub const DRAW_STEP: f64 = 5.0;

/// The plane a line is held in, where it is held in one: its runs objects, each with its elements or points.
pub fn plane_of(line: &Value) -> Option<i64> {
    let epsg = line.get("epsg").and_then(Value::as_i64)?;
    let runs = line.get("runs").and_then(Value::as_array)?;
    runs.iter().all(Value::is_object).then_some(epsg)
}

fn sign(run: &Value) -> f64 {
    if str_of(run, "counting") == Some("down") { -1.0 } else { 1.0 }
}

fn triple(p: &Value) -> Option<[f64; 3]> {
    let a = p.as_array()?;
    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?])
}

fn rows(run: &Value) -> &[Value] {
    run.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// A run's points in its line's plane `epsg`, [easting, northing, km] each: those it holds, or its elements
/// sampled every `step` metres, the kilometrage counted from its start.
pub fn run_points(run: &Value, epsg: i64, step: f64) -> Vec<[f64; 3]> {
    if let Some(points) = run.get("points").and_then(Value::as_array) {
        return points.iter().filter_map(triple).collect();
    }
    let (start, sign) = (num(run, "kmStart").unwrap_or(0.0), sign(run));
    let mut out: Vec<[f64; 3]> = Vec::new();
    let mut along = 0.0;
    for el in rows(run) {
        let length = num(el, "length").unwrap_or(0.0);
        let pts = element_points(el, epsg, step).unwrap_or_default();
        let n = pts.len().saturating_sub(1).max(1) as f64;
        for (i, [e, north]) in pts.iter().enumerate() {
            if i == 0 && !out.is_empty() {
                continue;
            }
            out.push([*e, *north, start + sign * (along + length * i as f64 / n)]);
        }
        along += length;
    }
    out
}

/// A line's runs in plane `epsg` as points [easting, northing, km], from either form; a run of elements sampled
/// every `step` metres.
pub fn plane_runs(line: &Value, epsg: i64, step: f64) -> Vec<Vec<[f64; 3]>> {
    let runs = line.get("runs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    match plane_of(line) {
        Some(own) => runs
            .iter()
            .map(|run| {
                run_points(run, own, step)
                    .into_iter()
                    .map(|[e, n, km]| {
                        let (e, n) = transform_plane_point(e, n, own, epsg).unwrap_or((f64::NAN, f64::NAN));
                        [e, n, km]
                    })
                    .collect()
            })
            .collect(),
        None => runs
            .iter()
            .map(|run| {
                run.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(triple)
                    .map(|[lon, lat, km]| {
                        let (e, n) = wgs84_to_plane(lon, lat, epsg).unwrap_or((f64::NAN, f64::NAN));
                        [e, n, km]
                    })
                    .collect()
            })
            .collect(),
    }
}

/// A line's runs in WGS 84 as [lon, lat, km], from either form — what a map draws; a run of elements sampled
/// every `step` metres.
pub fn wgs84_runs(line: &Value, step: f64) -> Vec<Vec<[f64; 3]>> {
    let runs = line.get("runs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    match plane_of(line) {
        Some(own) => runs
            .iter()
            .map(|run| {
                run_points(run, own, step)
                    .into_iter()
                    .filter_map(|[e, n, km]| plane_to_wgs84(e, n, own).ok().map(|(lon, lat)| [lon, lat, km]))
                    .collect()
            })
            .collect(),
        None => runs
            .iter()
            .map(|run| run.as_array().into_iter().flatten().filter_map(triple).collect())
            .collect(),
    }
}

/// The kilometre nearest a point of points `run`, and how far off it lies.
fn km_on_points(run: &[[f64; 3]], point: [f64; 2]) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for w in run.windows(2) {
        let ([ax, ay, akm], [bx, by, bkm]) = (w[0], w[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let sq = dx * dx + dy * dy;
        if sq == 0.0 {
            continue;
        }
        let t = (((point[0] - ax) * dx + (point[1] - ay) * dy) / sq).clamp(0.0, 1.0);
        let offset = (point[0] - (ax + t * dx)).hypot(point[1] - (ay + t * dy));
        if best.is_none_or(|b| offset < b.1) {
            best = Some((akm + t * (bkm - akm), offset));
        }
    }
    best
}

/// Where a point lies along an element of a run, in the line's plane: (along, offset), `along` within it.
fn along_element(el: &Value, p: Utm) -> Option<(f64, f64)> {
    let length = num(el, "length")?;
    let start = el.get("startNode")?.as_array()?;
    let s = Utm {
        e: start.first()?.as_f64()?,
        n: start.get(1)?.as_f64()?,
        zone: p.zone,
    };
    let bearing = num(el, "bearing")?;
    if str_of(el, "elementType") == Some("transition") {
        let (along, perp) = project_on_transition(
            s.e,
            s.n,
            p.e,
            p.n,
            bearing,
            length,
            num(el, "r1"),
            num(el, "r2"),
            is_bloss(str_of(el, "transitionType")),
        );
        return Some((along, perp.abs()));
    }
    let radius = num(el, "radius").filter(|r| *r != 0.0);
    let (along, _) = project_on_arc(s, p, bearing, radius);
    let along = along.clamp(0.0, length);
    let at = match radius {
        Some(r) => end_point_curved(s, bearing, along, r),
        None => end_point_straight(s, bearing, along),
    };
    Some((along, (p.e - at.e).hypot(p.n - at.n)))
}

/// The kilometre nearest a point (in plane `epsg`) on a line, and how far off it lies: on a run of elements the
/// start value plus or minus the station along them where the point falls, on a run of points between them.
pub fn km_at(line: &Value, point: [f64; 2], epsg: i64) -> Option<(f64, f64)> {
    let Some(own) = plane_of(line) else {
        return plane_runs(line, epsg, DRAW_STEP)
            .iter()
            .filter_map(|run| km_on_points(run, point))
            .min_by(|a, b| a.1.total_cmp(&b.1));
    };
    let (e, n) = transform_plane_point(point[0], point[1], epsg, own).ok()?;
    let p = Utm { e, n, zone: own };
    let mut best: Option<(f64, f64)> = None;
    for run in line.get("runs").and_then(Value::as_array).into_iter().flatten() {
        let found = if run.get("points").is_some() {
            km_on_points(&run_points(run, own, DRAW_STEP), [e, n])
        } else {
            let (start, sign) = (num(run, "kmStart").unwrap_or(0.0), sign(run));
            let mut station = 0.0;
            let mut nearest: Option<(f64, f64)> = None;
            for el in rows(run) {
                if let Some((along, offset)) = along_element(el, p)
                    && nearest.is_none_or(|b| offset < b.1)
                {
                    nearest = Some((start + sign * (station + along), offset));
                }
                station += num(el, "length").unwrap_or(0.0);
            }
            nearest
        };
        if let Some(f) = found
            && best.is_none_or(|b| f.1 < b.1)
        {
            best = Some(f);
        }
    }
    best
}

/// A line of WGS 84 in a plane (`plane_of`), as the document holds a source of points alone: in the UTM plane of
/// ETRS89 its first point lies in, its runs each with its kilometrage at the start, the way it counts and its
/// points [easting, northing, km] to the tenth of a millimetre. None for a line already in a plane or without a
/// point.
pub fn in_plane(line: &Value) -> Option<Value> {
    if plane_of(line).is_some() {
        return None;
    }
    let runs = line.get("runs")?.as_array()?;
    let first = runs.iter().filter_map(Value::as_array).flatten().find_map(triple)?;
    let epsg = crate::coords::epsg_for_lng_lat(first[0], first[1]);
    let round = |x: f64| (x * 1e4).round() / 1e4;
    let planar: Vec<Value> = runs
        .iter()
        .filter_map(|run| {
            let pts: Vec<[f64; 3]> = run.as_array()?.iter().filter_map(triple).collect();
            let (a, b) = (pts.first()?, pts.last()?);
            let points: Vec<Value> = pts
                .iter()
                .filter_map(|[lon, lat, km]| wgs84_to_plane(*lon, *lat, epsg).ok().map(|(e, n)| serde_json::json!([round(e), round(n), km])))
                .collect();
            (points.len() >= 2).then(|| serde_json::json!({ "kmStart": a[2], "counting": if b[2] < a[2] { "down" } else { "up" }, "points": points }))
        })
        .collect();
    let mut out = line.as_object()?.clone();
    for k in ["crs", "clip", "design"] {
        out.shift_remove(k);
    }
    out.insert("epsg".into(), serde_json::json!(epsg));
    out.insert("runs".into(), Value::Array(planar));
    Some(Value::Object(out))
}
