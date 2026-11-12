// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The one door for infra-data documents (`door.js`; decision 0021 — what
//! was "the core 1.0 document"): what comes in is checked
//! here — the file's shape, each domain's version and schema, every object
//! against its domain's schema, the blobs against their hashes — and what is
//! written is checked the same way before it leaves. A domain of an earlier
//! schema of its version is brought to this build's first, and an infra-data
//! 1.0 document to 2.0, the layers of decision 0025 (S-R10).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rail_across_js::{Object, get, js_string, stringify};
use serde_json::{Value, json};

use crate::canonical::content_hash;
use crate::core_format::{document_to_record, domain_version, record_to_document};
use crate::error::{PayloadError, Result};
use crate::layers;
use crate::schema::{Breach, SchemaSet};

/// The file format's name, as the envelope states it.
pub const FORMAT: &str = "infra-data";
/// The format's version, as the envelope states it.
pub const VERSION: &str = "2.0";
/// The version before, which the door reads and brings to this one.
pub const VERSION_BEFORE: &str = "1.0";
/// The format's name as it is spoken of: infra-data-2.0.
pub const FORMAT_NAME: &str = "infra-data-2.0";
/// The envelope the same documents had before decision 0021 ("the core 1.0
/// document"), which only the project server's one migration still reads
/// (`upgrade_envelope`).
pub const OLD_FORMAT: &str = "rail-across";

const ENVELOPE: &str = include_str!("../../../schemas/infra-data.schema.json");
const CORE: &str = include_str!("../../../schemas/core/2.0.schema.json");
const CIVIL: &str = include_str!("../../../schemas/civil/1.0.schema.json");
const SIGNALLING: &str = include_str!("../../../schemas/signalling/2.0.schema.json");
const ELECTRIFICATION: &str = include_str!("../../../schemas/electrification/1.0.schema.json");

struct Schemas {
    set: SchemaSet,
    domains: Vec<(&'static str, Value)>,
    hashes: HashMap<&'static str, String>,
    envelope: Value,
}

fn schemas() -> &'static Schemas {
    static SCHEMAS: OnceLock<Schemas> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        let parse = |text: &str| serde_json::from_str::<Value>(text).expect("a schema of this build");
        let envelope = parse(ENVELOPE);
        let domains = vec![
            ("core", parse(CORE)),
            ("civil", parse(CIVIL)),
            ("signalling", parse(SIGNALLING)),
            ("electrification", parse(ELECTRIFICATION)),
        ];
        let mut set = SchemaSet::new();
        set.add(envelope.clone()).expect("the envelope's schema");
        for (_, s) in &domains {
            set.add(s.clone()).expect("a domain's schema");
        }
        let hashes = domains.iter().map(|(d, s)| (*d, content_hash(s))).collect();
        Schemas {
            set,
            domains,
            hashes,
            envelope,
        }
    })
}

/// The hash each domain's schema goes by — what a document names it with.
pub fn schema_hashes() -> &'static HashMap<&'static str, String> {
    &schemas().hashes
}

/// The schema of a domain this build reads, by name, and the envelope's ("infra-data").
pub fn schema_of(name: &str) -> Option<&'static Value> {
    if name == FORMAT {
        return Some(&schemas().envelope);
    }
    schemas().domains.iter().find(|(d, _)| *d == name).map(|(_, s)| s)
}

