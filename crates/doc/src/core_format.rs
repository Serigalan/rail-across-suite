// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The record the tools work on and the infra-data-2.0 document it is kept
//! as, both ways, nothing lost (`coreFormat.js`): the domain `core` and the
//! layers on it — `civil`, `signalling`, `electrification` (decision 0008) —,
//! locations as one value each, a lifespan and an operational point as the
//! object states them (decision 0006), named runs of tracks as the core's
//! `trackPaths`, the bulk point arrays as blobs named by their hash. The record keeps the platforms at its top and the
//! stopping places in its signalling, each naming its platform; the document
//! has the stopping places in the core and the platforms, naming them, in civil.

use std::collections::{HashMap, HashSet};

use rail_across_js::{Object, get, has, js_less, js_string, js_trim, without_null};
use serde_json::{Map, Value, json};

use crate::canonical::{content_hash, derived_id};
use crate::error::{PayloadError, Result};
use crate::layers;

/// The domains this build reads and writes, with their versions.
pub const DOMAIN_VERSIONS: &[(&str, &str)] = &[("core", "6.0"), ("civil", "3.0"), ("signalling", "4.0"), ("electrification", "3.0")];

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
    "stoppingPlaces",
    "bridges",
    "retainingWalls",
    "noiseBarriers",
];
/// The civil structures, kept in the record as the document has them (decision 0008).
pub const STRUCTURES: &[&str] = &["bridges", "retainingWalls", "noiseBarriers"];
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

/// A record whose tracks and platforms state their station as text — what an
/// importer reads from a file that knows no operational points, or a record of
/// before decision 0006 — with the points those texts make (`withStationPoints`):
/// `operationalPoints` the register, each track and platform its `operationalPointId`,
/// the texts gone — one that states its station as no text names none. The points
/// the record has are kept, and a text names the one of its name and codes. A record
/// without such texts comes back as it was.
pub fn with_station_points(record: &Value) -> Value {
    let tracks = list(record, "tracks");
    let platforms = list(record, "platforms");
    let texts = |o: &Value, keys: &[&str]| keys.iter().any(|k| o.get(*k).is_some());
    if !tracks.iter().any(|t| texts(t, &["stationName", "uicStation"])) && !platforms.iter().any(|p| texts(p, &["stationName", "code"])) {
        return record.clone();
    }
    let register = operational_register(record.get("id"), tracks, platforms, list(record, "operationalPoints"));
    let pointed = |o: &Value, source: Source, drop: &[&str]| -> Value {
        if !texts(o, drop) {
            return o.clone();
        }
        let mut rest = without(&obj(o), drop);
        rest.shift_remove("operationalPointId");
        if let Some(id) = register.point_of(source) {
            rest.insert("operationalPointId".into(), json!(id));
        }
        Value::Object(rest)
    };
    let mut out = obj(record);
    let ts: Vec<Value> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| pointed(t, Source::Track(i), &["stationName", "uicStation"]))
        .collect();
    let ps: Vec<Value> = platforms
        .iter()
        .enumerate()
        .map(|(i, p)| pointed(p, Source::Platform(i), &["stationName", "code"]))
        .collect();
    out.insert("tracks".into(), Value::Array(ts));
    if record.get("platforms").is_some() {
        out.insert("platforms".into(), Value::Array(ps));
    }
    out.insert("operationalPoints".into(), Value::Array(register.points));
    Value::Object(out)
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

pub fn by_id(list: &mut [Value]) {
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
    let (mut domains, mut blobs) = record_domains(record, schema_hashes)?;
    km_lines_to_document(&mut domains, &mut blobs)?;
    // A record of before — a working copy, a test's fixture — may still hold the words and names of before or an id
    // that is no UUID: the door takes them on the way in (decision 0006).
    crate::words::forward(&mut domains);
    crate::ids::forward(&mut domains);
    tracks_by_column(&mut domains)?;
    sorted_collections(&mut domains);
    // The domains this build does not know, as they came (rail-across-1.0): written back unchanged.
    for (name, domain) in get(record, "keptDomains").and_then(Value::as_object).into_iter().flatten() {
        if domain_version(name).is_none() {
            domains.insert(name.clone(), domain.clone());
        }
    }
    Ok(json!({ "format": crate::door::FORMAT, "version": crate::door::VERSION, "domains": Value::Object(domains), "blobs": Value::Object(blobs) }))
}

