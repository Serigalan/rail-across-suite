// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The manifest of a document (`manifest.js`): per domain its version, its
//! schema and the hash of its fields and of each collection — the hash tree of
//! the data model.

use rail_across_js::{Object, cmp_utf16};
use serde_json::Value;

use crate::canonical::content_hash;

fn sorted_keys(o: &Object) -> Vec<&String> {
    let mut keys: Vec<&String> = o.keys().collect();
    keys.sort_by(|a, b| cmp_utf16(a, b));
    keys
}

/// The manifest of `doc`.
pub fn manifest_of(doc: &Value) -> Value {
    let empty = Object::new();
    let doc_domains = doc.get("domains").and_then(Value::as_object).unwrap_or(&empty);
    let mut domains = Object::new();
    for name in sorted_keys(doc_domains) {
        let d = &doc_domains[name.as_str()];
        let mut m = Object::new();
        m.insert("version".into(), d.get("version").cloned().unwrap_or(Value::Null));
        m.insert("schema".into(), d.get("schema").cloned().unwrap_or(Value::Null));
        if let Some(dep) = d.get("dependsOn").filter(|v| rail_across_js::truthy(Some(v))) {
            m.insert("dependsOn".into(), dep.clone());
        }
        m.insert("fields".into(), Value::String(content_hash(d.get("fields").unwrap_or(&Value::Null))));
        let cols = d.get("collections").and_then(Value::as_object).unwrap_or(&empty);
        let mut c = Object::new();
        for k in sorted_keys(cols) {
            c.insert(k.clone(), Value::String(content_hash(&cols[k.as_str()])));
        }
        m.insert("collections".into(), Value::Object(c));
        domains.insert(name.clone(), Value::Object(m));
    }
    let mut out = Object::new();
    out.insert("domains".into(), Value::Object(domains));
    Value::Object(out)
}

/// The hash of a document as a whole: its manifest's.
pub fn document_hash(doc: &Value) -> String {
    content_hash(&manifest_of(doc))
}
