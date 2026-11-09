// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The point cloud import as a WebAssembly module for the browser's import
//! worker (decision 0020, steps 7 and 10): LAS, LAZ and E57 read, converted
//! to the project's plane and tiled by `crates/pointcloud`, as the server's
//! jobs do. The file is read and the tiles are written through the page, by
//! four functions it gives the module (`ra`): `read` (a worker's
//! `FileReaderSync`), `write` (an OPFS `FileSystemSyncAccessHandle`),
//! `progress` and `now`. Everything is synchronous, as only a worker may be.
//!
//! Called as the core's module is: `alloc` a buffer, write `{"op", "input"}`
//! as JSON into it, `run`, and read the answer — `{"ok": …}` or
//! `{"error"}` — from the pointer `run` returns and `result_len`.
//! `packages/pointcloud/src/native.js` does this for the JavaScript.

use std::cell::Cell;

use rail_across_pointcloud::crs::PlaneMapper;
use rail_across_pointcloud::import::{Writer, import_levels, read_cloud_header, read_first_points};
use rail_across_pointcloud::source::Source;
use rail_across_pointcloud::{Error, Result};
use serde_json::{Map, Value, json};

#[cfg(target_arch = "wasm32")]
mod host {
    #[link(wasm_import_module = "ra")]
    unsafe extern "C" {
        /// Up to `len` bytes of the file from `offset` into `ptr`; how many there were.
        pub fn read(offset: f64, len: usize, ptr: *mut u8) -> usize;
        /// `len` bytes at `ptr` appended to the tiles of `level`; their offset there.
        pub fn write(level: u32, ptr: *const u8, len: usize) -> f64;
        /// Progress as JSON, `len` bytes at `ptr`.
        pub fn progress(ptr: *const u8, len: usize);
        /// Milliseconds, as `performance.now()`.
        pub fn now() -> f64;
    }
}

/// Outside WebAssembly there is no page: the crate builds (for the
/// workspace's checks) but reads nothing.
#[cfg(not(target_arch = "wasm32"))]
mod host {
    pub unsafe fn read(_offset: f64, _len: usize, _ptr: *mut u8) -> usize {
        0
    }
    pub unsafe fn write(_level: u32, _ptr: *const u8, _len: usize) -> f64 {
        0.0
    }
    pub unsafe fn progress(_ptr: *const u8, _len: usize) {}
    pub unsafe fn now() -> f64 {
        0.0
    }
}

/// The file the page has open, `size` bytes long.
struct PageFile {
    size: u64,
}

impl Source for PageFile {
    fn size(&self) -> u64 {
        self.size
    }
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        // SAFETY: the page writes at most `length` bytes into the buffer it is given.
        let n = unsafe { host::read(offset as f64, length, buf.as_mut_ptr()) };
        buf.truncate(n.min(length));
        Ok(buf)
    }
}

/// The tiles of one level, written by the page.
struct PageTiles {
    level: u32,
}

impl Writer for PageTiles {
    fn append(&mut self, bytes: &[u8]) -> Result<u64> {
        // SAFETY: the page reads `bytes.len()` bytes from the pointer, during the call.
        let at = unsafe { host::write(self.level, bytes.as_ptr(), bytes.len()) };
        if at.is_finite() && at >= 0.0 {
            Ok(at as u64)
        } else {
            Err(Error::msg("the tiles could not be written"))
        }
    }
}

fn post_progress(value: &Value) {
    let text = value.to_string();
    // SAFETY: the page reads `text.len()` bytes from the pointer, during the call.
    unsafe { host::progress(text.as_ptr(), text.len()) };
}

/// Progress is posted at most this often [s], and at the end.
const PROGRESS_EVERY: f64 = 0.25;

fn page_file(input: &Value) -> PageFile {
    PageFile {
        size: input["size"].as_f64().unwrap_or(0.0).max(0.0) as u64,
    }
}

