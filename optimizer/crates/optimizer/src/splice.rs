// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Splicing two elements into one chain — the construction of "Zwei Gleise
//! zusammenfügen" (services/optimizer's splice.py and the app's
//! solveSplice.js, which it replaces both; decision 0019). Reconnecting
//! existing elements ([`crate::reconnect`]) builds its arcs with it too.
//!
//! The two picked elements come in the plane of their track (one CRS for
//! both): each with its two ends, the bearing at its end in the element's own
//! direction and its signed radius (None for a straight). Which ends meet is
//! found here, not taken from the clicks (Paket S, AP S.2): each pick can be
//! joined at either end, which makes four pairs of ends, and every one is
//! constructed. Each construction runs the departure into the splice and meets
//! the arrival at its end, traversing it backwards; a pick joined at its other
//! end is handed over flipped. What comes back is every pair that fits, the
//! best first; the panel proposes that one and lets the others be chosen
//! (Entscheidung 180). Each is judged as a whole by the rule catalogue — what
//! is left of the two picks, what is inserted and the joints — and an error
//! there holds the panel's commit back.
//!
//! A solution is the chain from the departure's far end to the arrival's far
//! end, in travel order, every element in the app's plane form and with a
//! `role`: 'dep' and 'arr' for what is left of the two picked elements,
//! re-shaped, 'new' for what the splice inserts. Each element carries the
//! design speed and cant it will have, and the transitions it inserts are as
//! long as their mode asks (AP S.3): as given, or the Regellänge or
//! Mindestlänge between their neighbours in the very chain solved.
//!
//! Conventions are the geometry kernel's: bearings in degrees clockwise from
//! grid north, signed radius > 0 for a right-hand curve.

use crate::clearance::{self, CantModel, Neighbour, Spacing, auto_cant, check, described, largest_radius};
use crate::element::Element;
use crate::geometry::{Profile, Pt, arc_center, arc_forward, arc_sweep, dir_of, fit_curve_group, transition_end, transition_shift};
use crate::grenzen::{LengthBy, TransitionLengths, transition_lengths};
use crate::pruefung::{Context, check_track};
use crate::rules::Rules;
use serde_json::{Map, Value as Json, json};
use std::f64::consts::PI;

const DEG2RAD: f64 = PI / 180.0;
const RAD2DEG: f64 = 180.0 / PI;

/// A tangent point may lie this far behind the start of its element [m] —
/// the rounding of the element ends, not a construction that does not fit.
const TOL: f64 = 0.01;
/// Beyond this a transition is not a design any more [m].
const MAX_TRANSITION_LENGTH: f64 = 5000.0;
/// Samples along the arc a straight is spliced to, when the leaving point is
/// looked for: dense enough that two roots of the closing condition are never
/// within one step of each other on any railway geometry.
const ARC_STRAIGHT_SAMPLES: usize = 2000;

/// What a splice that does not fit says: the key the app translates, and what would fit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpliceError {
    pub code: String,
    pub r_max: Option<f64>,
    pub r_min: Option<f64>,
    pub l_max: Option<f64>,
    pub d_min: Option<f64>,
}

impl SpliceError {
    pub fn new(code: impl Into<String>) -> SpliceError {
        SpliceError {
            code: code.into(),
            ..SpliceError::default()
        }
    }

    pub fn params(&self) -> Json {
        let mut out = Map::new();
        for (key, value) in [("rMax", self.r_max), ("rMin", self.r_min), ("lMax", self.l_max), ("dMin", self.d_min)] {
            if let Some(v) = value {
                out.insert(key.into(), json!(v));
            }
        }
        Json::Object(out)
    }
}

type Res<T> = Result<T, SpliceError>;

// ── elements in the app's form ───────────────────────────────────────────────

/// What an element of a solution is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Dep,
    Arr,
    New,
}

impl Role {
    fn id(self) -> &'static str {
        match self {
            Role::Dep => "dep",
            Role::Arr => "arr",
            Role::New => "new",
        }
    }

    fn swapped(self) -> Role {
        match self {
            Role::Dep => Role::Arr,
            Role::Arr => Role::Dep,
            Role::New => Role::New,
        }
    }
}

/// An element of a solution, in the app's plane form.
#[derive(Debug, Clone)]
pub struct El {
    /// 0 straight, 1 arc, 2 transition.
    pub kind: u8,
    pub role: Role,
    pub start: Pt,
    pub end: Pt,
    pub bearing: f64,
    pub end_bearing: Option<f64>,
    pub length: f64,
    pub radius: Option<f64>,
    pub r1: Option<f64>,
    pub r2: Option<f64>,
    pub profile: Profile,
    pub speed: Option<f64>,
    pub cant: Option<f64>,
}

impl El {
    pub fn end_bearing(&self) -> f64 {
        self.end_bearing.unwrap_or(self.bearing)
    }

    /// The element as the rule check reads it.
    pub fn element(&self) -> Element {
        Element {
            element_type: Some(i64::from(self.kind)),
            transition_type: (self.kind == 2).then(|| self.profile.id().to_string()),
            radius: self.radius,
            r1: self.r1,
            r2: self.r2,
            length: Some(self.length),
            speed: self.speed,
            cant: self.cant,
            ..Element::default()
        }
    }

    pub fn to_json(&self) -> Json {
        let mut out = Map::new();
        out.insert("elementType".into(), json!(self.kind));
        out.insert("role".into(), json!(self.role.id()));
        if self.kind == 2 {
            out.insert("transitionType".into(), json!(self.profile.id()));
            out.insert("r1".into(), json!(self.r1));
            out.insert("r2".into(), json!(self.r2));
        }
        if self.kind == 1 {
            out.insert("radius".into(), json!(self.radius));
        }
        out.insert("startNode".into(), json!([self.start.0, self.start.1]));
        out.insert("endNode".into(), json!([self.end.0, self.end.1]));
        out.insert("bearing".into(), json!(self.bearing));
        if let Some(b) = self.end_bearing {
            out.insert("endBearing".into(), json!(b));
        }
        out.insert("length".into(), json!(self.length));
        if let Some(v) = self.speed {
            out.insert("speed".into(), json!(v));
        }
        if let Some(c) = self.cant {
            out.insert("cant".into(), json!(c));
        }
        Json::Object(out)
    }
}

