// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The tiles a point cloud is kept in after import (`tiles.js`; decisions
//! 116, 117, 145, 205, 206, 215): squares in the cloud's plane, each written
//! as one or more zlib segments — the voxel layout (7 bytes a point, 10 with
//! colour) or the original grid (14, 17 with colour, byte planes regrouped).
//! zlib's bytes may differ from the JavaScript's (fflate); what they hold
//! does not, and either side reads the other's.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use rail_across_js::js_round;
use serde_json::{Value, json};

use crate::{Error, Result};

/// Edge of a tile [m].
pub const TILE_SIZE: f64 = 2.0;

/// A level of detail (decision 206).
#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub level: u8,
    pub tile_size: f64,
    pub voxel: Option<f64>,
    pub per_metre: f64,
    pub tile_units: i64,
    pub voxel_units: i64,
}

pub const LEVELS: [Level; 5] = [
    Level {
        level: 0,
        tile_size: 2.0,
        voxel: None,
        per_metre: 0.0,
        tile_units: 0,
        voxel_units: 0,
    },
    Level {
        level: 1,
        tile_size: 2.0,
        voxel: Some(0.02),
        per_metre: 1000.0,
        tile_units: 2000,
        voxel_units: 20,
    },
    Level {
        level: 2,
        tile_size: 8.0,
        voxel: Some(0.08),
        per_metre: 500.0,
        tile_units: 4000,
        voxel_units: 40,
    },
    Level {
        level: 3,
        tile_size: 32.0,
        voxel: Some(0.32),
        per_metre: 125.0,
        tile_units: 4000,
        voxel_units: 40,
    },
    Level {
        level: 4,
        tile_size: 128.0,
        voxel: Some(1.28),
        per_metre: 31.25,
        tile_units: 4000,
        voxel_units: 40,
    },
];

pub fn levels_json() -> Value {
    Value::Array(
        LEVELS
            .iter()
            .map(|l| match l.voxel {
                None => json!({ "level": l.level, "tileSize": l.tile_size, "voxel": null }),
                Some(v) => json!({ "level": l.level, "tileSize": l.tile_size, "voxel": v, "perMetre": l.per_metre, "tileUnits": l.tile_units, "voxelUnits": l.voxel_units }),
            })
            .collect(),
    )
}

const Z_SPAN_UNITS: f64 = 65535.0;
const Z_SPAN_GRID: f64 = 4294967295.0;
const E57_GRID_STEP: f64 = 0.0001;

/// Shift bringing 16-bit colours into one byte.
pub fn color_shift(max: u16) -> u32 {
    if max > 255 { 8 } else { 0 }
}

/// Shift bringing a file's intensities into one byte, from the largest seen.
pub fn intensity_shift(max: u16) -> u32 {
    let mut shift = 0;
    while (max >> shift) > 255 {
        shift += 1;
    }
    shift
}

fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(6));
    e.write_all(raw).unwrap();
    e.finish().unwrap()
}

fn unzlib(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    ZlibDecoder::new(bytes).read_to_end(&mut out).map_err(|e| Error::msg(e.to_string()))?;
    Ok(out)
}

/// One band of a tile's points, ready to encode.
#[derive(Clone, Debug, Default)]
pub struct Band {
    pub x: Vec<u32>,
    pub y: Vec<u32>,
    pub z: Vec<f64>,
    pub i: Vec<u16>,
    pub rgb: Option<[Vec<u8>; 3]>,
    pub grid: bool,
}

impl Band {
    pub fn count(&self) -> usize {
        self.x.len()
    }
    fn push(&mut self, k: usize, from: &Band) {
        self.x.push(from.x[k]);
        self.y.push(from.y[k]);
        self.z.push(from.z[k]);
        self.i.push(from.i[k]);
        if let (Some(to), Some(src)) = (self.rgb.as_mut(), from.rgb.as_ref()) {
            for c in 0..3 {
                to[c].push(src[c][k]);
            }
        }
    }
}

/// An encoded segment: its bytes, its count and the height its z counts from.
#[derive(Clone, Debug)]
pub struct Segment {
    pub bytes: Vec<u8>,
    pub count: usize,
    pub z0: f64,
}