const SIGNALLING_FIRST: &str = "sha256:8e4fdf8b8d487f61be93b473521515fe2e5510ef3cdc21d994cd8a405aeb74a0";
const SIGNALLING_SECOND: &str = "sha256:72fccdfdcac3f5ce42d7f49d10fb9134e1c4ed792f7365ec9f01d997f0049724";
const SIGNALLING_THIRD: &str = "sha256:0194c1e7fe2a63741270025beb48929d106d74a80bf5b33ed4305965a5303487";
const SIGNALLING_FOURTH: &str = "sha256:cee619f2ae1b256a81ca5536e3851602cce96841d16bb2fe5f11ffe1a5e1f64c";
const SIGNALLING_FIFTH: &str = "sha256:d5c61938fb3da1cc80f638c051cac562c19ad207507373ac3381f3a97922d83c";
const SIGNALLING_SIXTH: &str = "sha256:aeb06433b03125995823095d7ab89567311237560cbc3b9e6a954385305f0134";
/// Signalling 1.0 as it last was, and core and layout 1.0: what the step to 2.0 starts from.
const SIGNALLING_SEVENTH: &str = "sha256:b28ada7513c555fa4696ff2b41c73bb7565143854c08e2e9bfb067e99018910c";
const CORE_FIRST: &str = "sha256:c237af09e3ac3915398d9c7795088518df4023bfd0b954053dc9f04112fbbaf6";
const CORE_SECOND: &str = "sha256:16ae19c7adbb08fedabbb0ad778fc765c7faa8a8b2c56cbdbc9adfa91f622a19";
const CORE_THIRD: &str = "sha256:14ac2357f8675371ff06e4f874b7b70ae3fbaa697127bf44f71c50f6382578b4";
const LAYOUT_FIRST: &str = "sha256:b2280dcf6cfe779e148b6c402ac46eb4ff1e8d96b55e6e3abcb370ef48ff1f1b";
/// Core 2.0 before its stopping places said whether they are passenger stops,
/// and before it held the named runs of tracks, which civil 1.0 held first.
const CORE_2_FIRST: &str = "sha256:53f47ced7884515ad1ed5ef2f19e4eb2b895a45b76797751110e99e2f4d08877";
const CORE_2_SECOND: &str = "sha256:9a5f24df5585bebdd3386021af91aa98325de4c80c5d13fca7092d2eb79ae84f";
/// Civil 1.0 with the named runs of tracks.
const CIVIL_FIRST: &str = "sha256:0b73763db4b8ca57f56472f4b8aeef8a8b02369f4f8a9e3f2b4072480c48b1fb";
/// The domains of infra-data 1.0, with the schema each had last.
const BEFORE: &[(&str, &str)] = &[("core", CORE_THIRD), ("layout", LAYOUT_FIRST), ("signalling", SIGNALLING_SEVENTH)];

fn obj(v: &Value) -> Object {
    v.as_object().cloned().unwrap_or_default()
}

fn list(v: &Value, key: &str) -> Vec<Value> {
    get(v, key).and_then(Value::as_array).cloned().unwrap_or_default()
}

/// `defined(o)`: JSON has no `undefined`; members absent stay absent.
fn set_opt(o: &mut Object, key: &str, v: Option<Value>) {
    if let Some(v) = v {
        o.insert(key.into(), v);
    }
}

fn with_schema(domain: &Value, schema: &str, collections: Option<Object>) -> Value {
    let mut d = obj(domain);
    d.insert("schema".into(), json!(schema));
    if let Some(c) = collections {
        d.insert("collections".into(), Value::Object(c));
    }
    Value::Object(d)
}