fn bearing_of(frm: Pt, to: Pt) -> f64 {
    ((to.0 - frm.0).atan2(to.1 - frm.1) * RAD2DEG).rem_euclid(360.0)
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

pub fn straight(frm: Pt, to: Pt, role: Role) -> El {
    El {
        kind: 0,
        role,
        start: frm,
        end: to,
        bearing: bearing_of(frm, to),
        end_bearing: None,
        length: dist(frm, to),
        radius: None,
        r1: None,
        r2: None,
        profile: Profile::Clothoid,
        speed: None,
        cant: None,
    }
}

/// The arc of `signed_r` from `frm` to `to`, the way it turns — always the one shorter than half a circle.
pub fn arc(frm: Pt, to: Pt, signed_r: f64, role: Role) -> Res<El> {
    let centre = arc_center(frm, to, signed_r).ok_or_else(|| SpliceError::new("splice_error_no_fit"))?;
    let sweep = arc_sweep(frm, to, centre, signed_r).abs();
    let side = if signed_r > 0.0 { 1.0 } else { -1.0 };
    let chord = bearing_of(frm, to);
    let turn = sweep * RAD2DEG;
    Ok(El {
        kind: 1,
        role,
        start: frm,
        end: to,
        bearing: (chord - side * turn / 2.0).rem_euclid(360.0),
        end_bearing: Some((chord + side * turn / 2.0).rem_euclid(360.0)),
        length: sweep * signed_r.abs(),
        radius: Some(signed_r),
        r1: None,
        r2: None,
        profile: Profile::Clothoid,
        speed: None,
        cant: None,
    })
}

/// What is left of a picked arc, re-shaped along its own circle to a
/// junction. `arc` takes the shorter way round; the junction may need the
/// longer one — the element would wrap more than half its circle — and then
/// its tangent there points the wrong way. `bearing_at` is the bearing the
/// chain has at the junction (`at_end`: the arc ends there, else it starts there).
fn reshaped_arc(frm: Pt, to: Pt, signed_r: f64, role: Role, bearing_at: f64, at_end: bool) -> Res<El> {
    let el = arc(frm, to, signed_r, role)?;
    let have = if at_end { el.end_bearing() } else { el.bearing };
    if turn(have, bearing_at).abs() > 1e-3 {
        return Err(SpliceError::new(format!("splice_error_{}_too_large", role.id())));
    }
    Ok(el)
}

/// A transition from `frm`; `end`/`end_bearing` snap it onto the construction's own junction.
#[allow(clippy::too_many_arguments)]
pub fn transition(frm: Pt, bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>, profile: Profile, end: Option<Pt>, end_bearing: Option<f64>) -> El {
    let (e, n, b) = transition_end(frm.0, frm.1, bearing, length, r1, r2, profile);
    El {
        kind: 2,
        role: Role::New,
        start: frm,
        end: end.unwrap_or((e, n)),
        bearing: bearing.rem_euclid(360.0),
        end_bearing: Some(end_bearing.unwrap_or(b).rem_euclid(360.0)),
        length,
        radius: None,
        r1,
        r2,
        profile,
        speed: None,
        cant: None,
    }
}

/// The element run the other way: ends and bearings swapped, curvature negated.
fn reversed_element(el: &El) -> El {
    let mut out = el.clone();
    out.start = el.end;
    out.end = el.start;
    out.bearing = (el.end_bearing() + 180.0).rem_euclid(360.0);
    out.end_bearing = Some((el.bearing + 180.0).rem_euclid(360.0));
    if el.kind == 1 {
        out.radius = el.radius.map(|r| -r);
    }
    if el.kind == 2 {
        out.r1 = el.r2.map(|r| -r);
        out.r2 = el.r1.map(|r| -r);
    }
    out
}

/// Signed bearing change from `frm` to `to`, in (−180, 180] degrees.
fn turn(frm: f64, to: f64) -> f64 {
    (to - frm + 180.0).rem_euclid(360.0) - 180.0
}

/// A transition laid from the origin heading east: (Δe, Δn, turn in degrees)
/// — where it ends, turned with any other start bearing (`placed`).
fn transition_shape(length: f64, r1: Option<f64>, r2: Option<f64>, profile: Profile) -> (f64, f64, f64) {
    if length.is_nan() || length <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let (e, n, b) = transition_end(0.0, 0.0, 90.0, length, r1, r2, profile);
    (e, n, turn(90.0, b))
}

/// Where a transition of `shape` ends that starts at `p` heading `bearing`.
fn placed(p: Pt, bearing: f64, shape: (f64, f64, f64)) -> Pt {
    let phi = (90.0 - bearing) * DEG2RAD;
    let (c, s) = (phi.cos(), phi.sin());
    (p.0 + shape.0 * c - shape.1 * s, p.1 + shape.0 * s + shape.1 * c)
}

/// Point and bearing `s` metres along an arc from `p` (negative: backwards).
fn point_on_arc(p: Pt, bearing: f64, s: f64, signed_r: f64) -> (Pt, f64) {
    arc_forward(p.0, p.1, bearing, signed_r, s / signed_r.abs())
}

/// Centre of the circle through `p` with tangent `bearing` and `signed_r`.
pub fn centre_of(p: Pt, bearing: f64, signed_r: f64) -> Pt {
    let rad = bearing * DEG2RAD;
    (p.0 + signed_r * rad.cos(), p.1 - signed_r * rad.sin())
}

/// What a construction says beside its elements.
#[derive(Debug, Clone, Default)]
pub struct Info {
    pub arc_length: Option<f64>,
    pub signed_r: Option<f64>,
    pub straight_length: Option<f64>,
    pub transition_length: Option<f64>,
    pub compound: Option<bool>,
    pub clearance: Option<Map<String, Json>>,
}

impl Info {
    fn to_json(&self) -> Json {
        let mut out = Map::new();
        for (key, value) in [
            ("arcLength", self.arc_length),
            ("signedR", self.signed_r),
            ("straightLength", self.straight_length),
            ("transitionLength", self.transition_length),
        ] {
            if let Some(v) = value {
                out.insert(key.into(), json!(v));
            }
        }
        if let Some(c) = self.compound {
            out.insert("compound".into(), json!(c));
        }
        if let Some(c) = &self.clearance {
            out.insert("clearance".into(), Json::Object(c.clone()));
        }
        Json::Object(out)
    }
}

/// A construction: its elements and what it says of them.
#[derive(Debug, Clone)]
pub struct Built {
    pub elements: Vec<El>,
    pub info: Info,
}

// ── two straights: an arc in the corner ──────────────────────────────────────

/// A pick as a construction takes it: its far end `start`, the end that meets
/// `end`, the bearing there, its signed radius.
#[derive(Debug, Clone)]
pub struct Pick {
    pub start: Pt,
    pub end: Pt,
    pub bearing: f64,
    pub radius: Option<f64>,
    /// The cant it carries, a magnitude [mm], and its design speed.
    pub cant: f64,
    pub speed: f64,
    /// How much of its track lies before and after it [m]: what joining it at
    /// the one or the other end gives up (`cost`).
    pub before: f64,
    pub after: f64,
    /// The run of its own curvature beside it, before and after [m]
    /// (trassierungCheck.runBeside): LP.EL.01 reads what stays of it with it.
    pub run_before: f64,
    pub run_after: f64,
    /// The one end it may be joined at, where it is not the element itself
    /// but a point at the end of its track (a transition, AP S.7).
    pub join_at: Option<End>,
    pub length: f64,
}

/// An end of a pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}

impl End {
    fn id(self) -> &'static str {
        match self {
            End::Start => "start",
            End::End => "end",
        }
    }
}

fn corner(dep: &Pick, arr: &Pick, radius: f64, l_dep: f64, l_arr: f64, profile: (Profile, Profile)) -> Res<(crate::geometry::CurveFit, Pt, Pt, Pt)> {
    let d1 = dir_of(dep.bearing);
    let a = dir_of(arr.bearing);
    let d2 = (-a.0, -a.1);
    if (d1.0 * d2.1 - d1.1 * d2.0).abs() < 1e-9 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let fit = fit_curve_group(dep.start, d1, arr.start, d2, radius, l_dep, l_arr, profile.0, profile.1)
        .ok_or_else(|| SpliceError::new("splice_error_clothoid_too_long"))?;
    Ok((fit, dep.start, arr.start, d2))
}

fn fits_on(fit: &crate::geometry::CurveFit) -> bool {
    fit.entry_len >= -TOL && fit.exit_len >= -TOL
}

/// The largest radius up to `upto` whose arc still fits on both elements.
fn largest_corner_radius(dep: &Pick, arr: &Pick, l_dep: f64, l_arr: f64, profile: (Profile, Profile), upto: f64) -> Option<f64> {
    let fits = |r: f64| corner(dep, arr, r, l_dep, l_arr, profile).is_ok_and(|c| fits_on(&c.0));
    let (mut lo, mut hi) = (1.0, upto);
    if !fits(lo) {
        return None;
    }
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo.floor())
}

/// An arc of `radius` in the corner of two straights, transitions optional,
/// the arrival met at its end. Where the arc does not fit on the two, the
/// answer names the largest radius that would.
pub fn splice_straights(dep: &Pick, arr: &Pick, radius: f64, l_dep: f64, l_arr: f64, profile: (Profile, Profile)) -> Res<Built> {
    if radius.is_nan() || radius <= 0.0 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let (fit, p1, p2, d2) = corner(dep, arr, radius, l_dep, l_arr, profile)?;
    if !fits_on(&fit) {
        let side = if fit.entry_len < -TOL { "dep" } else { "arr" };
        let mut e = SpliceError::new(format!("splice_error_{side}_too_large"));
        e.r_max = largest_corner_radius(dep, arr, l_dep, l_arr, profile, radius);
        return Err(e);
    }
    let sr = fit.signed_r;
    let exit_bearing = (d2.0.atan2(d2.1) * RAD2DEG).rem_euclid(360.0);
    let mut els = Vec::new();
    if fit.entry_len > 1e-6 {
        els.push(straight(p1, fit.cl_start, Role::Dep));
    }
    if l_dep > 0.0 {
        els.push(transition(
            fit.cl_start,
            dep.bearing,
            l_dep,
            None,
            Some(sr),
            profile.0,
            Some(fit.arc_start),
            Some(fit.arc_start_bearing),
        ));
    }
    els.push(arc(fit.arc_start, fit.arc_end, sr, Role::New)?);
    if l_arr > 0.0 {
        els.push(transition(
            fit.arc_end,
            fit.arc_end_bearing,
            l_arr,
            Some(sr),
            None,
            profile.1,
            Some(fit.cl_end),
            Some(exit_bearing),
        ));
    }
    if fit.exit_len > 1e-6 {
        els.push(straight(fit.cl_end, p2, Role::Arr));
    }
    Ok(Built {
        elements: els,
        info: Info {
            arc_length: Some(fit.arc_len),
            signed_r: Some(sr),
            ..Info::default()
        },
    })
}

// ── two arcs joined by a straight ────────────────────────────────────────────

