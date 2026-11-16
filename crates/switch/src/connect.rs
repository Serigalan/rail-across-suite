// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Connecting a turnout's branch, or a crossing's cross leg, to an open
//! track end (`connectToTrack.js`).

use rail_across_geom::kind::{ARC, STRAIGHT, TRANSITION, is_arc, is_transition, kind_of};
use std::cell::RefCell;
use std::collections::HashMap;

use rail_across_geom::element::{Utm, resolve_end_bearing, reverse_element};
use rail_across_geom::factory::{arc_element, straight_element, transition_element};
use rail_across_geom::transition::{curvature_of, radius_of_curvature};
use rail_across_js::{Object, get, js_hypot, js_number, number, strict_eq, truthy};
use rail_across_track::heights::track_length;
use rail_across_track::model::{join_tracks, rebuild_coords, reverse_track};
use rail_across_track::switch_model::element_belongs_to_switch;
use rail_across_track::topology::classify_track_ends;
use serde_json::{Value, json};

use crate::catalogue::STRAIGHT_CURVATURE;
use crate::mid_chain::{End, Mid, MidKind, chain_newton, mid_sign_sets, mid_unknowns};
use crate::route::{Route, switch_chain_bearing_at, switch_chain_point_utm, switch_route_bearing_at, switch_route_point_utm};

const DEG: f64 = std::f64::consts::PI / 180.0;
const MAX_HALF_TURN: f64 = 89.0;
const FIT_TOL: f64 = 1e-9;
const ROOT_TOL: f64 = 1e-6;
const PATIENCE: usize = 6;
pub const END_CHAIN_OPEN: usize = 3;

fn wrap(a: f64) -> f64 {
    let r = (((a + 180.0) % 360.0) + 360.0) % 360.0 - 180.0;
    if r == -180.0 { 180.0 } else { r }
}

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The connecting element at an open track end (`connectionAnchor`).
#[derive(Clone, Debug)]
pub struct Anchor {
    pub track_id: Value,
    pub endpoint: String,
    pub el_idx: usize,
    pub kind: &'static str,
    pub fixed: Utm,
    pub bearing: f64,
    pub r1: Option<f64>,
    pub old_length: f64,
    pub count: usize,
    pub runs: Vec<Value>,
    pub r0: Option<f64>,
    pub cant0: f64,
    pub cant_on: Option<f64>,
    pub neighbour_idx: Option<usize>,
    pub k0: f64,
}

impl Anchor {
    pub fn to_json(&self) -> Value {
        let o = |x: Option<f64>| x.map(number).unwrap_or(Value::Null);
        json!({
            "trackId": self.track_id, "endpoint": self.endpoint, "elIdx": self.el_idx, "kind": self.kind,
            "fixedUtm": self.fixed.to_json(), "bearing": number(self.bearing), "r1": o(self.r1), "oldLength": number(self.old_length),
            "count": self.count, "runs": self.runs, "r0": o(self.r0), "cant0": number(self.cant0), "cantOn": o(self.cant_on),
            "neighbourIdx": self.neighbour_idx, "k0": number(self.k0),
        })
    }
    pub fn from_json(v: &Value) -> Anchor {
        let kind = match v.get("kind").and_then(Value::as_str) {
            Some("transition") => "transition",
            Some("arc") => "arc",
            _ => "straight",
        };
        Anchor {
            track_id: v.get("trackId").cloned().unwrap_or(Value::Null),
            endpoint: v.get("endpoint").and_then(Value::as_str).unwrap_or("END").to_string(),
            el_idx: v.get("elIdx").and_then(Value::as_u64).unwrap_or(0) as usize,
            kind,
            fixed: Utm::from_json(&v["fixedUtm"]),
            bearing: f(v, "bearing").unwrap_or(f64::NAN),
            r1: f(v, "r1"),
            old_length: f(v, "oldLength").unwrap_or(f64::NAN),
            count: v.get("count").and_then(Value::as_u64).unwrap_or(1) as usize,
            runs: v.get("runs").and_then(Value::as_array).cloned().unwrap_or_default(),
            r0: f(v, "r0"),
            cant0: f(v, "cant0").unwrap_or(0.0),
            cant_on: f(v, "cantOn"),
            neighbour_idx: v.get("neighbourIdx").and_then(Value::as_u64).map(|x| x as usize),
            k0: f(v, "k0").unwrap_or(0.0),
        }
    }
}

