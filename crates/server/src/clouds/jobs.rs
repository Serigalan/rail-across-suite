// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The jobs over the clouds (`jobs.js`, `prepare.js`, `runs.js`): preparing an
//! uploaded cloud into its five levels, and the long runs (clearance scan,
//! rail trace) — as threads, two of each at once, each opening the database
//! for itself; a job that panics fails alone.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rail_across_pointcloud::crs::Transform;
use rail_across_pointcloud::import::{Header, Writer, import_levels, read_cloud_header};
use rail_across_pointcloud::section::Cloud as SectionCloud;
use rail_across_pointcloud::source::{FileSource, TileFile};
use rusqlite::Connection;
use serde_json::{Value, json};

use super::Storage;
use super::store::{CloudStore, RunStore};

const PROGRESS_EVERY: Duration = Duration::from_secs(2);
const RUN_PROGRESS_EVERY: Duration = Duration::from_secs(1);
const RUN_LEVEL: usize = 1;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The database a job opens for itself.
pub fn open(db: &str) -> Result<Connection, String> {
    crate::db::open_database(db, &crate::util::iso(now_ms())).map_err(|e| e.to_string())
}

struct FileWriter {
    out: BufWriter<fs::File>,
    at: u64,
}

impl Writer for FileWriter {
    fn append(&mut self, bytes: &[u8]) -> rail_across_pointcloud::Result<u64> {
        let o = self.at;
        self.out.write_all(bytes)?;
        self.at += bytes.len() as u64;
        Ok(o)
    }
}

fn part(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", path.display()))
}

/// The preparation of one uploaded cloud (`prepareCloud`).
pub fn prepare_cloud(db: &Connection, storage: &Storage, cloud_id: &str, log: &dyn Fn(&str)) -> Result<(), String> {
    let cs = || CloudStore::new(db, now_ms());
    let c = cs().get(cloud_id).map_err(|e| e.to_string())?.ok_or("cloud gone")?;
    let raw = storage.raw(&c.project_id, &c.id).map_err(|e| e.error)?;
    if !raw.exists() {
        return Err("the uploaded file is gone".into());
    }
    cs().set_status(&c.id, "processing", Some(0.0), None).map_err(|e| e.to_string())?;
    let source = FileSource::open(&raw).map_err(|e| e.message())?;
    let tiles: Vec<PathBuf> = (0..5).map(|l| storage.tiles(&c.project_id, &c.id, l).unwrap()).collect();
    let cleanup = || {
        for t in &tiles {
            let _ = fs::remove_file(part(t));
        }
        if cs().get(&c.id).ok().flatten().is_none() {
            storage.remove(&c.project_id, &c.id);
        }
    };
    let run = || -> Result<(), String> {
        let header = read_cloud_header(&source).map_err(|e| e.message())?;
        let mut writers: Vec<FileWriter> = tiles
            .iter()
            .map(|t| {
                Ok(FileWriter {
                    out: BufWriter::new(fs::File::create(part(t))?),
                    at: 0,
                })
            })
            .collect::<std::io::Result<_>>()
            .map_err(|e| e.to_string())?;
        let started = Instant::now();
        let mut last = Instant::now() - PROGRESS_EVERY;
        let gone = std::cell::Cell::new(false);
        let mut on_progress = |p: rail_across_pointcloud::import::ImportProgress| {
            if last.elapsed() < PROGRESS_EVERY {
                return;
            }
            last = Instant::now();
            if cs().get(&c.id).ok().flatten().is_none() {
                gone.set(true);
                return;
            }
            let share = if p.read.total_bytes > 0.0 { p.read.bytes / p.read.total_bytes } else { 0.0 };
            let _ = cs().set_progress(&c.id, share);
        };
        let aborted = || gone.get();
        let levels = [0usize, 1, 2, 3, 4];
        let bodies = {
            let refs: Vec<&mut dyn Writer> = writers.iter_mut().map(|w| w as &mut dyn Writer).collect();
            import_levels(&source, &header, None, refs, &levels, None, Some(&mut on_progress), Some(&aborted)).map_err(|e| e.message())?
        };
        if gone.get() || cs().get(&c.id).ok().flatten().is_none() {
            return Err("aborted".into());
        }
        for w in writers.iter_mut() {
            w.out.flush().map_err(|e| e.to_string())?;
            w.out.get_ref().sync_all().map_err(|e| e.to_string())?;
        }
        drop(writers);
        let file = match &header {
            Header::E57(h) => json!({ "name": c.file_name, "size": c.file_size, "format": "e57", "e57Version": h.version, "scans": h.scans.len() }),
            Header::Las(h) => json!({ "name": c.file_name, "size": c.file_size, "format": c.format, "lasVersion": h.version, "pointFormat": h.point_format }),
        };
        let mut summary = Vec::new();
        let mut bytes = 0i64;
        let mut level1 = Value::Null;
        for (level, body) in bodies {
            let t = &tiles[level];
            fs::rename(part(t), t).map_err(|e| e.to_string())?;
            let mut index = body.clone();
            for (k, v) in [
                ("id", json!(c.id)),
                ("name", json!(c.name)),
                ("crs", json!(c.crs)),
                ("sourceCrs", json!(c.crs)),
                ("heightEpsg", json!(c.height_epsg)),
                ("file", file.clone()),
                ("createdAt", json!(c.created_at)),
            ] {
                index[k] = v;
            }
            fs::write(
                storage.index(&c.project_id, &c.id, level).map_err(|e| e.error)?,
                rail_across_js::stringify(&index),
            )
            .map_err(|e| e.to_string())?;
            summary.push(json!({ "level": level, "points": index["points"], "bytes": index["bytes"], "tiles": index["tiles"].as_array().map_or(0, Vec::len), "segments": index["segments"] }));
            bytes += index["bytes"].as_i64().unwrap_or(0);
            if level == 1 {
                level1 = index;
            }
        }
        cs().set_ready(
            &c.id,
            level1["points"].as_i64().unwrap_or(0),
            bytes,
            level1["rgb"].as_bool().unwrap_or(false),
            &level1["bounds"],
            &json!(summary),
        )
        .map_err(|e| e.to_string())?;
        log(&format!(
            "cloud {}: {} points in {:.0} s, {bytes} bytes of tiles",
            c.id,
            header.point_count(),
            started.elapsed().as_secs_f64()
        ));
        if !c.keep_raw {
            let _ = fs::remove_file(&raw);
        }
        Ok(())
    };
    let r = run();
    if r.is_err() {
        cleanup();
    }
    r
}

