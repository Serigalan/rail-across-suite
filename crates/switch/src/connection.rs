// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A switch connection: two turnouts facing each other, laid into the two
//! tracks they join, and what lies between their branches
//! (`switchConnectionUtils.js`).

use rail_across_geom::cant::with_cant_ramp;
use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, arc_coords_from_radius, project_on_arc};
use rail_across_geom::factory::{arc_element, straight_element, transition_element};
use rail_across_geom::precision::{SAGITTA_ELEMENT, SAGITTA_TRACK};
use rail_across_geom::transition::compute_clothoid;
use rail_across_js::{Object, get, js_hypot, js_round, number};
use rail_across_rules::cant::{compute_cant_def_signed, max_switch_cant, max_switch_cant_def};
use rail_across_rules::element_length::{comparison_radius_limits, min_element_length};
use serde_json::{Value, json};

use crate::catalogue::{Kind, STRAIGHT_CURVATURE, SWITCH_CONNECTION_STAGES, switch_branch_length, switch_branch_sections, switch_straight_length};
use crate::mid_chain::{End, MID_OPEN, Mid, MidAsk, end_curvature, is_default_mids, mid_unknowns, solve_mid_chain};
use crate::route::{
    FormSection, Polyline, Route, chain_json, neg_r, switch_abreast, switch_branch_chain, switch_chain_bearing_at, switch_chain_joints, switch_chain_point_utm,
    switch_chain_segments_utm, switch_route_bearing_at, switch_route_point_utm, switch_route_radius_at, switch_route_slice, switch_route_varies, to_route,
};

const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;
const PI: f64 = std::f64::consts::PI;
const SOLVE_TOL: f64 = 1e-8;
const CANT_TOL: f64 = 1.0;
const SOLVE_STEPS: usize = 40;
const PIECE_EPS: f64 = 1e-9;
const MID_ROOM: f64 = 1e-3;
const SAME_CURVATURE: f64 = 1e-6;

fn normalize_angle(a: f64) -> f64 {
    let r = ((a + PI) % (2.0 * PI) + 2.0 * PI) % (2.0 * PI) - PI;
    if r == -PI { PI } else { r }
}

fn psi_of(bearing: f64) -> f64 {
    (bearing * DEG2RAD).cos().atan2((bearing * DEG2RAD).sin())
}

fn bearing_of(psi: f64) -> f64 {
    ((psi.cos().atan2(psi.sin()) * RAD2DEG) % 360.0 + 360.0) % 360.0
}

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

fn fv(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

/// Every form of a speed through the fallback stages (`fallbackChain`).
fn fallback_chain(speed: f64) -> Vec<&'static Value> {
    SWITCH_CONNECTION_STAGES
        .iter()
        .flat_map(|t| t.iter().copied().filter(|s| fv(s, "speed") == Some(speed)))
        .collect()
}

/// A form for both turnouts, or a pair as it is (`formPair`).
pub fn form_pair(form: &Value) -> (Value, Value) {
    match form.as_array() {
        Some(a) => (a.first().cloned().unwrap_or(Value::Null), a.get(1).cloned().unwrap_or(Value::Null)),
        None => (form.clone(), form.clone()),
    }
}

fn forms_for(speed: f64, form: Option<&Value>) -> Vec<(Value, Value)> {
    match form.filter(|f| rail_across_js::truthy(Some(f))) {
        Some(f) => vec![form_pair(f)],
        None => fallback_chain(speed).into_iter().map(form_pair).collect(),
    }
}

