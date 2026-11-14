// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The door for operations documents (operations-1.0; the roadmap, phase 4, O1;
//! decision 0029, phase 4): what is the operation's, in two domains — `vehicles`
//! 1.0, the vehicles a project's trains run with, each with its rolling stock as
//! RailJSON, and `timetables` 1.0, the timetables and their trains, the kinds of
//! trains, formations, connections, turnarounds, delay scenarios, studies. What
//! comes in is checked here, what is written is checked the same way before it
//! leaves; and the timetables as the suite keeps them (the record: `{ timetables:
//! [{ id, name, data: { trains, dispatch } }], … }`, decision 0016) become a
//! document and come back from one. A document of before — one domain
//! `operations` 1.0, the vehicles as blobs, a train's point naming its place in
//! one member — is brought to the two domains on its way in (`from_before`).
//!
//! The record names a train's kind by its name (`category`) and its vehicle by
//! its key in the catalogue (`vehicle`); the document by the ids of a train
//! kind and a formation, each drawn from the name or the key where the record
//! has none of its own — the same in every project, so a document read and
//! written again is the same document. A vehicle the record does not carry is
//! one of the suite's catalogue: the document names it by its key, without its
//! rolling stock. A train's order in its timetable is not kept: the document's
//! collections are sets, and the record lists a timetable's trains by departure.

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
/// The domains and their versions.
pub const DOMAINS: &[(&str, &str)] = &[("vehicles", "1.0"), ("timetables", "1.0")];

const ENVELOPE: &str = include_str!("../../../schemas/operations.schema.json");
const VEHICLES: &str = include_str!("../../../schemas/vehicles/1.0.schema.json");
const TIMETABLES: &str = include_str!("../../../schemas/timetables/1.0.schema.json");
/// The operations of before: one domain `operations` 1.0, named by core 3.1's types.
const ENVELOPE_BEFORE: &str = include_str!("../../../schemas/operations-before.schema.json");
const OPERATIONS_BEFORE: &str = include_str!("../../../schemas/operations/1.0.schema.json");
const CORE_OF_BEFORE: &str = include_str!("../../../schemas/core/3.1.schema.json");

/// The collections of the domain timetables, every one there in a document.
pub const COLLECTIONS: &[&str] = &["timetables", "trains", "trainKinds", "formations", "connections", "turnarounds", "delayScenarios", "studies"];

/// The collections the record keeps as the document has them.
const AS_THEY_ARE: &[&str] = &["connections", "turnarounds", "delayScenarios", "studies"];

/// The kinds of trains the suite offers (`CATEGORIES` of @rail-across/timetables),
/// in the rank a kind drawn from its name takes.
const KINDS: &[&str] = &["ICE", "IC", "EC", "RE", "RB", "S", "IRE", "Gz", "Lz", "Dz"];

struct Schemas {
    set: SchemaSet,
    domains: Vec<(&'static str, Value)>,
    hashes: HashMap<&'static str, String>,
}

fn schemas() -> &'static Schemas {
    static SCHEMAS: OnceLock<Schemas> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        let parse = |text: &str| serde_json::from_str::<Value>(text).expect("a schema of this build");
        let domains = vec![("vehicles", parse(VEHICLES)), ("timetables", parse(TIMETABLES))];
        let mut set = SchemaSet::new();
        set.add(parse(ENVELOPE)).expect("the envelope's schema");
        // The domains name the core's types (core#/$defs/Id, Ref): an id a UUID in lower case.
        set.add(crate::door::schema_of("core").cloned().expect("the core's schema")).expect("the core's schema");
        for (_, s) in &domains {
            set.add(s.clone()).expect("a domain's schema");
        }
        let hashes = domains.iter().map(|(d, s)| (*d, content_hash(s))).collect();
        Schemas { set, domains, hashes }
    })
}

