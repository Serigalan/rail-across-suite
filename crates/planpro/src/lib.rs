// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! PlanPro 1.10 (decision 0011): writing a project as a plan
//! (`packages/core/src/planpro`), and — as the check of what is written — a
//! small XML reader, PlanPro's enumerations and the conversion of a plan back
//! to a record of the suite.

pub mod archive;
pub mod mapping;
pub mod planpro;
pub mod write;
pub mod xml;