/// The design speeds a connection can be built for (`CONNECTION_SPEEDS`).
pub fn connection_speeds() -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for t in &SWITCH_CONNECTION_STAGES[0] {
        let s = fv(t, "speed").unwrap_or(f64::NAN);
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// One element of a stem.
#[derive(Clone, Copy, Debug)]
pub struct StemPiece {
    pub start: Utm,
    pub bearing: f64,
    pub route: Route,
    pub cant_start: Option<f64>,
    pub cant_end: Option<f64>,
    pub s0: f64,
}

/// The element a turnout of the connection sits on (a stem).
#[derive(Clone, Debug)]
pub struct Stem {
    pub start: Utm,
    pub bearing: f64,
    pub route: Route,
    pub along: f64,
    pub cant_start: Option<f64>,
    pub cant_end: Option<f64>,
    pub pieces: Vec<StemPiece>,
    pub dir: f64,
}

impl Stem {
    pub fn from_json(g: &Value) -> Stem {
        let piece = |p: &Value| StemPiece {
            start: Utm::from_json(&p["startUtm"]),
            bearing: fv(p, "bearing").unwrap_or(f64::NAN),
            route: to_route(&p["route"]),
            cant_start: fv(p, "cantStart"),
            cant_end: fv(p, "cantEnd"),
            s0: fv(p, "s0").unwrap_or(0.0),
        };
        Stem {
            start: Utm::from_json(&g["startUtm"]),
            bearing: fv(g, "bearing").unwrap_or(f64::NAN),
            route: to_route(&g["route"]),
            along: fv(g, "along").unwrap_or(f64::NAN),
            cant_start: fv(g, "cantStart"),
            cant_end: fv(g, "cantEnd"),
            pieces: g
                .get("pieces")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(piece).collect())
                .unwrap_or_default(),
            dir: fv(g, "dir").unwrap_or(1.0),
        }
    }
    pub fn to_json(&self) -> Value {
        let piece = |p: &StemPiece| json!({ "startUtm": p.start.to_json(), "bearing": number(p.bearing), "route": p.route.to_json(), "cantStart": p.cant_start.map(number), "cantEnd": p.cant_end.map(number), "s0": number(p.s0) });
        let mut o = json!({
            "startUtm": self.start.to_json(), "bearing": number(self.bearing), "route": self.route.to_json(), "along": number(self.along),
            "cantStart": self.cant_start.map(number), "cantEnd": self.cant_end.map(number), "dir": number(self.dir),
        });
        if !self.pieces.is_empty() {
            o["pieces"] = Value::Array(self.pieces.iter().map(piece).collect());
        }
        o
    }
    fn reversed(&self) -> Stem {
        Stem {
            dir: -self.dir,
            ..self.clone()
        }
    }
    fn own_piece(&self) -> StemPiece {
        StemPiece {
            start: self.start,
            bearing: self.bearing,
            route: self.route,
            cant_start: self.cant_start,
            cant_end: self.cant_end,
            s0: 0.0,
        }
    }
    fn pieces(&self) -> Vec<StemPiece> {
        if self.pieces.is_empty() {
            vec![self.own_piece()]
        } else {
            self.pieces.clone()
        }
    }
    fn extent(&self) -> (f64, f64) {
        let p = self.pieces();
        let last = p[p.len() - 1];
        (p[0].s0, last.s0 + last.route.length)
    }
    fn piece_at(&self, t: f64) -> (StemPiece, f64) {
        let p = self.pieces();
        let k = p.iter().position(|q| t <= q.s0 + q.route.length).unwrap_or(p.len() - 1);
        (p[k], t - p[k].s0)
    }
    fn station(&self, s: f64) -> f64 {
        self.along + self.dir * s
    }
    fn point(&self, s: f64) -> Utm {
        let (p, u) = self.piece_at(self.station(s));
        switch_route_point_utm(p.start, p.bearing, &p.route, u)
    }
    fn bearing_at(&self, s: f64) -> f64 {
        let (p, u) = self.piece_at(self.station(s));
        let b = switch_route_bearing_at(p.bearing, &p.route, u);
        if self.dir > 0.0 { b } else { (b + 180.0) % 360.0 }
    }
    fn radius(&self, s: f64) -> Option<f64> {
        let (p, u) = self.piece_at(self.station(s));
        let r = switch_route_radius_at(&p.route, u);
        if self.dir > 0.0 { r } else { neg_r(r) }
    }
    fn cant(&self, s: f64) -> f64 {
        let (from, to) = self.extent();
        let (p, u) = self.piece_at(to.min(from.max(self.station(s))));
        let l = p.route.length;
        let a = p.cant_start.unwrap_or(0.0);
        let b = p.cant_end.unwrap_or(a);
        let c = if l > 0.0 { a + (b - a) * u / l } else { a };
        if self.dir > 0.0 { c } else { -c }
    }
    fn under(&self, s: f64, length: f64) -> Vec<Route> {
        let (a, b) = (self.station(s), self.station(s + length));
        let (lo, hi) = (a.min(b), a.max(b));
        let pieces = self.pieces();
        let n = pieces.len();
        let mut out = Vec::new();
        for (k, p) in pieces.iter().enumerate() {
            let from = if k == 0 { lo } else { lo.max(p.s0) };
            let to = if k == n - 1 { hi } else { hi.min(p.s0 + p.route.length) };
            if to - from > PIECE_EPS {
                out.push(switch_route_slice(&p.route, from - p.s0, to - p.s0));
            }
        }
        if self.dir > 0.0 {
            out
        } else {
            out.into_iter()
                .rev()
                .map(|p| Route {
                    length: p.length,
                    r1: neg_r(p.r2),
                    r2: neg_r(p.r1),
                })
                .collect()
        }
    }
    fn foot(&self, p: Utm) -> Utm {
        let at = |s: f64| -> (Utm, f64) {
            let (piece, u) = self.piece_at(self.station(s));
            let l = piece.route.length;
            let edge = if switch_route_varies(&piece.route) { l.min(0f64.max(u)) } else { u };
            let q = switch_route_point_utm(piece.start, piece.bearing, &piece.route, edge);
            let rad = switch_route_bearing_at(piece.bearing, &piece.route, edge) * DEG2RAD;
            let d = u - edge;
            (
                Utm {
                    e: q.e + d * rad.sin(),
                    n: q.n + d * rad.cos(),
                    zone: q.zone,
                },
                if self.dir > 0.0 { rad } else { rad + PI },
            )
        };
        let mut s = 0.0;
        for _ in 0..20 {
            let (q, rad) = at(s);
            let ds = (p.e - q.e) * rad.sin() + (p.n - q.n) * rad.cos();
            s += ds;
            if ds.abs() < 1e-9 {
                break;
            }
        }
        at(s).0
    }
}

/// The stem oriented towards the pick on `other` (`orientStemToward`).
pub fn orient_stem_toward(g: &Stem, other: &Stem) -> Stem {
    let rad = g.bearing_at(0.0) * DEG2RAD;
    let from = g.point(0.0);
    let target = other.point(0.0);
    let toward = (target.e - from.e) * rad.sin() + (target.n - from.n) * rad.cos();
    if toward < 0.0 { g.reversed() } else { g.clone() }
}

struct Frame {
    stem1: Stem,
    stem2: Stem,
    s1: f64,
    toe1: Utm,
    psi_toe1: f64,
    side: f64,
    flipped2: bool,
    gap: f64,
}

fn connection_frame(g1: &Stem, g2: &Stem, s: f64) -> Frame {
    let psi1 = psi_of(g1.bearing_at(0.0));
    let (u1e, u1n) = (psi1.cos(), psi1.sin());
    let psi2 = psi_of(g2.bearing_at(0.0));
    let flipped2 = psi2.cos() * u1e + psi2.sin() * u1n < 0.0;
    let t2 = if flipped2 { g2.reversed() } else { g2.clone() };
    let toe1 = g1.point(s);
    let psi_toe1 = psi_of(g1.bearing_at(s));
    let (n1e, n1n) = (-psi_toe1.sin(), psi_toe1.cos());
    let foot2 = t2.foot(toe1);
    let (de, dn) = (foot2.e - toe1.e, foot2.n - toe1.n);
    let sd = sign(de * n1e + dn * n1n);
    let side = if sd != 0.0 && !sd.is_nan() { sd } else { 1.0 };
    Frame {
        stem1: g1.clone(),
        stem2: t2,
        s1: s,
        toe1,
        psi_toe1,
        side,
        flipped2,
        gap: js_hypot(&[de, dn]),
    }
}

fn form_chain_of(sw: &Value, side: f64) -> Vec<FormSection> {
    let rf = -side * fv(sw, "R").unwrap_or(f64::NAN);
    let sg = sign(rf);
    switch_branch_sections(Some(sw))
        .into_iter()
        .map(|s| FormSection {
            length: s.length,
            signed_r: s.r.map(|r| sg * r),
            signed_r_end: (s.kind == Kind::Clothoid).then(|| s.r_end.map(|r| sg * r)),
        })
        .collect()
}

struct First {
    chain1: Vec<Route>,
    b1e: Utm,
    t1: f64,
}

fn first_branch(fc: &[FormSection], stem1: &Stem, s1: f64, through: f64) -> Option<First> {
    let chain1 = switch_branch_chain(fc, &stem1.under(s1, through));
    if chain1.is_empty() {
        return None;
    }
    let bear1 = stem1.bearing_at(s1);
    let b1e = switch_chain_point_utm(stem1.point(s1), bear1, &chain1, None)?;
    let t1 = psi_of(switch_chain_bearing_at(bear1, &chain1, None));
    Some(First { chain1, b1e, t1 })
}

