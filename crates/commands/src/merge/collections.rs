// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What a project record is made of, for comparing and merging (`collections.js`).

use rail_across_js::{Object, get, js_string};
use serde_json::{Map, Value};

pub const WHOLE: &str = "*";
pub const LOCATION: &str = "at";

/// The members a record states a location in flat, where the document has one `at` (a platform's
/// `TrackRange`, an end mark's `TrackEnd`): merged as one value, as they are one in the document.
const FLAT_LOCATION: [&str; 5] = ["trackId", "startStation", "endStation", "side", "endpoint"];

/// Whether a member holds a layer the record carries: `{ fields, collections }`.
fn is_layer(v: &Value) -> bool {
    v.get("collections").is_some_and(Value::is_object)
}

/// Whether a member is a list of objects each with an id (`id`, a switch's `switchId`).
fn is_collection(v: &Value) -> bool {
    v.as_array()
        .is_some_and(|items| !items.is_empty() && items.iter().all(|o| o.get("id").is_some() || o.get("switchId").is_some()))
}

/// The collections the records hold, in the order they first come (`collectionsIn`; decision 0006): every
/// list of objects with ids at a record's top — "tracks" first —, and every list of the layers a record carries,
/// by its path ("signalling.signals"). Nothing is listed by hand: what a record holds is merged and compared object
/// by object.
pub fn collections_in(records: &[&Value]) -> Vec<String> {
    let mut out: Vec<String> = vec!["tracks".into()];
    let mut add = |name: String| {
        if !out.contains(&name) {
            out.push(name);
        }
    };
    for r in records {
        for (k, v) in r.as_object().into_iter().flatten() {
            if is_collection(v) {
                add(k.clone());
            }
        }
    }
    for r in records {
        for (k, v) in r.as_object().into_iter().flatten().filter(|(_, v)| is_layer(v)) {
            for (c, list) in v["collections"].as_object().into_iter().flatten() {
                if list.is_array() {
                    add(format!("{k}.{c}"));
                }
            }
        }
    }
    out
}

/// The layers the records carry, by the member that holds each ("signalling").
pub fn layers_in(records: &[&Value]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in records {
        for (k, _) in r.as_object().into_iter().flatten().filter(|(_, v)| is_layer(v)) {
            if !out.contains(k) {
                out.push(k.clone());
            }
        }
    }
    out
}

/// The objects of the collection at `path` of a record ("tracks", "signalling.signals").
pub fn list_of<'a>(record: &'a Value, path: &str) -> &'a [Value] {
    let v = match path.split_once('.') {
        Some((layer, c)) => record.get(layer).and_then(|l| l.get("collections")).and_then(|cs| cs.get(c)),
        None => record.get(path),
    };
    v.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The record with the collection at `path` set to `items`; a layer it names made where there is none.
pub fn set_list(record: &mut Value, path: &str, items: Vec<Value>) {
    match path.split_once('.') {
        Some((layer, c)) => {
            if !record.get(layer).is_some_and(is_layer) {
                record[layer] = serde_json::json!({ "fields": {}, "collections": {} });
            }
            record[layer]["collections"][c] = Value::Array(items);
        }
        None => record[path] = Value::Array(items),
    }
}

/// Whether `path` names a collection of the record's top, whose objects may state a location flat.
fn at_top(path: &str) -> bool {
    !path.contains('.')
}

/// The id an object goes by, as a string (`keyOf`): its `switchId` where it has one, else its `id`.
pub fn key_of(_name: &str, obj: &Value) -> String {
    match obj.get("switchId").or_else(|| obj.get("id")) {
        None => "undefined".into(),
        Some(v) => js_string(Some(v)),
    }
}

/// The objects of a collection by id, in their order (`indexCollection`).
pub fn index_collection(project: &Value, name: &str) -> Map<String, Value> {
    let mut map = Map::new();
    for obj in list_of(project, name) {
        map.insert(key_of(name, obj), obj.clone());
    }
    map
}

