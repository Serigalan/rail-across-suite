// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A project's operations in revisions (operations-1.0, O2; decision 0012):
//! each revision the whole operations document, numbered per project, by hash.
//! The timetables are read from the head, and a timetable saved, made, brought
//! in or deleted is a new revision; the door checks each before it is kept.

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use rail_across_doc::error::PayloadError;
use rail_across_doc::manifest::document_hash;
use rail_across_doc::operations::{from_operations, to_operations};

use crate::http::ApiError;
use crate::store::{pack, unpack};
use crate::util::iso;

/// The empty operations of a project that has none kept.
pub fn empty_record() -> Value {
    json!({ "timetables": [] })
}

/// The head of a project's operations: (its number — 0 for none —, the record it holds).
pub fn head(db: &Connection, project_id: &str) -> Result<(i64, Value), ApiError> {
    let row: Option<(i64, Vec<u8>)> = db
        .query_row(
            "SELECT number, payload FROM operations_revision WHERE project_id = ? ORDER BY number DESC LIMIT 1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        None => Ok((0, empty_record())),
        Some((number, payload)) => Ok((number, record_of(&payload)?)),
    }
}

/// The document of revision `number` of a project's operations, where there is one.
pub fn document(db: &Connection, project_id: &str, number: i64) -> Result<Option<Value>, ApiError> {
    let payload: Option<Vec<u8>> = db
        .query_row(
            "SELECT payload FROM operations_revision WHERE project_id = ? AND number = ?",
            params![project_id, number],
            |r| r.get(0),
        )
        .optional()?;
    Ok(payload.and_then(|p| unpack(&p)))
}

/// The record revision `number` holds, where there is one.
pub fn record_at(db: &Connection, project_id: &str, number: i64) -> Result<Option<Value>, ApiError> {
    match document(db, project_id, number)? {
        None => Ok(None),
        Some(doc) => Ok(Some(from_operations(&doc).map_err(|_| ApiError::new(500, "internal"))?)),
    }
}

fn record_of(payload: &[u8]) -> Result<Value, ApiError> {
    let doc = unpack(payload).ok_or_else(|| ApiError::new(500, "internal"))?;
    from_operations(&doc).map_err(|_| ApiError::new(500, "internal"))
}

/// The key a refusal of the door is told by (what the checks by hand said before, decision 0012).
pub fn fault_of(err: &PayloadError) -> &'static str {
    let path = err.details.first().map(String::as_str).unwrap_or("");
    if path.contains("/collections/trains/") {
        if path.contains("/points") { "points_invalid" } else { "train_invalid" }
    } else if path.contains("/collections/timetables/") && (path.contains("/dispatching") || path.contains("/blockingTimes")) {
        "dispatch_invalid"
    } else {
        "timetable_invalid"
    }
}

/// The document of a record, as the door takes it, else the key of its first fault (422).
pub fn checked_document(record: &Value) -> Result<Value, ApiError> {
    to_operations(record, true).map_err(|e| ApiError::new(422, fault_of(&e)))
}

/// Keep `record` as the next revision of a project's operations after `number`,
/// `changed` the timetables it changed (their makers and savers noted); its number.
pub fn commit(db: &Connection, project_id: &str, number: i64, record: &Value, changed: &[String], user: i64, now: i64) -> Result<i64, ApiError> {
    let doc = checked_document(record)?;
    let next = number + 1;
    let at = iso(now);
    db.execute(
        "INSERT INTO operations_revision (project_id, number, hash, payload, changed, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![project_id, next, document_hash(&doc), pack(&doc), json!(changed).to_string(), user, at],
    )?;
    for id in changed {
        db.execute(
            "INSERT INTO operations_timetable (project_id, timetable_id, created_by, created_at, updated_by, updated_at) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (project_id, timetable_id) DO UPDATE SET updated_by = excluded.updated_by, updated_at = excluded.updated_at",
            params![project_id, id, user, at, user, at],
        )?;
    }
    Ok(next)
}

/// The timetables of a record, as `[{ id, name, data }]`.
pub fn timetables(record: &Value) -> Vec<Value> {
    record.get("timetables").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// The record with its timetables `list`.
pub fn with_timetables(record: &Value, list: Vec<Value>) -> Value {
    let mut o = record.as_object().cloned().unwrap_or_default();
    o.insert("timetables".into(), Value::Array(list));
    Value::Object(o)
}

/// The revisions of a project's operations, newest first: number, hash, who made it when, what it changed.
pub fn revisions(db: &Connection, project_id: &str) -> Result<Vec<Value>, ApiError> {
    let list = db
        .prepare(
            "SELECT o.number, o.hash, o.changed, o.created_at, u.id AS uid, u.name AS uname FROM operations_revision o
             JOIN user u ON u.id = o.created_by WHERE o.project_id = ? ORDER BY o.number DESC",
        )?
        .query_map([project_id], |r| {
            Ok(json!({
                "number": r.get::<_, i64>("number")?, "hash": r.get::<_, String>("hash")?,
                "changed": serde_json::from_str::<Value>(&r.get::<_, String>("changed")?).unwrap_or(json!([])),
                "createdAt": r.get::<_, String>("created_at")?,
                "createdBy": { "id": r.get::<_, i64>("uid")?, "name": r.get::<_, String>("uname")? },
            }))
        })?
        .collect::<Result<_, _>>()?;
    Ok(list)
}