#[derive(Clone)]
struct Second {
    tp2: Utm,
    bear_own: f64,
    chain2own: Vec<Route>,
    b2a: Utm,
    t2: f64,
}

fn second_branch(fc: &[FormSection], stem2: &Stem, s2: f64, through: f64) -> Option<Second> {
    let back2 = stem2.reversed();
    let chain2own = switch_branch_chain(fc, &back2.under(-s2, through));
    if chain2own.is_empty() {
        return None;
    }
    let tp2 = stem2.point(s2);
    let bear_own = back2.bearing_at(-s2);
    let b2a = switch_chain_point_utm(tp2, bear_own, &chain2own, None)?;
    let t2 = psi_of(switch_chain_bearing_at(bear_own, &chain2own, None)) + PI;
    Some(Second {
        tp2,
        bear_own,
        chain2own,
        b2a,
        t2,
    })
}

fn worst_of(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    let (x, y) = (a.unwrap_or(0.0), b.unwrap_or(0.0));
    if x.abs() >= y.abs() { a } else { b }
}

fn worst_f(a: f64, b: f64) -> f64 {
    if a.abs() >= b.abs() { a } else { b }
}

fn branch_deficiency(chain: &[Route], ua: f64, ub: f64, speed: f64) -> f64 {
    chain
        .iter()
        .map(|p| match worst_of(p.r1, p.r2).filter(|r| *r != 0.0) {
            Some(r) => compute_cant_def_signed(speed, r, Some(worst_f(ua, ub))),
            None => 0.0,
        })
        .fold(0.0, f64::max)
}

fn curvature_at(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 && !r.is_nan() => 1.0 / r,
        _ => 0.0,
    }
}

/// What lies between the branches as LP.EL.01 and LP.KS.01 see it (`checkMidRun`).
#[derive(Clone, Debug, Default)]
pub struct MidCheck {
    pub overlap: bool,
    pub short: Option<(usize, f64, f64)>,
    pub jump: Option<(usize, f64, f64)>,
    pub hints: Vec<(usize, f64, f64)>,
}

impl MidCheck {
    pub fn to_json(&self) -> Value {
        json!({
            "overlap": self.overlap,
            "short": self.short.map(|(i, l, m)| json!({ "index": i, "length": number(l), "lMin": number(m) })),
            "jump": self.jump.map(|(i, rw, l)| json!({ "index": i, "rw": number(rw), "limit": number(l) })),
            "hints": self.hints.iter().map(|(i, rw, reg)| json!({ "index": i, "rw": number(*rw), "reg": number(*reg) })).collect::<Vec<_>>(),
        })
    }
    fn short_json(&self) -> Value {
        self.short
            .map(|(i, l, m)| json!({ "index": i, "length": number(l), "lMin": number(m) }))
            .unwrap_or(Value::Null)
    }
    fn jump_json(&self) -> Value {
        self.jump
            .map(|(i, rw, l)| json!({ "index": i, "rw": number(rw), "limit": number(l) }))
            .unwrap_or(Value::Null)
    }
    fn hints_json(&self) -> Value {
        Value::Array(
            self.hints
                .iter()
                .map(|(i, rw, reg)| json!({ "index": i, "rw": number(*rw), "reg": number(*reg) }))
                .collect(),
        )
    }
}

pub fn check_mid_run(chain1: &[Route], mid: &[Route], chain2: &[Route], speed: f64) -> MidCheck {
    let overlap = mid.iter().any(|p| !(p.length >= MID_ROOM));
    let all: Vec<Route> = chain1.iter().chain(mid).chain(chain2).copied().collect();
    let at0 = chain1.len();
    let constant = |p: &Route| p.r1 == p.r2 || (curvature_at(p.r1) - curvature_at(p.r2)).abs() < SAME_CURVATURE;
    let same_as = |p: &Route, k: f64| constant(p) && (curvature_at(p.r1) - k).abs() < SAME_CURVATURE;
    let run_length = |i: usize| -> f64 {
        let p = &all[i];
        if !constant(p) {
            return p.length;
        }
        let k = curvature_at(p.r1);
        let mut length = p.length;
        let mut j = i as i64 - 1;
        while j >= 0 && same_as(&all[j as usize], k) {
            length += all[j as usize].length;
            j -= 1;
        }
        let mut j = i + 1;
        while j < all.len() && same_as(&all[j], k) {
            length += all[j].length;
            j += 1;
        }
        length
    };
    let l_min = min_element_length(speed);
    let mut short = None;
    if !overlap && let Some(lm) = l_min {
        if let Some(i) = (0..mid.len()).find(|k| run_length(at0 + k) < lm - 1e-9) {
            short = Some((i, run_length(at0 + i), lm));
        }
    }
    let mut jump = None;
    let mut hints = Vec::new();
    if let Some((reg, discretion)) = comparison_radius_limits(speed) {
        for k in 0..=mid.len() {
            let (Some(a), Some(b)) = ((at0 + k).checked_sub(1).and_then(|i| all.get(i)), all.get(at0 + k)) else {
                continue;
            };
            let step = (curvature_at(a.r2) - curvature_at(b.r1)).abs();
            if step <= SAME_CURVATURE {
                continue;
            }
            let rw = 1.0 / step;
            if rw < discretion - MID_ROOM {
                if jump.is_none() {
                    jump = Some((k, rw, discretion));
                }
            } else if rw < reg - MID_ROOM {
                hints.push((k, rw, reg));
            }
        }
    }
    MidCheck { overlap, short, jump, hints }
}

fn solve_station(at: &dyn Fn(f64) -> f64, step: f64, span: f64) -> Option<f64> {
    let (mut a, mut fa) = (0.0, at(0.0));
    if fa.abs() < SOLVE_TOL {
        return Some(0.0);
    }
    let (mut b, mut fb) = (step, at(step));
    for _ in 0..SOLVE_STEPS {
        if !fa.is_finite() || !fb.is_finite() {
            break;
        }
        if (fb - fa).abs() < 1e-15 {
            break;
        }
        let next = b - fb * (b - a) / (fb - fa);
        if !next.is_finite() {
            break;
        }
        let stalled = (next - b).abs() < 1e-12;
        a = b;
        fa = fb;
        b = next;
        fb = at(b);
        if fb.abs() < SOLVE_TOL {
            return Some(b);
        }
        if stalled {
            break;
        }
    }
    let n = 96;
    let (mut prev_s, mut prev_f) = (-span, at(-span));
    for i in 1..=n {
        let s = -span + (2.0 * span * i as f64) / n as f64;
        let f = at(s);
        if prev_f.is_finite() && f.is_finite() && prev_f == 0.0 {
            return Some(prev_s);
        }
        if prev_f.is_finite() && f.is_finite() && prev_f * f < 0.0 {
            let (mut lo, mut hi, mut flo) = (prev_s, s, prev_f);
            for _ in 0..60 {
                let mid = (lo + hi) / 2.0;
                let fm = at(mid);
                if fm.abs() < SOLVE_TOL {
                    return Some(mid);
                }
                if flo * fm < 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                    flo = fm;
                }
            }
            return Some((lo + hi) / 2.0);
        }
        prev_s = s;
        prev_f = f;
    }
    None
}

