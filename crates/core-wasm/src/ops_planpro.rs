// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The operations of the PlanPro import, named as the JavaScript's.

use rail_across_planpro::{planpro, xml};
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
    let text = || a.opt(0).and_then(Value::as_str).unwrap_or("");
    match op {
        "planpro.parseXml" => Some(xml::parse_xml(text()).map(|el| el.to_json()).map_err(thrown)),
        "planpro.readPlanPro" => Some(
            xml::parse_xml(text())
                .and_then(|root| planpro::read_planpro(&root).map(|m| m.to_json()))
                .map_err(thrown),
        ),
        "planpro.planproRecord" => {
            let id = a.option(1, "id").and_then(Value::as_str);
            let title = a.option(1, "title").and_then(Value::as_str);
            Some(planpro::planpro_record(text(), id, title).map_err(thrown))
        }
        "planpro.placeSignalling" => Some(Ok(planpro::place_signalling(a.v(0), &a.list(1), a.v(2)))),
        _ => None,
    }
}
