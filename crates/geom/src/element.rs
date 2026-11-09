// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Element values (`elementUtils.js`): bearings an element's design values give.

use rail_across_js::{get, has, num, str_of};
use serde_json::Value;

use crate::transition::{is_bloss, js_mod360, transition_bearing_at};

/// The bearing [°] after `length` along an arc of signed radius `r` (none: a straight).
pub fn bearing_after(bearing: f64, length: f64, r: Option<f64>) -> f64 {
    let b = match r {
        Some(r) if r != 0.0 => bearing + (length / r) * 180.0 / std::f64::consts::PI,
        _ => bearing,
    };
    js_mod360(b)
}

/// The end bearing an element's own design values give: an arc's from its
/// bearing, length and radius, a transition's from its curvature profile —
/// none for a straight, which has none of its own (`derivedEndBearing`).
pub fn derived_end_bearing(el: &Value) -> Option<f64> {
    let bearing = num(el, "bearing").filter(|_| get(el, "bearing").is_some())?;
    let length = num(el, "length")?;
    if length.is_nan() || length < 0.0 {
        return None;
    }
    let radius = num(el, "radius").filter(|r| *r != 0.0 && !r.is_nan());
    if radius.is_some() {
        return Some(bearing_after(bearing, length, radius));
    }
    if num(el, "elementType") == Some(2.0) && has(el, "r1") {
        let r1 = num(el, "r1");
        let r2 = num(el, "r2");
        return Some(transition_bearing_at(bearing, length, r1, r2, is_bloss(str_of(el, "transitionType")), length));
    }
    None
}

use rail_across_js::{Object, js_round, number, truthy};
use serde_json::json;

use crate::coords::{Unsupported, plane_to_wgs84, wgs84_to_plane};
use crate::precision::{SAGITTA_DISPLAY, STRAIGHT_VERTEX_SPACING, SWITCH_ROUTE_STEP};
use crate::transition::transition_plane_points;

const TAU: f64 = 2.0 * std::f64::consts::PI;

/// Trims float noise (1e-9 m) so a derived length reads like the design value it is.
pub fn trim_noise(x: f64) -> f64 {
    js_round(x * 1e9) / 1e9
}

/// A plane point of the track's plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Utm {
    pub e: f64,
    pub n: f64,
    pub zone: i64,
}

impl Utm {
    pub fn to_json(self) -> Value {
        json!({ "easting": number(self.e), "northing": number(self.n), "zone": self.zone })
    }

    pub fn from_json(v: &Value) -> Utm {
        Utm {
            e: v.get("easting").and_then(Value::as_f64).unwrap_or(f64::NAN),
            n: v.get("northing").and_then(Value::as_f64).unwrap_or(f64::NAN),
            zone: v
                .get("zone")
                .and_then(|z| z.as_f64().or_else(|| z.as_str().and_then(|s| s.parse().ok())))
                .map(|z| z as i64)
                .unwrap_or(0),
        }
    }
}

fn point(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v?.as_array()?;
    let (e, n) = (a.first()?.as_f64()?, a.get(1)?.as_f64()?);
    if e.is_finite() && n.is_finite() { Some([e, n]) } else { None }
}

/// An element's stored plane node as a plane point, else its WGS 84 fallback projected (`nodeUtm`).
pub fn node_utm(node: Option<&Value>, wgs_fallback: Option<&Value>, epsg: i64) -> Result<Utm, Unsupported> {
    if let Some([e, n]) = point(node) {
        return Ok(Utm { e, n, zone: epsg });
    }
    let (lng, lat) = wgs_fallback
        .and_then(Value::as_array)
        .map(|a| {
            (
                a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
                a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
            )
        })
        .unwrap_or((f64::NAN, f64::NAN));
    let (e, n) = wgs84_to_plane(lng, lat, epsg)?;
    Ok(Utm { e, n, zone: epsg })
}