/// What one pair of forms builds (`buildConnection`'s answer).
#[derive(Clone)]
struct Conn {
    sw: Value,
    sw2: Value,
    w: f64,
    side: f64,
    delta: f64,
    s2: Option<f64>,
    tp1: Utm,
    b1e: Utm,
    b2a: Utm,
    tp2: Utm,
    lg: f64,
    signed_rg: Option<f64>,
    mid: Option<Vec<Route>>,
    check: MidCheck,
    chain1: Vec<Route>,
    chain2: Vec<Route>,
    signed_r1: Option<f64>,
    signed_r2: Option<f64>,
    l1: f64,
    l2: f64,
    cant1_start: f64,
    cant1_end: f64,
    cant2_start: f64,
    cant2_end: f64,
    cants1: Vec<(f64, f64)>,
    cants2: Vec<(f64, f64)>,
    cant_mid: f64,
    cant_lds1: f64,
    cant_lds2: f64,
    cant_def: f64,
    branch_cant_def: f64,
    bearing1: f64,
    bearing2: f64,
    bearing2a: f64,
    valid: bool,
    rule_only: bool,
    reason: Option<&'static str>,
}

fn unsolved(sw: &Value, sw2: &Value, reason: &'static str, frame: &Frame) -> Conn {
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: frame.toe1.zone,
    };
    Conn {
        sw: sw.clone(),
        sw2: sw2.clone(),
        w: (1.0 / fv(sw, "ratio").unwrap_or(f64::NAN)).atan(),
        side: frame.side,
        delta: 0.0,
        s2: None,
        tp1: frame.toe1,
        b1e: nan,
        b2a: nan,
        tp2: nan,
        lg: f64::NAN,
        signed_rg: None,
        mid: None,
        check: MidCheck::default(),
        chain1: vec![],
        chain2: vec![],
        signed_r1: None,
        signed_r2: None,
        l1: 0.0,
        l2: 0.0,
        cant1_start: 0.0,
        cant1_end: 0.0,
        cant2_start: 0.0,
        cant2_end: 0.0,
        cants1: vec![],
        cants2: vec![],
        cant_mid: 0.0,
        cant_lds1: 0.0,
        cant_lds2: 0.0,
        cant_def: 0.0,
        branch_cant_def: 0.0,
        bearing1: 0.0,
        bearing2: 0.0,
        bearing2a: 0.0,
        valid: false,
        rule_only: false,
        reason: Some(reason),
    }
}

fn to_lds(form: &Value, length: f64) -> f64 {
    length + fv(form, "lds").filter(|x| x.is_finite()).unwrap_or(0.0)
}

#[derive(Clone, Copy)]
struct At {
    residual: f64,
    lg: f64,
    delta: f64,
}

