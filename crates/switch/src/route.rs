// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Switch routes and chains: a route or a chain of pieces as it runs away
//! from the point it starts at (`route.js`).

use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, bearing_after};
use rail_across_geom::precision::SWITCH_ROUTE_STEP;
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{clothoid_radius_at, curvature_of, radius_of_curvature, sample_transition, transition_bearing_at, transition_point_at};
use rail_across_js::{has, number};
use serde_json::{Value, json};

use crate::catalogue::{STRAIGHT_CURVATURE, as_radius, bent_form, branch_radius};

/// A polyline of WGS84 [lng, lat] pairs.
pub type Polyline = Vec<[f64; 2]>;

/// A route: its length and the signed radius at either end (None straight).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    pub length: f64,
    pub r1: Option<f64>,
    pub r2: Option<f64>,
}

fn opt(v: Option<f64>) -> Value {
    v.map(number).unwrap_or(Value::Null)
}

fn fv(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

impl Route {
    pub fn to_json(self) -> Value {
        json!({ "length": number(self.length), "r1": opt(self.r1), "r2": opt(self.r2) })
    }
}

/// A route from { length, r1, r2 } or { length, radius } (`toRoute`).
pub fn to_route(v: &Value) -> Route {
    let length = fv(v, "length").unwrap_or(f64::NAN);
    if has(v, "r1") || has(v, "r2") {
        let r1 = as_radius(fv(v, "r1"));
        return Route {
            length,
            r1,
            r2: if has(v, "r2") { as_radius(fv(v, "r2")) } else { r1 },
        };
    }
    let r = as_radius(fv(v, "radius"));
    Route { length, r1: r, r2: r }
}

/// A chain from a route or a list of routes (`toChain`).
pub fn to_chain(v: &Value) -> Vec<Route> {
    match v.as_array() {
        Some(a) => a.iter().map(to_route).collect(),
        None => vec![to_route(v)],
    }
}

pub fn chain_json(c: &[Route]) -> Value {
    Value::Array(c.iter().map(|r| r.to_json()).collect())
}

pub fn neg_r(r: Option<f64>) -> Option<f64> {
    r.map(|r| -r)
}

/// Does the curvature change along the route (`switchRouteVaries`)?
pub fn switch_route_varies(r: &Route) -> bool {
    r.r1 != r.r2
}

/// Signed radius at `s` (`switchRouteRadiusAt`).
pub fn switch_route_radius_at(r: &Route, s: f64) -> Option<f64> {
    if switch_route_varies(r) {
        clothoid_radius_at(r.r1, r.r2, r.length, s)
    } else {
        r.r1
    }
}

fn utm_end_straight(u: Utm, bearing: f64, length: f64) -> Utm {
    let rad = bearing * std::f64::consts::PI / 180.0;
    Utm {
        e: u.e + length * rad.sin(),
        n: u.n + length * rad.cos(),
        zone: u.zone,
    }
}

fn arc_center(u: Utm, bearing: f64, signed_r: f64) -> (f64, f64, f64, f64) {
    let abs_r = signed_r.abs();
    let rad = bearing * std::f64::consts::PI / 180.0;
    let sgn = if signed_r >= 0.0 { -1.0 } else { 1.0 };
    let (te, tn) = (rad.sin(), rad.cos());
    let cx = u.e + sgn * abs_r * (-tn);
    let cy = u.n + sgn * abs_r * te;
    (cx, cy, abs_r, sgn)
}

fn utm_end_curved(u: Utm, bearing: f64, arc_length: f64, signed_r: f64) -> Utm {
    let (cx, cy, abs_r, sgn) = arc_center(u, bearing, signed_r);
    let a1 = (u.n - cy).atan2(u.e - cx);
    let a2 = a1 + sgn * arc_length / abs_r;
    Utm {
        e: cx + abs_r * a2.cos(),
        n: cy + abs_r * a2.sin(),
        zone: u.zone,
    }
}

/// End of a route from `u` (`utmEndRoute`).
pub fn utm_end_route(u: Utm, bearing: f64, length: f64, signed_r: Option<f64>) -> Utm {
    match signed_r.filter(|r| *r != 0.0 && !r.is_nan()) {
        Some(r) => utm_end_curved(u, bearing, length, r),
        None => utm_end_straight(u, bearing, length),
    }
}

/// Point of a route at `s` (`switchRoutePointUtm`).
pub fn switch_route_point_utm(origin: Utm, bearing: f64, r: &Route, s: f64) -> Utm {
    if switch_route_varies(r) {
        let [e, n] = transition_point_at(origin.e, origin.n, bearing, r.length, r.r1, r.r2, false, s);
        Utm { e, n, zone: origin.zone }
    } else {
        utm_end_route(origin, bearing, s, r.r1)
    }
}

/// Tangent bearing of a route at `s` (`switchRouteBearingAt`).
pub fn switch_route_bearing_at(bearing: f64, r: &Route, s: f64) -> f64 {
    if switch_route_varies(r) {
        transition_bearing_at(bearing, r.length, r.r1, r.r2, false, s)
    } else {
        bearing_after(bearing, s, r.r1)
    }
}

/// The piece of a route from `a` to `b` (`switchRouteSlice`).
pub fn switch_route_slice(r: &Route, a: f64, b: f64) -> Route {
    if !switch_route_varies(r) {
        return Route {
            length: b - a,
            r1: r.r1,
            r2: r.r1,
        };
    }
    Route {
        length: b - a,
        r1: switch_route_radius_at(r, a),
        r2: switch_route_radius_at(r, b),
    }
}

/// Branch of a switch on a stem route (`switchBranchRoute`).
pub fn switch_branch_route(form_signed_r: Option<f64>, stem: &Route, length: f64) -> Route {
    let form = form_signed_r.filter(|r| *r != 0.0 && !r.is_nan());
    if !switch_route_varies(stem) {
        let r = match form {
            Some(f) => branch_radius(f, stem.r1),
            None => stem.r1,
        };
        return Route { length, r1: r, r2: r };
    }
    let k_form = form.map(|f| 1.0 / f).unwrap_or(0.0);
    let k1 = curvature_of(stem.r1);
    let k_end = k1 + (curvature_of(stem.r2) - k1) * length / stem.length;
    Route {
        length,
        r1: radius_of_curvature(k1 + k_form),
        r2: radius_of_curvature(k_end + k_form),
    }
}

/// Through route of a record whose tracks do not state it (`switchStemRoute`).
pub fn switch_stem_route(sw: &Value, length: f64) -> Route {
    let r1 = as_radius(fv(sw, "mainRadius"));
    Route {
        length,
        r1,
        r2: if has(sw, "mainRadiusEnd") { as_radius(fv(sw, "mainRadiusEnd")) } else { r1 },
    }
}

/// The route an element describes (`switchElementRoute`).
pub fn switch_element_route(el: &Value) -> Route {
    let length = fv(el, "length").unwrap_or(f64::NAN);
    if fv(el, "elementType") == Some(2.0) {
        let r1 = as_radius(fv(el, "r1"));
        return Route {
            length,
            r1,
            r2: as_radius(fv(el, "r2")),
        };
    }
    let r = as_radius(fv(el, "radius"));
    Route { length, r1: r, r2: r }
}

/// The first `length` metres of a chain (`switchChainTo`).
pub fn switch_chain_to(chain: &[Route], length: f64) -> Vec<Route> {
    let mut out: Vec<Route> = Vec::new();
    let mut before = 0.0;
    for piece in chain {
        let left = length - before;
        if left <= NODE_TOL && !out.is_empty() {
            break;
        }
        if piece.length < left {
            out.push(*piece);
            before += piece.length;
        } else {
            out.push(switch_route_slice(piece, 0.0, left));
            return out;
        }
    }
    let short = length - before;
    if !out.is_empty() && short > 0.0 && short <= NODE_TOL {
        let last = out.pop().expect("a piece");
        out.push(switch_route_slice(&last, 0.0, length - (before - last.length)));
    }
    out
}

/// A piece of a chain placed in the plane.
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub route: Route,
    pub s0: f64,
    pub start: Utm,
    pub end: Utm,
    pub bearing: f64,
    pub end_bearing: f64,
}

