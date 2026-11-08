// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The commands of the core in Rust (decision 0020, step 5): what the
//! dialogs commit, as pure functions of their inputs (`commands/`), and the
//! three-way merge of project records (`merge/`).

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments, clippy::manual_range_contains)]
#![recursion_limit = "256"]

pub mod edit;
pub mod roles;
pub mod table;
pub mod tracks;