fn build_connection(sw: &Value, sw2: &Value, frame: &Frame, speed: f64, mids: Option<&[Mid]>, seed: Option<(f64, Vec<f64>)>) -> Conn {
    let (stem1, stem2, s1, side) = (&frame.stem1, &frame.stem2, frame.s1, frame.side);
    let w = (1.0 / fv(sw, "ratio").unwrap_or(f64::NAN)).atan();
    let lb = switch_branch_length(Some(sw));
    let lb2 = switch_branch_length(Some(sw2));
    let through = switch_straight_length(sw);
    let through2 = switch_straight_length(sw2);
    let fc = form_chain_of(sw, side);
    let fc2 = form_chain_of(sw2, side);
    let Some(one) = first_branch(&fc, stem1, s1, through) else {
        return unsolved(sw, sw2, "no_solution", frame);
    };
    let (b1e_e, b1e_n) = (one.b1e.e, one.b1e.n);
    let at = |s2: f64| -> Option<(Second, At)> {
        let two = second_branch(&fc2, stem2, s2, through2)?;
        let delta = normalize_angle(two.t2 - one.t1);
        let half = delta / 2.0;
        let (ue, un) = ((one.t1 + half).cos(), (one.t1 + half).sin());
        let (de, dn) = (two.b2a.e - b1e_e, two.b2a.n - b1e_n);
        let residual = de * -un + dn * ue;
        let lg = (de * ue + dn * un) / (if half.abs() < 1e-12 { 1.0 } else { half.sin() / half });
        Some((two, At { residual, lg, delta }))
    };
    let residual = |s2: f64| at(s2).map(|x| x.1.residual).unwrap_or(f64::NAN);
    let closed = solve_station(&residual, lb2, stem2.route.length + 2.0 * lb2);
    let single = mids.is_none_or(is_default_mids);
    if !single && mid_unknowns(mids.unwrap_or(&[])).len() != MID_OPEN {
        return unsolved(sw, sw2, "mid_unknowns", frame);
    }
    let mut chain_sol: Option<(f64, Vec<Route>)> = None;
    if !single {
        let first = closed.and_then(at);
        let end_at = |x: f64| -> Option<End> {
            let (h, _) = at(x)?;
            Some(End {
                utm: h.b2a,
                bearing: bearing_of(h.t2),
                k: -end_curvature(&h.chain2own),
            })
        };
        let p0 = stem2.point(0.0);
        let lg_start = match &first {
            Some((_, a)) if a.lg.is_finite() => a.lg,
            _ => js_hypot(&[b1e_e - p0.e, b1e_n - p0.n]),
        };
        let ask = MidAsk {
            mids: mids.unwrap_or(&[]),
            start: End {
                utm: one.b1e,
                bearing: bearing_of(one.t1),
                k: end_curvature(&one.chain1),
            },
            end_at: &end_at,
            s2_start: closed.unwrap_or(0.0),
            lg_start: Some(lg_start),
            delta_start: first.as_ref().map(|f| f.1.delta).unwrap_or(0.0),
            seed: seed.map(|(ds, values)| (s1 + ds, values)),
        };
        chain_sol = solve_mid_chain(&ask);
        if chain_sol.is_none() {
            return unsolved(sw, sw2, "mid_no_solution", frame);
        }
    }
    let s2 = if single { closed } else { chain_sol.as_ref().map(|c| c.0) };
    let hit = s2.and_then(at);
    let Some((two, hat)) = hit.filter(|h| !single || h.1.lg.is_finite()) else {
        return unsolved(sw, sw2, "no_solution", frame);
    };
    let s2 = s2.unwrap_or(f64::NAN);
    let delta = hat.delta;
    let mid = chain_sol.map(|c| c.1);
    let lg = match &mid {
        Some(m) => m.iter().map(|p| p.length).sum(),
        None => hat.lg,
    };
    let curvature = if lg > 0.0 { delta / lg } else { 0.0 };
    let signed_rg = if mid.is_some() || curvature.abs() < STRAIGHT_CURVATURE {
        None
    } else {
        Some(-1.0 / curvature)
    };
    let chain2: Vec<Route> = two
        .chain2own
        .iter()
        .rev()
        .map(|p| Route {
            length: p.length,
            r1: neg_r(p.r2),
            r2: neg_r(p.r1),
        })
        .collect();
    let joints1 = switch_chain_joints(&stem1.under(s1, through));
    let joints2 = switch_chain_joints(&stem2.reversed().under(-s2, through2));
    let cant_on1 = |u: f64| stem1.cant(s1 + switch_abreast(u, &joints1, lb, through));
    let cant_on2 = |v: f64| stem2.cant(s2 - switch_abreast(lb2 - v, &joints2, lb2, through2));
    let piece_cants = |chain: &[Route], at: &dyn Fn(f64) -> f64| -> Vec<(f64, f64)> {
        let mut u = 0.0;
        chain
            .iter()
            .map(|p| {
                let ends = (at(u), at(u + p.length));
                u += p.length;
                ends
            })
            .collect()
    };
    let cants1 = piece_cants(&one.chain1, &cant_on1);
    let cants2 = piece_cants(&chain2, &cant_on2);
    let (cant1_start, cant1_end) = (cants1[0].0, cants1[cants1.len() - 1].1);
    let (cant2_start, cant2_end) = (cants2[0].0, cants2[cants2.len() - 1].1);
    let cant_lds1 = stem1.cant(s1 + to_lds(sw, through));
    let cant_lds2 = stem2.cant(s2 - to_lds(sw2, through2));
    let cant_mid = js_round((cant_lds1 + cant_lds2) / 2.0);
    let def_mid = match &mid {
        Some(m) => m
            .iter()
            .flat_map(|p| [p.r1, p.r2])
            .map(|r| {
                r.filter(|r| *r != 0.0)
                    .map(|r| compute_cant_def_signed(speed, r, Some(cant_mid)))
                    .unwrap_or(0.0)
            })
            .fold(0.0, f64::max),
        None => signed_rg.map(|r| compute_cant_def_signed(speed, r, Some(cant_mid))).unwrap_or(0.0),
    };
    let mid_run = mid.clone().unwrap_or_else(|| {
        vec![Route {
            length: lg,
            r1: signed_rg,
            r2: signed_rg,
        }]
    });
    let check = check_mid_run(&one.chain1, &mid_run, &chain2, speed);
    let def_b1 = branch_deficiency(&one.chain1, cant1_start, cant1_end, speed);
    let def_b2 = branch_deficiency(&chain2, cant2_start, cant2_end, speed);
    let worst_cant = cant1_start.abs().max(cant1_end.abs()).max(cant2_start.abs()).max(cant2_end.abs());
    let mismatch = (cant_lds1 - cant_lds2).abs() > CANT_TOL;
    let reason = if !lg.is_finite() {
        Some("no_solution")
    } else if check.overlap {
        Some("too_short")
    } else if check.short.is_some() {
        Some("mid_element_short")
    } else if check.jump.is_some() {
        Some("mid_jump")
    } else if mismatch {
        Some("cant_mismatch")
    } else if worst_cant > max_switch_cant() {
        Some("cant_over")
    } else if def_b1.max(def_b2) > max_switch_cant_def() {
        Some("branch_too_sharp")
    } else if def_mid > max_switch_cant_def() {
        Some("too_sharp")
    } else {
        None
    };
    let rule_only = reason.is_some() && lg.is_finite() && !check.overlap && !mismatch;
    Conn {
        sw: sw.clone(),
        sw2: sw2.clone(),
        w,
        side,
        delta,
        s2: Some(s2),
        tp1: frame.toe1,
        b1e: one.b1e,
        b2a: two.b2a,
        tp2: two.tp2,
        lg,
        signed_rg,
        mid,
        check,
        signed_r1: one.chain1[0].r1,
        signed_r2: neg_r(two.chain2own[0].r1),
        chain1: one.chain1,
        chain2,
        l1: lb,
        l2: lb2,
        cant1_start,
        cant1_end,
        cant2_start,
        cant2_end,
        cants1,
        cants2,
        cant_mid,
        cant_lds1,
        cant_lds2,
        cant_def: def_mid,
        branch_cant_def: def_b1.max(def_b2),
        bearing1: bearing_of(frame.psi_toe1),
        bearing2: two.bear_own,
        bearing2a: bearing_of(two.t2),
        valid: reason.is_none(),
        rule_only,
        reason,
    }
}

fn on_element_range(g: &Stem, through: f64, opening: f64) -> Option<(f64, f64)> {
    let shifts = |a: f64, b: f64| {
        let (x, y) = ((a - g.along) / g.dir, (b - g.along) / g.dir);
        if x <= y { (x, y) } else { (y, x) }
    };
    let toe = shifts(0.0, g.route.length);
    let (e0, e1) = g.extent();
    let far = shifts(e0, e1);
    let far = (far.0 - opening * through, far.1 - opening * through);
    let (lo, hi) = (toe.0.max(far.0), toe.1.min(far.1));
    (hi >= lo).then_some((lo, hi))
}

fn end_run(chain: &[Route]) -> f64 {
    let Some(last) = chain.last() else { return 0.0 };
    if switch_route_varies(last) {
        return 0.0;
    }
    let k = curvature_at(last.r2);
    let mut length = 0.0;
    for p in chain.iter().rev() {
        if switch_route_varies(p) || (curvature_at(p.r1) - k).abs() >= SAME_CURVATURE {
            break;
        }
        length += p.length;
    }
    length
}

