// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Crossings and crossing switches: their angle, legs and geometry (`crossing.js`).

use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, bearing_after, curved_values, straight_values};
use rail_across_geom::factory::{arc_element, straight_element};
use rail_across_js::{Object, get, number};
use rail_across_track::switch_model::switch_element_mark;
use serde_json::{Value, json};

use crate::catalogue::STRAIGHT_CURVATURE;
use crate::route::{Route, neg_r, pts_json, route_coords, utm_end_route};

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

/// Crossing angle [rad] (`crossingAngle`).
pub fn crossing_angle(t: &Value) -> f64 {
    (1.0 / f(t, "ratio").unwrap_or(f64::NAN)).atan()
}

/// Radius of the legs, unsigned (`crossingLegRadius`).
pub fn crossing_leg_radius(t: Option<&Value>) -> Option<f64> {
    t.and_then(|t| f(t, "Rk"))
}

/// Signed radius of the legs (`crossingLegSignedRadius`).
pub fn crossing_leg_signed_radius(t: Option<&Value>, cross_angle_deg: f64) -> Option<f64> {
    let r = crossing_leg_radius(t).filter(|r| *r != 0.0 && !r.is_nan())?;
    Some(if cross_angle_deg >= 0.0 { -r } else { r })
}

/// How far each end lies from the crossing point (`crossingEndDistance`).
pub fn crossing_end_distance(t: &Value) -> f64 {
    f(t, "lt")
        .or_else(|| f(t, "lb"))
        .unwrap_or_else(|| f(t, "R").unwrap_or(f64::NAN) * (crossing_angle(t) / 2.0).tan())
}

fn slip_route(t: &Value, side: &str) -> Route {
    let a = crossing_angle(t);
    let sign = if side == "left" { -1.0 } else { 1.0 };
    let r = f(t, "R").unwrap_or(f64::NAN);
    Route {
        length: r * a,
        r1: Some(sign * r),
        r2: Some(sign * r),
    }
}

fn wgs(u: Utm, zone: i64) -> [f64; 2] {
    let (lng, lat) = plane_to_wgs84(u.e, u.n, zone).unwrap_or((f64::NAN, f64::NAN));
    [lng, lat]
}

fn route_between(from: Utm, from_wgs: [f64; 2], to: Utm, to_wgs: [f64; 2], signed_r: Option<f64>) -> (Route, Vec<[f64; 2]>) {
    match signed_r.filter(|r| *r != 0.0 && !r.is_nan()) {
        None => {
            let (length, _) = straight_values(from, to);
            (Route { length, r1: None, r2: None }, vec![from_wgs, to_wgs])
        }
        Some(r) => {
            let (length, bearing, _) = curved_values(from, to, r);
            let route = Route {
                length,
                r1: Some(r),
                r2: Some(r),
            };
            let mut coords = route_coords(from, from_wgs, bearing, &route, to_wgs);
            let n = coords.len();
            coords[n - 1] = to_wgs;
            (route, coords)
        }
    }
}

/// One wedge of a crossing's body as a closed ring (`crossingWedgeRing`).
pub fn crossing_wedge_ring(to_p: &[[f64; 2]], to_q: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out = vec![to_p[to_p.len() - 1]];
    out.extend(to_q.iter().rev());
    out.extend(&to_p[1..]);
    out
}

/// Do the pieces of a route all run on `signed_r` (`piecesOnRadius`)?
pub fn pieces_on_radius(pieces: &[Route], signed_r: Option<f64>) -> bool {
    match signed_r.filter(|r| *r != 0.0 && !r.is_nan()) {
        None => pieces.iter().all(|p| p.r1.is_none() && p.r2.is_none()),
        Some(r) => {
            let k = 1.0 / r;
            let on = |x: Option<f64>| x.is_some_and(|x| (1.0 / x - k).abs() < STRAIGHT_CURVATURE);
            pieces.iter().all(|p| on(p.r1) && on(p.r2))
        }
    }
}

fn utm_pair(u: Utm) -> Value {
    json!([number(u.e), number(u.n)])
}

fn opt(v: Option<f64>) -> Value {
    v.map(number).unwrap_or(Value::Null)
}

fn js_mod(x: f64, m: f64) -> f64 {
    x % m
}