impl Segment {
    pub fn to_json(self) -> Value {
        json!({
            "length": number(self.route.length), "r1": opt(self.route.r1), "r2": opt(self.route.r2), "s0": number(self.s0),
            "startUtm": self.start.to_json(), "endUtm": self.end.to_json(), "bearing": number(self.bearing), "endBearing": number(self.end_bearing),
        })
    }
}

/// Every piece of a chain placed in the plane (`switchChainSegmentsUtm`).
pub fn switch_chain_segments_utm(origin: Utm, bearing: f64, chain: &[Route]) -> Vec<Segment> {
    let mut out = Vec::new();
    let (mut start, mut b, mut s0) = (origin, bearing, 0.0);
    for piece in chain {
        let end = switch_route_point_utm(start, b, piece, piece.length);
        let end_bearing = switch_route_bearing_at(b, piece, piece.length);
        out.push(Segment {
            route: *piece,
            s0,
            start,
            end,
            bearing: b,
            end_bearing,
        });
        start = end;
        b = end_bearing;
        s0 += piece.length;
    }
    out
}

/// Point of a chain at `s` — its end for None (`switchChainPointUtm`).
pub fn switch_chain_point_utm(origin: Utm, bearing: f64, chain: &[Route], s: Option<f64>) -> Option<Utm> {
    let segs = switch_chain_segments_utm(origin, bearing, chain);
    let last = segs.last()?;
    let Some(s) = s else { return Some(last.end) };
    let seg = segs.iter().find(|g| s <= g.s0 + g.route.length).unwrap_or(last);
    Some(switch_route_point_utm(seg.start, seg.bearing, &seg.route, s - seg.s0))
}

