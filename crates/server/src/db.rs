// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The database (`db.js`): a file in WAL mode with foreign keys on, brought
//! up to date by the numbered SQL files of `server/migrations/` it has not
//! seen — the Node server's own, built in, so either server opens the
//! other's database.

use rusqlite::{Connection, OptionalExtension, params};

/// The migrations, in order: their file names and SQL.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_users.sql", include_str!("../../../server/migrations/001_users.sql")),
    ("002_projects.sql", include_str!("../../../server/migrations/002_projects.sql")),
    ("003_revision_image.sql", include_str!("../../../server/migrations/003_revision_image.sql")),
    ("004_project_template.sql", include_str!("../../../server/migrations/004_project_template.sql")),
    ("005_project_member.sql", include_str!("../../../server/migrations/005_project_member.sql")),
    ("006_point_cloud.sql", include_str!("../../../server/migrations/006_point_cloud.sql")),
    ("007_cloud_run.sql", include_str!("../../../server/migrations/007_cloud_run.sql")),
    ("008_share_link.sql", include_str!("../../../server/migrations/008_share_link.sql")),
    ("009_vehicle.sql", include_str!("../../../server/migrations/009_vehicle.sql")),
    ("010_timetable.sql", include_str!("../../../server/migrations/010_timetable.sql")),
    ("011_data_migration.sql", include_str!("../../../server/migrations/011_data_migration.sql")),
];

/// Open the database (a file, or ":memory:") and bring its schema up to date.
pub fn open_database(file: &str, now_iso: &str) -> rusqlite::Result<Connection> {
    let db = Connection::open(file)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "foreign_keys", "ON")?;
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    migrate(&db, now_iso)?;
    Ok(db)
}

fn migrate(db: &Connection, now_iso: &str) -> rusqlite::Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migration (
    version    INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    applied_at TEXT NOT NULL
  )",
    )?;
    let done: Vec<i64> = db
        .prepare("SELECT version FROM schema_migration")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for (file, sql) in MIGRATIONS {
        let version: i64 = file.split('_').next().unwrap_or("0").parse().unwrap_or(0);
        if done.contains(&version) {
            continue;
        }
        db.execute_batch("BEGIN")?;
        let applied = db.execute_batch(sql).and_then(|_| {
            db.execute(
                "INSERT INTO schema_migration (version, name, applied_at) VALUES (?, ?, ?)",
                params![version, file, now_iso],
            )
            .map(|_| ())
        });
        match applied {
            Ok(()) => db.execute_batch("COMMIT")?,
            Err(e) => {
                db.execute_batch("ROLLBACK")?;
                return Err(e);
            }
        }
    }
    Ok(())
}

/// The schema version a database is at.
pub fn schema_version(db: &Connection) -> i64 {
    db.query_row("SELECT MAX(version) FROM schema_migration", [], |r| r.get::<_, Option<i64>>(0))
        .optional()
        .ok()
        .flatten()
        .flatten()
        .unwrap_or(0)
}

/// A consistent copy of the database (`db.backup`).
pub fn backup(db: &Connection, file: &str) -> rusqlite::Result<()> {
    db.execute("VACUUM INTO ?", [file]).map(|_| ())
}
