// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The timetables of a project, for the Fahrplan-Editor (`routes/timetables.js`, decision 0016).

use std::collections::HashSet;

use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::http::{ApiError, Res, Resp};
use crate::routes::not_found;
use crate::routes::projects::project_or_404;
use crate::store::{Project, Store, transaction};
use crate::util::uuid;

const TIMETABLE_LIMIT: usize = 4 * 1024 * 1024;
const TIMETABLES_AT_ONCE: usize = 200;
const DAY_LIMIT: f64 = 2.0 * 86400.0;

fn is_text(v: Option<&Value>, max: usize, required: bool) -> bool {
    v.and_then(Value::as_str)
        .is_some_and(|s| s.encode_utf16().count() <= max && (!required || !rail_across_js::js_trim(s).is_empty()))
}

fn or_null(v: Option<&Value>, test: impl Fn(&Value) -> bool) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(v) => test(v),
    }
}

fn finite(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

fn is_time(v: &Value) -> bool {
    finite(v).is_some_and(|x| (0.0..DAY_LIMIT).contains(&x))
}

/// `text ?? ''`: a missing or null field as the empty text.
fn or_empty(v: Option<&Value>) -> Value {
    v.filter(|v| !v.is_null()).cloned().unwrap_or(json!(""))
}

fn is_point(p: &Value) -> bool {
    p.is_object()
        && is_text(p.get("id"), 64, true)
        && or_null(p.get("place"), |v| is_text(Some(v), 64, true))
        && is_text(p.get("station"), 200, true)
        && or_null(p.get("track"), |v| is_text(Some(v), 64, true))
        && p.get("stop").is_some_and(Value::is_boolean)
        && or_null(p.get("dwell"), |v| finite(v).is_some_and(|x| (0.0..=86400.0).contains(&x)))
        && or_null(p.get("departure"), is_time)
}

fn is_dispatch(d: &Value) -> bool {
    if !d.is_object() {
        return false;
    }
    let empty = json!({});
    let times = d.get("times").filter(|t| !t.is_null()).unwrap_or(&empty);
    or_null(d.get("mode"), |v| matches!(v.as_str(), Some("show" | "wait")))
        && or_null(d.get("priority"), |v| matches!(v.as_str(), Some("departure" | "category")))
        && or_null(d.get("categories"), |v| {
            v.as_array().is_some_and(|a| a.len() <= 100 && a.iter().all(|c| is_text(Some(c), 20, true)))
        })
        && times.is_object()
        && ["setup", "approach", "release"]
            .iter()
            .all(|k| or_null(times.get(*k), |v| finite(v).is_some_and(|x| (0.0..=10000.0).contains(&x))))
}

/// The key of a timetable's first fault (`timetableFault`).
pub fn timetable_fault(data: &Value) -> Option<&'static str> {
    let Some(trains) = data.get("trains").and_then(Value::as_array).filter(|t| data.is_object() && t.len() <= 5000) else {
        return Some("timetable_invalid");
    };
    if data.get("dispatch").is_some_and(|d| !d.is_null() && !is_dispatch(d)) {
        return Some("dispatch_invalid");
    }
    let mut ids = HashSet::new();
    for t in trains {
        if !t.is_object() || !is_text(t.get("id"), 64, true) || !ids.insert(t["id"].as_str().unwrap().to_string()) {
            return Some("train_invalid");
        }
        if !is_text(Some(&or_empty(t.get("number"))), 40, false)
            || !is_text(Some(&or_empty(t.get("category"))), 20, false)
            || !is_text(Some(&or_empty(t.get("name"))), 200, false)
        {
            return Some("train_invalid");
        }
        if !or_null(t.get("vehicle"), |v| is_text(Some(v), 200, true))
            || !or_null(t.get("mode"), |v| is_text(Some(v), 40, false))
            || !or_null(t.get("comfort"), |v| is_text(Some(v), 40, false))
        {
            return Some("train_invalid");
        }
        let distribution = t.get("distribution").filter(|v| !v.is_null()).cloned().unwrap_or(json!("mareco"));
        if !t.get("departure").is_some_and(is_time)
            || !is_text(Some(&or_empty(t.get("margin"))), 40, false)
            || !matches!(distribution.as_str(), Some("mareco" | "linear"))
        {
            return Some("train_invalid");
        }
        if !t
            .get("points")
            .and_then(Value::as_array)
            .is_some_and(|p| p.len() <= 300 && p.iter().all(is_point))
        {
            return Some("points_invalid");
        }
    }
    None
}

/// A timetable as it is kept — name, data as JSON — from what a request brings (`checkedTimetable`).
pub fn checked_timetable(given: &Value) -> Result<(String, String), ApiError> {
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
    if let Some(f) = timetable_fault(data) {
        return Err(ApiError::new(422, f));
    }
    let json = rail_across_js::stringify(data);
    if json.encode_utf16().count() > TIMETABLE_LIMIT {
        return Err(ApiError::new(413, "too_large"));
    }
    Ok((name, json))
}

