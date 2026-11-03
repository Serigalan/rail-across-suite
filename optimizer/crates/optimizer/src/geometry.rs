// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The geometry kernel of the optimizer — services/optimizer's geometry.py,
//! ported; the reference values it was held to are the tests here.
//!
//! Conventions — the app's, so an element chain crosses unchanged:
//! * plane coordinates (easting e, northing n) in the track's native CRS
//! * compass bearings in degrees (0 = grid north, clockwise)
//! * curvature κ = −1/r; signed radius r > 0 = right-hand curve
//! * transition profiles: clothoid (linear curvature), Bloss (cubic)
//!
//! The physical constant below (v = √(R (u + u_f) / 11.8)) is derived, with
//! its formula, in packages/core/constraints/physics.json; it stays a literal
//! here and a test checks that it has not drifted from the file. The limits a
//! run is held to are not here at all: they are the rules of the catalogue
//! ([`crate::grenzen`]). What is left is the buildability grid and the
//! search's own numerics — nothing a railway administration sets.

use std::f64::consts::PI;

pub type Pt = (f64, f64);

const DEG2RAD: f64 = PI / 180.0;
const RAD2DEG: f64 = 180.0 / PI;

/// v = √(R (u + u_f) / 11.8) — see packages/core/constraints/physics.json for the derivation.
pub const CANT_DEFICIENCY_COEFF: f64 = 11.8;

// The grid a run hands its numbers out on: radii in whole metres, lengths in
// ten centimetres (the cants' five millimetres are LP.KB.03's). These are the
// steps a designer works in and the numbers that go on a drawing; a radius of
// 703.47 m is not an answer anybody builds. Existing values are never snapped
// to it — what is already built is a fact to start from, not a proposal to
// round off.
pub const R_STEP: f64 = 1.0;
pub const R_MIN: f64 = 25.0;
pub const L_STEP: f64 = 0.1;

// How finely a curve is broken into the polyline the corridor is measured on.
// The budget is the mid-ordinate of one step against its arc, not an angle: at
// a fixed angular step the error grows with the radius, which is backwards,
// since the wide curves are the ones a run moves furthest. Five millimetres is
// one percent of the corridor the panel offers at its widest.
pub const SAMPLE_SAGITTA: f64 = 0.005;
pub const MAX_SAMPLES: usize = 400;

/// A transition's curvature profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Linear curvature.
    Clothoid,
    /// Cubic curvature.
    Bloss,
}

impl Profile {
    pub fn parse(s: &str) -> Profile {
        if s == "bloss" { Profile::Bloss } else { Profile::Clothoid }
    }

    pub fn id(self) -> &'static str {
        match self {
            Profile::Clothoid => "clothoid",
            Profile::Bloss => "bloss",
        }
    }
}

/// A hair of tolerance, so that a value already on the grid stays where it is
/// instead of being pushed a whole step by the last bit of its binary
/// representation — 60.0 / 0.1 is 600.0000000000001.
fn slack(quotient: f64) -> f64 {
    1e-9 * quotient.abs().max(1.0)
}

fn round9(x: f64) -> f64 {
    (x * 1e9).round() / 1e9
}

/// Largest multiple of `step` that is not above `value`.
pub fn snap_down(value: f64, step: f64) -> f64 {
    let q = value / step;
    round9((q + slack(q)).floor() * step)
}

/// Smallest multiple of `step` that is not below `value`.
pub fn snap_up(value: f64, step: f64) -> f64 {
    let q = value / step;
    round9((q - slack(q)).ceil() * step)
}

/// Permissible speed [km/h] for radius [m], cant u and cant deficiency uf [mm].
pub fn permissible_speed(radius: f64, u: f64, uf: f64) -> f64 {
    (radius.abs() * (u + uf) / CANT_DEFICIENCY_COEFF).sqrt()
}

/// Chord length whose mid-ordinate against `radius` is SAMPLE_SAGITTA.
pub fn sample_step(radius: f64) -> f64 {
    (8.0 * SAMPLE_SAGITTA * radius.abs()).sqrt()
}

/// The radius at which a curve with this cant runs at v — the inverse of
/// [`permissible_speed`], so a target speed can be turned into the radius
/// that reaches it instead of being searched for.
pub fn radius_for_speed(v: f64, u: f64, uf: f64) -> f64 {
    if u + uf > 0.0 {
        CANT_DEFICIENCY_COEFF * v * v / (u + uf)
    } else {
        f64::INFINITY
    }
}

