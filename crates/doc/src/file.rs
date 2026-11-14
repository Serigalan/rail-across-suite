// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The file rail-across-1.0 (`.railacross.json`; decision 0029, phase 5): one
//! project in one file — its infrastructure (the part infra-data-1.0) and its
//! operation (the part operations-1.0), each part its domains, the blobs of both
//! together; the envelope naming the project, the program that wrote the file and
//! when. A file carries no schemas: a domain names its schema by hash, and a
//! reader knows the versions it reads. Within the suite the parts are what the
//! project server keeps of them — an infra-data-2.0 document of a variant's
//! revision, an operations-1.0 document of the project's operations —; this
//! puts them into a file and takes them out again, each through its door.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::error::{PayloadError, Result};
use crate::schema::SchemaSet;

/// The file format's name and version, as the envelope states them.
pub const FORMAT: &str = "rail-across";
pub const VERSION: &str = "1.0";
/// The program that writes a file, where the caller names none.
pub const GENERATOR: &str = "Rail-Across-Suite";

const ENVELOPE: &str = include_str!("../../../schemas/rail-across.schema.json");

fn envelope() -> &'static SchemaSet {
    static SET: OnceLock<SchemaSet> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = SchemaSet::new();
        set.add(serde_json::from_str(ENVELOPE).expect("the envelope's schema")).expect("the envelope's schema");
        set
    })
}

/// Every text a value holds, wherever it stands.
fn texts<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
    match v {
        Value::String(s) => out.push(s),
        Value::Array(items) => items.iter().for_each(|x| texts(x, out)),
        Value::Object(o) => o.values().for_each(|x| texts(x, out)),
        _ => {}
    }
}

/// The blobs of `blobs` the domains name.
fn named_blobs(domains: &Value, blobs: &Value) -> Value {
    let mut used = Vec::new();
    texts(domains, &mut used);
    let mut out = serde_json::Map::new();
    for hash in used {
        if let Some(b) = blobs.get(hash) {
            out.insert(hash.to_string(), b.clone());
        }
    }
    Value::Object(out)
}

/// The file of a project: `infra` an infra-data-2.0 document, `operations` an operations-1.0 document — each
/// checked by its door —, `meta` `{ projectId?, generator?: { name, version }, savedAt? }`. The project's id is
/// the core's where the caller names none; a part that is missing is not in the file.
pub fn to_file(infra: Option<&Value>, operations: Option<&Value>, meta: &Value) -> Result<Value> {
    let mut file = serde_json::Map::new();
    file.insert("format".into(), json!(FORMAT));
    file.insert("version".into(), json!(VERSION));
    let mut blobs = serde_json::Map::new();
    let infra = infra.map(crate::door::check_document).transpose()?;
    let operations = operations.map(crate::operations::check_operations).transpose()?;
    let project = meta
        .get("projectId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| infra.as_ref().and_then(|d| d["domains"]["core"]["fields"]["id"].as_str().map(str::to_string)));
    file.insert("projectId".into(), json!(crate::ids::uuid_of(project.as_deref().unwrap_or("project"))));
    let generator = meta
        .get("generator")
        .cloned()
        .unwrap_or_else(|| json!({ "name": GENERATOR, "version": env!("CARGO_PKG_VERSION") }));
    file.insert("generator".into(), generator);
    if let Some(at) = meta.get("savedAt").filter(|v| v.is_string()) {
        file.insert("savedAt".into(), at.clone());
    }
    for (member, format, doc) in [("infraData", "infra-data", &infra), ("operations", "operations", &operations)] {
        let Some(doc) = doc else { continue };
        for (hash, b) in doc.get("blobs").and_then(Value::as_object).into_iter().flatten() {
            blobs.insert(hash.clone(), b.clone());
        }
        file.insert(member.into(), json!({ "format": format, "version": "1.0", "domains": doc["domains"] }));
    }
    file.insert("blobs".into(), Value::Object(blobs));
    let file = Value::Object(file);
    check_envelope(&file)?;
    Ok(file)
}

fn check_envelope(file: &Value) -> Result<()> {
    if file.get("format").and_then(Value::as_str) != Some(FORMAT) || file.get("version").and_then(Value::as_str) != Some(VERSION) {
        return Err(PayloadError::with("unsupported_version", vec![format!("/format: is not {FORMAT}-{VERSION}")]));
    }
    let found = envelope().check("rail-across", file, 20).unwrap_or_default();
    if !found.is_empty() {
        let details = found.iter().map(|b| format!("{}: {}", if b.path == "/" { "" } else { &b.path }, b.message)).collect();
        return Err(PayloadError::with("invalid_payload", details));
    }
    for (hash, value) in file["blobs"].as_object().into_iter().flatten() {
        if crate::canonical::content_hash(value) != *hash {
            return Err(PayloadError::with("invalid_payload", vec![format!("/blobs/{hash}: holds other content")]));
        }
    }
    Ok(())
}

/// Whether a parsed file says it is one of rail-across — of any version: its parts beside its envelope. (The
/// envelope of before decision 0021 had the same name, and its domains at its top.)
pub fn is_file(data: &Value) -> bool {
    data.get("format").and_then(Value::as_str) == Some(FORMAT) && (data.get("infraData").is_some() || data.get("operations").is_some())
}

/// The parts of a file, each through its door: `{ projectId, generator?, savedAt?, infra?, operations? }` —
/// `infra` an infra-data-2.0 document, `operations` an operations-1.0 document, the blobs each names with it.
pub fn from_file(file: &Value) -> Result<Value> {
    check_envelope(file)?;
    let blobs = file.get("blobs").cloned().unwrap_or(json!({}));
    let mut out = serde_json::Map::new();
    for k in ["projectId", "generator", "savedAt"] {
        if let Some(v) = file.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    if let Some(part) = file.get("infraData") {
        let doc = json!({ "format": "infra-data", "version": crate::door::VERSION, "domains": part["domains"], "blobs": named_blobs(&part["domains"], &blobs) });
        out.insert("infra".into(), crate::door::check_document(&doc).map_err(|e| prefixed(e, "/infraData"))?);
    }
    if let Some(part) = file.get("operations") {
        let doc = json!({ "format": "operations", "version": "1.0", "domains": part["domains"], "blobs": named_blobs(&part["domains"], &blobs) });
        out.insert("operations".into(), crate::operations::check_operations(&doc).map_err(|e| prefixed(e, "/operations"))?);
    }
    Ok(Value::Object(out))
}

/// A part's refusal with its place in the file.
fn prefixed(e: PayloadError, at: &str) -> PayloadError {
    let details = e.details.iter().map(|d| if d.starts_with('/') { format!("{at}{d}") } else { d.clone() }).collect();
    PayloadError::with(&e.code, details)
}
