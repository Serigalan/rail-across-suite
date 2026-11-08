// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Tiles by key: in memory, on disk, or fetched (`terrain/store.js`).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

use super::Source;
use super::tile::{Tile, read_tile};

fn env_num(name: &str, d: f64) -> f64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

const MISSING_TTL: i64 = 24 * 3600 * 1000;
const EXT: &str = ".dgm1";

/// Where the bytes of a URL come from: Ok(None) where there is no such file.
pub type Fetch = Arc<dyn Fn(&str) -> Result<Option<Vec<u8>>, String> + Send + Sync>;

/// The bytes at `url` over HTTP.
pub fn download(url: &str) -> Result<Option<Vec<u8>>, String> {
    let timeout = Duration::from_secs_f64(env_num("RA_TERRAIN_FETCH_TIMEOUT", 90.0));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into();
    let res = agent
        .get(url)
        .header("User-Agent", "rail-across terrain")
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let status = res.status().as_u16();
    if status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        return Err(format!("{url}: {status}"));
    }
    res.into_body().with_config().limit(1 << 30).read_to_vec().map(Some).map_err(|e| e.to_string())
}

pub type Got = (Option<Arc<Tile>>, bool);

pub struct TileStore {
    pub cache_dir: PathBuf,
    pub fetch: Fetch,
    memory: Mutex<Vec<(String, Arc<Tile>)>>,
    missing: Mutex<HashMap<String, i64>>,
    loading: Mutex<HashMap<String, Arc<(Mutex<Option<Got>>, Condvar)>>>,
    memory_tiles: usize,
    disk_tiles: usize,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl TileStore {
    pub fn new(cache_dir: PathBuf, fetch: Fetch) -> TileStore {
        TileStore {
            cache_dir,
            fetch,
            memory: Mutex::new(Vec::new()),
            missing: Mutex::new(HashMap::new()),
            loading: Mutex::new(HashMap::new()),
            memory_tiles: env_num("RA_TERRAIN_MEMORY_TILES", 24.0) as usize,
            disk_tiles: env_num("RA_TERRAIN_DISK_TILES", 400.0) as usize,
        }
    }

    pub fn from_env() -> TileStore {
        let dir = std::env::var("RA_TERRAIN_CACHE").map(PathBuf::from).unwrap_or_else(|_| {
            let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".cache").join("rail-across-terrain")
        });
        TileStore::new(dir, Arc::new(download))
    }

    fn disk_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}{EXT}", key.replacen('/', "_", 1)))
    }

    fn from_disk(&self, key: &str) -> Option<Tile> {
        let path = self.disk_path(key);
        let data = fs::read(&path).ok()?;
        let end = data.iter().position(|b| *b == 10)?;
        let head: serde_json::Value = serde_json::from_slice(&data[..end]).ok()?;
        let body = &data[end + 1..];
        let grid: Vec<f32> = body.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let (rows, cols) = (head["rows"].as_u64()? as usize, head["cols"].as_u64()? as usize);
        if grid.len() != rows * cols {
            return None;
        }
        let _ = fs::File::options().append(true).open(&path).and_then(|f| f.set_modified(SystemTime::now()));
        Some(Tile {
            grid,
            rows,
            cols,
            x0: head["x0"].as_f64()?,
            y0: head["y0"].as_f64()?,
            label: head["label"].as_str().unwrap_or("").into(),
        })
    }

    fn to_disk(&self, key: &str, tile: &Tile) {
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(&self.cache_dir)?;
            let head =
                serde_json::json!({ "rows": tile.rows, "cols": tile.cols, "x0": crate::util::n(tile.x0), "y0": crate::util::n(tile.y0), "label": tile.label });
            let mut bytes = (rail_across_js::stringify(&head) + "\n").into_bytes();
            for v in &tile.grid {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            let tmp = PathBuf::from(format!("{}.tmp", self.disk_path(key).display()));
            fs::write(&tmp, bytes)?;
            fs::rename(&tmp, self.disk_path(key))?;
            let mut files: Vec<(PathBuf, SystemTime)> = fs::read_dir(&self.cache_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().ends_with(EXT))
                .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
                .collect();
            files.sort_by_key(|f| f.1);
            let extra = files.len().saturating_sub(self.disk_tiles);
            for (p, _) in files.into_iter().take(extra) {
                let _ = fs::remove_file(p);
            }
            Ok(())
        };
        let _ = write();
    }

    fn load(&self, source: &Source, ek: i64, nk: i64) -> Result<Option<Tile>, String> {
        let key = format!("{}/{ek}_{nk}", source.id);
        if let Some(t) = self.from_disk(&key) {
            return Ok(Some(t));
        }
        for (label, url) in &source.vintages {
            let Some(bytes) = (self.fetch)(&url(ek, nk))? else { continue };
            let label = match label {
                Some(l) => format!("{} {l}", source.id),
                None => source.id.to_string(),
            };
            let tile = read_tile(&bytes, source.fmt, &label)?;
            self.to_disk(&key, &tile);
            return Ok(Some(tile));
        }
        Ok(None)
    }

    /// (tile, failed): the tile at kilometre (ek, nk), none where the source has none,
    /// and whether a none is a failed download. Two callers wanting one tile share its download.
    pub fn get(&self, source: &Source, ek: i64, nk: i64) -> Got {
        let key = format!("{}/{ek}_{nk}", source.id);
        {
            let mut mem = self.memory.lock().unwrap();
            if let Some(i) = mem.iter().position(|(k, _)| *k == key) {
                let hit = mem.remove(i);
                let tile = hit.1.clone();
                mem.push(hit);
                return (Some(tile), false);
            }
        }
        if self.missing.lock().unwrap().get(&key).is_some_and(|at| now_ms() - at < MISSING_TTL) {
            return (None, false);
        }
        let (slot, leader) = {
            let mut loading = self.loading.lock().unwrap();
            match loading.get(&key) {
                Some(s) => (s.clone(), false),
                None => {
                    let s = Arc::new((Mutex::new(None), Condvar::new()));
                    loading.insert(key.clone(), s.clone());
                    (s, true)
                }
            }
        };
        if !leader {
            let mut got = slot.0.lock().unwrap();
            while got.is_none() {
                got = slot.1.wait(got).unwrap();
            }
            return got.clone().unwrap();
        }
        let got: Got = match self.load(source, ek, nk) {
            Ok(Some(tile)) => {
                let tile = Arc::new(tile);
                let mut mem = self.memory.lock().unwrap();
                mem.push((key.clone(), tile.clone()));
                while mem.len() > self.memory_tiles {
                    mem.remove(0);
                }
                (Some(tile), false)
            }
            Ok(None) => {
                self.missing.lock().unwrap().insert(key.clone(), now_ms());
                (None, false)
            }
            Err(_) => (None, true),
        };
        *slot.0.lock().unwrap() = Some(got.clone());
        slot.1.notify_all();
        self.loading.lock().unwrap().remove(&key);
        got
    }
}
