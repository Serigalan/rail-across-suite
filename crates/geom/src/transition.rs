// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Transitions (`clothoidUtils.js`): the heading change along a clothoid or a
//! Bloss curve, and the curvature a signed radius stands for.

/// The curvature a signed radius stands for: κ = −1/R (R > 0 a right-hand
/// curve, clockwise), 0 for a straight (none or 0).
pub fn to_kappa(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 => -1.0 / r,
        _ => 0.0,
    }
}

/// Whether `kind` names a Bloss curve; anything else is a clothoid.
pub fn is_bloss(kind: Option<&str>) -> bool {
    kind == Some("bloss")
}

/// Heading change Δφ(s) of a transition of `length` between curvatures κ1 → κ2:
/// clothoid κ linear in s, Bloss κ1 + (κ2−κ1)·(3(s/L)² − 2(s/L)³).
pub fn transition_heading(bloss: bool, kappa1: f64, kappa2: f64, length: f64, s: f64) -> f64 {
    let dk = kappa2 - kappa1;
    if bloss {
        kappa1 * s + dk * (s.powi(3) / (length * length) - s.powi(4) / (2.0 * length.powi(3)))
    } else {
        kappa1 * s + dk * s * s / (2.0 * length)
    }
}

/// The bearing [°] at `s` along a transition that leaves on `bearing`.
pub fn transition_bearing_at(bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>, bloss: bool, s: f64) -> f64 {
    let dphi = transition_heading(bloss, to_kappa(r1), to_kappa(r2), length, s);
    js_mod360(bearing - dphi * (180.0 / std::f64::consts::PI))
}

/// `((x % 360) + 360) % 360` with JavaScript's `%` (the sign of the dividend).
pub fn js_mod360(x: f64) -> f64 {
    ((x % 360.0) + 360.0) % 360.0
}

use crate::coords::{Unsupported, plane_coords_to_wgs84};
use crate::precision::SAGITTA_ELEMENT;

/// Sub-intervals of the composite Simpson rule per drawn segment.
pub const SUBDIV: usize = 8;

fn clothoid_steps(kappa1: f64, kappa2: f64, length: f64, max_deviation: f64) -> usize {
    let radii: Vec<f64> = [kappa1, kappa2].iter().filter(|k| k.abs() > 1e-12).map(|k| 1.0 / k.abs()).collect();
    if radii.is_empty() {
        return 2;
    }
    let min_r = radii.iter().copied().fold(f64::INFINITY, f64::min);
    let total_angle = ((kappa1 + kappa2) * length / 2.0).abs();
    if total_angle < 1e-9 {
        return 2;
    }
    let max_angle = 2.0 * (1.0 - max_deviation / min_r).clamp(-1.0, 1.0).acos();
    if max_angle < 1e-12 {
        return 500;
    }
    ((total_angle / max_angle).ceil() as usize).clamp(2, 500)
}

/// March a transition in the track's plane (`sampleTransitionUtm`): `steps`
/// points after the start, each reached by `subdiv` Simpson sub-intervals,
/// up to `end` along the curve.
#[allow(clippy::too_many_arguments)]
pub fn sample_transition(
    e: f64,
    n: f64,
    bearing: f64,
    length: f64,
    r1: Option<f64>,
    r2: Option<f64>,
    bloss: bool,
    steps: usize,
    subdiv: usize,
    end: f64,
) -> Vec<[f64; 2]> {
    let kappa1 = to_kappa(r1);
    let kappa2 = to_kappa(r2);
    let phi0 = (90.0 - bearing) * (std::f64::consts::PI / 180.0);
    let ds = end / steps as f64;
    let (mut x, mut y) = (e, n);
    let mut pts = vec![[x, y]];
    for i in 0..steps {
        let s0 = end * i as f64 / steps as f64;
        let h = ds / subdiv as f64;
        for j in 0..subdiv {
            let a = s0 + j as f64 * h;
            let pa = phi0 + transition_heading(bloss, kappa1, kappa2, length, a);
            let pm = phi0 + transition_heading(bloss, kappa1, kappa2, length, a + h / 2.0);
            let pb = phi0 + transition_heading(bloss, kappa1, kappa2, length, a + h);
            x += h / 6.0 * (pa.cos() + 4.0 * pm.cos() + pb.cos());
            y += h / 6.0 * (pa.sin() + 4.0 * pm.sin() + pb.sin());
        }
        pts.push([x, y]);
    }
    pts
}

/// Plane points of a transition, as many as the sagitta asks and at least `min_steps` (`transitionPlanePoints`).
#[allow(clippy::too_many_arguments)]
pub fn transition_plane_points(
    e: f64,
    n: f64,
    bearing: f64,
    length: f64,
    r1: Option<f64>,
    r2: Option<f64>,
    bloss: bool,
    max_deviation: f64,
    min_steps: usize,
) -> Vec<[f64; 2]> {
    let steps = min_steps.max(clothoid_steps(to_kappa(r1), to_kappa(r2), length, max_deviation));
    sample_transition(e, n, bearing, length, r1, r2, bloss, steps, SUBDIV, length)
}

