// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The steps that change the data rather than the schema (`dataMigrations.js`),
//! each run once, before the server listens.

use rail_across_geom::chain::{snap_node_gaps, snap_track_joints};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::checks::{check_kept, normalized_record};
use crate::store::{
    STORED_FORMAT, STORED_FORMAT_1_0, STORED_FORMAT_CORE_4_0, STORED_FORMAT_CORE_5_0, STORED_FORMAT_LIFESPANS, STORED_FORMAT_REFERENCES, STORED_FORMAT_STAGES,
    Store, pack, unpack,
};
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
            let message = format!("{closed} Stöße bis 1 mm geschlossen – derselbe Ort ist jetzt 1 µm (Entscheidung 0005).");
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

/// The document a revision kept before decision 0006 holds: one of the
/// envelope before it (`schema_version` 3) with today's envelope, one kept as
/// an oltjson-1.1 record or file (2) turned into one — its planning statuses
/// lifespans with their days open, its station texts operational points
/// (decision 0006) —; None where it is neither.
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
            let record = rail_across_doc::core_format::with_station_points(&lived(&record));
            rail_across_doc::persistence::project_document(&record, false, true).ok()
        }
        _ => None,
    }
}

/// An oltjson-1.1 record's planning statuses as lifespans (decision 0007): new
/// begins on a day open, removal ends on one, existing states none.
fn lived(record: &Value) -> Value {
    let mut out = record.clone();
    for list in ["tracks", "switches", "platforms", "endMarks"] {
        for o in out.get_mut(list).and_then(Value::as_array_mut).into_iter().flatten() {
            let Some(o) = o.as_object_mut() else { continue };
            match o.remove("status").as_ref().and_then(Value::as_str) {
                Some("new") => o.insert("lifespan".into(), json!({ "from": "open" })),
                Some("removal") => o.insert("lifespan".into(), json!({ "to": "open" })),
                _ => None,
            };
        }
    }
    out
}

/// Every revision kept in a format before decision 0006 rewritten as its
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

/// Every revision kept as an infra-data-1.0 document rewritten as today's,
/// in place (decision 0008): the platforms and the layout into civil, the
/// stopping places into the core. One the door cannot read stays as it was,
/// and is read through the door each time it is asked for.
fn from_infra_data_1_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_1_0)
}

/// Every revision kept as an infra-data-2.0 document with its register of
/// construction stages rewritten as today's, in place: its domains of an
/// earlier schema of 2.0 brought to 2.0's last (decisions 0008 and 0007), and its
/// lifespans stated by day, without the register (decision 0007). One the door
/// cannot read stays as it was, and is read through the door each time it is
/// asked for.
fn from_infra_data_2_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_STAGES)
}

/// Every revision kept with its lifespans by day rewritten as today's, in
/// place (decision 0006): a reference that names nothing — the tools
/// reported one instead before — cut as the deletion of what it named would
/// have cut it. One the door cannot read stays as it was, and is read through
/// the door each time it is asked for.
fn from_lifespans_by_day(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_LIFESPANS)
}

/// Every revision kept with its references naming something rewritten as the door reads it, in place — core 4.0
/// when the step came (decision 0006): the kinds as words, the references ending in `Id`, every id
/// a UUID, a km line in one plane as runs; core 5.0 since. One the door cannot read stays as it was, and
/// is read through the door each time it is asked for.
fn from_references(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_REFERENCES)
}

/// Every revision kept as core 4.0 rewritten as core 5.0 states it, in place (decision 0006): a switch's
/// `B1` its branch and `B2` its main track again. One the door cannot read stays as it was, and is read through the
/// door each time it is asked for.
fn from_core_4_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_CORE_4_0)
}

/// Every revision kept as core 5.0 rewritten as core 6.0 states it, in place (decision 0006): a track's plan by
/// column. One whose elements lie more than a millimetre apart the door does not read: it stays as it was, and is
/// refused each time it is asked for.
fn from_core_5_0(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    through_the_door(db, STORED_FORMAT_CORE_5_0)
}

