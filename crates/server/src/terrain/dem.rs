// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The Länder's DGM1 as map tiles in terrain-RGB to the centimetre
//! (`terrain/dem.js`): the height of every pixel's centre read from the DGM1
//! tile it falls in, gaps filled from DGM5 and then from the nearest pixel.

use std::io::{Cursor, Write};
use std::sync::Mutex;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use super::sources;
use super::store::{Fetch, TileStore, download};
use super::to_plane;

const SIZE: usize = 256;
pub const MIN_ZOOM: i64 = 16;
pub const MAX_ZOOM: i64 = 17;
const BASE_SHIFT: f64 = 10000.0;
const STEP: f64 = 0.01;
const DGM5_URL: &str = "https://sg.geodatenzentrum.de/gdz_basemapde_3d_gelaende/dgm5_rgb_tiles/{z}/{x}/{y}.png";
const DGM5_MAX_ZOOM: i64 = 15;
const DGM5_MEMORY: usize = 64;
const MAX_SOURCE_TILES: usize = 16;

fn pixel_lng_lat(z: i64, x: i64, y: i64) -> (Vec<f64>, Vec<f64>) {
    let n = 2f64.powi(z as i32);
    let mut lng = Vec::with_capacity(SIZE);
    let mut lat = Vec::with_capacity(SIZE);
    for i in 0..SIZE {
        let t = (i as f64 + 0.5) / SIZE as f64;
        lng.push((x as f64 + t) / n * 360.0 - 180.0);
        lat.push((std::f64::consts::PI * (1.0 - 2.0 * (y as f64 + t) / n)).sinh().atan() * 180.0 / std::f64::consts::PI);
    }
    (lng, lat)
}

fn dgm1_heights(z: i64, x: i64, y: i64, store: &TileStore) -> Result<(Vec<f64>, bool), String> {
    let (lng, lat) = pixel_lng_lat(z, x, y);
    let mut heights = vec![f64::NAN; SIZE * SIZE];
    let mut failed = false;
    let mut wanted = 0;
    for s in sources() {
        let [w, south, e, north] = s.bbox;
        let mut todo: Vec<((i64, i64), Vec<(usize, f64, f64)>)> = Vec::new();
        for r in 0..SIZE {
            if lat[r] < south || lat[r] > north {
                continue;
            }
            for c in 0..SIZE {
                let i = r * SIZE + c;
                if !heights[i].is_nan() || lng[c] < w || lng[c] > e {
                    continue;
                }
                let (east, northing) = to_plane(s.crs, lng[c], lat[r]);
                let key = s.tile_of(east, northing);
                match todo.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, list)) => list.push((i, east, northing)),
                    None => todo.push((key, vec![(i, east, northing)])),
                }
            }
        }
        wanted += todo.len();
        if wanted > MAX_SOURCE_TILES {
            return Err("map tile too large for the DGM1".into());
        }
        for ((ek, nk), pixels) in todo {
            let (tile, tile_failed) = store.get(s, ek, nk);
            failed |= tile_failed;
            let Some(tile) = tile else { continue };
            for (i, east, northing) in pixels {
                heights[i] = tile.height(east, northing).unwrap_or(f64::NAN);
            }
        }
    }
    Ok((heights, failed))
}

/// A DGM5 tile decoded to heights.
struct Grid {
    grid: Vec<f64>,
    size: usize,
}

fn decode_dgm5(bytes: &[u8]) -> Result<Grid, String> {
    let mut d = image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let (w, h) = d.dimensions();
    let channels = if d.has_alpha() { 4 } else { 3 };
    let mut buf = vec![0u8; d.output_buffer_size().ok_or("webp too large")?];
    d.read_image(&mut buf).map_err(|e| e.to_string())?;
    let n = (w * h) as usize;
    let mut grid = Vec::with_capacity(n);
    for i in 0..n {
        let raw = f64::from(buf[channels * i]) * 65536.0 + f64::from(buf[channels * i + 1]) * 256.0 + f64::from(buf[channels * i + 2]);
        grid.push(if raw == 100000.0 { f64::NAN } else { raw * 0.1 - 10000.0 });
    }
    Ok(Grid { grid, size: w as usize })
}