/// The unit vector (east, north) of a compass bearing.
pub fn dir_of(bearing_deg: f64) -> Pt {
    let b = bearing_deg * DEG2RAD;
    (b.sin(), b.cos())
}

fn kappa(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 => -1.0 / r,
        _ => 0.0,
    }
}

/// The heading polynomial of a transition as coefficients of
/// h(s) = a1 s + a2 s² + a3 s³ + a4 s⁴, so that a sampling loop can evaluate
/// it inline.
pub fn heading_coeffs(profile: Profile, k1: f64, k2: f64, length: f64) -> [f64; 4] {
    let dk = k2 - k1;
    match profile {
        Profile::Bloss => [k1, 0.0, dk / (length * length), -dk / (2.0 * length.powi(3))],
        Profile::Clothoid => [k1, dk / (2.0 * length), 0.0, 0.0],
    }
}

fn horner(a: &[f64; 4], s: f64) -> f64 {
    s * (a[0] + s * (a[1] + s * (a[2] + s * a[3])))
}

/// Heading change (math angle) at arc position s of a transition.
pub fn heading_at(profile: Profile, k1: f64, k2: f64, length: f64, s: f64) -> f64 {
    horner(&heading_coeffs(profile, k1, k2, length), s)
}

/// Gauss–Legendre nodes and weights on [0, 1].
#[allow(clippy::excessive_precision)]
const GL_NODES: [(f64, f64); 8] = [
    (0.01985507175123191, 0.05061426814518834),
    (0.10166676129318664, 0.11119051722668717),
    (0.23723379504183550, 0.15685332293894352),
    (0.40828267875217511, 0.18134189168918088),
    (0.59171732124782483, 0.18134189168918088),
    (0.76276620495816450, 0.15685332293894352),
    (0.89833323870681336, 0.11119051722668717),
    (0.98014492824876809, 0.05061426814518834),
];

/// Shift parameters (p, t, φ) of a straight-to-arc transition — eight
/// Gauss–Legendre nodes, the same values to 1e-12 m as Simpson with n = 200
/// for railway L/R.
pub fn transition_shift(length: f64, radius: f64, profile: Profile) -> (f64, f64, f64) {
    if length.is_nan() || length <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let a = heading_coeffs(profile, 0.0, 1.0 / radius, length);
    let (mut x, mut y) = (0.0, 0.0);
    for (u, w) in GL_NODES {
        let p = horner(&a, u * length);
        x += w * p.cos();
        y += w * p.sin();
    }
    x *= length;
    y *= length;
    let phi = length / (2.0 * radius);
    (y + radius * phi.cos() - radius, x - radius * phi.sin(), phi)
}

/// Steps that keep a transition inside the sagitta budget at its tight end.
pub fn transition_steps(length: f64, r1: Option<f64>, r2: Option<f64>) -> usize {
    let tight = [r1, r2].into_iter().flatten().filter(|r| *r != 0.0).map(f64::abs).fold(f64::INFINITY, f64::min);
    if !tight.is_finite() || length <= 0.0 {
        return 1; // no curvature: the chord is the curve
    }
    ((length / sample_step(tight)).ceil() as usize).clamp(4, MAX_SAMPLES)
}

/// Points along a transition (composite Simpson per step, on-curve vertices).
#[allow(clippy::too_many_arguments)]
pub fn sample_transition(e: f64, n: f64, bearing_deg: f64, length: f64, r1: Option<f64>, r2: Option<f64>, profile: Profile, steps: Option<usize>) -> Vec<Pt> {
    let steps = steps.unwrap_or_else(|| transition_steps(length, r1, r2));
    let a = heading_coeffs(profile, kappa(r1), kappa(r2), length);
    let phi0 = (90.0 - bearing_deg) * DEG2RAD;
    let mut pts = Vec::with_capacity(steps + 1);
    pts.push((e, n));
    let (mut x, mut y) = (e, n);
    let h = length / steps as f64;
    let w = h / 6.0;
    for i in 0..steps {
        let sa = i as f64 * h;
        let sm = sa + h / 2.0;
        let sb = sa + h;
        let (pa, pm, pb) = (phi0 + horner(&a, sa), phi0 + horner(&a, sm), phi0 + horner(&a, sb));
        x += w * (pa.cos() + 4.0 * pm.cos() + pb.cos());
        y += w * (pa.sin() + 4.0 * pm.sin() + pb.sin());
        pts.push((x, y));
    }
    pts
}