/// An object split into the values it is merged as (`fieldsOf`): member by member, a location the record
/// states flat (`trackId` beside its stations, side or end) as one value, `at`.
pub fn fields_of(name: &str, obj: &Value) -> Map<String, Value> {
    let mut fields = Map::new();
    let flat = at_top(name) && obj.get("trackId").is_some() && obj.get(LOCATION).is_none();
    let mut at = Object::new();
    for (k, v) in obj.as_object().into_iter().flatten() {
        if flat && FLAT_LOCATION.contains(&k.as_str()) {
            at.insert(k.clone(), v.clone());
        } else {
            fields.insert(k.clone(), v.clone());
        }
    }
    if !at.is_empty() {
        fields.insert(LOCATION.into(), Value::Object(at));
    }
    fields
}

/// The inverse of fieldsOf (`objectFromFields`): a location merged as one value stated flat again where it
/// was — a document's own `at` names its anchor, a flat one does not.
pub fn object_from_fields(name: &str, fields: &Map<String, Value>) -> Value {
    let mut obj = Object::new();
    for (k, v) in fields {
        let flat = k == LOCATION && at_top(name) && v.is_object() && v.get("anchor").is_none() && v.get("trackId").is_some();
        if flat {
            for (lk, lv) in v.as_object().into_iter().flatten() {
                obj.insert(lk.clone(), lv.clone());
            }
        } else {
            obj.insert(k.clone(), v.clone());
        }
    }
    Value::Object(obj)
}

/// The record's own fields (`projectFields`): what is neither a collection of `collections` nor a layer —
/// a layer stands here by its `fields`, its collections being merged as collections.
pub fn project_fields(project: &Value, collections: &[String]) -> Map<String, Value> {
    project
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| !collections.contains(k))
        .map(|(k, v)| {
            (
                k.clone(),
                if is_layer(v) {
                    v.get("fields").cloned().unwrap_or(serde_json::json!({}))
                } else {
                    v.clone()
                },
            )
        })
        .collect()
}

/// Whether two values are the same, key order aside (`sameValue`).
pub fn same_value(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => same(a, b),
        _ => false,
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        _ => a == b,
    }
}

/// What a person calls an object (`objectLabel`).
pub fn object_label(name: &str, obj: Option<&Value>, tracks_by_id: Option<&Map<String, Value>>) -> Value {
    let Some(obj) = obj.filter(|o| !o.is_null()) else { return Value::Null };
    let or = |keys: &[&str]| -> Value { keys.iter().find_map(|k| get(obj, k).cloned()).unwrap_or(Value::Null) };
    match name {
        "tracks" => {
            if let (Some(l), Some(t)) = (get(obj, "lineNumber"), get(obj, "trackNumber")) {
                return Value::String(format!("{}-{}", js_string(Some(l)), js_string(Some(t))));
            }
            or(&["name", "id"])
        }
        "switches" => or(&["name", "label", "switchId"]),
        // A platform by its track: its station is a point of its own (decision 0006).
        "platforms" => match tracks_by_id.and_then(|m| m.get(&js_string(obj.get("trackId")))) {
            Some(t) => object_label("tracks", Some(t), None),
            None => obj.get("id").cloned().unwrap_or(Value::Null),
        },
        "kmLines" => Value::String(js_string(obj.get("lineNumber"))),
        "operationalPoints" => or(&["name", "rl100", "uic", "id"]),
        "axisSurveys" | "referenceAxes" | "routes" => or(&["name", "id"]),
        "endMarks" => {
            if let Some(track) = tracks_by_id.and_then(|m| m.get(&js_string(obj.get("trackId")))) {
                return Value::String(format!(
                    "{} {}",
                    js_string(Some(&object_label("tracks", Some(track), None))),
                    js_string(obj.get("endpoint"))
                ));
            }
            get(obj, "id").cloned().unwrap_or(Value::Null)
        }
        _ => or(&["name", "label", "id"]),
    }
}
