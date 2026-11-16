// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The one door for infra-data documents (`door.js`; decision 0021 — what
//! was "the core 1.0 document"): what comes in is checked
//! here — the file's shape, each domain's version and schema, every object
//! against its domain's schema, the blobs against their hashes, what a schema
//! cannot say (a lifespan's days) — and what is written is checked the same
//! way before it leaves. A document of before is brought forward: an
//! infra-data 1.0 document to 2.0, the layers of decision 0025 (S-R10), a
//! domain of an earlier schema of 2.0 to 2.0's last — both told apart by the
//! schemas' hashes, as they were then —, and from there version by version
//! (`STEPS`; decision 0028: every change of a schema raises the number).

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
const CORE: &str = include_str!("../../../schemas/core/5.0.schema.json");
const CIVIL: &str = include_str!("../../../schemas/civil/3.0.schema.json");
const SIGNALLING: &str = include_str!("../../../schemas/signalling/4.0.schema.json");
const ELECTRIFICATION: &str = include_str!("../../../schemas/electrification/3.0.schema.json");

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
/// Core 2.0 with the runs, before its stages had days and switches a stage link (decision 0027).
const CORE_2_THIRD: &str = "sha256:3c3b80c99c8ba41e0ab2da7c00e1f821a0231e821afd11c2888b5350390bea5c";
/// Civil 1.0 with the named runs of tracks.
const CIVIL_FIRST: &str = "sha256:0b73763db4b8ca57f56472f4b8aeef8a8b02369f4f8a9e3f2b4072480c48b1fb";
/// Civil 1.0 without the runs, before its bridges, retaining walls and noise barriers.
const CIVIL_SECOND: &str = "sha256:252301f2051a66b451c4c6c8280af0e0f5aca2ca09fa6db2fcf53e9d8c402a60";
/// The domains as infra-data 2.0 had them last, before versions counted (decision 0028): the version and the
/// schema each went by. What the steps by number start from.
const CORE_2_LAST: &str = "sha256:98b48fc30ca564b929df9b839b49d918c4f6c5b2f7d29393cce8e268818a32e3";
const CIVIL_LAST: &str = "sha256:976db8ee32fc843127777a4a90345dcb074d6cb962916ec0b4856b693a6d241b";
const SIGNALLING_2_LAST: &str = "sha256:b46b740ba722a6eae945038d8a6149cd0c540258df478a77b4a9b8988ae7d515";
const ELECTRIFICATION_LAST: &str = "sha256:b7149b613fc281af8623e05eb0fb7924e788f49533d6d0c9e018f1fed37b0b30";
/// The versions before this build's that a step starts from, with the hash each one's schema goes by.
const PAST: &[(&str, &str, &str)] = &[
    ("core", "2.0", CORE_2_LAST),
    ("civil", "1.0", CIVIL_LAST),
    ("signalling", "2.0", SIGNALLING_2_LAST),
    ("electrification", "1.0", ELECTRIFICATION_LAST),
    // Lifespans by day, a reference free to name nothing (decision 0028, phase A).
    ("core", "3.0", "sha256:72e2181866b52fa05ca684e66c5b2e4fcc91fdf46f57ad63a0690ccfec210805"),
    ("civil", "2.0", "sha256:b96345ebf422de4fb8d7c1bf426b6db7468f5f85f36d531df86a2e0d17f4e03c"),
    ("signalling", "3.0", "sha256:cb1bad694c35458f33858b91d4893c137da0c378ce92a7a7217addbc13d27083"),
    ("electrification", "2.0", "sha256:812af795a72e4fec5e9f15cfec93bf2180907775ee98db987a9d8c92dfec2eb9"),
    // Every reference names an object, the kinds still numbers (decision 0028, phase C).
    ("core", "3.1", "sha256:47b5516e9ea7bcf1e059a157091df3f9675d384f4daebe7e1d49c6ad590bcf2c"),
    ("civil", "2.1", "sha256:19ddfd584a19ab1a7011bd3d207b7838446482357c7a5933e0366a268f17f879"),
    ("signalling", "3.1", "sha256:2b365df5f72c616a99c70729f50afd5fdd243f14cff4798f78a70f0855a1e14d"),
    // Electrified ranges of tracks with their systems, before masts and contact wires (decision 0029, phase 3).
    ("electrification", "2.1", "sha256:45c09436f949eb966d091985b23378eb9736d0db4f2991957f78209f07dbf8a1"),
    // A switch with B1 its main track (decision 0029, phase 1), before core 5.0 had B1 its branch again.
    ("core", "4.0", "sha256:2dd6dee14dc22be99741f2b095a314185273f3662f809433d7639fe4c59f6abf"),
];