/// The tracks of a document's domains with their plan by column, as core 6.0 states it (`alignment::to_columns`;
/// decision 0006): their elements chained from the first start point. Refused where an element ends more than
/// 1 mm from where it is stated, naming each such track and element.
pub(crate) fn tracks_by_column(domains: &mut Object) -> Result<()> {
    let Some(Value::Array(tracks)) = domains.get_mut("core").and_then(|c| c.get_mut("collections")).and_then(|c| c.get_mut("tracks")) else {
        return Ok(());
    };
    let mut apart = Vec::new();
    for (i, t) in tracks.iter_mut().enumerate() {
        let Some(track) = t.as_object_mut() else { continue };
        let Some(rows) = track.shift_remove("elements") else { continue };
        match crate::alignment::to_columns(rows.as_array().map(Vec::as_slice).unwrap_or(&[])) {
            Ok(Some(columns)) => {
                track.insert("horizontalAlignment".into(), columns);
            }
            Ok(None) => {}
            Err(found) => {
                let name = js_string(track.get("name").or(track.get("id")));
                apart.extend(found.into_iter().map(|f| format!("/domains/core/collections/tracks/{i}: {name}, {f}")));
            }
        }
    }
    if apart.is_empty() {
        Ok(())
    } else {
        Err(PayloadError::with("invalid_payload", apart))
    }
}

/// The runs of a document's km lines by column, as core 6.0 states them, where core 5.0 gave their design
/// elements by row (`alignment::to_km_columns`). Refused where a run's elements do not hold together.
pub(crate) fn km_runs_by_column(domains: &mut Object) -> Result<()> {
    let Some(Value::Array(lines)) = domains
        .get_mut("core")
        .and_then(|c| c.get_mut("collections"))
        .and_then(|c| c.get_mut("kmLines"))
    else {
        return Ok(());
    };
    let mut apart = Vec::new();
    for (i, line) in lines.iter_mut().enumerate() {
        for (k, run) in line.get_mut("runs").and_then(Value::as_array_mut).into_iter().flatten().enumerate() {
            let Some(o) = run.as_object_mut() else { continue };
            let Some(rows) = o.shift_remove("elements") else { continue };
            match crate::alignment::to_km_columns(rows.as_array().map(Vec::as_slice).unwrap_or(&[])) {
                Ok(Some(columns)) => {
                    o.insert("horizontalAlignment".into(), columns);
                }
                Ok(None) => {}
                Err(found) => apart.extend(found.into_iter().map(|f| format!("/domains/core/collections/kmLines/{i}/runs/{k}: {f}"))),
            }
        }
    }
    if apart.is_empty() {
        Ok(())
    } else {
        Err(PayloadError::with("invalid_payload", apart))
    }
}

/// The km lines of a record's domains as core 6.0 states them: in one plane, their runs by column where the
/// record gives their design, else their points as blobs (decision 0006); one without a point is no line.
/// Refused where a design's elements do not hold together.
pub(crate) fn km_lines_to_document(domains: &mut Object, blobs: &mut Object) -> Result<()> {
    let Some(Value::Array(lines)) = domains
        .get_mut("core")
        .and_then(|c| c.get_mut("collections"))
        .and_then(|c| c.get_mut("kmLines"))
    else {
        return Ok(());
    };
    let mut blob = |value: &Value| -> Value {
        let hash = content_hash(value);
        blobs.insert(hash.clone(), value.clone());
        Value::String(hash)
    };
    let mut out = Vec::with_capacity(lines.len());
    for line in lines.iter() {
        let line = crate::km_lines::to_document(line, &mut blob)?;
        if line.get("epsg").is_some() {
            out.push(line);
        }
    }
    *lines = out;
    Ok(())
}

