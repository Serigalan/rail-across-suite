// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The geometry module of the core in Rust (decision 0005): the
//! projected plane and the element — straights, arcs and transitions
//! (clothoid, Bloss) —, their cants, the coordinate systems with the proj4
//! port they convert on, parallels and plane changes, the chain invariants and
//! the one tolerance table. Elements are the JSON objects the record holds;
//! each function answers as its namesake in `packages/core/src/geometry` does.

// `!(a < b)` is not `a >= b` where a value may be NaN: the JavaScript this
// answers as reads a NaN as failing every comparison, and so does this crate.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod cant;
pub mod chain;
pub mod coords;
pub mod element;
pub mod factory;
pub mod kind;
pub mod km_line;
pub mod parallel;
pub mod plane_transform;
pub mod precision;
pub mod proj;
pub mod reconstruct;
pub mod tolerances;
pub mod transition;