/// End point and end bearing of a transition — the offset integrated directly
/// by eight Gauss–Legendre nodes, to about 1e-12 m over a railway transition.
pub fn transition_end(e: f64, n: f64, bearing_deg: f64, length: f64, r1: Option<f64>, r2: Option<f64>, profile: Profile) -> (f64, f64, f64) {
    let (k1, k2) = (kappa(r1), kappa(r2));
    let a = heading_coeffs(profile, k1, k2, length);
    let phi0 = (90.0 - bearing_deg) * DEG2RAD;
    let (mut x, mut y) = (0.0, 0.0);
    for (u, w) in GL_NODES {
        let p = phi0 + horner(&a, u * length);
        x += w * p.cos();
        y += w * p.sin();
    }
    let dphi_end = (k1 + k2) * length / 2.0;
    let end_bearing = (bearing_deg - dphi_end * RAD2DEG).rem_euclid(360.0);
    (e + length * x, n + length * y, end_bearing)
}

/// Centre of the arc through two points with a signed radius.
pub fn arc_center(s: Pt, e: Pt, signed_r: f64) -> Option<Pt> {
    let (dx, dy) = (e.0 - s.0, e.1 - s.1);
    let chord = dx.hypot(dy);
    let abs_r = signed_r.abs();
    if chord < 1e-6 || abs_r < chord / 2.0 {
        return None;
    }
    let (rpx, rpy) = (dy / chord, -dx / chord);
    let h = (abs_r * abs_r - (chord / 2.0).powi(2)).sqrt();
    let sgn = if signed_r >= 0.0 { -1.0 } else { 1.0 };
    Some(((s.0 + e.0) / 2.0 - sgn * h * rpx, (s.1 + e.1) / 2.0 - sgn * h * rpy))
}

/// Signed sweep angle between the two endpoint angles (app convention).
pub fn arc_sweep(s: Pt, e: Pt, c: Pt, signed_r: f64) -> f64 {
    let two_pi = 2.0 * PI;
    let a1 = (s.1 - c.1).atan2(s.0 - c.0).rem_euclid(two_pi);
    let a2 = (e.1 - c.1).atan2(e.0 - c.0).rem_euclid(two_pi);
    if signed_r >= 0.0 {
        -(a1 - a2 + two_pi).rem_euclid(two_pi)
    } else {
        (a2 - a1 + two_pi).rem_euclid(two_pi)
    }
}

/// Points along an arc; the sagitta budget sets the step, `max_step_angle`
/// is only a ceiling, so that a very tight curve still gets a few points.
pub fn sample_arc(s: Pt, e: Pt, signed_r: f64, max_step_angle: f64) -> Vec<Pt> {
    let Some(c) = arc_center(s, e, signed_r) else {
        return vec![s, e];
    };
    let sweep = arc_sweep(s, e, c, signed_r);
    let abs_r = signed_r.abs();
    let step = max_step_angle.min(sample_step(abs_r) / abs_r);
    let n_seg = ((sweep.abs() / step).ceil() as usize).clamp(2, MAX_SAMPLES);
    let a1 = (s.1 - c.1).atan2(s.0 - c.0);
    (0..=n_seg)
        .map(|i| {
            let a = a1 + i as f64 / n_seg as f64 * sweep;
            (c.0 + abs_r * a.cos(), c.1 + abs_r * a.sin())
        })
        .collect()
}

/// Which way a curve turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    /// Two arcs turning opposite ways.
    S,
}

/// Compass tangent bearing at circle angle `angle` for the given curve side.
pub fn arc_bearing_at(angle: f64, side: Side) -> f64 {
    let rad = if side == Side::Right {
        angle.sin().atan2(-angle.cos())
    } else {
        (-angle.sin()).atan2(angle.cos())
    };
    (rad * RAD2DEG + 360.0).rem_euclid(360.0)
}