pub fn connection_anchor(
    track: &Value,
    endpoint: &str,
    tracks: Option<&[Value]>,
    switches: &[Value],
    end_marks: &[Value],
    count: usize,
) -> Result<Anchor, &'static str> {
    let es = els(track);
    if es.is_empty() {
        return Err("connect_end_none");
    }
    if es.len() < count {
        return Err("connect_end_too_few");
    }
    let own_tracks = [track.clone()];
    let ends = classify_track_ends(tracks.unwrap_or(&own_tracks), switches, end_marks);
    let end = ends
        .as_array()
        .into_iter()
        .flatten()
        .find(|e| strict_eq(e.get("trackId"), track.get("id")) && e.get("endpoint").and_then(Value::as_str) == Some(endpoint));
    if end.is_none_or(|e| e.get("state").and_then(Value::as_str) != Some("open")) {
        return Err("connect_end_not_open");
    }
    let at_end = endpoint == "END";
    let el_idx = if at_end { es.len() - count } else { 0 };
    let own = &es[el_idx..el_idx + count];
    if own
        .iter()
        .any(|el| switches.iter().any(|sw| element_belongs_to_switch(el, sw)) || truthy(el.get("switchBranch")))
    {
        return Err("connect_end_in_switch");
    }
    if own
        .iter()
        .any(|el| is_transition(el) && get(el, "transitionType").and_then(Value::as_str).unwrap_or("clothoid") != "clothoid")
    {
        return Err("connect_end_bloss");
    }
    let runs: Vec<Value> = if at_end {
        own.to_vec()
    } else {
        own.iter().rev().map(reverse_element).collect()
    };
    let run = &runs[0];
    let kind = kind_of(run).unwrap_or(STRAIGHT);
    let epsg = js_number(track.get("epsg")) as i64;
    let neighbour_idx = if at_end {
        el_idx.checked_sub(1)
    } else {
        Some(el_idx + count).filter(|i| *i < es.len())
    };
    let neighbour = neighbour_idx.map(|i| &es[i]);
    let bearing = match neighbour {
        None => f(run, "bearing").unwrap_or(f64::NAN),
        Some(n) if at_end => resolve_end_bearing(n, epsg),
        Some(n) => (f(n, "bearing").unwrap_or(f64::NAN) + 180.0) % 360.0,
    };
    let before: Option<Value> = neighbour.map(|n| if at_end { n.clone() } else { reverse_element(n) });
    let end_radius = |e: &Value| match kind_of(e) {
        Some(TRANSITION) => f(e, "r2"),
        Some(ARC) => e.get("radius").and_then(Value::as_f64),
        _ => None,
    };
    let end_cant = |e: &Value| {
        if is_transition(e) {
            f(e, "cantEnd").unwrap_or(0.0)
        } else {
            f(e, "cant").unwrap_or(0.0)
        }
    };
    let start = run.get("startNode").and_then(Value::as_array);
    let sn = |i: usize| start.and_then(|s| s.get(i)).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let r1 = match kind {
        "transition" => f(run, "r1"),
        "arc" => run.get("radius").and_then(Value::as_f64),
        _ => None,
    };
    let k0 = curvature_of(match &before {
        Some(b) => end_radius(b),
        None if is_transition(run) => f(run, "r1"),
        None => f(run, "radius"),
    });
    Ok(Anchor {
        track_id: track.get("id").cloned().unwrap_or(Value::Null),
        endpoint: endpoint.to_string(),
        el_idx,
        kind,
        fixed: Utm {
            e: sn(0),
            n: sn(1),
            zone: epsg,
        },
        bearing,
        r1,
        old_length: own.iter().map(|e| js_number(e.get("length"))).sum(),
        count,
        r0: before.as_ref().and_then(end_radius),
        cant0: before.as_ref().map(end_cant).unwrap_or(0.0),
        cant_on: before.as_ref().and_then(|b| {
            if is_transition(b) {
                f(b, "cantEnd")
            } else {
                Some(f(b, "cant").unwrap_or(0.0))
            }
        }),
        neighbour_idx,
        k0,
        runs,
    })
}

