// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The PlanPro export, named as the JavaScript calls it.

use rail_across_planpro::write;
use serde_json::{Value, json};

use crate::args::Args;
use crate::ops::Failure;

fn thrown(m: String) -> Failure {
    Failure {
        error: "thrown".into(),
        detail: Some(m.clone()),
        data: Some(json!({ "message": m })),
    }
}

/// Runs operation `op`, where it is one of these.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    match op {
        "planpro.writePlanPro" => Some(write::write_planpro(a.v(0), a.opt(1).unwrap_or(&Value::Null)).map_err(thrown)),
        _ => None,
    }
}