/// The domains of a record's document and its blobs, in the words, names and ids the record keeps, every
/// collection sorted by id: what `record_to_document` states as core 4.0 does.
pub fn record_domains(record: &Value, schema_hashes: &HashMap<&str, String>) -> Result<(Object, Object)> {
    let known: HashSet<&str> = CORE_FIELDS
        .iter()
        .chain(CIVIL_FIELDS)
        .chain(COLLECTIONS)
        .chain(CARRIED)
        .chain(&["keptDomains"])
        .copied()
        .collect();
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

    let tracks = list(record, "tracks");
    let platforms = list(record, "platforms");
    let mut blobs = Object::new();
    let mut blob = |value: &Value| -> Value {
        let hash = content_hash(value);
        blobs.insert(hash.clone(), value.clone());
        Value::String(hash)
    };
    let core_tracks: Vec<Value> = tracks
        .iter()
        .map(|t| {
            let mut rest = obj(t);
            if let Some(Value::Array(els)) = rest.get("elements").filter(|e| rail_across_js::truthy(Some(e))) {
                let els: Vec<Value> = els.iter().map(|el| renamed(el, "speed", "designSpeed")).collect();
                rest.insert("elements".into(), Value::Array(els));
            }
            Value::Object(without_null(rest))
        })
        .collect();
    let switches: Vec<Value> = list(record, "switches")
        .iter()
        .map(|sw| {
            let mut o = Object::new();
            o.insert("id".into(), opt(sw.get("switchId").cloned()));
            for (k, v) in without(&obj(sw), &["switchId"]) {
                o.insert(k, v);
            }
            Value::Object(without_null(o))
        })
        .collect();
    let core_platforms: Vec<Value> = platforms
        .iter()
        .map(|p| {
            let mut at = Object::new();
            at.insert("anchor".into(), json!("station"));
            for k in ["trackId", "startStation", "endStation", "side"] {
                at.insert(k.into(), opt(p.get(k).cloned()));
            }
            let rest = without(&obj(p), &["trackId", "startStation", "endStation", "side"]);
            let mut extra = Object::new();
            extra.insert("at".into(), Value::Object(without_null(at)));
            Value::Object(spread(extra, without_null(rest)))
        })
        .collect();
    let end_marks: Vec<Value> = list(record, "endMarks")
        .iter()
        .map(|m| {
            let rest = without(&obj(m), &["trackId", "endpoint"]);
            let mut at = Object::new();
            at.insert("trackId".into(), opt(m.get("trackId").cloned()));
            at.insert("endpoint".into(), opt(m.get("endpoint").cloned()));
            let mut extra = without_null(rest);
            extra.insert("at".into(), Value::Object(without_null(at)));
            Value::Object(extra)
        })
        .collect();

    // The stopping places are the core's, the platforms (civil) naming those at them, as in the record
    // (decision 0006).
    let mut signalling = get(record, "signalling").map(obj);
    let mut sig_cols = signalling
        .as_ref()
        .map(|d| obj(d.get("collections").unwrap_or(&Value::Null)))
        .unwrap_or_default();
    let civil_platforms = core_platforms;

    let core_fields = pick(record, CORE_FIELDS);
    let mut core_cols = Object::new();
    core_cols.insert("tracks".into(), Value::Array(core_tracks));
    core_cols.insert("switches".into(), Value::Array(switches));
    core_cols.insert("endMarks".into(), Value::Array(end_marks));
    core_cols.insert("kmLines".into(), Value::Array(list(record, "kmLines").to_vec()));
    let points: Vec<Value> = list(record, "operationalPoints").iter().map(|p| Value::Object(without_null(obj(p)))).collect();
    core_cols.insert("operationalPoints".into(), Value::Array(points));
    let stopping: Vec<Value> = list(record, "stoppingPlaces").iter().map(|h| Value::Object(without_null(obj(h)))).collect();
    core_cols.insert("stoppingPlaces".into(), Value::Array(stopping));

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
    for name in STRUCTURES {
        let mut items = list(record, name).to_vec();
        if !items.is_empty() {
            by_id(&mut items);
            civil_cols.insert((*name).into(), Value::Array(items));
        }
    }
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
            for name in ["contactWires", "masts", "suspensionPoints", "sectionInsulators"] {
                if !cols.contains_key(name) {
                    cols.insert(name.into(), json!([]));
                }
            }
            domains.insert("electrification".into(), layer("electrification", fields, cols));
        }
    }
    sorted_collections(&mut domains);
    Ok((domains, blobs))
}

/// Every collection of the domains sorted by id.
pub(crate) fn sorted_collections(domains: &mut Object) {
    for domain in domains.values_mut() {
        if let Some(Value::Object(cols)) = domain.get_mut("collections") {
            for v in cols.values_mut() {
                if let Value::Array(items) = v {
                    by_id(items);
                }
            }
        }
    }
}

// ── Document → record ───────────────────────────────────────────────────────

/// A document with its tracks' elements by row, as core 5.0 stated them: for what reads a track element by element.
pub fn with_tracks_by_row(doc: &Value) -> Value {
    let mut out = doc.clone();
    if let Some(Value::Object(domains)) = out.get_mut("domains") {
        tracks_by_row(domains);
    }
    out
}

/// The tracks of a document's domains with their elements by row again (`alignment::to_rows`), as the record
/// keeps them; a track without a plan has no element.
fn tracks_by_row(domains: &mut Object) {
    let Some(Value::Array(tracks)) = domains.get_mut("core").and_then(|c| c.get_mut("collections")).and_then(|c| c.get_mut("tracks")) else {
        return;
    };
    for track in tracks.iter_mut().filter_map(Value::as_object_mut) {
        let rows = track
            .shift_remove("horizontalAlignment")
            .map(|a| crate::alignment::to_rows(&a))
            .unwrap_or_default();
        if !track.contains_key("elements") {
            track.insert("elements".into(), Value::Array(rows));
        }
    }
}