/// One transition–arc–transition between two straights.
#[derive(Debug, Clone)]
pub struct CurveFit {
    pub radius: f64,
    pub signed_r: f64,
    pub side: Side,
    pub centre: Pt,
    pub arc_start_ang: f64,
    pub sweep: f64,
    pub cl_start: Pt,
    pub cl_end: Pt,
    pub arc_start: Pt,
    pub arc_end: Pt,
    pub arc_len: f64,
    pub entry_len: f64,
    pub exit_len: f64,
    pub arc_start_bearing: f64,
    pub arc_end_bearing: f64,
}

fn cross(a: Pt, b: Pt) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn dot(a: Pt, b: Pt) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

/// Fit transition–arc–transition between two directed tangent lines: `p1`,
/// `d1` a point on and the direction of the entry line (the outer end of the
/// entry straight), `p2`, `d2` the same of the exit line. None where no fit exists.
#[allow(clippy::too_many_arguments)]
pub fn fit_curve_group(p1: Pt, d1: Pt, p2: Pt, d2: Pt, radius: f64, l1: f64, l2: f64, type1: Profile, type2: Profile) -> Option<CurveFit> {
    let det = cross(d1, d2);
    let turn = det.atan2(dot(d1, d2));
    if turn.abs() < 1e-9 {
        return None;
    }
    let side = if turn > 0.0 { Side::Left } else { Side::Right };
    let sign = if side == Side::Right { 1.0 } else { -1.0 };
    let signed_r = if side == Side::Left { -radius } else { radius };

    let (p1_shift, t1, phi1) = transition_shift(l1, radius, type1);
    let (p2_shift, t2, phi2) = transition_shift(l2, radius, type2);

    let rhs1 = sign * (radius + p1_shift) + p1.0 * d1.1 - p1.1 * d1.0;
    let rhs2 = sign * (radius + p2_shift) + p2.0 * d2.1 - p2.1 * d2.0;
    let ce = (-rhs1 * d2.0 + rhs2 * d1.0) / det;
    let cn = (d1.1 * rhs2 - d2.1 * rhs1) / det;

    let proj1 = (ce - p1.0) * d1.0 + (cn - p1.1) * d1.1;
    let virt1 = (p1.0 + proj1 * d1.0, p1.1 + proj1 * d1.1);
    let proj2 = (ce - p2.0) * d2.0 + (cn - p2.1) * d2.1;
    let virt2 = (p2.0 + proj2 * d2.0, p2.1 + proj2 * d2.1);
    let cl_start = (virt1.0 - t1 * d1.0, virt1.1 - t1 * d1.1);
    let cl_end = (virt2.0 + t2 * d2.0, virt2.1 + t2 * d2.1);

    let th1 = (virt1.1 - cn).atan2(virt1.0 - ce);
    let th2 = (virt2.1 - cn).atan2(virt2.0 - ce);
    let arc_start_ang = th1 - phi1 * sign;
    let arc_end_ang = th2 + phi2 * sign;
    let mut sweep = arc_end_ang - arc_start_ang;
    if side == Side::Left && sweep < 0.0 {
        sweep += 2.0 * PI;
    }
    if side == Side::Right && sweep > 0.0 {
        sweep -= 2.0 * PI;
    }
    if sweep.abs() < 1e-6 || sweep.abs() > PI + 1e-6 {
        return None;
    }

    Some(CurveFit {
        radius,
        signed_r,
        side,
        centre: (ce, cn),
        arc_start_ang,
        sweep,
        cl_start,
        cl_end,
        arc_start: (ce + radius * arc_start_ang.cos(), cn + radius * arc_start_ang.sin()),
        arc_end: (ce + radius * arc_end_ang.cos(), cn + radius * arc_end_ang.sin()),
        arc_len: sweep.abs() * radius,
        entry_len: (cl_start.0 - p1.0) * d1.0 + (cl_start.1 - p1.1) * d1.1,
        exit_len: (p2.0 - cl_end.0) * d2.0 + (p2.1 - cl_end.1) * d2.1,
        arc_start_bearing: arc_bearing_at(arc_start_ang, side),
        arc_end_bearing: arc_bearing_at(arc_start_ang + sweep, side),
    })
}

/// A piece of a fitted curve.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Transition {
        start: Pt,
        bearing: f64,
        length: f64,
        r1: Option<f64>,
        r2: Option<f64>,
        profile: Profile,
    },
    Arc {
        start: Pt,
        end: Pt,
        signed_r: f64,
        sweep: f64,
        bearing: f64,
        end_bearing: f64,
    },
}

