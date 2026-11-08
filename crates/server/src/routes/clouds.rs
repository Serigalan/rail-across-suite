// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Point clouds on the server (`routes/clouds.js`, AP 13.1–13.5): the list,
//! the resumable upload in checked pieces, re-referencing, the tiles served
//! to members, the disk for the admin view.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

use serde_json::{Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::clouds::store::{Cloud, CloudStore, public_cloud};
use crate::clouds::{DISK_RESERVE, DISK_WARN, PROJECT_QUOTA, Storage, estimate_tile_bytes, read_ranges};
use crate::http::{ApiError, Body, Payload, Res, Resp};
use crate::routes::{not_found, number, text};
use crate::store::Store;
use crate::util::sha256_hex;

const CHUNK_BYTES: usize = 8 * 1024 * 1024;
/// The body the upload route takes.
pub const CHUNK_LIMIT: usize = CHUNK_BYTES + 64 * 1024;
const FILE_LIMIT: f64 = 64.0 * 1024.0 * 1024.0 * 1024.0;
const MEMBER_TTL: i64 = 60 * 1000;
const FORMATS: [&str; 3] = ["las", "laz", "e57"];

pub fn storage(ctx: &Ctx) -> Result<Storage, ApiError> {
    ctx.app
        .settings
        .cloud_root
        .clone()
        .map(|root| Storage { root })
        .ok_or_else(|| ApiError::new(503, "clouds_unavailable"))
}

fn editor(u: &User) -> Result<(), ApiError> {
    if u.may_edit_clouds() {
        Ok(())
    } else {
        Err(ApiError::new(403, "clouds_not_allowed"))
    }
}

/// The project (its id) the user may see, membership believed for a minute.
fn project_or_404(ctx: &Ctx, u: &User) -> Result<String, ApiError> {
    storage(ctx)?;
    let id = ctx.req.param("id").to_string();
    let key = format!("{}|{}|{id}", ctx.req.cookies.get("ra_session").map(String::as_str).unwrap_or(""), u.id);
    let t = ctx.now();
    if ctx.app.seen.lock().unwrap().get(&key).is_some_and(|until| *until > t) {
        return Ok(id);
    }
    let s = Store::new(&ctx.db, t);
    match s.project(&id)? {
        Some(p) if s.can_see(u, &p)? => {
            let mut seen = ctx.app.seen.lock().unwrap();
            if seen.len() > 10000 {
                seen.clear();
            }
            seen.insert(key, t + MEMBER_TTL);
            Ok(p.id)
        }
        _ => Err(not_found()),
    }
}

fn cloud_or_404(ctx: &Ctx, u: &User) -> Result<Cloud, ApiError> {
    let p = project_or_404(ctx, u)?;
    match CloudStore::new(&ctx.db, ctx.now()).get(ctx.req.param("cid"))? {
        Some(c) if c.project_id == p => Ok(c),
        _ => Err(not_found()),
    }
}

/// The level of `L0` … `L4`, of a ready cloud.
pub fn level_of(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    (b.len() == 2 && b[0] == b'L' && (b'0'..=b'4').contains(&b[1])).then(|| usize::from(b[1] - b'0'))
}

fn level_or_404(ctx: &Ctx, c: &Cloud) -> Result<usize, ApiError> {
    match level_of(ctx.req.param("level")) {
        Some(l) if c.status == "ready" => Ok(l),
        _ => Err(not_found()),
    }
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, &u)?;
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let active = cs.active_transforms(&p)?;
    let list: Vec<Value> = cs.list(&p)?.iter().map(|c| public_cloud(c, active.get(&c.id))).collect();
    Ok(Resp::ok(json!({ "clouds": list, "mayEdit": u.may_edit_clouds() })))
}