/// The record an infra-data-2.0 document holds — the inverse of record_to_document (`documentToRecord`).
pub fn document_to_record(doc: &Value) -> Result<Value> {
    let mut domains = obj(&doc["domains"]);
    tracks_by_row(&mut domains);
    crate::words::back(&mut domains);
    let doc = &json!({ "domains": Value::Object(domains), "blobs": doc.get("blobs").cloned().unwrap_or(json!({})) });
    let core = &doc["domains"]["core"];
    let empty_civil = json!({ "fields": {}, "collections": {} });
    let civil = get(&doc["domains"], "civil").unwrap_or(&empty_civil);
    let c = &core["collections"];
    let blobs = doc.get("blobs").cloned().unwrap_or(json!({}));
    let blob = |hash: Option<&Value>| -> Result<Value> {
        let h = js_string(hash);
        blobs
            .get(&h)
            .cloned()
            .ok_or_else(|| PayloadError::with("invalid_payload", vec![format!("/blobs: lacks {h}")]))
    };
    let l = civil.get("collections").cloned().unwrap_or(json!({}));

    let mut tracks = Vec::new();
    for t in list(c, "tracks") {
        let mut rest = obj(t);
        if let Some(Value::Array(els)) = rest.get("elements").filter(|e| rail_across_js::truthy(Some(e))) {
            let els: Vec<Value> = els.iter().map(|el| renamed(el, "designSpeed", "speed")).collect();
            rest.insert("elements".into(), Value::Array(els));
        }
        tracks.push(Value::Object(without_null(rest)));
    }
    let mut switches = Vec::new();
    for sw in list(c, "switches") {
        let mut o = Object::new();
        o.insert("switchId".into(), opt(sw.get("id").cloned()));
        for (k, v) in without(&obj(sw), &["id"]) {
            o.insert(k, v);
        }
        switches.push(Value::Object(without_null(o)));
    }
    let civil_platforms = list(&l, "platforms");
    let mut platforms = Vec::new();
    for pf in civil_platforms {
        let mut o = without(&obj(pf), &["at"]);
        for (k, v) in without(&obj(pf.get("at").unwrap_or(&Value::Null)), &["anchor"]) {
            o.insert(k, v);
        }
        platforms.push(Value::Object(without_null(o)));
    }
    let mut end_marks = Vec::new();
    for m in list(c, "endMarks") {
        let mut o = without(&obj(m), &["at"]);
        for (k, v) in obj(m.get("at").unwrap_or(&Value::Null)) {
            o.insert(k, v);
        }
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

    let operational_points = list(c, "operationalPoints").to_vec();

    let mut collections: Vec<(&str, Vec<Value>)> = vec![
        ("tracks", tracks),
        ("switches", switches),
        ("platforms", platforms),
        ("endMarks", end_marks),
        ("routes", routes),
        (
            "kmLines",
            list(c, "kmLines")
                .iter()
                .map(|l| crate::km_lines::to_record(l, &blobs))
                .collect::<Result<Vec<_>>>()?,
        ),
        ("axisSurveys", axis_surveys),
        ("referenceAxes", reference_axes),
        ("operationalPoints", operational_points),
        ("stoppingPlaces", list(c, "stoppingPlaces").to_vec()),
        ("bridges", list(&l, "bridges").to_vec()),
        ("retainingWalls", list(&l, "retainingWalls").to_vec()),
        ("noiseBarriers", list(&l, "noiseBarriers").to_vec()),
    ];
    let mut record = spread(obj(&core["fields"]), obj(civil.get("fields").unwrap_or(&Value::Null)));
    for name in COLLECTIONS {
        let pos = collections.iter().position(|(n, _)| n == name).expect("collection");
        let (_, items) = std::mem::take(&mut collections[pos]);
        if ALWAYS.contains(name) || !items.is_empty() {
            record.insert((*name).into(), Value::Array(items));
        }
    }
    let kept: Object = doc["domains"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, _)| domain_version(name).is_none())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if !kept.is_empty() {
        record.insert("keptDomains".into(), Value::Object(kept));
    }
    for name in CARRIED {
        let Some(d) = doc["domains"].get(name) else { continue };
        let mut o = Object::new();
        o.insert("fields".into(), d.get("fields").cloned().unwrap_or(json!({})));
        o.insert("collections".into(), d.get("collections").cloned().unwrap_or(json!({})));
        record.insert((*name).into(), Value::Object(o));
    }
    Ok(Value::Object(record))
}