fn order_of(b: &Band) -> Vec<usize> {
    let mut order: Vec<usize> = (0..b.count()).collect();
    order.sort_by(|&p, &q| {
        let d = b.z[p] - b.z[q];
        let d = if d != 0.0 { d } else { f64::from(b.y[p]) - f64::from(b.y[q]) };
        let d = if d != 0.0 { d } else { f64::from(b.x[p]) - f64::from(b.x[q]) };
        d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    order
}

fn delta_bytes(values: &[u8], order: &[usize]) -> Vec<u8> {
    let mut p = 0u8;
    order
        .iter()
        .map(|&k| {
            let d = values[k].wrapping_sub(p);
            p = values[k];
            d
        })
        .collect()
}

fn undelta_bytes(deltas: &[u8]) -> Vec<u8> {
    let mut p = 0u8;
    deltas
        .iter()
        .map(|d| {
            p = p.wrapping_add(*d);
            p
        })
        .collect()
}

/// One voxel segment in bytes (`encodeSegment`).
pub fn encode_segment(b: &Band) -> Segment {
    let count = b.count();
    let z0 = b.z.iter().copied().fold(f64::INFINITY, f64::min);
    let order = order_of(b);
    let mut raw = vec![0u8; count * if b.rgb.is_some() { 10 } else { 7 }];
    let (mut px, mut py, mut pz) = (0i64, 0i64, 0i64);
    for (n, &k) in order.iter().enumerate() {
        let zz = (b.z[k] - z0) as i64;
        let put = |raw: &mut Vec<u8>, at: usize, v: i64| raw[at..at + 2].copy_from_slice(&((v & 0xFFFF) as u16).to_le_bytes());
        put(&mut raw, 2 * n, i64::from(b.x[k]) - px);
        put(&mut raw, count * 2 + 2 * n, i64::from(b.y[k]) - py);
        put(&mut raw, count * 4 + 2 * n, zz - pz);
        raw[count * 6 + n] = b.i[k] as u8;
        px = i64::from(b.x[k]);
        py = i64::from(b.y[k]);
        pz = zz;
    }
    if let Some(rgb) = &b.rgb {
        for c in 0..3 {
            raw[count * (7 + c)..count * (8 + c)].copy_from_slice(&delta_bytes(&rgb[c], &order));
        }
    }
    Segment { bytes: zlib(&raw), count, z0 }
}

/// A decoded segment: x, y, z in units (or grid steps), the intensity byte,
/// the intensity as stored (original grid) and the colour.
#[derive(Clone, Debug, Default)]
pub struct Decoded {
    pub x: Vec<u32>,
    pub y: Vec<u32>,
    pub z: Vec<u32>,
    pub i: Vec<u8>,
    pub intensity: Option<Vec<u16>>,
    pub rgb: Option<[Vec<u8>; 3]>,
}

impl Decoded {
    pub fn bytes(&self) -> usize {
        self.x.len() * 13 + self.intensity.as_ref().map_or(0, |v| v.len() * 2) + self.rgb.as_ref().map_or(0, |c| c[0].len() * 3)
    }

    pub fn to_json(&self) -> Value {
        let mut v = json!({ "x": self.x, "y": self.y, "z": self.z, "i": self.i });
        if let Some(intensity) = &self.intensity {
            v["intensity"] = json!(intensity);
        }
        if let Some([r, g, b]) = &self.rgb {
            v["r"] = json!(r);
            v["g"] = json!(g);
            v["b"] = json!(b);
        }
        v
    }
}

fn u16s(raw: &[u8], at: usize, count: usize) -> Vec<u16> {
    (0..count).map(|n| u16::from_le_bytes([raw[at + 2 * n], raw[at + 2 * n + 1]])).collect()
}

fn u32s(raw: &[u8], at: usize, count: usize) -> Vec<u32> {
    (0..count)
        .map(|n| u32::from_le_bytes(raw[at + 4 * n..at + 4 * n + 4].try_into().unwrap()))
        .collect()
}

/// A voxel segment back (`decodeSegment`).
pub fn decode_segment(bytes: &[u8], count: usize, rgb: bool) -> Result<Decoded> {
    let raw = unzlib(bytes)?;
    if raw.len() < count * if rgb { 10 } else { 7 } {
        return Err(Error::msg("segment too short"));
    }
    let (xs, ys, zs) = (u16s(&raw, 0, count), u16s(&raw, count * 2, count), u16s(&raw, count * 4, count));
    let (mut px, mut py, mut pz) = (0u16, 0u16, 0u16);
    let mut d = Decoded {
        x: Vec::with_capacity(count),
        y: Vec::with_capacity(count),
        z: Vec::with_capacity(count),
        ..Decoded::default()
    };
    for n in 0..count {
        px = px.wrapping_add(xs[n]);
        py = py.wrapping_add(ys[n]);
        pz = pz.wrapping_add(zs[n]);
        d.x.push(u32::from(px));
        d.y.push(u32::from(py));
        d.z.push(u32::from(pz));
    }
    d.i = raw[count * 6..count * 7].to_vec();
    if rgb {
        d.rgb = Some([0, 1, 2].map(|c| undelta_bytes(&raw[count * (7 + c)..count * (8 + c)])));
    }
    Ok(d)
}

/// The grid of a file's original resolution: scale and offset per axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub scale: [f64; 3],
    pub offset: [f64; 3],
}

impl Grid {
    pub fn to_json(&self) -> Value {
        let n = |v: &[f64; 3]| json!(v.iter().map(|x| rail_across_js::number(*x)).collect::<Vec<_>>());
        json!({ "scale": n(&self.scale), "offset": n(&self.offset) })
    }
    pub fn from_json(v: &Value) -> Option<Grid> {
        let three = |k: &str| -> Option<[f64; 3]> {
            let a = v.get(k)?.as_array()?;
            Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?])
        };
        Some(Grid {
            scale: three("scale")?,
            offset: three("offset")?,
        })
    }
}

