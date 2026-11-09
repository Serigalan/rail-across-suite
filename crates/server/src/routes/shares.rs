// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Read-only links to a project's point clouds (`routes/shares.js`).

use std::collections::HashSet;
use std::fs;

use rusqlite::{OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::clouds::read_ranges;
use crate::clouds::store::{Cloud, CloudStore, public_cloud};
use crate::http::{ApiError, Res, Resp};
use crate::routes::clouds::{level_of, storage};
use crate::routes::{not_found, number};
use crate::store::{Project, Store, public_variant};
use crate::util::{base64url, iso, parse_iso, random_bytes};

const MAX_DAYS: f64 = 365.0;
const DAY: i64 = 24 * 3600 * 1000;

#[derive(Clone)]
struct Share {
    id: i64,
    token: String,
    project_id: String,
    variant_id: Option<String>,
    cloud_ids: Option<String>,
    label: String,
    created_by: i64,
    created_by_name: Option<String>,
    created_at: String,
    expires_at: Option<String>,
}

fn share_row(r: &Row) -> rusqlite::Result<Share> {
    Ok(Share {
        id: r.get("id")?,
        token: r.get("token")?,
        project_id: r.get("project_id")?,
        variant_id: r.get("variant_id")?,
        cloud_ids: r.get("cloud_ids")?,
        label: r.get("label")?,
        created_by: r.get("created_by")?,
        created_by_name: r.get("created_by_name").ok().flatten(),
        created_at: r.get("created_at")?,
        expires_at: r.get("expires_at")?,
    })
}

fn public_share(s: &Share) -> Value {
    json!({
        "id": s.id, "token": s.token, "variantId": s.variant_id,
        "cloudIds": s.cloud_ids.as_deref().and_then(|c| serde_json::from_str::<Value>(c).ok()),
        "label": s.label, "createdBy": s.created_by, "createdByName": s.created_by_name, "createdAt": s.created_at, "expiresAt": s.expires_at,
    })
}

const BY_ID: &str = "SELECT s.*, u.name AS created_by_name FROM share_link s LEFT JOIN user u ON u.id = s.created_by WHERE s.id = ?";

fn managed_or_404(ctx: &Ctx, u: &User) -> Result<Project, ApiError> {
    let s = Store::new(&ctx.db, ctx.now());
    let p = match s.project(ctx.req.param("id"))? {
        Some(p) if s.can_see(u, &p)? => p,
        _ => return Err(not_found()),
    };
    if !Store::can_manage(u, &p) {
        return Err(ApiError::new(403, "share_not_allowed"));
    }
    Ok(p)
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = managed_or_404(ctx, &u)?;
    let list: Vec<Value> = ctx
        .db
        .prepare("SELECT s.*, u.name AS created_by_name FROM share_link s LEFT JOIN user u ON u.id = s.created_by WHERE s.project_id = ? ORDER BY s.id DESC")?
        .query_map([&p.id], share_row)?
        .map(|s| s.map(|s| public_share(&s)))
        .collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "shares": list })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = managed_or_404(ctx, &u)?;
    let b = ctx.req.json().clone();
    let s = Store::new(&ctx.db, ctx.now());
    let mut variant_id = None;
    if let Some(v) = b.get("variantId").filter(|v| !v.is_null()) {
        match s.variant(&rail_across_js::js_string(Some(v)))? {
            Some(v) if v.project_id == p.id => variant_id = Some(v.id),
            _ => return Err(ApiError::new(422, "share_variant")),
        }
    }
    let mut cloud_ids: Option<Vec<String>> = None;
    if let Some(c) = b.get("cloudIds").filter(|v| !v.is_null()) {
        let Some(list) = c.as_array().filter(|l| !l.is_empty()) else {
            return Err(ApiError::new(422, "share_clouds"));
        };
        let mut seen = HashSet::new();
        let ids: Vec<String> = list
            .iter()
            .map(|v| rail_across_js::js_string(Some(v)))
            .filter(|id| seen.insert(id.clone()))
            .collect();
        let cs = CloudStore::new(&ctx.db, ctx.now());
        for id in &ids {
            if cs.get(id)?.map(|c| c.project_id) != Some(p.id.clone()) {
                return Err(ApiError::new(422, "share_clouds"));
            }
        }
        cloud_ids = Some(ids);
    }
    let days = b.get("expiresInDays").filter(|v| !v.is_null()).map(|v| number(Some(v)));
    if days.is_some_and(|d| !(d.is_finite() && d.fract() == 0.0 && (1.0..=MAX_DAYS).contains(&d))) {
        return Err(ApiError::new(422, "share_expiry"));
    }
    let t = ctx.now();
    let label: String = rail_across_js::js_trim(
        &b.get("label")
            .filter(|v| !v.is_null())
            .map(|v| rail_across_js::js_string(Some(v)))
            .unwrap_or_default(),
    )
    .chars()
    .take(200)
    .collect();
    ctx.db.execute(
        "INSERT INTO share_link (token, project_id, variant_id, cloud_ids, label, created_by, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            base64url(&random_bytes(24)),
            p.id,
            variant_id,
            cloud_ids.map(|c| rail_across_js::stringify(&json!(c))),
            label,
            u.id,
            iso(t),
            days.map(|d| iso(t + d as i64 * DAY))
        ],
    )?;
    let id = ctx.db.last_insert_rowid();
    let share = ctx.db.query_row(BY_ID, [id], share_row)?;
    Ok(Resp::json(201, json!({ "share": public_share(&share) })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = managed_or_404(ctx, &u)?;
    let sid = number(Some(&json!(ctx.req.param("sid"))));
    let s = if sid.is_finite() && sid.fract() == 0.0 {
        ctx.db.query_row(BY_ID, [sid as i64], share_row).optional()?
    } else {
        None
    };
    let s = s.filter(|s| s.project_id == p.id).ok_or_else(not_found)?;
    ctx.db.execute("DELETE FROM share_link WHERE id = ?", [s.id])?;
    Ok(Resp::empty(204))
}

fn token_ok(t: &str) -> bool {
    (20..=100).contains(&t.len()) && t.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn share_or_404(ctx: &Ctx) -> Result<(Share, Project), ApiError> {
    let token = ctx.req.param("token");
    let s = if token_ok(token) {
        ctx.db.query_row("SELECT * FROM share_link WHERE token = ?", [token], share_row).optional()?
    } else {
        None
    };
    let s = s
        .filter(|s| s.expires_at.as_deref().is_none_or(|e| parse_iso(e).is_none_or(|t| t > ctx.now())))
        .ok_or_else(|| ApiError::new(404, "share_not_found"))?;
    let p = Store::new(&ctx.db, ctx.now())
        .project(&s.project_id)?
        .ok_or_else(|| ApiError::new(404, "share_not_found"))?;
    Ok((s, p))
}

fn shown(s: &Share, c: &Cloud) -> bool {
    c.status == "ready"
        && s.cloud_ids
            .as_deref()
            .is_none_or(|ids| serde_json::from_str::<Vec<String>>(ids).unwrap_or_default().contains(&c.id))
}

fn cloud_or_404(ctx: &Ctx) -> Result<(Cloud, usize), ApiError> {
    storage(ctx)?;
    let (s, _) = share_or_404(ctx)?;
    let c = CloudStore::new(&ctx.db, ctx.now()).get(ctx.req.param("cid"))?;
    let c = c.filter(|c| c.project_id == s.project_id && shown(&s, c)).ok_or_else(not_found)?;
    let level = level_of(ctx.req.param("level")).ok_or_else(not_found)?;
    Ok((c, level))
}

fn no_store(r: Resp) -> Resp {
    r.header("cache-control", "private, no-store")
}

pub fn open(ctx: &mut Ctx) -> Res {
    let (s, p) = share_or_404(ctx)?;
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let active = cs.active_transforms(&p.id)?;
    let v = match &s.variant_id {
        Some(id) => Store::new(&ctx.db, ctx.now()).variant(id)?,
        None => None,
    };
    let clouds: Vec<Value> = cs
        .list(&p.id)?
        .iter()
        .filter(|c| shown(&s, c))
        .map(|c| public_cloud(c, active.get(&c.id)))
        .collect();
    Ok(no_store(Resp::ok(json!({
        "share": { "projectId": p.id, "projectTitle": p.title, "variantName": v.map(|v| v.name), "label": s.label, "expiresAt": s.expires_at },
        "clouds": clouds,
    }))))
}

pub fn head(ctx: &mut Ctx) -> Res {
    let (s, _) = share_or_404(ctx)?;
    let st = Store::new(&ctx.db, ctx.now());
    let v = match &s.variant_id {
        Some(id) => st.variant(id)?,
        None => None,
    };
    let v = v.filter(|v| v.project_id == s.project_id).ok_or_else(not_found)?;
    let r = st.revision(v.head_revision_id)?.ok_or_else(not_found)?;
    Ok(no_store(Resp::ok(
        json!({ "variant": public_variant(&v, r.meta.clone(), 0), "revision": r.meta, "payload": r.payload }),
    )))
}

pub fn index(ctx: &mut Ctx) -> Res {
    let (c, level) = cloud_or_404(ctx)?;
    let bytes = fs::read(storage(ctx)?.index(&c.project_id, &c.id, level)?)?;
    Ok(no_store(Resp::bytes(200, "application/json", bytes)))
}

pub fn ranges(ctx: &mut Ctx) -> Res {
    let (c, level) = cloud_or_404(ctx)?;
    let out = read_ranges(&storage(ctx)?.tiles(&c.project_id, &c.id, level)?, ctx.req.field("ranges"))?;
    Ok(no_store(Resp::bytes(200, "application/octet-stream", out)))
}