/// A transition drawn from its start (`computeClothoidUtm`): WGS 84 coords, end point, end bearing.
pub struct Clothoid {
    pub coords: Vec<[f64; 2]>,
    pub end: [f64; 2],
    pub end_bearing: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn compute_clothoid(
    e: f64,
    n: f64,
    zone: i64,
    bearing: f64,
    length: f64,
    r1: Option<f64>,
    r2: Option<f64>,
    max_deviation: f64,
    bloss: bool,
) -> Result<Clothoid, Unsupported> {
    let kappa1 = to_kappa(r1);
    let kappa2 = to_kappa(r2);
    let pts = transition_plane_points(e, n, bearing, length, r1, r2, bloss, max_deviation, 2);
    let coords = plane_coords_to_wgs84(&pts, zone)?;
    let end = pts[pts.len() - 1];
    let phi0 = (90.0 - bearing) * (std::f64::consts::PI / 180.0);
    let dphi_end = (kappa1 + kappa2) * length / 2.0;
    let end_bearing = ((90.0 - (phi0 + dphi_end) * (180.0 / std::f64::consts::PI)) % 360.0 + 360.0) % 360.0;
    Ok(Clothoid { coords, end, end_bearing })
}

/// The default sagitta of `computeClothoidUtm`.
pub const DEFAULT_SAGITTA: f64 = SAGITTA_ELEMENT;

/// Point of a transition at station `s` (`transitionPointAtUtm`).
#[allow(clippy::too_many_arguments)]
pub fn transition_point_at(e: f64, n: f64, bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>, bloss: bool, s: f64) -> [f64; 2] {
    let end = s.max(0.0).min(length);
    let steps = 8usize.max((end / length * 512.0).ceil() as usize);
    let pts = sample_transition(e, n, bearing, length, r1, r2, bloss, 1, steps, end);
    pts[pts.len() - 1]
}

/// Shift parameters of a straight ↔ arc transition of length `l` into radius `r` (`transitionShift`): (p, t, phi).
pub fn transition_shift(l: f64, r: f64, bloss: bool) -> (f64, f64, f64) {
    if l.is_nan() || l <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let pts = sample_transition(0.0, 0.0, 90.0, l, None, Some(-r), bloss, 1, 1000, l);
    let [x, y] = pts[pts.len() - 1];
    let phi = l / (2.0 * r);
    (y + r * phi.cos() - r, x - r * phi.sin(), phi)
}

const CURVATURE_ZERO: f64 = 1e-12;

/// Curvature 1/r of a signed radius, in its own sign; 0 for a straight (`curvatureOf`).
pub fn curvature_of(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 && !r.is_nan() => 1.0 / r,
        _ => 0.0,
    }
}

/// Signed radius of a curvature; None where it is none (`radiusOfCurvature`).
pub fn radius_of_curvature(k: f64) -> Option<f64> {
    if k.abs() < CURVATURE_ZERO { None } else { Some(1.0 / k) }
}

/// Signed radius of a clothoid at station `s` (`clothoidRadiusAt`).
pub fn clothoid_radius_at(r1: Option<f64>, r2: Option<f64>, length: f64, s: f64) -> Option<f64> {
    if s <= 0.0 {
        return r1;
    }
    if s >= length {
        return r2;
    }
    let k1 = curvature_of(r1);
    radius_of_curvature(k1 + (curvature_of(r2) - k1) * s / length)
}

/// Offset of a point from a transition (`projectOnTransitionUtm`): (along, perp).
#[allow(clippy::too_many_arguments)]
pub fn project_on_transition(se: f64, sn: f64, px: f64, py: f64, bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>, bloss: bool) -> (f64, f64) {
    if length.is_nan() || length <= 0.0 {
        return (0.0, 0.0);
    }
    let steps = 16usize.max(length.ceil() as usize);
    let pts = sample_transition(se, sn, bearing, length, r1, r2, bloss, steps, 4, length);
    let mut best_d2 = f64::INFINITY;
    let mut best_s = 0.0;
    for i in 0..steps {
        let [ax, ay] = pts[i];
        let [bx, by] = pts[i + 1];
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let u = if l2 > 0.0 {
            (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ex = ax + u * dx - px;
        let ey = ay + u * dy - py;
        let d2 = ex * ex + ey * ey;
        if d2 < best_d2 {
            best_d2 = d2;
            best_s = length * (i as f64 + u) / steps as f64;
        }
    }
    let deg = std::f64::consts::PI / 180.0;
    let mut s = best_s;
    for _ in 0..6 {
        let foot = transition_point_at(se, sn, bearing, length, r1, r2, bloss, s);
        let b = transition_bearing_at(bearing, length, r1, r2, bloss, s) * deg;
        let ds = (px - foot[0]) * b.sin() + (py - foot[1]) * b.cos();
        let next = (s + ds).max(0.0).min(length);
        let done = (next - s).abs() < 1e-10;
        s = next;
        if done {
            break;
        }
    }
    let foot = transition_point_at(se, sn, bearing, length, r1, r2, bloss, s);
    let b = transition_bearing_at(bearing, length, r1, r2, bloss, s) * deg;
    let (de, dn) = (px - foot[0], py - foot[1]);
    (s, de * b.cos() - dn * b.sin())
}