fn is_int(x: f64) -> bool {
    x.is_finite() && x.fract() == 0.0
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, &u)?;
    editor(&u)?;
    let b = ctx.req.json();
    let name = text(b.get("name"), 200)
        .filter(|n| !n.is_empty())
        .or_else(|| text(b.get("fileName"), 200))
        .unwrap_or_default();
    let file_name = text(b.get("fileName"), 255).unwrap_or_default();
    let file_size = number(b.get("fileSize"));
    let format = b
        .get("format")
        .filter(|v| !v.is_null())
        .map(|v| rail_across_js::js_string(Some(v)))
        .unwrap_or_default();
    let crs = b.get("crs").filter(|v| !v.is_null()).map(|v| number(Some(v)));
    let height = number(b.get("heightEpsg"));
    let source_points = b.get("pointCount").filter(|v| !v.is_null()).map(|v| number(Some(v))).unwrap_or(0.0);
    if name.is_empty() || file_name.is_empty() {
        return Err(ApiError::new(422, "name_required"));
    }
    if !FORMATS.contains(&format.as_str()) {
        return Err(ApiError::new(422, "cloud_format"));
    }
    if !is_int(file_size) || file_size <= 0.0 || file_size > FILE_LIMIT {
        return Err(ApiError::new(422, "cloud_size"));
    }
    if crs.is_some_and(|c| !is_int(c)) {
        return Err(ApiError::new(422, "cloud_crs"));
    }
    if !is_int(height) {
        return Err(ApiError::new(422, "cloud_height"));
    }
    if !source_points.is_finite() || source_points < 0.0 {
        return Err(ApiError::new(422, "cloud_points"));
    }
    let rgb = rail_across_js::truthy(b.get("rgb"));
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let needed = file_size as i64 + estimate_tile_bytes(source_points, rgb);
    let used = cs.used(&p)? + estimate_tile_bytes(cs.pending_points(&p)?, false);
    if used + needed > PROJECT_QUOTA {
        return Err(ApiError::new(507, "cloud_quota")
            .with("needed", json!(needed))
            .with("used", json!(used))
            .with("quota", json!(PROJECT_QUOTA)));
    }
    let st = storage(ctx)?;
    let (free, _) = st.disk();
    if free - needed < DISK_RESERVE {
        return Err(ApiError::new(507, "cloud_disk").with("needed", json!(needed)).with("free", json!(free)));
    }
    let id = cs.create(
        &p,
        &name,
        &file_name,
        file_size as i64,
        &format,
        crs.map(|c| c as i64),
        height as i64,
        rail_across_js::truthy(b.get("keepRaw")),
        source_points,
        rgb,
        u.id,
    )?;
    st.ensure(&p, &id)?;
    fs::File::create(st.raw(&p, &id)?)?;
    let c = cs.get(&id)?.unwrap();
    Ok(Resp::json(201, json!({ "cloud": public_cloud(&c, None), "chunkBytes": CHUNK_BYTES })))
}

pub fn piece(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    if c.status != "uploading" {
        return Err(ApiError::new(409, "cloud_not_uploading"));
    }
    let offset = ctx.req.query.get("offset").map(|o| number(Some(&json!(o)))).unwrap_or(f64::NAN);
    let body: &[u8] = match &ctx.req.body {
        Body::Bytes(b) if !b.is_empty() => b,
        _ => return Err(ApiError::new(422, "chunk_empty")),
    };
    if !is_int(offset) || offset < 0.0 {
        return Err(ApiError::new(422, "chunk_offset"));
    }
    let offset = offset as i64;
    let len = body.len() as i64;
    if offset + len > c.file_size {
        return Err(ApiError::new(422, "chunk_past_end"));
    }
    if ctx.req.header("x-olt-sha256").unwrap_or("") != sha256_hex(body) {
        return Err(ApiError::new(422, "chunk_corrupt"));
    }
    if offset + len <= c.received {
        return Ok(Resp::ok(json!({ "received": c.received })));
    }
    if offset != c.received {
        return Err(ApiError::new(409, "chunk_offset").with("received", json!(c.received)));
    }
    if !ctx.app.busy.lock().unwrap().insert(c.id.clone()) {
        return Err(ApiError::new(409, "chunk_busy").with("received", json!(c.received)));
    }
    let written = (|| -> Result<(), ApiError> {
        let st = storage(ctx)?;
        let mut f = fs::OpenOptions::new().write(true).open(st.raw(&c.project_id, &c.id)?)?;
        f.seek(SeekFrom::Start(offset as u64))?;
        f.write_all(body)?;
        if !CloudStore::new(&ctx.db, ctx.now()).received(&c.id, c.received, offset + len)? {
            return Err(ApiError::new(409, "chunk_offset"));
        }
        Ok(())
    })();
    ctx.app.busy.lock().unwrap().remove(&c.id);
    written?;
    Ok(Resp::ok(json!({ "received": offset + len })))
}

