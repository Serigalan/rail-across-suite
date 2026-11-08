// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The point clouds, their jobs, re-referencings and long runs on the
//! database (`cloudStore.js`, `runStore.js`).

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::util::{iso, n, uuid};

/// A point cloud row.
#[derive(Clone, Debug)]
pub struct Cloud {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub file_name: String,
    pub file_size: i64,
    pub format: String,
    pub crs: Option<i64>,
    pub height_epsg: i64,
    pub status: String,
    pub received: i64,
    pub sha256: Option<String>,
    pub keep_raw: bool,
    pub source_points: i64,
    pub points: Option<i64>,
    pub bytes: i64,
    pub rgb: bool,
    pub bounds: Option<String>,
    pub levels: String,
    pub progress: Option<f64>,
    pub error: Option<String>,
    pub created_by: i64,
    pub created_at: String,
    pub ready_at: Option<String>,
}

impl Cloud {
    fn from_row(r: &Row) -> rusqlite::Result<Cloud> {
        Ok(Cloud {
            id: r.get("id")?,
            project_id: r.get("project_id")?,
            name: r.get("name")?,
            file_name: r.get("file_name")?,
            file_size: r.get("file_size")?,
            format: r.get("format")?,
            crs: r.get("crs")?,
            height_epsg: r.get("height_epsg")?,
            status: r.get("status")?,
            received: r.get("received")?,
            sha256: r.get("sha256")?,
            keep_raw: r.get::<_, i64>("keep_raw")? != 0,
            source_points: r.get("source_points")?,
            points: r.get("points")?,
            bytes: r.get("bytes")?,
            rgb: r.get::<_, i64>("rgb")? != 0,
            bounds: r.get("bounds")?,
            levels: r.get("levels")?,
            progress: r.get("progress")?,
            error: r.get("error")?,
            created_by: r.get("created_by")?,
            created_at: r.get("created_at")?,
            ready_at: r.get("ready_at")?,
        })
    }
}

/// A transformation row.
#[derive(Clone, Debug)]
pub struct Transform {
    pub id: i64,
    pub cloud_id: String,
    pub matrix: String,
    pub params: String,
    pub pairs: String,
    pub residuals: String,
    pub reference_cloud_id: Option<String>,
    pub active: bool,
    pub created_by: i64,
    pub created_at: String,
    pub created_by_name: Option<String>,
}

impl Transform {
    fn from_row(r: &Row) -> rusqlite::Result<Transform> {
        Ok(Transform {
            id: r.get("id")?,
            cloud_id: r.get("cloud_id")?,
            matrix: r.get("matrix")?,
            params: r.get("params")?,
            pairs: r.get("pairs")?,
            residuals: r.get("residuals")?,
            reference_cloud_id: r.get("reference_cloud_id")?,
            active: r.get::<_, i64>("active")? != 0,
            created_by: r.get("created_by")?,
            created_at: r.get("created_at")?,
            created_by_name: r.get("created_by_name").ok(),
        })
    }

    /// As the API shows it (`publicTransform`); `full` with pairs and residuals.
    pub fn public(&self, full: bool) -> Value {
        let parse = |s: &str, d: Value| serde_json::from_str::<Value>(s).unwrap_or(d);
        let params = parse(if self.params.is_empty() { "{}" } else { &self.params }, json!({}));
        let mut v = json!({
            "id": self.id, "cloudId": self.cloud_id, "matrix": parse(&self.matrix, Value::Null),
            "crs": params.get("crs").cloned().unwrap_or(Value::Null), "active": self.active,
            "referenceCloudId": self.reference_cloud_id, "createdBy": self.created_by, "createdAt": self.created_at,
        });
        if full {
            v["createdByName"] = json!(self.created_by_name);
            v["params"] = params;
            v["pairs"] = parse(if self.pairs.is_empty() { "[]" } else { &self.pairs }, json!([]));
            v["residuals"] = parse(if self.residuals.is_empty() { "{}" } else { &self.residuals }, json!({}));
        }
        v
    }
}