/// The hash the schema of a domain's version goes by: this build's, or one of a version before (`PAST`).
fn hash_of(name: &str, version: &str) -> Option<&'static str> {
    if domain_version(name) == Some(version) {
        return schema_hashes().get(name).map(String::as_str);
    }
    PAST.iter().find(|(d, v, _)| *d == name && *v == version).map(|(_, _, h)| *h)
}
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

/// An infra-data 2.0 document with its core 2.0 and civil 1.0 of an earlier schema
/// brought to their last: a stopping place a platform names is a
/// passenger stop (core's second), the named runs of tracks are the core's, no
/// longer civil's (core's third, civil's second), stages may have days (core's fourth).
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
        *v = with_schema(v, CIVIL_SECOND, Some(cols));
    }
    // Bridges, retaining walls and noise barriers only add: what was is as it was.
    if let Some(v) = civil.as_mut().filter(|v| schema(Some(v)).as_deref() == Some(CIVIL_SECOND)) {
        *v = with_schema(v, CIVIL_LAST, None);
    }
    if let Some(c) = core.as_mut() {
        if schema(Some(c)).as_deref() == Some(CORE_2_SECOND) {
            let mut cols = obj(c.get("collections").unwrap_or(&Value::Null));
            cols.insert("trackPaths".into(), runs.unwrap_or(json!([])));
            *c = with_schema(c, CORE_2_THIRD, Some(cols));
        }
        // Days on the stages and the stage link only add: what was is as it was.
        if schema(Some(c)).as_deref() == Some(CORE_2_THIRD) {
            *c = with_schema(c, CORE_2_LAST, None);
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
/// platform where there is none. The domains are 2.0's as it was last (`PAST`). A
/// domain of another schema is left as it was, for the check to name.
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
        d.insert("version".into(), json!("2.0"));
        d.insert("schema".into(), json!(SIGNALLING_2_LAST));
        d.insert("collections".into(), Value::Object(cols));
        signalling = Some(Value::Object(d));
    }
    core_cols.insert("stoppingPlaces".into(), Value::Array(sorted_by_id(layers::core_stopping_places(&stopping))));
    let layout = domains.get("layout");
    let layout_cols = layout.map(|l| obj(l.get("collections").unwrap_or(&Value::Null))).unwrap_or_default();
    core_cols.insert("trackPaths".into(), layout_cols.get("trackPaths").cloned().unwrap_or(json!([])));
    let mut c = obj(&core);
    c.insert("version".into(), json!("2.0"));
    c.insert("schema".into(), json!(CORE_2_LAST));
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
        d.insert("version".into(), json!("1.0"));
        d.insert("schema".into(), json!(CIVIL_LAST));
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
    out.insert("domains".into(), Value::Object(out_domains));
    Value::Object(out)
}

// ── Steps by number ─────────────────────────────────────────────────────────

/// One step of the format: the domains it changes, each from one version to the next, and what it does to a
/// document that holds them at the versions it starts from.
struct Step {
    domains: &'static [(&'static str, &'static str, &'static str)],
    apply: fn(&Value) -> Value,
}

/// The steps in their order. A step applies where a domain it names is at the version it starts from; the
/// others it names are then at theirs too, or the document is of no version this build reads.
const STEPS: &[Step] = &[
    Step { domains: LIFESPANS_BY_DAY, apply: lifespans_by_day },
    Step { domains: REFERENCES_RESOLVED, apply: references_resolved },
    Step { domains: WORDS_AND_NAMES, apply: words_and_names },
    Step { domains: CONTACT_WIRES, apply: contact_wires },
    Step { domains: SWITCH_PORTS, apply: switch_ports },
];