pub fn complete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    if c.status != "uploading" {
        return Err(ApiError::new(409, "cloud_not_uploading"));
    }
    if c.received != c.file_size {
        return Err(ApiError::new(409, "cloud_incomplete").with("received", json!(c.received)));
    }
    let path = storage(ctx)?.raw(&c.project_id, &c.id)?;
    let mut f = fs::File::open(path)?;
    let mut h = <sha2::Sha256 as sha2::Digest>::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let k = f.read(&mut buf)?;
        if k == 0 {
            break;
        }
        sha2::Digest::update(&mut h, &buf[..k]);
    }
    let sum = crate::util::hex(&sha2::Digest::finalize(h));
    let cs = CloudStore::new(&ctx.db, ctx.now());
    cs.enqueue(&c.id, u.id, Some(&sum))?;
    Ok(Resp::ok(json!({ "cloud": public_cloud(&cs.get(&c.id)?.unwrap(), None) })))
}

pub fn retry(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    if c.status != "failed" {
        return Err(ApiError::new(409, "cloud_not_failed"));
    }
    if !storage(ctx)?.raw(&c.project_id, &c.id)?.exists() {
        return Err(ApiError::new(409, "cloud_raw_gone"));
    }
    let cs = CloudStore::new(&ctx.db, ctx.now());
    cs.enqueue(&c.id, u.id, None)?;
    Ok(Resp::ok(json!({ "cloud": public_cloud(&cs.get(&c.id)?.unwrap(), None) })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    CloudStore::new(&ctx.db, ctx.now()).remove(&c.id)?;
    storage(ctx)?.remove(&c.project_id, &c.id);
    Ok(Resp::empty(204))
}

pub fn transforms(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    let list: Vec<Value> = CloudStore::new(&ctx.db, ctx.now()).transforms(&c.id)?.iter().map(|t| t.public(true)).collect();
    Ok(Resp::ok(json!({ "transforms": list })))
}

pub fn add_transform(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    if c.status != "ready" {
        return Err(ApiError::new(409, "cloud_not_ready"));
    }
    let b = ctx.req.json().clone();
    let m = b
        .get("matrix")
        .and_then(Value::as_array)
        .filter(|m| m.len() == 16 && m.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)));
    let Some(m) = m else { return Err(ApiError::new(422, "transform_matrix")) };
    let crs = b.get("crs").and_then(Value::as_f64).filter(|x| is_int(*x));
    let Some(crs) = crs else { return Err(ApiError::new(422, "transform_crs")) };
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let reference = match b.get("referenceCloudId").filter(|v| !v.is_null()) {
        None => None,
        Some(r) => {
            let r = cs.get(&rail_across_js::js_string(Some(r)))?;
            match r {
                Some(r) if r.project_id == c.project_id && r.id != c.id => Some(r.id),
                _ => return Err(ApiError::new(422, "transform_reference")),
            }
        }
    };
    let pairs: Vec<Value> = b
        .get("pairs")
        .and_then(Value::as_array)
        .map(|p| p.iter().take(500).cloned().collect())
        .unwrap_or_default();
    let mut params = b.get("params").filter(|p| p.is_object()).cloned().unwrap_or(json!({}));
    params["crs"] = json!(crs as i64);
    let residuals = b.get("residuals").filter(|v| !v.is_null()).cloned().unwrap_or(json!({}));
    let id = cs.add_transform(&c.id, &Value::Array(m.clone()), &params, &json!(pairs), &residuals, reference.as_deref(), u.id)?;
    let t = cs.transforms(&c.id)?.into_iter().find(|t| t.id == id).unwrap();
    let mut out = t.public(true);
    // The Node server shows a fresh transformation without its maker's name.
    out["createdByName"] = Value::Null;
    Ok(Resp::json(201, json!({ "transform": out })))
}