/// Geometry of a crossing or crossing switch (`computeCrossingGeometryUtm`).
pub fn compute_crossing_geometry_utm(
    centre_utm: Utm,
    bearing: f64,
    t: &Value,
    cross_angle_deg: f64,
    start_wgs_main: Option<[f64; 2]>,
    start_wgs_cross: Option<[f64; 2]>,
    epsg: Option<i64>,
) -> Value {
    let zone = epsg.unwrap_or(centre_utm.zone);
    let centre = Utm { zone, ..centre_utm };
    let td = crossing_end_distance(t);
    let cross_bearing = js_mod(bearing + cross_angle_deg + 360.0, 360.0);
    let leg_r = crossing_leg_signed_radius(Some(t), cross_angle_deg);
    let back_r = neg_r(leg_r);
    let behind = |b: f64| js_mod(b + 180.0, 360.0);
    let a_utm = utm_end_route(centre, behind(bearing), td, back_r);
    let c_utm = utm_end_route(centre, bearing, td, leg_r);
    let b_utm = utm_end_route(centre, behind(cross_bearing), td, back_r);
    let d_utm = utm_end_route(centre, cross_bearing, td, leg_r);
    let a_wgs = start_wgs_main.unwrap_or_else(|| wgs(a_utm, zone));
    let c_wgs = wgs(c_utm, zone);
    let b_wgs = start_wgs_cross.unwrap_or_else(|| wgs(b_utm, zone));
    let d_wgs = wgs(d_utm, zone);
    let o_wgs = wgs(centre, zone);
    let leg = |lb: f64, r: Option<f64>, port: [f64; 2]| {
        let mut c = route_coords(centre, o_wgs, lb, &Route { length: td, r1: r, r2: r }, port);
        let n = c.len();
        c[n - 1] = port;
        c
    };
    let to_a = leg(behind(bearing), back_r, a_wgs);
    let to_c = leg(bearing, leg_r, c_wgs);
    let to_b = leg(behind(cross_bearing), back_r, b_wgs);
    let to_d = leg(cross_bearing, leg_r, d_wgs);
    let rev = |c: &[[f64; 2]]| c.iter().rev().copied().collect::<Vec<_>>();
    let kind = t.get("kind").and_then(Value::as_str);
    let has_slip1 = matches!(kind, Some("single_slip") | Some("double_slip"));
    let has_slip2 = kind == Some("double_slip");
    let (mut slip1, mut slip2): (Option<(Route, Vec<[f64; 2]>)>, Option<(Route, Vec<[f64; 2]>)>) = (None, None);
    if leg_r.is_none() {
        let cross_right = cross_angle_deg >= 0.0;
        let build = |side: &str, from: Utm, from_wgs: [f64; 2], to_wgs: [f64; 2], lb: f64| {
            let route = slip_route(t, side);
            let coords = route_coords(from, from_wgs, lb, &route, to_wgs);
            (route, coords)
        };
        if has_slip1 {
            slip1 = Some(build(if cross_right { "right" } else { "left" }, a_utm, a_wgs, d_wgs, bearing));
        }
        if has_slip2 {
            slip2 = Some(build(if cross_right { "left" } else { "right" }, b_utm, b_wgs, c_wgs, cross_bearing));
        }
    } else {
        if has_slip1 {
            slip1 = Some(route_between(a_utm, a_wgs, d_utm, d_wgs, None));
        }
        if has_slip2 {
            let ri = f(t, "Ri").unwrap_or(f64::NAN);
            slip2 = Some(route_between(b_utm, b_wgs, c_utm, c_wgs, Some(leg_r.unwrap_or(0.0).signum() * ri)));
        }
    }
    let fill = vec![pts_json(&crossing_wedge_ring(&to_a, &to_b)), pts_json(&crossing_wedge_ring(&to_c, &to_d))];
    let mut main = rev(&to_a);
    main.extend(&to_c[1..]);
    let mut cross = rev(&to_b);
    cross.extend(&to_d[1..]);
    let sc = |s: &Option<(Route, Vec<[f64; 2]>)>| s.as_ref().map(|s| pts_json(&s.1)).unwrap_or(Value::Null);
    let sr = |s: &Option<(Route, Vec<[f64; 2]>)>| s.as_ref().map(|s| s.0.to_json()).unwrap_or(Value::Null);
    let w = |p: [f64; 2]| json!([number(p[0]), number(p[1])]);
    json!({
        "kind": t.get("kind"), "label": t.get("label"),
        "mainCoords": pts_json(&main), "crossCoords": pts_json(&cross),
        "legCoords": { "A": pts_json(&rev(&to_a)), "B": pts_json(&rev(&to_b)), "C": pts_json(&to_c), "D": pts_json(&to_d) },
        "mainLegR": opt(leg_r), "crossLegR": opt(leg_r),
        "slip1Coords": sc(&slip1), "slip2Coords": sc(&slip2), "slip1Route": sr(&slip1), "slip2Route": sr(&slip2),
        "fillCoords": fill,
        "portA": utm_pair(a_utm), "portB": utm_pair(b_utm), "portC": utm_pair(c_utm), "portD": utm_pair(d_utm),
        "portA_wgs": w(a_wgs), "portB_wgs": w(b_wgs), "portC_wgs": w(c_wgs), "portD_wgs": w(d_wgs),
        "portA_utm": a_utm.to_json(), "portB_utm": b_utm.to_json(), "portC_utm": c_utm.to_json(), "portD_utm": d_utm.to_json(),
        "mainEndDistance": number(td), "centreUtm": centre.to_json(), "mainBearing": number(bearing), "crossBearing": number(cross_bearing),
    })
}

