// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Projects, variants, revisions and images (`routes/projects.js`, AP 10.5).

use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::{self, User};
use crate::checks::{axis_surveys_changed, check_kept, check_record, fresh_errors, normalized_record};
use crate::http::{ApiError, Res, Resp};
use crate::routes::timetables::checked_timetables;
use crate::routes::{not_found, number, text};
use crate::store::{Project, Store, Variant, public_variant, transaction};
use crate::util::{from_base64, sha256_hex};

const IMAGE_LIMIT: usize = 5 * 1024 * 1024;

fn store<'a>(ctx: &'a Ctx) -> Store<'a> {
    Store::new(&ctx.db, ctx.now())
}

/// A project the user may see, else "not there" (decision 127).
pub fn project_or_404(ctx: &Ctx, id: &str, user: &User) -> Result<Project, ApiError> {
    let s = store(ctx);
    match s.project(id)? {
        Some(p) if s.can_see(user, &p)? => Ok(p),
        _ => Err(not_found()),
    }
}

fn variant_or_404(ctx: &Ctx, id: &str, user: &User) -> Result<Variant, ApiError> {
    let v = store(ctx).variant(id)?.ok_or_else(not_found)?;
    project_or_404(ctx, &v.project_id, user)?;
    Ok(v)
}

fn admin_for_template(user: &User, p: &Project, v: Option<&Variant>) -> Result<(), ApiError> {
    if p.template && v.is_none_or(|v| v.parent_variant_id.is_none()) && user.role != "admin" {
        return Err(ApiError::new(403, "template_admin"));
    }
    Ok(())
}

fn not_template(p: &Project, v: Option<&Variant>) -> Result<(), ApiError> {
    if p.template && v.is_none_or(|v| v.parent_variant_id.is_none()) {
        return Err(ApiError::new(403, "template_read_only"));
    }
    Ok(())
}

/// A revision id as JavaScript's `Number(id)` reads it.
fn revision_id(v: &Value) -> Option<i64> {
    let x = number(Some(v));
    (x.is_finite() && x.fract() == 0.0).then_some(x as i64)
}

fn revision_or_404(ctx: &Ctx, id: &Value, user: &User) -> Result<Value, ApiError> {
    let r = revision_id(id).map(|i| store(ctx).revision_meta(i)).transpose()?.unwrap_or(Value::Null);
    if r.is_null() {
        return Err(not_found());
    }
    project_or_404(ctx, r["projectId"].as_str().unwrap_or(""), user)?;
    Ok(r)
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    Ok(Resp::ok(json!({ "projects": store(ctx).list_projects(&u)? })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let title = text(ctx.req.field("title"), 200)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ApiError::new(422, "title_required"))?;
    let description = text(ctx.req.field("description"), 4000).unwrap_or_default();
    let template = rail_across_js::truthy(ctx.req.field("template"));
    if template && u.role != "admin" {
        return Err(ApiError::new(403, "template_admin"));
    }
    let timetables = checked_timetables(ctx.req.field("timetables").filter(|v| !v.is_null()).unwrap_or(&json!([])))?;
    let checked = match ctx.req.field("payload").filter(|v| !v.is_null()) {
        Some(given) => check_record(given)?,
        None => check_kept(json!({ "id": "new", "tracks": [], "switches": [], "platforms": [] }))?,
    };
    let variant_name = text(ctx.req.field("variantName"), 100)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| if template { "Vorlage" } else { "Bestand" }.into());
    let s = store(ctx);
    let (pid, vid, rid) = transaction(&ctx.db, || {
        let made = s.create_project(
            &title,
            Some(&description),
            &checked.record,
            u.id,
            &checked.error_keys(),
            &variant_name,
            template,
        )?;
        s.add_timetables(&made.0, &timetables, u.id)?;
        Ok(made)
    })?;
    let project = s.project_summary(&u, &pid)?;
    Ok(Resp::json(
        201,
        json!({ "project": project, "projectId": pid, "variantId": vid, "revisionId": rid, "errors": checked.errors, "warnings": checked.warnings, "timetables": timetables.len() }),
    ))
}

