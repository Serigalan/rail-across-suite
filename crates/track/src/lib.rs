// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track module of the core in Rust (decision 0005): the records
//! a project is made of — tracks with their stationing and heights, switch
//! records and their ports, links, track ends and buffer stops, platforms,
//! routes, groups, splitting and joining. Each function answers as its
//! namesake in `packages/core/src/track` does.

// `!(a < b)` is not `a >= b` where a value may be NaN: the JavaScript this
// answers as reads a NaN as failing every comparison, and so does this crate.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod bands;
pub mod cross_section;
pub mod diagram;
pub mod end_marks;
pub mod groups;
pub mod heights;
pub mod ids;
pub mod merge;
pub mod model;
pub mod platforms;
pub mod routes;
pub mod split;
pub mod status;
pub mod switch_model;
pub mod topology;
pub mod util;
