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
