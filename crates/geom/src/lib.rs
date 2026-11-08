// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The geometry module of the core in Rust (decision 0020, step 2): the
//! projected plane and the element — straights, arcs and transitions
//! (clothoid, Bloss) —, their cants, the coordinate systems and the one
//! tolerance table. Elements are the JSON objects the record holds; each
//! function answers as its namesake in `packages/core/src/geometry` does.

pub mod cant;
pub mod element;
pub mod tolerances;
pub mod transition;