impl Segment {
    fn shifted(self, by: Pt) -> Segment {
        let add = |p: Pt| (p.0 + by.0, p.1 + by.1);
        match self {
            Segment::Transition {
                start,
                bearing,
                length,
                r1,
                r2,
                profile,
            } => Segment::Transition {
                start: add(start),
                bearing,
                length,
                r1,
                r2,
                profile,
            },
            Segment::Arc {
                start,
                end,
                signed_r,
                sweep,
                bearing,
                end_bearing,
            } => Segment::Arc {
                start: add(start),
                end: add(end),
                signed_r,
                sweep,
                bearing,
                end_bearing,
            },
        }
    }

    pub fn start(&self) -> Pt {
        match self {
            Segment::Transition { start, .. } | Segment::Arc { start, .. } => *start,
        }
    }
}

/// A curve of one or more arcs between two straights.
#[derive(Debug, Clone)]
pub struct GroupFit {
    pub segments: Vec<Segment>,
    pub cl_start: Pt,
    pub cl_end: Pt,
    pub entry_len: f64,
    pub exit_len: f64,
    pub thetas: Vec<f64>,
    pub signed: Vec<f64>,
    pub side: Side,
}

/// March along an arc: end point and end bearing after |sweep| radians.
pub fn arc_forward(e: f64, n: f64, bearing: f64, signed_r: f64, sweep_mag: f64) -> (Pt, f64) {
    let side = if signed_r >= 0.0 { 1.0 } else { -1.0 }; // +1 = right-hand curve
    let d = dir_of(bearing);
    let abs_r = signed_r.abs();
    let cx = e + abs_r * if side > 0.0 { d.1 } else { -d.1 };
    let cy = n + abs_r * if side > 0.0 { -d.0 } else { d.0 };
    let ang = -side * sweep_mag; // right = clockwise in the plane
    let (ca, sa) = (ang.cos(), ang.sin());
    let (rx, ry) = (e - cx, n - cy);
    (
        (cx + rx * ca - ry * sa, cy + rx * sa + ry * ca),
        (bearing + side * sweep_mag * RAD2DEG).rem_euclid(360.0),
    )
}

/// Walk transition–arc–…–transition from `start` with bearing `b1`; the arcs
/// carry their own sign, so they may turn either way. Returns the segments,
/// the end point and the end bearing.
fn march(start: Pt, b1: f64, signed: &[f64], thetas: &[f64], trans_l: &[f64], types: &[Profile]) -> (Vec<Segment>, Pt, f64) {
    let n = signed.len();
    let (mut pos, mut brg) = (start, b1);
    let mut segments = Vec::with_capacity(2 * n + 1);
    for i in 0..=n {
        let length = trans_l[i];
        if length > 0.0 {
            let r1 = (i > 0).then(|| signed[i - 1]);
            let r2 = (i < n).then(|| signed[i]);
            segments.push(Segment::Transition {
                start: pos,
                bearing: brg,
                length,
                r1,
                r2,
                profile: types[i],
            });
            let (ee, en, eb) = transition_end(pos.0, pos.1, brg, length, r1, r2, types[i]);
            pos = (ee, en);
            brg = eb;
        }
        if i == n {
            break;
        }
        let (start, start_brg) = (pos, brg);
        (pos, brg) = arc_forward(pos.0, pos.1, brg, signed[i], thetas[i]);
        segments.push(Segment::Arc {
            start,
            end: pos,
            signed_r: signed[i],
            sweep: thetas[i],
            bearing: start_brg,
            end_bearing: brg,
        });
    }
    (segments, pos, brg)
}