/// The clouds a run reads (`runClouds`).
fn run_clouds(db: &Connection, storage: &Storage, run: &Value) -> Result<Vec<SectionCloud>, String> {
    let cs = CloudStore::new(db, now_ms());
    let ids: Vec<String> = serde_json::from_str(run["clouds"].as_str().unwrap_or("[]")).unwrap_or_default();
    let mut out = Vec::new();
    for id in ids {
        let Some(c) = cs.get(&id).map_err(|e| e.to_string())? else { continue };
        if Some(c.project_id.as_str()) != run["project_id"].as_str() || c.status != "ready" {
            continue;
        }
        let index = storage.index(&c.project_id, &c.id, RUN_LEVEL).map_err(|e| e.error)?;
        if !index.exists() {
            continue;
        }
        let t = cs.active_transform(&c.id).map_err(|e| e.to_string())?;
        let transform = t.and_then(|t| {
            let m: Vec<f64> = serde_json::from_str(&t.matrix).ok()?;
            let p: Value = serde_json::from_str(&t.params).ok()?;
            Some(Transform {
                matrix: std::array::from_fn(|i| m.get(i).copied().unwrap_or(f64::NAN)),
                crs: p["crs"].as_i64()?,
            })
        });
        if c.crs.is_none() && transform.is_none() {
            continue;
        }
        let body: Value = serde_json::from_slice(&fs::read(&index).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let source = Arc::new(TileFile(
            FileSource::open(storage.tiles(&c.project_id, &c.id, RUN_LEVEL).map_err(|e| e.error)?).map_err(|e| e.message())?,
        ));
        out.push(SectionCloud::from_index(&c.id, Some(RUN_LEVEL as i64), &body, c.crs, transform, source));
    }
    if out.is_empty() {
        return Err("none of the clouds is there any more".into());
    }
    Ok(out)
}

/// One long run (`executeRun`): Ok(true) done, Ok(false) cancelled.
pub fn execute_run(db: &Connection, storage: &Storage, run: &Value, log: &dyn Fn(&str)) -> Result<bool, String> {
    let id = run["id"].as_i64().unwrap_or(0);
    let params: Value = serde_json::from_str(run["params"].as_str().unwrap_or("{}")).map_err(|e| e.to_string())?;
    let clouds = run_clouds(db, storage, run)?;
    let cancelled = AtomicBool::new(false);
    let mut last = Instant::now() - RUN_PROGRESS_EVERY;
    let mut on_progress = |share: f64| {
        if last.elapsed() < RUN_PROGRESS_EVERY {
            return;
        }
        last = Instant::now();
        let rs = RunStore::new(db, now_ms());
        if rs.get(id).ok().flatten().is_none_or(|r| r["status"] != "running") {
            cancelled.store(true, Ordering::Relaxed);
            return;
        }
        let _ = rs.set_progress(id, share);
    };
    let aborted = || cancelled.load(Ordering::Relaxed);
    let started = Instant::now();
    let list = |k: &str| params.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let result = match run["kind"].as_str() {
        Some("clearance") => {
            let track = &params["track"];
            let tracks = match params.get("tracks").and_then(Value::as_array) {
                Some(t) => t.clone(),
                None => vec![track.clone()],
            };
            let areas: Vec<_> = list("areas").iter().map(rail_across_pointcloud::clearance::ring_of).collect();
            rail_across_pointcloud::clearance::scan_clearance(
                &clouds,
                track,
                &rail_across_pointcloud::clearance::ring_of(&params["ring"]),
                &areas,
                params.get("from").and_then(Value::as_f64).unwrap_or(0.0),
                params.get("to").and_then(Value::as_f64),
                &tracks,
                &list("switches"),
                None,
                &mut on_progress,
                &aborted,
            )
        }
        Some("trace") => {
            let g = &params["guide"];
            let guide = match g["kind"].as_str() {
                Some("track") => rail_across_pointcloud::rail_trace::Guide::track(&g["track"]),
                Some("line") => {
                    let v: Vec<[f64; 2]> = g["vertices"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|p| [p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)])
                        .collect();
                    rail_across_pointcloud::rail_trace::Guide::line(&v, g["epsg"].clone())
                }
                _ => return Err("unknown guide".into()),
            };
            let rail = params["rail"].as_str().map(str::to_string);
            let mut detect = |origin: (f64, f64), bearing: f64, epsg: &Value, around: f64, window: f64| {
                rail_across_pointcloud::rail_trace::detect_in_clouds(
                    &clouds,
                    origin,
                    bearing,
                    rail_across_js::js_number(Some(epsg)) as i64,
                    around,
                    window,
                    rail.as_deref(),
                )
            };
            rail_across_pointcloud::rail_trace::trace_track(&guide, None, &mut detect, &mut on_progress, &aborted)
        }
        other => return Err(format!("unknown run {}", other.unwrap_or("undefined"))),
    };
    let result = match result {
        Ok(r) => r,
        Err(rail_across_pointcloud::Error::Aborted) => return Ok(false),
        Err(e) => return Err(e.message()),
    };
    let rs = RunStore::new(db, now_ms());
    if !rs.done(id, &result).map_err(|e| e.to_string())? {
        return Ok(false);
    }
    log(&format!(
        "run {id} ({}) done in {:.0} s",
        run["kind"].as_str().unwrap_or(""),
        started.elapsed().as_secs_f64()
    ));
    Ok(true)
}