/// A cloud as the API shows it (`publicCloud`), with the transformation in force.
pub fn public_cloud(c: &Cloud, transform: Option<&Transform>) -> Value {
    json!({
        "id": c.id, "projectId": c.project_id, "name": c.name,
        "file": { "name": c.file_name, "size": c.file_size, "format": c.format },
        "crs": c.crs, "heightEpsg": c.height_epsg, "status": c.status, "received": c.received,
        "progress": c.progress.map(n), "error": c.error, "keepRaw": c.keep_raw, "sourcePoints": c.source_points,
        "points": c.points, "bytes": c.bytes, "rgb": c.rgb,
        "bounds": c.bounds.as_deref().and_then(|b| serde_json::from_str::<Value>(b).ok()),
        "levels": serde_json::from_str::<Value>(if c.levels.is_empty() { "[]" } else { &c.levels }).unwrap_or(json!([])),
        "createdBy": c.created_by, "createdAt": c.created_at, "readyAt": c.ready_at,
        "transform": transform.map(|t| t.public(false)),
    })
}

pub struct CloudStore<'a> {
    pub db: &'a Connection,
    pub now: i64,
}

impl<'a> CloudStore<'a> {
    pub fn new(db: &'a Connection, now: i64) -> Self {
        CloudStore { db, now }
    }

    fn iso(&self) -> String {
        iso(self.now)
    }

    pub fn list(&self, project: &str) -> rusqlite::Result<Vec<Cloud>> {
        self.db
            .prepare("SELECT * FROM point_cloud WHERE project_id = ? ORDER BY created_at DESC")?
            .query_map([project], Cloud::from_row)?
            .collect()
    }

    pub fn get(&self, id: &str) -> rusqlite::Result<Option<Cloud>> {
        self.db.query_row("SELECT * FROM point_cloud WHERE id = ?", [id], Cloud::from_row).optional()
    }

    pub fn create(
        &self,
        project: &str,
        name: &str,
        file_name: &str,
        file_size: i64,
        format: &str,
        crs: Option<i64>,
        height: i64,
        keep_raw: bool,
        source_points: f64,
        rgb: bool,
        user: i64,
    ) -> rusqlite::Result<String> {
        let id = uuid();
        self.db.execute(
            "INSERT INTO point_cloud (id, project_id, name, file_name, file_size, format, crs, height_epsg, status, keep_raw, source_points, rgb, created_by, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'uploading', ?, ?, ?, ?, ?)",
            params![id, project, name, file_name, file_size, format, crs, height, keep_raw as i64, source_points, rgb as i64, user, self.iso()],
        )?;
        Ok(id)
    }

    pub fn received(&self, id: &str, from: i64, to: i64) -> rusqlite::Result<bool> {
        Ok(self
            .db
            .execute("UPDATE point_cloud SET received = ? WHERE id = ? AND received = ?", params![to, id, from])?
            == 1)
    }

    pub fn enqueue(&self, id: &str, user: i64, sha256: Option<&str>) -> rusqlite::Result<()> {
        self.db.execute_batch("BEGIN")?;
        let r = (|| {
            self.db.execute(
                "UPDATE point_cloud SET status = 'queued', sha256 = COALESCE(?, sha256), error = NULL, progress = NULL WHERE id = ?",
                params![sha256, id],
            )?;
            self.db.execute(
                "INSERT INTO cloud_job (cloud_id, kind, status, created_by, created_at) VALUES (?, 'prepare', 'queued', ?, ?)",
                params![id, user, self.iso()],
            )?;
            Ok(())
        })();
        self.db.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
        r
    }

