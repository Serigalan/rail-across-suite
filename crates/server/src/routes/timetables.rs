// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The timetables of a project, for the Fahrplan-Editor (decision 0012): read from
//! the head of the project's operations and kept as a new revision of them
//! (operations-1.0, O2; decision 0012). A timetable's version is the number of
//! the revision it was read from; a save on an older one is taken where the
//! timetable is as it was there — someone saved another meanwhile —, and refused
//! where it changed since.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::http::{ApiError, Res, Resp};
use crate::operations;
use crate::routes::not_found;
use crate::routes::projects::project_or_404;
use crate::store::{Project, Store, transaction};

const TIMETABLE_LIMIT: usize = 4 * 1024 * 1024;
const TIMETABLES_AT_ONCE: usize = 200;
const TRAINS_AT_MOST: usize = 5000;

/// A timetable's data as the door takes it — `{ trains, dispatch? }` —, else the
/// key of its first fault: what the schema cannot say by hand (a list of trains,
/// a train's id once in its timetable), the rest by the door (`fault_of`).
pub fn timetable_fault(data: &Value) -> Option<&'static str> {
    let Some(trains) = data
        .get("trains")
        .and_then(Value::as_array)
        .filter(|t| data.is_object() && t.len() <= TRAINS_AT_MOST)
    else {
        return Some("timetable_invalid");
    };
    let mut ids = HashSet::new();
    for t in trains {
        if let Some(id) = t.get("id").and_then(Value::as_str) {
            if !ids.insert(id) {
                return Some("train_invalid");
            }
        }
    }
    let record = json!({ "timetables": [{ "id": "timetable", "name": "timetable", "data": data }] });
    rail_across_doc::operations::to_operations(&record, true)
        .err()
        .map(|e| operations::fault_of(&e))
}

/// A timetable as it is kept — name, data — from what a request brings (`checkedTimetable`).
pub fn checked_timetable(given: &Value) -> Result<(String, Value), ApiError> {
    let name = given
        .get("name")
        .filter(|v| !v.is_null())
        .map(|v| rail_across_js::js_string(Some(v)))
        .unwrap_or_default();
    let name: String = rail_across_js::js_trim(&name).chars().take(200).collect();
    if name.is_empty() {
        return Err(ApiError::new(422, "name_required"));
    }
    let empty = json!({ "trains": [] });
    let data = given.get("data").filter(|v| !v.is_null()).unwrap_or(&empty);
    if rail_across_js::stringify(data).encode_utf16().count() > TIMETABLE_LIMIT {
        return Err(ApiError::new(413, "too_large"));
    }
    if let Some(f) = timetable_fault(data) {
        return Err(ApiError::new(422, f));
    }
    Ok((name, data.clone()))
}

/// Timetables a request brings to be added at once (`checkedTimetables`).
pub fn checked_timetables(given: &Value) -> Result<Vec<(String, Value)>, ApiError> {
    let Some(list) = given.as_array().filter(|l| l.len() <= TIMETABLES_AT_ONCE) else {
        return Err(ApiError::new(422, "timetable_invalid"));
    };
    list.iter().map(checked_timetable).collect()
}

fn may_edit(user: &User, p: &Project) -> bool {
    !p.template || user.role == "admin"
}

/// Who made a timetable and who saved it last, and when.
fn meta(db: &Connection, project_id: &str, id: &str) -> rusqlite::Result<Value> {
    db.query_row(
        "SELECT m.created_by, c.name, m.created_at, m.updated_by, u.name, m.updated_at FROM operations_timetable m
         JOIN user c ON c.id = m.created_by JOIN user u ON u.id = m.updated_by WHERE m.project_id = ? AND m.timetable_id = ?",
        params![project_id, id],
        |r| {
            Ok(json!({
                "createdBy": { "id": r.get::<_, i64>(0)?, "name": r.get::<_, String>(1)? }, "createdAt": r.get::<_, String>(2)?,
                "updatedBy": { "id": r.get::<_, i64>(3)?, "name": r.get::<_, String>(4)? }, "updatedAt": r.get::<_, String>(5)?,
            }))
        },
    )
    .optional()
    .map(|m| m.unwrap_or(json!({ "createdBy": null, "createdAt": null, "updatedBy": null, "updatedAt": null })))
}

/// A timetable of the head as the API shows it.
fn public(db: &Connection, t: &Value, number: i64, user: &User, p: &Project) -> rusqlite::Result<Value> {
    let id = t["id"].as_str().unwrap_or_default();
    let mut o = json!({
        "id": id, "projectId": p.id, "name": t["name"], "version": number, "data": t["data"], "mayEdit": may_edit(user, p),
    });
    if let (Some(o), Value::Object(m)) = (o.as_object_mut(), meta(db, &p.id, id)?) {
        o.extend(m);
    }
    Ok(o)
}

fn find<'a>(record: &'a Value, id: &str) -> Option<&'a Value> {
    record["timetables"].as_array()?.iter().find(|t| t["id"] == id)
}

