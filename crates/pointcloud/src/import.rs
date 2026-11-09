// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The import of one LAS/LAZ/E57 file into tiles (`importPipeline.js`, `cloudReader.js`):
//! read batch by batch, thinned to each level's voxel, a tile written once the
//! scanner has moved on. The server makes all five levels in one pass.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::{Value, json};

use crate::e57::{E57Header, is_e57, read_e57_header, read_e57_points};
use crate::las::{LasHeader, read_las_header, read_las_points};
use crate::source::Source;
use crate::tiles::{Band, Grid, LEVELS, TileBuilder, color_shift, encode_grid_segment, encode_segment, intensity_shift, original_grid, split_into_bands};
use crate::{Batch, Error, Progress, ReadOptions, Result};

const INDEX_VERSION: u32 = 3;

/// The header of either format (`readCloudHeader`).
#[derive(Clone, Debug)]
pub enum Header {
    Las(LasHeader),
    E57(E57Header),
}

impl Header {
    pub fn to_json(&self) -> Value {
        match self {
            Header::Las(h) => h.to_json(),
            Header::E57(h) => h.to_json(),
        }
    }
    pub fn rgb(&self) -> bool {
        match self {
            Header::Las(h) => h.rgb,
            Header::E57(h) => h.rgb,
        }
    }
    pub fn point_count(&self) -> u64 {
        match self {
            Header::Las(h) => h.point_count,
            Header::E57(h) => h.point_count,
        }
    }
    pub fn grid(&self) -> Grid {
        match self {
            Header::Las(h) => original_grid(false, h.scale, h.offset),
            Header::E57(h) => original_grid(true, h.scale, [0.0; 3]),
        }
    }
}

/// The header of the file behind a source, either format.
pub fn read_cloud_header(source: &dyn Source) -> Result<Header> {
    let head = source.read(0, (source.size() as usize).min(8))?;
    if is_e57(&head) {
        Ok(Header::E57(read_e57_header(source)?))
    } else {
        Ok(Header::Las(read_las_header(source)?))
    }
}

/// The points, batch by batch (`readCloudPoints`).
pub fn read_cloud_points(source: &dyn Source, header: &Header, o: ReadOptions, each: &mut dyn FnMut(Batch) -> Result<()>) -> Result<()> {
    match header {
        Header::Las(h) => read_las_points(source, h, o, each),
        Header::E57(h) => read_e57_points(source, h, o, each),
    }
}

/// The first `n` points (`readFirstPoints`): [x, y, z, intensity].
pub fn read_first_points(source: &dyn Source, header: &Header, n: usize) -> Result<Vec<[f64; 4]>> {
    let mut out = Vec::new();
    if n == 0 {
        return Ok(out);
    }
    let r = read_cloud_points(source, header, ReadOptions::default(), &mut |b| {
        for i in 0..b.count {
            if out.len() >= n {
                break;
            }
            out.push([b.x[i], b.y[i], b.z[i], f64::from(b.intensity[i])]);
        }
        if out.len() >= n { Err(Error::Aborted) } else { Ok(()) }
    });
    match r {
        Ok(()) | Err(Error::Aborted) => Ok(out),
        Err(e) => Err(e),
    }
}

/// Where a level's segments go: `append` stores one and gives its offset.
pub trait Writer {
    fn append(&mut self, bytes: &[u8]) -> Result<u64>;
}

/// A writer into memory.
#[derive(Default)]
pub struct MemoryWriter(pub Vec<u8>);

impl Writer for MemoryWriter {
    fn append(&mut self, bytes: &[u8]) -> Result<u64> {
        let at = self.0.len() as u64;
        self.0.extend_from_slice(bytes);
        Ok(at)
    }
}

/// What the import reports along the way.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImportProgress {
    pub read: Progress,
    pub kept: u64,
    pub bytes_written: u64,
    pub elapsed: f64,
    pub remaining: Option<f64>,
}

