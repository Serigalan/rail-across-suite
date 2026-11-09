// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What changed from one state to another (`diff.js`).

use rail_across_doc::persistence::dehydrate_projects;
use serde_json::{Map, Value, json};

use super::collections::{collection_names, fields_of, index_collection, object_label, project_fields, same_value};

/// A record as merged: dehydrated (`primary`).
pub fn primary(project: &Value) -> Value {
    let p = if project.is_null() { json!({}) } else { project.clone() };
    dehydrate_projects(&[p]).remove(0)
}

fn changed_fields(name: Option<&str>, a: &Value, b: &Value) -> Vec<String> {
    let (fa, fb): (Map<String, Value>, Map<String, Value>) = match name {
        Some(n) => (fields_of(n, a), fields_of(n, b)),
        None => (project_fields(a), project_fields(b)),
    };
    let mut names: Vec<String> = fa.keys().cloned().collect();
    for k in fb.keys() {
        if !names.contains(k) {
            names.push(k.clone());
        }
    }
    names.into_iter().filter(|n| !same_value(fa.get(n), fb.get(n))).collect()
}

/// What changed from `base` to `other` (`diffProject`).
pub fn diff_project(base: &Value, other: &Value) -> Value {
    let (a, b) = (primary(base), primary(other));
    let mut result = json!({ "project": { "changed": changed_fields(None, &a, &b) } });
    for name in collection_names() {
        let (before, after) = (index_collection(&a, name), index_collection(&b, name));
        let (mut added, mut removed, mut changed) = (vec![], vec![], vec![]);
        for (id, obj) in &after {
            match before.get(id) {
                None => added.push(json!({ "id": id, "label": object_label(name, Some(obj), None) })),
                Some(prev) => {
                    let fields = changed_fields(Some(name), prev, obj);
                    if !fields.is_empty() {
                        changed.push(json!({ "id": id, "label": object_label(name, Some(obj), None), "fields": fields }));
                    }
                }
            }
        }
        for (id, obj) in &before {
            if !after.contains_key(id) {
                removed.push(json!({ "id": id, "label": object_label(name, Some(obj), None) }));
            }
        }
        result[name] = json!({ "added": added, "removed": removed, "changed": changed });
    }
    result
}

/// How many objects a diff touches (`diffSize`).
pub fn diff_size(diff: &Value) -> usize {
    let len = |v: &Value| v.as_array().map(Vec::len).unwrap_or(0);
    let mut n = len(&diff["project"]["changed"]);
    for name in collection_names() {
        n += len(&diff[name]["added"]) + len(&diff[name]["removed"]) + len(&diff[name]["changed"]);
    }
    n
}

/// The diff as a flat list (`diffEntries`).
pub fn diff_entries(diff: &Value) -> Vec<Value> {
    let mut list: Vec<Value> = diff["project"]["changed"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| json!({ "collection": "project", "id": f, "label": f, "kind": "changed", "fields": [f] }))
        .collect();
    for name in collection_names() {
        for kind in ["added", "removed", "changed"] {
            for e in diff[name][kind].as_array().into_iter().flatten() {
                let mut o = json!({ "collection": name });
                for (k, v) in e.as_object().into_iter().flatten() {
                    o[k] = v.clone();
                }
                o["kind"] = json!(kind);
                list.push(o);
            }
        }
    }
    list
}