/// Operation `op` on `input`.
fn run_op(op: &str, input: &Value) -> Result<Value> {
    let n = rail_across_js::number;
    match op {
        // The header of the file: `{ size }`.
        "header" => Ok(read_cloud_header(&page_file(input))?.to_json()),
        // The first `n` points, as `[{ x, y, z, intensity }]`: `{ size, n }`.
        "firstPoints" => {
            let source = page_file(input);
            let header = read_cloud_header(&source)?;
            let count = input["n"].as_f64().filter(|x| *x > 0.0).unwrap_or(0.0) as usize;
            let points = read_first_points(&source, &header, count)?;
            Ok(Value::Array(
                points
                    .iter()
                    .map(|p| json!({ "x": n(p[0]), "y": n(p[1]), "z": n(p[2]), "intensity": n(p[3]) }))
                    .collect(),
            ))
        }
        // A grid the page has read (proj4js's `subgrids`), for the conversion: `{ key, subgrids }`.
        "loadGrid" => {
            rail_across_geom::coords::load_grid_json(input);
            Ok(json!(true))
        }
        // The levels of the file into the page's tile files, the points converted from
        // `sourceCrs` to `targetCrs`: `{ size, levels, sourceCrs, targetCrs, idlePoints? }`.
        // Answers each level's index body, by level.
        "importLevels" => {
            let source = page_file(input);
            let header = read_cloud_header(&source)?;
            let levels: Vec<usize> = input["levels"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_u64).map(|l| l as usize).filter(|l| *l <= 4).collect())
                .unwrap_or_default();
            if levels.is_empty() {
                return Err(Error::msg("no level to import"));
            }
            let (from, to) = (input["sourceCrs"].as_i64().unwrap_or(0), input["targetCrs"].as_i64().unwrap_or(0));
            let mapper = if from != to && from != 0 && to != 0 {
                Some(PlaneMapper::new(from, to)?)
            } else {
                None
            };
            let map = mapper.as_ref().map(|m| move |e: f64, nn: f64| m.map(e, nn));
            let mut tiles: Vec<PageTiles> = levels.iter().map(|&l| PageTiles { level: l as u32 }).collect();
            let writers: Vec<&mut dyn Writer> = tiles.iter_mut().map(|t| t as &mut dyn Writer).collect();
            let last = Cell::new(f64::NEG_INFINITY);
            let mut on_progress = |p: rail_across_pointcloud::import::ImportProgress| {
                let done = p.read.total_bytes > 0.0 && p.read.bytes >= p.read.total_bytes;
                if !done && p.elapsed - last.get() < PROGRESS_EVERY {
                    return;
                }
                last.set(p.elapsed);
                post_progress(&json!({
                    "bytes": n(p.read.bytes), "totalBytes": n(p.read.total_bytes),
                    "points": n(p.read.points), "totalPoints": n(p.read.total_points),
                    "kept": p.kept, "bytesWritten": p.bytes_written,
                    "elapsed": n(p.elapsed), "remaining": p.remaining.map(n).unwrap_or(Value::Null),
                }));
            };
            let bodies = import_levels(
                &source,
                &header,
                map.as_ref().map(|m| m as &dyn Fn(f64, f64) -> (f64, f64)),
                writers,
                &levels,
                input["idlePoints"].as_u64(),
                Some(&mut on_progress),
                None,
            )?;
            let mut out = Map::new();
            for (level, body) in bodies {
                out.insert(level.to_string(), body);
            }
            Ok(Value::Object(out))
        }
        _ => Err(Error::msg(format!("unknown operation {op}"))),
    }
}

thread_local! {
    static RESULT_LEN: Cell<usize> = const { Cell::new(0) };
}

fn host_clock() -> f64 {
    // SAFETY: the page's clock, a plain number.
    unsafe { host::now() / 1000.0 }
}

/// One request as JSON text, answered as JSON text: `{"ok": …}` or `{"error"}`.
pub fn dispatch(request: &str) -> String {
    rail_across_pointcloud::set_clock(host_clock);
    let answer = match serde_json::from_str::<Value>(request) {
        Ok(req) => {
            let op = req.get("op").and_then(Value::as_str).unwrap_or("");
            match run_op(op, req.get("input").unwrap_or(&Value::Null)) {
                Ok(value) => json!({ "ok": value }),
                Err(e) => json!({ "error": e.message() }),
            }
        }
        Err(err) => json!({ "error": format!("invalid request: {err}") }),
    };
    answer.to_string()
}

/// Allocates `len` bytes in the module's memory.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees `len` bytes at `ptr`, allocated by `alloc` or returned by `run`.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `run` and `result_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller hands back a buffer of this module of that capacity.
    unsafe { drop(Vec::from_raw_parts(ptr, 0, len.max(1))) };
}

/// Runs the request of `len` bytes at `ptr` (and frees them); returns the
/// answer's JSON, whose length `result_len` tells.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(ptr: *mut u8, len: usize) -> *mut u8 {
    // SAFETY: the caller wrote `len` bytes into a buffer from `alloc(len)`.
    let input = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let output = match std::str::from_utf8(&input) {
        Ok(request) => dispatch(request),
        Err(_) => r#"{"error":"the request is not UTF-8"}"#.to_string(),
    };
    let bytes = output.into_bytes();
    let len = bytes.len();
    RESULT_LEN.with(|l| l.set(len));
    let out = alloc(len);
    // SAFETY: `out` has room for `len` bytes, which do not overlap `bytes`.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, len) };
    out
}

/// The length of the last answer `run` returned.
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    RESULT_LEN.with(|l| l.get())
}