pub fn splice_arcs_straight(dep: &Pick, arr: &Pick, l_dep: f64, l_arr: f64, profile: (Profile, Profile)) -> Res<Built> {
    let (r1s, r2s) = (dep.radius.unwrap(), arr.radius.unwrap());
    let (r1, r2) = (r1s.abs(), r2s.abs());
    let left_normal = |d: Pt| (-d.1, d.0);
    let (d1, d2) = (dir_of(dep.bearing), dir_of(arr.bearing));
    let sgn1 = if r1s >= 0.0 { -1.0 } else { 1.0 };
    let sgn2 = if r2s >= 0.0 { -1.0 } else { 1.0 };
    let (ln1, ln2) = (left_normal(d1), left_normal(d2));
    let o1 = (dep.end.0 + sgn1 * r1 * ln1.0, dep.end.1 + sgn1 * r1 * ln1.1);
    let o2 = (arr.end.0 + sgn2 * r2 * ln2.0, arr.end.1 + sgn2 * r2 * ln2.1);
    // Side of each centre against the straight's travel (+1 = left of it): the
    // departure arc runs forward, the arrival arc backwards.
    let sig1 = if r1s < 0.0 { 1.0 } else { -1.0 };
    let sig2 = if r2s < 0.0 { -1.0 } else { 1.0 };
    let (p1, t1, phi1) = transition_shift(l_dep, r1, profile.0);
    let (p2, t2, phi2) = transition_shift(l_arr, r2, profile.1);
    let (dd1, dd2) = (r1 + p1, r2 + p2);

    let (de, dn) = (o2.0 - o1.0, o2.1 - o1.1);
    let m = de.hypot(dn);
    if m < 1e-6 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let c = sig2 * dd2 - sig1 * dd1;
    if c.abs() > m + 1e-6 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let gamma = de.atan2(dn);
    let base = (c / m).clamp(-1.0, 1.0).asin();
    let mut best: Option<(f64, Pt, Pt, Pt, f64)> = None;
    for bm in [gamma + base, gamma + PI - base] {
        let b = (bm * RAD2DEG).rem_euclid(360.0);
        let u = dir_of(b);
        let lnu = left_normal(u);
        let f1 = (o1.0 - sig1 * dd1 * lnu.0, o1.1 - sig1 * dd1 * lnu.1);
        let f2 = (o2.0 - sig2 * dd2 * lnu.0, o2.1 - sig2 * dd2 * lnu.1);
        let fwd = (f2.0 - f1.0) * u.0 + (f2.1 - f1.1) * u.1;
        if best.is_none_or(|b| fwd > b.4) {
            best = Some((b, u, f1, f2, fwd));
        }
    }
    let Some((b, u, f1, f2, fwd)) = best else {
        return Err(SpliceError::new("splice_error_parallel"));
    };
    if fwd <= 0.0 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let s1 = (f1.0 + t1 * u.0, f1.1 + t1 * u.1);
    let s2 = (f2.0 - t2 * u.0, f2.1 - t2 * u.1);
    let straight_len = (s2.0 - s1.0) * u.0 + (s2.1 - s1.1) * u.1;
    if straight_len < -TOL {
        return Err(SpliceError::new("splice_error_clothoid_too_long"));
    }
    let u_ang = u.1.atan2(u.0);
    let a_dep = u_ang - sig1 * phi1;
    let j1 = (o1.0 - r1 * (a_dep + sig1 * PI / 2.0).cos(), o1.1 - r1 * (a_dep + sig1 * PI / 2.0).sin());
    let bearing_j1 = (90.0 - a_dep * RAD2DEG).rem_euclid(360.0);
    let a_arr = u_ang + sig2 * phi2;
    let j2 = (o2.0 - r2 * (a_arr + sig2 * PI / 2.0).cos(), o2.1 - r2 * (a_arr + sig2 * PI / 2.0).sin());

    let mut els = vec![reshaped_arc(dep.start, j1, r1s, Role::Dep, bearing_j1, true)?];
    if l_dep > 0.0 {
        els.push(transition(j1, bearing_j1, l_dep, Some(r1s), None, profile.0, Some(s1), Some(b)));
    }
    els.push(straight(s1, s2, Role::New));
    let mut bearing_j2 = b;
    if l_arr > 0.0 {
        let tr = transition(s2, b, l_arr, None, Some(-r2s), profile.1, Some(j2), None);
        bearing_j2 = tr.end_bearing();
        els.push(tr);
    }
    els.push(reshaped_arc(j2, arr.start, -r2s, Role::Arr, bearing_j2, false)?);
    Ok(Built {
        elements: els,
        info: Info {
            straight_length: Some(straight_len),
            ..Info::default()
        },
    })
}

// ── two arcs joined directly by one transition ───────────────────────────────

/// Compound or reverse curve: the transition whose end circle is the arrival
/// circle. Where the transition starts on circle 1 only turns the figure
/// about its centre, so the distance from that centre to the circle the
/// transition osculates at its end depends on the length alone; the length
/// solves d(L) = |O1 O2| on its own, and the start follows by turning the
/// figure until the two centres line up.
pub fn splice_arcs_transition(dep: &Pick, arr: &Pick, profile: Profile) -> Res<Built> {
    let r1 = dep.radius.unwrap();
    let r2 = -arr.radius.unwrap(); // the arrival arc is run backwards
    if (r1 - r2).abs() < 1e-9 {
        return Err(SpliceError::new("splice_error_arcs_no_fit"));
    }
    let o1 = centre_of(dep.end, dep.bearing, r1);
    let o2 = centre_of(arr.end, arr.bearing, arr.radius.unwrap());
    let target = (o2.0 - o1.0).hypot(o2.1 - o1.1);

    let canon = |length: f64| {
        let (e, n, b) = if length > 0.0 {
            transition_end(0.0, 0.0, 0.0, length, Some(r1), Some(r2), profile)
        } else {
            (0.0, 0.0, 0.0)
        };
        let c = centre_of((e, n), b, r2);
        ((e, n), b, c, (c.0 - r1).hypot(c.1))
    };
    let d0 = canon(0.0).3;
    let d1 = canon(1e-3).3;
    let direction = if d1 != d0 { (d1 - d0).signum() } else { 1.0 };
    if (target - d0) * direction < -1e-6 {
        return Err(SpliceError::new("splice_error_arcs_no_fit"));
    }
    let reach = |length: f64| (canon(length).3 - target) * direction;
    let mut hi = 20.0_f64.max((r1 - r2).abs());
    while reach(hi) < 0.0 && hi < MAX_TRANSITION_LENGTH {
        hi *= 2.0;
    }
    if reach(hi) < 0.0 {
        return Err(SpliceError::new("splice_error_arcs_too_far"));
    }
    let mut lo = 0.0;
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        if reach(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let length = (lo + hi) / 2.0;
    if length.is_nan() || length <= 0.0 {
        return Err(SpliceError::new("splice_error_arcs_no_fit"));
    }
    let (end, b_end, c, _) = canon(length);
    let theta = (o2.1 - o1.1).atan2(o2.0 - o1.0) - c.1.atan2(c.0 - r1);
    let (ct, st) = (theta.cos(), theta.sin());
    let place = |e: f64, n: f64| (o1.0 + (e - r1) * ct - n * st, o1.1 + (e - r1) * st + n * ct);
    let turned = |bearing: f64| (bearing - theta * RAD2DEG).rem_euclid(360.0);
    let (j1, j2) = (place(0.0, 0.0), place(end.0, end.1));
    let dep_arc = reshaped_arc(dep.start, j1, r1, Role::Dep, turned(0.0), true)?;
    let tr = transition(j1, turned(0.0), length, Some(r1), Some(r2), profile, Some(j2), Some(turned(b_end)));
    let arr_arc = reshaped_arc(j2, arr.start, r2, Role::Arr, turned(b_end), false)?;
    Ok(Built {
        elements: vec![dep_arc, tr, arr_arc],
        info: Info {
            transition_length: Some(length),
            compound: Some((r1 > 0.0) == (r2 > 0.0)),
            ..Info::default()
        },
    })
}

// ── an arc and a straight, joined by a new arc ───────────────────────────────

struct ArcBuild {
    j1: Pt,
    b1: f64,
    a1: Pt,
    a2: Pt,
    ba2: f64,
    end: Pt,
    arc_len: f64,
    residual: f64,
}

/// A new arc of `radius` between an arc and a straight, transitions optional.
///
/// Built forwards from the station `s` on the arc where the chain leaves it:
/// the straight's direction fixes the whole turn, the transitions take their
/// share, which leaves the new arc's sweep and with it the chain. One
/// condition remains — the far end has to land on the straight — and it is
/// solved for `s` over the whole arc the element can become, for both hands
/// of the new arc; the root that moves the junction least wins.
pub fn splice_arc_straight(dep: &Pick, arr: &Pick, radius: f64, l_dep: f64, l_arr: f64, profile: (Profile, Profile)) -> Res<Built> {
    if radius.is_nan() || radius <= 0.0 {
        return Err(SpliceError::new("splice_error_parallel"));
    }
    let arc_is_dep = dep.radius.is_some();
    if arc_is_dep == arr.radius.is_some() {
        return Err(SpliceError::new("splice_error_mixed"));
    }
    // Always solved with the arc leading: read backwards, the other case is this one.
    let (a, b) = if arc_is_dep { (dep, arr) } else { (arr, dep) };
    let (la, lb) = if arc_is_dep { (l_dep, l_arr) } else { (l_arr, l_dep) };
    let (fa, fb) = if arc_is_dep { profile } else { (profile.1, profile.0) };
    let ra = a.radius.unwrap();

    let exit_bearing = (b.bearing + 180.0).rem_euclid(360.0);
    let exit_dir = dir_of(exit_bearing);
    let normal = (-exit_dir.1, exit_dir.0);

    // The two transitions do not change with where they start, only turn with
    // it: worked out once per hand of the new arc.
    let shapes = |rn: f64| (transition_shape(la, Some(ra), Some(rn), fa), transition_shape(lb, Some(rn), None, fb));
    let shape_of = [shapes(radius), shapes(-radius)];
    let hand = |rn: f64| if rn == radius { 0 } else { 1 };

    let build = |s: f64, rn: f64| -> Option<ArcBuild> {
        let (j1, b1) = point_on_arc(a.end, a.bearing, s, ra);
        let (shape_in, shape_out) = shape_of[hand(rn)];
        let arc_turn = turn(b1 + shape_in.2 + shape_out.2, exit_bearing);
        if arc_turn == 0.0 || arc_turn.signum() != rn.signum() {
            return None;
        }
        let arc_len = arc_turn.abs() * DEG2RAD * radius;
        let a1 = placed(j1, b1, shape_in);
        let ba1 = b1 + shape_in.2;
        let (a2, ba2) = arc_forward(a1.0, a1.1, ba1, rn, arc_len / radius);
        let end = placed(a2, ba2, shape_out);
        let residual = (end.0 - b.end.0) * normal.0 + (end.1 - b.end.1) * normal.1;
        Some(ArcBuild {
            j1,
            b1,
            a1,
            a2,
            ba2,
            end,
            arc_len,
            residual,
        })
    };

    // How far the junction may move along the arc: back to its start, or on
    // until the element would be half a circle.
    let len_a = arc_center(a.start, a.end, ra).map_or(0.0, |c| arc_sweep(a.start, a.end, c, ra).abs() * ra.abs());
    let (s_min, s_max) = (-len_a, PI * ra.abs() - len_a);

    let f = |s: f64, rn: f64| build(s, rn).map_or(f64::NAN, |c| c.residual);

    // The closing residual at one station, as the scan reads it.
    let sampled = |x: f64, rn: f64| -> f64 {
        let side_a = if ra >= 0.0 { 1.0 } else { -1.0 };
        let side_n = if rn >= 0.0 { 1.0 } else { -1.0 };
        let (shape_in, shape_out) = shape_of[hand(rn)];
        let b0 = a.bearing * DEG2RAD;
        let cx = a.end.0 + side_a * ra.abs() * b0.cos();
        let cy = a.end.1 - side_a * ra.abs() * b0.sin();
        let (rx, ry) = (a.end.0 - cx, a.end.1 - cy);
        let ang = -side_a * x / ra.abs();
        let j1 = (cx + rx * ang.cos() - ry * ang.sin(), cy + rx * ang.sin() + ry * ang.cos());
        let b1 = (a.bearing - ang * RAD2DEG).rem_euclid(360.0);
        let arc_turn = ((exit_bearing - (b1 + shape_in.2 + shape_out.2) + 180.0).rem_euclid(360.0) + 360.0).rem_euclid(360.0) - 180.0;
        if arc_turn == 0.0 || arc_turn.signum() != side_n {
            return f64::NAN;
        }
        let sweep = arc_turn.abs() * DEG2RAD;
        let place = |p: Pt, bearing: f64, shape: (f64, f64, f64)| {
            let phi = (90.0 - bearing) * DEG2RAD;
            (p.0 + shape.0 * phi.cos() - shape.1 * phi.sin(), p.1 + shape.0 * phi.sin() + shape.1 * phi.cos())
        };
        let a1 = place(j1, b1, shape_in);
        let ba1 = (b1 + shape_in.2) * DEG2RAD;
        let (ccx, ccy) = (a1.0 + side_n * radius * ba1.cos(), a1.1 - side_n * radius * ba1.sin());
        let (qx, qy) = (a1.0 - ccx, a1.1 - ccy);
        let t = -side_n * sweep;
        let a2 = (ccx + qx * t.cos() - qy * t.sin(), ccy + qx * t.sin() + qy * t.cos());
        let end = place(a2, b1 + shape_in.2 + side_n * sweep * RAD2DEG, shape_out);
        (end.0 - b.end.0) * normal.0 + (end.1 - b.end.1) * normal.1
    };

    let mut chosen: Option<(f64, f64, ArcBuild)> = None;
    for rn in [radius, -radius] {
        let n = ARC_STRAIGHT_SAMPLES;
        let xs: Vec<f64> = (0..=n).map(|i| s_min + (s_max - s_min) * i as f64 / n as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|&x| sampled(x, rn)).collect();
        for i in 0..n {
            let (y0, y1) = (ys[i], ys[i + 1]);
            if !(y0.is_finite() && y1.is_finite()) || y0 * y1 > 0.0 {
                continue;
            }
            let (mut lo, mut hi, mut flo) = (xs[i], xs[i + 1], y0);
            for _ in 0..80 {
                let mid = (lo + hi) / 2.0;
                let fm = f(mid, rn);
                if !fm.is_finite() {
                    break;
                }
                if flo * fm <= 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                    flo = fm;
                }
            }
            let s = (lo + hi) / 2.0;
            let Some(built) = build(s, rn) else { continue };
            if built.residual.abs() > 1e-4 {
                continue;
            }
            // The re-shaped straight must not run past its far end.
            let along = (built.end.0 - b.start.0) * exit_dir.0 + (built.end.1 - b.start.1) * exit_dir.1;
            if along > TOL {
                continue;
            }
            if chosen.as_ref().is_none_or(|c| s.abs() < c.1.abs()) {
                chosen = Some((rn, s, built));
            }
        }
    }
    let Some((mut rn, _, c)) = chosen else {
        return Err(SpliceError::new("splice_error_no_fit"));
    };
    let (role_a, role_b) = if arc_is_dep { (Role::Dep, Role::Arr) } else { (Role::Arr, Role::Dep) };
    let mut els = Vec::new();
    if dist(c.j1, a.start) > 1e-6 {
        els.push(reshaped_arc(a.start, c.j1, ra, role_a, c.b1, true)?);
    }
    if la > 0.0 {
        els.push(transition(c.j1, c.b1, la, Some(ra), Some(rn), fa, Some(c.a1), None));
    }
    els.push(arc(c.a1, c.a2, rn, Role::New)?);
    if lb > 0.0 {
        els.push(transition(c.a2, c.ba2, lb, Some(rn), None, fb, Some(c.end), None));
    }
    if dist(b.start, c.end) > 1e-6 {
        els.push(straight(c.end, b.start, role_b));
    }
    if !arc_is_dep {
        // Solved read backwards: the chain, and the hand of the new arc, turned round.
        els = els.iter().rev().map(reversed_element).collect();
        rn = -rn;
    }
    Ok(Built {
        elements: els,
        info: Info {
            arc_length: Some(c.arc_len),
            signed_r: Some(rn),
            ..Info::default()
        },
    })
}