/// The centre of the arc of signed radius `signed_r` from (sE, sN) to (eE, eN) (`arcCenter`): (cx, cy, chord, absR).
pub fn arc_center(s_e: f64, s_n: f64, e_e: f64, e_n: f64, signed_r: f64) -> Option<(f64, f64, f64, f64)> {
    let dx = e_e - s_e;
    let dy = e_n - s_n;
    let chord = (dx * dx + dy * dy).sqrt();
    let abs_r = signed_r.abs();
    if chord < 1e-6 || abs_r < chord / 2.0 {
        return None;
    }
    let rpx = dy / chord;
    let rpy = -dx / chord;
    let h = (abs_r * abs_r - (chord / 2.0) * (chord / 2.0)).sqrt();
    let sgn = if signed_r >= 0.0 { -1.0 } else { 1.0 };
    Some(((s_e + e_e) / 2.0 - sgn * h * rpx, (s_n + e_n) / 2.0 - sgn * h * rpy, chord, abs_r))
}

fn tangent_bearing(pe: f64, pn: f64, cx: f64, cy: f64, signed_r: f64) -> f64 {
    let re = pe - cx;
    let rn = pn - cy;
    let te = if signed_r >= 0.0 { rn } else { -rn };
    let tn = if signed_r >= 0.0 { -re } else { re };
    (te.atan2(tn) * 180.0 / std::f64::consts::PI + 360.0) % 360.0
}

/// A straight's values from its two plane points (`computeStraightValuesUtm`): (length, bearing).
pub fn straight_values(s: Utm, e: Utm) -> (f64, f64) {
    let dx = e.e - s.e;
    let dy = e.n - s.n;
    let length = (dx * dx + dy * dy).sqrt();
    let bearing = (dx.atan2(dy) * 180.0 / std::f64::consts::PI + 360.0) % 360.0;
    (trim_noise(length), bearing)
}

fn norm_tau(a: f64) -> f64 {
    ((a % TAU) + TAU) % TAU
}

fn sweep_of(signed_r: f64, a1: f64, a2: f64) -> f64 {
    if signed_r >= 0.0 {
        -((norm_tau(a1) - norm_tau(a2) + TAU) % TAU)
    } else {
        (norm_tau(a2) - norm_tau(a1) + TAU) % TAU
    }
}

/// An arc's values from its two plane points and radius (`computeCurvedValuesUtm`): (length, bearing, endBearing).
pub fn curved_values(s: Utm, e: Utm, signed_r: f64) -> (f64, f64, f64) {
    let Some((cx, cy, _, abs_r)) = arc_center(s.e, s.n, e.e, e.n, signed_r) else {
        return (0.0, 0.0, 0.0);
    };
    let a1 = (s.n - cy).atan2(s.e - cx);
    let a2 = (e.n - cy).atan2(e.e - cx);
    let sweep = sweep_of(signed_r, a1, a2);
    (
        trim_noise(abs_r * sweep.abs()),
        tangent_bearing(s.e, s.n, cx, cy, signed_r),
        tangent_bearing(e.e, e.n, cx, cy, signed_r),
    )
}

/// `computeStraightValuesUtm` as the JavaScript returns it.
pub fn straight_values_json(s: Utm, e: Utm) -> Value {
    let (length, bearing) = straight_values(s, e);
    json!({ "startNode": [number(s.e), number(s.n)], "endNode": [number(e.e), number(e.n)], "epsg": s.zone, "length": number(length), "bearing": number(bearing) })
}

/// `computeCurvedValuesUtm` as the JavaScript returns it.
pub fn curved_values_json(s: Utm, e: Utm, signed_r: f64) -> Value {
    let (length, bearing, end_bearing) = curved_values(s, e, signed_r);
    json!({ "startNode": [number(s.e), number(s.n)], "endNode": [number(e.e), number(e.n)], "epsg": s.zone, "length": number(length), "bearing": number(bearing), "endBearing": number(end_bearing), "radius": number(signed_r) })
}