const LIFESPANS_BY_DAY: &[(&str, &str, &str)] =
    &[("core", "2.0", "3.0"), ("civil", "1.0", "2.0"), ("signalling", "2.0", "3.0"), ("electrification", "1.0", "2.0")];

const REFERENCES_RESOLVED: &[(&str, &str, &str)] =
    &[("core", "3.0", "3.1"), ("civil", "2.0", "2.1"), ("signalling", "3.0", "3.1"), ("electrification", "2.0", "2.1")];

const WORDS_AND_NAMES: &[(&str, &str, &str)] = &[("core", "3.1", "4.0"), ("civil", "2.1", "3.0"), ("signalling", "3.1", "4.0")];

const CONTACT_WIRES: &[(&str, &str, &str)] = &[("electrification", "2.1", "3.0")];

const SWITCH_PORTS: &[(&str, &str, &str)] = &[("core", "4.0", "5.0")];

fn version_of<'a>(doc: &'a Value, name: &str) -> Option<&'a str> {
    doc["domains"].get(name)?.get("version")?.as_str()
}

/// Lifespans by day (decision 0028): a lifespan named construction stages of `core.fields.stages` — the first
/// the existing state, the last the final one, each between them beginning on its day —, and states days now.
/// A `from` is the day its stage begins, a `to` the day before its stage does; the final stage is "open", as is
/// a stage the list lacks or one without a day; a `from` at the existing state is no statement. The list goes.
fn lifespans_by_day(doc: &Value) -> Value {
    let mut domains = obj(&doc["domains"]);
    let stages = list(&doc["domains"]["core"]["fields"], "stages");
    let first = stages.first().map(|s| js_string(s.get("id")));
    let day_of = |id: &str| -> Option<String> {
        let between = stages.get(1..stages.len().saturating_sub(1)).unwrap_or(&[]);
        let stage = between.iter().find(|s| s.get("id").and_then(Value::as_str) == Some(id))?;
        stage.get("date").and_then(Value::as_str).filter(|d| crate::stages::is_day(d)).map(str::to_string)
    };
    let by_day = |lifespan: &Value| -> Option<Value> {
        let mut out = Object::new();
        if let Some(id) = lifespan.get("from").and_then(Value::as_str).filter(|id| Some(*id) != first.as_deref()) {
            out.insert("from".into(), json!(day_of(id).unwrap_or_else(|| crate::stages::OPEN.into())));
        }
        if let Some(id) = lifespan.get("to").and_then(Value::as_str) {
            let day = day_of(id).and_then(|d| crate::stages::day_before(&d));
            out.insert("to".into(), json!(day.unwrap_or_else(|| crate::stages::OPEN.into())));
        }
        (!out.is_empty()).then_some(Value::Object(out))
    };
    for (name, _, to) in LIFESPANS_BY_DAY {
        let Some(Value::Object(domain)) = domains.get_mut(*name) else { continue };
        domain.insert("version".into(), json!(to));
        domain.insert("schema".into(), json!(hash_of(name, to)));
        if *name == "core"
            && let Some(Value::Object(fields)) = domain.get_mut("fields")
        {
            fields.shift_remove("stages");
        }
        let Some(Value::Object(cols)) = domain.get_mut("collections") else { continue };
        for items in cols.values_mut().filter_map(Value::as_array_mut) {
            for o in items.iter_mut().filter_map(Value::as_object_mut) {
                let Some(lifespan) = o.get("lifespan") else { continue };
                match by_day(lifespan) {
                    Some(l) => o.insert("lifespan".into(), l),
                    None => o.shift_remove("lifespan"),
                };
            }
        }
    }
    let mut out = obj(doc);
    out.insert("domains".into(), Value::Object(domains));
    Value::Object(out)
}

/// References that name something (decision 0028, phase C): a document of before could hold a reference to an
/// object that was not there — the tools reported it instead —; it is cut as the deletion of that object would
/// have cut it (`without_dangling`). The domains are at the step's versions after it.
fn references_resolved(doc: &Value) -> Value {
    let mut out = without_dangling(doc).0;
    let Some(Value::Object(domains)) = out.get_mut("domains") else { return out };
    for (name, _, to) in REFERENCES_RESOLVED {
        let Some(Value::Object(domain)) = domains.get_mut(*name) else { continue };
        domain.insert("version".into(), json!(to));
        domain.insert("schema".into(), json!(hash_of(name, to)));
    }
    out
}

