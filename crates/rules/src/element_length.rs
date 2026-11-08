// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The shortest element and the comparison radius at a joint (`elementLength.js`).

use crate::bundled::catalog_limit;

/// l_min of LP.EL.01 at design speed `v` [m] (`minElementLength`).
pub fn min_element_length(v: f64) -> Option<f64> {
    if !(v > 0.0) {
        return None;
    }
    catalog_limit(
        "LP.EL.01",
        "l_min",
        &[("element.design_speed", v.into()), ("element.length", 0.0.into())],
        false,
    )
}

/// The comparison radius LP.KS.01 asks for at design speed `v` [m]: (reg, discretion) (`comparisonRadiusLimits`).
pub fn comparison_radius_limits(v: f64) -> Option<(f64, f64)> {
    if !(v > 0.0) {
        return None;
    }
    let scope = [
        ("physics.curvature_jump", true.into()),
        ("prev.design_speed", v.into()),
        ("next.design_speed", v.into()),
        ("physics.r_w", 0.0.into()),
    ];
    Some((
        catalog_limit("LP.KS.01", "reg", &scope, false)?,
        catalog_limit("LP.KS.01", "discretion", &scope, false)?,
    ))
}