// ── the picks ────────────────────────────────────────────────────────────────

fn bad(what: &str) -> String {
    format!("invalid request: {what}")
}

fn point(raw: &Json, key: &str) -> Result<Pt, String> {
    let v = raw
        .get(key)
        .and_then(Json::as_array)
        .filter(|a| a.len() == 2)
        .ok_or_else(|| bad(&format!("pick {key}")))?;
    Ok((v[0].as_f64().ok_or_else(|| bad(key))?, v[1].as_f64().ok_or_else(|| bad(key))?))
}

fn num(raw: &Json, key: &str) -> f64 {
    raw.get(key).and_then(Json::as_f64).unwrap_or(0.0)
}

impl Pick {
    /// A pick of a request: {start, end, bearing, radius, cant, speed, length,
    /// before, after, runBefore, runAfter, joinAt}.
    pub fn from_json(raw: &Json) -> Result<Pick, String> {
        if !raw.is_object() {
            return Err(bad("pick"));
        }
        let radius = raw.get("radius").and_then(Json::as_f64).filter(|r| *r != 0.0);
        let mut p = Pick {
            start: point(raw, "start")?,
            end: point(raw, "end")?,
            bearing: raw.get("bearing").and_then(Json::as_f64).ok_or_else(|| bad("bearing"))?,
            radius,
            cant: num(raw, "cant").abs(),
            speed: num(raw, "speed").max(0.0),
            before: num(raw, "before").max(0.0),
            after: num(raw, "after").max(0.0),
            run_before: num(raw, "runBefore").max(0.0),
            run_after: num(raw, "runAfter").max(0.0),
            join_at: match raw.get("joinAt").and_then(Json::as_str) {
                Some("start") => Some(End::Start),
                Some("end") => Some(End::End),
                _ => None,
            },
            length: 0.0,
        };
        p.length = raw.get("length").and_then(Json::as_f64).filter(|l| *l != 0.0).unwrap_or_else(|| p.length_of());
        Ok(p)
    }

    /// The bearing at the pick's start, in its own direction.
    fn start_bearing(&self) -> f64 {
        let Some(r) = self.radius else { return self.bearing };
        let centre = centre_of(self.end, self.bearing, r);
        let v = (centre.0 - self.start.0, centre.1 - self.start.1);
        // The centre lies `r` to the right of the tangent: (cos b, −sin b) · r.
        ((-v.1 / r).atan2(v.0 / r) * RAD2DEG).rem_euclid(360.0)
    }

    /// The length where the request does not give it: its chord, or its arc.
    fn length_of(&self) -> f64 {
        match self.radius {
            Some(r) if dist(self.start, self.end) >= 1e-9 => {
                let sweep = ((self.bearing - self.start_bearing()) * r.signum()).rem_euclid(360.0);
                sweep * DEG2RAD * r.abs()
            }
            _ => dist(self.start, self.end),
        }
    }

    /// The pick run the other way.
    fn flipped(&self) -> Pick {
        Pick {
            start: self.end,
            end: self.start,
            bearing: (self.start_bearing() + 180.0).rem_euclid(360.0),
            radius: self.radius.map(|r| -r),
            before: self.after,
            after: self.before,
            ..self.clone()
        }
    }

    /// The pick as a construction takes it, joined at `end`.
    fn toward(&self, end: End) -> Pick {
        if end == End::End { self.clone() } else { self.flipped() }
    }

    fn track_length(&self) -> f64 {
        self.before + self.length + self.after
    }

    pub fn at(&self, end: End) -> Pt {
        match end {
            End::Start => self.start,
            End::End => self.end,
        }
    }

    /// An order of two picks that neither the clicks nor the direction a
    /// track is stored in can change: by the lower of their two ends.
    fn key(&self) -> (f64, f64) {
        let r = |p: Pt| ((p.0 * 1000.0).round() / 1000.0, (p.1 * 1000.0).round() / 1000.0);
        let (a, b) = (r(self.start), r(self.end));
        if a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater) { b } else { a }
    }
}