/// Tangent bearing of a chain at `s` — its end for None (`switchChainBearingAt`).
pub fn switch_chain_bearing_at(bearing: f64, chain: &[Route], s: Option<f64>) -> f64 {
    let (mut b, mut s0) = (bearing, 0.0);
    for (i, piece) in chain.iter().enumerate() {
        if let Some(s) = s
            && (s <= s0 + piece.length || i == chain.len() - 1)
        {
            return switch_route_bearing_at(b, piece, s - s0);
        }
        b = switch_route_bearing_at(b, piece, piece.length);
        s0 += piece.length;
    }
    b
}

const SLICE_EPS: f64 = 1e-9;

/// The pieces of a chain between `a` and `b` (`switchChainSlice`).
pub fn switch_chain_slice(chain: &[Route], a: f64, b: f64) -> Vec<Route> {
    let mut out = Vec::new();
    let mut s0 = 0.0;
    for piece in chain {
        let s1 = s0 + piece.length;
        let (from, to) = (a.max(s0), b.min(s1));
        if to - from > SLICE_EPS {
            out.push(switch_route_slice(piece, from - s0, to - s0));
        }
        s0 = s1;
    }
    out
}

/// A section of a form's chain (`switchFormChain`'s entries).
#[derive(Clone, Copy, Debug)]
pub struct FormSection {
    pub length: f64,
    pub signed_r: Option<f64>,
    /// A clothoid section's end radius: Some(None) where it ends straight.
    pub signed_r_end: Option<Option<f64>>,
}

impl FormSection {
    pub fn from_json(v: &Value) -> FormSection {
        FormSection {
            length: fv(v, "length").unwrap_or(f64::NAN),
            signed_r: fv(v, "signedR"),
            signed_r_end: has(v, "signedREnd").then(|| fv(v, "signedREnd")),
        }
    }
}

pub fn form_chain(v: &Value) -> Vec<FormSection> {
    match v.as_array() {
        Some(a) => a.iter().map(FormSection::from_json).collect(),
        None => vec![FormSection::from_json(v)],
    }
}

/// The interior joints of a chain (`switchChainJoints`).
pub fn switch_chain_joints(chain: &[Route]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut s = 0.0;
    for piece in chain.iter().take(chain.len().saturating_sub(1)) {
        s += piece.length;
        out.push(s);
    }
    out
}

/// The station of one route abreast of `u` of the other (`switchAbreast`).
pub fn switch_abreast(u: f64, joints: &[f64], from_length: f64, to_length: f64) -> f64 {
    let lim = from_length.min(to_length);
    let last = joints.iter().copied().filter(|j| *j < lim).fold(0.0, f64::max);
    if u <= last || !(from_length > last) {
        return u;
    }
    last + (u - last) * (to_length - last) / (from_length - last)
}

