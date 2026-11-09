// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Long runs over a project's clouds (`routes/runs.js`, AP 13.7).

use serde_json::{Map, Value, json};

use crate::app::Ctx;
use crate::auth::User;
use crate::clouds::store::{CloudStore, RunStore, public_run};
use crate::http::{ApiError, Res, Resp};
use crate::routes::{not_found, number};
use crate::store::Store;

const ACTIVE_LIMIT: i64 = 4;

fn finite(v: &Value) -> bool {
    v.as_f64().is_some_and(f64::is_finite)
}

fn is_ring(v: Option<&Value>) -> bool {
    v.and_then(Value::as_array)
        .is_some_and(|r| r.len() >= 3 && r.iter().all(|p| p.as_array().is_some_and(|p| p.len() >= 2 && p.iter().all(finite))))
}

fn is_track(t: Option<&Value>) -> bool {
    t.is_some_and(|t| {
        t.is_object() && t.get("id").is_some_and(|i| !i.is_null()) && t.get("elements").is_some_and(Value::is_array) && {
            let e = number(t.get("epsg"));
            e.is_finite() && e.fract() == 0.0
        }
    })
}

fn or_array(v: Option<&Value>) -> bool {
    v.is_none_or(|v| v.is_null() || v.is_array())
}

/// What a run needs, checked for its shape (`checkParams`).
fn check_params(kind: &str, p: Option<&Value>) -> bool {
    let Some(p) = p.filter(|p| p.is_object()) else { return false };
    if kind == "clearance" {
        return is_track(p.get("track")) && is_ring(p.get("ring")) && or_array(p.get("areas")) && or_array(p.get("tracks")) && or_array(p.get("switches"));
    }
    if kind == "trace" {
        let g = p.get("guide");
        if !p.get("rail").is_some_and(Value::is_string) || !g.is_some_and(Value::is_object) {
            return false;
        }
        let g = g.unwrap();
        return match g.get("kind").and_then(Value::as_str) {
            Some("track") => is_track(g.get("track")),
            Some("line") => {
                g.get("epsg").and_then(Value::as_f64).is_some_and(|e| e.is_finite() && e.fract() == 0.0)
                    && g.get("vertices")
                        .and_then(Value::as_array)
                        .is_some_and(|v| v.len() >= 2 && v.iter().all(|x| x.as_array().is_some_and(|x| x.len() == 2 && x.iter().all(finite))))
            }
            _ => false,
        };
    }
    false
}

fn project_or_404(ctx: &Ctx, u: &User) -> Result<String, ApiError> {
    crate::routes::clouds::storage(ctx)?;
    let s = Store::new(&ctx.db, ctx.now());
    match s.project(ctx.req.param("id"))? {
        Some(p) if s.can_see(u, &p)? => Ok(p.id),
        _ => Err(not_found()),
    }
}

fn editor(u: &User) -> Result<(), ApiError> {
    if u.may_edit_clouds() {
        Ok(())
    } else {
        Err(ApiError::new(403, "clouds_not_allowed"))
    }
}

fn run_or_404(ctx: &Ctx, u: &User) -> Result<Value, ApiError> {
    let p = project_or_404(ctx, u)?;
    let id = number(Some(&json!(ctx.req.param("rid"))));
    let r = if id.is_finite() && id.fract() == 0.0 {
        RunStore::new(&ctx.db, ctx.now()).get(id as i64)?
    } else {
        None
    };
    match r {
        Some(r) if r["project_id"].as_str() == Some(p.as_str()) => Ok(r),
        _ => Err(not_found()),
    }
}

pub fn list(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, &u)?;
    let runs: Vec<Value> = RunStore::new(&ctx.db, ctx.now()).list(&p)?.iter().map(|r| public_run(r, false)).collect();
    Ok(Resp::ok(json!({ "runs": runs, "mayEdit": u.may_edit_clouds() })))
}

pub fn create(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let p = project_or_404(ctx, &u)?;
    editor(&u)?;
    let b = ctx.req.json();
    let kind = b.get("kind").and_then(Value::as_str).unwrap_or("");
    if kind != "clearance" && kind != "trace" {
        return Err(ApiError::new(422, "run_kind"));
    }
    if !check_params(kind, b.get("params")) {
        return Err(ApiError::new(422, "run_params"));
    }
    let cs = CloudStore::new(&ctx.db, ctx.now());
    let placed = cs.active_transforms(&p)?;
    let ready: Vec<String> = cs
        .list(&p)?
        .into_iter()
        .filter(|c| c.status == "ready" && (c.crs.is_some() || placed.contains_key(&c.id)))
        .map(|c| c.id)
        .collect();
    if ready.is_empty() {
        return Err(ApiError::new(409, "run_no_clouds"));
    }
    let runs = RunStore::new(&ctx.db, ctx.now());
    if runs.active(&p)? >= ACTIVE_LIMIT {
        return Err(ApiError::new(429, "runs_busy").with("limit", json!(ACTIVE_LIMIT)));
    }
    runs.prune()?;
    let subject = match b.get("subject").and_then(Value::as_object) {
        Some(o) => {
            let mut m = Map::new();
            for (k, v) in o.iter().take(8) {
                m.insert(k.clone(), json!(rail_across_js::js_string(Some(v)).chars().take(200).collect::<String>()));
            }
            Value::Object(m)
        }
        None => json!({}),
    };
    let id = runs.create(&p, kind, &subject, &b["params"], &json!(ready), u.id)?;
    Ok(Resp::json(201, json!({ "run": public_run(&runs.get(id)?.unwrap(), false) })))
}

pub fn get(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    Ok(Resp::ok(json!({ "run": public_run(&run_or_404(ctx, &u)?, true) })))
}

pub fn delete(ctx: &mut Ctx) -> Res {
    let u = ctx.user()?.clone();
    let r = run_or_404(ctx, &u)?;
    editor(&u)?;
    let runs = RunStore::new(&ctx.db, ctx.now());
    let id = r["id"].as_i64().unwrap();
    if matches!(r["status"].as_str(), Some("queued" | "running")) {
        runs.finish(id, "cancelled", None)?;
        return Ok(Resp::ok(json!({ "run": public_run(&runs.get(id)?.unwrap(), false) })));
    }
    runs.remove(id)?;
    Ok(Resp::empty(204))
}
