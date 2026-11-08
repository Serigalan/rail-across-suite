// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Finding a user to add to a project (`routes/users.js`, decision 127).

use serde_json::json;

use crate::app::Ctx;
use crate::http::{ApiError, Res, Resp};

const SEARCH_LIMIT: i64 = 20;

pub fn search(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let text = ctx.req.query.get("q").map(|q| rail_across_js::js_trim(q).to_string()).unwrap_or_default();
    if text.encode_utf16().count() < 2 {
        return Err(ApiError::new(422, "query_too_short"));
    }
    let cut: String = text.chars().take(100).collect();
    let mut q = String::from("%");
    for c in cut.chars() {
        if matches!(c, '\\' | '%' | '_') {
            q.push('\\');
        }
        q.push(c);
    }
    q.push('%');
    let users: Vec<serde_json::Value> = ctx
        .db
        .prepare(&format!(
            "SELECT id, login, name FROM user WHERE active = 1 AND (login LIKE ?1 ESCAPE '\\' OR name LIKE ?1 ESCAPE '\\') ORDER BY name COLLATE NOCASE LIMIT {SEARCH_LIMIT}"
        ))?
        .query_map([&q], |r| Ok(json!({ "id": r.get::<_, i64>(0)?, "login": r.get::<_, String>(1)?, "name": r.get::<_, String>(2)? })))?
        .collect::<Result<_, _>>()?;
    Ok(Resp::ok(json!({ "users": users })))
}