/// Core 4.0 (decision 0029, phases 1 and 2): the kinds as words, the references ending in `Id`, no plane of an
/// element's own, a switch with `B1` its main track (`words`); a km line in one plane as runs (`km_lines`), every id
/// a UUID (`ids`). The domains are at the step's versions after it.
fn words_and_names(doc: &Value) -> Value {
    let mut out = doc.clone();
    let mut blobs = obj(&doc["blobs"]);
    let Some(Value::Object(domains)) = out.get_mut("domains") else { return out };
    crate::words::forward(domains);
    crate::words::swap_switch_ports(domains);
    // A km line in one plane, its runs' points as blobs; every id a UUID (decision 0029, phase 2).
    crate::core_format::km_lines_to_document(domains, &mut blobs);
    crate::ids::forward(domains);
    crate::core_format::sorted_collections(domains);
    for (name, _, to) in WORDS_AND_NAMES {
        let Some(Value::Object(domain)) = domains.get_mut(*name) else { continue };
        domain.insert("version".into(), json!(to));
        domain.insert("schema".into(), json!(hash_of(name, to)));
    }
    out.as_object_mut().expect("document").insert("blobs".into(), Value::Object(blobs));
    out
}

/// Electrification 3.0 (decision 0029, phase 3): an electrified range of a track with its system becomes a
/// contact wire of that voltage, carried at both ends of the range by a suspension point on a mast there — where
/// a wire runs follows from what carries it, so it runs where the range ran. Name and lifespan go with the wire,
/// the lifespan with the masts and points too; their ids are drawn from the range's.
fn contact_wires(doc: &Value) -> Value {
    let mut out = doc.clone();
    let Some(Value::Object(domain)) = out.get_mut("domains").and_then(|d| d.get_mut("electrification")) else { return out };
    let ranges = list(domain.get("collections").unwrap_or(&Value::Null), "electrifications");
    let (mut wires, mut masts, mut points) = (Vec::new(), Vec::new(), Vec::new());
    for r in &ranges {
        let id = js_string(r.get("id"));
        let at = r.get("at").cloned().unwrap_or(Value::Null);
        let mut wire = Object::new();
        wire.insert("id".into(), json!(id));
        set_opt(&mut wire, "name", r.get("name").cloned());
        wire.insert("voltage".into(), r.get("system").cloned().unwrap_or(Value::Null));
        set_opt(&mut wire, "lifespan", r.get("lifespan").cloned());
        wires.push(Value::Object(wire));
        for (end, station) in [("start", at.get("startStation")), ("end", at.get("endStation"))] {
            let mut place = Object::new();
            place.insert("anchor".into(), json!("station"));
            place.insert("trackId".into(), at.get("trackId").cloned().unwrap_or(Value::Null));
            place.insert("station".into(), station.cloned().unwrap_or(json!(0)));
            let mast_id = crate::canonical::derived_id(&format!("{id}:mast:{end}"));
            let mut mast = Object::new();
            mast.insert("id".into(), json!(mast_id));
            mast.insert("kind".into(), json!("other"));
            let mut mast_at = place.clone();
            set_opt(&mut mast_at, "side", at.get("side").cloned());
            mast.insert("at".into(), Value::Object(mast_at));
            set_opt(&mut mast, "lifespan", r.get("lifespan").cloned());
            masts.push(Value::Object(mast));
            let mut point = Object::new();
            point.insert("id".into(), json!(crate::canonical::derived_id(&format!("{id}:point:{end}"))));
            point.insert("mastId".into(), json!(mast_id));
            point.insert("at".into(), Value::Object(place));
            point.insert("contactWireId".into(), json!(id));
            set_opt(&mut point, "lifespan", r.get("lifespan").cloned());
            points.push(Value::Object(point));
        }
    }
    let mut cols = Object::new();
    cols.insert("contactWires".into(), Value::Array(sorted_by_id(wires)));
    cols.insert("masts".into(), Value::Array(sorted_by_id(masts)));
    cols.insert("suspensionPoints".into(), Value::Array(sorted_by_id(points)));
    cols.insert("sectionInsulators".into(), json!([]));
    domain.insert("collections".into(), Value::Object(cols));
    domain.insert("version".into(), json!("3.0"));
    domain.insert("schema".into(), json!(hash_of("electrification", "3.0")));
    out
}