pub fn set_active(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    editor(&u)?;
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let id = ctx.req.field("id").filter(|v| !v.is_null()).map(|v| number(Some(v)));
    if let Some(id) = id {
        let owner = if is_int(id) { cs.transform(id as i64)?.map(|t| t.cloud_id) } else { None };
        if owner.as_deref() != Some(c.id.as_str()) {
            return Err(not_found());
        }
    }
    cs.set_active_transform(&c.id, id.map(|i| i as i64))?;
    let active = cs.active_transform(&c.id)?;
    Ok(Resp::ok(json!({ "cloud": public_cloud(&cs.get(&c.id)?.unwrap(), active.as_ref()) })))
}

fn immutable(r: Resp, etag: &str) -> Resp {
    r.header("cache-control", "private, max-age=31536000, immutable").header("etag", etag)
}

pub fn index(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    let level = level_or_404(ctx, &c)?;
    let etag = format!("\"{}-L{level}-{}\"", c.id, c.ready_at.clone().unwrap_or_else(|| "null".into()));
    if ctx.req.header("if-none-match") == Some(etag.as_str()) {
        return Ok(Resp::empty(304));
    }
    let bytes = fs::read(storage(ctx)?.index(&c.project_id, &c.id, level)?)?;
    Ok(immutable(Resp::bytes(200, "application/json", bytes), &etag))
}

pub fn tiles(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    let level = level_or_404(ctx, &c)?;
    let path = storage(ctx)?.tiles(&c.project_id, &c.id, level)?;
    let size = fs::metadata(&path)?.len();
    let etag = format!("\"{}-L{level}-{size}\"", c.id);
    if ctx.req.header("if-none-match") == Some(etag.as_str()) {
        return Ok(Resp::empty(304));
    }
    let base = |status: u16, payload: Payload| {
        immutable(
            Resp {
                status,
                headers: vec![],
                payload,
            },
            &etag,
        )
        .header("accept-ranges", "bytes")
        .header("content-type", "application/octet-stream")
    };
    let range = ctx.req.header("range").unwrap_or("");
    let parsed = range
        .strip_prefix("bytes=")
        .and_then(|r| r.split_once('-'))
        .filter(|(a, b)| a.bytes().all(|c| c.is_ascii_digit()) && b.bytes().all(|c| c.is_ascii_digit()));
    let Some((a, b)) = parsed else {
        return Ok(base(200, Payload::File(path, 0, size)));
    };
    let num = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let s = size as f64;
    let mut start = if a.is_empty() { s - num(b) } else { num(a) };
    let mut end = if a.is_empty() || b.is_empty() { s - 1.0 } else { num(b) };
    end = end.min(s - 1.0);
    if !(start >= 0.0) || start > end {
        return Ok(Resp::empty(416)
            .header("content-range", format!("bytes */{size}"))
            .header("cache-control", "private, max-age=31536000, immutable")
            .header("etag", etag.clone())
            .header("accept-ranges", "bytes"));
    }
    start = start.max(0.0);
    let len = (end - start + 1.0) as u64;
    Ok(base(206, Payload::File(path, start as u64, len)).header("content-range", format!("bytes {}-{}/{size}", start as u64, end as u64)))
}

pub fn ranges(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let c = cloud_or_404(ctx, &u)?;
    let level = level_or_404(ctx, &c)?;
    let out = read_ranges(&storage(ctx)?.tiles(&c.project_id, &c.id, level)?, ctx.req.field("ranges"))?;
    Ok(Resp::bytes(200, "application/octet-stream", out).header("cache-control", "private, no-store"))
}

pub fn admin(ctx: &mut Ctx) -> Res {
    ctx.admin()?;
    let st = storage(ctx)?;
    let (free, total) = st.disk();
    Ok(Resp::ok(json!({
        "disk": { "free": free, "total": total, "warn": free < DISK_WARN, "warnBelow": DISK_WARN },
        "quota": PROJECT_QUOTA,
        "projects": CloudStore::new(&ctx.db, ctx.now()).by_project()?,
    })))
}