    pub fn set_status(&self, id: &str, status: &str, progress: Option<f64>, error: Option<&str>) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE point_cloud SET status = ?, progress = ?, error = ? WHERE id = ?",
            params![status, progress, error, id],
        )?;
        Ok(())
    }

    pub fn set_progress(&self, id: &str, share: f64) -> rusqlite::Result<()> {
        self.db
            .execute("UPDATE point_cloud SET progress = ? WHERE id = ? AND status = 'processing'", params![share, id])?;
        Ok(())
    }

    pub fn set_ready(&self, id: &str, points: i64, bytes: i64, rgb: bool, bounds: &Value, levels: &Value) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE point_cloud SET status = 'ready', progress = 1, error = NULL, points = ?, bytes = ?, rgb = ?, bounds = ?, levels = ?, ready_at = ? WHERE id = ?",
            params![points, bytes, rgb as i64, rail_across_js::stringify(bounds), rail_across_js::stringify(levels), self.iso(), id],
        )?;
        Ok(())
    }

    pub fn remove(&self, id: &str) -> rusqlite::Result<()> {
        self.db.execute("DELETE FROM point_cloud WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn used(&self, project: &str) -> rusqlite::Result<i64> {
        self.db.query_row(
            "SELECT COALESCE(SUM(bytes + CASE WHEN status = 'ready' AND keep_raw = 0 THEN 0 ELSE file_size END), 0) FROM point_cloud WHERE project_id = ?",
            [project],
            |r| r.get(0),
        )
    }

    pub fn pending_points(&self, project: &str) -> rusqlite::Result<f64> {
        self.db.query_row(
            "SELECT COALESCE(SUM(source_points), 0) FROM point_cloud WHERE project_id = ? AND status <> 'ready' AND status <> 'failed'",
            [project],
            |r| r.get::<_, f64>(0),
        )
    }

    pub fn by_project(&self) -> rusqlite::Result<Value> {
        let rows: Vec<Value> = self
            .db
            .prepare(
                "SELECT c.project_id, p.title, COUNT(*) AS clouds, SUM(c.bytes) AS bytes,
                   SUM(CASE WHEN c.status = 'ready' AND c.keep_raw = 0 THEN 0 ELSE c.file_size END) AS raw
                 FROM point_cloud c JOIN project p ON p.id = c.project_id GROUP BY c.project_id ORDER BY bytes DESC",
            )?
            .query_map([], |r| {
                Ok(json!({ "project_id": r.get::<_, String>(0)?, "title": r.get::<_, String>(1)?, "clouds": r.get::<_, i64>(2)?, "bytes": r.get::<_, i64>(3)?, "raw": r.get::<_, i64>(4)? }))
            })?
            .collect::<Result<_, _>>()?;
        Ok(Value::Array(rows))
    }

    /// The oldest queued job, now running — or none: (job id, cloud id).
    pub fn claim_job(&self) -> rusqlite::Result<Option<(i64, String)>> {
        self.db.execute_batch("BEGIN IMMEDIATE")?;
        let r = (|| {
            let job: Option<(i64, String)> = self
                .db
                .query_row("SELECT id, cloud_id FROM cloud_job WHERE status = 'queued' ORDER BY id LIMIT 1", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()?;
            let Some(job) = job else { return Ok(None) };
            if self.db.execute(
                "UPDATE cloud_job SET status = 'running', started_at = ? WHERE id = ? AND status = 'queued'",
                params![self.iso(), job.0],
            )? != 1
            {
                return Ok(None);
            }
            Ok(Some(job))
        })();
        self.db.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
        r
    }

    pub fn job(&self, id: i64) -> rusqlite::Result<Option<String>> {
        self.db.query_row("SELECT cloud_id FROM cloud_job WHERE id = ?", [id], |r| r.get(0)).optional()
    }

    pub fn finish_job(&self, id: i64, status: &str, error: Option<&str>) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE cloud_job SET status = ?, finished_at = ?, error = ? WHERE id = ?",
            params![status, self.iso(), error, id],
        )?;
        Ok(())
    }

    pub fn requeue_running(&self) -> rusqlite::Result<()> {
        self.db
            .execute("UPDATE cloud_job SET status = 'queued', started_at = NULL WHERE status = 'running'", [])?;
        self.db
            .execute("UPDATE point_cloud SET status = 'queued', progress = NULL WHERE status = 'processing'", [])?;
        Ok(())
    }

    pub fn transforms(&self, cloud: &str) -> rusqlite::Result<Vec<Transform>> {
        self.db
            .prepare(
                "SELECT t.*, u.name AS created_by_name FROM point_cloud_transform t JOIN user u ON u.id = t.created_by WHERE t.cloud_id = ? ORDER BY t.id DESC",
            )?
            .query_map([cloud], Transform::from_row)?
            .collect()
    }

    pub fn transform(&self, id: i64) -> rusqlite::Result<Option<Transform>> {
        self.db
            .query_row("SELECT * FROM point_cloud_transform WHERE id = ?", [id], Transform::from_row)
            .optional()
    }

    pub fn active_transform(&self, cloud: &str) -> rusqlite::Result<Option<Transform>> {
        self.db
            .query_row(
                "SELECT * FROM point_cloud_transform WHERE cloud_id = ? AND active = 1",
                [cloud],
                Transform::from_row,
            )
            .optional()
    }

    pub fn active_transforms(&self, project: &str) -> rusqlite::Result<HashMap<String, Transform>> {
        let list: Vec<Transform> = self
            .db
            .prepare("SELECT t.* FROM point_cloud_transform t JOIN point_cloud c ON c.id = t.cloud_id WHERE c.project_id = ? AND t.active = 1")?
            .query_map([project], Transform::from_row)?
            .collect::<Result<_, _>>()?;
        Ok(list.into_iter().map(|t| (t.cloud_id.clone(), t)).collect())
    }

    pub fn add_transform(
        &self,
        cloud: &str,
        matrix: &Value,
        params_: &Value,
        pairs: &Value,
        residuals: &Value,
        reference: Option<&str>,
        user: i64,
    ) -> rusqlite::Result<i64> {
        self.db.execute_batch("BEGIN")?;
        let r = (|| {
            self.db.execute("UPDATE point_cloud_transform SET active = 0 WHERE cloud_id = ?", [cloud])?;
            self.db.execute(
                "INSERT INTO point_cloud_transform (cloud_id, matrix, params, pairs, residuals, reference_cloud_id, active, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)",
                params![
                    cloud,
                    rail_across_js::stringify(matrix),
                    rail_across_js::stringify(params_),
                    rail_across_js::stringify(pairs),
                    rail_across_js::stringify(residuals),
                    reference,
                    user,
                    self.iso()
                ],
            )?;
            Ok(self.db.last_insert_rowid())
        })();
        self.db.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
        r
    }

    pub fn set_active_transform(&self, cloud: &str, id: Option<i64>) -> rusqlite::Result<bool> {
        self.db.execute_batch("BEGIN")?;
        let r = (|| {
            self.db.execute("UPDATE point_cloud_transform SET active = 0 WHERE cloud_id = ?", [cloud])?;
            Ok(match id {
                None => true,
                Some(id) => {
                    self.db
                        .execute("UPDATE point_cloud_transform SET active = 1 WHERE id = ? AND cloud_id = ?", params![id, cloud])?
                        == 1
                }
            })
        })();
        self.db.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
        r
    }
}

