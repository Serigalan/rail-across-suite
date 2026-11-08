// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reading a PlanPro 1.10 plan in Rust (decision 0020, step 6; decision 0011):
//! a small XML reader, PlanPro's enumerations and the conversion of a plan to
//! a record of the suite (`packages/core/src/planpro`).

pub mod mapping;
pub mod planpro;
pub mod xml;
