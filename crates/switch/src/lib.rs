// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch module of the core in Rust (decision 0005): the forms
//! of DB Ril 800.0120 and what the geometry reads of them, the routes of a
//! laid switch, its symbol, where it may lie, switch connections, connecting
//! to an open end and what stays when one is deleted (`packages/core/src/switch`).

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments, clippy::manual_range_contains)]
#![recursion_limit = "256"]

pub mod cants;
pub mod catalogue;
pub mod connect;
pub mod connection;
pub mod crossing;
pub mod delete;
pub mod detail;
pub mod detail_scene;
pub mod gradient;
pub mod mid_chain;
pub mod placement;
pub mod route;
pub mod rulebook;
pub mod sleepers;
pub mod slider;
pub mod symbol;