/// Branch of a switch on the stem chain (`switchBranchChain`): at every
/// station the stem's curvature abreast plus the form's. Past the last element
/// change under the turnout the branch is stretched onto the rest of the stem
/// (`switch_abreast`). Over an arc there the branch keeps the Ril's bent radius
/// (`branch_radius`); over a clothoid the stem's curvature is carried over
/// scaled by the ratio of the two lengths, so the branch turns by exactly the
/// angle the stem turns beside it and its tangent at WE meets the stem's at the
/// form's 1:n — a turnout whose WE is a transition's straight end is the plain
/// turnout there.
pub fn switch_branch_chain(sections: &[FormSection], stem: &[Route]) -> Vec<Route> {
    let total: f64 = sections.iter().map(|s| s.length).sum();
    let pieces: Vec<Route> = stem.iter().copied().filter(|p| p.length > SLICE_EPS).collect();
    if pieces.is_empty() || !(total > 0.0) {
        return vec![];
    }
    let through: f64 = pieces.iter().map(|p| p.length).sum();
    let joints = switch_chain_joints(&pieces);
    let abreast = |u: f64| switch_abreast(u, &joints, total, through);
    let last = joints.iter().copied().filter(|j| *j < total.min(through)).fold(0.0, f64::max);
    let stretch = if total > last { (through - last) / (total - last) } else { 1.0 };
    let scale = |u: f64| if u > last + SLICE_EPS { stretch } else { 1.0 };
    let mut starts = Vec::new();
    let mut at = 0.0;
    for s in sections {
        starts.push(at);
        at += s.length;
    }
    let mut cuts: Vec<f64> = [0.0, total]
        .into_iter()
        .chain(starts.iter().copied())
        .chain(joints.iter().map(|j| switch_abreast(*j, &joints, through, total)))
        .filter(|u| *u >= 0.0 && *u <= total)
        .collect();
    cuts.sort_by(|x, y| (x - y).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut dedup: Vec<f64> = Vec::new();
    for u in cuts {
        if dedup.last().is_none_or(|p| u - p > SLICE_EPS) {
            dedup.push(u);
        }
    }
    let mut cuts = dedup;
    let n = cuts.len();
    if total - cuts[n - 1] > SLICE_EPS {
        cuts.push(total);
    } else {
        cuts[n - 1] = total;
    }
    let holding = |lens: &[f64], x: f64| -> (usize, f64) {
        let mut s0 = 0.0;
        for (i, l) in lens.iter().enumerate() {
            if x <= s0 + l || i == lens.len() - 1 {
                return (i, s0);
            }
            s0 += l;
        }
        (0, 0.0)
    };
    let along = |k1: f64, k2: f64, length: f64, x: f64| {
        if x <= SLICE_EPS {
            k1
        } else if x >= length - SLICE_EPS {
            k2
        } else {
            k1 + (k2 - k1) * x / length
        }
    };
    let piece_lens: Vec<f64> = pieces.iter().map(|p| p.length).collect();
    let section_lens: Vec<f64> = sections.iter().map(|s| s.length).collect();
    let mut out = Vec::new();
    for i in 1..cuts.len() {
        let (u0, u1) = (cuts[i - 1], cuts[i]);
        let mid = (u0 + u1) / 2.0;
        let (pi, s0) = holding(&piece_lens, abreast(mid));
        let piece = pieces[pi];
        let (si, _) = holding(&section_lens, mid);
        let section = sections[si];
        let section_varies = section.signed_r_end.is_some_and(|e| e != section.signed_r);
        if !switch_route_varies(&piece) && !section_varies {
            out.push(switch_branch_route(section.signed_r, &piece, u1 - u0));
            continue;
        }
        let k = if switch_route_varies(&piece) { scale(mid) } else { 1.0 };
        let k_stem = |u: f64| k * along(curvature_of(piece.r1), curvature_of(piece.r2), piece.length, abreast(u) - s0);
        let k_form = |u: f64| {
            if section_varies {
                along(
                    curvature_of(section.signed_r),
                    curvature_of(section.signed_r_end.flatten()),
                    section.length,
                    u - starts[si],
                )
            } else {
                curvature_of(section.signed_r)
            }
        };
        out.push(Route {
            length: u1 - u0,
            r1: radius_of_curvature(k_stem(u0) + k_form(u0)),
            r2: radius_of_curvature(k_stem(u1) + k_form(u1)),
        });
    }
    out
}

/// Bauform of a turnout from its two chains (`switchChainBentForm`).
pub fn switch_chain_bent_form(stem: &[Route], branch: &[Route], symmetric: bool) -> &'static str {
    if symmetric {
        return "sym";
    }
    let mut tightest: Option<f64> = None;
    for piece in stem {
        for r in [piece.r1, piece.r2].into_iter().flatten() {
            if tightest.is_none_or(|t| r.abs() < t.abs()) {
                tightest = Some(r);
            }
        }
    }
    let Some(t) = tightest else { return "plain" };
    let (s0, b0) = (
        stem[0],
        branch.first().copied().unwrap_or(Route {
            length: 0.0,
            r1: None,
            r2: None,
        }),
    );
    if Some(t) == s0.r1 {
        return bent_form(s0.r1, b0.r1);
    }
    let k = 1.0 / t + curvature_of(b0.r1) - curvature_of(s0.r1);
    bent_form(Some(t), if k.abs() < STRAIGHT_CURVATURE { None } else { Some(1.0 / k) })
}