fn arc_to(fixed: Utm, bearing: f64, port: Utm) -> Option<Route> {
    let (de, dn) = (port.e - fixed.e, port.n - fixed.n);
    let c = js_hypot(&[de, dn]);
    if c < 1e-6 {
        return None;
    }
    let alpha = wrap(de.atan2(dn) / DEG - bearing);
    if alpha.abs() >= MAX_HALF_TURN {
        return None;
    }
    let a = alpha * DEG;
    let r = c / (2.0 * a.sin());
    if !r.is_finite() || (1.0 / r).abs() < STRAIGHT_CURVATURE {
        return Some(Route { length: c, r1: None, r2: None });
    }
    Some(Route {
        length: 2.0 * a * r,
        r1: Some(r),
        r2: Some(r),
    })
}

fn newton2(miss: &dyn Fn(f64, f64) -> [f64; 2], mut l: f64, mut k: f64, iters: usize, stop_early: bool) -> Option<(f64, f64)> {
    for _ in 0..iters {
        let fv = miss(l, k);
        if js_hypot(&fv) < FIT_TOL {
            if stop_early {
                return Some((l, k));
            }
            break;
        }
        let (hl, hk) = (1e-6f64.max(1e-7 * l), 1e-9);
        let (fl, fk) = (miss(l + hl, k), miss(l, k + hk));
        let (a, b) = ((fl[0] - fv[0]) / hl, (fk[0] - fv[0]) / hk);
        let (c, d) = ((fl[1] - fv[1]) / hl, (fk[1] - fv[1]) / hk);
        let det = a * d - b * c;
        if !det.is_finite() || det.abs() < 1e-18 {
            return None;
        }
        l -= (d * fv[0] - b * fv[1]) / det;
        k -= (a * fv[1] - c * fv[0]) / det;
        if !(l > 0.0) || !k.is_finite() {
            return None;
        }
    }
    (js_hypot(&miss(l, k)) < 1e-6).then_some((l, k))
}

fn transition_to(fixed: Utm, bearing: f64, r1: Option<f64>, port: Utm) -> Option<Route> {
    let arc = arc_to(fixed, bearing, port)?;
    let k0 = curvature_of(r1);
    let route = |l: f64, k: f64| Route {
        length: l,
        r1,
        r2: radius_of_curvature(k),
    };
    let miss = |l: f64, k: f64| {
        let p = switch_route_point_utm(fixed, bearing, &route(l, k), l);
        [p.e - port.e, p.n - port.n]
    };
    let (l, k) = newton2(&miss, arc.length, 2.0 * curvature_of(arc.r1) - k0, 40, true)?;
    Some(route(l, k))
}

/// A refitted connecting element (`fitConnectingElement`).
#[derive(Clone, Debug)]
pub struct Fit {
    pub route: Route,
    pub lead: Option<Route>,
    pub end_bearing: f64,
    pub miss: f64,
    pub pieces: Option<Vec<Route>>,
}

impl Fit {
    pub fn to_json(&self) -> Value {
        let mut o = json!({ "route": self.route.to_json(), "endBearing": number(self.end_bearing), "miss": number(self.miss) });
        if let Some(l) = self.lead {
            o["lead"] = l.to_json();
        }
        o
    }
    pub fn from_json(v: &Value) -> Fit {
        Fit {
            route: crate::route::to_route(&v["route"]),
            lead: v.get("lead").filter(|l| !l.is_null()).map(crate::route::to_route),
            end_bearing: f(v, "endBearing").unwrap_or(f64::NAN),
            miss: f(v, "miss").unwrap_or(f64::NAN),
            pieces: v.get("pieces").and_then(Value::as_array).map(|_| crate::route::to_chain(&v["pieces"])),
        }
    }
}

pub fn fit_connecting_element(a: &Anchor, port: Utm, bearing_in: f64, lead: f64) -> Option<Fit> {
    if lead > 0.0 && a.kind != "transition" {
        return fit_with_lead(a, port, bearing_in, lead);
    }
    let route = if a.kind == "transition" {
        transition_to(a.fixed, a.bearing, a.r1, port)
    } else {
        arc_to(a.fixed, a.bearing, port)
    }?;
    if !(route.length > 0.0) {
        return None;
    }
    let end_bearing = switch_route_bearing_at(a.bearing, &route, route.length);
    Some(Fit {
        route,
        lead: None,
        end_bearing,
        miss: wrap(end_bearing - bearing_in),
        pieces: None,
    })
}