/// The two branch ends of a connection, each on its own (`connectionBranchEnds`).
pub fn connection_branch_ends(g1: &Stem, g2: &Stem, speed: f64, step: f64, reach: f64, form: Option<&Value>) -> Value {
    let frame = connection_frame(g1, g2, 0.0);
    let (stem1, stem2, side) = (&frame.stem1, &frame.stem2, frame.side);
    let grid = |whole: Option<(f64, f64)>| -> Vec<f64> {
        let Some(w) = whole else { return vec![] };
        let range = (w.0.max(-reach), w.1.min(reach));
        if range.1 < range.0 {
            return vec![];
        }
        let n = 1usize.max(((range.1 - range.0) / step).ceil() as usize);
        (0..=n).map(|i| range.0 + (range.1 - range.0) * i as f64 / n as f64).collect()
    };
    let forms: Vec<Value> = forms_for(speed, form)
        .into_iter()
        .map(|(sw, sw2)| {
            let through = switch_straight_length(&sw);
            let through2 = switch_straight_length(&sw2);
            let fc = form_chain_of(&sw, side);
            let fc2 = form_chain_of(&sw2, side);
            let branch_ok = |chain: &[Route], ua: f64, ub: f64| {
                ua.abs().max(ub.abs()) <= max_switch_cant() && branch_deficiency(chain, ua, ub, speed) <= max_switch_cant_def()
            };
            let first: Vec<Value> = grid(on_element_range(stem1, through, 1.0))
                .into_iter()
                .filter_map(|s| {
                    let one = first_branch(&fc, stem1, s, through)?;
                    let (ua, ub) = (stem1.cant(s), stem1.cant(s + through));
                    Some(json!({
                        "s": number(s), "x": number(one.b1e.e), "y": number(one.b1e.n), "psi": number(one.t1),
                        "kappa": number(-end_curvature(&one.chain1)), "tail": number(end_run(&one.chain1)),
                        "cantLds": number(stem1.cant(s + to_lds(&sw, through))), "ok": branch_ok(&one.chain1, ua, ub),
                    }))
                })
                .collect();
            let second: Vec<Value> = grid(on_element_range(stem2, through2, -1.0))
                .into_iter()
                .filter_map(|s| {
                    let two = second_branch(&fc2, stem2, s, through2)?;
                    let (ua, ub) = (stem2.cant(s - through2), stem2.cant(s));
                    let chain2: Vec<Route> = two
                        .chain2own
                        .iter()
                        .rev()
                        .map(|p| Route {
                            length: p.length,
                            r1: neg_r(p.r2),
                            r2: neg_r(p.r1),
                        })
                        .collect();
                    Some(json!({
                        "s": number(s), "x": number(two.b2a.e), "y": number(two.b2a.n), "psi": number(two.t2),
                        "kappa": number(end_curvature(&two.chain2own)), "tail": number(end_run(&two.chain2own)),
                        "cantLds": number(stem2.cant(s - to_lds(&sw2, through2))), "ok": branch_ok(&chain2, ua, ub),
                    }))
                })
                .collect();
            json!({ "sw": sw, "sw2": sw2, "through": number(through), "through2": number(through2), "first": first, "second": second })
        })
        .collect();
    json!({ "flipped2": frame.flipped2, "forms": forms })
}

/// Which speeds this pair of tracks can be connected with (`computeSwitchConnections`).
pub fn compute_switch_connections(g1: &Stem, g2: &Stem, s: f64, mids: Option<&[Mid]>, seed: Option<(f64, Vec<f64>)>) -> Vec<Value> {
    let frame = connection_frame(g1, g2, s);
    connection_speeds()
        .into_iter()
        .map(|speed| {
            let valid = fallback_chain(speed)
                .into_iter()
                .any(|sw| build_connection(sw, sw, &frame, speed, mids, seed.clone()).valid);
            json!({ "speed": number(speed), "valid": valid })
        })
        .collect()
}

fn wgs(u: Utm, zone: i64) -> [f64; 2] {
    let (lng, lat) = plane_to_wgs84(u.e, u.n, zone).unwrap_or((f64::NAN, f64::NAN));
    [lng, lat]
}

fn pj(p: [f64; 2]) -> Value {
    json!([number(p[0]), number(p[1])])
}

fn poly(p: &[[f64; 2]]) -> Value {
    Value::Array(p.iter().map(|q| pj(*q)).collect())
}

fn coords_of(v: Option<Vec<Value>>) -> Vec<[f64; 2]> {
    v.unwrap_or_default()
        .iter()
        .map(|p| [p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)])
        .collect()
}

fn route_coords(start: Utm, bearing: f64, r: &Route, sagitta: f64, start_wgs: [f64; 2], end_wgs: [f64; 2]) -> Polyline {
    if switch_route_varies(r) {
        let cl = compute_clothoid(start.e, start.n, start.zone, bearing, r.length, r.r1, r.r2, sagitta, false);
        let inner: Vec<[f64; 2]> = cl.map(|c| c.coords).unwrap_or_default();
        let mut out = vec![start_wgs];
        if inner.len() > 2 {
            out.extend(inner[1..inner.len() - 1].iter().map(|p| [p[0], p[1]]));
        }
        out.push(end_wgs);
        return out;
    }
    let Some(r1) = r.r1.filter(|x| *x != 0.0 && !x.is_nan()) else {
        return vec![start_wgs, end_wgs];
    };
    let end = switch_route_point_utm(start, bearing, r, r.length);
    match coords_of(arc_coords_from_radius(start, end, r1, sagitta).ok().flatten()) {
        a if !a.is_empty() => {
            let mut out = vec![start_wgs];
            if a.len() > 2 {
                out.extend(&a[1..a.len() - 1]);
            }
            out.push(end_wgs);
            out
        }
        _ => vec![start_wgs, end_wgs],
    }
}

/// A placed piece of a branch or the middle.
#[derive(Clone, Debug)]
pub struct Piece {
    pub start: Utm,
    pub end: Utm,
    pub bearing: f64,
    pub route: Route,
    pub s0: f64,
    pub coords: Polyline,
    pub render: Polyline,
}

impl Piece {
    pub fn to_json(&self) -> Value {
        json!({
            "startUtm": self.start.to_json(), "endUtm": self.end.to_json(), "bearing": number(self.bearing), "route": self.route.to_json(),
            "s0": number(self.s0), "coords": poly(&self.coords), "renderCoords": poly(&self.render),
        })
    }
    pub fn from_json(v: &Value) -> Piece {
        let c = |k: &str| coords_of(v.get(k).and_then(Value::as_array).cloned());
        Piece {
            start: Utm::from_json(&v["startUtm"]),
            end: Utm::from_json(&v["endUtm"]),
            bearing: fv(v, "bearing").unwrap_or(f64::NAN),
            route: to_route(&v["route"]),
            s0: fv(v, "s0").unwrap_or(0.0),
            coords: c("coords"),
            render: c("renderCoords"),
        }
    }
}

