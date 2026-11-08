// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The suite's vehicles for the Fahrplan-Sim (`routes/vehicles.js`, decision 0014).

use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::http::{ApiError, Res, Resp};
use crate::routes::not_found;
use crate::util::uuid;

const VEHICLE_LIMIT: usize = 512 * 1024;

fn finite(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn is_curve(c: Option<&Value>) -> bool {
    let Some(c) = c.filter(|c| c.is_object()) else { return false };
    let (Some(speeds), Some(efforts)) = (c.get("speeds").and_then(Value::as_array), c.get("max_efforts").and_then(Value::as_array)) else {
        return false;
    };
    speeds.len() >= 2
        && speeds.len() == efforts.len()
        && speeds
            .iter()
            .enumerate()
            .all(|(i, s)| finite(Some(s)).is_some_and(|s| s >= 0.0 && (i == 0 || finite(Some(&speeds[i - 1])).is_some_and(|p| s > p))))
        && efforts.iter().all(|e| finite(Some(e)).is_some_and(|e| e >= 0.0))
}

/// The key of a vehicle's first fault (`vehicleFault`).
pub fn vehicle_fault(v: Option<&Value>) -> Option<String> {
    let Some(v) = v.filter(|v| v.is_object()) else {
        return Some("vehicle_invalid".into());
    };
    let name_ok = v
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|n| !rail_across_js::js_trim(n).is_empty() && n.encode_utf16().count() <= 200);
    if !name_ok {
        return Some("name_required".into());
    }
    for key in ["mass", "length", "max_speed", "const_gamma"] {
        if !finite(v.get(key)).is_some_and(|x| x > 0.0) {
            return Some(format!("{key}_invalid"));
        }
    }
    if !finite(v.get("inertia_coefficient")).is_some_and(|x| x >= 1.0) {
        return Some("inertia_coefficient_invalid".into());
    }
    let rr = v.get("rolling_resistance").filter(|r| r.is_object());
    if !rr.is_some_and(|rr| ["A", "B", "C"].iter().all(|k| finite(rr.get(*k)).is_some_and(|x| x >= 0.0))) {
        return Some("rolling_resistance_invalid".into());
    }
    let ec = v.get("effort_curves").filter(|e| e.is_object());
    let modes = ec.and_then(|e| e.get("modes")).and_then(Value::as_object);
    let default_ok = |e: &Value| {
        e.get("default_mode")
            .and_then(Value::as_str)
            .and_then(|d| modes.and_then(|m| m.get(d)))
            .is_some_and(Value::is_object)
    };
    if !(ec.is_some_and(default_ok) && modes.is_some_and(|m| !m.is_empty())) {
        return Some("effort_curves_invalid".into());
    }
    for mode in modes.unwrap().values() {
        let curves = mode.get("curves").filter(|c| !c.is_null());
        if !mode.is_object() || !is_curve(mode.get("default_curve")) || curves.is_some_and(|c| !c.is_array()) {
            return Some("effort_curves_invalid".into());
        }
        if !curves
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .all(|c| c.is_object() && c.get("cond").is_some_and(Value::is_object) && is_curve(c.get("curve")))
        {
            return Some("effort_curves_invalid".into());
        }
    }
    None
}

const SELECT: &str = "SELECT v.*, c.name AS created_by_name FROM vehicle v JOIN user c ON c.id = v.created_by";

fn public(r: &Row, user: &User) -> rusqlite::Result<Value> {
    let created_by: i64 = r.get("created_by")?;
    Ok(json!({
        "id": r.get::<_, String>("id")?, "version": r.get::<_, i64>("version")?,
        "data": serde_json::from_str::<Value>(&r.get::<_, String>("data")?).unwrap_or(Value::Null),
        "createdBy": { "id": created_by, "name": r.get::<_, String>("created_by_name")? },
        "createdAt": r.get::<_, String>("created_at")?, "updatedAt": r.get::<_, String>("updated_at")?,
        "mayEdit": user.role == "admin" || created_by == user.id,
    }))
}

fn get_one(ctx: &Ctx, id: &str, user: &User) -> rusqlite::Result<Option<Value>> {
    ctx.db
        .query_row(&format!("{SELECT} WHERE v.id = ? AND v.deleted_at IS NULL"), [id], |r| public(r, user))
        .optional()
}

fn checked(ctx: &Ctx) -> Result<(String, String), ApiError> {
    let data = ctx.req.field("data");
    if let Some(f) = vehicle_fault(data) {
        return Err(ApiError::new(422, &f));
    }
    let json = rail_across_js::stringify(data.unwrap());
    if json.encode_utf16().count() > VEHICLE_LIMIT {
        return Err(ApiError::new(413, "too_large"));
    }
    let name: String = rail_across_js::js_trim(data.unwrap()["name"].as_str().unwrap()).chars().take(200).collect();
    Ok((name, json))
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let list: Vec<Value> = ctx
        .db
        .prepare(&format!("{SELECT} WHERE v.deleted_at IS NULL ORDER BY v.name COLLATE NOCASE, v.id"))?
        .query_map([], |r| public(r, &u))?
        .collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "vehicles": list })))
}

pub fn get(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    Ok(Resp::ok(json!({ "vehicle": get_one(ctx, ctx.req.param("id"), &u)?.ok_or_else(not_found)? })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let (name, data) = checked(ctx)?;
    let id = uuid();
    let at = ctx.iso();
    ctx.db.execute(
        "INSERT INTO vehicle (id, name, data, created_by, created_at, updated_by, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![id, name, data, u.id, at, u.id, at],
    )?;
    Ok(Resp::json(201, json!({ "vehicle": get_one(ctx, &id, &u)? })))
}

pub fn update(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let id = ctx.req.param("id").to_string();
    let row = get_one(ctx, &id, &u)?.ok_or_else(not_found)?;
    if row["mayEdit"] != json!(true) {
        return Err(ApiError::new(403, "not_allowed"));
    }
    let (name, data) = checked(ctx)?;
    let version = ctx.req.field("version").and_then(|v| v.as_f64().filter(|f| f.fract() == 0.0 && f.is_finite()));
    let Some(version) = version else {
        return Err(ApiError::new(422, "version_required"));
    };
    let changed = ctx.db.execute(
        "UPDATE vehicle SET name = ?, data = ?, version = version + 1, updated_by = ?, updated_at = ? WHERE id = ? AND version = ? AND deleted_at IS NULL",
        params![name, data, u.id, ctx.iso(), id, version as i64],
    )?;
    if changed == 0 {
        return Ok(Resp::json(409, json!({ "error": "stale_version", "vehicle": get_one(ctx, &id, &u)? })));
    }
    Ok(Resp::ok(json!({ "vehicle": get_one(ctx, &id, &u)? })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let id = ctx.req.param("id").to_string();
    let row = get_one(ctx, &id, &u)?.ok_or_else(not_found)?;
    if row["mayEdit"] != json!(true) {
        return Err(ApiError::new(403, "not_allowed"));
    }
    ctx.db
        .execute("UPDATE vehicle SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL", params![ctx.iso(), id])?;
    Ok(Resp::empty(204))
}
