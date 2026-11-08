// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The API under /api, route by route as the Node server registers them.

use serde_json::{Value, json};

use crate::app::Ctx;
use crate::http::{ApiError, Res, Resp, matches};

pub mod admin;
pub mod auth;
pub mod clouds;
pub mod projects;
pub mod runs;
pub mod shares;
pub mod site;
pub mod terrain;
pub mod timetables;
pub mod users;
pub mod vehicles;

pub type Handler = fn(&mut Ctx) -> Res;

/// The routes: method, pattern, handler.
const ROUTES: &[(&str, &str, Handler)] = &[
    ("GET", "/api/health", health),
    ("POST", "/api/login", auth::login),
    ("POST", "/api/logout", auth::logout),
    ("GET", "/api/me", auth::me),
    ("POST", "/api/me/password", auth::password),
    ("GET", "/api/admin/users", admin::list),
    ("POST", "/api/admin/users", admin::create),
    ("PATCH", "/api/admin/users/:id", admin::patch),
    ("GET", "/api/admin/clouds", clouds::admin),
    ("GET", "/api/projects", projects::list),
    ("POST", "/api/projects", projects::create),
    ("PATCH", "/api/projects/:id", projects::patch),
    ("DELETE", "/api/projects/:id", projects::delete),
    ("GET", "/api/projects/:id/members", projects::members),
    ("POST", "/api/projects/:id/members", projects::add_member),
    ("DELETE", "/api/projects/:id/members/:userId", projects::remove_member),
    ("PUT", "/api/blobs", projects::put_blob),
    ("GET", "/api/blobs/:hash", projects::get_blob),
    ("POST", "/api/projects/:id/copy", projects::copy),
    ("POST", "/api/projects/:id/variants", projects::branch),
    ("PATCH", "/api/variants/:id", projects::patch_variant),
    ("GET", "/api/variants/:id", projects::variant),
    ("GET", "/api/variants/:id/head", projects::head),
    ("GET", "/api/variants/:id/revisions", projects::history),
    ("POST", "/api/variants/:id/revisions", projects::check_in),
    ("GET", "/api/revisions/:id", projects::revision),
    ("GET", "/api/revisions/:a/base/:b", projects::base),
    ("GET", "/api/revisions/:id/remaps", projects::remaps),
    ("GET", "/api/users", users::search),
    ("GET", "/api/projects/:id/clouds", clouds::list),
    ("POST", "/api/projects/:id/clouds", clouds::create),
    ("PUT", "/api/projects/:id/clouds/:cid/raw", clouds::piece),
    ("POST", "/api/projects/:id/clouds/:cid/complete", clouds::complete),
    ("POST", "/api/projects/:id/clouds/:cid/retry", clouds::retry),
    ("DELETE", "/api/projects/:id/clouds/:cid", clouds::delete),
    ("GET", "/api/projects/:id/clouds/:cid/transforms", clouds::transforms),
    ("POST", "/api/projects/:id/clouds/:cid/transforms", clouds::add_transform),
    ("POST", "/api/projects/:id/clouds/:cid/transforms/active", clouds::set_active),
    ("GET", "/api/projects/:id/clouds/:cid/:level/index", clouds::index),
    ("GET", "/api/projects/:id/clouds/:cid/:level/tiles", clouds::tiles),
    ("POST", "/api/projects/:id/clouds/:cid/:level/ranges", clouds::ranges),
    ("GET", "/api/projects/:id/runs", runs::list),
    ("POST", "/api/projects/:id/runs", runs::create),
    ("GET", "/api/projects/:id/runs/:rid", runs::get),
    ("DELETE", "/api/projects/:id/runs/:rid", runs::delete),
    ("GET", "/api/projects/:id/shares", shares::list),
    ("POST", "/api/projects/:id/shares", shares::create),
    ("DELETE", "/api/projects/:id/shares/:sid", shares::delete),
    ("GET", "/api/share/:token", shares::open),
    ("GET", "/api/share/:token/head", shares::head),
    ("GET", "/api/share/:token/clouds/:cid/:level/index", shares::index),
    ("POST", "/api/share/:token/clouds/:cid/:level/ranges", shares::ranges),
    ("GET", "/api/vehicles", vehicles::list),
    ("GET", "/api/vehicles/:id", vehicles::get),
    ("POST", "/api/vehicles", vehicles::create),
    ("PUT", "/api/vehicles/:id", vehicles::update),
    ("DELETE", "/api/vehicles/:id", vehicles::delete),
    ("GET", "/api/projects/:id/timetables", timetables::list),
    ("POST", "/api/projects/:id/timetables", timetables::create),
    ("POST", "/api/projects/:id/timetables/import", timetables::import),
    ("GET", "/api/timetables/:id", timetables::get),
    ("PUT", "/api/timetables/:id", timetables::update),
    ("DELETE", "/api/timetables/:id", timetables::delete),
    ("POST", "/api/terrain", terrain::terrain),
    ("GET", "/api/dem/:z/:x/:file", terrain::dem),
    ("POST", "/api/mdb", terrain::mdb),
    ("POST", "/api/__test/sweep", test_sweep),
    ("GET", "/api/__test/failures", test_failures),
    ("POST", "/api/__test/users", test_create_user),
];

