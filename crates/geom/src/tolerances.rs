// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The one tolerance table (`tolerances.js`): what "the same place" means.

/// Decimals positions, lengths, radii and stations are kept to: 0.1 µm.
pub const POSITION_DIGITS: i32 = 7;

/// Two points or two stations closer than this are the same place [m]: 1 µm.
pub const NODE_TOL: f64 = 1e-6;