/// The arc polyline in the plane, at least `min_segments` segments (`arcPlanePoints`).
pub fn arc_plane_points(s: Utm, e: Utm, signed_r: f64, max_deviation: f64, min_segments: usize) -> Option<Vec<[f64; 2]>> {
    let (cx, cy, _, abs_r) = arc_center(s.e, s.n, e.e, e.n, signed_r)?;
    let a1 = (s.n - cy).atan2(s.e - cx);
    let a2 = (e.n - cy).atan2(e.e - cx);
    let sweep = sweep_of(signed_r, a1, a2);
    let max_angle = 2.0 * (1.0 - max_deviation / abs_r).acos();
    let n = min_segments.max((sweep.abs() / max_angle).ceil() as usize);
    Some(
        (0..=n)
            .map(|i| {
                let angle = a1 + (i as f64 / n as f64) * sweep;
                [cx + abs_r * angle.cos(), cy + abs_r * angle.sin()]
            })
            .collect(),
    )
}

fn wgs(e: f64, n: f64, zone: i64) -> Result<Value, Unsupported> {
    let (lng, lat) = plane_to_wgs84(e, n, zone)?;
    Ok(json!([number(lng), number(lat)]))
}

/// The arc polyline in WGS 84 (`arcCoordsFromRadiusUtm`).
pub fn arc_coords_from_radius(s: Utm, e: Utm, signed_r: f64, max_deviation: f64) -> Result<Option<Vec<Value>>, Unsupported> {
    match arc_plane_points(s, e, signed_r, max_deviation, 2) {
        None => Ok(None),
        Some(pts) => pts.iter().map(|[x, y]| wgs(*x, *y, s.zone)).collect::<Result<Vec<_>, _>>().map(Some),
    }
}