/// The grid the original resolution keeps a file's points on (`originalGrid`).
pub fn original_grid(e57: bool, scale: [f64; 3], offset: [f64; 3]) -> Grid {
    if e57 {
        let s = if scale[0] != 0.0 && !scale[0].is_nan() { scale[0] } else { E57_GRID_STEP };
        let step = E57_GRID_STEP.min(s);
        return Grid {
            scale: [step; 3],
            offset: [0.0; 3],
        };
    }
    Grid { scale, offset }
}

fn grid_base(t: i64, step: f64, offset: f64) -> f64 {
    ((t as f64 * TILE_SIZE - offset) / step).floor()
}

/// Where a segment's points lie (`segmentPlacement`): [ox, oy, oz, sx, sy, sz].
pub fn segment_placement(tile_size: f64, per_metre: Option<f64>, grid: Option<&Grid>, tx: i64, ty: i64, z0: f64) -> [f64; 6] {
    match grid {
        None => {
            let unit = 1.0 / per_metre.unwrap_or(1000.0);
            [tx as f64 * tile_size, ty as f64 * tile_size, z0 * unit, unit, unit, unit]
        }
        Some(g) => {
            let [sx, sy, sz] = g.scale;
            let [gx, gy, gz] = g.offset;
            [gx + grid_base(tx, sx, gx) * sx, gy + grid_base(ty, sy, gy) * sy, gz + z0 * sz, sx, sy, sz]
        }
    }
}

const GRID_COLUMNS: [usize; 4] = [4, 4, 4, 2];
const GRID_RGB_COLUMNS: [usize; 7] = [4, 4, 4, 2, 1, 1, 1];

fn shuffle(raw: &[u8], columns: &[usize], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut start = 0;
    for &w in columns {
        for b in 0..w {
            for k in 0..count {
                out.push(raw[start + k * w + b]);
            }
        }
        start += count * w;
    }
    out
}

fn unshuffle(bytes: &[u8], columns: &[usize], count: usize) -> Vec<u8> {
    let mut out = vec![0u8; bytes.len()];
    let (mut o, mut start) = (0, 0);
    for &w in columns {
        for b in 0..w {
            for k in 0..count {
                out[start + k * w + b] = bytes[o];
                o += 1;
            }
        }
        start += count * w;
    }
    out
}