/// Every revision of a project's operations kept with one domain `operations` rewritten in the two domains
/// `vehicles` and `timetables`, in place (decision 0006): a train's point naming an end mark of the
/// project's infrastructure — any revision of any variant — by `endMarkId`, else its operational point. One the
/// door cannot read stays as it was, and is read through the door each time it is asked for.
fn to_two_domains(db: &Connection, _now: i64) -> rusqlite::Result<String> {
    let rows: Vec<(i64, String, Vec<u8>)> = db
        .prepare("SELECT id, project_id, payload FROM operations_revision")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut marks_of: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
    let (mut written, mut left) = (0, 0);
    for (id, project, payload) in rows {
        let Some(doc) = unpack(&payload) else {
            left += 1;
            continue;
        };
        if doc["domains"].get("operations").is_none() {
            continue;
        }
        if !marks_of.contains_key(&project) {
            let payloads: Vec<Vec<u8>> = db
                .prepare("SELECT payload FROM revision WHERE project_id = ?")?
                .query_map([&project], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            let mut marks = std::collections::HashSet::new();
            for p in payloads {
                let Some(infra) = unpack(&p).and_then(|p| rail_across_doc::door::check_document(&p).ok()) else {
                    continue;
                };
                for m in infra["domains"]["core"]["collections"]["endMarks"].as_array().into_iter().flatten() {
                    if let Some(mark) = m.get("id").and_then(Value::as_str) {
                        marks.insert(mark.to_string());
                    }
                }
            }
            marks_of.insert(project.clone(), marks);
        }
        let next = rail_across_doc::operations::from_before(&doc, &marks_of[&project]);
        match rail_across_doc::operations::check_operations(&next) {
            Ok(next) => {
                db.execute(
                    "UPDATE operations_revision SET payload = ?, hash = ? WHERE id = ?",
                    params![pack(&next), rail_across_doc::manifest::document_hash(&next), id],
                )?;
                written += 1;
            }
            Err(_) => left += 1,
        }
    }
    Ok(format!("{written} Revisionen des Betriebs umgeschrieben, {left} nicht lesbar"))
}

/// Every revision kept in the form `from` rewritten as the door reads it, in today's form.
fn through_the_door(db: &Connection, from: i64) -> rusqlite::Result<String> {
    let rows: Vec<(i64, Vec<u8>)> = db
        .prepare("SELECT id, payload FROM revision WHERE schema_version = ?")?
        .query_map([from], |r| Ok((r.get(0)?, r.get(1)?)))?
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

/// Every project's timetables kept as rows of `timetable` (decision 0012) made
/// the first revision of its operations (operations-1.0, O2; decision 0012),
/// each timetable under its id, its maker and last saver kept. One the door
/// does not take is left out — it stays in its row — and counted.
fn to_operations(db: &Connection, now: i64) -> rusqlite::Result<String> {
    type Row = (String, String, String, String, i64, String, i64, String);
    let rows: Vec<Row> = db
        .prepare(
            "SELECT id, project_id, name, data, created_by, created_at, updated_by, updated_at FROM timetable
             WHERE deleted_at IS NULL ORDER BY project_id, created_at, rowid",
        )?
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
        })?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok("0 Fahrpläne".into());
    }
    let system = system_user(db, now)?;
    let (mut projects, mut taken, mut left) = (0, 0, 0);
    let mut i = 0;
    while i < rows.len() {
        let project = rows[i].1.clone();
        let mine: Vec<&Row> = rows[i..].iter().take_while(|r| r.1 == project).collect();
        i += mine.len();
        let kept: i64 = db.query_row("SELECT COUNT(*) FROM operations_revision WHERE project_id = ?", [&project], |r| r.get(0))?;
        if kept > 0 {
            continue;
        }
        let mut list = Vec::new();
        let mut ids = Vec::new();
        for (id, _, name, data, ..) in &mine {
            let data: Value = serde_json::from_str(data).unwrap_or(Value::Null);
            if crate::routes::timetables::timetable_fault(&data).is_some() {
                left += 1;
                continue;
            }
            // The timetable under the id its document names it by (decision 0006).
            let id = rail_across_doc::ids::uuid_of(id);
            list.push(json!({ "id": id, "name": name, "data": data }));
            ids.push(id);
        }
        if list.is_empty() {
            continue;
        }
        let record = json!({ "timetables": list });
        if crate::operations::commit(db, &project, 0, &record, &ids, system, now).is_err() {
            left += ids.len();
            continue;
        }
        for (id, _, _, _, created_by, created_at, updated_by, updated_at) in &mine {
            db.execute(
                "UPDATE operations_timetable SET created_by = ?, created_at = ?, updated_by = ?, updated_at = ? WHERE project_id = ? AND timetable_id = ?",
                params![created_by, created_at, updated_by, updated_at, project, rail_across_doc::ids::uuid_of(id)],
            )?;
        }
        projects += 1;
        taken += ids.len();
    }
    Ok(format!("{taken} Fahrpläne in {projects} Projekten, {left} nicht lesbar"))
}

/// Run the data steps this database has not seen, each once: (name, note) of those run.
pub fn run_data_migrations(db: &Connection, now: i64) -> rusqlite::Result<Vec<(String, String)>> {
    let done: Vec<String> = db
        .prepare("SELECT name FROM data_migration")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut ran = Vec::new();
    // In this order: a later step reads the revisions in the form the first gives them.
    let steps: [(&str, fn(&Connection, i64) -> rusqlite::Result<String>); 13] = [
        ("0021-infra-data", to_infra_data),
        ("0020-node-tolerance-1um", close_node_gaps),
        ("0025-infra-data-2.0", from_infra_data_1_0),
        ("0025-passenger-stops-and-runs", from_infra_data_2_0),
        ("0026-operations", to_operations),
        // The core's schema with days on its stages (decision 0007): the revisions named by it, as they were.
        ("0027-stage-days", from_infra_data_2_0),
        // Civil's schema with bridges, retaining walls and noise barriers: the revisions named by it, as they were.
        ("0027-civil-structures", from_infra_data_2_0),
        // No stages, the lifespans by day (decision 0007): every revision kept before it.
        ("0028-lifespans-by-day", from_infra_data_2_0),
        // A reference names something (decision 0006): every revision a build of phase A or B kept.
        ("0028-references", from_lifespans_by_day),
        // Core 4.0 of rail-across-1.0 (decision 0006): every revision kept before it.
        ("0029-core-4.0", from_references),
        // Operations in the domains vehicles and timetables (decision 0006): every revision of operations.
        ("0029-operations", to_two_domains),
        // Core 5.0 (decision 0006): a switch's B1 its branch again, in every revision kept as core 4.0.
        ("0029-switch-ports", from_core_4_0),
        // Core 6.0 (decision 0006): a track's plan by column, in every revision kept as core 5.0.
        ("0030-alignment-columns", from_core_5_0),
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
