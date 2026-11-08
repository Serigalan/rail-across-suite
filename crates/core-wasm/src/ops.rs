// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The operations the core's face can call, by name. Each takes the input as
//! JSON and answers JSON; a module of the core that has moved to Rust adds its
//! operations here.

use serde_json::{Value, json};

/// Why an operation gave no answer: a code, and what was wrong where known.
pub struct Failure {
    pub error: String,
    pub detail: Option<String>,
}

impl Failure {
    pub fn new(error: &str, detail: impl Into<Option<String>>) -> Self {
        Failure {
            error: error.to_string(),
            detail: detail.into(),
        }
    }
}

/// Runs operation `op` on `input`.
pub fn run(op: &str, _input: Value) -> Result<Value, Failure> {
    match op {
        "version" => Ok(json!({ "module": "rail-across-core", "version": env!("CARGO_PKG_VERSION") })),
        _ => Err(Failure::new("unknown_op", Some(op.to_string()))),
    }
}