// ── the constructions, and which ends meet ───────────────────────────────────

/// How a transition beside a pick gets its length: as given, or the shortest
/// the rules allow at their Regelwert or down to their Ermessensgrenze
/// (Entscheidung 176).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Fixed,
    Regular,
    Minimum,
}

impl Mode {
    pub fn parse(s: Option<&str>) -> Result<Mode, String> {
        match s {
            None | Some("fixed") => Ok(Mode::Fixed),
            Some("regular") => Ok(Mode::Regular),
            Some("minimum") => Ok(Mode::Minimum),
            Some(other) => Err(bad(&format!("mode {other}"))),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Mode::Fixed => "fixed",
            Mode::Regular => "regular",
            Mode::Minimum => "minimum",
        }
    }
}

/// Two arcs: over a straight, or directly over one transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcJoin {
    Straight,
    Transition,
}

impl ArcJoin {
    fn id(self) -> &'static str {
        match self {
            ArcJoin::Straight => "straight",
            ArcJoin::Transition => "transition",
        }
    }
}

/// What a splice is asked for.
#[derive(Debug, Clone)]
pub struct Spec {
    pub radius: f64,
    /// The transitions beside each pick, in the order they were clicked (0: none).
    pub lengths: [f64; 2],
    pub modes: [Mode; 2],
    /// The shape of a single transition from arc to arc.
    pub profile: Profile,
    /// The shape of the transition beside each pick.
    pub profiles: [Profile; 2],
    pub arc_join: ArcJoin,
    /// Of what the splice inserts; the cant on a new arc.
    pub speed: f64,
    pub cant: f64,
}

/// The four pairs of ends two picks can meet at: (the first pick's, the second's).
const PAIRS: [[End; 2]; 4] = [[End::End, End::Start], [End::Start, End::End], [End::End, End::End], [End::Start, End::Start]];

fn construct(dep: &Pick, arr: &Pick, spec: &Spec, l_dep: f64, l_arr: f64, profile: (Profile, Profile)) -> Res<Built> {
    match (dep.radius, arr.radius) {
        (None, None) => splice_straights(dep, arr, spec.radius, l_dep, l_arr, profile),
        (Some(_), Some(_)) if spec.arc_join == ArcJoin::Transition => splice_arcs_transition(dep, arr, spec.profile),
        (Some(_), Some(_)) => splice_arcs_straight(dep, arr, l_dep, l_arr, profile),
        _ => splice_arc_straight(dep, arr, spec.radius, l_dep, l_arr, profile),
    }
}

/// Which pick departs where `ends` meet. An end meeting a start leaves the
/// tracks as they run. Two ends or two starts turn one track round, and that
/// is the shorter of the two (Entscheidung 182): two ends meet with the
/// longer departing, two starts with the shorter departing backwards. Equally
/// long, the second pick is the one turned.
fn departure(picks: &[Pick; 2], ends: [End; 2]) -> usize {
    if ends[0] != ends[1] {
        return if ends[0] == End::End { 0 } else { 1 };
    }
    let (l0, l1) = (picks[0].track_length(), picks[1].track_length());
    if (l0 - l1).abs() < 1e-6 {
        return if ends[0] == End::End { 0 } else { 1 };
    }
    let longer = if l0 > l1 { 0 } else { 1 };
    if ends[0] == End::End { longer } else { 1 - longer }
}

fn swap_side(code: &str) -> String {
    code.replace("_dep_", "_tmp_").replace("_arr_", "_dep_").replace("_tmp_", "_arr_")
}

/// The transition beside one pick: its mode and length, and the rules on it.
#[derive(Debug, Clone)]
pub struct Side {
    pub mode: Mode,
    pub length: f64,
    pub rules: Option<TransitionLengths>,
}

fn by_json(by: &[LengthBy]) -> Json {
    by.iter().map(|b| json!({"id": b.id, "length": b.length, "binding": b.binding})).collect()
}

impl Side {
    fn to_json(&self) -> Json {
        let mut out = Map::new();
        out.insert("mode".into(), json!(self.mode.id()));
        out.insert("length".into(), json!(self.length));
        if let Some(r) = &self.rules {
            out.insert("regular".into(), json!(r.regular));
            out.insert("minimum".into(), json!(r.minimum));
            out.insert("regularBy".into(), by_json(&r.regular_by));
            out.insert("minimumBy".into(), by_json(&r.minimum_by));
        }
        Json::Object(out)
    }
}

/// One finding of the catalogue on a solution.
#[derive(Debug, Clone)]
pub struct Found {
    pub at: String,
    pub index: Vec<usize>,
    pub id: String,
    pub severity: String,
}

/// One way the two picks can be joined.
#[derive(Debug, Clone)]
pub struct Solution {
    pub elements: Vec<El>,
    pub info: Info,
    pub dep_pick: usize,
    pub ends: [End; 2],
    pub reverse_dep: bool,
    pub reverse_arr: bool,
    pub lengths: Option<[Side; 2]>,
    pub rebuilt: f64,
    pub alternative: bool,
    pub arc_join: Option<ArcJoin>,
    pub findings: Vec<Found>,
    pub worst: Option<String>,
    pub judged: bool,
    /// What reconnecting adds (reconnect.rs).
    pub extra: Map<String, Json>,
}

impl Solution {
    pub fn to_json(&self) -> Json {
        let mut out = Map::new();
        out.insert("elements".into(), self.elements.iter().map(El::to_json).collect());
        out.insert("info".into(), self.info.to_json());
        out.insert("depPick".into(), json!(self.dep_pick));
        out.insert("ends".into(), json!([self.ends[0].id(), self.ends[1].id()]));
        out.insert("reverseDep".into(), json!(self.reverse_dep));
        out.insert("reverseArr".into(), json!(self.reverse_arr));
        if let Some(sides) = &self.lengths {
            out.insert("lengths".into(), sides.iter().map(Side::to_json).collect());
        }
        out.insert("rebuilt".into(), json!(self.rebuilt));
        out.insert("alternative".into(), json!(self.alternative));
        if let Some(j) = self.arc_join {
            out.insert("arcJoin".into(), json!(j.id()));
        }
        out.insert(
            "findings".into(),
            self.findings
                .iter()
                .map(|f| json!({"at": f.at, "index": f.index, "id": f.id, "severity": f.severity}))
                .collect(),
        );
        out.insert("worst".into(), json!(self.worst.as_deref().unwrap_or("ok")));
        out.insert("judged".into(), json!(self.judged));
        out.extend(self.extra.clone());
        Json::Object(out)
    }
}

/// The splice where `ends` meet with the transitions `lengths` beside the two
/// picks. Constructed from the pick `key` puts first, so that the clicks
/// cannot change what is found, and read the way `departure` says.
pub fn pair(picks: &[Pick; 2], ends: [End; 2], spec: &Spec, lengths: [f64; 2]) -> Res<Solution> {
    let mut c = if picks[0].key() <= picks[1].key() { 0 } else { 1 };
    if ends[c] != ends[1 - c] && ends[c] == End::Start {
        c = 1 - c; // an end meeting a start departs from the end
    }
    let d = departure(picks, ends);
    let built = construct(
        &picks[c].toward(ends[c]),
        &picks[1 - c].toward(ends[1 - c]),
        spec,
        lengths[c],
        lengths[1 - c],
        (spec.profiles[c], spec.profiles[1 - c]),
    )
    .map_err(|mut e| {
        if c != d {
            e.code = swap_side(&e.code);
        }
        e
    })?;
    let mut built = built;
    if c != d {
        // Read the other way: the chain turned round, departure and arrival swapped.
        built.elements = built
            .elements
            .iter()
            .rev()
            .map(|el| {
                let mut out = reversed_element(el);
                out.role = el.role.swapped();
                out
            })
            .collect();
        built.info.signed_r = built.info.signed_r.map(|r| -r);
    }
    let mut sol = Solution {
        elements: built.elements,
        info: built.info,
        dep_pick: d,
        ends,
        reverse_dep: ends[d] == End::Start,
        reverse_arr: ends[1 - d] == End::End,
        lengths: None,
        rebuilt: 0.0,
        alternative: false,
        arc_join: None,
        findings: Vec::new(),
        worst: None,
        judged: false,
        extra: Map::new(),
    };
    annotate(&mut sol, picks, spec);
    Ok(sol)
}

/// Each element's design speed and cant, as the app will write them: what is
/// left of a picked element keeps its own, what the splice inserts takes the
/// dialog's speed, and an inserted arc the dialog's cant — each signed with
/// its curve. A transition's cant is its neighbours'.
fn annotate(sol: &mut Solution, picks: &[Pick; 2], spec: &Spec) {
    let d = sol.dep_pick;
    for el in &mut sol.elements {
        let own = match el.role {
            Role::Dep => Some(&picks[d]),
            Role::Arr => Some(&picks[1 - d]),
            Role::New => None,
        };
        el.speed = Some(own.map_or(spec.speed, |p| p.speed));
        match el.kind {
            1 => el.cant = Some(own.map_or(spec.cant, |p| p.cant).copysign(el.radius.unwrap_or(1.0))),
            0 => el.cant = Some(0.0),
            _ => {}
        }
    }
}

