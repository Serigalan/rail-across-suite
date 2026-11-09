// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Which form field a rule's finding belongs to (`ruleFields.js`, R10.4).

use serde_json::{Value as Json, json};

use crate::bundled::{rule_by_id, severity_rank, worst_severity};
use crate::catalogue::Source;

fn field_of(from: &str) -> Option<&'static str> {
    match from {
        "element.design_speed" | "physics.u_f" | "physics.delta_u_f" => Some("speed"),
        "element.radius" => Some("radius"),
        "element.cant" | "physics.delta_u" => Some("cant"),
        "element.length" => Some("length"),
        _ => None,
    }
}

/// The fields of a rule's inputs (`ruleFields`).
pub fn rule_fields(id: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if let Some(rule) = rule_by_id(id) {
        for input in &rule.inputs {
            if let Source::From(_, from) = &input.source
                && let Some(f) = field_of(from)
                && !out.contains(&f)
            {
                out.push(f);
            }
        }
    }
    out
}

/// The findings of rule results per field, past a hint (`fieldFindings`).
pub fn field_findings(results: &[Json]) -> Json {
    let mut out = serde_json::Map::new();
    for result in results {
        let severity = result["severity"].as_str();
        if severity_rank(severity) <= severity_rank(Some("hint")) {
            continue;
        }
        let id = result["id"].as_str().unwrap_or("");
        for field in rule_fields(id) {
            let seen = out.get(field).cloned();
            let seen_severity = seen.as_ref().and_then(|s| s["severity"].as_str().map(str::to_string));
            let worst = worst_severity([seen_severity.as_deref(), severity]).map(str::to_string);
            let mut ids: Vec<Json> = seen.as_ref().and_then(|s| s["ids"].as_array().cloned()).unwrap_or_default();
            if !ids.iter().any(|x| x.as_str() == Some(id)) {
                ids.push(json!(id));
            }
            out.insert(field.to_string(), json!({ "severity": worst, "ids": ids }));
        }
    }
    Json::Object(out)
}