/// The same, placed from its port A (`computeCrossingGeometryFromPortA`).
pub fn compute_crossing_geometry_from_port_a(port_utm: Utm, port_wgs: [f64; 2], bearing: f64, t: &Value, cross_angle_deg: f64) -> Value {
    let td = crossing_end_distance(t);
    let r = crossing_leg_signed_radius(Some(t), cross_angle_deg);
    let centre = utm_end_route(port_utm, bearing, td, r);
    let b = bearing_after(bearing, td, r);
    let mut g = compute_crossing_geometry_utm(centre, b, t, cross_angle_deg, Some(port_wgs), None, None);
    g["portA"] = utm_pair(port_utm);
    g["portA_utm"] = port_utm.to_json();
    g
}

fn coords_of(v: &Value) -> Vec<[f64; 2]> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| [p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)])
                .collect()
        })
        .unwrap_or_default()
}

/// The elements a crossing kind is committed as (`crossingElements`).
pub fn crossing_elements(g: &Value, identity: &Value) -> Value {
    let piece = |from: &Value, to: &Value, signed_r: Option<f64>, coords: &Value, route: &str| -> Value {
        let mut extra: Object = switch_element_mark(identity, route).as_object().cloned().unwrap_or_default();
        extra.insert("geometry".into(), json!({ "type": "LineString", "coordinates": pts_json(&coords_of(coords)) }));
        let (s, e) = (Utm::from_json(from), Utm::from_json(to));
        let el = match signed_r.filter(|r| *r != 0.0 && !r.is_nan()) {
            Some(r) => arc_element(s, e, r, &extra),
            None => straight_element(s, e, &extra),
        };
        let mut el = el.unwrap_or(Value::Null);
        if let Some(o) = el.as_object_mut() {
            o.remove("renderCoords");
        }
        el
    };
    let fo = |k: &str| g.get(k).and_then(Value::as_f64);
    let slip = |k: &str, from: &str, to: &str, coords: &str| match g.get(k).filter(|v| !v.is_null()) {
        Some(r) => piece(&g[from], &g[to], r.get("r1").and_then(Value::as_f64), &g[coords], k.trim_end_matches("Route")),
        None => Value::Null,
    };
    json!({
        "A": piece(&g["portA_utm"], &g["centreUtm"], fo("mainLegR"), &g["legCoords"]["A"], "main"),
        "C": piece(&g["centreUtm"], &g["portC_utm"], fo("mainLegR"), &g["legCoords"]["C"], "main"),
        "B": piece(&g["portB_utm"], &g["centreUtm"], fo("crossLegR"), &g["legCoords"]["B"], "cross"),
        "D": piece(&g["centreUtm"], &g["portD_utm"], fo("crossLegR"), &g["legCoords"]["D"], "cross"),
        "slip1": slip("slip1Route", "portA_utm", "portD_utm", "slip1Coords"),
        "slip2": slip("slip2Route", "portB_utm", "portC_utm", "slip2Coords"),
    })
}