/// The transitions a solution inserts, by the pick they stand beside: (pick
/// index, element index). Those before the inserted arc or straight stand
/// beside the departure, those after it beside the arrival; a single
/// transition from arc to arc stands beside neither.
fn sides(sol: &Solution) -> Vec<(usize, usize)> {
    let els = &sol.elements;
    let Some(body) = els.iter().position(|el| el.role == Role::New && el.kind != 2) else {
        return Vec::new();
    };
    let d = sol.dep_pick;
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, el) in els.iter().enumerate() {
        if el.role == Role::New && el.kind == 2 {
            let k = if i < body { d } else { 1 - d };
            out.retain(|(kk, _)| *kk != k);
            out.push((k, i));
        }
    }
    out
}

/// The shortest lengths the rules allow for each transition the solution
/// inserts, between its neighbours there.
fn side_rules(rules: &Rules, sol: &Solution, spec: &Spec) -> [Option<TransitionLengths>; 2] {
    let els = &sol.elements;
    let mut out = [None, None];
    for (k, i) in sides(sol) {
        let prev = (i > 0).then(|| els[i - 1].element());
        let next = els.get(i + 1).map(El::element);
        out[k] = transition_lengths(
            prev.as_ref(),
            next.as_ref(),
            els[i].r1,
            spec.profiles[k].id(),
            spec.speed,
            &rules.reg,
            &rules.discretion,
        )
        .ok();
    }
    out
}

/// How often the lengths are set from the chain they stand in and the chain
/// built again with them (Paket S, AP S.3).
const MAX_ROUNDS: usize = 8;

/// The splice where `ends` meet, the transitions each as long as its mode
/// asks (Entscheidung 179): 'fixed' as given, 'regular' or 'minimum' the
/// Regellänge or Mindestlänge between its neighbours in the very chain it
/// stands in. Built, the lengths read off, built again with them, until they
/// stand. Without a design speed the rules say nothing, and a side keeps the
/// length given.
pub fn solve(rules: &Rules, picks: &[Pick; 2], ends: [End; 2], spec: &Spec) -> Res<Solution> {
    let mut cur = spec.lengths;
    for _ in 0..MAX_ROUNDS {
        let mut sol = pair(picks, ends, spec, cur)?;
        let found = side_rules(rules, &sol, spec);
        let mut want = cur;
        for k in 0..2 {
            let ask = match (spec.modes[k], &found[k]) {
                (Mode::Regular, Some(r)) => r.regular,
                (Mode::Minimum, Some(r)) => r.minimum,
                _ => None,
            };
            if let Some(length) = ask
                && cur[k] > 0.0
            {
                want[k] = length;
            }
        }
        if want.iter().zip(&cur).all(|(a, b)| (a - b).abs() < 1e-9) {
            continuous(&sol.elements)?;
            let [f0, f1] = found;
            let side = |k: usize, f: Option<TransitionLengths>| match f {
                Some(r) => Side {
                    mode: spec.modes[k],
                    length: cur[k],
                    rules: Some(r),
                },
                None => Side {
                    mode: Mode::Fixed,
                    length: 0.0,
                    rules: None,
                },
            };
            sol.lengths = Some([side(0, f0), side(1, f1)]);
            return Ok(sol);
        }
        cur = want;
    }
    Err(SpliceError::new("splice_error_lengths_unstable"))
}

fn gap(picks: &[Pick; 2], ends: [End; 2]) -> f64 {
    dist(picks[0].at(ends[0]), picks[1].at(ends[1]))
}

fn allowed_pairs(picks: &[Pick; 2]) -> Vec<[End; 2]> {
    PAIRS
        .iter()
        .copied()
        .filter(|ends| (0..2).all(|i| picks[i].join_at.is_none_or(|j| j == ends[i])))
        .collect()
}

/// Between an arc and a straight: does the new arc curve the same way as the
/// picked arc — as the splice lays it without transitions, where the ends lie
/// closest? Where that does not fit either, no: the reverse curve, the
/// stricter of the two.
fn same_hand(picks: &[Pick; 2], spec: &Spec) -> bool {
    let k = if picks[0].radius.is_some() { 0 } else { 1 };
    let mut pairs = allowed_pairs(picks);
    pairs.sort_by(|a, b| gap(picks, *a).total_cmp(&gap(picks, *b)));
    for ends in pairs {
        let Ok(sol) = pair(picks, ends, spec, [0.0, 0.0]) else { continue };
        let role = if sol.dep_pick == k { Role::Dep } else { Role::Arr };
        let own = sol.elements.iter().rev().find(|el| el.role == role && el.kind == 1);
        if let (Some(own), Some(r)) = (own, sol.info.signed_r)
            && r != 0.0
        {
            return (own.radius.unwrap_or(0.0) > 0.0) == (r > 0.0);
        }
    }
    false
}

/// The transition beside each pick where no splice fits: the lengths the
/// rules ask for between the picked element and what the splice would put
/// beside it — the new arc of the radius asked for, or between two arcs the
/// straight — so that the dialog can still offer them.
fn unsolved_lengths(rules: &Rules, picks: &[Pick; 2], spec: &Spec) -> [Side; 2] {
    let two_arcs = picks.iter().all(|p| p.radius.is_some());
    let direct = two_arcs && spec.arc_join == ArcJoin::Transition;
    let same = !two_arcs && spec.radius > 0.0 && picks.iter().any(|p| p.radius.is_some()) && same_hand(picks, spec);
    let side = |k: usize| -> Side {
        let p = &picks[k];
        if direct || spec.lengths[k].is_nan() || spec.lengths[k] <= 0.0 {
            return Side {
                mode: Mode::Fixed,
                length: 0.0,
                rules: None,
            };
        }
        let plain = Side {
            mode: spec.modes[k],
            length: spec.lengths[k],
            rules: None,
        };
        if !two_arcs && spec.radius <= 0.0 {
            return plain;
        }
        let own = match p.radius {
            Some(r) => Element {
                element_type: Some(1),
                radius: Some(r.abs()),
                cant: Some(p.cant),
                speed: Some(p.speed),
                ..Element::default()
            },
            None => Element {
                element_type: Some(0),
                cant: Some(0.0),
                speed: Some(p.speed),
                ..Element::default()
            },
        };
        let body = if two_arcs {
            Element {
                element_type: Some(0),
                cant: Some(0.0),
                speed: Some(spec.speed),
                ..Element::default()
            }
        } else {
            let hand = if p.radius.is_none() || same { 1.0 } else { -1.0 };
            Element {
                element_type: Some(1),
                radius: Some(hand * spec.radius),
                cant: Some(hand * spec.cant),
                speed: Some(spec.speed),
                ..Element::default()
            }
        };
        let found = transition_lengths(
            Some(&own),
            Some(&body),
            own.radius,
            spec.profiles[k].id(),
            spec.speed,
            &rules.reg,
            &rules.discretion,
        )
        .ok();
        Side { rules: found, ..plain }
    };
    [side(0), side(1)]
}

/// How much a solution rebuilds [m]: the track it gives up beyond the ends
/// that meet, how far it re-shapes the two picked elements, and the new
/// stretch it inserts.
pub fn cost(sol: &Solution, picks: &[Pick; 2]) -> f64 {
    let d = sol.dep_pick;
    let mut cost = 0.0;
    for (i, role) in [(d, Role::Dep), (1 - d, Role::Arr)] {
        let p = &picks[i];
        cost += if sol.ends[i] == End::End { p.after } else { p.before };
        let kept: f64 = sol.elements.iter().filter(|el| el.role == role).map(|el| el.length).sum();
        cost += (kept - p.length).abs();
    }
    cost + sol.elements.iter().filter(|el| el.role == Role::New).map(|el| el.length).sum::<f64>()
}

/// How far an element turns [rad], signed (right > 0).
fn element_turn(el: &El) -> f64 {
    match el.kind {
        1 => el.length / el.radius.unwrap(),
        2 => turn(el.bearing, el.end_bearing()) * DEG2RAD,
        _ => 0.0,
    }
}

/// Does a solution turn by more than half a circle beyond what the two picked
/// elements turned already — run round in a loop to reach an end that faces
/// away? No track is joined that way; such a pair is dropped.
pub fn loops(sol: &Solution, picks: &[Pick; 2]) -> bool {
    let d = sol.dep_pick;
    let mut own = 0.0;
    for (i, flip) in [(d, sol.reverse_dep), (1 - d, sol.reverse_arr)] {
        if let Some(r) = picks[i].radius {
            own += picks[i].length / if flip { -r } else { r };
        }
    }
    (sol.elements.iter().map(element_turn).sum::<f64>() - own).abs() > PI + 1e-6
}

/// The runs beside the stretch a solution writes, [before, after] in its
/// direction (Entscheidung 363).
fn run_context(sol: &Solution, picks: &[Pick; 2]) -> [Option<Element>; 2] {
    let (dep, arr) = (&picks[sol.dep_pick], &picks[1 - sol.dep_pick]);
    let run = |p: &Pick, reversed: bool, length: f64| -> Option<Element> {
        (length > 0.0).then(|| Element {
            element_type: Some(if p.radius.is_none() { 0 } else { 1 }),
            radius: p.radius.map(|r| if reversed { -r } else { r }),
            length: Some(length),
            ..Element::default()
        })
    };
    [
        run(dep, sol.reverse_dep, if sol.reverse_dep { dep.run_after } else { dep.run_before }),
        run(arr, sol.reverse_arr, if sol.reverse_arr { arr.run_before } else { arr.run_after }),
    ]
}

