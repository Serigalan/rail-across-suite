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

/// A record come back from its document with each collection in the order the record had it: what was there before
/// keeps its place, what is new follows (decision 0029, phase 6 — the document sorts by id, the record keeps the
/// order it was given).
pub fn in_order_of(record: &mut Value, before: &Value) {
    fn order(list: &mut Value, before: Option<&Value>) {
        let (Value::Array(items), Some(Value::Array(was))) = (list, before) else {
            return;
        };
        let key = |x: &Value| x.get("id").or_else(|| x.get("switchId")).and_then(Value::as_str).map(str::to_string);
        let place: std::collections::HashMap<String, usize> = was.iter().enumerate().filter_map(|(i, x)| Some((key(x)?, i))).collect();
        items.sort_by_key(|x| key(x).and_then(|k| place.get(&k).copied()).unwrap_or(usize::MAX));
    }
    let Some(o) = record.as_object_mut() else { return };
    for (k, v) in o.iter_mut() {
        if matches!(k.as_str(), "signalling" | "electrification") {
            for (c, list) in v.get_mut("collections").and_then(Value::as_object_mut).into_iter().flatten() {
                order(list, before.get(k).and_then(|d| d.get("collections")).and_then(|cs| cs.get(c)));
            }
        } else {
            order(v, before.get(k));
        }
    }
}
