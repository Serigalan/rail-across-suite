// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The record the tools work on and the infra-data-2.0 document it is kept
//! as, both ways, nothing lost (`coreFormat.js`): the domain `core` and the
//! layers on it — `civil`, `signalling`, `electrification` (decision 0025) —,
//! locations as one value each, the planning status as a lifespan over two
//! construction stages, station names and codes as an operational point
//! register, named runs of tracks as the core's `trackPaths`, the bulk point arrays as
//! blobs named by their hash. The record keeps the platforms at its top and the
//! stopping places in its signalling, each naming its platform; the document
//! has the stopping places in the core and the platforms, naming them, in civil.

use std::collections::{HashMap, HashSet};

use rail_across_js::{Object, get, has, js_less, js_string, js_trim, strict_eq, without_null};
use serde_json::{Map, Value, json};

use crate::canonical::{content_hash, derived_id};
use crate::error::{PayloadError, Result};
use crate::layers;

/// The domains this build reads and writes, with their versions.
pub const DOMAIN_VERSIONS: &[(&str, &str)] = &[("core", "2.0"), ("civil", "1.0"), ("signalling", "2.0"), ("electrification", "1.0")];

/// The version of domain `name` this build reads, where it reads it.
pub fn domain_version(name: &str) -> Option<&'static str> {
    DOMAIN_VERSIONS.iter().find(|(d, _)| *d == name).map(|(_, v)| *v)
}

const CORE_FIELDS: &[&str] = &[
    "id",
    "title",
    "description",
    "creator",
    "createdAt",
    "crsSystem",
    "lineCategory",
    "gaugeProfile",
];
const CIVIL_FIELDS: &[&str] = &["planHeader", "image", "imageHash", "osrd"];
const CARRIED: &[&str] = &["signalling", "electrification"];
const COLLECTIONS: &[&str] = &[
    "tracks",
    "switches",
    "platforms",
    "endMarks",
    "routes",
    "kmLines",
    "axisSurveys",
    "referenceAxes",
    "operationalPoints",
];
const ALWAYS: &[&str] = &["tracks", "switches", "platforms"];

