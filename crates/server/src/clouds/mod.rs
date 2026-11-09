// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Point clouds on the server (phase 13): their rows and jobs (`cloudStore.js`,
//! `runStore.js`), where they lie on disk (`storage.js`, `ranges.js`), and the
//! jobs that prepare them and run over them (`jobs.js`, `prepare.js`,
//! `runs.js`) — as threads of the server or of `rail-across-server cloudjobs`.

pub mod jobs;
pub mod store;

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::http::ApiError;

/// Largest share of the disk a project's clouds may take [bytes].
pub const PROJECT_QUOTA: i64 = 25 * 1024_i64.pow(3);
/// Free space an upload must leave on the disk [bytes].
pub const DISK_RESERVE: i64 = 5 * 1024_i64.pow(3);
/// Below this much free space the admin view warns [bytes].
pub const DISK_WARN: i64 = 20 * 1024_i64.pow(3);

fn safe(id: &str) -> bool {
    id.len() == 36 && id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || c == b'-')
}

/// Where the clouds lie on disk: `<root>/<project>/<cloud>/…`.
#[derive(Clone, Debug)]
pub struct Storage {
    pub root: PathBuf,
}

impl Storage {
    pub fn dir(&self, project: &str, cloud: &str) -> Result<PathBuf, ApiError> {
        if !safe(project) || !safe(cloud) {
            return Err(ApiError::new(500, "internal"));
        }
        Ok(self.root.join(project).join(cloud))
    }
    pub fn ensure(&self, project: &str, cloud: &str) -> Result<PathBuf, ApiError> {
        let d = self.dir(project, cloud)?;
        fs::create_dir_all(&d)?;
        Ok(d)
    }
    pub fn raw(&self, project: &str, cloud: &str) -> Result<PathBuf, ApiError> {
        Ok(self.dir(project, cloud)?.join("raw.part"))
    }
    pub fn tiles(&self, project: &str, cloud: &str, level: usize) -> Result<PathBuf, ApiError> {
        Ok(self.dir(project, cloud)?.join(format!("tiles-L{level}.bin")))
    }
    pub fn index(&self, project: &str, cloud: &str, level: usize) -> Result<PathBuf, ApiError> {
        Ok(self.dir(project, cloud)?.join(format!("index-L{level}.json")))
    }
    pub fn remove(&self, project: &str, cloud: &str) {
        if let Ok(d) = self.dir(project, cloud) {
            let _ = fs::remove_dir_all(d);
        }
    }
    /// Free and total bytes of the disk the clouds lie on.
    pub fn disk(&self) -> (i64, i64) {
        let _ = fs::create_dir_all(&self.root);
        disk_space(&self.root)
    }
}

#[cfg(unix)]
fn disk_space(path: &Path) -> (i64, i64) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = CString::new(path.as_os_str().as_bytes()) else { return (0, 0) };
    let mut s = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: statvfs fills the struct it is given; the path is a valid C string.
    if unsafe { libc::statvfs(c.as_ptr(), s.as_mut_ptr()) } != 0 {
        return (0, 0);
    }
    // SAFETY: statvfs succeeded, so the struct is initialised.
    let s = unsafe { s.assume_init() };
    #[allow(clippy::unnecessary_cast)]
    ((s.f_bavail as u64 * s.f_frsize as u64) as i64, (s.f_blocks as u64 * s.f_frsize as u64) as i64)
}

#[cfg(windows)]
fn disk_space(path: &Path) -> (i64, i64) {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, free_to_caller: *mut u64, total: *mut u64, total_free: *mut u64) -> i32;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let (mut free, mut total, mut all_free) = (0u64, 0u64, 0u64);
    // SAFETY: the path is a NUL-terminated UTF-16 string; the three outputs are valid u64s.
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut all_free) } == 0 {
        return (0, 0);
    }
    (free as i64, total as i64)
}

#[cfg(not(any(unix, windows)))]
fn disk_space(_: &Path) -> (i64, i64) {
    (i64::MAX / 4, i64::MAX / 4)
}

/// Bytes the tiles of a delivery will take on all five levels.
pub fn estimate_tile_bytes(points: f64, rgb: bool) -> i64 {
    rail_across_js::js_round(points * (5.9 + if rgb { 2.0 } else { 0.0 })) as i64
}

const MAX_RANGES: usize = 64;
const MAX_RANGES_BYTES: u64 = 64 * 1024 * 1024;

/// Up to 64 `[offset, length]` of a tile file, back to back (`readRanges`).
pub fn read_ranges(path: &Path, ranges: Option<&Value>) -> Result<Vec<u8>, ApiError> {
    let Some(list) = ranges.and_then(Value::as_array).filter(|l| !l.is_empty() && l.len() <= MAX_RANGES) else {
        return Err(ApiError::new(422, "ranges_invalid"));
    };
    let int = |v: Option<&Value>| v.and_then(Value::as_f64).filter(|f| f.fract() == 0.0 && f.is_finite());
    let mut parsed = Vec::new();
    let mut total = 0u64;
    for r in list {
        let (Some(o), Some(l)) = (int(r.get(0)), int(r.get(1))) else {
            return Err(ApiError::new(422, "ranges_invalid"));
        };
        if !r.is_array() || o < 0.0 || l <= 0.0 {
            return Err(ApiError::new(422, "ranges_invalid"));
        }
        parsed.push((o as u64, l as u64));
        total += l as u64;
    }
    if total > MAX_RANGES_BYTES {
        return Err(ApiError::new(413, "too_large"));
    }
    let size = fs::metadata(path)?.len();
    let mut f = fs::File::open(path)?;
    let mut out = Vec::with_capacity(total as usize);
    for (o, l) in parsed {
        if o + l > size {
            return Err(ApiError::new(416, "range_past_end"));
        }
        f.seek(SeekFrom::Start(o))?;
        let mut buf = vec![0u8; l as usize];
        f.read_exact(&mut buf)?;
        out.extend(buf);
    }
    Ok(out)
}
