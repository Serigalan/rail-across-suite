// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The door for operations documents (operations-1.0; the roadmap, phase 4, O1):
//! what is the operation's — a project's timetables and their trains, the kinds
//! of trains, formations and the vehicles they run with, connections,
//! turnarounds, delay scenarios, studies — in the envelope of infra-data, one
//! domain `operations` 1.0. What comes in is checked here, what is written is
//! checked the same way before it leaves; and the timetables as the suite keeps
//! them (the record: `{ timetables: [{ id, name, data: { trains, dispatch } }],
//! … }`, decision 0016) become a document and come back from one.
//!
//! The record names a train's kind by its name (`category`) and its vehicle by
//! its key in the catalogue (`vehicle`); the document by the ids of a train
//! kind and a formation, each drawn from the name or the key where the record
//! has none of its own — the same in every project, so a document read and
//! written again is the same document. A train's order in its timetable is not
//! kept: the document's collections are sets, and the record lists a
//! timetable's trains by departure.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rail_across_js::{Object, js_string};
use serde_json::{Value, json};

use crate::canonical::{content_hash, derived_id};
use crate::error::{PayloadError, Result};
use crate::schema::{Breach, SchemaSet};

/// The file format's name and version, as the envelope states them.
pub const FORMAT: &str = "operations";
pub const VERSION: &str = "1.0";
/// The format's name as it is spoken of.
pub const FORMAT_NAME: &str = "operations-1.0";
/// The domain's name and version.
pub const DOMAIN: &str = "operations";
pub const DOMAIN_VERSION: &str = "1.0";

const ENVELOPE: &str = include_str!("../../../schemas/operations.schema.json");
const OPERATIONS: &str = include_str!("../../../schemas/operations/1.0.schema.json");

/// The collections of the domain, every one there in a document.
pub const COLLECTIONS: &[&str] = &[
    "timetables",
    "trains",
    "trainKinds",
    "formations",
    "vehicles",
    "connections",
    "turnarounds",
    "delayScenarios",
    "studies",
];

/// The collections the record keeps as the document has them.
const AS_THEY_ARE: &[&str] = &["connections", "turnarounds", "delayScenarios", "studies"];

/// The kinds of trains the suite offers (`CATEGORIES` of @rail-across/timetables),
/// in the rank a kind drawn from its name takes.
const KINDS: &[&str] = &["ICE", "IC", "EC", "RE", "RB", "S", "IRE", "Gz", "Lz", "Dz"];

struct Schemas {
    set: SchemaSet,
    domain: Value,
    hash: String,
}

fn schemas() -> &'static Schemas {
    static SCHEMAS: OnceLock<Schemas> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        let parse = |text: &str| serde_json::from_str::<Value>(text).expect("a schema of this build");
        let domain = parse(OPERATIONS);
        let mut set = SchemaSet::new();
        set.add(parse(ENVELOPE)).expect("the envelope's schema");
        // The domain names the core's types (core#/$defs/Id, Ref).
        set.add(crate::door::schema_of("core").cloned().expect("the core's schema"))
            .expect("the core's schema");
        set.add(domain.clone()).expect("the domain's schema");
        let hash = content_hash(&domain);
        Schemas { set, domain, hash }
    })
}

/// The hash the domain's schema goes by.
pub fn schema_hash() -> &'static str {
    &schemas().hash
}

/// The domain's schema, and the envelope's under "operations-document".
pub fn schema_of(name: &str) -> Option<Value> {
    match name {
        DOMAIN => Some(schemas().domain.clone()),
        "operations-document" => serde_json::from_str(ENVELOPE).ok(),
        _ => None,
    }
}