/// Core 5.0 (decision 0029, phase 6): a switch's `B1` its branch and `B2` its main track again, as the suite always
/// had them and the OSRD export writes them.
fn switch_ports(doc: &Value) -> Value {
    let mut out = doc.clone();
    let Some(Value::Object(domains)) = out.get_mut("domains") else { return out };
    crate::words::swap_switch_ports(domains);
    for (name, _, to) in SWITCH_PORTS {
        let Some(Value::Object(domain)) = domains.get_mut(*name) else { continue };
        domain.insert("version".into(), json!(to));
        domain.insert("schema".into(), json!(hash_of(name, to)));
    }
    out
}

/// A reference of a domain's collection: the domain, and the reference as the schema walk gives it, its paths
/// within the domain's `{ fields, collections }`.
pub struct DomainReference {
    pub domain: String,
    pub reference: crate::schema::Reference,
}

/// Every reference a document's domains hold, by what their schemas mark (`x-target`).
pub fn references_of(doc: &Value) -> Vec<DomainReference> {
    let mut out = Vec::new();
    for (name, domain) in doc.get("domains").and_then(Value::as_object).into_iter().flatten() {
        if schemas().set.get(name).is_none() {
            continue;
        }
        let mut part = Object::new();
        part.insert("fields".into(), domain.get("fields").cloned().unwrap_or(json!({})));
        part.insert("collections".into(), domain.get("collections").cloned().unwrap_or(json!({})));
        for reference in schemas().set.references(name, &Value::Object(part)).unwrap_or_default() {
            out.push(DomainReference { domain: name.clone(), reference });
        }
    }
    out
}

/// The references of a document that name nothing: the target collection (`domain.collection`) has no object of
/// that id — or the document no such domain.
pub fn dangling(doc: &Value) -> Vec<DomainReference> {
    let mut ids: HashMap<String, HashSet<String>> = HashMap::new();
    references_of(doc)
        .into_iter()
        .filter(|r| {
            let target = r.reference.target.clone();
            let have = ids.entry(target.clone()).or_insert_with(|| {
                let (d, c) = target.split_once('.').unwrap_or(("", ""));
                list(&doc["domains"][d]["collections"], c).iter().filter_map(|o| o.get("id").map(|v| stringify(v))).collect()
            });
            !have.contains(&stringify(&json!(r.reference.id)))
        })
        .collect()
}

/// The steps of a JSON pointer, a number where it is one.
fn pointer_steps(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_string).collect()
}

/// Remove what `path` names in `value` — a member, or an item of a list.
fn remove_at(value: &mut Value, path: &[String]) {
    let Some((last, parents)) = path.split_last() else { return };
    let mut node = value;
    for p in parents {
        node = match node {
            Value::Object(o) => match o.get_mut(p) {
                Some(n) => n,
                None => return,
            },
            Value::Array(a) => match p.parse::<usize>().ok().and_then(|i| a.get_mut(i)) {
                Some(n) => n,
                None => return,
            },
            _ => return,
        };
    }
    match node {
        Value::Object(o) => {
            o.shift_remove(last);
        }
        Value::Array(a) => {
            if let Some(i) = last.parse::<usize>().ok().filter(|i| *i < a.len()) {
                a.remove(i);
            }
        }
        _ => {}
    }
}

