// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Signing in and out, who am I, my own password (`routes/auth.js`, AP 10.4).

use rail_across_js::js_string;
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::{self, COOKIE};
use crate::http::{ApiError, Res, Resp};

pub fn login(ctx: &mut Ctx) -> Res {
    let login = ctx.req.field("login");
    let password = ctx.req.field("password");
    let brake_key = js_string(login);
    let address = ctx.req.ip.clone();
    let wait = auth::braked(ctx.app, &brake_key, &address);
    if wait > 0 {
        return Err(ApiError::new(429, "too_many_attempts").with("retryAfter", json!(wait)));
    }
    let lookup = login.filter(|v| !v.is_null()).map(|v| js_string(Some(v))).unwrap_or_default();
    let pw = password.filter(|v| !v.is_null()).map(|v| js_string(Some(v))).unwrap_or_default();
    // The brake keys by String(login); the lookup by String(login ?? '').
    let user = check_keyed(ctx, &brake_key, &lookup, &pw, &address)?;
    let Some(user) = user else {
        let after = auth::braked(ctx.app, &brake_key, &address);
        return Err(if after > 0 {
            ApiError::new(429, "too_many_attempts").with("retryAfter", json!(after))
        } else {
            ApiError::new(401, "invalid_credentials")
        });
    };
    let token = auth::open_session(&ctx.db, user.id, ctx.now())?;
    let fresh = auth::user_by_id(&ctx.db, user.id)?.unwrap();
    Ok(Resp::ok(json!({ "user": fresh.public() })).cookie(ctx.app.session_cookie(&token)))
}

fn check_keyed(ctx: &Ctx, key: &str, lookup: &str, password: &str, address: &str) -> Result<Option<auth::User>, ApiError> {
    let found = auth::user_by_login(&ctx.db, lookup)?;
    if found.as_ref().is_some_and(|u| u.active && auth::verify_password(&u.password_hash, password)) {
        ctx.app.failures.lock().unwrap().remove(&format!("{}|{address}", key.to_lowercase()));
        return Ok(found);
    }
    auth::record_failure(ctx.app, key, address);
    Ok(None)
}

pub fn logout(ctx: &mut Ctx) -> Res {
    match &ctx.user {
        Some(u) => auth::close_all_sessions(&ctx.db, u.id)?,
        None => auth::close_session(&ctx.db, ctx.req.cookies.get(COOKIE).map(String::as_str))?,
    }
    Ok(Resp::empty(204).cookie(ctx.app.clear_session_cookie()))
}

pub fn me(ctx: &mut Ctx) -> Res {
    let u = ctx.user.as_ref().ok_or_else(|| ApiError::new(401, "unauthenticated"))?;
    Ok(Resp::ok(json!({ "user": u.public() })))
}

pub fn password(ctx: &mut Ctx) -> Res {
    let u = ctx.user.clone().ok_or_else(|| ApiError::new(401, "unauthenticated"))?;
    let s = |v: Option<&Value>| v.filter(|v| !v.is_null()).map(|v| js_string(Some(v))).unwrap_or_default();
    let (current, next) = (ctx.req.field("current"), ctx.req.field("next"));
    if !auth::verify_password(&u.password_hash, &s(current)) {
        return Err(ApiError::new(403, "wrong_password"));
    }
    if !auth::password_acceptable(next) {
        return Err(ApiError::new(422, "password_too_short"));
    }
    if next == current {
        return Err(ApiError::new(422, "password_unchanged"));
    }
    auth::set_password(&ctx.db, u.id, next.and_then(Value::as_str).unwrap_or(""), false)?;
    Ok(Resp::empty(204))
}
