// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Every id a UUID in lower case (decision 0029, phase 2): an id of before that is none — a km line's
//! `km-6050`, a short id a tool or a test gave — becomes the UUID drawn from it (`derived_id`), and every
//! reference to it the same, since a reference is named for what it names and ends in `Id` or `Ids` (phase 1).
//! The same id gives the same UUID on every device; a UUID in capitals is written in lower case.

use rail_across_js::Object;
use serde_json::Value;

use crate::canonical::derived_id;

/// Whether `s` is a UUID in lower case.
pub fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
        })
}

/// The UUID an id goes by.
pub fn uuid_of(id: &str) -> String {
    if is_uuid(id) {
        return id.to_string();
    }
    let lower = id.to_ascii_lowercase();
    if is_uuid(&lower) { lower } else { derived_id(id) }
}

fn is_reference(key: &str) -> bool {
    key == "id" || key.ends_with("Id") || key.ends_with("Ids")
}

fn walk(value: &mut Value) {
    match value {
        Value::Object(o) => {
            for (k, v) in o.iter_mut() {
                if is_reference(k) {
                    match v {
                        Value::String(s) => *s = uuid_of(s),
                        Value::Array(items) => {
                            for item in items.iter_mut() {
                                if let Value::String(s) = item {
                                    *s = uuid_of(s);
                                }
                            }
                        }
                        _ => walk(v),
                    }
                } else {
                    walk(v);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(walk),
        _ => {}
    }
}

/// The ids of a document's domains as UUIDs: every object of every collection, every reference in it, and the
/// project's id in the core's fields. The other fields are left: what they hold under `id` is a catalogue's key
/// or a foreign program's (OSRD's), not an object of the document.
pub fn forward(domains: &mut Object) {
    for domain in domains.values_mut() {
        if let Some(cols) = domain.get_mut("collections") {
            walk(cols);
        }
    }
    if let Some(Value::String(id)) = domains.get_mut("core").and_then(|c| c.get_mut("fields")).and_then(|f| f.get_mut("id")) {
        *id = uuid_of(id);
    }
}

// The record names some references as before (decision 0029, phase 1): not ending in `Id`.
const BEFORE: &[&str] = &["operationalPoint", "dep", "boundedBy", "clearSections"];

fn collect(value: &Value, parent: &str, out: &mut std::collections::HashMap<String, String>) {
    let take = |out: &mut std::collections::HashMap<String, String>, s: &str| {
        out.insert(uuid_of(s), s.to_string());
    };
    match value {
        Value::Object(o) => {
            for (k, v) in o {
                let reference = is_reference(k) || BEFORE.contains(&k.as_str()) || (parent == "passOn" && (k == "left" || k == "right"));
                match v {
                    Value::String(s) if reference => take(out, s),
                    Value::Array(items) if reference && items.iter().all(Value::is_string) => {
                        items.iter().filter_map(Value::as_str).for_each(|s| take(out, s));
                    }
                    _ if k == "catalogue" || k == "osrd" => {}
                    _ => collect(v, k, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect(v, parent, out)),
        _ => {}
    }
}

/// The ids a record holds, by the UUID each goes by in its document: what `restored` gives back.
pub fn originals(record: &Value) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    collect(record, "", &mut out);
    out.retain(|uuid, id| uuid != id);
    out
}

/// A record come back from its document with the ids it had before (`originals`), each collection in the order of
/// those ids as the document had them before: what the record works on, until it follows the document (decision
/// 0029, phase 6).
pub fn restored_record(record: &mut Value, originals: &std::collections::HashMap<String, String>) {
    restored(record, originals);
    let Some(o) = record.as_object_mut() else { return };
    for (k, v) in o.iter_mut() {
        let lists: Vec<&mut Value> = if matches!(k.as_str(), "signalling" | "electrification") {
            v.get_mut("collections")
                .and_then(Value::as_object_mut)
                .map(|c| c.values_mut().collect())
                .unwrap_or_default()
        } else {
            vec![v]
        };
        for list in lists {
            if let Value::Array(items) = list
                && items.iter().all(|x| x.get("id").is_some())
            {
                crate::core_format::by_id(items);
            }
        }
    }
}

fn restored(value: &mut Value, originals: &std::collections::HashMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(id) = originals.get(s.as_str()) {
                *s = id.clone();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| restored(v, originals)),
        Value::Object(o) => o.values_mut().for_each(|v| restored(v, originals)),
        _ => {}
    }
}