/// The schemas of before (`OPERATIONS_BEFORE`) and the hash its domain went by.
fn before() -> &'static (SchemaSet, String) {
    static BEFORE: OnceLock<(SchemaSet, String)> = OnceLock::new();
    BEFORE.get_or_init(|| {
        let parse = |text: &str| serde_json::from_str::<Value>(text).expect("a schema of this build");
        let domain = parse(OPERATIONS_BEFORE);
        let mut set = SchemaSet::new();
        set.add(parse(ENVELOPE_BEFORE)).expect("the envelope's schema");
        set.add(parse(CORE_OF_BEFORE)).expect("the core's schema");
        set.add(domain.clone()).expect("the domain's schema");
        let hash = content_hash(&domain);
        (set, hash)
    })
}

/// The hash each domain's schema goes by.
pub fn schema_hashes() -> &'static HashMap<&'static str, String> {
    &schemas().hashes
}

/// A domain's schema by its name, and the envelope's under "operations-document".
pub fn schema_of(name: &str) -> Option<Value> {
    match name {
        "operations-document" => serde_json::from_str(ENVELOPE).ok(),
        _ => schemas().domains.iter().find(|(d, _)| *d == name).map(|(_, s)| s.clone()),
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

fn check_before(name: &str, value: &Value) -> Vec<Breach> {
    before().0.check(name, value, 20).unwrap_or_else(|e| vec![Breach { path: "/".into(), message: e }])
}

/// The id of a vehicle drawn from its key: one vehicle of a key in a document.
fn vehicle_id(key: &str) -> String {
    derived_id(&format!("vehicle\u{0}{key}"))
}

// ── The door ────────────────────────────────────────────────────────────────

/// Check an operations document; returns it as this build reads it — a document of before brought to the two
/// domains (`from_before`) —, or a PayloadError: `unsupported_version` for another format or version,
/// `invalid_payload` (with its details) for anything else that is not as the schemas say — a reference into the
/// document's own domains that finds nothing among them.
pub fn check_operations(given: &Value) -> Result<Value> {
    if given.get("format").and_then(Value::as_str) != Some(FORMAT) || given.get("version").and_then(Value::as_str) != Some(VERSION) {
        return Err(PayloadError::with("unsupported_version", vec![format!("/format: is not {FORMAT_NAME}")]));
    }
    if given["domains"].get("operations").is_some() {
        check_before_document(given)?;
        let doc = from_before(given, &HashSet::new());
        check_now(&doc)?;
        return Ok(doc);
    }
    check_now(given)?;
    Ok(given.clone())
}

/// A document of the two domains checked: its shape, each domain's version and schema, the carried schemas and
/// blobs against their hashes, each id once in its collection, and every reference into the two domains finding
/// what it names. Those into the railway are checked where the operations are laid on a variant.
fn check_now(given: &Value) -> Result<()> {
    let shape = check("operations-document", given);
    if !shape.is_empty() {
        return Err(PayloadError::with("invalid_payload", breaches(&shape, "")));
    }
    for (name, version) in DOMAINS {
        let theirs = given["domains"][name].get("version").and_then(Value::as_str).unwrap_or("");
        if theirs != *version {
            return Err(PayloadError::with("unsupported_version", vec![format!("/domains/{name}: {theirs}")]));
        }
    }
    for member in ["schemas", "blobs"] {
        for (hash, value) in given.get(member).and_then(Value::as_object).into_iter().flatten() {
            if content_hash(value) != *hash {
                return Err(PayloadError::with("invalid_payload", vec![format!("/{member}/{hash}: holds other content")]));
            }
        }
    }
    let mut found = Vec::new();
    let mut ids: HashMap<String, HashSet<String>> = HashMap::new();
    let mut references = Vec::new();
    for (name, version) in DOMAINS {
        let domain = &given["domains"][name];
        if domain.get("schema").and_then(Value::as_str) != schema_hashes().get(name).map(String::as_str) {
            found.push(format!("/domains/{name}/schema: is not the schema of {name} {version}"));
            continue;
        }
        let deps = domain.get("dependsOn").cloned().unwrap_or(json!([]));
        let want = if *name == "timetables" { json!(["vehicles"]) } else { json!([]) };
        if deps != want {
            found.push(format!("/domains/{name}/dependsOn: is not {want}"));
        }
        let part = json!({ "fields": domain.get("fields").cloned().unwrap_or(json!({})), "collections": domain.get("collections").cloned().unwrap_or(json!({})) });
        found.extend(breaches(&check(name, &part), &format!("/domains/{name}")));
        for (cname, items) in domain["collections"].as_object().into_iter().flatten() {
            let seen = ids.entry(format!("{name}.{cname}")).or_default();
            for o in items.as_array().into_iter().flatten() {
                if !seen.insert(js_string(o.get("id"))) {
                    found.push(format!("/domains/{name}/collections/{cname}: holds {} twice", js_string(o.get("id"))));
                }
            }
        }
        for r in schemas().set.references(name, &part).unwrap_or_default() {
            references.push((name, r));
        }
    }
    if found.is_empty() {
        for (name, r) in references {
            let ours = r.target.starts_with("vehicles.") || r.target.starts_with("timetables.");
            if ours && !ids.get(&r.target).is_some_and(|s| s.contains(&r.id)) {
                found.push(format!("/domains/{name}{}: names {}, which {} lacks", r.path, r.id, r.target));
            }
        }
    }
    if !found.is_empty() {
        return Err(PayloadError::with("invalid_payload", found));
    }
    Ok(())
}

/// A train's point of before as timetables 1.0 states it: its place an end mark where `end_marks` has it, else an
/// operational point; one that named its station by its name alone the point drawn from that name.
fn point_from_before(p: &Value, end_marks: &HashSet<String>) -> Value {
    let mut o = Object::new();
    put(&mut o, "id", p.get("id").cloned());
    match given(p.get("place")) {
        Some(Value::String(place)) if end_marks.contains(&place) => {
            o.insert("endMarkId".into(), json!(place));
        }
        Some(place) => {
            o.insert("operationalPointId".into(), place);
        }
        None => put(&mut o, "operationalPointId", filled(p.get("station")).map(|n| json!(derived_id(&format!("station\u{0}{n}"))))),
    }
    put(&mut o, "trackId", given(p.get("track")));
    put(&mut o, "stop", p.get("stop").cloned());
    for k in ["dwell", "leastDwell", "departure"] {
        put(&mut o, k, p.get(k).cloned());
    }
    put(&mut o, "itineraryId", given(p.get("itinerary")));
    Value::Object(o)
}

fn renamed(v: &Value, from: &str, to: &str) -> Value {
    let mut o = obj(v);
    if let Some(x) = o.shift_remove(from) {
        o.insert(to.into(), x);
    }
    Value::Object(o)
}

/// A document of before (one domain `operations` 1.0) in the two domains of today (decision 0029, phase 4): the
/// vehicles with their rolling stock in the object, one of the catalogue named by its key, a formation naming its
/// vehicles by id; a train's point naming an operational point or an end mark — those in `end_marks` — by id, with
/// its track and itinerary; a connection's and a delay rule's place an operational point, a study's stretch its
/// two; every id a UUID.
pub fn from_before(doc: &Value, end_marks: &HashSet<String>) -> Value {
    let cols = &doc["domains"]["operations"]["collections"];
    let blobs = doc.get("blobs").cloned().unwrap_or(json!({}));
    let mut vehicles: Vec<Value> = Vec::new();
    let mut known: HashSet<String> = HashSet::new();
    for v in list(cols, "vehicles") {
        let Some(key) = filled(v.get("key")) else { continue };
        if !known.insert(key.clone()) {
            continue;
        }
        let mut o = Object::new();
        o.insert("id".into(), json!(vehicle_id(&key)));
        o.insert("key".into(), json!(key));
        put(&mut o, "rollingStock", blobs.get(js_string(v.get("rollingStock"))).cloned());
        vehicles.push(Value::Object(o));
    }
    let mut formations = Vec::new();
    for f in list(cols, "formations") {
        let mut o = obj(f);
        let units: Vec<Value> = list(f, "tractionUnits")
            .iter()
            .map(|u| {
                let key = js_string(u.get("vehicle"));
                if known.insert(key.clone()) {
                    vehicles.push(json!({ "id": vehicle_id(&key), "key": key }));
                }
                json!({ "vehicleId": vehicle_id(&key) })
            })
            .collect();
        o.insert("tractionUnits".into(), Value::Array(units));
        formations.push(Value::Object(o));
    }
    let trains: Vec<Value> = list(cols, "trains")
        .iter()
        .map(|t| {
            let mut o = obj(t);
            o.insert("points".into(), Value::Array(list(t, "points").iter().map(|p| point_from_before(p, end_marks)).collect()));
            Value::Object(o)
        })
        .collect();
    let connections: Vec<Value> = list(cols, "connections").iter().map(|c| renamed(c, "place", "operationalPointId")).collect();
    let scenarios: Vec<Value> = list(cols, "delayScenarios")
        .iter()
        .map(|s| {
            let mut o = obj(s);
            for key in ["entryDelays", "dwellDelays"] {
                if let Some(Value::Array(rules)) = s.get(key) {
                    o.insert(key.into(), Value::Array(rules.iter().map(|r| renamed(r, "place", "operationalPointId")).collect()));
                }
            }
            Value::Object(o)
        })
        .collect();
    let studies: Vec<Value> = list(cols, "studies")
        .iter()
        .map(|s| {
            let mut o = obj(s);
            if let Some(Value::Array(stretches)) = s.get("stretches") {
                let next = stretches.iter().map(|x| renamed(&renamed(x, "from", "fromOperationalPointId"), "to", "toOperationalPointId")).collect();
                o.insert("stretches".into(), Value::Array(next));
            }
            Value::Object(o)
        })
        .collect();
    let mut tcols = Object::new();
    tcols.insert("timetables".into(), json!(list(cols, "timetables")));
    tcols.insert("trains".into(), Value::Array(trains));
    tcols.insert("trainKinds".into(), json!(list(cols, "trainKinds")));
    tcols.insert("formations".into(), Value::Array(formations));
    tcols.insert("connections".into(), Value::Array(connections));
    tcols.insert("turnarounds".into(), json!(list(cols, "turnarounds")));
    tcols.insert("delayScenarios".into(), Value::Array(scenarios));
    tcols.insert("studies".into(), Value::Array(studies));
    let mut vcols = Object::new();
    vcols.insert("vehicles".into(), Value::Array(vehicles));
    documented(vcols, tcols)
}

/// The document of the two domains' collections: every id a UUID — a kind of train in a timetable's order too —,
/// every collection sorted by id.
fn documented(vehicles: Object, timetables: Object) -> Value {
    let mut domains = Object::new();
    domains.insert("vehicles".into(), json!({ "version": "1.0", "schema": schema_hashes()["vehicles"], "fields": {}, "collections": vehicles }));
    domains.insert(
        "timetables".into(),
        json!({ "version": "1.0", "schema": schema_hashes()["timetables"], "dependsOn": ["vehicles"], "fields": {}, "collections": timetables }),
    );
    crate::ids::forward(&mut domains);
    for t in domains["timetables"]["collections"]["timetables"].as_array_mut().into_iter().flatten() {
        if let Some(Value::Array(order)) = t.get_mut("dispatching").and_then(|d| d.get_mut("kindOrder")) {
            for k in order.iter_mut() {
                if let Value::String(id) = k {
                    *id = crate::ids::uuid_of(id);
                }
            }
        }
    }
    crate::core_format::sorted_collections(&mut domains);
    json!({ "format": FORMAT, "version": VERSION, "domains": Value::Object(domains), "blobs": {} })
}

/// A document of before checked as the door checked it then: one domain `operations` 1.0, its references into
/// itself finding what they name, its carried vehicles what their hashes say.
fn check_before_document(given: &Value) -> Result<()> {
    const DOMAIN: &str = "operations";
    const DOMAIN_VERSION: &str = "1.0";
    const COLLECTIONS: &[&str] = &["timetables", "trains", "trainKinds", "formations", "vehicles", "connections", "turnarounds", "delayScenarios", "studies"];
    let shape = check_before("operations-document-before", given);
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
    if domain.get("schema").and_then(Value::as_str) != Some(before().1.as_str()) {
        return Err(PayloadError::with(
            "invalid_payload",
            vec![format!("/domains/{DOMAIN}/schema: is not the schema of {DOMAIN} {DOMAIN_VERSION}")],
        ));
    }
    let part = json!({ "fields": domain.get("fields").cloned().unwrap_or(json!({})), "collections": domain.get("collections").cloned().unwrap_or(json!({})) });
    found.extend(breaches(&check_before(DOMAIN, &part), &format!("/domains/{DOMAIN}")));
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
    Ok(())
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

/// What the record gives for a field: nothing where it is missing, null or an
/// empty text, else the value as it is — what is not of its type the schema then
/// refuses, nothing is turned into something else on the way.
fn given(v: Option<&Value>) -> Option<Value> {
    match v {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(v) => Some(v.clone()),
    }
}

/// A record's point as timetables 1.0 states it. A point of before — its place in one member, `place`, or only
/// the name of its station; `track`, `itinerary` — as a working copy may still hold it, is read as the door reads
/// a document of before, its place an operational point.
fn point_to_document(p: &Value) -> Value {
    if !p.is_object() {
        return p.clone();
    }
    if ["place", "station", "track", "itinerary"].iter().any(|k| p.get(*k).is_some()) {
        let mut now = point_from_before(p, &HashSet::new());
        if let Some(o) = now.as_object_mut() {
            for k in ["operationalPointId", "endMarkId", "trackId", "itineraryId"] {
                if let Some(v) = given(p.get(k)) {
                    o.insert(k.into(), v);
                }
            }
        }
        return now;
    }
    let mut o = Object::new();
    put(&mut o, "id", p.get("id").cloned());
    put(&mut o, "operationalPointId", given(p.get("operationalPointId")));
    put(&mut o, "endMarkId", given(p.get("endMarkId")));
    put(&mut o, "trackId", given(p.get("trackId")));
    put(&mut o, "stop", p.get("stop").cloned());
    for k in ["dwell", "leastDwell", "departure"] {
        put(&mut o, k, p.get(k).cloned());
    }
    put(&mut o, "itineraryId", given(p.get("itineraryId")));
    Value::Object(o)
}

fn dispatching_of(dispatch: &Value, kinds: &mut Kinds) -> (Option<Value>, Option<Value>) {
    if dispatch.is_null() {
        return (None, None);
    }
    if !dispatch.is_object() {
        return (Some(dispatch.clone()), None);
    }
    let mut d = Object::new();
    put(&mut d, "mode", dispatch.get("mode").cloned());
    put(
        &mut d,
        "priority",
        dispatch.get("priority").map(|p| if p == "category" { json!("kind") } else { p.clone() }),
    );
    match dispatch.get("categories") {
        Some(Value::Array(order)) => {
            let ids: Vec<Value> = order
                .iter()
                .map(|c| match c.as_str() {
                    Some(name) if !name.trim().is_empty() => json!(kinds.of(name)),
                    _ => c.clone(),
                })
                .collect();
            d.insert("kindOrder".into(), Value::Array(ids));
        }
        other => put(&mut d, "kindOrder", other.cloned()),
    }
    let mut times = Object::new();
    match dispatch.get("times") {
        Some(Value::Object(given_times)) => {
            for k in ["setup", "approach", "release", "sight"] {
                put(&mut times, k, given_times.get(k).cloned());
            }
        }
        // what is no object stands where the schema refuses it
        Some(v) if !v.is_null() => {
            times.insert("setup".into(), v.clone());
        }
        _ => {}
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
/// where `check`. `vehicles` in the record (`[{ key, vehicle }]`) are carried, their
/// rolling stock in the object; a vehicle named by its key and not carried is the
/// catalogue's, the document names it by its key alone.
pub fn to_operations(record: &Value, check: bool) -> Result<Value> {
    let mut vehicles: Vec<Value> = Vec::new();
    let mut known: HashSet<String> = HashSet::new();
    for v in list(record, "vehicles") {
        let Some(key) = filled(v.get("key")) else { continue };
        if known.insert(key.clone()) {
            vehicles.push(json!({ "id": vehicle_id(&key), "key": key, "rollingStock": v.get("vehicle").cloned().unwrap_or(Value::Null) }));
        }
    }
    let mut unit_of = |key: &str| -> Value {
        if known.insert(key.to_string()) {
            vehicles.push(json!({ "id": vehicle_id(key), "key": key }));
        }
        json!({ "vehicleId": vehicle_id(key) })
    };

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
    // The record's formations name their vehicles by key, as its trains do.
    let mut formations: Vec<Value> = list(record, "formations")
        .iter()
        .map(|f| {
            let mut o = obj(f);
            let units: Vec<Value> = list(f, "tractionUnits").iter().map(|u| unit_of(&js_string(u.get("vehicle")))).collect();
            o.insert("tractionUnits".into(), Value::Array(units));
            Value::Object(o)
        })
        .collect();
    let mut formation_ids: HashSet<String> = formations.iter().map(|f| js_string(f.get("id"))).collect();

    let mut timetables = Vec::new();
    let mut trains = Vec::new();
    let mut train_ids = HashSet::new();
    for t in list(record, "timetables") {
        // Ids as the document states them, so that one given twice is found twice (decision 0029, phase 2).
        let tid = crate::ids::uuid_of(&js_string(t.get("id")));
        let data = t.get("data").cloned().unwrap_or(json!({}));
        let mut o = Object::new();
        o.insert("id".into(), json!(tid));
        put(&mut o, "name", t.get("name").cloned());
        let (dispatching, times) = dispatching_of(data.get("dispatch").unwrap_or(&Value::Null), &mut kinds);
        put(&mut o, "dispatching", dispatching);
        put(&mut o, "blockingTimes", times);
        timetables.push(Value::Object(o));
        for tr in list(&data, "trains") {
            if !tr.is_object() {
                trains.push(tr.clone());
                continue;
            }
            let mut o = Object::new();
            match tr.get("id").and_then(Value::as_str) {
                Some(given_id) => {
                    let mut id = crate::ids::uuid_of(given_id);
                    // An id twice in the project — a timetable imported twice — is drawn anew from its timetable's.
                    if !train_ids.insert(id.clone()) {
                        id = derived_id(&format!("{tid}\u{0}train\u{0}{id}"));
                        train_ids.insert(id.clone());
                    }
                    o.insert("id".into(), json!(id));
                }
                None => put(&mut o, "id", tr.get("id").cloned()),
            }
            o.insert("timetableId".into(), json!(tid));
            put(&mut o, "number", given(tr.get("number")));
            put(&mut o, "name", given(tr.get("name")));
            put(
                &mut o,
                "trainKindId",
                given(tr.get("category")).map(|c| match c.as_str() {
                    Some(name) => json!(kinds.of(name)),
                    None => c,
                }),
            );
            let formation = match (given(tr.get("formation")), given(tr.get("vehicle"))) {
                (Some(f), _) => Some(f),
                (None, Some(Value::String(key))) => {
                    let fid = formation_id(&key);
                    if formation_ids.insert(fid.clone()) {
                        formations.push(json!({ "id": fid, "tractionUnits": [unit_of(&key)] }));
                    }
                    Some(json!(fid))
                }
                (None, other) => other,
            };
            put(&mut o, "formationId", formation);
            put(&mut o, "mode", given(tr.get("mode")));
            put(&mut o, "comfort", given(tr.get("comfort")));
            put(&mut o, "departure", tr.get("departure").cloned());
            put(&mut o, "margin", given(tr.get("margin")));
            put(&mut o, "distribution", tr.get("distribution").cloned());
            match tr.get("points") {
                Some(Value::Array(points)) => {
                    o.insert("points".into(), Value::Array(points.iter().map(point_to_document).collect()));
                }
                other => put(&mut o, "points", other.cloned()),
            }
            trains.push(Value::Object(o));
        }
    }

    let mut tcols = Object::new();
    tcols.insert("timetables".into(), Value::Array(timetables));
    tcols.insert("trains".into(), Value::Array(trains));
    tcols.insert("trainKinds".into(), Value::Array(kinds.list));
    tcols.insert("formations".into(), Value::Array(formations));
    for name in AS_THEY_ARE {
        tcols.insert((*name).into(), Value::Array(list(record, name).to_vec()));
    }
    let mut vcols = Object::new();
    vcols.insert("vehicles".into(), Value::Array(vehicles));
    let doc = documented(vcols, tcols);
    if check {
        check_operations(&doc)?;
    }
    Ok(doc)
}

// ── Document → record ───────────────────────────────────────────────────────

fn point_to_record(p: &Value) -> Value {
    let mut o = obj(p);
    for k in ["trackId", "departure", "itineraryId"] {
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
    let cols = &doc["domains"]["timetables"]["collections"];
    let vehicles_by_id: HashMap<String, &Value> =
        list(&doc["domains"]["vehicles"]["collections"], "vehicles").iter().map(|v| (js_string(v.get("id")), v)).collect();
    let key_of = |u: &Value| -> Value {
        vehicles_by_id.get(&js_string(u.get("vehicleId"))).and_then(|v| v.get("key")).cloned().unwrap_or(Value::Null)
    };
    let kinds: HashMap<String, &Value> = list(cols, "trainKinds").iter().map(|k| (js_string(k.get("id")), k)).collect();
    let formations: HashMap<String, &Value> = list(cols, "formations").iter().map(|f| (js_string(f.get("id")), f)).collect();
    // A formation of one vehicle, no more, the record names by the vehicle.
    let single = |f: &Value| -> Option<String> {
        let units = list(f, "tractionUnits");
        let more = ["name", "hauledLoad", "brakePosition", "brakedWeightPercentage"]
            .iter()
            .any(|k| f.get(*k).is_some());
        (units.len() == 1 && !more).then(|| js_string(Some(&key_of(&units[0]))))
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
    // Those the record names by a vehicle it keeps no more; the others with their vehicles by key.
    for f in list(cols, "formations") {
        if !named_alone.contains(&js_string(f.get("id"))) {
            let mut o = obj(f);
            let units: Vec<Value> = list(f, "tractionUnits").iter().map(|u| json!({ "vehicle": key_of(u) })).collect();
            o.insert("tractionUnits".into(), Value::Array(units));
            kept_formations.push(Value::Object(o));
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
    // The vehicles carried: those with their rolling stock.
    let vehicles: Vec<Value> = list(&doc["domains"]["vehicles"]["collections"], "vehicles")
        .iter()
        .filter_map(|v| v.get("rollingStock").map(|stock| json!({ "key": v["key"], "hash": content_hash(stock), "vehicle": stock })))
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