/// How long a finished run is kept [ms] (decision 238).
const RUN_KEEP: i64 = 7 * 24 * 3600 * 1000;

/// A run row as JSON with the database's names.
fn run_row(r: &Row) -> rusqlite::Result<Value> {
    let get_s = |k: &str| r.get::<_, Option<String>>(k).ok().flatten();
    Ok(json!({
        "id": r.get::<_, i64>("id")?, "project_id": r.get::<_, String>("project_id")?, "kind": r.get::<_, String>("kind")?,
        "status": r.get::<_, String>("status")?, "subject": get_s("subject"), "params": get_s("params"), "clouds": get_s("clouds"),
        "progress": r.get::<_, Option<f64>>("progress")?.map(n), "result": get_s("result"), "error": get_s("error"),
        "created_by": r.get::<_, i64>("created_by")?, "created_at": r.get::<_, String>("created_at")?,
        "started_at": get_s("started_at"), "finished_at": get_s("finished_at"),
    }))
}

/// A run as the API shows it (`publicRun`).
pub fn public_run(row: &Value, with_result: bool) -> Value {
    let parse = |k: &str, d: &str| serde_json::from_str::<Value>(row[k].as_str().filter(|s| !s.is_empty()).unwrap_or(d)).unwrap_or(Value::Null);
    let mut v = json!({
        "id": row["id"], "projectId": row["project_id"], "kind": row["kind"], "status": row["status"],
        "subject": parse("subject", "{}"), "clouds": parse("clouds", "[]"), "progress": row["progress"], "error": row["error"],
        "createdBy": row["created_by"], "createdAt": row["created_at"], "startedAt": row["started_at"], "finishedAt": row["finished_at"],
    });
    if with_result {
        v["result"] = row["result"].as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or(Value::Null);
    }
    v
}

