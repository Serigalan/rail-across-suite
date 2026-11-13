// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What changed from one state to another (`diff.js`).

use rail_across_doc::persistence::dehydrate_projects;
use serde_json::{Map, Value, json};

use super::collections::{collections_in, fields_of, index_collection, object_label, project_fields, same_value};

/// A record as merged: dehydrated (`primary`).
pub fn primary(project: &Value) -> Value {
    let p = if project.is_null() { json!({}) } else { project.clone() };
    dehydrate_projects(&[p]).remove(0)
}

fn changed_fields(name: Option<&str>, a: &Value, b: &Value, collections: &[String]) -> Vec<String> {
    let (fa, fb): (Map<String, Value>, Map<String, Value>) = match name {
        Some(n) => (fields_of(n, a), fields_of(n, b)),
        None => (project_fields(a, collections), project_fields(b, collections)),
    };
    let mut names: Vec<String> = fa.keys().cloned().collect();
    for k in fb.keys() {
        if !names.contains(k) {
            names.push(k.clone());
        }
    }
    names.into_iter().filter(|n| !same_value(fa.get(n), fb.get(n))).collect()
}

/// What changed from `base` to `other` (`diffProject`): the record's own fields, and each collection either
/// holds (collectionsIn) object by object.
pub fn diff_project(base: &Value, other: &Value) -> Value {
    let (a, b) = (primary(base), primary(other));
    let collections = collections_in(&[&a, &b]);
    let mut result = json!({ "project": { "changed": changed_fields(None, &a, &b, &collections) } });
    for name in collections.iter().map(String::as_str) {
        let (before, after) = (index_collection(&a, name), index_collection(&b, name));
        let (mut added, mut removed, mut changed) = (vec![], vec![], vec![]);
        for (id, obj) in &after {
            match before.get(id) {
                None => added.push(json!({ "id": id, "label": object_label(name, Some(obj), None) })),
                Some(prev) => {
                    let fields = changed_fields(Some(name), prev, obj, &collections);
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
    for d in diff.as_object().into_iter().flatten().filter(|(k, _)| *k != "project").map(|(_, d)| d) {
        n += len(&d["added"]) + len(&d["removed"]) + len(&d["changed"]);
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
    for (name, d) in diff.as_object().into_iter().flatten().filter(|(k, _)| *k != "project") {
        for kind in ["added", "removed", "changed"] {
            for e in d[kind].as_array().into_iter().flatten() {
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
