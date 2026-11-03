// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track optimizer of the Layout-Tool, in Rust — replacing the Python
//! service (services/optimizer) piece by piece, for the browser
//! (WebAssembly).
//!
//! This is its foundation: the rule catalogue read and applied
//! ([`expr`], [`katalog`]), judged on an element chain as the app's element
//! table judges it ([`pruefung`]), turned into the limits a run is held to
//! ([`grenzen`]), and the geometry kernel ([`geometry`]). The catalogue is not
//! compiled in — the app hands over the file it bundles. The shared vectors in
//! packages/core/constraints/tests/ hold the expression language, the check
//! and the shortest transition lengths to the app's own (tests/vectors.rs).

pub mod element;
pub mod expr;
pub mod geometry;
pub mod grenzen;
pub mod katalog;
pub mod pruefung;

use std::collections::HashMap;

/// The catalogue `data` (one file of packages/core/constraints) with the
/// kernel's physics: the one number the catalogue asks of the physics
/// document is u0_factor, for the comparison radius between two rows of its
/// table.
pub fn load_katalog(data: &serde_json::Value) -> Result<katalog::Katalog, expr::Error> {
    katalog::Katalog::new(data, HashMap::from([("u0_factor".to_string(), geometry::CANT_DEFICIENCY_COEFF)]))
}