/// The steps compared as a pointer orders them: a number by its value.
fn by_steps(a: &[String], b: &[String]) -> std::cmp::Ordering {
    for (x, y) in a.iter().zip(b) {
        let o = match (x.parse::<usize>(), y.parse::<usize>()) {
            (Ok(m), Ok(n)) => m.cmp(&n),
            _ => x.cmp(y),
        };
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    a.len().cmp(&b.len())
}

/// A document without the references that name nothing (`withoutDangling`): each cut where the schema lets it go
/// — an optional reference alone, an item of a list, else the object that requires it —, an object the cuts
/// leave short of what its schema asks taken as a whole, and on until nothing dangles: what was cut goes with
/// what named it, as a deletion takes along what is nothing without the object it took. Returns the document
/// and what was cut: [{ domain, path, target, id }], `path` the cut's within the domain.
pub fn without_dangling(doc: &Value) -> (Value, Vec<Value>) {
    let mut doc = doc.clone();
    let mut cut = Vec::new();
    for _ in 0..64 {
        let found = dangling(&doc);
        if found.is_empty() {
            break;
        }
        let mut by_domain: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for r in &found {
            cut.push(json!({ "domain": r.domain, "path": r.reference.cut, "target": r.reference.target, "id": r.reference.id }));
            by_domain.entry(r.domain.clone()).or_default().push(pointer_steps(&r.reference.cut));
        }
        for (name, mut paths) in by_domain {
            // An ancestor's cut takes its descendants; the deepest and last first, so the others stay where they are.
            paths.sort_by(|a, b| by_steps(b, a));
            paths.dedup();
            let kept: Vec<Vec<String>> = paths
                .iter()
                .filter(|p| !paths.iter().any(|q| q.len() < p.len() && p.starts_with(q)))
                .cloned()
                .collect();
            let Some(domain) = doc.get_mut("domains").and_then(|d| d.get_mut(&name)) else { continue };
            let mut part = json!({ "fields": domain.get("fields").cloned().unwrap_or(json!({})), "collections": domain.get("collections").cloned().unwrap_or(json!({})) });
            for p in &kept {
                remove_at(&mut part, p);
            }
            // An object a cut left short of its schema goes as a whole (a boundary without its last mark).
            loop {
                let short: Vec<Vec<String>> = check(&name, &part)
                    .iter()
                    .map(|b| pointer_steps(&b.path))
                    .filter(|p| p.len() >= 3 && p[0] == "collections")
                    .map(|p| p[..3].to_vec())
                    .collect();
                if short.is_empty() {
                    break;
                }
                let mut short = short;
                short.sort_by(|a, b| by_steps(b, a));
                short.dedup();
                for p in &short {
                    remove_at(&mut part, p);
                    cut.push(json!({ "domain": name, "path": format!("/{}", p.join("/")), "target": null, "id": null }));
                }
            }
            if let Some(o) = domain.as_object_mut() {
                o.insert("fields".into(), part["fields"].take());
                o.insert("collections".into(), part["collections"].take());
            }
        }
    }
    // An element that was a branch of a switch cut away is no branch any more: a branch names its switch.
    for track in doc["domains"]["core"]["collections"]["tracks"].as_array_mut().into_iter().flatten() {
        for el in track.get_mut("elements").and_then(Value::as_array_mut).into_iter().flatten() {
            if let Some(o) = el.as_object_mut().filter(|o| o.get("switchBranch") == Some(&Value::Bool(true)) && !o.contains_key("switchId")) {
                o.shift_remove("switchBranch");
            }
        }
    }
    (doc, cut)
}

/// A document of infra-data 2.0 brought forward step by step to the versions this build reads; refused where a
/// step finds its domains at versions that do not belong together, or of another schema than the version's.
fn stepped(doc: Value) -> Result<Value> {
    let mut doc = doc;
    for step in STEPS {
        let present: Vec<_> = step.domains.iter().filter(|(name, _, _)| doc["domains"].get(*name).is_some()).collect();
        if !present.iter().any(|(name, from, _)| version_of(&doc, name) == Some(*from)) {
            continue;
        }
        if let Some((name, _, _)) = present.iter().find(|(name, from, _)| version_of(&doc, name) != Some(*from)) {
            return Err(PayloadError::with(
                "unsupported_version",
                vec![format!("/domains/{name}: {} does not go with the other domains' versions", version_of(&doc, name).unwrap_or(""))],
            ));
        }
        let wrong: Vec<String> = present
            .iter()
            .filter(|(name, from, _)| doc["domains"][*name].get("schema").and_then(Value::as_str) != hash_of(name, from))
            .map(|(name, from, _)| format!("/domains/{name}/schema: is not the schema of {name} {from}"))
            .collect();
        if !wrong.is_empty() {
            return Err(PayloadError::with("invalid_payload", wrong));
        }
        doc = (step.apply)(&doc);
    }
    Ok(doc)
}

/// Whether version `a` ("N.M") is later than `b`.
fn later(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Option<(u32, u32)> {
        let (major, minor) = v.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    };
    matches!((parts(a), parts(b)), (Some(a), Some(b)) if a > b)
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

/// Check an infra-data document; returns it as this build reads it — an infra-data 1.0
/// one brought to 2.0, a domain of an earlier schema of 2.0 to 2.0's last, and from there
/// version by version —, or a PayloadError (`checkDocument`).
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
    // What the file carries is checked as it came: the schemas and the blobs against their hashes.
    for (member, what) in [("schemas", "another schema"), ("blobs", "other content")] {
        for (hash, value) in given.get(member).and_then(Value::as_object).into_iter().flatten() {
            if content_hash(value) != *hash {
                return Err(PayloadError::with("invalid_payload", vec![format!("/{member}/{hash}: holds {what}")]));
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
    let mut doc = stepped(doc)?;
    // A document brought forward that carried its schemas carries this build's.
    if doc != *given && given.get("schemas").is_some() {
        let carried = carried_schemas(&doc);
        doc.as_object_mut().expect("document").insert("schemas".into(), carried);
    }
    let mut found = Vec::new();
    if let Some(Value::Object(domains)) = doc.get("domains") {
        for (name, domain) in domains {
            let version = domain.get("version");
            // A domain this build does not know is kept as it came, and written back (rail-across-1.0).
            if domain_version(name).is_none() {
                continue;
            }
            if domain_version(name).map(|v| Value::String(v.into())).as_ref() != version {
                let newer = domain_version(name).zip(version.and_then(Value::as_str)).filter(|(ours, theirs)| later(theirs, ours));
                return Err(PayloadError::with(
                    "unsupported_version",
                    vec![match newer {
                        Some((ours, theirs)) => format!("/domains/{name}: {theirs} is newer than this program reads ({ours})"),
                        None => format!("/domains/{name}: {}", js_string(version)),
                    }],
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
                    for (i, o) in list.as_array().into_iter().flatten().enumerate() {
                        let id = o.get("id").map(stringify).unwrap_or_else(|| "undefined".into());
                        if !seen.insert(id) {
                            found.push(format!("/domains/{name}/collections/{cname}: holds {} twice", js_string(o.get("id"))));
                        }
                        // What the schema cannot say of a lifespan: its days are days, the first not after the last.
                        if let Some(breach) = o.get("lifespan").filter(|l| l.is_object()).and_then(crate::stages::lifespan_breach) {
                            found.push(format!("/domains/{name}/collections/{cname}/{i}/lifespan{breach}"));
                        }
                    }
                }
            }
        }
    }
    found.extend(switch_branches_without_switch(&doc));
    // What the schema cannot say of a reference: it names an object of its collection (decision 0028, phase C).
    if found.is_empty() {
        for r in dangling(&doc) {
            found.push(format!(
                "/domains/{}{}: names {}, which {} lacks",
                r.domain, r.reference.path, js_string(Some(&json!(r.reference.id))), r.reference.target
            ));
        }
    }
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

/// This build's schemas of the domains a document holds, by their hashes — what a file that travels carries.
fn carried_schemas(doc: &Value) -> Value {
    let mut carried = Object::new();
    for name in doc["domains"].as_object().into_iter().flat_map(|o| o.keys()) {
        if let (Some(h), Some(s)) = (schema_hashes().get(name.as_str()), schema_of(name)) {
            carried.insert(h.clone(), s.clone());
        }
    }
    Value::Object(carried)
}

/// The infra-data-2.0 document of a record (dehydrated), checked as the door checks
/// what comes in; `carry_schemas` puts the domains' schemas into it, `check:
/// false` leaves the check out (`toDocument`).
pub fn to_document(record: &Value, carry_schemas: bool, check: bool) -> Result<Value> {
    let mut doc = record_to_document(record, schema_hashes())?;
    if carry_schemas {
        let carried = carried_schemas(&doc);
        doc.as_object_mut().expect("document").insert("schemas".into(), carried);
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