/// One step of a signalling domain of an earlier schema, where there is one.
fn signalling_step(domain: &Value) -> Option<Value> {
    let schema = domain.get("schema").and_then(Value::as_str)?;
    let cols = domain.get("collections").cloned().unwrap_or(json!({}));
    match schema {
        SIGNALLING_FIRST => {
            let mut rest = obj(&cols);
            rest.shift_remove("signals");
            rest.shift_remove("pzbElements");
            let signals: Vec<Value> = list(&cols, "signals")
                .iter()
                .map(|s| {
                    let system = s.get("system").filter(|v| rail_across_js::truthy(Some(v))).cloned();
                    let kind = s.get("type").filter(|v| rail_across_js::truthy(Some(v))).cloned();
                    let mut o = obj(s);
                    o.shift_remove("system");
                    o.shift_remove("type");
                    let logical = match (&system, &kind) {
                        (Some(sys), Some(t)) => Some(json!([{ "system": sys, "type": t }])),
                        (Some(sys), None) => Some(json!([{ "system": sys }])),
                        (None, Some(t)) if t == "shunting" => Some(json!([{ "system": "sh", "type": t }])),
                        _ => None,
                    };
                    set_opt(&mut o, "logicalSignals", logical);
                    Value::Object(o)
                })
                .collect();
            let balises: Vec<Value> = list(&cols, "pzbElements")
                .iter()
                .map(|p| {
                    let mut o = obj(p);
                    let frequency = o.shift_remove("frequency");
                    let ina = o.shift_remove("ina");
                    let speed_check = o.shift_remove("speedCheck");
                    o.insert("system".into(), json!("pzb"));
                    let mut settings = Object::new();
                    set_opt(&mut settings, "frequency", frequency);
                    set_opt(&mut settings, "ina", ina);
                    if let Some(Value::Object(sc)) = speed_check {
                        for (k, v) in sc {
                            settings.insert(k, v);
                        }
                    }
                    o.insert("settings".into(), Value::Object(settings));
                    Value::Object(o)
                })
                .collect();
            rest.insert("signals".into(), Value::Array(signals));
            rest.insert("balises".into(), Value::Array(balises));
            Some(with_schema(domain, SIGNALLING_SECOND, Some(rest)))
        }
        SIGNALLING_SECOND => {
            let mut c = obj(&cols);
            c.insert("speedSections".into(), json!([]));
            Some(with_schema(domain, SIGNALLING_THIRD, Some(c)))
        }
        SIGNALLING_THIRD => {
            let mut rest = obj(&cols);
            for k in ["speedSections", "signals", "routes"] {
                rest.shift_remove(k);
            }
            let routes = list(&cols, "routes");
            let mut speeds: Vec<(String, Vec<Value>)> = Vec::new();
            for r in &routes {
                let Some(speed) = get(r, "speed") else { continue };
                let start = js_string(r.get("startSignalId"));
                let entry = json!({ "routeId": r.get("id").cloned().unwrap_or(Value::Null), "speed": speed });
                match speeds.iter_mut().find(|(s, _)| *s == start) {
                    Some((_, v)) => v.push(entry),
                    None => speeds.push((start, vec![entry])),
                }
            }
            let of_logical = ["screen", "indicator"];
            let signals: Vec<Value> = list(&cols, "signals")
                .iter()
                .map(|s| {
                    let logicals = list(s, "logicalSignals");
                    if logicals.is_empty() {
                        return s.clone();
                    }
                    let frames = list(s, "frames");
                    let mut place = obj(s);
                    let function = place.shift_remove("function");
                    place.shift_remove("frames");
                    place.shift_remove("logicalSignals");
                    let kind_of = |f: &Value| f.get("kind").and_then(Value::as_str).map(str::to_string);
                    let own: Vec<Value> = frames
                        .iter()
                        .filter(|f| !kind_of(f).is_some_and(|k| of_logical.contains(&k.as_str())))
                        .cloned()
                        .collect();
                    let shown: Vec<Value> = frames
                        .iter()
                        .filter(|f| kind_of(f).is_some_and(|k| of_logical.contains(&k.as_str())))
                        .cloned()
                        .collect();
                    if !own.is_empty() || s.get("kind").and_then(Value::as_str) == Some("real") {
                        place.insert("frames".into(), Value::Array(own));
                    }
                    // `{ ...first, function, frames, signallingSpeeds }`: a member given as
                    // undefined replaces the logical signal's own, and `defined` drops it.
                    let mut first = obj(&logicals[0]);
                    let mut put = |key: &str, v: Option<Value>| match v {
                        Some(v) => {
                            first.insert(key.into(), v);
                        }
                        None => {
                            first.shift_remove(key);
                        }
                    };
                    put("function", function);
                    put("frames", if shown.is_empty() { None } else { Some(Value::Array(shown)) });
                    let id = js_string(s.get("id"));
                    put("signallingSpeeds", speeds.iter().find(|(k, _)| *k == id).map(|(_, v)| Value::Array(v.clone())));
                    let mut all = vec![Value::Object(first)];
                    all.extend(logicals[1..].iter().cloned());
                    place.insert("logicalSignals".into(), Value::Array(all));
                    Value::Object(place)
                })
                .collect();
            let routes: Vec<Value> = routes
                .iter()
                .map(|r| {
                    let mut o = obj(r);
                    o.shift_remove("speed");
                    Value::Object(o)
                })
                .collect();
            rest.insert("signals".into(), Value::Array(signals));
            rest.insert("routes".into(), Value::Array(routes));
            Some(with_schema(domain, SIGNALLING_FOURTH, Some(rest)))
        }
        SIGNALLING_FOURTH => Some(with_schema(domain, SIGNALLING_FIFTH, None)),
        SIGNALLING_FIFTH => {
            let mut c = obj(&cols);
            c.insert("stoppingPlaces".into(), json!([]));
            c.insert("itineraries".into(), json!([]));
            Some(with_schema(domain, SIGNALLING_SIXTH, Some(c)))
        }
        SIGNALLING_SIXTH => Some(with_schema(domain, SIGNALLING_SEVENTH, None)),
        _ => None,
    }
}

