// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A payload coming in, taken through the browser's own door (`checks.js`,
//! decision 91) — in the core's crates now: parsed, dehydrated, validated,
//! its signalling checked.

use std::collections::HashSet;

use rail_across_doc::persistence::{dehydrate_projects, parse_projects_payload, project_document, projects_payload};
use rail_across_project::validate::{new_findings, validate_project};
use serde_json::{Map, Value, json};

use crate::http::ApiError;

fn as_payload(payload: &Value, id: Option<&str>) -> Value {
    if rail_across_js::truthy(payload.get("format")) {
        return payload.clone();
    }
    let mut record = payload.as_object().cloned().unwrap_or_default();
    if payload.get("id").is_none_or(Value::is_null) {
        if let Some(id) = id {
            record.insert("id".into(), json!(id));
        }
    }
    projects_payload(vec![Value::Object(record)])
}

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

/// `checkRecord`.
pub fn check_record(payload: &Value, id: Option<&str>) -> Result<Checked, ApiError> {
    if !payload.is_object() {
        return Err(ApiError::new(422, "invalid_payload"));
    }
    let parsed = parse_projects_payload(&as_payload(payload, id)).map_err(|e| ApiError::new(422, &e.code))?;
    let record = dehydrate_projects(&parsed).into_iter().next().unwrap_or(Value::Null);
    let v = validate_project(&record);
    let list = |k: &str| v.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let (mut errors, mut warnings) = (list("errors"), list("warnings"));
    if rail_across_js::truthy(record.get("signalling")) {
        let doc = project_document(&record, false, true).map_err(|e| ApiError::new(422, &e.code))?;
        let found = rail_across_signalling::check::check_signalling(&doc["domains"]["core"], &doc["domains"]["signalling"]);
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

/// `normalizedRecord`: a stored payload as checkRecord would hand it on, not validated.
pub fn normalized_record(payload: &Value) -> Value {
    parse_projects_payload(&as_payload(payload, None))
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