/// The two queues — preparations and long runs, (`jobs`, `runs`) threads of each — over `db`, until `stop`.
pub fn run_queues(db_path: String, storage: Storage, parallel: (usize, usize), stop: Arc<AtomicBool>, log: Arc<dyn Fn(&str) + Send + Sync>) {
    if let Ok(db) = open(&db_path) {
        let _ = CloudStore::new(&db, now_ms()).requeue_running();
        let rs = RunStore::new(&db, now_ms());
        let _ = rs.requeue_running();
        let _ = rs.prune();
    }
    let workers: Vec<std::thread::JoinHandle<()>> = (0..parallel.0 + parallel.1)
        .map(|k| {
            let (db_path, storage, stop, log) = (db_path.clone(), storage.clone(), stop.clone(), log.clone());
            std::thread::spawn(move || {
                let runs = k >= parallel.0;
                let Ok(db) = open(&db_path) else { return };
                while !stop.load(Ordering::Relaxed) {
                    let worked = if runs {
                        take_run(&db, &storage, &*log)
                    } else {
                        take_job(&db, &storage, &*log)
                    };
                    if !worked {
                        for _ in 0..20 {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "the job crashed".into())
}

/// One preparation from the queue, if there is one.
pub fn take_job(db: &Connection, storage: &Storage, log: &dyn Fn(&str)) -> bool {
    let cs = CloudStore::new(db, now_ms());
    let Ok(Some((job, cloud))) = cs.claim_job() else { return false };
    log(&format!("job {job} (prepare) on cloud {cloud} started"));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| prepare_cloud(db, storage, &cloud, log))).unwrap_or_else(|p| Err(panic_text(p)));
    let cs = CloudStore::new(db, now_ms());
    match r {
        Ok(()) => {
            let _ = cs.finish_job(job, "done", None);
            log(&format!("job {job} (prepare) on cloud {cloud} done"));
        }
        Err(e) => {
            let reason: String = e.chars().take(500).collect();
            let _ = cs.finish_job(job, "failed", Some(&reason));
            if cs.get(&cloud).ok().flatten().is_some() {
                let _ = cs.set_status(&cloud, "failed", None, Some(&reason));
            }
            log(&format!("job {job} (prepare) on cloud {cloud} failed: {reason}"));
        }
    }
    true
}

/// One long run from the queue, if there is one.
pub fn take_run(db: &Connection, storage: &Storage, log: &dyn Fn(&str)) -> bool {
    let rs = RunStore::new(db, now_ms());
    let Ok(Some(run)) = rs.claim() else { return false };
    let id = run["id"].as_i64().unwrap_or(0);
    log(&format!(
        "run {id} ({}) in project {} started",
        run["kind"].as_str().unwrap_or(""),
        run["project_id"].as_str().unwrap_or("")
    ));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute_run(db, storage, &run, log))).unwrap_or_else(|p| Err(panic_text(p)));
    if let Err(e) = r {
        let reason: String = e.chars().take(500).collect();
        let _ = RunStore::new(db, now_ms()).finish(id, "failed", Some(&reason));
        log(&format!("run {id} failed: {reason}"));
    }
    true
}