fn obj(v: &Value) -> Object {
    v.as_object().cloned().unwrap_or_default()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    get(v, key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// `pick(obj, keys)`: the members of `keys` that are not null, in that order.
fn pick(v: &Value, keys: &[&str]) -> Object {
    let mut out = Object::new();
    for k in keys {
        if let Some(x) = get(v, k) {
            out.insert((*k).into(), x.clone());
        }
    }
    out
}

/// A text, or nothing where it is empty (`text` of coreFormat: the value as it is).
fn text(v: Option<&Value>) -> Option<Value> {
    match v {
        None | Some(Value::Null) => None,
        Some(x) if js_trim(&js_string(Some(x))).is_empty() => None,
        Some(x) => Some(x.clone()),
    }
}

/// `{ ...a, ...b }` of two objects: a member of `b` takes the place `a` gave it.
fn spread(mut a: Object, b: Object) -> Object {
    for (k, v) in b {
        a.insert(k, v);
    }
    a
}

fn without(o: &Object, keys: &[&str]) -> Object {
    o.iter()
        .filter(|(k, _)| !keys.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn opt(v: Option<Value>) -> Value {
    v.unwrap_or(Value::Null)
}

// ── Construction stages ─────────────────────────────────────────────────────

/// The two stages of a project that has none of its own: existing and final, ids drawn from the project's.
pub fn default_stages(project_id: &str) -> Value {
    json!([
        { "id": derived_id(&format!("{project_id}\u{0}stage\u{0}existing")) },
        { "id": derived_id(&format!("{project_id}\u{0}stage\u{0}final")) },
    ])
}

pub(crate) fn lifespan_of(status: Option<&Value>, final_id: &str) -> Value {
    match status.and_then(Value::as_str) {
        Some("new") => json!({ "from": final_id }),
        Some("removal") => json!({ "to": final_id }),
        _ => Value::Null,
    }
}

fn status_of(lifespan: Option<&Value>, final_id: &str) -> Result<Value> {
    let Some(l) = lifespan.filter(|l| rail_across_js::truthy(Some(l))) else {
        return Ok(Value::Null);
    };
    let keys = l.as_object().map(|o| o.len()).unwrap_or(0);
    let fin = Value::String(final_id.to_string());
    if keys == 1 && strict_eq(l.get("from"), Some(&fin)) {
        return Ok(json!("new"));
    }
    if keys == 1 && strict_eq(l.get("to"), Some(&fin)) {
        return Ok(json!("removal"));
    }
    Err(PayloadError::new("stages_unsupported"))
}

// ── Operational points ──────────────────────────────────────────────────────

/// Which object of the record a station text comes from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Source {
    Track(usize),
    Platform(usize),
}

#[derive(Clone)]
struct Named {
    source: Source,
    name: Option<Value>,
    uic: Option<Value>,
    rl100: Option<Value>,
}

impl Named {
    fn field(&self, k: &str) -> Option<&Value> {
        match k {
            "uic" => self.uic.as_ref(),
            "rl100" => self.rl100.as_ref(),
            _ => self.name.as_ref(),
        }
    }
}

fn or_empty(v: Option<&Value>) -> String {
    v.map(|x| js_string(Some(x))).unwrap_or_default()
}

fn key_of(name: Option<&Value>, uic: Option<&Value>, rl100: Option<&Value>) -> String {
    match name {
        Some(n) => format!("name\u{0}{}", js_string(Some(n))),
        None => format!("code\u{0}{}\u{0}{}", or_empty(uic), or_empty(rl100)),
    }
}

/// The register a record's station texts make: every point, and the point
/// each track and platform names.
pub struct Register {
    pub points: Vec<Value>,
    of: HashMap<Source, String>,
}

impl Register {
    /// The id of the point a track or platform names, where it names one.
    pub fn point_of(&self, source: Source) -> Option<&str> {
        self.of.get(&source).map(String::as_str)
    }
}

/// The register the tracks' and platforms' station texts make, with the points
/// the record keeps of it (`stored`), and which point each names (`operationalRegister`).
pub fn operational_register(project_id: Option<&Value>, tracks: &[Value], platforms: &[Value], stored: &[Value]) -> Register {
    let pid = js_string(project_id);
    let mut named: Vec<Named> = Vec::new();
    for (i, t) in tracks.iter().enumerate() {
        named.push(Named {
            source: Source::Track(i),
            name: text(t.get("stationName")),
            uic: text(t.get("uicStation")),
            rl100: None,
        });
    }
    for (i, p) in platforms.iter().enumerate() {
        named.push(Named {
            source: Source::Platform(i),
            name: text(p.get("stationName")),
            uic: None,
            rl100: text(p.get("code")),
        });
    }
    named.retain(|e| e.name.is_some() || e.uic.is_some() || e.rl100.is_some());

    let mut group_order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut kept: HashMap<String, Value> = HashMap::new();
    let mut points: Map<String, Value> = Map::new();
    for p in stored {
        let key = key_of(text(p.get("name")).as_ref(), text(p.get("uic")).as_ref(), text(p.get("rl100")).as_ref());
        if kept.contains_key(&key) {
            points.insert(format!("id\u{0}{}", js_string(p.get("id"))), p.clone());
        } else {
            kept.insert(key.clone(), p.clone());
            group_order.push(key.clone());
            groups.insert(key, Vec::new());
        }
    }
    for (n, e) in named.iter().enumerate() {
        let key = key_of(e.name.as_ref(), e.uic.as_ref(), e.rl100.as_ref());
        if !groups.contains_key(&key) {
            group_order.push(key.clone());
            groups.insert(key.clone(), Vec::new());
        }
        groups.get_mut(&key).expect("group").push(n);
    }

    let derived = |own: &str, name: Option<Value>, rl100: Option<Value>, uic: Option<Value>| -> Value {
        let mut o = Object::new();
        o.insert("id".into(), Value::String(derived_id(&format!("{pid}\u{0}operationalPoint\u{0}{own}"))));
        o.insert("name".into(), opt(name));
        o.insert("rl100".into(), opt(rl100));
        o.insert("uic".into(), opt(uic));
        Value::Object(without_null(o))
    };
    let own_key = |key: &str, e: &Named| format!("{key}\u{0}{}\u{0}{}", or_empty(e.uic.as_ref()), or_empty(e.rl100.as_ref()));

    let mut of: HashMap<Source, String> = HashMap::new();
    for key in &group_order {
        let entries: Vec<&Named> = groups[key].iter().map(|&n| &named[n]).collect();
        let distinct = |k: &str| -> Vec<String> {
            let mut seen: Vec<String> = Vec::new();
            for e in &entries {
                if let Some(v) = e.field(k) {
                    let s = js_string(Some(v));
                    if !seen.contains(&s) {
                        seen.push(s);
                    }
                }
            }
            seen
        };
        if let Some(keep) = kept.get(key) {
            let agreed = |k: &str| -> Option<Value> {
                let d = distinct(k);
                if d.len() == 1 { Some(Value::String(d[0].clone())) } else { None }
            };
            let mut point = Object::new();
            point.insert("id".into(), opt(keep.get("id").cloned()));
            point.insert("name".into(), opt(keep.get("name").cloned()));
            point.insert("rl100".into(), opt(get(keep, "rl100").cloned().or_else(|| agreed("rl100"))));
            point.insert("uic".into(), opt(get(keep, "uic").cloned().or_else(|| agreed("uic"))));
            let point = Value::Object(without_null(point));
            points.insert(key.clone(), point.clone());
            for e in &entries {
                let fits = ["rl100", "uic"]
                    .iter()
                    .all(|k| e.field(k).is_none() || js_string(e.field(k)) == js_string(point.get(*k)));
                if fits {
                    of.insert(e.source, js_string(point.get("id")));
                    continue;
                }
                let own = own_key(key, e);
                if !points.contains_key(&own) {
                    points.insert(own.clone(), derived(&own, e.name.clone(), e.rl100.clone(), e.uic.clone()));
                }
                of.insert(e.source, js_string(points[&own].get("id")));
            }
            continue;
        }
        let agree = distinct("uic").len() <= 1 && distinct("rl100").len() <= 1;
        for e in &entries {
            let own = if agree { key.clone() } else { own_key(key, e) };
            if !points.contains_key(&own) {
                let from: Vec<&Named> = if agree { entries.clone() } else { vec![e] };
                let rl100 = from.iter().find(|x| x.rl100.is_some()).and_then(|x| x.rl100.clone());
                let uic = from.iter().find(|x| x.uic.is_some()).and_then(|x| x.uic.clone());
                points.insert(own.clone(), derived(&own, e.name.clone(), rl100, uic));
            }
            of.insert(e.source, js_string(points[&own].get("id")));
        }
    }
    // A point kept under a name that the names make anew is the one they make.
    let mut unique: Map<String, Value> = Map::new();
    for p in points.values() {
        unique.insert(js_string(p.get("id")), p.clone());
    }
    Register {
        points: unique.into_iter().map(|(_, v)| v).collect(),
        of,
    }
}

/// What the record keeps of the register (`keptPoints`).
fn kept_points(project_id: Option<&Value>, tracks: &[Value], platforms: &[Value], points: &[Value], named: &HashSet<String>) -> Vec<Value> {
    let same = |a: &Value, b: Option<&Value>| -> bool {
        let Some(b) = b else { return false };
        ["name", "rl100", "uic"].iter().all(|k| {
            let x = get(a, k).cloned().unwrap_or(Value::Null);
            let y = get(b, k).cloned().unwrap_or(Value::Null);
            strict_eq(Some(&x), Some(&y))
        })
    };
    let made_with = |kept: &[Value]| -> HashMap<String, Value> {
        operational_register(project_id, tracks, platforms, kept)
            .points
            .into_iter()
            .map(|p| (js_string(p.get("id")), p))
            .collect()
    };
    let made = made_with(&[]);
    let id_of = |p: &Value| js_string(p.get("id"));
    let mut kept: Vec<Value> = points
        .iter()
        .filter(|p| named.contains(&id_of(p)) || !same(p, made.get(&id_of(p))))
        .cloned()
        .collect();
    let candidates: Vec<Value> = kept.iter().filter(|k| !named.contains(&id_of(k))).cloned().collect();
    for p in candidates {
        let others: Vec<Value> = kept.iter().filter(|k| **k != p).cloned().collect();
        if same(&p, made_with(&others).get(&id_of(&p))) {
            kept = others;
        }
    }
    kept.iter()
        .map(|p| {
            let mut o = Object::new();
            for k in ["id", "name", "rl100", "uic"] {
                o.insert(k.into(), opt(p.get(k).cloned()));
            }
            Value::Object(without_null(o))
        })
        .collect()
}

// ── The design speed ────────────────────────────────────────────────────────

fn renamed(el: &Value, from: &str, to: &str) -> Value {
    if !has(el, from) {
        return el.clone();
    }
    let mut o = obj(el);
    let v = o.shift_remove(from).unwrap_or(Value::Null);
    o.insert(to.into(), v);
    Value::Object(o)
}

// ── Record → document ───────────────────────────────────────────────────────

pub(crate) fn by_id(list: &mut [Value]) {
    list.sort_by(|a, b| {
        if js_less(a.get("id"), b.get("id")) {
            std::cmp::Ordering::Less
        } else if js_less(b.get("id"), a.get("id")) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
}

/// The infra-data-2.0 document of a record (dehydrated), every collection sorted by id (`recordToDocument`).
pub fn record_to_document(record: &Value, schema_hashes: &HashMap<&str, String>) -> Result<Value> {
    let known: HashSet<&str> = CORE_FIELDS.iter().chain(CIVIL_FIELDS).chain(COLLECTIONS).chain(CARRIED).copied().collect();
    let unknown: Vec<String> = record
        .as_object()
        .map(|o| o.keys().filter(|k| !known.contains(k.as_str())).cloned().collect())
        .unwrap_or_default();
    if !record.is_object() || get(record, "id").is_none() {
        return Err(PayloadError::with("invalid_payload", vec!["/: lacks id".into()]));
    }
    if !unknown.is_empty() {
        return Err(PayloadError::with(
            "invalid_payload",
            unknown.iter().map(|k| format!("/{k}: is not a field of the record")).collect(),
        ));
    }

    let pid = js_string(record.get("id"));
    let stages = default_stages(&pid);
    let final_id = js_string(stages[1].get("id"));
    let tracks = list(record, "tracks");
    let platforms = list(record, "platforms");
    let register = operational_register(record.get("id"), tracks, platforms, list(record, "operationalPoints"));
    let mut blobs = Object::new();
    let mut blob = |value: &Value| -> Value {
        let hash = content_hash(value);
        blobs.insert(hash.clone(), value.clone());
        Value::String(hash)
    };
    let located = |source: Option<Source>, status: Option<&Value>, extra: Object| -> Value {
        let mut o = extra;
        o.insert(
            "operationalPoint".into(),
            source
                .and_then(|s| register.point_of(s))
                .map(|s| Value::String(s.into()))
                .unwrap_or(Value::Null),
        );
        o.insert("lifespan".into(), lifespan_of(status, &final_id));
        Value::Object(without_null(o))
    };

    let core_tracks: Vec<Value> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut rest = without(&obj(t), &["status", "stationName", "uicStation"]);
            if let Some(Value::Array(els)) = rest.get("elements").filter(|e| rail_across_js::truthy(Some(e))) {
                let els: Vec<Value> = els.iter().map(|el| renamed(el, "speed", "designSpeed")).collect();
                rest.insert("elements".into(), Value::Array(els));
            }
            located(Some(Source::Track(i)), t.get("status"), without_null(rest))
        })
        .collect();
    let switches: Vec<Value> = list(record, "switches")
        .iter()
        .map(|sw| {
            let mut o = Object::new();
            o.insert("id".into(), opt(sw.get("switchId").cloned()));
            for (k, v) in without(&obj(sw), &["switchId", "status"]) {
                o.insert(k, v);
            }
            o.insert("lifespan".into(), lifespan_of(sw.get("status"), &final_id));
            Value::Object(without_null(o))
        })
        .collect();
    let core_platforms: Vec<Value> = platforms
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut at = Object::new();
            at.insert("anchor".into(), json!("station"));
            for k in ["trackId", "startStation", "endStation", "side"] {
                at.insert(k.into(), opt(p.get(k).cloned()));
            }
            let rest = without(&obj(p), &["trackId", "startStation", "endStation", "side", "status", "stationName", "code"]);
            let mut extra = Object::new();
            extra.insert("at".into(), Value::Object(without_null(at)));
            located(Some(Source::Platform(i)), p.get("status"), spread(extra, without_null(rest)))
        })
        .collect();
    let end_marks: Vec<Value> = list(record, "endMarks")
        .iter()
        .map(|m| {
            let rest = without(&obj(m), &["trackId", "endpoint", "status"]);
            let mut at = Object::new();
            at.insert("trackId".into(), opt(m.get("trackId").cloned()));
            at.insert("endpoint".into(), opt(m.get("endpoint").cloned()));
            let mut extra = without_null(rest);
            extra.insert("at".into(), Value::Object(without_null(at)));
            located(None, m.get("status"), extra)
        })
        .collect();

    // The stopping places come out of the record's signalling into the core; a
    // boundary marked by a platform is marked by the stopping place at it, and the
    // platforms (civil) name the stopping places at them.
    let mut signalling = get(record, "signalling").map(obj);
    let mut sig_cols = signalling
        .as_ref()
        .map(|d| obj(d.get("collections").unwrap_or(&Value::Null)))
        .unwrap_or_default();
    let mut stopping: Vec<Value> = sig_cols.get("stoppingPlaces").and_then(Value::as_array).cloned().unwrap_or_default();
    layers::boundaries_by_stopping_place(&mut sig_cols, &mut stopping, &core_platforms);
    sig_cols.shift_remove("stoppingPlaces");
    let civil_platforms = layers::platforms_naming(&core_platforms, &stopping);

    let mut core_fields = pick(record, CORE_FIELDS);
    core_fields.insert("stages".into(), stages);
    let mut core_cols = Object::new();
    core_cols.insert("tracks".into(), Value::Array(core_tracks));
    core_cols.insert("switches".into(), Value::Array(switches));
    core_cols.insert("endMarks".into(), Value::Array(end_marks));
    core_cols.insert("kmLines".into(), Value::Array(list(record, "kmLines").to_vec()));
    core_cols.insert("operationalPoints".into(), Value::Array(register.points.clone()));
    core_cols.insert("stoppingPlaces".into(), Value::Array(layers::core_stopping_places(&stopping)));

    let track_paths: Vec<Value> = list(record, "routes")
        .iter()
        .map(|r| {
            let mut o = without(&obj(r), &["trackIds"]);
            o.insert("path".into(), json!({ "trackIds": get(r, "trackIds").cloned().unwrap_or(json!([])) }));
            Value::Object(without_null(o))
        })
        .collect();
    core_cols.insert("trackPaths".into(), Value::Array(track_paths));
    let mut with_blob = |items: &[Value]| -> Vec<Value> {
        items
            .iter()
            .map(|s| {
                let mut o = obj(s);
                let points = s.get("points").cloned().unwrap_or(Value::Null);
                o.insert("points".into(), blob(&points));
                Value::Object(o)
            })
            .collect()
    };
    let axis_surveys = with_blob(list(record, "axisSurveys"));
    let reference_axes = with_blob(list(record, "referenceAxes"));

    let hash = |d: &str| schema_hashes.get(d).cloned().map(Value::String).unwrap_or(Value::Null);
    let layer = |name: &str, fields: Object, collections: Object| -> Value {
        let mut o = Object::new();
        o.insert("version".into(), json!(domain_version(name)));
        o.insert("schema".into(), hash(name));
        o.insert("dependsOn".into(), json!(["core"]));
        o.insert("fields".into(), Value::Object(fields));
        o.insert("collections".into(), Value::Object(collections));
        Value::Object(o)
    };
    let mut core = Object::new();
    core.insert("version".into(), json!(domain_version("core")));
    core.insert("schema".into(), hash("core"));
    core.insert("fields".into(), Value::Object(core_fields));
    core.insert("collections".into(), Value::Object(core_cols));
    let mut domains = Object::new();
    domains.insert("core".into(), Value::Object(core));

    // A layer only where the record holds anything of it: a document is the core
    // and what is planned on it, every layer may be missing.
    let civil_fields = pick(record, CIVIL_FIELDS);
    let mut civil_cols = Object::new();
    civil_cols.insert("platforms".into(), Value::Array(civil_platforms));
    civil_cols.insert("axisSurveys".into(), Value::Array(axis_surveys));
    civil_cols.insert("referenceAxes".into(), Value::Array(reference_axes));
    if layers::holds_anything(&civil_fields, &civil_cols) {
        domains.insert("civil".into(), layer("civil", civil_fields, civil_cols));
    }
    if let Some(d) = signalling.take() {
        let fields = obj(d.get("fields").unwrap_or(&Value::Null));
        if layers::holds_anything(&fields, &sig_cols) {
            for name in layers::SIGNALLING_COLLECTIONS {
                if !sig_cols.contains_key(*name) {
                    sig_cols.insert((*name).into(), json!([]));
                }
            }
            domains.insert("signalling".into(), layer("signalling", fields, sig_cols));
        }
    }
    if let Some(d) = get(record, "electrification") {
        let fields = obj(d.get("fields").unwrap_or(&Value::Null));
        let mut cols = obj(d.get("collections").unwrap_or(&Value::Null));
        if layers::holds_anything(&fields, &cols) {
            if !cols.contains_key("electrifications") {
                cols.insert("electrifications".into(), json!([]));
            }
            domains.insert("electrification".into(), layer("electrification", fields, cols));
        }
    }
    for domain in domains.values_mut() {
        if let Some(Value::Object(cols)) = domain.get_mut("collections") {
            for v in cols.values_mut() {
                if let Value::Array(items) = v {
                    by_id(items);
                }
            }
        }
    }
    Ok(json!({ "format": crate::door::FORMAT, "version": crate::door::VERSION, "domains": Value::Object(domains), "blobs": Value::Object(blobs) }))
}

// ── Document → record ───────────────────────────────────────────────────────

/// The record an infra-data-2.0 document holds — the inverse of record_to_document (`documentToRecord`).
pub fn document_to_record(doc: &Value) -> Result<Value> {
    let core = &doc["domains"]["core"];
    let empty_civil = json!({ "fields": {}, "collections": {} });
    let civil = get(&doc["domains"], "civil").unwrap_or(&empty_civil);
    let stages = core["fields"]["stages"].as_array().cloned().unwrap_or_default();
    if stages.len() != 2 {
        return Err(PayloadError::new("stages_unsupported"));
    }
    let final_id = js_string(stages[1].get("id"));
    let c = &core["collections"];
    let mut points: HashMap<String, Value> = HashMap::new();
    for p in list(c, "operationalPoints") {
        points.insert(js_string(p.get("id")), p.clone());
    }
    let point_of = |o: &Value| -> Result<Value> {
        match get(o, "operationalPoint") {
            None => Ok(json!({})),
            Some(id) => points
                .get(&js_string(Some(id)))
                .cloned()
                .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/operationalPoints: lacks {}", js_string(Some(id)))])),
        }
    };
    let blobs = doc.get("blobs").cloned().unwrap_or(json!({}));
    let blob = |hash: Option<&Value>| -> Result<Value> {
        let h = js_string(hash);
        blobs
            .get(&h)
            .cloned()
            .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/blobs: lacks {h}")]))
    };
    let status = |o: &Value| status_of(o.get("lifespan"), &final_id);
    let l = civil.get("collections").cloned().unwrap_or(json!({}));

    let mut tracks = Vec::new();
    for t in list(c, "tracks") {
        let p = point_of(t)?;
        let mut rest = without(&obj(t), &["lifespan", "operationalPoint"]);
        if let Some(Value::Array(els)) = rest.get("elements").filter(|e| rail_across_js::truthy(Some(e))) {
            let els: Vec<Value> = els.iter().map(|el| renamed(el, "designSpeed", "speed")).collect();
            rest.insert("elements".into(), Value::Array(els));
        }
        rest.insert("stationName".into(), opt(p.get("name").cloned()));
        rest.insert("uicStation".into(), opt(p.get("uic").cloned()));
        rest.insert("status".into(), status(t)?);
        tracks.push(Value::Object(without_null(rest)));
    }
    let mut switches = Vec::new();
    for sw in list(c, "switches") {
        let mut o = Object::new();
        o.insert("switchId".into(), opt(sw.get("id").cloned()));
        for (k, v) in without(&obj(sw), &["id", "lifespan"]) {
            o.insert(k, v);
        }
        o.insert("status".into(), status(sw)?);
        switches.push(Value::Object(without_null(o)));
    }
    let civil_platforms = list(&l, "platforms");
    let mut platforms = Vec::new();
    for pf in civil_platforms {
        let p = point_of(pf)?;
        let mut o = without(&obj(pf), &["at", "lifespan", "operationalPoint", "stoppingPlaceIds"]);
        for (k, v) in without(&obj(pf.get("at").unwrap_or(&Value::Null)), &["anchor"]) {
            o.insert(k, v);
        }
        o.insert("stationName".into(), opt(p.get("name").cloned()));
        o.insert("code".into(), opt(p.get("rl100").cloned()));
        o.insert("status".into(), status(pf)?);
        platforms.push(Value::Object(without_null(o)));
    }
    let mut end_marks = Vec::new();
    for m in list(c, "endMarks") {
        let mut o = without(&obj(m), &["at", "lifespan"]);
        for (k, v) in obj(m.get("at").unwrap_or(&Value::Null)) {
            o.insert(k, v);
        }
        o.insert("status".into(), status(m)?);
        end_marks.push(Value::Object(without_null(o)));
    }
    let routes: Vec<Value> = list(c, "trackPaths")
        .iter()
        .map(|r| {
            let mut o = without(&obj(r), &["path"]);
            if let Some(ids) = r.get("path").and_then(|p| p.get("trackIds")) {
                o.insert("trackIds".into(), ids.clone());
            }
            Value::Object(o)
        })
        .collect();
    let with_blob = |items: &[Value]| -> Result<Vec<Value>> {
        let mut out = Vec::new();
        for s in items {
            let mut o = obj(s);
            o.insert("points".into(), blob(s.get("points"))?);
            out.push(Value::Object(o));
        }
        Ok(out)
    };
    let axis_surveys = with_blob(list(&l, "axisSurveys"))?;
    let reference_axes = with_blob(list(&l, "referenceAxes"))?;

    let sc = doc["domains"]
        .get("signalling")
        .and_then(|s| s.get("collections"))
        .cloned()
        .unwrap_or(json!({}));
    let stopping = layers::record_stopping_places(list(c, "stoppingPlaces"), civil_platforms);
    let mut named = HashSet::new();
    for o in list(&sc, "signals").iter().chain(list(&sc, "stationBoundaries")) {
        if let Some(id) = o.get("operationalPointId").filter(|v| rail_across_js::truthy(Some(v))) {
            named.insert(js_string(Some(id)));
        }
    }
    let operational_points = kept_points(core["fields"].get("id"), &tracks, &platforms, list(c, "operationalPoints"), &named);

    let mut collections: Vec<(&str, Vec<Value>)> = vec![
        ("tracks", tracks),
        ("switches", switches),
        ("platforms", platforms),
        ("endMarks", end_marks),
        ("routes", routes),
        ("kmLines", list(c, "kmLines").to_vec()),
        ("axisSurveys", axis_surveys),
        ("referenceAxes", reference_axes),
        ("operationalPoints", operational_points),
    ];
    let core_fields = without(&obj(&core["fields"]), &["stages"]);
    let mut record = spread(core_fields, obj(civil.get("fields").unwrap_or(&Value::Null)));
    for name in COLLECTIONS {
        let pos = collections.iter().position(|(n, _)| n == name).expect("collection");
        let (_, items) = std::mem::take(&mut collections[pos]);
        if ALWAYS.contains(name) || !items.is_empty() {
            record.insert((*name).into(), Value::Array(items));
        }
    }
    for name in CARRIED {
        let d = doc["domains"].get(name);
        // The record's signalling holds the stopping places, which the document keeps in the core.
        let with_stopping = *name == "signalling" && !stopping.is_empty();
        if d.is_none() && !with_stopping {
            continue;
        }
        let mut o = Object::new();
        o.insert("fields".into(), d.and_then(|d| d.get("fields")).cloned().unwrap_or(json!({})));
        let mut cols = obj(d.and_then(|d| d.get("collections")).unwrap_or(&Value::Null));
        if *name == "signalling" {
            cols.insert("stoppingPlaces".into(), Value::Array(stopping.clone()));
        }
        o.insert("collections".into(), Value::Object(cols));
        record.insert((*name).into(), Value::Object(o));
    }
    Ok(Value::Object(record))
}