pub type Params = std::collections::HashMap<String, String>;

/// The handler of a request's route and its parameters; None where no route takes it.
pub fn find(method: &str, path: &str, settings: &crate::app::Settings) -> Option<(Handler, Params)> {
    let method = if method == "HEAD" { "GET" } else { method };
    let test_mode = settings.test_mode;
    // The local package has no user management: no sign-in, password, admin or user routes; /me is its own.
    if settings.local {
        if method == "GET" && path == "/api/me" {
            return Some((local_me as Handler, Params::new()));
        }
        if LOCAL_ABSENT.iter().any(|(m, p)| *m == method && matches(p, path).is_some()) {
            return None;
        }
    }
    // A static part beats a parameter, as in Fastify's router: fewer parameters first.
    let mut best: Option<(usize, Handler, Params)> = None;
    for (m, pattern, h) in ROUTES {
        if *m != method || (pattern.starts_with("/api/__test") && !test_mode) {
            continue;
        }
        if let Some(params) = matches(pattern, path) {
            if best.as_ref().is_none_or(|b| params.len() < b.0) {
                best = Some((params.len(), *h, params));
            }
        }
    }
    best.map(|(_, h, p)| (h, p))
}

/// The routes the local package does not have.
const LOCAL_ABSENT: &[(&str, &str)] = &[
    ("POST", "/api/login"),
    ("POST", "/api/logout"),
    ("POST", "/api/me/password"),
    ("GET", "/api/admin/users"),
    ("POST", "/api/admin/users"),
    ("PATCH", "/api/admin/users/:id"),
    ("GET", "/api/users"),
];

fn health(ctx: &mut Ctx) -> Res {
    ctx.db.query_row("select 1", [], |_| Ok(()))?;
    Ok(Resp::ok(if ctx.app.settings.local {
        json!({ "status": "ok", "local": true })
    } else {
        json!({ "status": "ok" })
    }))
}

/// /api/me of the local package: the one implicit local user.
fn local_me(ctx: &mut Ctx) -> Res {
    let u = ctx.user.as_ref().ok_or_else(|| ApiError::new(401, "unauthenticated"))?;
    let mut user = u.public();
    user["local"] = json!(true);
    Ok(Resp::ok(json!({ "user": user })))
}

fn test_sweep(ctx: &mut Ctx) -> Res {
    crate::auth::sweep(ctx.app, &ctx.db)?;
    Ok(Resp::ok(json!({})))
}

fn test_failures(ctx: &mut Ctx) -> Res {
    Ok(Resp::ok(json!({ "count": ctx.app.failures.lock().unwrap().len() })))
}

fn test_create_user(ctx: &mut Ctx) -> Res {
    let b = ctx.req.json();
    let s = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let user = crate::auth::create_user(
        &ctx.db,
        &s("login"),
        &b.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| s("login")),
        &s("password"),
        b.get("role").and_then(Value::as_str).unwrap_or("user"),
        b.get("mustChangePassword").and_then(Value::as_bool).unwrap_or(true),
        ctx.now(),
    )?;
    Ok(Resp::ok(json!({ "user": user.public() })))
}

/// `text(v, max)`: a value as trimmed text of at most `max` characters, absent as None.
pub fn text(v: Option<&Value>, max: usize) -> Option<String> {
    let v = v.filter(|v| !v.is_null())?;
    let s = rail_across_js::js_string(Some(v));
    Some(rail_across_js::js_trim(&s).chars().take(max).collect())
}

/// `Number(v)` of a JSON value or a route parameter.
pub fn number(v: Option<&Value>) -> f64 {
    rail_across_js::js_number(v)
}

pub fn not_found() -> ApiError {
    ApiError::new(404, "not_found")
}