pub fn patch(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let id = ctx.req.param("id").to_string();
    admin_for_template(&u, &project_or_404(ctx, &id, &u)?, None)?;
    let title = text(ctx.req.field("title"), 200);
    if title.as_deref() == Some("") {
        return Err(ApiError::new(422, "title_required"));
    }
    let description = text(ctx.req.field("description"), 4000);
    let s = store(ctx);
    s.patch_project(&id, title.as_deref(), description.as_deref())?;
    Ok(Resp::ok(json!({ "project": s.project_summary(&u, &id)? })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    admin_for_template(&u, &p, None)?;
    if u.role != "admin" && p.created_by != u.id {
        return Err(ApiError::new(403, "not_allowed"));
    }
    store(ctx).delete_project(&p.id)?;
    Ok(Resp::empty(204))
}

fn managed_or_403(ctx: &Ctx, u: &User) -> Result<Project, ApiError> {
    let p = project_or_404(ctx, ctx.req.param("id"), u)?;
    if p.template {
        return Err(ApiError::new(422, "template_members"));
    }
    if !Store::can_manage(u, &p) {
        return Err(ApiError::new(403, "not_allowed"));
    }
    Ok(p)
}

pub fn members(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    Ok(Resp::ok(
        json!({ "members": store(ctx).members(&p.id)?, "canManage": Store::can_manage(&u, &p) && !p.template }),
    ))
}

pub fn add_member(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = managed_or_403(ctx, &u)?;
    let id = number(ctx.req.field("userId"));
    let user = if id.is_finite() && id.fract() == 0.0 {
        auth::user_by_id(&ctx.db, id as i64)?
    } else {
        None
    };
    let Some(user) = user.filter(|u| u.active) else {
        return Err(ApiError::new(422, "user_not_found"));
    };
    if user.id == p.created_by {
        return Err(ApiError::new(422, "member_is_creator"));
    }
    let s = store(ctx);
    s.add_member(&p.id, user.id, u.id)?;
    Ok(Resp::json(201, json!({ "members": s.members(&p.id)? })))
}

pub fn remove_member(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = managed_or_403(ctx, &u)?;
    let s = store(ctx);
    let id = number(Some(&json!(ctx.req.param("userId"))));
    if id.is_finite() {
        s.remove_member(&p.id, id as i64)?;
    }
    Ok(Resp::ok(json!({ "members": s.members(&p.id)? })))
}

pub fn put_blob(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let mime = rail_across_js::js_string(ctx.req.field("mime"));
    let ok = mime
        .strip_prefix("image/")
        .is_some_and(|t| matches!(t, "png" | "jpeg" | "gif" | "webp" | "svg+xml"));
    if !ok {
        return Err(ApiError::new(422, "image_type"));
    }
    let data = ctx
        .req
        .field("data")
        .filter(|v| !v.is_null())
        .map(|v| rail_across_js::js_string(Some(v)))
        .unwrap_or_default();
    let bytes = from_base64(&data);
    if bytes.is_empty() {
        return Err(ApiError::new(422, "image_empty"));
    }
    if bytes.len() > IMAGE_LIMIT {
        return Err(ApiError::new(413, "too_large"));
    }
    let hash = sha256_hex(&bytes);
    ctx.db.execute(
        "INSERT OR IGNORE INTO blob (hash, mime, size, data, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        params![hash, mime, bytes.len() as i64, bytes, u.id, ctx.iso()],
    )?;
    Ok(Resp::ok(json!({ "hash": hash })))
}

pub fn get_blob(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let hash = ctx.req.param("hash");
    let valid = hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
    let b: Option<(String, Vec<u8>)> = if valid {
        ctx.db
            .query_row("SELECT mime, data FROM blob WHERE hash = ?", [hash], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
    } else {
        None
    };
    let (mime, data) = b.ok_or_else(not_found)?;
    Ok(Resp::bytes(200, &mime, data)
        .header("content-security-policy", "default-src 'none'; style-src 'unsafe-inline'; sandbox")
        .header("x-content-type-options", "nosniff")
        .header("cache-control", "private, max-age=31536000, immutable"))
}

pub fn copy(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    if !p.template {
        return Err(ApiError::new(422, "not_template"));
    }
    let s = store(ctx);
    let variants: Vec<Variant> = s
        .variants(&p.id)?
        .into_iter()
        .filter(|v| !v.archived && v.parent_variant_id.is_none())
        .collect();
    let from = match ctx.req.field("variant").filter(|v| !v.is_null()) {
        Some(want) => variants.iter().find(|v| want.as_str() == Some(v.id.as_str())),
        None => variants.first(),
    }
    .cloned()
    .ok_or_else(|| ApiError::new(422, "variant_mismatch"))?;
    let title = text(ctx.req.field("title"), 200).filter(|t| !t.is_empty()).unwrap_or_else(|| p.title.clone());
    let head = s.revision(from.head_revision_id)?.ok_or_else(not_found)?;
    let checked = check_record(&head.payload)?;
    let ((pid, vid, rid), copied) = transaction(&ctx.db, || {
        let made = s.create_project(
            &title,
            Some(p.description.as_deref().unwrap_or("")),
            &checked.record,
            u.id,
            &checked.error_keys(),
            "Bestand",
            false,
        )?;
        let n = s.copy_timetables(&p.id, &made.0, u.id)?;
        Ok((made, n))
    })?;
    Ok(Resp::json(
        201,
        json!({ "project": s.project_summary(&u, &pid)?, "projectId": pid, "variantId": vid, "revisionId": rid, "timetables": copied }),
    ))
}

pub fn branch(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, ctx.req.param("id"), &u)?;
    not_template(&p, None)?;
    let name = text(ctx.req.field("name"), 100)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| ApiError::new(422, "name_required"))?;
    let from_id = ctx
        .req
        .field("fromVariant")
        .filter(|v| !v.is_null())
        .map(|v| rail_across_js::js_string(Some(v)))
        .unwrap_or_default();
    let from = variant_or_404(ctx, &from_id, &u)?;
    if from.project_id != p.id {
        return Err(ApiError::new(422, "variant_mismatch"));
    }
    let mut at = from.head_revision_id;
    if let Some(r) = ctx.req.field("fromRevision").filter(|v| !v.is_null()) {
        let r = revision_or_404(ctx, r, &u)?;
        if r["projectId"].as_str() != Some(&p.id) {
            return Err(ApiError::new(422, "revision_mismatch"));
        }
        at = r["id"].as_i64().unwrap_or(at);
    }
    let s = store(ctx);
    let id = s.branch(&p.id, &name, &from.id, at, u.id)?;
    let v = s.variant(&id)?.unwrap();
    Ok(Resp::json(201, json!({ "variant": public_variant(&v, s.revision_meta(at)?, 0) })))
}

pub fn patch_variant(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let v = variant_or_404(ctx, ctx.req.param("id"), &u)?;
    let s = store(ctx);
    admin_for_template(&u, &s.project(&v.project_id)?.ok_or_else(not_found)?, Some(&v))?;
    let name = text(ctx.req.field("name"), 100);
    if name.as_deref() == Some("") {
        return Err(ApiError::new(422, "name_required"));
    }
    if let Some(n) = name {
        s.rename_variant(&v.id, &n)?;
    }
    if let Some(a) = ctx.req.field("archived") {
        s.archive_variant(&v.id, rail_across_js::truthy(Some(a)))?;
    }
    let now = s.variant(&v.id)?.unwrap();
    Ok(Resp::ok(json!({ "variant": public_variant(&now, s.revision_meta(now.head_revision_id)?, 0) })))
}

pub fn variant(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let v = variant_or_404(ctx, ctx.req.param("id"), &u)?;
    Ok(Resp::ok(
        json!({ "variant": public_variant(&v, store(ctx).revision_meta(v.head_revision_id)?, 0) }),
    ))
}

pub fn head(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let v = variant_or_404(ctx, ctx.req.param("id"), &u)?;
    let r = store(ctx).revision(v.head_revision_id)?.ok_or_else(not_found)?;
    Ok(Resp::ok(
        json!({ "variant": public_variant(&v, r.meta.clone(), 0), "revision": r.meta, "payload": r.payload }),
    ))
}

pub fn history(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let v = variant_or_404(ctx, ctx.req.param("id"), &u)?;
    let limit = ctx.req.query.get("limit").map(|l| number(Some(&json!(l)))).unwrap_or(500.0).min(2000.0);
    let limit = if limit.is_nan() { 0 } else { limit.max(0.0) as usize };
    Ok(Resp::ok(json!({ "revisions": store(ctx).history(&v, limit)? })))
}

pub fn revision(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let meta = revision_or_404(ctx, &json!(ctx.req.param("id")), &u)?;
    let r = store(ctx).revision(meta["id"].as_i64().unwrap())?.ok_or_else(not_found)?;
    Ok(Resp::ok(json!({ "revision": r.meta, "payload": r.payload, "remaps": r.remaps })))
}

pub fn base(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let a = revision_or_404(ctx, &json!(ctx.req.param("a")), &u)?;
    let b = revision_or_404(ctx, &json!(ctx.req.param("b")), &u)?;
    if a["projectId"] != b["projectId"] {
        return Err(ApiError::new(422, "project_mismatch"));
    }
    let s = store(ctx);
    let base = s.common_base(a["projectId"].as_str().unwrap(), a["id"].as_i64().unwrap(), b["id"].as_i64().unwrap())?;
    Ok(Resp::ok(
        json!({ "base": base.map(|b| s.revision_meta(b)).transpose()?.unwrap_or(Value::Null) }),
    ))
}

pub fn remaps(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let head = revision_or_404(ctx, &json!(ctx.req.param("id")), &u)?;
    let base = match ctx.req.query.get("base") {
        Some(b) => Some(revision_or_404(ctx, &json!(b), &u)?),
        None => None,
    };
    if base.as_ref().is_some_and(|b| b["projectId"] != head["projectId"]) {
        return Err(ApiError::new(422, "project_mismatch"));
    }
    let r = store(ctx).remaps_between(
        head["projectId"].as_str().unwrap(),
        base.and_then(|b| b["id"].as_i64()),
        head["id"].as_i64().unwrap(),
    )?;
    Ok(Resp::ok(json!({ "remaps": r })))
}

pub fn check_in(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let v = variant_or_404(ctx, ctx.req.param("id"), &u)?;
    let s = store(ctx);
    not_template(&s.project(&v.project_id)?.ok_or_else(not_found)?, Some(&v))?;
    let b = ctx.req.json();
    let base = b
        .get("base")
        .and_then(|x| x.as_f64().filter(|f| f.fract() == 0.0 && f.is_finite()))
        .map(|f| f as i64);
    let Some(base) = base else { return Err(ApiError::new(422, "base_required")) };
    if base != v.head_revision_id {
        return Ok(Resp::json(409, json!({ "error": "stale_base", "head": s.revision_meta(v.head_revision_id)? })));
    }
    let merge_parent = b.get("mergeParent").filter(|m| !m.is_null());
    let mut merge_id = None;
    if let Some(m) = merge_parent {
        let mp = revision_or_404(ctx, m, &u)?;
        if mp["projectId"].as_str() != Some(&v.project_id) {
            return Err(ApiError::new(422, "project_mismatch"));
        }
        merge_id = mp["id"].as_i64();
    }
    let remaps = b.get("remaps").filter(|r| !r.is_null()).cloned().unwrap_or(json!([]));
    if !remaps.is_array() {
        return Err(ApiError::new(422, "invalid_remaps"));
    }
    let checked = check_record(b.get("payload").unwrap_or(&Value::Null))?;
    if checked.record.get("id").and_then(Value::as_str) != Some(&v.project_id) {
        return Err(ApiError::new(422, "project_mismatch"));
    }
    let parent = s.revision(base)?.ok_or_else(not_found)?;
    if !u.may_edit_clouds() {
        let merge_record = match merge_id {
            Some(m) => Some(normalized_record(&s.revision(m)?.ok_or_else(not_found)?.payload)),
            None => None,
        };
        if axis_surveys_changed(&checked.record, &normalized_record(&parent.payload), merge_record.as_ref()) {
            return Err(ApiError::new(403, "clouds_not_allowed").with(
                "message",
                json!("Messachsen speichern oder löschen darf nur der Admin oder wer das Recht „Punktwolken bearbeiten“ hat."),
            ));
        }
    }
    let fresh = fresh_errors(&checked.errors, &parent.error_keys);
    if !fresh.is_empty() {
        return Ok(Resp::json(422, json!({ "error": "invalid_record", "errors": fresh })));
    }
    let remaps: Vec<Value> = remaps
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e.get("from").is_some_and(Value::is_string) && e.get("to").is_some_and(Value::is_array))
        .map(|e| json!({ "from": e["from"], "to": e["to"].as_array().unwrap().iter().map(|t| json!(rail_across_js::js_string(Some(t)))).collect::<Vec<_>>() }))
        .collect();
    let message = text(b.get("message"), 2000).unwrap_or_default();
    match s.check_in(&v, base, merge_id, u.id, &message, &checked.record, &json!(remaps), &checked.error_keys())? {
        Err(head) => Ok(Resp::json(409, json!({ "error": "stale_base", "head": s.revision_meta(head)? }))),
        Ok(rev) => Ok(Resp::json(201, json!({ "revision": s.revision_meta(rev)?, "warnings": checked.warnings }))),
    }
}
