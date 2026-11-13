// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A payload coming in, taken through the browser's own door (`checks.js`,
//! decision 91) — in the core's crates now: parsed, dehydrated, validated,
//! its signalling checked.

use std::collections::HashSet;

use rail_across_doc::persistence::{dehydrate_projects, parse_projects_payload, project_document};
use rail_across_project::validate::{new_findings, validate_project};
use serde_json::{Map, Value, json};

use crate::http::ApiError;

/// A checked record: { record, errors, warnings }.
pub struct Checked {
    pub record: Value,
    pub errors: Vec<Value>,
    pub warnings: Vec<Value>,
}

impl Checked {
    pub fn error_keys(&self) -> Vec<String> {
        self.errors.iter().map(|e| rail_across_js::js_string(e.get("key"))).collect()
    }
}

/// `checkRecord`: a payload — an infra-data-1.0 document, the one format
/// (decision 0021) — taken through the door, its record validated.
pub fn check_record(payload: &Value) -> Result<Checked, ApiError> {
    if !payload.is_object() {
        return Err(ApiError::new(422, "invalid_payload"));
    }
    let parsed = parse_projects_payload(payload).map_err(|e| ApiError::new(422, &e.code))?;
    check_kept(dehydrate_projects(&parsed).into_iter().next().unwrap_or(Value::Null))
}

/// A record the server made itself (a data migration's), validated as `check_record` validates.
pub fn check_kept(record: Value) -> Result<Checked, ApiError> {
    let v = validate_project(&record);
    let list = |k: &str| v.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let (mut errors, mut warnings) = (list("errors"), list("warnings"));
    if rail_across_js::truthy(record.get("signalling")) {
        project_document(&record, false, true).map_err(|e| ApiError::new(422, &e.code))?;
        // The signalling's check reads the domains in the words the record keeps (decision 0029, phase 1).
        let domains = rail_across_doc::persistence::record_domains(&record).map_err(|e| ApiError::new(422, &e.code))?;
        let found = rail_across_signalling::check::check_signalling_on(&domains["core"], &domains["signalling"], &domains["civil"]);
        let keyed = |f: &Value| {
            let mut o: Map<String, Value> = f.as_object().cloned().unwrap_or_default();
            o.insert("domain".into(), json!("signalling"));
            o.insert("key".into(), json!(format!("signalling:{}", rail_across_js::js_string(f.get("key")))));
            Value::Object(o)
        };
        errors.extend(found["errors"].as_array().into_iter().flatten().map(keyed));
        warnings.extend(found["warnings"].as_array().into_iter().flatten().map(keyed));
    }
    Ok(Checked { record, errors, warnings })
}

/// `normalizedRecord`: a stored document as checkRecord would hand on its record, not validated.
pub fn normalized_record(payload: &Value) -> Value {
    parse_projects_payload(payload)
        .ok()
        .and_then(|p| dehydrate_projects(&p).into_iter().next())
        .unwrap_or_else(|| payload.clone())
}

/// The errors a revision with `known` keys did not have (`freshErrors`).
pub fn fresh_errors(errors: &[Value], known: &[Value]) -> Vec<Value> {
    let prev: Vec<Value> = known.iter().map(|k| json!({ "key": k })).collect();
    new_findings(errors, &prev)
}

fn canonical(v: &Value) -> String {
    rail_across_js::canonical_json(v).unwrap_or_default()
}

/// Whether the measured axes are other than the parents have them (`axisSurveysChanged`, decision 216).
pub fn axis_surveys_changed(record: &Value, base: &Value, merge_parent: Option<&Value>) -> bool {
    let by_id = |r: &Value| -> Vec<(String, String)> {
        r.get("axisSurveys")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|s| (rail_across_js::js_string(s.get("id")), canonical(s)))
            .collect()
    };
    let lookup = |m: &[(String, String)], id: &str| m.iter().rev().find(|(k, _)| k == id).map(|(_, v)| v.clone());
    let now = by_id(record);
    let mut parents = vec![by_id(base)];
    if let Some(m) = merge_parent {
        parents.push(by_id(m));
    }
    let mut ids: HashSet<String> = now.iter().map(|(k, _)| k.clone()).collect();
    for p in &parents {
        ids.extend(p.iter().map(|(k, _)| k.clone()));
    }
    ids.iter().any(|id| !parents.iter().any(|p| lookup(p, id) == lookup(&now, id)))
}