/// Segments a switch route gets (`switchRouteSegments`).
pub fn switch_route_segments(length: f64, signed_r: Option<f64>, signed_r_end: Option<f64>) -> usize {
    let curved = |r: Option<f64>| r.is_some_and(|r| r != 0.0 && !r.is_nan());
    if curved(signed_r) || curved(signed_r_end) {
        2usize.max((length / SWITCH_ROUTE_STEP).ceil() as usize)
    } else {
        1
    }
}

const ROUTE_SUBDIV: usize = 8;

/// Plane points of a clothoid route (`clothoidRoutePoints`).
pub fn clothoid_route_points(origin: Utm, bearing: f64, r: &Route) -> Vec<[f64; 2]> {
    let n = switch_route_segments(r.length, r.r1, r.r2);
    sample_transition(origin.e, origin.n, bearing, r.length, r.r1, r.r2, false, n, ROUTE_SUBDIV, r.length)
}

fn wgs(e: f64, n: f64, zone: i64) -> [f64; 2] {
    let (lng, lat) = plane_to_wgs84(e, n, zone).unwrap_or((f64::NAN, f64::NAN));
    [lng, lat]
}

fn utm_arc_coords(start: Utm, bearing: f64, arc_length: f64, signed_r: f64) -> Vec<[f64; 2]> {
    let (cx, cy, abs_r, sgn) = arc_center(start, bearing, signed_r);
    let a1 = (start.n - cy).atan2(start.e - cx);
    let sweep = sgn * arc_length / abs_r;
    let n = switch_route_segments(arc_length, Some(signed_r), Some(signed_r));
    (0..=n)
        .map(|i| {
            let angle = a1 + (i as f64 / n as f64) * sweep;
            wgs(cx + abs_r * angle.cos(), cy + abs_r * angle.sin(), start.zone)
        })
        .collect()
}

/// Polyline (WGS84) of a route (`routeCoords`).
pub fn route_coords(origin: Utm, origin_wgs: [f64; 2], bearing: f64, r: &Route, end_wgs: [f64; 2]) -> Vec<[f64; 2]> {
    if switch_route_varies(r) {
        let pts = clothoid_route_points(origin, bearing, r);
        let mut out = vec![origin_wgs];
        if pts.len() > 2 {
            out.extend(pts[1..pts.len() - 1].iter().map(|p| wgs(p[0], p[1], origin.zone)));
        }
        out.push(end_wgs);
        return out;
    }
    match r.r1.filter(|r| *r != 0.0 && !r.is_nan()) {
        Some(rad) => {
            let mut out = vec![origin_wgs];
            out.extend(utm_arc_coords(origin, bearing, r.length, rad).into_iter().skip(1));
            out
        }
        None => vec![origin_wgs, end_wgs],
    }
}

/// Polyline (WGS84) of a chain and its segments with theirs (`chainGeometry`).
pub fn chain_geometry(origin: Utm, origin_wgs: [f64; 2], bearing: f64, chain: &[Route], end_wgs: [f64; 2]) -> (Polyline, Vec<(Segment, Polyline)>) {
    let segs = switch_chain_segments_utm(origin, bearing, chain);
    let mut coords: Vec<[f64; 2]> = Vec::new();
    let mut out = Vec::new();
    let n = segs.len();
    for (i, seg) in segs.into_iter().enumerate() {
        let from = if i == 0 { origin_wgs } else { coords[coords.len() - 1] };
        let to = if i == n - 1 { end_wgs } else { wgs(seg.end.e, seg.end.n, origin.zone) };
        let c = route_coords(seg.start, from, seg.bearing, &seg.route, to);
        coords.extend(if i == 0 { &c[..] } else { &c[1..] });
        out.push((seg, c));
    }
    (coords, out)
}

pub fn pts_json(p: &[[f64; 2]]) -> Value {
    Value::Array(p.iter().map(|q| json!([number(q[0]), number(q[1])])).collect())
}