/// What the catalogue finds on the whole stretch a solution writes — the
/// rest of the departure, what is inserted, the rest of the arrival and the
/// joints between them (Paket S, AP S.4) — with the speed and cant each
/// element will carry.
pub fn judge(rules: &Rules, sol: &mut Solution, picks: &[Pick; 2]) {
    let k = rules.katalog;
    let els: Vec<Element> = sol.elements.iter().map(El::element).collect();
    let [before, after] = run_context(sol, picks);
    let context = Context {
        before: before.as_ref(),
        after: after.as_ref(),
    };
    let ok = k.rank("ok");
    let found = check_track(k, &els, None, context).unwrap_or_default();
    sol.findings = found
        .into_iter()
        .filter(|f| k.rank(&f.severity) > ok)
        .map(|f| Found {
            index: f.at.split('|').filter_map(|x| x[1..].parse().ok()).collect(),
            at: f.at,
            id: f.rule,
            severity: f.severity,
        })
        .collect();
    let mut worst: Option<&Found> = None;
    for f in &sol.findings {
        if worst.is_none_or(|w| k.rank(&f.severity) > k.rank(&w.severity)) {
            worst = Some(f);
        }
    }
    sol.worst = Some(worst.map_or("ok".to_string(), |w| w.severity.clone()));
    sol.judged = sol.elements.iter().any(|el| el.speed.is_some_and(|v| v != 0.0));
}

fn rank_key(rules: &Rules, sol: &Solution) -> (i32, bool, f64) {
    (
        rules.katalog.rank(sol.worst.as_deref().unwrap_or("ok")),
        sol.alternative,
        (sol.rebuilt * 1e6).round() / 1e6,
    )
}

/// How far two elements may part or kink at a junction and still be one
/// chain [m, degrees] — the rounding of the constructions, nothing a track could show.
const JOIN_GAP: f64 = 1e-4;
const JOIN_KINK: f64 = 1e-4;

/// Is the chain one line — no gap, no kink, and every transition meeting the
/// curvature of its neighbours? A hard condition, not a rule.
fn continuous(els: &[El]) -> Res<()> {
    let k = |r: Option<f64>| r.filter(|r| *r != 0.0).map_or(0.0, |r| 1.0 / r);
    let k_end = |el: &El| if el.kind == 2 { k(el.r2) } else { k(el.radius) };
    let k_start = |el: &El| if el.kind == 2 { k(el.r1) } else { k(el.radius) };
    for w in els.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if dist(a.end, b.start) > JOIN_GAP
            || turn(a.end_bearing(), b.bearing).abs() > JOIN_KINK
            || ((a.kind == 2 || b.kind == 2) && (k_end(a) - k_start(b)).abs() > 1e-9)
        {
            return Err(SpliceError::new("splice_error_discontinuous"));
        }
    }
    Ok(())
}

/// Where the request has no transitions to offer an alternative with, two
/// arcs joined over a straight get them at the Regellänge, the search
/// starting here [m].
const ALTERNATIVE_START: f64 = 60.0;

/// The ways to try, the one asked for first: for two arcs also the other way
/// to join them (Entscheidung 180, AP S.5). The straight comes with the
/// transitions asked for, and where none were, with transitions at the
/// Regellänge — two canted arcs need their ramps; without a design speed
/// there is no Regellänge, and none (Entscheidung 183).
fn joins(picks: &[Pick; 2], spec: &Spec) -> Vec<Spec> {
    if picks[0].radius.is_none() || picks[1].radius.is_none() {
        return vec![spec.clone()];
    }
    let mut other = spec.clone();
    other.arc_join = if spec.arc_join == ArcJoin::Transition {
        ArcJoin::Straight
    } else {
        ArcJoin::Transition
    };
    if other.arc_join == ArcJoin::Straight && spec.lengths.iter().all(|l| *l == 0.0) && spec.speed > 0.0 {
        other.lengths = [ALTERNATIVE_START; 2];
        other.modes = [Mode::Regular; 2];
    }
    vec![spec.clone(), other]
}

/// What builds the solution where two ends meet — the plain construction, or one held to a spacing.
pub type Build<'a> = dyn FnMut(&[Pick; 2], [End; 2], &Spec) -> Res<Option<Solution>> + 'a;

/// The answer to a splice: every pair of ends that can be joined, the best first.
pub struct Answer {
    pub solutions: Vec<Solution>,
    /// Where only the other way of joining two arcs fits: why the way asked for does not.
    pub requested: Option<SpliceError>,
}

/// Every pair of ends that can be joined, the best first — for two arcs both
/// ways to join them — or the error of the pair whose ends lie closest, where
/// none can, with what would fit (`explain`). `build` replaces the plain
/// construction (the spacing).
pub fn solutions(rules: &Rules, picks: &[Pick; 2], spec: &Spec, build: &mut Build) -> Res<Answer> {
    let two_arcs = picks.iter().all(|p| p.radius.is_some());
    let mut found: Vec<Solution> = Vec::new();
    let mut errors: Vec<(f64, [End; 2], SpliceError)> = Vec::new();
    let pairs = allowed_pairs(picks);
    for (k, sp) in joins(picks, spec).iter().enumerate() {
        for &ends in &pairs {
            let mut sol = match build(picks, ends, sp) {
                Ok(Some(sol)) => sol,
                Ok(None) => continue,
                Err(e) => {
                    if k == 0 {
                        errors.push((gap(picks, ends), ends, e));
                    }
                    continue;
                }
            };
            if loops(&sol, picks) {
                continue;
            }
            sol.rebuilt = cost(&sol, picks);
            sol.alternative = k > 0;
            if two_arcs {
                sol.arc_join = Some(sp.arc_join);
            }
            judge(rules, &mut sol, picks);
            found.push(sol);
        }
    }
    if found.is_empty() && errors.is_empty() {
        return Err(SpliceError::new("splice_error_no_fit"));
    }
    found.sort_by(|a, b| rank_key(rules, a).partial_cmp(&rank_key(rules, b)).unwrap_or(std::cmp::Ordering::Equal));
    let mut answer = Answer {
        solutions: found,
        requested: None,
    };
    if !answer.solutions.iter().any(|s| !s.alternative) {
        // Nothing the way asked for: why, and what would fit — the error
        // itself, or beside the other way of joining that does.
        let Some(i) = (0..errors.len()).min_by(|&a, &b| errors[a].0.total_cmp(&errors[b].0)) else {
            return Ok(answer);
        };
        let (_, ends, mut e) = errors.swap_remove(i);
        explain(rules, picks, ends, spec, &mut e);
        if answer.solutions.is_empty() {
            return Err(e);
        }
        answer.requested = Some(e);
    }
    Ok(answer)
}

/// The radius search of `explain`: down from the radius asked for in steps of
/// this ratio, no further than this, then halved to the metre.
const EXPLAIN_RATIO: f64 = 0.9;
const EXPLAIN_R_MIN: f64 = 50.0;

