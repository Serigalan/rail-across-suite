// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The signalling domain in Rust (decision 0020, step 6): the catalogue of
//! signal aspects and train protection (DB Ril 301 after PlanPro 1.10), the
//! check of what the schema cannot say, the overlaps' lengths and the extent
//! of the track sections over the net (`packages/core/src/signalling`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod catalogue;
pub mod check;
pub mod overlaps;
pub mod sections;
