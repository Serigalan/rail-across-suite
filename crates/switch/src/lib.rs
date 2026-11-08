// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch module of the core in Rust (decision 0020, step 4): the forms
//! of DB Ril 800.0120 and what the geometry reads of them, the routes of a
//! laid switch, its symbol, where it may lie, switch connections, connecting
//! to an open end and what stays when one is deleted (`packages/core/src/switch`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod catalogue;
pub mod regelwerk;
pub mod route;
