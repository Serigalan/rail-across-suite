// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The steps that change the data rather than the schema (`dataMigrations.js`),
//! each run once, before the server listens.

use rail_across_geom::chain::{snap_node_gaps, snap_track_joints};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::checks::{check_kept, normalized_record};
use crate::store::{STORED_FORMAT, STORED_FORMAT_1_0, Store, pack, unpack};
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
            let Ok(checked) = check_kept(record) else { continue };
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

/// The document a revision kept before decision 0021 holds: one of the
/// envelope before it (`schema_version` 3) with today's envelope, one kept as
/// an oltjson-1.1 record or file (2) turned into one; None where it is neither.
pub fn infra_data_of(version: i64, payload: &Value) -> Option<Value> {
    match version {
        3 => {
            let doc = rail_across_doc::door::upgrade_envelope(payload);
            rail_across_doc::door::check_document(&doc).ok()
        }
        2 => {
            let record = match payload.get("projects").and_then(Value::as_array) {
                Some(projects) => projects.first()?.clone(),
                None => payload.clone(),
            };
            rail_across_doc::persistence::project_document(&record, false, true).ok()
        }
        _ => None,
    }
}

/// Every revision kept in a format before decision 0021 rewritten as its
/// infra-data-1.0 document, in place: a revision stays the state it was, only
/// its form changes. One that cannot be read stays as it was and is refused
/// as `format_outdated`, as before.
fn to_infra_data(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    let rows: Vec<(i64, i64, Vec<u8>)> = db
        .prepare("SELECT id, schema_version, payload FROM revision WHERE schema_version IN (2, 3)")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let (mut written, mut left) = (0, 0);
    for (id, version, payload) in rows {
        match unpack(&payload).and_then(|p| infra_data_of(version, &p)) {
            Some(doc) => {
                db.execute(
                    "UPDATE revision SET schema_version = ?, payload = ? WHERE id = ?",
                    params![STORED_FORMAT, pack(&doc), id],
                )?;
                written += 1;
            }
            None => left += 1,
        }
    }
    Ok(format!("{written} Revisionen umgeschrieben, {left} nicht lesbar"))
}

/// Every revision kept as an infra-data-1.0 document rewritten as its
/// infra-data-2.0 one, in place (decision 0025): the platforms and the layout
/// into civil, the stopping places into the core. One the door cannot read
/// stays as it was, and is read through the door each time it is asked for.
fn to_infra_data_2_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    let rows: Vec<(i64, Vec<u8>)> = db
        .prepare("SELECT id, payload FROM revision WHERE schema_version = ?")?
        .query_map([STORED_FORMAT_1_0], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let (mut written, mut left) = (0, 0);
    for (id, payload) in rows {
        match unpack(&payload).and_then(|p| rail_across_doc::door::check_document(&p).ok()) {
            Some(doc) => {
                db.execute(
                    "UPDATE revision SET schema_version = ?, payload = ? WHERE id = ?",
                    params![STORED_FORMAT, pack(&doc), id],
                )?;
                written += 1;
            }
            None => left += 1,
        }
    }
    Ok(format!("{written} Revisionen umgeschrieben, {left} nicht lesbar"))
}

/// Every infra-data-2.0 revision whose domains are of an earlier schema of 2.0
/// brought to this build's, in place — the core's stopping places a platform
/// names made passenger stops (decision 0025). One the door cannot read stays
/// as it was.
fn within_infra_data_2_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    let rows: Vec<(i64, Vec<u8>)> = db
        .prepare("SELECT id, payload FROM revision WHERE schema_version = ?")?
        .query_map([STORED_FORMAT], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let (mut written, mut left) = (0, 0);
    for (id, payload) in rows {
        let Some(given) = unpack(&payload) else {
            left += 1;
            continue;
        };
        match rail_across_doc::door::check_document(&given) {
            Ok(doc) if doc != given => {
                db.execute("UPDATE revision SET payload = ? WHERE id = ?", params![pack(&doc), id])?;
                written += 1;
            }
            Ok(_) => {}
            Err(_) => left += 1,
        }
    }
    Ok(format!("{written} Revisionen umgeschrieben, {left} nicht lesbar"))
}

/// Run the data steps this database has not seen, each once: (name, note) of those run.
pub fn run_data_migrations(db: &Connection, now: i64) -> rusqlite::Result<Vec<(String, String)>> {
    let done: Vec<String> = db
        .prepare("SELECT name FROM data_migration")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut ran = Vec::new();
    // In this order: a later step reads the revisions in the form the first gives them.
    let steps: [(&str, fn(&Connection, i64) -> rusqlite::Result<String>); 4] = [
        ("0021-infra-data", to_infra_data),
        ("0020-node-tolerance-1um", close_node_gaps),
        ("0025-infra-data-2.0", to_infra_data_2_0),
        ("0025-passenger-stops-and-runs", within_infra_data_2_0),
    ];
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