/// What would fit where `ends` do not (AP S.5): `rMax` the largest radius up
/// to the one asked for that does and, between an arc and a straight, `rMin`
/// the smallest above it — where the case has a radius to choose — and `lMax`
/// the longest transitions, as long on both sides, that do at the radius
/// asked for — where shorter ones fit at all. Two straights name `rMax`
/// themselves.
fn explain(rules: &Rules, picks: &[Pick; 2], ends: [End; 2], spec: &Spec, e: &mut SpliceError) {
    if e.code == "splice_error_clearance" || e.code == "splice_error_discontinuous" {
        return;
    }
    let fits = |sp: &Spec, lengths: Option<[f64; 2]>| match lengths {
        None => solve(rules, picks, ends, sp).is_ok(),
        Some(l) => pair(picks, ends, sp, l).is_ok(),
    };
    let with_radius = |r: f64| Spec { radius: r, ..spec.clone() };
    let bounded = |r: f64, failed: f64| {
        let (mut lo, mut hi) = (r, failed);
        while (hi - lo).abs() > 0.5 {
            let mid = (lo + hi) / 2.0;
            if fits(&with_radius(mid), None) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo < hi { lo.floor() } else { lo.ceil() }
    };
    let free = picks[0].radius.is_none() || picks[1].radius.is_none();
    if free && spec.radius > 0.0 && e.r_max.is_none() {
        // Between an arc and a straight a radius can be too small as much as
        // too large: the nearest that fits either way.
        let (mut failed, mut r) = (spec.radius, spec.radius * EXPLAIN_RATIO);
        while r >= EXPLAIN_R_MIN && !fits(&with_radius(r), None) {
            failed = r;
            r *= EXPLAIN_RATIO;
        }
        if r >= EXPLAIN_R_MIN {
            e.r_max = Some(bounded(r, failed));
        }
        if picks.iter().any(|p| p.radius.is_some()) {
            let (mut failed, mut r) = (spec.radius, spec.radius / EXPLAIN_RATIO);
            while r <= clearance::R_MAX && !fits(&with_radius(r), None) {
                failed = r;
                r /= EXPLAIN_RATIO;
            }
            if r <= clearance::R_MAX {
                e.r_min = Some(bounded(r, failed));
            }
        }
    }
    let sides: Vec<usize> = (0..2).filter(|&k| spec.lengths[k] > 0.0).collect();
    let direct = !free && spec.arc_join == ArcJoin::Transition;
    if !sides.is_empty() && !direct {
        let at = |length: f64| [0, 1].map(|k| if sides.contains(&k) { length } else { 0.0 });
        let longest = spec.lengths[0].max(spec.lengths[1]);
        if fits(spec, Some(at(0.0))) && !fits(spec, Some(at(longest))) {
            let (mut lo, mut hi) = (0.0, longest);
            while hi - lo > 0.05 {
                let mid = (lo + hi) / 2.0;
                if fits(spec, Some(at(mid))) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            e.l_max = Some((lo * 10.0).floor() / 10.0);
        }
    }
}

/// The cant of the arc a splice inserts where no transition comes between it
/// and a pick (Entscheidung 315): it stays as that pick's — the first one's
/// where neither side has one —, a magnitude. None where a transition stands
/// beside both: the cant the app proposes for the radius then.
pub fn kept_cant(picks: &[Pick; 2], spec: &Spec) -> Option<f64> {
    (0..2)
        .find(|&k| spec.modes[k] == Mode::Fixed && (spec.lengths[k].is_nan() || spec.lengths[k] <= 0.0))
        .map(|k| picks[k].cant)
}

/// The splice held to a spacing to a neighbouring track: checked at `radius`,
/// or with `maximize` the largest radius that keeps it — where the case has a
/// radius of its own to choose (not two arcs), for every pair of ends on its
/// own, each radius with the cant the app proposes for it and the transitions
/// as long as their mode asks at that radius and cant.
fn with_clearance(rules: &Rules, picks: &[Pick; 2], spec: &Spec, cl: &Json) -> Result<Res<Answer>, String> {
    let d_min = cl.get("dMin").and_then(Json::as_f64).filter(|d| *d > 0.0).ok_or_else(|| bad("dMin"))?;
    let half: Vec<Pt> = cl
        .get("profile")
        .and_then(Json::as_array)
        .map(|a| a.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect())
        .ok_or_else(|| bad("profile"))?;
    if half.is_empty() {
        return Err(bad("profile"));
    }
    let spacing = Spacing::new(&half);
    let corners = [picks[0].start, picks[0].end, picks[1].start, picks[1].end];
    let bbox = corners
        .iter()
        .fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |b, c| {
            (b.0.min(c.0), b.1.min(c.1), b.2.max(c.0), b.3.max(c.1))
        });
    let neighbour = Neighbour::new(&Neighbour::points_of(cl.get("ref").unwrap_or(&Json::Null)), bbox);
    let speed = cl.get("speed").and_then(Json::as_f64).filter(|v| *v != 0.0).unwrap_or(spec.speed);
    let model = cl.get("cantModel").and_then(CantModel::from_json).unwrap_or_default();

    let describe = |sol: &Solution, u_new: f64| {
        let d = sol.dep_pick;
        let cant_of = |el: &El| {
            let sign = if el.radius.unwrap_or(0.0) > 0.0 { 1.0 } else { -1.0 };
            sign * match el.role {
                Role::Dep => picks[d].cant,
                Role::Arr => picks[1 - d].cant,
                Role::New => u_new,
            }
        };
        described(check(&sol.elements, &neighbour, &spacing, d_min, &cant_of), d_min)
    };

    let free = !(picks[0].radius.is_some() && picks[1].radius.is_some());
    if cl.get("maximize").and_then(Json::as_bool).unwrap_or(false) && free {
        // The arc inserted has to be an element of its own length: between an
        // arc and a straight the old arc can otherwise be run on round to the
        // straight, and the "largest radius" is a sliver of any radius at all.
        let l_min = cl.get("lMin").and_then(Json::as_f64).unwrap_or(0.0);
        let mut search = |pk: &[Pick; 2], ends: [End; 2], sp: &Spec| -> Res<Option<Solution>> {
            let mut r_max = clearance::R_MAX;
            if let Err(e) = solve(rules, pk, ends, &Spec { radius: r_max, ..sp.clone() }) {
                // Too large to fit on the elements at all: the search starts where it fits.
                if let Some(r) = e.r_max.filter(|r| *r != 0.0) {
                    r_max = r;
                }
            }
            let build = |r: f64| -> Option<Solution> {
                // The cant follows the radius as the app proposes it — where a
                // side has no transition it stays as the pick's there
                // (Entscheidung 315) —, and the transitions in their mode follow
                // both (AP S.6).
                let u = kept_cant(pk, sp).unwrap_or_else(|| auto_cant(speed, r, &model));
                let mut sol = solve(
                    rules,
                    pk,
                    ends,
                    &Spec {
                        radius: r,
                        cant: u,
                        ..sp.clone()
                    },
                )
                .ok()?;
                if sol.info.arc_length.unwrap_or(0.0) < l_min {
                    return None;
                }
                let mut info = describe(&sol, u);
                info.insert("cant".into(), json!(u));
                sol.info.clearance = Some(info);
                Some(sol)
            };
            let kept = |sol: &Solution| sol.info.clearance.as_ref().and_then(|c| c.get("kept")).and_then(Json::as_bool).unwrap_or(false);
            let Some((r, mut sol)) = largest_radius(build, kept, r_max) else {
                let mut e = SpliceError::new("splice_error_clearance");
                e.d_min = Some(d_min);
                return Err(e);
            };
            if let Some(info) = sol.info.clearance.as_mut() {
                info.insert("radius".into(), json!(r));
                info.insert("maximized".into(), json!(true));
            }
            Ok(Some(sol))
        };
        return Ok(solutions(rules, picks, spec, &mut search));
    }
    let u_new = cl.get("cant").and_then(Json::as_f64).unwrap_or(0.0).abs();
    let mut checked = |pk: &[Pick; 2], ends: [End; 2], sp: &Spec| -> Res<Option<Solution>> {
        let mut sol = solve(rules, pk, ends, sp)?;
        sol.info.clearance = Some(describe(&sol, u_new));
        Ok(Some(sol))
    };
    Ok(solutions(rules, picks, spec, &mut checked))
}

fn profile_of(raw: Option<&Json>, fallback: &str) -> Result<Profile, String> {
    match raw.and_then(Json::as_str).filter(|s| !s.is_empty()).unwrap_or(fallback) {
        "clothoid" => Ok(Profile::Clothoid),
        "bloss" => Ok(Profile::Bloss),
        other => Err(bad(&format!("transition {other}"))),
    }
}

/// The picks and the spec of a request.
pub fn parse(payload: &Json) -> Result<([Pick; 2], Spec), String> {
    if !payload.is_object() {
        return Err(bad("payload"));
    }
    let picks = [Pick::from_json(&payload["dep"])?, Pick::from_json(&payload["arr"])?];
    let transition = payload.get("transition").and_then(Json::as_str).unwrap_or("clothoid");
    let spec = Spec {
        radius: num(payload, "radius"),
        lengths: [num(payload, "lDep").max(0.0), num(payload, "lArr").max(0.0)],
        modes: [
            Mode::parse(payload.get("modeDep").and_then(Json::as_str))?,
            Mode::parse(payload.get("modeArr").and_then(Json::as_str))?,
        ],
        profile: profile_of(payload.get("transition"), "clothoid")?,
        profiles: [
            profile_of(payload.get("transitionDep"), transition)?,
            profile_of(payload.get("transitionArr"), transition)?,
        ],
        arc_join: match payload.get("arcJoin").and_then(Json::as_str) {
            Some("transition") => ArcJoin::Transition,
            _ => ArcJoin::Straight,
        },
        speed: num(payload, "speed").max(0.0),
        cant: num(payload, "cant").abs(),
    };
    Ok((picks, spec))
}

fn answer_json(answer: Answer) -> Json {
    let mut out = Map::new();
    out.insert("solutions".into(), answer.solutions.iter().map(Solution::to_json).collect());
    if let Some(e) = answer.requested {
        out.insert("requested".into(), json!({"error": e.code, "params": e.params()}));
    }
    Json::Object(out)
}

/// The answer to a splice request.
///
/// Body: {"dep": pick, "arr": pick, "radius": m, "speed": km/h, "cant": mm,
/// "lDep": m, "lArr": m, "modeDep": mode, "modeArr": mode, "transition",
/// "transitionDep", "transitionArr": "clothoid"|"bloss", "arcJoin":
/// "straight"|"transition", "clearance": optional} with a pick {"start",
/// "end", "bearing" at the end, "radius" signed or null, "cant", "speed",
/// "length", "before", "after", "runBefore", "runAfter", "joinAt"}.
///
/// Answers {"solutions": [...]} the best first, each {elements, info,
/// depPick, ends, reverseDep, reverseArr, rebuilt, lengths, findings, worst,
/// judged} — or {"error", "params", "lengths"} for a splice that does not fit
/// where any two ends meet. `clearance` {ref, dMin, profile, cant, maximize,
/// speed, cantModel, lMin} holds it to a spacing to another track.
pub fn splice_payload(rules: &Rules, payload: &Json) -> Result<Json, String> {
    let (picks, spec) = parse(payload)?;
    let cl = payload.get("clearance").filter(|c| !c.is_null());
    if cl.is_some_and(|c| !c.is_object()) {
        return Err(bad("clearance"));
    }
    let result = match cl {
        Some(cl) => with_clearance(rules, &picks, &spec, cl)?,
        None => solutions(rules, &picks, &spec, &mut |pk, ends, sp| solve(rules, pk, ends, sp).map(Some)),
    };
    Ok(match result {
        Ok(answer) => answer_json(answer),
        Err(e) => json!({
            "error": e.code,
            "params": e.params(),
            "lengths": unsolved_lengths(rules, &picks, &spec).iter().map(Side::to_json).collect::<Vec<_>>(),
        }),
    })
}