fn obj(v: &Value) -> Object {
    v.as_object().cloned().unwrap_or_default()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

/// A non-empty text of `v`, else nothing.
fn filled(v: Option<&Value>) -> Option<String> {
    text(v).filter(|s| !s.trim().is_empty())
}

fn put(o: &mut Object, key: &str, v: Option<Value>) {
    if let Some(v) = v.filter(|v| !v.is_null()) {
        o.insert(key.into(), v);
    }
}

fn sorted(mut items: Vec<Value>) -> Vec<Value> {
    crate::core_format::by_id(&mut items);
    items
}

fn breaches(errors: &[Breach], prefix: &str) -> Vec<String> {
    errors
        .iter()
        .map(|e| format!("{prefix}{}: {}", if e.path == "/" { "" } else { &e.path }, e.message))
        .collect()
}

fn check(name: &str, value: &Value) -> Vec<Breach> {
    schemas()
        .set
        .check(name, value, 20)
        .unwrap_or_else(|e| vec![Breach { path: "/".into(), message: e }])
}

// ── The door ────────────────────────────────────────────────────────────────

/// Check an operations document; returns it, or a PayloadError:
/// `unsupported_version` for another format or version, `invalid_payload`
/// (with its details) for anything else that is not as the schemas say — a
/// reference into the document's own collections that finds nothing, a carried
/// vehicle that is not what its hash says, among them.
pub fn check_operations(given: &Value) -> Result<Value> {
    if given.get("format").and_then(Value::as_str) != Some(FORMAT) || given.get("version").and_then(Value::as_str) != Some(VERSION) {
        return Err(PayloadError::with("unsupported_version", vec![format!("/format: is not {FORMAT_NAME}")]));
    }
    let shape = check("operations-document", given);
    if !shape.is_empty() {
        return Err(PayloadError::with("invalid_payload", breaches(&shape, "")));
    }
    let domain = &given["domains"][DOMAIN];
    if domain.get("version").and_then(Value::as_str) != Some(DOMAIN_VERSION) {
        return Err(PayloadError::with(
            "unsupported_version",
            vec![format!("/domains/{DOMAIN}: {}", js_string(domain.get("version")))],
        ));
    }
    if let Some(Value::Object(carried)) = given.get("schemas") {
        for (hash, schema) in carried {
            if content_hash(schema) != *hash {
                return Err(PayloadError::with("invalid_payload", vec![format!("/schemas/{hash}: holds another schema")]));
            }
        }
    }
    let blobs = given.get("blobs").and_then(Value::as_object).cloned().unwrap_or_default();
    for (hash, value) in &blobs {
        if content_hash(value) != *hash {
            return Err(PayloadError::with("invalid_payload", vec![format!("/blobs/{hash}: holds other content")]));
        }
    }
    let mut found = Vec::new();
    if domain.get("schema").and_then(Value::as_str) != Some(schema_hash()) {
        return Err(PayloadError::with(
            "invalid_payload",
            vec![format!("/domains/{DOMAIN}/schema: is not the schema of {DOMAIN} {DOMAIN_VERSION}")],
        ));
    }
    let part = json!({ "fields": domain.get("fields").cloned().unwrap_or(json!({})), "collections": domain.get("collections").cloned().unwrap_or(json!({})) });
    found.extend(breaches(&check(DOMAIN, &part), &format!("/domains/{DOMAIN}")));
    if !found.is_empty() {
        return Err(PayloadError::with("invalid_payload", found));
    }
    let cols = &domain["collections"];
    let mut ids: HashMap<&str, HashSet<String>> = HashMap::new();
    for name in COLLECTIONS {
        let seen = ids.entry(name).or_default();
        for o in list(cols, name) {
            let id = js_string(o.get("id"));
            if !seen.insert(id) {
                found.push(format!("/domains/{DOMAIN}/collections/{name}: holds {} twice", js_string(o.get("id"))));
            }
        }
    }
    // The references into the document itself find what they name; those into
    // the railway are checked where the operations are laid on a variant.
    let mut dangling = |path: String, target: &str, v: Option<&Value>| {
        if let Some(r) = v.and_then(Value::as_str) {
            if !ids.get(target).is_some_and(|s| s.contains(r)) {
                found.push(format!("{path}: names no {target} {r}"));
            }
        }
    };
    let base = format!("/domains/{DOMAIN}/collections");
    for (i, t) in list(cols, "timetables").iter().enumerate() {
        for (j, k) in list(&t["dispatching"], "kindOrder").iter().enumerate() {
            dangling(format!("{base}/timetables/{i}/dispatching/kindOrder/{j}"), "trainKinds", Some(k));
        }
    }
    for (i, t) in list(cols, "trains").iter().enumerate() {
        dangling(format!("{base}/trains/{i}/timetableId"), "timetables", t.get("timetableId"));
        dangling(format!("{base}/trains/{i}/trainKindId"), "trainKinds", t.get("trainKindId"));
        dangling(format!("{base}/trains/{i}/formationId"), "formations", t.get("formationId"));
    }
    for name in ["connections", "turnarounds"] {
        for (i, c) in list(cols, name).iter().enumerate() {
            dangling(format!("{base}/{name}/{i}/fromTrainId"), "trains", c.get("fromTrainId"));
            dangling(format!("{base}/{name}/{i}/toTrainId"), "trains", c.get("toTrainId"));
        }
    }
    for (i, s) in list(cols, "delayScenarios").iter().enumerate() {
        for key in ["entryDelays", "dwellDelays"] {
            for (j, r) in list(s, key).iter().enumerate() {
                dangling(format!("{base}/delayScenarios/{i}/{key}/{j}/trainKindId"), "trainKinds", r.get("trainKindId"));
            }
        }
    }
    for (i, s) in list(cols, "studies").iter().enumerate() {
        dangling(format!("{base}/studies/{i}/timetableId"), "timetables", s.get("timetableId"));
        dangling(format!("{base}/studies/{i}/delayScenarioId"), "delayScenarios", s.get("delayScenarioId"));
    }
    for (i, v) in list(cols, "vehicles").iter().enumerate() {
        let hash = js_string(v.get("hash"));
        if js_string(v.get("rollingStock")) != hash {
            found.push(format!("{base}/vehicles/{i}/rollingStock: is not its hash"));
        } else if !blobs.contains_key(&hash) {
            found.push(format!("{base}/vehicles/{i}/rollingStock: the file carries no blob {hash}"));
        }
    }
    if !found.is_empty() {
        return Err(PayloadError::with("invalid_payload", found));
    }
    Ok(given.clone())
}

// ── Record → document ───────────────────────────────────────────────────────

/// The id of a kind of train drawn from its name.
fn kind_id(name: &str) -> String {
    derived_id(&format!("trainKind\u{0}{name}"))
}

/// The id of the formation of one vehicle, drawn from the vehicle's key.
fn formation_id(key: &str) -> String {
    derived_id(&format!("formation\u{0}{key}"))
}

fn point_to_document(p: &Value) -> Value {
    let mut o = Object::new();
    put(&mut o, "id", p.get("id").cloned());
    put(&mut o, "place", p.get("place").filter(|v| filled(Some(v)).is_some()).cloned());
    put(&mut o, "station", Some(json!(text(p.get("station")).unwrap_or_default())));
    put(&mut o, "track", p.get("track").filter(|v| filled(Some(v)).is_some()).cloned());
    put(&mut o, "stop", Some(json!(p.get("stop").and_then(Value::as_bool).unwrap_or(false))));
    for k in ["dwell", "leastDwell", "departure"] {
        put(&mut o, k, p.get(k).filter(|v| v.is_number()).cloned());
    }
    put(&mut o, "itinerary", p.get("itinerary").filter(|v| filled(Some(v)).is_some()).cloned());
    Value::Object(o)
}

fn dispatching_of(dispatch: &Value, kinds: &mut Kinds) -> (Option<Value>, Option<Value>) {
    if !dispatch.is_object() {
        return (None, None);
    }
    let mut d = Object::new();
    put(&mut d, "mode", dispatch.get("mode").cloned());
    put(
        &mut d,
        "priority",
        dispatch
            .get("priority")
            .and_then(Value::as_str)
            .map(|p| json!(if p == "category" { "kind" } else { p })),
    );
    if let Some(Value::Array(order)) = dispatch.get("categories") {
        let ids: Vec<Value> = order.iter().filter_map(|c| filled(Some(c))).map(|c| json!(kinds.of(&c))).collect();
        d.insert("kindOrder".into(), Value::Array(ids));
    }
    let mut times = Object::new();
    for k in ["setup", "approach", "release", "sight"] {
        put(&mut times, k, dispatch["times"].get(k).filter(|v| v.is_number()).cloned());
    }
    ((!d.is_empty()).then_some(Value::Object(d)), (!times.is_empty()).then_some(Value::Object(times)))
}

/// The kinds of trains: those the record has, and one drawn from each name it uses besides.
struct Kinds {
    by_name: HashMap<String, String>,
    list: Vec<Value>,
}

impl Kinds {
    fn of(&mut self, name: &str) -> String {
        if let Some(id) = self.by_name.get(name) {
            return id.clone();
        }
        let id = kind_id(name);
        let mut o = Object::new();
        o.insert("id".into(), json!(id));
        o.insert("name".into(), json!(name));
        if let Some(rank) = KINDS.iter().position(|k| *k == name) {
            o.insert("rank".into(), json!(rank));
        }
        self.list.push(Value::Object(o));
        self.by_name.insert(name.to_string(), id.clone());
        id
    }
}

/// The operations document of a record — the timetables as the suite keeps them,
/// and the collections it has besides —, checked as the door checks what comes in
/// where `check`. `vehicles` in the record (`[{ key, vehicle }]`) are carried: their
/// rolling stock a blob, the formations naming them pinned to its hash.
pub fn to_operations(record: &Value, check: bool) -> Result<Value> {
    let mut blobs = Object::new();

    let mut vehicles = Vec::new();
    let mut pinned: HashMap<String, String> = HashMap::new();
    for v in list(record, "vehicles") {
        let Some(key) = filled(v.get("key")) else { continue };
        let stock = v.get("vehicle").cloned().unwrap_or(Value::Null);
        let hash = content_hash(&stock);
        blobs.insert(hash.clone(), stock);
        pinned.insert(key.clone(), hash.clone());
        vehicles.push(json!({ "id": derived_id(&format!("vehicle\u{0}{hash}")), "key": key, "hash": hash, "rollingStock": hash }));
    }

    let mut kinds = Kinds {
        by_name: HashMap::new(),
        list: Vec::new(),
    };
    for k in list(record, "trainKinds") {
        if let Some(name) = filled(k.get("name")) {
            kinds.by_name.entry(name).or_insert_with(|| js_string(k.get("id")));
        }
        kinds.list.push(k.clone());
    }
    let mut formations: Vec<Value> = list(record, "formations").to_vec();
    let mut formation_ids: HashSet<String> = formations.iter().map(|f| js_string(f.get("id"))).collect();

    let mut timetables = Vec::new();
    let mut trains = Vec::new();
    let mut train_ids = HashSet::new();
    for t in list(record, "timetables") {
        let tid = js_string(t.get("id"));
        let data = t.get("data").cloned().unwrap_or(json!({}));
        let mut o = Object::new();
        o.insert("id".into(), json!(tid));
        o.insert("name".into(), json!(text(t.get("name")).unwrap_or_default()));
        let (dispatching, times) = dispatching_of(data.get("dispatch").unwrap_or(&Value::Null), &mut kinds);
        put(&mut o, "dispatching", dispatching);
        put(&mut o, "blockingTimes", times);
        timetables.push(Value::Object(o));
        for tr in list(&data, "trains") {
            let mut id = js_string(tr.get("id"));
            // An id twice in the project — a timetable imported twice — is drawn anew from its timetable's.
            if !train_ids.insert(id.clone()) {
                id = derived_id(&format!("{tid}\u{0}train\u{0}{id}"));
                train_ids.insert(id.clone());
            }
            let mut o = Object::new();
            o.insert("id".into(), json!(id));
            o.insert("timetableId".into(), json!(tid));
            put(&mut o, "number", filled(tr.get("number")).map(Value::String));
            put(&mut o, "name", filled(tr.get("name")).map(Value::String));
            put(&mut o, "trainKindId", filled(tr.get("category")).map(|c| json!(kinds.of(&c))));
            let formation = match (filled(tr.get("formation")), filled(tr.get("vehicle"))) {
                (Some(f), _) => Some(f),
                (None, Some(key)) => {
                    let fid = formation_id(&key);
                    if formation_ids.insert(fid.clone()) {
                        let mut unit = Object::new();
                        unit.insert("vehicle".into(), json!(key));
                        put(&mut unit, "hash", pinned.get(&key).map(|h| json!(h)));
                        formations.push(json!({ "id": fid, "tractionUnits": [Value::Object(unit)] }));
                    }
                    Some(fid)
                }
                _ => None,
            };
            put(&mut o, "formationId", formation.map(Value::String));
            put(&mut o, "mode", filled(tr.get("mode")).map(Value::String));
            put(&mut o, "comfort", filled(tr.get("comfort")).map(Value::String));
            put(&mut o, "departure", tr.get("departure").cloned());
            put(&mut o, "margin", filled(tr.get("margin")).map(Value::String));
            put(&mut o, "distribution", tr.get("distribution").cloned());
            o.insert("points".into(), Value::Array(list(tr, "points").iter().map(point_to_document).collect()));
            trains.push(Value::Object(o));
        }
    }

    let mut cols = Object::new();
    cols.insert("timetables".into(), Value::Array(sorted(timetables)));
    cols.insert("trains".into(), Value::Array(sorted(trains)));
    cols.insert("trainKinds".into(), Value::Array(sorted(kinds.list)));
    cols.insert("formations".into(), Value::Array(sorted(formations)));
    cols.insert("vehicles".into(), Value::Array(sorted(vehicles)));
    for name in AS_THEY_ARE {
        cols.insert((*name).into(), Value::Array(sorted(list(record, name).to_vec())));
    }
    let doc = json!({
        "format": FORMAT,
        "version": VERSION,
        "domains": { DOMAIN: { "version": DOMAIN_VERSION, "schema": schema_hash(), "fields": {}, "collections": cols } },
        "blobs": blobs,
    });
    if check {
        check_operations(&doc)?;
    }
    Ok(doc)
}

// ── Document → record ───────────────────────────────────────────────────────

fn point_to_record(p: &Value) -> Value {
    let mut o = obj(p);
    for k in ["place", "track", "departure", "itinerary"] {
        o.entry(k).or_insert(Value::Null);
    }
    Value::Object(o)
}

/// The record an operations document holds, once the door has checked it: the
/// timetables as the suite keeps them, each train naming its kind by name and
/// its vehicle by key — or its formation by id, where that is more than one
/// vehicle —; the carried vehicles as `[{ key, hash, vehicle }]`; the other
/// collections as they are.
pub fn from_operations(doc: &Value) -> Result<Value> {
    let doc = check_operations(doc)?;
    let cols = &doc["domains"][DOMAIN]["collections"];
    let kinds: HashMap<String, &Value> = list(cols, "trainKinds").iter().map(|k| (js_string(k.get("id")), k)).collect();
    let formations: HashMap<String, &Value> = list(cols, "formations").iter().map(|f| (js_string(f.get("id")), f)).collect();
    // A formation of one vehicle, no more, the record names by the vehicle.
    let single = |f: &Value| -> Option<String> {
        let units = list(f, "tractionUnits");
        let more = ["name", "hauledLoad", "brakePosition", "brakedWeightPercentage"]
            .iter()
            .any(|k| f.get(*k).is_some());
        (units.len() == 1 && !more).then(|| js_string(units[0].get("vehicle")))
    };
    let mut kept_formations: Vec<Value> = Vec::new();
    let mut named_alone: HashSet<String> = HashSet::new();
    let mut by_timetable: HashMap<String, Vec<Value>> = HashMap::new();
    for tr in list(cols, "trains") {
        let mut o = Object::new();
        o.insert("id".into(), tr["id"].clone());
        o.insert("number".into(), json!(text(tr.get("number")).unwrap_or_default()));
        let kind = tr.get("trainKindId").and_then(|k| kinds.get(&js_string(Some(k))));
        o.insert("category".into(), json!(kind.map(|k| js_string(k.get("name"))).unwrap_or_default()));
        o.insert("name".into(), json!(text(tr.get("name")).unwrap_or_default()));
        let formation = tr.get("formationId").map(|f| js_string(Some(f)));
        match formation.as_ref().and_then(|f| formations.get(f).map(|x| (f, *x))) {
            Some((id, f)) => match single(f) {
                Some(key) => {
                    named_alone.insert(id.clone());
                    o.insert("vehicle".into(), json!(key));
                }
                None => {
                    o.insert("vehicle".into(), Value::Null);
                    o.insert("formation".into(), json!(id));
                }
            },
            None => {
                o.insert("vehicle".into(), Value::Null);
            }
        }
        o.insert("mode".into(), tr.get("mode").cloned().unwrap_or(Value::Null));
        put(&mut o, "comfort", tr.get("comfort").cloned());
        o.insert("departure".into(), tr["departure"].clone());
        o.insert("margin".into(), json!(text(tr.get("margin")).unwrap_or_default()));
        put(&mut o, "distribution", tr.get("distribution").cloned());
        o.insert("points".into(), Value::Array(list(tr, "points").iter().map(point_to_record).collect()));
        by_timetable.entry(js_string(tr.get("timetableId"))).or_default().push(Value::Object(o));
    }
    // Those the record names by a vehicle it keeps no more; one drawn from a key
    // and pinned to a hash it keeps, so the pin is not lost.
    for f in list(cols, "formations") {
        let id = js_string(f.get("id"));
        let pinned = list(f, "tractionUnits").iter().any(|u| u.get("hash").is_some());
        if !named_alone.contains(&id) || pinned {
            kept_formations.push(f.clone());
        }
    }
    let mut timetables = Vec::new();
    for t in list(cols, "timetables") {
        let tid = js_string(t.get("id"));
        let mut trains = by_timetable.remove(&tid).unwrap_or_default();
        trains.sort_by(|a, b| {
            let (x, y) = (a["departure"].as_f64().unwrap_or(0.0), b["departure"].as_f64().unwrap_or(0.0));
            x.total_cmp(&y)
                .then_with(|| rail_across_js::cmp_utf16(&js_string(a.get("id")), &js_string(b.get("id"))))
        });
        let mut data = Object::new();
        data.insert("trains".into(), Value::Array(trains));
        let mut dispatch = Object::new();
        if let Some(d) = t.get("dispatching") {
            put(&mut dispatch, "mode", d.get("mode").cloned());
            put(
                &mut dispatch,
                "priority",
                d.get("priority")
                    .and_then(Value::as_str)
                    .map(|p| json!(if p == "kind" { "category" } else { p })),
            );
            if let Some(Value::Array(order)) = d.get("kindOrder") {
                let names: Vec<Value> = order.iter().filter_map(|k| kinds.get(&js_string(Some(k))).map(|k| k["name"].clone())).collect();
                dispatch.insert("categories".into(), Value::Array(names));
            }
        }
        if let Some(times) = t.get("blockingTimes") {
            dispatch.insert("times".into(), times.clone());
        }
        if !dispatch.is_empty() {
            data.insert("dispatch".into(), Value::Object(dispatch));
        }
        timetables.push(json!({ "id": tid, "name": t["name"], "data": data }));
    }
    let blobs = doc.get("blobs").cloned().unwrap_or(json!({}));
    let vehicles: Vec<Value> = list(cols, "vehicles")
        .iter()
        .map(|v| json!({ "key": v["key"], "hash": v["hash"], "vehicle": blobs.get(js_string(v.get("hash"))).cloned().unwrap_or(Value::Null) }))
        .collect();
    let mut record = Object::new();
    record.insert("timetables".into(), Value::Array(timetables));
    record.insert("trainKinds".into(), Value::Array(list(cols, "trainKinds").to_vec()));
    record.insert("formations".into(), Value::Array(kept_formations));
    record.insert("vehicles".into(), Value::Array(vehicles));
    for name in AS_THEY_ARE {
        record.insert((*name).into(), Value::Array(list(cols, name).to_vec()));
    }
    Ok(Value::Object(record))
}