/// Fit a (compound) curve of n same-side arcs between two directed tangents.
///
/// `radii` positive, all turning to the same side; `sweeps_free` the sweeps of
/// the first n − 1 arcs [rad] (the last follows from the corner's total turn);
/// `trans_l` the n + 1 transition lengths (entry, between arcs, exit; 0 =
/// none) and `types` their profiles.
///
/// Forward construction from the entry tangent; the start position along the
/// entry line is then solved linearly so the end lands on the exit line (the
/// end heading matches by construction). For n = 1 this is
/// [`fit_curve_group`]. None where no fit exists.
#[allow(clippy::too_many_arguments)]
pub fn fit_compound_group(
    p1: Pt,
    d1: Pt,
    b1: f64,
    p2: Pt,
    d2: Pt,
    _b2: f64,
    radii: &[f64],
    sweeps_free: &[f64],
    trans_l: &[f64],
    types: &[Profile],
) -> Option<GroupFit> {
    let turn = cross(d1, d2).atan2(dot(d1, d2));
    if turn.abs() < 1e-9 {
        return None;
    }
    let side = if turn > 0.0 { -1.0 } else { 1.0 }; // +1 = right-hand curves
    let n = radii.len();
    let signed: Vec<f64> = radii.iter().map(|r| side * r).collect();

    let phi: f64 = trans_l
        .iter()
        .enumerate()
        .filter(|(_, l)| **l > 0.0)
        .map(|(i, l)| {
            let k1 = if i == 0 { 0.0 } else { 1.0 / radii[i - 1] };
            let k2 = if i == n { 0.0 } else { 1.0 / radii[i] };
            l * (k1 + k2) / 2.0
        })
        .sum();

    let theta_last = turn.abs() - sweeps_free.iter().sum::<f64>() - phi;
    let mut thetas = sweeps_free.to_vec();
    thetas.push(theta_last);
    if thetas.iter().any(|t| *t <= 1e-6) || thetas.iter().sum::<f64>() + phi > PI {
        return None;
    }

    // March in local coordinates (anchor at the entry tangent point).
    let (segments, pos, _) = march((0.0, 0.0), b1, &signed, &thetas, trans_l, types);

    // Solve the start position s along the entry line: the end must lie on the exit line.
    let n2 = (-d2.1, d2.0);
    let denom = dot(d1, n2);
    if denom.abs() < 1e-12 {
        return None;
    }
    let s = ((p2.0 - p1.0 - pos.0) * n2.0 + (p2.1 - p1.1 - pos.1) * n2.1) / denom;
    let anchor = (p1.0 + s * d1.0, p1.1 + s * d1.1);
    let end = (pos.0 + anchor.0, pos.1 + anchor.1);
    Some(GroupFit {
        segments: segments.into_iter().map(|seg| seg.shifted(anchor)).collect(),
        cl_start: anchor,
        cl_end: end,
        entry_len: s,
        exit_len: (p2.0 - end.0) * d2.0 + (p2.1 - end.1) * d2.1,
        thetas,
        signed,
        side: if side > 0.0 { Side::Right } else { Side::Left },
    })
}

const S_BRACKET_STEPS: i32 = 64; // ladder rungs searched for a sign change in θ₁
const S_BISECT_STEPS: usize = 60;
const S_THETA_MIN: f64 = 1e-6; // below any sweep a real curve has
const S_THETA_MAX: f64 = 1.2; // rad; beyond this an S between two straights is none

