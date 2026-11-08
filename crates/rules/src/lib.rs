// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The rules module of the core in Rust (decision 0020, step 3): DB Ril
//! 800.0110 as a catalogue of rules in a small expression language
//! ([`expr`], [`katalog`]), applied to an element chain as the element table
//! applies it ([`pruefung`] — `trassierungCheck.js`), turned into the limits a
//! run is held to ([`grenzen`], [`rules`]), and the cant and speed rules the
//! app computes with ([`cant`], [`speed`], [`element_length`]). One check for
//! the app and the optimizer, which took these modules here from its own
//! crate; the shared vectors in packages/core/constraints/tests/ hold it to
//! the JavaScript, and the calls the JavaScript records to the rest.

// `!(a < b)` is not `a >= b` where a value may be NaN: the JavaScript this
// answers as reads a NaN as failing every comparison, and so does this crate.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod bundled;
pub mod cant;
pub mod check;
pub mod element;
pub mod element_length;
pub mod expr;
pub mod grenzen;
pub mod katalog;
pub mod length_formula;
pub mod physics;
pub mod pruefung;
pub mod rule_fields;
pub mod rules;
pub mod speed;

use std::collections::HashMap;

/// The catalogue `data` (one file of packages/core/constraints) with the
/// physics it asks for: u0_factor, for the comparison radius between two rows
/// of its table.
pub fn load_katalog(data: &serde_json::Value) -> Result<katalog::Katalog, expr::Error> {
    katalog::Katalog::new(data, HashMap::from([("u0_factor".to_string(), physics::CANT_DEFICIENCY_COEFF)]))
}
