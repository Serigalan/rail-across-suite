// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The survey module of the core in Rust (decision 0020, step 4): axes
//! measured in point clouds, reference axes read from a survey's alignment,
//! the shift values between a design and the existing axis, and the alignment
//! fit's bookkeeping (`packages/core/src/survey`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod axis;
pub mod fit;
pub mod reference;
pub mod shift;