fn chain_pieces(start: Utm, bearing: f64, chain: &[Route], zone: i64, start_wgs: [f64; 2], end_wgs: [f64; 2]) -> Vec<Piece> {
    let segs = switch_chain_segments_utm(start, bearing, chain);
    let n = segs.len();
    segs.into_iter()
        .enumerate()
        .map(|(i, seg)| {
            let a = if i == 0 { start_wgs } else { wgs(seg.start, zone) };
            let b = if i == n - 1 { end_wgs } else { wgs(seg.end, zone) };
            Piece {
                start: seg.start,
                end: seg.end,
                bearing: seg.bearing,
                route: seg.route,
                s0: seg.s0,
                coords: route_coords(seg.start, seg.bearing, &seg.route, SAGITTA_ELEMENT, a, b),
                render: route_coords(seg.start, seg.bearing, &seg.route, SAGITTA_TRACK, a, b),
            }
        })
        .collect()
}

fn join_coords(parts: &[&Polyline]) -> Polyline {
    let mut out: Polyline = Vec::new();
    for p in parts {
        if out.is_empty() {
            out.extend(p.iter());
        } else {
            out.extend(p.iter().skip(1));
        }
    }
    out
}

/// A switch connection between two tracks for a speed (`solveSwitchConnection`).
pub fn solve_switch_connection(g1: &Stem, g2: &Stem, speed: f64, s: f64, mids: Option<&[Mid]>, seed: Option<(f64, Vec<f64>)>, form: Option<&Value>) -> Value {
    let zone = g1.start.zone;
    let frame = connection_frame(g1, g2, s);
    let mut c: Option<Conn> = None;
    let mut refused: Option<Conn> = None;
    for (sw, sw2) in forms_for(speed, form) {
        let built = build_connection(&sw, &sw2, &frame, speed, mids, seed.clone());
        let valid = built.valid;
        if !valid && refused.is_none() && built.rule_only {
            refused = Some(built.clone());
        }
        c = Some(built);
        if valid {
            break;
        }
    }
    let Some(mut c) = c else { return Value::Null };
    if !c.valid
        && let Some(r) = refused
    {
        c = r;
    }
    let overridable = !c.valid && c.rule_only;
    let gap = frame.gap;
    let through_length = switch_straight_length(&c.sw);
    let through_length2 = switch_straight_length(&c.sw2);
    if !c.lg.is_finite() {
        return json!({
            "TP1": frame.toe1.to_json(), "R": c.sw.get("R"), "w": number((1.0 / fv(&c.sw, "ratio").unwrap_or(f64::NAN)).atan()), "delta": 0, "Lg": Value::Null, "signedRg": Value::Null,
            "arc1Coords": Value::Null, "arc2Coords": Value::Null, "midCoords": [], "allCoords": [], "branch1": [], "branch2": [],
            "valid": false, "overridable": false, "reason": c.reason, "zone": zone, "gap": number(gap),
            "switchType": c.sw, "switchType2": c.sw2, "throughLength": number(through_length), "throughLength2": number(through_length2),
            "s2": Value::Null, "flipped2": frame.flipped2,
            "cant1Start": 0, "cant1End": 0, "cant2Start": 0, "cant2End": 0, "cantMid": 0,
        });
    }
    let (tp1w, b1ew, b2aw, tp2w) = (wgs(c.tp1, zone), wgs(c.b1e, zone), wgs(c.b2a, zone), wgs(c.tp2, zone));
    let drawn = c.valid || overridable;
    let branch1 = chain_pieces(c.tp1, c.bearing1, &c.chain1, zone, tp1w, b1ew);
    let branch2 = chain_pieces(c.b2a, c.bearing2a, &c.chain2, zone, b2aw, tp2w);
    let join = |ps: &[Piece], render: bool| join_coords(&ps.iter().map(|p| if render { &p.render } else { &p.coords }).collect::<Vec<_>>());
    let arc1 = drawn.then(|| join(&branch1, false));
    let arc1r = drawn.then(|| join(&branch1, true));
    let arc2 = drawn.then(|| join(&branch2, false));
    let arc2r = drawn.then(|| join(&branch2, true));
    let mid_arc = |sagitta: f64| -> Polyline {
        match coords_of(c.signed_rg.and_then(|r| arc_coords_from_radius(c.b1e, c.b2a, r, sagitta).ok().flatten())) {
            a if !a.is_empty() => {
                let mut out = vec![b1ew];
                if a.len() > 2 {
                    out.extend(&a[1..a.len() - 1]);
                }
                out.push(b2aw);
                out
            }
            _ => vec![b1ew, b2aw],
        }
    };
    let mid_pieces: Option<Vec<Piece>> = c.mid.as_ref().map(|m| {
        let mut placed = chain_pieces(c.b1e, switch_chain_bearing_at(c.bearing1, &c.chain1, None), m, zone, b1ew, b2aw);
        if let Some(last) = placed.last_mut() {
            last.end = c.b2a;
        }
        placed
    });
    let mid_coords = match &mid_pieces {
        Some(p) => join(p, false),
        None if drawn && c.signed_rg.is_some() => mid_arc(SAGITTA_ELEMENT),
        None => vec![b1ew, b2aw],
    };
    let mid_render = match &mid_pieces {
        Some(p) => join(p, true),
        None if drawn && c.signed_rg.is_some() => mid_arc(SAGITTA_TRACK),
        None => vec![b1ew, b2aw],
    };
    let mut all: Polyline = arc1.clone().unwrap_or_default();
    all.extend(mid_coords.iter().skip(1));
    all.extend(arc2.clone().unwrap_or_default().iter().skip(1));
    let valid = c.valid && arc1.is_some() && arc2.is_some();
    let laenge = project_on_arc(c.tp1, c.tp2, c.bearing1, frame.stem1.radius(frame.s1)).0;
    let s2 = c.s2.unwrap_or(f64::NAN);
    let opt = |x: Option<f64>| x.map(number).unwrap_or(Value::Null);
    let pair_list = |v: &[(f64, f64)]| Value::Array(v.iter().map(|(a, b)| json!([number(*a), number(*b)])).collect());
    let pieces_json = |v: &[Piece]| Value::Array(v.iter().map(Piece::to_json).collect());
    json!({
        "TP1": c.tp1.to_json(), "B1E": c.b1e.to_json(), "B2A": c.b2a.to_json(), "TP2": c.tp2.to_json(),
        "tp1Wgs": pj(tp1w), "b1eWgs": pj(b1ew), "b2aWgs": pj(b2aw), "tp2Wgs": pj(tp2w),
        "delta": number(c.delta), "s1": number(frame.s1), "s2": number(s2), "flipped2": frame.flipped2, "side": number(c.side),
        "R": c.sw.get("R"), "w": number(c.w), "L1": number(c.l1), "L2": number(c.l2), "Lg": number(c.lg),
        "signedR1": opt(c.signed_r1), "signedR2": opt(c.signed_r2), "signedRg": opt(c.signed_rg),
        "midPieces": mid_pieces.as_ref().map(|p| pieces_json(p)).unwrap_or(Value::Null),
        "midShort": c.check.short_json(), "midJump": c.check.jump_json(), "midJumpHints": c.check.hints_json(),
        "chain1": chain_json(&c.chain1), "chain2": chain_json(&c.chain2), "branch1": pieces_json(&branch1), "branch2": pieces_json(&branch2),
        "bearing2A": number(c.bearing2a),
        "stemR1": opt(frame.stem1.radius(frame.s1)), "stemR2": opt(frame.stem2.radius(s2)),
        "stemRoute1": chain_json(&frame.stem1.under(frame.s1, through_length)),
        "stemRoute2": chain_json(&frame.stem2.reversed().under(-s2, through_length2)),
        "cant1Start": number(c.cant1_start), "cant1End": number(c.cant1_end), "cant2Start": number(c.cant2_start), "cant2End": number(c.cant2_end),
        "cantMid": number(c.cant_mid), "cants1": pair_list(&c.cants1), "cants2": pair_list(&c.cants2),
        "cantLds1": number(c.cant_lds1), "cantLds2": number(c.cant_lds2), "cantDef": number(c.cant_def), "branchCantDef": number(c.branch_cant_def),
        "bearing1": number(c.bearing1), "bearing2": number(c.bearing2),
        "arc1Coords": arc1.as_deref().map(poly).unwrap_or(Value::Null), "arc1CoordsRender": arc1r.as_deref().map(poly).unwrap_or(Value::Null),
        "arc2Coords": arc2.as_deref().map(poly).unwrap_or(Value::Null), "arc2CoordsRender": arc2r.as_deref().map(poly).unwrap_or(Value::Null),
        "midCoords": poly(&mid_coords), "midCoordsRender": poly(&mid_render), "allCoords": poly(&all),
        "valid": valid, "overridable": overridable && arc1.is_some() && arc2.is_some(), "reason": c.reason,
        "zone": zone, "gap": number(gap), "laenge": number(laenge),
        "switchType": c.sw, "switchType2": c.sw2, "throughLength": number(through_length), "throughLength2": number(through_length2),
    })
}