fn fit_with_lead(a: &Anchor, port: Utm, bearing_in: f64, lead: f64) -> Option<Fit> {
    let plain = arc_to(a.fixed, a.bearing, port)?;
    let chain = |l: f64, k: f64| {
        let r = radius_of_curvature(k);
        [Route { length: lead, r1: a.r0, r2: r }, Route { length: l, r1: r, r2: r }]
    };
    let miss = |l: f64, k: f64| {
        let p = switch_chain_point_utm(a.fixed, a.bearing, &chain(l, k), None).unwrap_or(a.fixed);
        [p.e - port.e, p.n - port.n]
    };
    let (l, k) = newton2(&miss, 1f64.max(plain.length - lead / 2.0), curvature_of(plain.r1), 50, false)?;
    let [lead_route, route] = chain(l, k);
    let end_bearing = switch_chain_bearing_at(a.bearing, &[lead_route, route], None);
    Some(Fit {
        route,
        lead: Some(lead_route),
        end_bearing,
        miss: wrap(end_bearing - bearing_in),
        pieces: None,
    })
}

/// What `portAt(x)` answers: the port, the bearing to arrive on, and whatever else the caller keeps.
#[derive(Clone, Debug)]
pub struct PortAt {
    pub port: Utm,
    pub bearing_in: f64,
    pub extra: Value,
}

/// A root of the connection (`connectionRoots`'s entries).
#[derive(Clone, Debug)]
pub struct Root {
    pub x: f64,
    pub fit: Fit,
    pub at: PortAt,
}

