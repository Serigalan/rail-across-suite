// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What the Layout-Tool cannot do in the browser (`routes/terrain.js`, decision 0005):
//! the Länder's DGM1 for points and as relief tiles, and the MDB import.

use serde_json::Value;

use crate::app::Ctx;
use crate::http::{ApiError, Body, Res, Resp};
use crate::mdb::{MdbError, convert_mdb};
use crate::terrain::dem::{MAX_ZOOM, MIN_ZOOM, render};

fn env_num(name: &str, d: f64) -> f64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

pub fn max_mdb_bytes() -> usize {
    env_num("RA_MDB_MAX_BODY", 128.0 * 1024.0 * 1024.0) as usize
}

const DEM_MAX_AGE: i64 = 7 * 24 * 3600;

pub fn terrain(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let Some(points) = ctx.req.field("lnglat").and_then(Value::as_array) else {
        return Err(ApiError::new(400, "invalid_payload"));
    };
    if points.len() as f64 > env_num("RA_TERRAIN_MAX_POINTS", 20000.0) {
        return Err(ApiError::new(413, "too_large"));
    }
    Ok(Resp::ok(crate::terrain::sample(points, &ctx.app.terrain)))
}

pub fn dem(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let int = |s: &str| s.parse::<f64>().ok().filter(|v| v.fract() == 0.0);
    let (z, x) = (int(ctx.req.param("z")), int(ctx.req.param("x")));
    let file = ctx.req.param("file");
    let y = file
        .strip_suffix(".png")
        .filter(|d| (1..=7).contains(&d.len()) && d.bytes().all(|c| c.is_ascii_digit()))
        .and_then(int);
    let (Some(z), Some(x), Some(y)) = (z, x, y) else {
        return Err(ApiError::new(400, "invalid_payload"));
    };
    let (z, x, y) = (z as i64, x as i64, y as i64);
    if z < MIN_ZOOM || z > MAX_ZOOM || x >= 1 << z || y >= 1 << z {
        return Err(ApiError::new(400, "invalid_payload"));
    }
    let (png, keep) = render(z, x, y, &ctx.app.terrain, &ctx.app.dgm5).map_err(|e| {
        eprintln!("dem: {e}");
        ApiError::new(500, "internal")
    })?;
    let cache = if keep { format!("private, max-age={DEM_MAX_AGE}") } else { "no-store".into() };
    match png {
        None => Ok(Resp::json(404, serde_json::json!({ "error": "not_found" })).header("cache-control", cache)),
        Some(png) => Ok(Resp::bytes(200, "image/png", png).header("cache-control", cache)),
    }
}

pub fn mdb(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let bytes = match &ctx.req.body {
        Body::Bytes(b) if !b.is_empty() => b.clone(),
        _ => return Err(ApiError::new(400, "invalid_payload")),
    };
    match convert_mdb(&bytes) {
        Ok(v) => Ok(Resp::ok(v)),
        Err(MdbError { code, message }) => {
            if code == "internal" {
                eprintln!("mdb: {message}");
            }
            let status = match code.as_str() {
                "not_a_database" => 400,
                "no_records" => 422,
                "timeout" => 504,
                _ => 500,
            };
            Err(ApiError::new(status, &code))
        }
    }
}