/// One segment of the original resolution in bytes (`encodeGridSegment`).
pub fn encode_grid_segment(b: &Band) -> Segment {
    let count = b.count();
    let z0 = b.z.iter().copied().fold(f64::INFINITY, f64::min);
    let order = order_of(b);
    let rgb = b.rgb.is_some();
    let mut raw = vec![0u8; count * if rgb { 17 } else { 14 }];
    let (mut px, mut py, mut pz) = (0i64, 0i64, 0i64);
    for (n, &k) in order.iter().enumerate() {
        let zz = (b.z[k] - z0) as i64;
        let put = |raw: &mut Vec<u8>, at: usize, v: i64| raw[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
        put(&mut raw, 4 * n, i64::from(b.x[k]) - px);
        put(&mut raw, count * 4 + 4 * n, i64::from(b.y[k]) - py);
        put(&mut raw, count * 8 + 4 * n, zz - pz);
        raw[count * 12 + 2 * n..count * 12 + 2 * n + 2].copy_from_slice(&b.i[k].to_le_bytes());
        px = i64::from(b.x[k]);
        py = i64::from(b.y[k]);
        pz = zz;
    }
    if let Some(c) = &b.rgb {
        for ch in 0..3 {
            raw[count * (14 + ch)..count * (15 + ch)].copy_from_slice(&delta_bytes(&c[ch], &order));
        }
    }
    let columns: &[usize] = if rgb { &GRID_RGB_COLUMNS } else { &GRID_COLUMNS };
    Segment {
        bytes: zlib(&shuffle(&raw, columns, count)),
        count,
        z0,
    }
}

/// A segment of the original resolution back (`decodeGridSegment`).
pub fn decode_grid_segment(bytes: &[u8], count: usize, shift: u32, rgb: bool) -> Result<Decoded> {
    let columns: &[usize] = if rgb { &GRID_RGB_COLUMNS } else { &GRID_COLUMNS };
    let unz = unzlib(bytes)?;
    if unz.len() < count * if rgb { 17 } else { 14 } {
        return Err(Error::msg("segment too short"));
    }
    let raw = unshuffle(&unz, columns, count);
    let (xs, ys, zs) = (u32s(&raw, 0, count), u32s(&raw, count * 4, count), u32s(&raw, count * 8, count));
    let intensity = u16s(&raw, count * 12, count);
    let (mut px, mut py, mut pz) = (0u32, 0u32, 0u32);
    let mut d = Decoded::default();
    for n in 0..count {
        px = px.wrapping_add(xs[n]);
        py = py.wrapping_add(ys[n]);
        pz = pz.wrapping_add(zs[n]);
        d.x.push(px);
        d.y.push(py);
        d.z.push(pz);
        d.i.push((intensity[n] >> shift).min(255) as u8);
    }
    d.intensity = Some(intensity);
    if rgb {
        d.rgb = Some([0, 1, 2].map(|c| undelta_bytes(&raw[count * (14 + c)..count * (15 + c)])));
    }
    Ok(d)
}

/// The points of one tile cut into height bands (`splitIntoBands`).
pub fn split_into_bands(cols: Band) -> Vec<Band> {
    let span = if cols.grid { Z_SPAN_GRID } else { Z_SPAN_UNITS };
    let z_min = cols.z.iter().copied().fold(f64::INFINITY, f64::min);
    let z_max = cols.z.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if z_max - z_min <= span {
        return vec![cols];
    }
    let mut order: Vec<i64> = Vec::new();
    let mut bands: HashMap<i64, Band> = HashMap::new();
    for k in 0..cols.count() {
        let b = ((cols.z[k] - z_min) / (span + 1.0)).floor() as i64;
        let band = bands.entry(b).or_insert_with(|| {
            order.push(b);
            Band {
                grid: cols.grid,
                rgb: cols.rgb.as_ref().map(|_| [vec![], vec![], vec![]]),
                ..Band::default()
            }
        });
        band.push(k, &cols);
    }
    order.into_iter().map(|b| bands.remove(&b).unwrap()).collect()
}

struct Tile {
    tx: i64,
    ty: i64,
    cols: Band,
    voxels: HashSet<i64>,
    bx: f64,
    by: f64,
    last: u64,
}

/// Sorts points into tiles and thins them on the way (`TileBuilder`): of the
/// points in one voxel of `level` the first is kept; with a `grid` every
/// point, in grid steps. A tile idle for `idle_points` points is handed to
/// `on_tile` and forgotten.
pub struct TileBuilder<'a> {
    on_tile: Box<dyn FnMut(i64, i64, Band) + 'a>,
    idle_points: u64,
    grid: Option<Grid>,
    level: Level,
    rgb: bool,
    keys: Vec<Option<i64>>,
    tiles: HashMap<i64, (usize, Tile)>,
    pub seen: u64,
    pub kept: u64,
}

fn tile_key(tx: i64, ty: i64) -> i64 {
    (tx + 33554432) * 67108864 + (ty + 33554432)
}

impl<'a> TileBuilder<'a> {
    pub fn new(on_tile: Box<dyn FnMut(i64, i64, Band) + 'a>, idle_points: u64, grid: Option<Grid>, level: Level, rgb: bool) -> Self {
        TileBuilder {
            on_tile,
            idle_points,
            grid,
            level,
            rgb,
            keys: vec![],
            tiles: HashMap::new(),
            seen: 0,
            kept: 0,
        }
    }

    fn tile(&mut self, tx: i64, ty: i64, make: impl FnOnce() -> Tile) -> &mut Tile {
        let key = tile_key(tx, ty);
        if !self.tiles.contains_key(&key) {
            self.keys.push(Some(key));
            self.tiles.insert(key, (self.keys.len() - 1, make()));
        }
        &mut self.tiles.get_mut(&key).unwrap().1
    }