/// DGM5 tiles, decoded to heights, the last few kept.
pub struct Dgm5 {
    pub fetch: Fetch,
    memory: Mutex<Vec<(String, Option<std::sync::Arc<Grid>>)>>,
}

impl Default for Dgm5 {
    fn default() -> Self {
        Dgm5 {
            fetch: std::sync::Arc::new(download),
            memory: Mutex::new(Vec::new()),
        }
    }
}

impl Dgm5 {
    fn tile(&self, z: i64, x: i64, y: i64) -> Option<std::sync::Arc<Grid>> {
        let key = format!("{z}/{x}/{y}");
        if let Some((_, g)) = self.memory.lock().unwrap().iter().find(|(k, _)| *k == key) {
            return g.clone();
        }
        let url = DGM5_URL
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string());
        let grid = match (self.fetch)(&url) {
            Ok(Some(bytes)) => match decode_dgm5(&bytes) {
                Ok(g) => Some(std::sync::Arc::new(g)),
                Err(_) => return None,
            },
            Ok(None) => None,
            Err(_) => return None,
        };
        let mut mem = self.memory.lock().unwrap();
        mem.push((key, grid.clone()));
        while mem.len() > DGM5_MEMORY {
            mem.remove(0);
        }
        grid
    }

    /// DGM5 under map tile z/x/y at its pixel centres.
    pub fn heights(&self, z: i64, x: i64, y: i64) -> Option<Vec<f64>> {
        let dz = (z - DGM5_MAX_ZOOM).max(0);
        let tile = self.tile(z - dz, x >> dz, y >> dz)?;
        let k = 2f64.powi(dz as i32);
        let size = tile.size;
        let at = |v: f64, base: i64| ((v - (base << dz) as f64) / k * size as f64 - 0.5).max(0.0).min(size as f64 - 1.0);
        let mut out = vec![0.0; SIZE * SIZE];
        for r in 0..SIZE {
            let row = at(y as f64 + (r as f64 + 0.5) / SIZE as f64, y >> dz);
            let r0 = row.floor() as usize;
            let r1 = (r0 + 1).min(size - 1);
            let tr = row - r0 as f64;
            for c in 0..SIZE {
                let col = at(x as f64 + (c as f64 + 0.5) / SIZE as f64, x >> dz);
                let c0 = col.floor() as usize;
                let c1 = (c0 + 1).min(size - 1);
                let tc = col - c0 as f64;
                let g = &tile.grid;
                out[r * SIZE + c] = g[r0 * size + c0] * (1.0 - tc) * (1.0 - tr)
                    + g[r0 * size + c1] * tc * (1.0 - tr)
                    + g[r1 * size + c0] * (1.0 - tc) * tr
                    + g[r1 * size + c1] * tc * tr;
            }
        }
        Some(out)
    }
}

