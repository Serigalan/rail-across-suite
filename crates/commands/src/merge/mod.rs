// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Comparing and merging two states of a project (`merge/`): the three-way
//! merge, carrying references across splits and joins, the domains' locations.

pub mod carry;
pub mod collections;
pub mod diff;
pub mod remap;
pub mod three_way;