/// The polyline an element is drawn with on the map (`displayCoords`).
pub fn display_coords(el: &Value, epsg: i64, spacing: f64) -> Result<Vec<Value>, Unsupported> {
    let coords: Vec<Value> = el
        .get("geometry")
        .and_then(|g| g.get("coordinates"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if coords.len() < 2 || epsg == 0 {
        return Ok(coords);
    }
    let (Some([se, sn]), Some([ee, en])) = (point(el.get("startNode")), point(el.get("endNode"))) else {
        return Ok(coords);
    };
    let s = Utm { e: se, n: sn, zone: epsg };
    let e = Utm { e: ee, n: en, zone: epsg };
    let length = num(el, "length").unwrap_or(0.0);
    let min_segments = if truthy(el.get("switchId")) {
        2usize.max((length / SWITCH_ROUTE_STEP).ceil() as usize)
    } else {
        2
    };
    let radius = num(el, "radius").filter(|r| *r != 0.0 && !r.is_nan());
    let plane: Option<Vec<[f64; 2]>> = if let Some(r) = radius {
        arc_plane_points(s, e, r, SAGITTA_DISPLAY, min_segments)
    } else if num(el, "elementType") == Some(2.0) && has(el, "r1") {
        let bearing = num(el, "bearing").filter(|b| b.is_finite());
        let Some(bearing) = bearing.filter(|_| length > 0.0) else { return Ok(coords) };
        Some(transition_plane_points(
            se,
            sn,
            bearing,
            length,
            num(el, "r1"),
            num(el, "r2"),
            is_bloss(str_of(el, "transitionType")),
            SAGITTA_DISPLAY,
            min_segments,
        ))
    } else if coords.len() == 2 {
        let n = ((ee - se).hypot(en - sn) / spacing).ceil() as usize;
        if n < 2 {
            return Ok(coords);
        }
        Some(
            (0..=n)
                .map(|i| [se + (ee - se) * i as f64 / n as f64, sn + (en - sn) * i as f64 / n as f64])
                .collect(),
        )
    } else {
        None
    };
    let Some(plane) = plane else { return Ok(coords) };
    let mut out = vec![coords[0].clone()];
    for [x, y] in &plane[1..plane.len() - 1] {
        out.push(wgs(*x, *y, epsg)?);
    }
    out.push(coords[coords.len() - 1].clone());
    Ok(out)
}

/// The default spacing of `displayCoords`.
pub const DISPLAY_SPACING: f64 = STRAIGHT_VERTEX_SPACING;

/// The end of a straight of `length` from `s` at `bearing` (`endPointStraightUtm`).
pub fn end_point_straight(s: Utm, bearing: f64, length: f64) -> Utm {
    let rad = bearing * std::f64::consts::PI / 180.0;
    Utm {
        e: s.e + length * rad.sin(),
        n: s.n + length * rad.cos(),
        zone: s.zone,
    }
}

/// The end of an arc of `arc_length` from `s` at `bearing` with signed radius (`endPointCurvedUtm`).
pub fn end_point_curved(s: Utm, bearing: f64, arc_length: f64, signed_r: f64) -> Utm {
    let abs_r = signed_r.abs();
    let rad = bearing * std::f64::consts::PI / 180.0;
    let sgn = if signed_r >= 0.0 { -1.0 } else { 1.0 };
    let (te, tn) = (rad.sin(), rad.cos());
    let cx = s.e + sgn * abs_r * (-tn);
    let cy = s.n + sgn * abs_r * te;
    let a1 = (s.n - cy).atan2(s.e - cx);
    let a2 = a1 + sgn * arc_length / abs_r;
    Utm {
        e: cx + abs_r * a2.cos(),
        n: cy + abs_r * a2.sin(),
        zone: s.zone,
    }
}

/// Along/perpendicular offset of a point from a line through `s` (`projectOnBearingUtm`).
pub fn project_on_bearing(s: Utm, p: Utm, bearing: f64) -> (f64, f64) {
    let rad = bearing * std::f64::consts::PI / 180.0;
    let (dir_e, dir_n) = (rad.sin(), rad.cos());
    let (de, dn) = (p.e - s.e, p.n - s.n);
    (de * dir_e + dn * dir_n, de * dir_n - dn * dir_e)
}

/// The same for an arc (`projectOnArcUtm`).
pub fn project_on_arc(s: Utm, p: Utm, bearing: f64, signed_r: Option<f64>) -> (f64, f64) {
    let Some(r) = signed_r.filter(|r| *r != 0.0 && !r.is_nan()) else {
        return project_on_bearing(s, p, bearing);
    };
    let abs_r = r.abs();
    let rad = bearing * std::f64::consts::PI / 180.0;
    let sgn = if r >= 0.0 { -1.0 } else { 1.0 };
    let cx = s.e + sgn * abs_r * (-rad.cos());
    let cy = s.n + sgn * abs_r * rad.sin();
    let a1 = (s.n - cy).atan2(s.e - cx);
    let ap = (p.n - cy).atan2(p.e - cx);
    let pi = std::f64::consts::PI;
    let d = ((ap - a1 + pi) % TAU + TAU) % TAU - pi;
    let rr = (p.e - cx).hypot(p.n - cy);
    (sgn * d * abs_r, sgn * (rr - abs_r))
}

/// The signed radius of the circle through three points (`signedRadiusFrom3PointsUtm`).
pub fn signed_radius_from_3_points(p1: Utm, p2: Utm, p3: Utm) -> Option<f64> {
    let d = 2.0 * (p1.e * (p2.n - p3.n) + p2.e * (p3.n - p1.n) + p3.e * (p1.n - p2.n));
    if d.abs() < 1e-10 {
        return None;
    }
    let s1 = p1.e.powf(2.0) + p1.n.powf(2.0);
    let s2 = p2.e.powf(2.0) + p2.n.powf(2.0);
    let s3 = p3.e.powf(2.0) + p3.n.powf(2.0);
    let cx = (s1 * (p2.n - p3.n) + s2 * (p3.n - p1.n) + s3 * (p1.n - p2.n)) / d;
    let cy = (s1 * (p3.e - p2.e) + s2 * (p1.e - p3.e) + s3 * (p2.e - p1.e)) / d;
    let r = ((p1.e - cx).powf(2.0) + (p1.n - cy).powf(2.0)).sqrt();
    let (dx, dy) = (p2.e - p1.e, p2.n - p1.n);
    Some(if (dx * (cy - p1.n) - dy * (cx - p1.e)) > 0.0 { -r } else { r })
}

/// End bearing of an element in memory (`resolveEndBearing`).
pub fn resolve_end_bearing(el: &Value, epsg: i64) -> f64 {
    if let Some(b) = get(el, "endBearing").and_then(Value::as_f64) {
        return b;
    }
    if let (Some(r), Some([se, sn]), Some([ee, en])) = (
        num(el, "radius").filter(|r| *r != 0.0 && !r.is_nan()),
        raw_point(el.get("startNode")),
        raw_point(el.get("endNode")),
    ) {
        return curved_values(Utm { e: se, n: sn, zone: epsg }, Utm { e: ee, n: en, zone: epsg }, r).2;
    }
    get(el, "bearing").and_then(Value::as_f64).unwrap_or(0.0)
}

fn raw_point(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v.filter(|v| truthy(Some(v)))?.as_array()?;
    Some([
        a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ])
}

fn flip(v: Option<&Value>) -> Value {
    match v.and_then(Value::as_f64) {
        Some(x) => number(-x),
        None => Value::Null,
    }
}

/// An element running the other way (`reverseElement`).
pub fn reverse_element(el: &Value) -> Value {
    let bearing = num(el, "bearing").unwrap_or(f64::NAN);
    let end_bearing = get(el, "endBearing").and_then(Value::as_f64);
    let new_bearing = (end_bearing.unwrap_or(bearing) + 180.0) % 360.0;
    let mut out: Object = el.as_object().cloned().unwrap_or_default();
    let start = el.get("startNode").cloned();
    let end = el.get("endNode").cloned();
    match end {
        Some(v) => out.insert("startNode".into(), v),
        None => out.shift_remove("startNode"),
    };
    match start {
        Some(v) => out.insert("endNode".into(), v),
        None => out.shift_remove("endNode"),
    };
    out.insert("bearing".into(), number(new_bearing));
    if end_bearing.is_some() {
        out.insert("endBearing".into(), number((bearing + 180.0) % 360.0));
    }
    if get(el, "radius").is_some() {
        out.insert("radius".into(), flip(el.get("radius")));
    }
    if get(el, "cant").is_some() {
        out.insert("cant".into(), flip(el.get("cant")));
    }
    if num(el, "elementType") == Some(2.0) && has(el, "r1") {
        out.insert("r1".into(), flip(get(el, "r2")));
        out.insert("r2".into(), flip(get(el, "r1")));
    }
    if has(el, "cantStart") || has(el, "cantEnd") {
        out.insert("cantStart".into(), flip(get(el, "cantEnd")));
        out.insert("cantEnd".into(), flip(get(el, "cantStart")));
    }
    let mut geometry = el.get("geometry").and_then(Value::as_object).cloned().unwrap_or_default();
    let mut coords: Vec<Value> = el
        .get("geometry")
        .and_then(|g| g.get("coordinates"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    coords.reverse();
    geometry.insert("coordinates".into(), Value::Array(coords));
    out.insert("geometry".into(), Value::Object(geometry));
    if let Some(Value::Array(rc)) = el.get("renderCoords").filter(|v| truthy(Some(v))) {
        let mut rc = rc.clone();
        rc.reverse();
        out.insert("renderCoords".into(), Value::Array(rc));
    }
    Value::Object(out)
}

/// An element sampled every `step` metres or less, in equal steps, as plane
/// points [[e, n], …] (`elementPoints` of the commands module).
pub fn element_points(el: &Value, epsg: i64, step: f64) -> Result<Vec<[f64; 2]>, Unsupported> {
    let len = num(el, "length").filter(|_| get(el, "length").is_some()).unwrap_or(0.0);
    let first = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0));
    let start = node_utm(el.get("startNode"), first, epsg)?;
    let n = 1usize.max((len / step - 1e-9).ceil() as usize);
    let bearing = num(el, "bearing").unwrap_or(f64::NAN);
    if num(el, "elementType") == Some(2.0) {
        return Ok(crate::transition::sample_transition(
            start.e,
            start.n,
            bearing,
            len,
            num(el, "r1"),
            num(el, "r2"),
            is_bloss(str_of(el, "transitionType")),
            n,
            1,
            len,
        ));
    }
    let radius = get(el, "radius").and_then(Value::as_f64);
    Ok((0..=n)
        .map(|i| {
            let s = len * i as f64 / n as f64;
            let p = match radius {
                Some(r) => end_point_curved(start, bearing, s, r),
                None => end_point_straight(start, bearing, s),
            };
            [p.e, p.n]
        })
        .collect())
}