/// Every NaN set to the height of the nearest pixel that has one (`fillNearest`).
pub fn fill_nearest(heights: &mut [f64]) {
    const INF: f64 = 1e20;
    let mut col_row = vec![-1i64; SIZE * SIZE];
    for c in 0..SIZE {
        let mut last = -1i64;
        for r in 0..SIZE {
            if !heights[r * SIZE + c].is_nan() {
                last = r as i64;
            }
            col_row[r * SIZE + c] = last;
        }
        last = -1;
        for r in (0..SIZE).rev() {
            if !heights[r * SIZE + c].is_nan() {
                last = r as i64;
            }
            let up = col_row[r * SIZE + c];
            if last >= 0 && (up < 0 || last - (r as i64) < (r as i64) - up) {
                col_row[r * SIZE + c] = last;
            }
        }
    }
    let mut out = heights.to_vec();
    let mut f = vec![0.0; SIZE];
    let mut v = vec![0usize; SIZE];
    let mut zb = vec![0.0; SIZE + 1];
    for r in 0..SIZE {
        for c in 0..SIZE {
            let near = col_row[r * SIZE + c];
            f[c] = if near < 0 { INF } else { ((near - r as i64) as f64).powi(2) };
        }
        let mut k = 0usize;
        v[0] = 0;
        zb[0] = f64::NEG_INFINITY;
        zb[1] = f64::INFINITY;
        for q in 1..SIZE {
            let qf = q as f64;
            let mut s = ((f[q] + qf * qf) - (f[v[k]] + (v[k] * v[k]) as f64)) / (2.0 * qf - 2.0 * v[k] as f64);
            while s <= zb[k] {
                k -= 1;
                s = ((f[q] + qf * qf) - (f[v[k]] + (v[k] * v[k]) as f64)) / (2.0 * qf - 2.0 * v[k] as f64);
            }
            k += 1;
            v[k] = q;
            zb[k] = s;
            zb[k + 1] = f64::INFINITY;
        }
        k = 0;
        for c in 0..SIZE {
            while zb[k + 1] < c as f64 {
                k += 1;
            }
            let i = r * SIZE + c;
            if heights[i].is_nan() && f[v[k]] < INF {
                out[i] = heights[col_row[r * SIZE + v[k]] as usize * SIZE + v[k]];
            }
        }
    }
    heights.copy_from_slice(&out);
}

fn chunk(kind: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    let crc = crc32fast::hash(&body);
    out.extend(body);
    out.extend(crc.to_be_bytes());
    out
}

/// Heights as a terrain-RGB PNG in the centimetre encoding (`encode`).
pub fn encode(heights: &[f64]) -> Vec<u8> {
    let mut raw = vec![0u8; SIZE * (1 + 3 * SIZE)];
    for r in 0..SIZE {
        let row = r * (1 + 3 * SIZE);
        for c in 0..SIZE {
            let v = rail_across_js::js_round((heights[r * SIZE + c] + BASE_SHIFT) / STEP).clamp(0.0, 16_777_215.0);
            let v = if v.is_nan() { 0 } else { v as u32 };
            raw[row + 1 + 3 * c] = (v >> 16) as u8;
            raw[row + 2 + 3 * c] = (v >> 8) as u8;
            raw[row + 3 + 3 * c] = v as u8;
        }
    }
    let mut header = vec![0u8; 13];
    header[0..4].copy_from_slice(&(SIZE as u32).to_be_bytes());
    header[4..8].copy_from_slice(&(SIZE as u32).to_be_bytes());
    header[8] = 8;
    header[9] = 2;
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(&raw).unwrap();
    let mut png = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend(chunk(b"IHDR", &header));
    png.extend(chunk(b"IDAT", &z.finish().unwrap()));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn covers(z: i64, x: i64, y: i64) -> bool {
    let n = 2f64.powi(z as i32);
    let west = x as f64 / n * 360.0 - 180.0;
    let east = (x + 1) as f64 / n * 360.0 - 180.0;
    let lat = |t: f64| (std::f64::consts::PI * (1.0 - 2.0 * t / n)).sinh().atan() * 180.0 / std::f64::consts::PI;
    let (north, south) = (lat(y as f64), lat((y + 1) as f64));
    sources()
        .iter()
        .any(|s| s.bbox[0] <= east && west <= s.bbox[2] && s.bbox[1] <= north && south <= s.bbox[3])
}

/// (png, keep) for map tile z/x/y (`render`).
pub fn render(z: i64, x: i64, y: i64, store: &TileStore, dgm5: &Dgm5) -> Result<(Option<Vec<u8>>, bool), String> {
    if !covers(z, x, y) {
        return Ok((None, true));
    }
    let (mut heights, failed) = dgm1_heights(z, x, y, store)?;
    let gaps = heights.iter().filter(|h| h.is_nan()).count();
    if gaps == heights.len() {
        return Ok((None, !failed));
    }
    if gaps > 0 {
        if let Some(under) = dgm5.heights(z, x, y) {
            for (h, u) in heights.iter_mut().zip(under) {
                if h.is_nan() {
                    *h = u;
                }
            }
        }
        fill_nearest(&mut heights);
    }
    Ok((Some(encode(&heights)), !failed))
}
