// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The one physical constant the rules compute with (`rulebookDefaults.js`,
//! packages/core/constraints/physics.json).

/// v = √(R (u + u_f) / 11.8) — see packages/core/constraints/physics.json for the derivation.
pub const CANT_DEFICIENCY_COEFF: f64 = 11.8;

/// Permissible speed [km/h] for radius [m], cant u and cant deficiency uf [mm].
pub fn permissible_speed(radius: f64, u: f64, uf: f64) -> f64 {
    (radius.abs() * (u + uf) / CANT_DEFICIENCY_COEFF).sqrt()
}
