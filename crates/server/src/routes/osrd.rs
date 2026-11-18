// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! OSRD's nets for the Fahrplan-Sim's *Fahrzeit rechnen*: the RailJSON files laid by hand into the server's
//! directory of them (`RA_SERVER_OSRD`), listed and handed out to whoever is signed in, never written here.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::app::Ctx;
use crate::http::{ApiError, Payload, Res, Resp};
use crate::routes::not_found;
use crate::util::iso;

/// A file's id: its name without `.json`, of letters, digits, `.`, `-` and `_`, not starting with a dot.
fn id_of(name: &str) -> Option<&str> {
    let id = name.strip_suffix(".json")?;
    let ok = !id.is_empty() && !id.starts_with('.') && id.len() <= 200 && id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_'));
    ok.then_some(id)
}

fn root(ctx: &Ctx) -> Result<PathBuf, ApiError> {
    ctx.app.settings.osrd_root.clone().ok_or_else(|| ApiError::new(404, "osrd_off"))
}

/// The nets in the directory, by name: `{ id, name, size, updatedAt }` — the name the id without `.railjson`.
pub fn list(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let dir = root(ctx)?;
    let mut nets: Vec<Value> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = id_of(&name)?.to_string();
            let meta = entry.metadata().ok().filter(|m| m.is_file())?;
            let at = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| iso(d.as_millis() as i64));
            let label = id.strip_suffix(".railjson").unwrap_or(&id).to_string();
            Some(json!({ "id": id, "name": label, "size": meta.len(), "updatedAt": at }))
        })
        .collect();
    nets.sort_by_key(|n| n["name"].as_str().unwrap_or_default().to_lowercase());
    Ok(Resp::ok(json!({ "nets": nets })))
}

/// One net: the file as it lies there.
pub fn get(ctx: &mut Ctx) -> Res {
    ctx.user()?;
    let dir = root(ctx)?;
    let file = format!("{}.json", ctx.req.param("id"));
    if id_of(&file).is_none() {
        return Err(not_found());
    }
    let path = dir.join(&file);
    let size = std::fs::metadata(&path).ok().filter(|m| m.is_file()).ok_or_else(not_found)?.len();
    Ok(Resp {
        status: 200,
        headers: vec![
            ("content-type".into(), "application/json; charset=utf-8".into()),
            ("cache-control".into(), "private, no-cache".into()),
        ],
        payload: Payload::File(path, 0, size),
    })
}

#[cfg(test)]
mod tests {
    use super::id_of;

    #[test]
    fn a_file_is_named_safely() {
        assert_eq!(id_of("morocco.railjson.json"), Some("morocco.railjson"));
        assert_eq!(id_of("a_b-1.json"), Some("a_b-1"));
        for bad in [".hidden.json", "../x.json", "a b.json", "x.txt", ".json"] {
            assert_eq!(id_of(bad), None, "{bad}");
        }
    }
}