struct Sink<'w> {
    level: usize,
    tiles: Vec<(i64, i64, Vec<[f64; 4]>)>,
    index: HashMap<(i64, i64), usize>,
    bytes: u64,
    segments: u64,
    writer: &'w mut dyn Writer,
    error: Option<Error>,
}

fn file_color_shift(e57: bool, batch: &Batch) -> u32 {
    if e57 {
        return 8;
    }
    let mut max = 0u16;
    for k in 0..batch.count {
        for c in [&batch.red, &batch.green, &batch.blue].into_iter().flatten() {
            max = max.max(c[k]);
        }
    }
    if max == 0 { 8 } else { color_shift(max) }
}

/// Every level in `levels` in one pass (`importLevels`): `writers[k]` stores
/// the segments of `levels[k]`. Returns the index body of each level, by level.
pub fn import_levels(
    source: &dyn Source,
    header: &Header,
    mapper: Option<&dyn Fn(f64, f64) -> (f64, f64)>,
    writers: Vec<&mut dyn Writer>,
    levels: &[usize],
    idle_points: Option<u64>,
    mut on_progress: Option<&mut dyn FnMut(ImportProgress)>,
    aborted: Option<&dyn Fn() -> bool>,
) -> Result<Vec<(usize, Value)>> {
    let rgb = header.rgb();
    let e57 = matches!(header, Header::E57(_));
    let grid = levels.contains(&0).then(|| header.grid());
    let idle = idle_points.unwrap_or(2_000_000);
    let sinks: Vec<RefCell<Sink>> = levels
        .iter()
        .zip(writers)
        .map(|(&level, writer)| {
            RefCell::new(Sink {
                level,
                tiles: vec![],
                index: HashMap::new(),
                bytes: 0,
                segments: 0,
                writer,
                error: None,
            })
        })
        .collect();
    let mut builders: Vec<TileBuilder> = sinks
        .iter()
        .map(|sink| {
            let level = sink.borrow().level;
            let on_tile = Box::new(move |tx: i64, ty: i64, cols: Band| {
                let mut s = sink.borrow_mut();
                for band in split_into_bands(cols) {
                    let seg = if s.level == 0 { encode_grid_segment(&band) } else { encode_segment(&band) };
                    let offset = match s.writer.append(&seg.bytes) {
                        Ok(o) => o,
                        Err(e) => {
                            s.error.get_or_insert(e);
                            continue;
                        }
                    };
                    let k = match s.index.get(&(tx, ty)) {
                        Some(&k) => k,
                        None => {
                            s.tiles.push((tx, ty, vec![]));
                            let k = s.tiles.len() - 1;
                            s.index.insert((tx, ty), k);
                            k
                        }
                    };
                    s.tiles[k].2.push([offset as f64, seg.bytes.len() as f64, seg.count as f64, seg.z0]);
                    s.bytes += seg.bytes.len() as u64;
                    s.segments += 1;
                }
            }) as Box<dyn FnMut(i64, i64, Band)>;
            TileBuilder::new(on_tile, idle, if level == 0 { grid } else { None }, LEVELS[level], rgb)
        })
        .collect();
    let original = levels.iter().position(|&l| l == 0);
    let voxels: Vec<usize> = (0..levels.len()).filter(|&k| levels[k] != 0).collect();
    let convert = !voxels.is_empty();
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let (mut shift, mut cshift): (Option<u32>, u32) = (None, 0);
    let started = crate::clock();
    let progress_state = RefCell::new((0u64, 0u64));
    let mut on_read = |p: Progress| {
        let elapsed = crate::clock() - started;
        let share = if p.total_bytes > 0.0 { p.bytes / p.total_bytes } else { 0.0 };
        let (kept, written) = *progress_state.borrow();
        if let Some(f) = on_progress.as_mut() {
            f(ImportProgress {
                read: p,
                kept,
                bytes_written: written,
                elapsed,
                remaining: (share > 0.0).then(|| elapsed * (1.0 - share) / share),
            });
        }
    };
    let o = ReadOptions {
        on_progress: Some(&mut on_read),
        aborted,
    };
    let first_voxel = voxels.first().copied().or(original);
    read_cloud_points(source, header, o, &mut |batch| {
        let shift = *shift.get_or_insert_with(|| {
            let max = batch.intensity[..batch.count].iter().copied().max().unwrap_or(0);
            if rgb {
                cshift = file_color_shift(e57, &batch);
            }
            intensity_shift(max)
        });
        for k in 0..batch.count {
            let (fe, fn_, z) = (batch.x[k], batch.y[k], batch.z[k]);
            let (e, n) = match (convert, mapper) {
                (true, Some(m)) => m(fe, fn_),
                _ => (fe, fn_),
            };
            if !e.is_finite() || !n.is_finite() || !z.is_finite() {
                continue;
            }
            bounds[0] = bounds[0].min(e);
            bounds[3] = bounds[3].max(e);
            bounds[1] = bounds[1].min(n);
            bounds[4] = bounds[4].max(n);
            bounds[2] = bounds[2].min(z);
            bounds[5] = bounds[5].max(z);
            let (mut r, mut g, mut b) = (0u8, 0u8, 0u8);
            if rgb {
                let c = |v: &Option<Vec<u16>>| (v.as_ref().unwrap()[k] >> cshift).min(255) as u8;
                r = c(&batch.red);
                g = c(&batch.green);
                b = c(&batch.blue);
            }
            if let Some(o) = original {
                builders[o].add(fe, fn_, z, batch.intensity[k], r, g, b);
            }
            if !voxels.is_empty() {
                let i = (batch.intensity[k] >> shift).min(255);
                for &v in &voxels {
                    builders[v].add(e, n, z, i, r, g, b);
                }
            }
        }
        for b in builders.iter_mut() {
            b.flush_idle();
        }
        let kept = first_voxel.map(|k| builders[k].kept).unwrap_or(0);
        let written = sinks.iter().map(|s| s.borrow().bytes).sum();
        *progress_state.borrow_mut() = (kept, written);
        if let Some(e) = sinks.iter().find_map(|s| s.borrow_mut().error.take()) {
            return Err(e);
        }
        Ok(())
    })?;
    for b in builders.iter_mut() {
        b.flush_all();
    }
    if let Some(e) = sinks.iter().find_map(|s| s.borrow_mut().error.take()) {
        return Err(e);
    }
    let kept: Vec<u64> = builders.iter().map(|b| b.kept).collect();
    drop(builders);
    let n = rail_across_js::number;
    let bounds_json = json!({
        "minE": n(bounds[0]), "minN": n(bounds[1]), "minZ": n(bounds[2]),
        "maxE": n(bounds[3]), "maxN": n(bounds[4]), "maxZ": n(bounds[5]),
    });
    let mut out = Vec::new();
    for (k, sink) in sinks.into_iter().enumerate() {
        let s = sink.into_inner();
        let def = LEVELS[s.level];
        let mut body = json!({
            "version": INDEX_VERSION,
            "level": def.level,
            "tileSize": n(def.tile_size),
            "rgb": rgb,
            "sourcePoints": header.point_count(),
            "points": kept[k],
            "bytes": s.bytes,
            "segments": s.segments,
            "intensityShift": shift.unwrap_or(0),
            "bounds": bounds_json,
            "tiles": s.tiles.iter().map(|(tx, ty, segs)| json!([tx, ty, segs.iter().map(|q| q.iter().map(|v| n(*v)).collect::<Vec<_>>()).collect::<Vec<_>>()])).collect::<Vec<_>>(),
        });
        if def.level == 0 {
            body["resolution"] = json!("original");
            body["voxel"] = Value::Null;
            body["grid"] = grid.map(|g| g.to_json()).unwrap_or(Value::Null);
        } else {
            body["resolution"] = json!("voxel");
            body["voxel"] = json!(def.voxel);
            body["perMetre"] = n(def.per_metre);
        }
        out.push((s.level, body));
    }
    Ok(out)
}
