// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The steps that change the data rather than the schema (`dataMigrations.js`),
//! each run once, before the server listens.

use rail_across_geom::chain::{snap_node_gaps, snap_track_joints};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::checks::{check_record, normalized_record};
use crate::store::Store;
use crate::util::{hex, iso, random_bytes};

/// The login of the user the server's own revisions name.
pub const SYSTEM_LOGIN: &str = "system";
/// What two places closer than were the same place before NODE_TOL was 1 µm [m].
pub const LEGACY_NODE_TOL: f64 = 1e-3;

fn system_user(db: &Connection, now: i64) -> rusqlite::Result<i64> {
    let get = || {
        db.query_row("SELECT id FROM user WHERE login = ?", [SYSTEM_LOGIN], |r| r.get::<_, i64>(0))
            .optional()
    };
    if let Some(id) = get()? {
        return Ok(id);
    }
    db.execute(
        "INSERT INTO user (login, name, password_hash, role, active, must_change_password, created_at) VALUES (?, 'Rail-Across-Suite', ?, 'user', 0, 0, ?)",
        params![SYSTEM_LOGIN, hex(&random_bytes(32)), iso(now)],
    )?;
    Ok(get()?.unwrap())
}

/// A record kept while the same place was 1 mm, closed to what it is now (`closeLegacyGaps`): (record, closed).
pub fn close_legacy_gaps(record: &Value) -> (Value, usize) {
    let mut closed = 0;
    let tracks: Vec<Value> = record
        .get("tracks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|t| {
            let els = t.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
            let epsg = rail_across_js::js_number(t.get("epsg"));
            let (elements, snapped) = snap_node_gaps(&els, if epsg.is_finite() { epsg as i64 } else { 0 }, LEGACY_NODE_TOL);
            closed += snapped.len();
            if snapped.is_empty() {
                t
            } else {
                let mut o = t.as_object().cloned().unwrap_or_default();
                o.insert("elements".into(), Value::Array(elements));
                Value::Object(o)
            }
        })
        .collect();
    let (joined, snapped) = snap_track_joints(&tracks, LEGACY_NODE_TOL);
    closed += snapped.len();
    if closed == 0 {
        return (record.clone(), 0);
    }
    let mut o = record.as_object().cloned().unwrap_or_default();
    o.insert("tracks".into(), Value::Array(joined));
    (Value::Object(o), closed)
}

fn close_node_gaps(db: &Connection, now: i64) -> rusqlite::Result<String> {
    let projects: Vec<String> = db
        .prepare("SELECT id FROM project WHERE deleted_at IS NULL ORDER BY created_at")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let store = Store::new(db, now);
    let (mut written, mut joints) = (0, 0);
    for project in projects {
        for v in store.variants(&project)? {
            if v.archived {
                continue;
            }
            let Ok(Some(head)) = store.revision(v.head_revision_id) else { continue };
            let (record, closed) = close_legacy_gaps(&normalized_record(&head.payload));
            if closed == 0 {
                continue;
            }
            let Ok(checked) = check_record(&record, None) else { continue };
            let author = system_user(db, now)?;
            let message = format!("{closed} Stöße bis 1 mm geschlossen – derselbe Ort ist jetzt 1 µm (Entscheidung 0020).");
            if let Ok(Ok(_)) = store.check_in(
                &v,
                v.head_revision_id,
                None,
                author,
                &message,
                &checked.record,
                &json!([]),
                &checked.error_keys(),
            ) {
                written += 1;
                joints += closed;
            }
        }
    }
    Ok(format!("{written} Revisionen, {joints} Stöße"))
}

/// Run the data steps this database has not seen, each once: (name, note) of those run.
pub fn run_data_migrations(db: &Connection, now: i64) -> rusqlite::Result<Vec<(String, String)>> {
    let done: Vec<String> = db
        .prepare("SELECT name FROM data_migration")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut ran = Vec::new();
    let steps: [(&str, fn(&Connection, i64) -> rusqlite::Result<String>); 1] = [("0020-node-tolerance-1um", close_node_gaps)];
    for (name, step) in steps {
        if done.iter().any(|d| d == name) {
            continue;
        }
        let note = step(db, now)?;
        db.execute(
            "INSERT INTO data_migration (name, applied_at, note) VALUES (?, ?, ?)",
            params![name, iso(now), note],
        )?;
        ran.push((name.to_string(), note));
    }
    Ok(ran)
}