/// One step of a core domain of an earlier schema, where there is one.
fn core_step(domain: &Value) -> Option<Value> {
    match domain.get("schema").and_then(Value::as_str)? {
        CORE_FIRST => {
            let cols = domain.get("collections").cloned().unwrap_or(json!({}));
            let tracks: Vec<Value> = list(&cols, "tracks")
                .iter()
                .map(|t| {
                    let Some(Value::Array(els)) = t.get("elements").filter(|e| rail_across_js::truthy(Some(e))) else {
                        return t.clone();
                    };
                    let els: Vec<Value> = els
                        .iter()
                        .map(|el| {
                            if !rail_across_js::has(el, "speed") {
                                return el.clone();
                            }
                            let mut o = obj(el);
                            let speed = o.shift_remove("speed").unwrap_or(Value::Null);
                            o.insert("designSpeed".into(), speed);
                            Value::Object(o)
                        })
                        .collect();
                    let mut o = obj(t);
                    o.insert("elements".into(), Value::Array(els));
                    Value::Object(o)
                })
                .collect();
            let mut c = obj(&cols);
            c.insert("tracks".into(), Value::Array(tracks));
            Some(with_schema(domain, CORE_SECOND, Some(c)))
        }
        CORE_SECOND => Some(with_schema(domain, CORE_THIRD, None)),
        _ => None,
    }
}

/// An infra-data 1.0 document with every domain of an earlier schema of 1.0
/// brought to 1.0's last, as the door did while 1.0 was this build's.
fn migrated_within_1_0(doc: &Value) -> Value {
    let domains = &doc["domains"];
    let mut core = domains.get("core").cloned();
    let mut signalling = domains.get("signalling").cloned();
    let mut changed = false;
    if let Some(c) = core.as_mut().filter(|c| c.get("version").and_then(Value::as_str) == Some("1.0")) {
        while let Some(next) = core_step(c) {
            *c = next;
            changed = true;
        }
    }
    if let Some(s) = signalling.as_mut().filter(|s| s.get("version").and_then(Value::as_str) == Some("1.0")) {
        while let Some(next) = signalling_step(s) {
            *s = next;
            changed = true;
        }
    }
    if !changed {
        return doc.clone();
    }
    let mut d = obj(domains);
    if let Some(c) = core {
        d.insert("core".into(), c);
    }
    if let Some(s) = signalling {
        d.insert("signalling".into(), s);
    }
    let mut out = obj(doc);
    out.insert("domains".into(), Value::Object(d));
    Value::Object(out)
}

/// An infra-data 2.0 document with its core and civil of an earlier schema of
/// 2.0 brought to this build's: a stopping place a platform names is a
/// passenger stop (core's second), the named runs of tracks are the core's, no
/// longer civil's (core's third, civil's second).
fn migrated_within_2_0(doc: &Value) -> Value {
    let schema = |d: Option<&Value>| d.and_then(|d| d.get("schema")).and_then(Value::as_str).map(str::to_string);
    let mut core = doc["domains"].get("core").cloned();
    let mut civil = doc["domains"].get("civil").cloned();
    let (core_was, civil_was) = (schema(core.as_ref()), schema(civil.as_ref()));
    if let Some(c) = core.as_mut() {
        if schema(Some(c)).as_deref() == Some(CORE_2_FIRST) {
            let platforms = civil.as_ref().map(|c| list(&c["collections"], "platforms")).unwrap_or_default();
            let mut cols = obj(c.get("collections").unwrap_or(&Value::Null));
            if let Some(Value::Array(places)) = cols.get("stoppingPlaces").cloned() {
                cols.insert("stoppingPlaces".into(), Value::Array(layers::passenger_stops(&places, &platforms)));
            }
            *c = with_schema(c, CORE_2_SECOND, Some(cols));
        }
    }
    // The runs leave civil of before, for the core.
    let mut runs = None;
    if let Some(v) = civil.as_mut().filter(|v| schema(Some(v)).as_deref() == Some(CIVIL_FIRST)) {
        let mut cols = obj(v.get("collections").unwrap_or(&Value::Null));
        runs = cols.shift_remove("trackPaths");
        *v = with_schema(v, &schema_hashes()["civil"], Some(cols));
    }
    if let Some(c) = core.as_mut() {
        if schema(Some(c)).as_deref() == Some(CORE_2_SECOND) {
            let mut cols = obj(c.get("collections").unwrap_or(&Value::Null));
            cols.insert("trackPaths".into(), runs.unwrap_or(json!([])));
            *c = with_schema(c, &schema_hashes()["core"], Some(cols));
        }
    }
    if schema(core.as_ref()) == core_was && schema(civil.as_ref()) == civil_was {
        return doc.clone();
    }
    let mut d = obj(&doc["domains"]);
    for (name, domain) in [("core", core), ("civil", civil)] {
        if let Some(domain) = domain {
            d.insert(name.into(), domain);
        }
    }
    let mut out = obj(doc);
    out.insert("domains".into(), Value::Object(d));
    // A document that carries its schemas carries this build's of what it changed.
    if let Some(Value::Object(carried)) = doc.get("schemas") {
        let mut carried = carried.clone();
        for (name, was) in [("core", core_was), ("civil", civil_was)] {
            let now = &schema_hashes()[name];
            if let Some(was) = was.filter(|w| w != now && carried.contains_key(w)) {
                carried.shift_remove(&was);
                carried.insert(now.clone(), schema_of(name).cloned().unwrap_or(Value::Null));
            }
        }
        out.insert("schemas".into(), Value::Object(carried));
    }
    Value::Object(out)
}