    pub fn add(&mut self, e: f64, n: f64, z: f64, intensity: u16, r: u8, g: u8, b: u8) {
        if let Some(grid) = self.grid {
            self.add_on_grid(grid, e, n, z, intensity, r, g, b);
            return;
        }
        let Level {
            per_metre,
            tile_units,
            voxel_units,
            ..
        } = self.level;
        let (x, y, zz) = (js_round(e * per_metre), js_round(n * per_metre), js_round(z * per_metre));
        let tx = (x / tile_units as f64).floor() as i64;
        let ty = (y / tile_units as f64).floor() as i64;
        let lx = x - (tx * tile_units) as f64;
        let ly = y - (ty * tile_units) as f64;
        let rgb = self.rgb;
        self.seen += 1;
        let seen = self.seen;
        let tile = self.tile(tx, ty, || Tile {
            tx,
            ty,
            cols: Band {
                rgb: rgb.then(|| [vec![], vec![], vec![]]),
                ..Band::default()
            },
            voxels: HashSet::new(),
            bx: 0.0,
            by: 0.0,
            last: 0,
        });
        tile.last = seen;
        let per = (tile_units / voxel_units) as f64;
        let vu = voxel_units as f64;
        let voxel = ((zz / vu).floor() * per * per + (ly / vu).floor() * per + (lx / vu).floor()) as i64;
        if !tile.voxels.insert(voxel) {
            return;
        }
        // The voxel layout keeps x and y as Uint16 and z as Int32.
        tile.cols.x.push(lx as u16 as u32);
        tile.cols.y.push(ly as u16 as u32);
        tile.cols.z.push(f64::from(zz as i64 as i32));
        tile.cols.i.push(intensity & 0xFF);
        if let Some(c) = tile.cols.rgb.as_mut() {
            c[0].push(r);
            c[1].push(g);
            c[2].push(b);
        }
        self.kept += 1;
    }

    fn add_on_grid(&mut self, grid: Grid, e: f64, n: f64, z: f64, intensity: u16, r: u8, g: u8, b: u8) {
        let [sx, sy, sz] = grid.scale;
        let [gx, gy, gz] = grid.offset;
        let (u, v, w) = (js_round((e - gx) / sx), js_round((n - gy) / sy), js_round((z - gz) / sz));
        let tx = ((gx + u * sx) / TILE_SIZE).floor() as i64;
        let ty = ((gy + v * sy) / TILE_SIZE).floor() as i64;
        let rgb = self.rgb;
        self.seen += 1;
        let seen = self.seen;
        let tile = self.tile(tx, ty, || Tile {
            tx,
            ty,
            cols: Band {
                grid: true,
                rgb: rgb.then(|| [vec![], vec![], vec![]]),
                ..Band::default()
            },
            voxels: HashSet::new(),
            bx: grid_base(tx, sx, gx),
            by: grid_base(ty, sy, gy),
            last: 0,
        });
        tile.last = seen;
        tile.cols.x.push((u - tile.bx) as i64 as u32);
        tile.cols.y.push((v - tile.by) as i64 as u32);
        tile.cols.z.push(w);
        tile.cols.i.push(intensity);
        if let Some(c) = tile.cols.rgb.as_mut() {
            c[0].push(r);
            c[1].push(g);
            c[2].push(b);
        }
        self.kept += 1;
    }

    /// Hand on every tile nobody has added to for a while.
    pub fn flush_idle(&mut self) {
        for slot in 0..self.keys.len() {
            let Some(key) = self.keys[slot] else { continue };
            if self.seen - self.tiles[&key].1.last > self.idle_points {
                self.emit(key);
            }
        }
        self.compact();
    }

    /// Hand on every tile still held.
    pub fn flush_all(&mut self) {
        for slot in 0..self.keys.len() {
            if let Some(key) = self.keys[slot] {
                self.emit(key);
            }
        }
        self.compact();
    }

    fn emit(&mut self, key: i64) {
        let (slot, tile) = self.tiles.remove(&key).unwrap();
        self.keys[slot] = None;
        if tile.cols.count() > 0 {
            (self.on_tile)(tile.tx, tile.ty, tile.cols);
        }
    }

    fn compact(&mut self) {
        if self.keys.len() < 64 || self.keys.len() < 2 * self.tiles.len() {
            return;
        }
        self.keys.retain(Option::is_some);
        for (slot, key) in self.keys.iter().enumerate() {
            self.tiles.get_mut(&key.unwrap()).unwrap().0 = slot;
        }
    }
}
