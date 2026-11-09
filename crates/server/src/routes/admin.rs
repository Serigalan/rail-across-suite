// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The user administration, admins only (`routes/admin.js`).

use rusqlite::params;
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::{self, User};
use crate::http::{ApiError, Res, Resp};
use crate::store::transaction;

const ROLES: [&str; 2] = ["admin", "user"];

fn login_valid(login: &str) -> bool {
    let n = login.chars().count();
    (2..=64).contains(&n) && login.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '@' | '-'))
}

pub fn list(ctx: &mut Ctx) -> Res {
    ctx.admin()?;
    let users: Vec<User> = ctx
        .db
        .prepare("SELECT * FROM user ORDER BY login COLLATE NOCASE")?
        .query_map([], User::from_row)?
        .collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "users": users.iter().map(User::public).collect::<Vec<_>>() })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    ctx.admin()?;
    let b = ctx.req.json();
    let s = |k: &str| b.get(k).filter(|v| !v.is_null()).map(|v| rail_across_js::js_string(Some(v)));
    let login = s("login").unwrap_or_default();
    let role = b.get("role").filter(|v| !v.is_null()).cloned().unwrap_or(json!("user"));
    if !login_valid(rail_across_js::js_trim(&login)) {
        return Err(ApiError::new(422, "login_invalid"));
    }
    let Some(role) = role.as_str().filter(|r| ROLES.contains(r)) else {
        return Err(ApiError::new(422, "role_invalid"));
    };
    if !auth::password_acceptable(b.get("password")) {
        return Err(ApiError::new(422, "password_too_short"));
    }
    if auth::user_by_login(&ctx.db, &login)?.is_some() {
        return Err(ApiError::new(409, "login_taken"));
    }
    let name = s("name")
        .map(|n| rail_across_js::js_trim(&n).to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| rail_across_js::js_trim(&login).to_string());
    let user = auth::create_user(&ctx.db, &login, &name, b["password"].as_str().unwrap_or(""), role, true, ctx.now())?;
    if rail_across_js::truthy(b.get("canEditClouds")) {
        ctx.db.execute("UPDATE user SET can_edit_clouds = 1 WHERE id = ?", [user.id])?;
    }
    let user = auth::user_by_id(&ctx.db, user.id)?.unwrap();
    Ok(Resp::json(201, json!({ "user": user.public() })))
}

pub fn patch(ctx: &mut Ctx) -> Res {
    ctx.admin()?;
    let id = crate::routes::number(Some(&json!(ctx.req.param("id"))));
    let user = if id.fract() == 0.0 && id.is_finite() {
        auth::user_by_id(&ctx.db, id as i64)?
    } else {
        None
    };
    let user = user.ok_or_else(crate::routes::not_found)?;
    let b = ctx.req.json().clone();
    let has = |k: &str| b.get(k).is_some();
    let role = b.get("role");
    let active = b.get("active");
    let loses_admin = user.role == "admin" && user.active && ((has("role") && role != Some(&json!("admin"))) || active == Some(&json!(false)));
    if loses_admin {
        let admins: i64 = ctx
            .db
            .query_row("SELECT COUNT(*) FROM user WHERE role = 'admin' AND active = 1", [], |r| r.get(0))?;
        if admins <= 1 {
            return Err(ApiError::new(409, "last_admin"));
        }
    }
    if has("role") && !role.and_then(Value::as_str).is_some_and(|r| ROLES.contains(&r)) {
        return Err(ApiError::new(422, "role_invalid"));
    }
    if has("password") && !auth::password_acceptable(b.get("password")) {
        return Err(ApiError::new(422, "password_too_short"));
    }
    let db = &*ctx.db;
    transaction(db, || {
        if let Some(name) = b.get("name") {
            let n = rail_across_js::js_string(Some(name));
            let n = rail_across_js::js_trim(&n);
            db.execute(
                "UPDATE user SET name = ? WHERE id = ?",
                params![if n.is_empty() { user.login.as_str() } else { n }, user.id],
            )?;
        }
        if let Some(r) = role {
            db.execute("UPDATE user SET role = ? WHERE id = ?", params![r.as_str(), user.id])?;
        }
        if let Some(a) = active {
            db.execute(
                "UPDATE user SET active = ? WHERE id = ?",
                params![rail_across_js::truthy(Some(a)) as i64, user.id],
            )?;
        }
        if let Some(c) = b.get("canEditClouds") {
            db.execute(
                "UPDATE user SET can_edit_clouds = ? WHERE id = ?",
                params![rail_across_js::truthy(Some(c)) as i64, user.id],
            )?;
        }
        Ok(())
    })?;
    if let Some(p) = b.get("password").and_then(Value::as_str) {
        auth::set_password(db, user.id, p, true)?;
    }
    if active == Some(&json!(false)) || has("password") {
        auth::close_all_sessions(db, user.id)?;
    }
    let user = auth::user_by_id(db, user.id)?.unwrap();
    Ok(Resp::ok(json!({ "user": user.public() })))
}