fn sorted_by_id(mut items: Vec<Value>) -> Vec<Value> {
    crate::core_format::by_id(&mut items);
    items
}

/// infra-data 1.0, its domains at 1.0's last schemas, as infra-data 2.0 (decision 0025):
/// `layout` and the platforms into `civil`; the stopping places out of `signalling` into
/// the core, a stopping place's platform into the platform's `stoppingPlaceIds`; a
/// boundary marked by a platform marked by the stopping place at it, one made over the
/// platform where there is none. Schemas it carried, it carries as 2.0's. A domain of
/// another schema is left as it was, for the check to name.
fn to_2_0(doc: &Value) -> Value {
    let domains = obj(&doc["domains"]);
    let schema_is = |name: &str, hash: &str| domains.get(name).and_then(|d| d.get("schema")).and_then(Value::as_str) == Some(hash);
    if !BEFORE.iter().all(|(name, hash)| !domains.contains_key(*name) || schema_is(name, hash)) {
        let mut out = obj(doc);
        out.insert("version".into(), json!(VERSION));
        return Value::Object(out);
    }
    let core = domains.get("core").cloned().unwrap_or(json!({}));
    let mut core_cols = obj(core.get("collections").unwrap_or(&Value::Null));
    let platforms = core_cols.shift_remove("platforms").and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let mut out_domains = Object::new();

    let mut stopping = Vec::new();
    let mut signalling = None;
    if let Some(sig) = domains.get("signalling") {
        let mut cols = obj(sig.get("collections").unwrap_or(&Value::Null));
        stopping = cols.shift_remove("stoppingPlaces").and_then(|v| v.as_array().cloned()).unwrap_or_default();
        layers::boundaries_by_stopping_place(&mut cols, &mut stopping, &platforms);
        let mut d = obj(sig);
        d.insert("version".into(), json!(crate::core_format::domain_version("signalling")));
        d.insert("schema".into(), json!(schema_hashes()["signalling"]));
        d.insert("collections".into(), Value::Object(cols));
        signalling = Some(Value::Object(d));
    }
    core_cols.insert("stoppingPlaces".into(), Value::Array(sorted_by_id(layers::core_stopping_places(&stopping))));
    let layout = domains.get("layout");
    let layout_cols = layout.map(|l| obj(l.get("collections").unwrap_or(&Value::Null))).unwrap_or_default();
    core_cols.insert("trackPaths".into(), layout_cols.get("trackPaths").cloned().unwrap_or(json!([])));
    let mut c = obj(&core);
    c.insert("version".into(), json!(crate::core_format::domain_version("core")));
    c.insert("schema".into(), json!(schema_hashes()["core"]));
    c.insert("collections".into(), Value::Object(core_cols));
    out_domains.insert("core".into(), Value::Object(c));

    let mut civil_cols = Object::new();
    civil_cols.insert("platforms".into(), Value::Array(layers::platforms_naming(&platforms, &stopping)));
    for k in ["axisSurveys", "referenceAxes"] {
        civil_cols.insert(k.into(), layout_cols.get(k).cloned().unwrap_or(json!([])));
    }
    let civil_fields = layout.map(|l| obj(l.get("fields").unwrap_or(&Value::Null))).unwrap_or_default();
    if layout.is_some() || layers::holds_anything(&civil_fields, &civil_cols) {
        let mut d = Object::new();
        d.insert("version".into(), json!(crate::core_format::domain_version("civil")));
        d.insert("schema".into(), json!(schema_hashes()["civil"]));
        d.insert("dependsOn".into(), json!(["core"]));
        d.insert("fields".into(), Value::Object(civil_fields));
        d.insert("collections".into(), Value::Object(civil_cols));
        out_domains.insert("civil".into(), Value::Object(d));
    }
    if let Some(s) = signalling {
        out_domains.insert("signalling".into(), s);
    }

    let mut out = obj(doc);
    out.insert("version".into(), json!(VERSION));
    if doc.get("schemas").is_some() {
        let mut carried = Object::new();
        for name in out_domains.keys() {
            if let (Some(h), Some(s)) = (schema_hashes().get(name.as_str()), schema_of(name)) {
                carried.insert(h.clone(), s.clone());
            }
        }
        out.insert("schemas".into(), Value::Object(carried));
    }
    out.insert("domains".into(), Value::Object(out_domains));
    Value::Object(out)
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

/// Check an infra-data document; returns it as infra-data-2.0 — an infra-data 1.0
/// one brought to it, a domain of an earlier schema of its version first brought to
/// that version's last —, or a PayloadError (`checkDocument`).
pub fn check_document(given: &Value) -> Result<Value> {
    let version = given.get("version").and_then(Value::as_str);
    if given.get("format").and_then(Value::as_str) != Some(FORMAT) || !matches!(version, Some(VERSION | VERSION_BEFORE)) {
        return Err(PayloadError::with("unsupported_version", vec![format!("/format: is not {FORMAT_NAME}")]));
    }
    // The envelope's shape is the same in both versions but for the version itself.
    let mut as_now = obj(given);
    as_now.insert("version".into(), json!(VERSION));
    let shape = check(FORMAT, &Value::Object(as_now));
    if !shape.is_empty() {
        return Err(PayloadError::with("invalid_payload", breaches(&shape, "")));
    }
    if version == Some(VERSION_BEFORE) {
        for (name, domain) in given["domains"].as_object().into_iter().flatten() {
            let known = BEFORE.iter().any(|(d, _)| d == name);
            if !known || domain.get("version").and_then(Value::as_str) != Some("1.0") {
                return Err(PayloadError::with(
                    "unsupported_version",
                    vec![format!("/domains/{name}: {}", js_string(domain.get("version")))],
                ));
            }
        }
    }
    let doc = if version == Some(VERSION_BEFORE) {
        let within = migrated_within_1_0(given);
        let wrong: Vec<String> = BEFORE
            .iter()
            .filter(|(name, hash)| {
                within["domains"]
                    .get(*name)
                    .is_some_and(|d| d.get("schema").and_then(Value::as_str) != Some(*hash))
            })
            .map(|(name, _)| format!("/domains/{name}/schema: is not the schema of {name} 1.0"))
            .collect();
        if !wrong.is_empty() {
            return Err(PayloadError::with("invalid_payload", wrong));
        }
        to_2_0(&within)
    } else {
        migrated_within_2_0(given)
    };
    if let Some(Value::Object(schemas)) = doc.get("schemas") {
        for (hash, schema) in schemas {
            if content_hash(schema) != *hash {
                return Err(PayloadError::with("invalid_payload", vec![format!("/schemas/{hash}: holds another schema")]));
            }
        }
    }
    if let Some(Value::Object(blobs)) = doc.get("blobs") {
        for (hash, value) in blobs {
            if content_hash(value) != *hash {
                return Err(PayloadError::with("invalid_payload", vec![format!("/blobs/{hash}: holds other content")]));
            }
        }
    }
    let mut found = Vec::new();
    if let Some(Value::Object(domains)) = doc.get("domains") {
        for (name, domain) in domains {
            let version = domain.get("version");
            if domain_version(name).map(|v| Value::String(v.into())).as_ref() != version {
                return Err(PayloadError::with(
                    "unsupported_version",
                    vec![format!("/domains/{name}: {}", js_string(version))],
                ));
            }
            if domain.get("schema").and_then(Value::as_str) != schema_hashes().get(name.as_str()).map(String::as_str) {
                found.push(format!("/domains/{name}/schema: is not the schema of {name} {}", js_string(version)));
                continue;
            }
            let deps = domain.get("dependsOn").and_then(Value::as_array).cloned().unwrap_or_default();
            if name != "core" && !(deps.len() == 1 && deps[0] == json!("core")) {
                found.push(format!("/domains/{name}/dependsOn: is not [\"core\"]"));
            }
            let mut part = Object::new();
            if let Some(f) = domain.get("fields") {
                part.insert("fields".into(), f.clone());
            }
            if let Some(c) = domain.get("collections") {
                part.insert("collections".into(), c.clone());
            }
            found.extend(breaches(&check(name, &Value::Object(part)), &format!("/domains/{name}")));
            if let Some(Value::Object(cols)) = domain.get("collections") {
                for (cname, list) in cols {
                    let mut seen = HashSet::new();
                    for o in list.as_array().into_iter().flatten() {
                        let id = o.get("id").map(stringify).unwrap_or_else(|| "undefined".into());
                        if !seen.insert(id) {
                            found.push(format!("/domains/{name}/collections/{cname}: holds {} twice", js_string(o.get("id"))));
                        }
                    }
                }
            }
        }
    }
    found.extend(switch_branches_without_switch(&doc));
    if !found.is_empty() {
        return Err(PayloadError::with("invalid_payload", found));
    }
    Ok(doc)
}

/// What the schema cannot say: an element marked as a switch's branch names
/// the switch it belongs to (the rule the record's door held before decision 0021).
fn switch_branches_without_switch(doc: &Value) -> Vec<String> {
    let tracks = doc["domains"]["core"]["collections"]["tracks"].as_array().cloned().unwrap_or_default();
    let mut found = Vec::new();
    for (i, track) in tracks.iter().enumerate() {
        for (j, el) in track["elements"].as_array().into_iter().flatten().enumerate() {
            if el.get("switchBranch") == Some(&Value::Bool(true)) && !el.get("switchId").is_some_and(|id| rail_across_js::truthy(Some(id))) {
                found.push(format!(
                    "/domains/core/collections/tracks/{i}/elements/{j}: is a switch's branch without its switchId"
                ));
            }
        }
    }
    found
}

/// The infra-data-2.0 document of a record (dehydrated), checked as the door checks
/// what comes in; `carry_schemas` puts the domains' schemas into it, `check:
/// false` leaves the check out (`toDocument`).
pub fn to_document(record: &Value, carry_schemas: bool, check: bool) -> Result<Value> {
    let mut doc = record_to_document(record, schema_hashes())?;
    if carry_schemas {
        let mut carried = Object::new();
        for d in doc["domains"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default() {
            if let (Some(h), Some(s)) = (schema_hashes().get(d.as_str()), schema_of(&d)) {
                carried.insert(h.clone(), s.clone());
            }
        }
        doc.as_object_mut().expect("document").insert("schemas".into(), Value::Object(carried));
    }
    if check { check_document(&doc) } else { Ok(doc) }
}

/// The record an infra-data document holds, once the door has checked it (`fromDocument`).
pub fn from_document(doc: &Value) -> Result<Value> {
    document_to_record(&check_document(doc)?)
}

/// A document of the envelope before decision 0021 (`format: "rail-across"`,
/// no version) in today's (`format: "infra-data"`, `version: "1.0"`) — the
/// domains are the same. Anything else comes back as it was. Only the project
/// server's migration of its revisions calls this (decision 0021, point 2).
pub fn upgrade_envelope(doc: &Value) -> Value {
    match doc {
        Value::Object(o) if o.get("format").and_then(Value::as_str) == Some(OLD_FORMAT) => {
            let mut out = Object::new();
            out.insert("format".into(), json!(FORMAT));
            out.insert("version".into(), json!(VERSION_BEFORE));
            for (k, v) in o {
                if k != "format" && k != "version" {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}