/// Timetables a request brings to be added at once (`checkedTimetables`).
pub fn checked_timetables(given: &Value) -> Result<Vec<(String, String)>, ApiError> {
    let Some(list) = given.as_array().filter(|l| l.len() <= TIMETABLES_AT_ONCE) else {
        return Err(ApiError::new(422, "timetable_invalid"));
    };
    list.iter().map(checked_timetable).collect()
}

const SELECT: &str = "SELECT t.*, c.name AS created_by_name, u.name AS updated_by_name FROM timetable t
                  JOIN user c ON c.id = t.created_by JOIN user u ON u.id = t.updated_by";

fn may_edit(user: &User, p: &Project) -> bool {
    !p.template || user.role == "admin"
}

fn public(r: &Row, user: &User, p: &Project) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": r.get::<_, String>("id")?, "projectId": r.get::<_, String>("project_id")?, "name": r.get::<_, String>("name")?,
        "version": r.get::<_, i64>("version")?, "data": serde_json::from_str::<Value>(&r.get::<_, String>("data")?).unwrap_or(Value::Null),
        "createdBy": { "id": r.get::<_, i64>("created_by")?, "name": r.get::<_, String>("created_by_name")? }, "createdAt": r.get::<_, String>("created_at")?,
        "updatedBy": { "id": r.get::<_, i64>("updated_by")?, "name": r.get::<_, String>("updated_by_name")? }, "updatedAt": r.get::<_, String>("updated_at")?,
        "mayEdit": may_edit(user, p),
    }))
}

fn get_one(ctx: &Ctx, id: &str, user: &User, p: &Project) -> rusqlite::Result<Option<Value>> {
    ctx.db
        .query_row(&format!("{SELECT} WHERE t.id = ? AND t.deleted_at IS NULL"), [id], |r| public(r, user, p))
        .optional()
}

fn timetable_or_404(ctx: &Ctx, user: &User) -> Result<(String, Project), ApiError> {
    let id = ctx.req.param("id");
    let project_id: Option<String> = ctx
        .db
        .query_row("SELECT project_id FROM timetable WHERE id = ? AND deleted_at IS NULL", [id], |r| r.get(0))
        .optional()?;
    let project_id = project_id.ok_or_else(not_found)?;
    Ok((id.to_string(), project_or_404(ctx, &project_id, user)?))
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    let list: Vec<Value> = ctx
        .db
        .prepare(&format!(
            "{SELECT} WHERE t.project_id = ? AND t.deleted_at IS NULL ORDER BY t.name COLLATE NOCASE, t.created_at"
        ))?
        .query_map([&p.id], |r| public(r, &u, &p))?
        .collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "timetables": list, "mayEdit": may_edit(&u, &p) })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let (name, data) = checked_timetable(ctx.req.json())?;
    let id = uuid();
    let at = ctx.iso();
    ctx.db.execute(
        "INSERT INTO timetable (id, project_id, name, data, created_by, created_at, updated_by, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![id, p.id, name, data, u.id, at, u.id, at],
    )?;
    Ok(Resp::json(201, json!({ "timetable": get_one(ctx, &id, &u, &p)? })))
}

pub fn import(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    let given = checked_timetables(ctx.req.field("timetables").unwrap_or(&Value::Null))?;
    let s = Store::new(&ctx.db, ctx.now());
    let ids = transaction(&ctx.db, || Ok(s.add_timetables(&p.id, &given, u.id)?))?;
    let mut out = Vec::new();
    for id in ids {
        out.push(get_one(ctx, &id, &u, &p)?);
    }
    Ok(Resp::json(201, json!({ "timetables": out })))
}

pub fn get(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (id, p) = timetable_or_404(ctx, &u)?;
    Ok(Resp::ok(json!({ "timetable": get_one(ctx, &id, &u, &p)? })))
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
    let changed = ctx.db.execute(
        "UPDATE timetable SET name = ?, data = ?, version = version + 1, updated_by = ?, updated_at = ? WHERE id = ? AND version = ? AND deleted_at IS NULL",
        params![name, data, u.id, ctx.iso(), id, version as i64],
    )?;
    if changed == 0 {
        return Ok(Resp::json(409, json!({ "error": "stale_version", "timetable": get_one(ctx, &id, &u, &p)? })));
    }
    Ok(Resp::ok(json!({ "timetable": get_one(ctx, &id, &u, &p)? })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (id, p) = timetable_or_404(ctx, &u)?;
    if !may_edit(&u, &p) {
        return Err(ApiError::new(403, "template_admin"));
    }
    ctx.db.execute(
        "UPDATE timetable SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL",
        params![ctx.iso(), id],
    )?;
    Ok(Resp::empty(204))
}
