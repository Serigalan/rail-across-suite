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
