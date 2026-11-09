// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The operations the core's face can call, by name. Each takes the input as
//! JSON and answers JSON; a module of the core that has moved to Rust adds its
//! operations here, named `<module>.<function>` as the JavaScript names it.

use rail_across_doc::{PayloadError, core_format, door, manifest, persistence, stations};
use rail_across_js::{canonical_json, get, truthy};
use serde_json::{Value, json};

/// Why an operation gave no answer: a code, what was wrong where known, and
/// what the face needs to rebuild the error (a PayloadError's code and details).
pub struct Failure {
    pub error: String,
    pub detail: Option<String>,
    pub data: Option<Value>,
}

impl Failure {
    pub fn new(error: &str, detail: impl Into<Option<String>>) -> Self {
        Failure {
            error: error.to_string(),
            detail: detail.into(),
            data: None,
        }
    }
}

impl From<PayloadError> for Failure {
    fn from(err: PayloadError) -> Self {
        Failure {
            error: "payload_error".into(),
            detail: Some(err.to_string()),
            data: Some(err.to_json()),
        }
    }
}

fn flag(input: &Value, key: &str, otherwise: bool) -> bool {
    match get(input, key) {
        None => otherwise,
        some => truthy(some),
    }
}

/// Runs operation `op` on `input`. What the input names of the project the
/// module holds (`{"$held": …}`, decision 0021) is lent to the operation for
/// the call and taken back after it (`ops_session::lend`).
pub fn run(op: &str, mut input: Value) -> Result<Value, Failure> {
    if let Some(answer) = crate::ops_session::run_owned(op, &mut input) {
        return answer;
    }
    let lent = crate::ops_session::lend(&mut input);
    let answer = run_on(op, &input);
    crate::ops_session::give_back(&mut input, lent);
    answer
}

fn run_on(op: &str, input: &Value) -> Result<Value, Failure> {
    if let Some(answer) = crate::ops_geom::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_rules::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_track::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_survey::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_session::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_commands::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_switch::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_views::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_signalling::run(op, input) {
        return answer;
    }
    if let Some(answer) = crate::ops_planpro::run(op, input) {
        return answer;
    }
    match op {
        "version" => Ok(json!({ "module": "rail-across-core", "version": env!("CARGO_PKG_VERSION") })),
        "core.seed" => {
            let n = |i: usize| input.get(i).and_then(Value::as_u64).unwrap_or(0);
            rail_across_js::seed_ids(n(0) << 32 | n(1), n(2) << 32 | n(3));
            Ok(json!(true))
        }

        // document
        // The document itself where the door changed nothing, so the face hands back the one it was given.
        "document.checkDocument" => {
            let doc = door::check_document(input)?;
            Ok(if doc == *input { json!({ "same": true }) } else { json!({ "doc": doc }) })
        }
        "document.toDocument" => Ok(door::to_document(
            &input["record"],
            flag(input, "carrySchemas", false),
            flag(input, "check", true),
        )?),
        "document.fromDocument" => Ok(door::from_document(input)?),
        "document.recordToDocument" => Ok(core_format::record_to_document(&input["record"], door::schema_hashes())?),
        "document.manifestOf" => Ok(manifest::manifest_of(input)),
        "document.documentHash" => Ok(json!(manifest::document_hash(input))),
        "document.canonicalJson" => canonical_json(input).map(Value::String).map_err(|e| Failure::new("no_json", Some(e.0))),
        "document.defaultStages" => Ok(core_format::default_stages(input.as_str().unwrap_or(""))),
        "document.stationRegister" => Ok(stations::station_register_json(input)),
        "document.renamedStation" => {
            let id = input.get("id").and_then(Value::as_str).unwrap_or("");
            let uic = input.as_object().and_then(|o| o.get("uic")).map(|u| if u.is_null() { None } else { Some(u) });
            Ok(stations::renamed_station(&input["record"], id, input.get("name").filter(|n| !n.is_null()), uic))
        }

        // project
        "project.parseProjectsPayload" => Ok(json!({ "projects": persistence::parse_projects_payload(input)? })),
        "project.projectDocument" => Ok(persistence::project_document(
            &input["record"],
            flag(input, "carrySchemas", false),
            flag(input, "check", true),
        )?),
        "project.dehydrateProjects" => {
            let projects = input.as_array().cloned().unwrap_or_default();
            Ok(Value::Array(persistence::dehydrate_projects(&projects)))
        }
        "project.parseTracksPayload" => {
            let ids: Vec<String> = input["switchIds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            Ok(Value::Array(persistence::parse_tracks_payload(&input["data"], &ids)?))
        }

        _ => Err(Failure::new("unknown_op", Some(op.to_string()))),
    }
}
