// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The repository's constraint files, read where the app reads them.

#![allow(dead_code)]

use rail_across_optimizer::katalog::Katalog;
use rail_across_optimizer::load_katalog;
use serde_json::Value;
use std::path::PathBuf;

pub fn constraints(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/core/constraints").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

pub fn katalog() -> Katalog {
    load_katalog(&constraints("db-ril-800-0110.json")).unwrap()
}
