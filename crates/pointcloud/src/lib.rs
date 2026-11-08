// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Point clouds in Rust (decision 0020, step 7; `packages/pointcloud`): LAS,
//! LAZ and E57 read in pieces, tiled at several voxel levels, cut into thin
//! sections, searched for rail heads to trace a track, registered onto each
//! other and checked against the clearance contour — what the project server
//! runs in its jobs. The browser keeps the JavaScript (and laz-perf); the
//! tile files the two write are the same format and read by either.

#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_contains,
    clippy::byte_char_slices
)]

pub mod base64;
pub mod clearance;
pub mod crs;
pub mod e57;
pub mod e57_xml;
pub mod import;
pub mod las;
pub mod ops;
pub mod rail_detect;
pub mod rail_trace;
pub mod registration;
pub mod section;
pub mod slice;
pub mod source;
pub mod tiles;

/// Why reading or computing stopped: aborted by the caller, or an error the
/// JavaScript would throw, by its message.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Aborted,
    Msg(String),
}

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Error::Msg(m.into())
    }
    pub fn message(&self) -> String {
        match self {
            Error::Aborted => "aborted".into(),
            Error::Msg(m) => m.clone(),
        }
    }
}

impl From<String> for Error {
    fn from(m: String) -> Self {
        Error::Msg(m)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Msg(e.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A batch of points as either reader gives it: x, y, z in the file's units
/// (metres), the intensity as stored (E57: on 0…65535), colour where the file
/// has it; `records` the records read for it (E57 leaves invalid ones out).
#[derive(Clone, Debug, Default)]
pub struct Batch {
    pub count: usize,
    pub records: usize,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
    pub intensity: Vec<u16>,
    pub red: Option<Vec<u16>>,
    pub green: Option<Vec<u16>>,
    pub blue: Option<Vec<u16>>,
}

impl Batch {
    pub fn new(n: usize, rgb: bool) -> Self {
        Batch {
            count: n,
            records: n,
            x: vec![0.0; n],
            y: vec![0.0; n],
            z: vec![0.0; n],
            intensity: vec![0; n],
            red: rgb.then(|| vec![0; n]),
            green: rgb.then(|| vec![0; n]),
            blue: rgb.then(|| vec![0; n]),
        }
    }

    pub fn truncate(&mut self, n: usize) {
        self.count = n;
        self.x.truncate(n);
        self.y.truncate(n);
        self.z.truncate(n);
        self.intensity.truncate(n);
        for c in [&mut self.red, &mut self.green, &mut self.blue].into_iter().flatten() {
            c.truncate(n);
        }
    }
}

/// How far a reader has come: bytes of the file and points.
#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    pub bytes: f64,
    pub total_bytes: f64,
    pub points: f64,
    pub total_points: f64,
}

/// What a reader is handed besides the file: progress after each batch, and
/// whether the caller has asked it to stop.
#[derive(Default)]
pub struct ReadOptions<'a> {
    pub on_progress: Option<&'a mut dyn FnMut(Progress)>,
    pub aborted: Option<&'a dyn Fn() -> bool>,
}

impl ReadOptions<'_> {
    pub fn check(&self) -> Result<()> {
        match self.aborted {
            Some(f) if f() => Err(Error::Aborted),
            _ => Ok(()),
        }
    }
    pub fn progress(&mut self, p: Progress) {
        if let Some(f) = self.on_progress.as_mut() {
            f(p);
        }
    }
}