/// Where along the host track the construction connects (`connectionRoots`).
pub fn connection_roots(a: &Anchor, port_at: &dyn Fn(f64) -> Option<PortAt>, from: f64, to: f64, step: f64, near: Option<f64>, lead: f64) -> Vec<Root> {
    let near = near.unwrap_or((from + to) / 2.0);
    let miss = |x: f64| -> (f64, Option<(Fit, PortAt)>) {
        let at = port_at(x);
        let fit = at.as_ref().and_then(|at| fit_connecting_element(a, at.port, at.bearing_in, lead));
        match (fit, at) {
            (Some(fit), Some(at)) => (fit.miss, Some((fit, at))),
            _ => (f64::NAN, None),
        }
    };
    type Probe = (f64, f64, Option<(Fit, PortAt)>);
    let probe = |x: f64| -> Probe {
        let (fv, d) = miss(x);
        (x, fv, d)
    };
    let mut roots: Vec<Probe> = Vec::new();
    let n = 1usize.max(((to - from) / step).ceil() as usize);
    let mut prev = probe(from);
    for i in 1..=n {
        let cur = probe(from + (to - from) * i as f64 / n as f64);
        if prev.1.is_finite() && prev.1.abs() < ROOT_TOL {
            roots.push(prev.clone());
        } else if prev.1.is_finite() && cur.1.is_finite() && prev.1 * cur.1 < 0.0 && (prev.1 - cur.1).abs() < 90.0 {
            let (mut lo, mut hi) = (prev.clone(), cur.clone());
            let mut k = 0;
            while k < 100 && hi.0 - lo.0 > 1e-10 {
                let mid = probe((lo.0 + hi.0) / 2.0);
                if !mid.1.is_finite() {
                    break;
                }
                if mid.1 == 0.0 {
                    lo = mid.clone();
                    hi = mid;
                    break;
                }
                if lo.1 * mid.1 < 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
                k += 1;
            }
            let best = if lo.1.abs() <= hi.1.abs() { lo } else { hi };
            if best.1.is_finite() && best.1.abs() < ROOT_TOL {
                roots.push(best);
            }
        }
        prev = cur;
    }
    if prev.1.is_finite() && prev.1.abs() < ROOT_TOL {
        roots.push(prev);
    }
    roots.sort_by(|p, q| (p.1.abs() - q.1.abs()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut kept: Vec<Probe> = Vec::new();
    for (i, r) in roots.iter().enumerate() {
        if roots.iter().position(|q| (q.0 - r.0).abs() < 1e-3) == Some(i) {
            kept.push(r.clone());
        }
    }
    kept.sort_by(|p, q| ((p.0 - near).abs() - (q.0 - near).abs()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    kept.into_iter().filter_map(|(x, _, d)| d.map(|(fit, at)| Root { x, fit, at })).collect()
}

fn spec_type(el: &Value) -> MidKind {
    match kind_of(el) {
        Some(TRANSITION) => MidKind::Transition,
        Some(ARC) => MidKind::Arc,
        _ => MidKind::Straight,
    }
}

/// The spec the dialog starts from at an open end (`endChainDefault`).
pub fn end_chain_default(a: &Anchor) -> Vec<Mid> {
    let in_track: Vec<Value> = if a.endpoint == "END" {
        a.runs.clone()
    } else {
        a.runs.iter().rev().map(reverse_element).collect()
    };
    in_track
        .iter()
        .map(|el| {
            let kind = spec_type(el);
            let given = if kind == MidKind::Arc {
                el.get("radius").and_then(Value::as_f64)
            } else {
                None
            };
            Mid {
                kind,
                length: None,
                radius: given.map(f64::abs),
                side: given.map(|g| {
                    if g > 0.0 {
                        1.0
                    } else if g < 0.0 {
                        -1.0
                    } else {
                        g
                    }
                }),
            }
        })
        .collect()
}

/// How many values a spec leaves open (`endChainOpen`).
pub fn end_chain_open(mids: &[Mid]) -> usize {
    mid_unknowns(mids).len()
}

fn run_spec(a: &Anchor, mids: &[Mid]) -> Vec<Mid> {
    if a.endpoint == "END" {
        return mids.to_vec();
    }
    mids.iter()
        .rev()
        .map(|m| match m.side.filter(|s| *s != 0.0 && !s.is_nan()) {
            Some(s) => Mid { side: Some(-s), ..*m },
            None => *m,
        })
        .collect()
}

/// The elements at an open end laid anew to a spec (`solveEndChain`): Ok(Some((x, pieces))), Ok(None), or the error key.
pub fn solve_end_chain(
    a: &Anchor,
    mids: &[Mid],
    end_at: &dyn Fn(f64) -> Option<End>,
    station: Option<f64>,
    near: f64,
) -> Result<Option<(f64, Vec<Route>)>, &'static str> {
    let spec = run_spec(a, mids);
    let open = mid_unknowns(&spec);
    let free = station.is_none();
    if open.len() + usize::from(free) != END_CHAIN_OPEN {
        return Err("connect_chain_unknowns");
    }
    let start = End {
        utm: a.fixed,
        bearing: a.bearing,
        k: a.k0,
    };
    let cache: RefCell<HashMap<u64, Option<End>>> = RefCell::new(HashMap::new());
    let end_of = |x: f64| -> Option<End> {
        if let Some(v) = cache.borrow().get(&x.to_bits()) {
            return *v;
        }
        let v = end_at(x);
        cache.borrow_mut().insert(x.to_bits(), v);
        v
    };
    let old_k: Vec<f64> = a
        .runs
        .iter()
        .map(|el| curvature_of(if is_arc(el) { el.get("radius").and_then(Value::as_f64) } else { None }))
        .collect();
    let value = |u: &crate::mid_chain::Unknown, scale_l: f64, k: Option<f64>| -> f64 {
        if u.length {
            js_number(a.runs[u.i].get("length")) * scale_l
        } else {
            k.unwrap_or_else(|| {
                if old_k[u.i] != 0.0 && !old_k[u.i].is_nan() {
                    old_k[u.i]
                } else {
                    1.0 / 1000.0
                }
            })
        }
    };
    let variations: [(f64, Option<f64>); 4] = [(1.0, None), (1.0, Some(0.0)), (0.5, None), (1.5, None)];
    let s_starts: Vec<f64> = if free {
        vec![near, near - 20.0, near + 20.0]
    } else {
        vec![station.unwrap_or(f64::NAN)]
    };
    for signs in mid_sign_sets(&spec, 1.0) {
        for (scale_l, k) in variations {
            for &s0 in &s_starts {
                let x0: Vec<f64> = (if free { vec![s0] } else { vec![] })
                    .into_iter()
                    .chain(open.iter().map(|u| value(u, scale_l, k)))
                    .collect();
                if let Some((s2, pieces)) = chain_newton(&spec, &signs, start, &end_of, &x0, if free { None } else { station }, Some(PATIENCE)) {
                    return Ok(Some((s2, pieces)));
                }
            }
        }
    }
    Ok(None)
}

const SHAPE_FIELDS: [&str; 12] = [
    "elementType",
    "startNode",
    "endNode",
    "bearing",
    "endBearing",
    "length",
    "radius",
    "r1",
    "r2",
    "transitionType",
    "geometry",
    "renderCoords",
];

fn keep_of(old: &Value) -> Object {
    old.as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, _)| !SHAPE_FIELDS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn without_cants(o: &Object) -> Object {
    o.iter()
        .filter(|(k, _)| !matches!(k.as_str(), "cant" | "cantStart" | "cantEnd"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn merged(el: Value, keep: &Object) -> Value {
    let mut o = el.as_object().cloned().unwrap_or_default();
    for (k, v) in keep {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o)
}

fn clothoid() -> Value {
    json!("clothoid")
}

fn refitted_elements(track: &Value, a: &Anchor, fit: &Fit, port: Utm) -> Vec<Value> {
    let old = &els(track)[a.el_idx];
    let keep = keep_of(old);
    let route = fit.route;
    let (mut start, mut bearing) = (a.fixed, a.bearing);
    let mut built: Vec<(Value, Object)> = Vec::new();
    let cant_at = |u: f64| if a.endpoint == "END" { u } else { -u };
    if let Some(lead) = fit.lead {
        let plain = without_cants(&keep);
        let arc_cant = if is_transition(old) {
            f(old, "cantEnd").unwrap_or(0.0)
        } else {
            f(old, "cant").unwrap_or(0.0)
        };
        let mut extra = Object::new();
        extra.insert("cantStart".into(), number(a.cant0));
        extra.insert("cantEnd".into(), number(cant_at(arc_cant)));
        if let Ok((el, end, eb)) = transition_element(start, bearing, lead.length, lead.r1, lead.r2, &clothoid(), None, &extra) {
            built.push((el, plain));
            start = end;
            bearing = eb;
        }
    }
    let none = Object::new();
    let last = if route.r1 != route.r2 {
        transition_element(start, bearing, route.length, route.r1, route.r2, &clothoid(), Some(port), &none).map(|t| t.0)
    } else if let Some(r) = route.r1.filter(|r| *r != 0.0) {
        arc_element(start, port, r, &none)
    } else {
        straight_element(start, port, &none)
    }
    .unwrap_or(Value::Null);
    let inherits = fit.lead.is_none() && route.r1.is_some_and(|r| r != 0.0) && route.r1 == route.r2 && a.cant_on.is_some();
    let mut k = keep.clone();
    if inherits {
        k.insert("cant".into(), number(cant_at(a.cant_on.unwrap_or(0.0))));
    }
    built.push((last, k));
    let own: Vec<Value> = built
        .into_iter()
        .map(|(el, k)| merged(if a.endpoint == "END" { el } else { reverse_element(&el) }, &k))
        .collect();
    if a.endpoint == "END" { own } else { own.into_iter().rev().collect() }
}

fn chain_elements(a: &Anchor, pieces: &[Route], port: Utm) -> Vec<Value> {
    let (mut start, mut bearing) = (a.fixed, a.bearing);
    let mut cant_in = a.cant_on.unwrap_or(a.cant0);
    let none = Object::new();
    let mut built = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let last = i == pieces.len() - 1;
        let old = &a.runs[i];
        let end = if last {
            port
        } else {
            switch_chain_point_utm(start, bearing, &[*piece], None).unwrap_or(start)
        };
        let transition = piece.r1 != piece.r2;
        let el = if transition {
            transition_element(start, bearing, piece.length, piece.r1, piece.r2, &clothoid(), Some(end), &none).map(|t| t.0)
        } else if let Some(r) = piece.r1.filter(|r| *r != 0.0) {
            arc_element(start, end, r, &none)
        } else {
            straight_element(start, end, &none)
        }
        .unwrap_or(Value::Null);
        let keep = keep_of(old);
        let plain = without_cants(&keep);
        let kind = if transition {
            TRANSITION
        } else if piece.r1.is_some_and(|r| r != 0.0) {
            ARC
        } else {
            STRAIGHT
        };
        let fields = if kind == kind_of(old).unwrap_or(STRAIGHT) {
            keep
        } else if transition {
            let mut p = plain;
            if keep.contains_key("cantStart") {
                p.insert("cantStart".into(), keep["cantStart"].clone());
                p.insert("cantEnd".into(), keep.get("cantEnd").cloned().unwrap_or(Value::Null));
            }
            p
        } else {
            let mut p = plain;
            p.insert("cant".into(), number(cant_in));
            p
        };
        let made = merged(el, &fields);
        cant_in = if is_transition(&made) {
            f(&made, "cantEnd").unwrap_or(cant_in)
        } else {
            f(&made, "cantEnd").or_else(|| f(&made, "cant")).unwrap_or(0.0)
        };
        start = end;
        bearing = switch_route_bearing_at(bearing, piece, piece.length);
        built.push(made);
    }
    if a.endpoint == "END" {
        built
    } else {
        built.iter().map(reverse_element).rev().collect()
    }
}

/// The track with its connecting element refitted (`refitTrackEnd`).
pub fn refit_track_end(track: &Value, a: &Anchor, fit: &Fit, port: Utm, speed_limit: Option<f64>) -> Value {
    let cap = |el: Value| -> Value {
        match speed_limit {
            Some(lim) if lim > 0.0 && f(&el, "speed").unwrap_or(0.0) > lim => {
                let mut e = el;
                e["speed"] = number(lim);
                e
            }
            _ => el,
        }
    };
    let made: Vec<Value> = match &fit.pieces {
        Some(p) => chain_elements(a, p, port),
        None => refitted_elements(track, a, fit, port),
    }
    .into_iter()
    .map(cap)
    .collect();
    let count = a.count;
    let mut elements = Vec::new();
    for (i, e) in els(track).iter().enumerate() {
        if i == a.el_idx {
            elements.extend(made.iter().cloned());
        } else if i > a.el_idx && i < a.el_idx + count {
        } else if Some(i) == a.neighbour_idx {
            elements.push(cap(e.clone()));
        } else {
            elements.push(e.clone());
        }
    }
    let delta = made.iter().map(|e| js_number(e.get("length"))).sum::<f64>() - a.old_length;
    let total = track_length(track) + delta;
    let heights: Vec<Value> = get(track, "heights")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| {
            if a.endpoint == "BEGIN" {
                let mut q = p.clone();
                q["station"] = number(js_number(p.get("station")) + delta);
                q
            } else {
                p.clone()
            }
        })
        .filter(|p| {
            let s = js_number(p.get("station"));
            s >= -1e-6 && s <= total + 1e-6
        })
        .collect();
    let mut rest = track.as_object().cloned().unwrap_or_default();
    rest.remove("heights");
    rest.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    rest.insert("elements".into(), Value::Array(elements));
    if !heights.is_empty() {
        rest.insert("heights".into(), Value::Array(heights));
    }
    Value::Object(rest)
}

/// How many elements the refitted end is made of (`refitCount`).
pub fn refit_count(fit: &Fit) -> usize {
    match &fit.pieces {
        Some(p) => p.len(),
        None if fit.lead.is_some() => 2,
        None => 1,
    }
}

/// The refitted track and the new route as one track (`joinRouteIntoTrack`): (track, shift).
pub fn join_route_into_track(refitted: &Value, a: &Anchor, route: &Value) -> (Value, f64) {
    if a.endpoint == "END" {
        return (join_tracks(refitted, &reverse_track(route)), 0.0);
    }
    let mut head = route.as_object().cloned().unwrap_or_default();
    for (k, v) in refitted.as_object().into_iter().flatten() {
        if !matches!(k.as_str(), "elements" | "coordinates" | "heights") {
            head.insert(k.clone(), v.clone());
        }
    }
    head.remove("heights");
    let mut joined = join_tracks(&Value::Object(head), refitted);
    joined["id"] = refitted.get("id").cloned().unwrap_or(Value::Null);
    (joined, track_length(route))
}