/// Fit an S-curve — two arcs that turn opposite ways — between two tangents.
///
/// The same-side solver cannot: it closes the heading by handing the last arc
/// what is left of the corner's turn, then slides the shape along the entry
/// line until it meets the exit line — worthless between two near-parallel
/// straights, which is what an S-curve is usually for. Here the shape itself
/// closes the gap: the heading fixes one sweep against the other, which
/// leaves how far the first arc sweeps; the sideways distance to the exit
/// line grows with it, so it is bracketed and bisected. `s_entry` — where
/// along the entry line the curve begins — stays as it is.
#[allow(clippy::too_many_arguments)]
pub fn fit_s_group(p1: Pt, d1: Pt, b1: f64, p2: Pt, d2: Pt, b2: f64, signed: &[f64], trans_l: &[f64], types: &[Profile], s_entry: f64) -> Option<GroupFit> {
    if signed.len() != 2 {
        return None;
    }
    let side = [signed[0].signum_or_plus(), signed[1].signum_or_plus()];
    if side[0] == side[1] {
        return None;
    }
    let start = (p1.0 + s_entry * d1.0, p1.1 + s_entry * d1.1);

    // The turn the arcs have to contribute between them, once the transitions
    // have had their say — measured by marching with no sweep at all.
    let (_, _, flat) = march(start, b1, signed, &[0.0, 0.0], trans_l, types);
    let delta = ((b2 - flat) + 540.0).rem_euclid(360.0) - 180.0;
    let delta = delta * DEG2RAD;

    let thetas_for = |theta1: f64| [theta1, (delta - side[0] * theta1) / side[1]];
    let n2 = (-d2.1, d2.0);
    let lateral = |theta1: f64| -> Option<f64> {
        let thetas = thetas_for(theta1);
        if thetas[0].min(thetas[1]) <= 1e-7 || thetas[0] + thetas[1] > PI {
            return None;
        }
        let (_, end, _) = march(start, b1, signed, &thetas, trans_l, types);
        Some((end.0 - p2.0) * n2.0 + (end.1 - p2.1) * n2.1)
    };

    // Bracket θ₁ on a geometric ladder, then bisect: the sideways distance
    // goes with the square of the sweep near zero, where a Newton step walks
    // out of the useful range, and a railway S sweeps a few hundredths of a
    // radian, so the first rung has to sit below every reachable root.
    let mut bracket = None;
    let mut prev: Option<(f64, f64)> = None;
    for k in 0..=S_BRACKET_STEPS {
        let theta = S_THETA_MIN * (S_THETA_MAX / S_THETA_MIN).powf(f64::from(k) / f64::from(S_BRACKET_STEPS));
        let Some(value) = lateral(theta) else { break };
        if let Some((t, f)) = prev
            && (value < 0.0) != (f < 0.0)
        {
            bracket = Some((t, theta));
            break;
        }
        prev = Some((theta, value));
    }
    let (mut lo, mut hi) = bracket?;
    let mut f_lo = lateral(lo)?;
    for _ in 0..S_BISECT_STEPS {
        let mid = 0.5 * (lo + hi);
        match lateral(mid) {
            None => hi = mid,
            Some(f_mid) if (f_mid < 0.0) == (f_lo < 0.0) => {
                lo = mid;
                f_lo = f_mid;
            }
            Some(_) => hi = mid,
        }
    }
    let thetas = thetas_for(lo);
    if lateral(lo)?.abs() > 1e-6 {
        return None;
    }
    let (segments, end, _) = march(start, b1, signed, &thetas, trans_l, types);
    Some(GroupFit {
        cl_start: segments[0].start(),
        segments,
        cl_end: end,
        entry_len: s_entry,
        exit_len: (p2.0 - end.0) * d2.0 + (p2.1 - end.1) * d2.1,
        thetas: thetas.to_vec(),
        signed: signed.to_vec(),
        side: Side::S,
    })
}

trait SignumOrPlus {
    fn signum_or_plus(self) -> f64;
}

impl SignumOrPlus for f64 {
    /// +1 for zero as well — a radius ≥ 0 counts as a right-hand curve.
    fn signum_or_plus(self) -> f64 {
        if self >= 0.0 { 1.0 } else { -1.0 }
    }
}