pub struct RunStore<'a> {
    pub db: &'a Connection,
    pub now: i64,
}

impl<'a> RunStore<'a> {
    pub fn new(db: &'a Connection, now: i64) -> Self {
        RunStore { db, now }
    }

    pub fn create(&self, project: &str, kind: &str, subject: &Value, params_: &Value, clouds: &Value, user: i64) -> rusqlite::Result<i64> {
        self.db.execute(
            "INSERT INTO cloud_run (project_id, kind, status, subject, params, clouds, created_by, created_at) VALUES (?, ?, 'queued', ?, ?, ?, ?, ?)",
            params![
                project,
                kind,
                rail_across_js::stringify(subject),
                rail_across_js::stringify(params_),
                rail_across_js::stringify(clouds),
                user,
                iso(self.now)
            ],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Value>> {
        self.db.query_row("SELECT * FROM cloud_run WHERE id = ?", [id], run_row).optional()
    }

    pub fn list(&self, project: &str) -> rusqlite::Result<Vec<Value>> {
        self.db
            .prepare("SELECT * FROM cloud_run WHERE project_id = ? ORDER BY id DESC LIMIT 20")?
            .query_map([project], run_row)?
            .collect()
    }

    pub fn active(&self, project: &str) -> rusqlite::Result<i64> {
        self.db.query_row(
            "SELECT COUNT(*) FROM cloud_run WHERE project_id = ? AND status IN ('queued', 'running')",
            [project],
            |r| r.get(0),
        )
    }

    pub fn claim(&self) -> rusqlite::Result<Option<Value>> {
        self.db.execute_batch("BEGIN IMMEDIATE")?;
        let r = (|| {
            let id: Option<i64> = self
                .db
                .query_row("SELECT id FROM cloud_run WHERE status = 'queued' ORDER BY id LIMIT 1", [], |r| r.get(0))
                .optional()?;
            let Some(id) = id else { return Ok(None) };
            if self.db.execute(
                "UPDATE cloud_run SET status = 'running', started_at = ?, progress = 0 WHERE id = ? AND status = 'queued'",
                params![iso(self.now), id],
            )? != 1
            {
                return Ok(None);
            }
            self.get(id)
        })();
        self.db.execute_batch(if r.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
        r
    }

    pub fn set_progress(&self, id: i64, share: f64) -> rusqlite::Result<()> {
        self.db
            .execute("UPDATE cloud_run SET progress = ? WHERE id = ? AND status = 'running'", params![share, id])?;
        Ok(())
    }

    pub fn done(&self, id: i64, result: &Value) -> rusqlite::Result<bool> {
        Ok(self.db.execute(
            "UPDATE cloud_run SET status = 'done', progress = 1, result = ?, finished_at = ? WHERE id = ? AND status = 'running'",
            params![rail_across_js::stringify(result), iso(self.now), id],
        )? == 1)
    }

    pub fn finish(&self, id: i64, status: &str, error: Option<&str>) -> rusqlite::Result<bool> {
        Ok(self.db.execute(
            "UPDATE cloud_run SET status = ?, error = ?, finished_at = ? WHERE id = ? AND status IN ('queued', 'running')",
            params![status, error, iso(self.now), id],
        )? == 1)
    }

    pub fn remove(&self, id: i64) -> rusqlite::Result<()> {
        self.db.execute("DELETE FROM cloud_run WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn requeue_running(&self) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE cloud_run SET status = 'queued', started_at = NULL, progress = NULL WHERE status = 'running'",
            [],
        )?;
        Ok(())
    }

    pub fn prune(&self) -> rusqlite::Result<usize> {
        self.db.execute(
            "DELETE FROM cloud_run WHERE status NOT IN ('queued', 'running') AND created_at < ?",
            [iso(self.now - RUN_KEEP)],
        )
    }
}