/// The timetable of the head a request names, and its project; 404 where the head has none of that id.
fn timetable_or_404(ctx: &Ctx, user: &User) -> Result<(String, Project), ApiError> {
    let id = ctx.req.param("id");
    let project_id: Option<String> = ctx
        .db
        .query_row("SELECT project_id FROM operations_timetable WHERE timetable_id = ?", [id], |r| r.get(0))
        .optional()?;
    let project_id = project_id.ok_or_else(not_found)?;
    let p = project_or_404(ctx, &project_id, user)?;
    let (_, record) = operations::head(&ctx.db, &p.id)?;
    if find(&record, id).is_none() {
        return Err(not_found());
    }
    Ok((id.to_string(), p))
}

fn one(ctx: &Ctx, id: &str, user: &User, p: &Project) -> Result<Value, ApiError> {
    let (number, record) = operations::head(&ctx.db, &p.id)?;
    let t = find(&record, id).ok_or_else(not_found)?;
    Ok(public(&ctx.db, t, number, user, p)?)
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    let (number, record) = operations::head(&ctx.db, &p.id)?;
    let mut list = operations::timetables(&record);
    list.sort_by(|a, b| {
        let key = |t: &Value| t["name"].as_str().unwrap_or_default().to_lowercase();
        key(a).cmp(&key(b)).then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let list: Vec<Value> = list.iter().map(|t| public(&ctx.db, t, number, &u, &p)).collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "timetables": list, "mayEdit": may_edit(&u, &p) })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let given = checked_timetable(ctx.req.json())?;
    let s = Store::new(&ctx.db, ctx.now());
    let ids = transaction(&ctx.db, || s.add_timetables(&p.id, &[given], u.id))?;
    Ok(Resp::json(201, json!({ "timetable": one(ctx, &ids[0], &u, &p)? })))
}

pub fn import(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let given = checked_timetables(ctx.req.field("timetables").unwrap_or(&Value::Null))?;
    let s = Store::new(&ctx.db, ctx.now());
    let ids = transaction(&ctx.db, || s.add_timetables(&p.id, &given, u.id))?;
    let mut out = Vec::new();
    for id in ids {
        out.push(one(ctx, &id, &u, &p)?);
    }
    Ok(Resp::json(201, json!({ "timetables": out })))
}

pub fn get(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (id, p) = timetable_or_404(ctx, &u)?;
    Ok(Resp::ok(json!({ "timetable": one(ctx, &id, &u, &p)? })))
}

pub fn update(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (id, p) = timetable_or_404(ctx, &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let (name, data) = checked_timetable(ctx.req.json())?;
    let version = ctx.req.field("version").and_then(|v| v.as_f64().filter(|f| f.fract() == 0.0 && f.is_finite()));
    let Some(version) = version else {
        return Err(ApiError::new(422, "version_required"));
    };
    let version = version as i64;
    let (number, record) = operations::head(&ctx.db, &p.id)?;
    if version != number {
        // Read from an older revision: taken where the timetable is as it was there.
        let base = operations::record_at(&ctx.db, &p.id, version)?;
        let was = base.as_ref().and_then(|b| find(b, &id));
        if was.is_none() || was != find(&record, &id) {
            return Ok(Resp::json(409, json!({ "error": "stale_version", "timetable": one(ctx, &id, &u, &p)? })));
        }
    }
    let list: Vec<Value> = operations::timetables(&record)
        .into_iter()
        .map(|t| {
            if t["id"] == id.as_str() {
                json!({ "id": id, "name": name, "data": data })
            } else {
                t
            }
        })
        .collect();
    let next = operations::with_timetables(&record, list);
    let now = ctx.now();
    transaction(&ctx.db, || {
        operations::commit(&ctx.db, &p.id, number, &next, std::slice::from_ref(&id), u.id, now)
    })?;
    Ok(Resp::ok(json!({ "timetable": one(ctx, &id, &u, &p)? })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (id, p) = timetable_or_404(ctx, &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let (number, record) = operations::head(&ctx.db, &p.id)?;
    let list: Vec<Value> = operations::timetables(&record).into_iter().filter(|t| t["id"] != id.as_str()).collect();
    let next = operations::with_timetables(&record, list);
    let now = ctx.now();
    transaction(&ctx.db, || {
        operations::commit(&ctx.db, &p.id, number, &next, std::slice::from_ref(&id), u.id, now)
    })?;
    Ok(Resp::empty(204))
}

/// The head of a project's operations as its document: `{ number, hash, document }` (an empty one where none is kept).
pub fn operations_head(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    let (number, record) = operations::head(&ctx.db, &p.id)?;
    let doc = match operations::document(&ctx.db, &p.id, number)? {
        Some(doc) => doc,
        None => operations::checked_document(&record)?,
    };
    Ok(Resp::ok(
        json!({ "number": number, "hash": rail_across_doc::manifest::document_hash(&doc), "document": doc }),
    ))
}

/// The revisions of a project's operations, newest first.
pub fn operations_revisions(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    Ok(Resp::ok(json!({ "revisions": operations::revisions(&ctx.db, &p.id)? })))
}

/// One revision of a project's operations as its document.
pub fn operations_revision(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    let number: i64 = ctx.req.param("number").parse().map_err(|_| not_found())?;
    let doc = operations::document(&ctx.db, &p.id, number)?.ok_or_else(not_found)?;
    Ok(Resp::ok(
        json!({ "number": number, "hash": rail_across_doc::manifest::document_hash(&doc), "document": doc }),
    ))
}
