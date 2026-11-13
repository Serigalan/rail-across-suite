// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What a project record is made of, for comparing and merging (`collections.js`).

use rail_across_js::{Object, get, js_string};
use serde_json::{Map, Value};

pub const WHOLE: &str = "*";
pub const LOCATION: &str = "at";

/// A collection: its name, the key its objects go by, whether it is one value, its location fields.
pub struct Collection {
    pub name: &'static str,
    pub key: &'static str,
    pub whole: bool,
    pub location: &'static [&'static str],
}

pub const COLLECTIONS: [Collection; 9] = [
    Collection {
        name: "tracks",
        key: "id",
        whole: false,
        location: &[],
    },
    Collection {
        name: "switches",
        key: "switchId",
        whole: false,
        location: &[],
    },
    Collection {
        name: "platforms",
        key: "id",
        whole: false,
        location: &["trackId", "startStation", "endStation", "side"],
    },
    Collection {
        name: "kmLines",
        key: "lineNumber",
        whole: true,
        location: &[],
    },
    Collection {
        name: "endMarks",
        key: "id",
        whole: true,
        location: &[],
    },
    Collection {
        name: "axisSurveys",
        key: "id",
        whole: true,
        location: &[],
    },
    Collection {
        name: "referenceAxes",
        key: "id",
        whole: true,
        location: &[],
    },
    Collection {
        name: "routes",
        key: "id",
        whole: true,
        location: &[],
    },
    Collection {
        name: "operationalPoints",
        key: "id",
        whole: false,
        location: &[],
    },
];

pub fn collection(name: &str) -> Option<&'static Collection> {
    COLLECTIONS.iter().find(|c| c.name == name)
}

pub fn collection_names() -> impl Iterator<Item = &'static str> {
    COLLECTIONS.iter().map(|c| c.name)
}

/// The id an object goes by, as a string (`keyOf`).
pub fn key_of(name: &str, obj: &Value) -> String {
    let key = collection(name).map(|c| c.key).unwrap_or("id");
    match obj.get(key) {
        None => "undefined".into(),
        Some(v) => js_string(Some(v)),
    }
}

/// The objects of a collection by id, in their order (`indexCollection`).
pub fn index_collection(project: &Value, name: &str) -> Map<String, Value> {
    let mut map = Map::new();
    for obj in project.get(name).and_then(Value::as_array).into_iter().flatten() {
        map.insert(key_of(name, obj), obj.clone());
    }
    map
}

/// An object split into the values it is merged as (`fieldsOf`).
pub fn fields_of(name: &str, obj: &Value) -> Map<String, Value> {
    let c = collection(name).expect("a collection");
    let mut fields = Map::new();
    if c.whole {
        fields.insert(WHOLE.into(), obj.clone());
        return fields;
    }
    let mut at = Object::new();
    for (k, v) in obj.as_object().into_iter().flatten() {
        if c.location.contains(&k.as_str()) {
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

/// The inverse of fieldsOf (`objectFromFields`).
pub fn object_from_fields(name: &str, fields: &Map<String, Value>) -> Value {
    let c = collection(name).expect("a collection");
    if c.whole {
        return fields.get(WHOLE).cloned().unwrap_or(Value::Null);
    }
    let mut obj = Object::new();
    for (k, v) in fields {
        if k == LOCATION && !c.location.is_empty() {
            for (lk, lv) in v.as_object().into_iter().flatten() {
                obj.insert(lk.clone(), lv.clone());
            }
        } else {
            obj.insert(k.clone(), v.clone());
        }
    }
    Value::Object(obj)
}

/// The project's own fields (`projectFields`).
pub fn project_fields(project: &Value) -> Map<String, Value> {
    project
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| collection(k).is_none())
        .map(|(k, v)| (k.clone(), v.clone()))
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
        // A platform by its track: its station is a point of its own (decision 0028).
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
        _ => get(obj, "id").cloned().unwrap_or(Value::Null),
    }
}