/// Max over `points` of the distance to the polyline `poly`.
pub fn max_dist_to_polyline(points: &[Pt], poly: &[Pt]) -> f64 {
    if points.is_empty() || poly.len() < 2 {
        return 0.0;
    }
    let mut worst: f64 = 0.0;
    for &(px, py) in points {
        let mut best = f64::INFINITY;
        for w in poly.windows(2) {
            let ((ax, ay), (bx, by)) = (w[0], w[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let den = dx * dx + dy * dy;
            let t = if den > 0.0 {
                (((px - ax) * dx + (py - ay) * dy) / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (ex, ey) = (px - (ax + t * dx), py - (ay + t * dy));
            best = best.min(ex * ex + ey * ey);
        }
        worst = worst.max(best);
    }
    worst.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pt, b: Pt) -> bool {
        (a.0 - b.0).hypot(a.1 - b.1) < 1e-6
    }

    #[test]
    fn transition_shift_matches_the_app() {
        let (p, t, phi) = transition_shift(60.0, 800.0, Profile::Clothoid);
        assert!((p - 0.1874905834282572).abs() < 1e-9 && (t - 29.99859380493031).abs() < 1e-9 && phi == 0.0375);
        let (p, t, phi) = transition_shift(70.0, 800.0, Profile::Bloss);
        assert!((p - 0.15311799746666566).abs() < 1e-9 && (t - 34.99893667338733).abs() < 1e-9 && phi == 0.04375);
    }

    fn corner() -> (Pt, Pt, Pt, Pt) {
        let p1 = (500000.0, 5600000.0);
        let d1 = dir_of(40.0);
        let vertex = (p1.0 + 600.0 * d1.0, p1.1 + 600.0 * d1.1);
        let d2 = dir_of(65.0);
        (p1, d1, (vertex.0 + 600.0 * d2.0, vertex.1 + 600.0 * d2.1), d2)
    }

    #[test]
    fn fit_curve_group_matches_the_app() {
        let (p1, d1, p2, d2) = corner();
        let fit = fit_curve_group(p1, d1, p2, d2, 800.0, 60.0, 70.0, Profile::Clothoid, Profile::Bloss).unwrap();
        assert!(close(fit.cl_start, (500252.4133371916, 5600300.814501417)));
        assert!(close(fit.cl_end, (500578.2357312029, 5600549.420344572)));
        assert!(close(fit.arc_start, (500291.54964623053, 5600346.288662639)));
        assert!(close(fit.arc_end, (500515.1934940613, 5600519.0096806055)));
        assert!((fit.sweep - -0.3550823129988734).abs() < 1e-9 && fit.signed_r == 800.0);
    }

    #[test]
    fn compound_of_one_arc_is_the_single_fit() {
        let (p1, d1, p2, d2) = corner();
        let fit = fit_curve_group(p1, d1, p2, d2, 800.0, 60.0, 70.0, Profile::Clothoid, Profile::Bloss).unwrap();
        let cfit = fit_compound_group(p1, d1, 40.0, p2, d2, 65.0, &[800.0], &[], &[60.0, 70.0], &[Profile::Clothoid, Profile::Bloss]).unwrap();
        assert!(close(cfit.cl_start, fit.cl_start) && close(cfit.cl_end, fit.cl_end));
        let Some(Segment::Arc {
            start, end, sweep, signed_r, ..
        }) = cfit.segments.iter().find(|s| matches!(s, Segment::Arc { .. }))
        else {
            panic!("no arc");
        };
        assert!(close(*start, fit.arc_start) && close(*end, fit.arc_end));
        assert!((sweep - fit.sweep.abs()).abs() < 1e-9 && *signed_r == fit.signed_r);
    }

    #[test]
    fn transition_end_is_the_end_of_its_samples() {
        for profile in [Profile::Clothoid, Profile::Bloss] {
            let (e, n, _) = transition_end(10.0, 20.0, 33.0, 120.0, Some(-900.0), Some(600.0), profile);
            let pts = sample_transition(10.0, 20.0, 33.0, 120.0, Some(-900.0), Some(600.0), profile, Some(400));
            assert!(close((e, n), *pts.last().unwrap()), "{profile:?}");
        }
    }

    #[test]
    fn s_group_closes_between_parallel_straights() {
        // Two parallel straights 4 m apart, joined by R 1500 / R −1500 with
        // 40 m clothoids at both ends and a 30 m one between.
        let (p1, d1) = ((0.0, 0.0), dir_of(0.0));
        let (p2, d2) = ((4.0, 1000.0), dir_of(0.0));
        let fit = fit_s_group(
            p1,
            d1,
            0.0,
            p2,
            d2,
            0.0,
            &[1500.0, -1500.0],
            &[40.0, 30.0, 40.0],
            &[Profile::Clothoid; 3],
            100.0,
        )
        .unwrap();
        assert_eq!(fit.side, Side::S);
        let lateral = (fit.cl_end.0 - p2.0) * -d2.1 + (fit.cl_end.1 - p2.1) * d2.0;
        assert!(lateral.abs() < 1e-6, "{lateral}");
        assert!(fit.thetas.iter().all(|t| *t > 0.0));
    }

    #[test]
    fn snapping_keeps_values_on_the_grid() {
        assert_eq!(snap_down(60.0, L_STEP), 60.0);
        assert_eq!(snap_up(60.0, L_STEP), 60.0);
        assert_eq!(snap_down(703.47, R_STEP), 703.0);
        assert_eq!(snap_up(703.47, R_STEP), 704.0);
    }

    #[test]
    fn distance_to_a_polyline() {
        let poly = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)];
        assert!((max_dist_to_polyline(&[(5.0, 3.0), (12.0, 5.0)], &poly) - 3.0).abs() < 1e-12);
    }
}