fn connection_element(a: Utm, b: Utm, bearing: f64, route: &Route, speed: f64, cant_a: f64, cant_b: f64, coords: &Polyline, render: &Polyline) -> Value {
    let geometry = json!({ "type": "LineString", "coordinates": poly(coords) });
    let truthy = |x: f64| x != 0.0 && !x.is_nan();
    let has_cant = truthy(cant_a) || truthy(cant_b);
    if switch_route_varies(route) {
        let mut extra = Object::new();
        extra.insert("endBearing".into(), number(switch_route_bearing_at(bearing, route, route.length)));
        extra.insert("speed".into(), number(speed));
        if has_cant {
            extra.insert("cantStart".into(), number(cant_a));
            extra.insert("cantEnd".into(), number(cant_b));
        }
        extra.insert("geometry".into(), geometry);
        extra.insert("renderCoords".into(), poly(render));
        return transition_element(a, bearing, route.length, route.r1, route.r2, &json!("clothoid"), Some(b), &extra)
            .map(|t| t.0)
            .unwrap_or(Value::Null);
    }
    let mut extra = Object::new();
    extra.insert("speed".into(), number(speed));
    extra.insert("geometry".into(), geometry);
    let el = match route.r1.filter(|r| *r != 0.0 && !r.is_nan()) {
        Some(r) => {
            extra.insert("renderCoords".into(), poly(render));
            arc_element(a, b, r, &extra)
        }
        None => straight_element(a, b, &extra),
    }
    .unwrap_or(Value::Null);
    if has_cant { with_cant_ramp(&el, cant_a, cant_b) } else { el }
}

/// The track elements of a solved connection (`buildConnectionElements`).
pub fn build_connection_elements(result: &Value, speed: f64) -> Value {
    let pieces = |k: &str| -> Vec<Piece> {
        result
            .get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(Piece::from_json).collect())
            .unwrap_or_default()
    };
    let cants = |k: &str| -> Option<Vec<(Option<f64>, Option<f64>)>> {
        result
            .get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|p| (p.get(0).and_then(Value::as_f64), p.get(1).and_then(Value::as_f64))).collect())
    };
    let f = |k: &str| fv(result, k).unwrap_or(f64::NAN);
    let branch_els = |ps: &[Piece], cs: Option<Vec<(Option<f64>, Option<f64>)>>, ca: f64, cb: f64| -> Vec<Value> {
        let total: f64 = ps.iter().map(|p| p.route.length).sum();
        let cant_at = |s: f64| if total > 0.0 { ca + (cb - ca) * s / total } else { ca };
        ps.iter()
            .enumerate()
            .map(|(i, p)| {
                let c = cs.as_ref().and_then(|c| c.get(i)).copied().unwrap_or((None, None));
                connection_element(
                    p.start,
                    p.end,
                    p.bearing,
                    &p.route,
                    speed,
                    c.0.unwrap_or_else(|| cant_at(p.s0)),
                    c.1.unwrap_or_else(|| cant_at(p.s0 + p.route.length)),
                    &p.coords,
                    &p.render,
                )
            })
            .collect()
    };
    let cant_mid = f("cantMid");
    let mid_els: Vec<Value> = match result.get("midPieces").and_then(Value::as_array) {
        Some(mp) => mp
            .iter()
            .map(Piece::from_json)
            .map(|p| connection_element(p.start, p.end, p.bearing, &p.route, speed, cant_mid, cant_mid, &p.coords, &p.render))
            .collect(),
        None => {
            let rg = fv(result, "signedRg");
            let mid = Route { length: 0.0, r1: rg, r2: rg };
            let c = |k: &str| coords_of(result.get(k).and_then(Value::as_array).cloned());
            vec![connection_element(
                Utm::from_json(&result["B1E"]),
                Utm::from_json(&result["B2A"]),
                f64::NAN,
                &mid,
                speed,
                cant_mid,
                cant_mid,
                &c("midCoords"),
                &c("midCoordsRender"),
            )]
        }
    };
    json!({
        "branch1": branch_els(&pieces("branch1"), cants("cants1"), f("cant1Start"), f("cant1End")),
        "midEl": mid_els.first().cloned().unwrap_or(Value::Null),
        "midEls": mid_els,
        "branch2": branch_els(&pieces("branch2"), cants("cants2"), f("cant2Start"), f("cant2End")),
    })
}
