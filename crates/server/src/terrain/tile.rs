// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A 1 m grid of heights from an XYZ list or a GeoTIFF (`terrain/tile.js`).

use std::io::{Cursor, Read};

use tiff::decoder::{Decoder, DecodingResult};
use tiff::tags::Tag;

use super::Fmt;

const NO_DATA_BELOW: f32 = -1000.0;

/// Row 0 the northernmost, (x0, y0) the centre of its first cell, NaN where there is no data.
#[derive(Clone, Debug)]
pub struct Tile {
    pub grid: Vec<f32>,
    pub rows: usize,
    pub cols: usize,
    pub x0: f64,
    pub y0: f64,
    pub label: String,
}

impl Tile {
    /// Bilinear between the four nearest cells, clamped at the edge; none where a cell with a say has no data.
    pub fn height(&self, e: f64, n: f64) -> Option<f64> {
        let (rows, cols) = (self.rows as f64, self.cols as f64);
        let c = (e - self.x0).max(0.0).min(cols - 1.0);
        let r = (self.y0 - n).max(0.0).min(rows - 1.0);
        let (c0, r0) = (c.floor() as usize, r.floor() as usize);
        let c1 = (c0 + 1).min(self.cols - 1);
        let r1 = (r0 + 1).min(self.rows - 1);
        let (tc, tr) = (c - c0 as f64, r - r0 as f64);
        let mut sum = 0.0;
        for (v, w) in [
            (self.grid[r0 * self.cols + c0], (1.0 - tc) * (1.0 - tr)),
            (self.grid[r0 * self.cols + c1], tc * (1.0 - tr)),
            (self.grid[r1 * self.cols + c0], (1.0 - tc) * tr),
            (self.grid[r1 * self.cols + c1], tc * tr),
        ] {
            if w <= 0.0 {
                continue;
            }
            if !v.is_finite() {
                return None;
            }
            sum += f64::from(v) * w;
        }
        Some(sum)
    }
}

/// An XYZ list as a tile, placed by its coordinates (`parseXyz`).
pub fn parse_xyz(text: &str, label: &str) -> Result<Tile, String> {
    let values: Vec<f64> = text
        .split_whitespace()
        .map(|s| rail_across_js::js_number(Some(&serde_json::Value::String(s.into()))))
        .collect();
    if values.len() < 3 || values.len() % 3 != 0 || values.iter().any(|v| v.is_nan()) {
        return Err("not an XYZ list".into());
    }
    let (mut x0, mut y0) = (f64::INFINITY, f64::NEG_INFINITY);
    for t in values.chunks(3) {
        x0 = x0.min(t[0]);
        y0 = y0.max(t[1]);
    }
    let (mut rows, mut cols) = (0usize, 0usize);
    for t in values.chunks(3) {
        cols = cols.max(rail_across_js::js_round(t[0] - x0) as usize + 1);
        rows = rows.max(rail_across_js::js_round(y0 - t[1]) as usize + 1);
    }
    let mut grid = vec![f32::NAN; rows * cols];
    for t in values.chunks(3) {
        let z = t[2] as f32;
        let k = rail_across_js::js_round(y0 - t[1]) as usize * cols + rail_across_js::js_round(t[0] - x0) as usize;
        grid[k] = if z < NO_DATA_BELOW { f32::NAN } else { z };
    }
    Ok(Tile {
        grid,
        rows,
        cols,
        x0,
        y0,
        label: label.into(),
    })
}

fn doubles(d: &mut Decoder<Cursor<&[u8]>>, tag: u16) -> Option<Vec<f64>> {
    d.get_tag_f64_vec(Tag::Unknown(tag)).ok()
}

/// A single-band GeoTIFF as a tile, placed by its tie point and pixel size (`parseTif`).
pub fn parse_tif(bytes: &[u8], label: &str) -> Result<Tile, String> {
    let mut d = Decoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if d.get_tag_u32(Tag::SamplesPerPixel).unwrap_or(1) != 1 {
        return Err("not a single band of heights".into());
    }
    let tie = doubles(&mut d, 33922);
    let scale = doubles(&mut d, 33550);
    let (Some(tie), Some(scale)) = (tie, scale) else {
        return Err("not georeferenced".into());
    };
    if tie.len() < 6 || scale.len() < 2 {
        return Err("not georeferenced".into());
    }
    let (sx, sy) = (scale[0], scale[1]);
    if (sx - 1.0).abs() > 1e-6 || (sy - 1.0).abs() > 1e-6 {
        return Err("not a 1 m grid".into());
    }
    // GTRasterTypeGeoKey (1025) in the GeoKey directory: 2 is PixelIsPoint.
    let keys = d.get_tag_u32_vec(Tag::Unknown(34735)).unwrap_or_default();
    let mut raster_type = 1;
    for k in keys.chunks(4).skip(1) {
        if k.len() == 4 && k[0] == 1025 && k[1] == 0 {
            raster_type = k[3];
        }
    }
    let half = if raster_type == 2 { 0.0 } else { 0.5 };
    let x0 = tie[3] - tie[0] * sx + half * sx;
    let y0 = tie[4] + tie[1] * sy - half * sy;
    let no_data: Option<f32> = d
        .get_tag_ascii_string(Tag::Unknown(42113))
        .ok()
        .and_then(|s| s.trim_matches(char::from(0)).trim().parse::<f64>().ok())
        .map(|v| v as f32);
    let (w, h) = d.dimensions().map_err(|e| e.to_string())?;
    let data = d.read_image().map_err(|e| e.to_string())?;
    let mut grid: Vec<f32> = match data {
        DecodingResult::F32(v) => v,
        DecodingResult::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::U8(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U32(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::I32(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::I8(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U64(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::I64(v) => v.into_iter().map(|x| x as f32).collect(),
        _ => return Err("unsupported sample format".into()),
    };
    for v in grid.iter_mut() {
        if no_data.is_some_and(|nd| *v == nd) || *v < NO_DATA_BELOW {
            *v = f32::NAN;
        }
    }
    Ok(Tile {
        grid,
        rows: h as usize,
        cols: w as usize,
        x0,
        y0,
        label: label.into(),
    })
}

/// The bytes of one download as a tile, by the source's format (`readTile`).
pub fn read_tile(bytes: &[u8], fmt: Fmt, label: &str) -> Result<Tile, String> {
    if fmt == Fmt::Tif {
        return parse_tif(bytes, label);
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let names: Vec<String> = zip.file_names().map(str::to_string).collect();
    let read = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str| -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        zip.by_name(name).map_err(|e| e.to_string())?.read_to_end(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    };
    if fmt == Fmt::TifZip {
        let name = names.iter().find(|n| {
            let l = n.to_lowercase();
            l.ends_with(".tif") || l.ends_with(".tiff")
        });
        let Some(name) = name else { return Err("no GeoTIFF in the archive".into()) };
        return parse_tif(&read(&mut zip, name)?, label);
    }
    let lists: Vec<&String> = names.iter().filter(|n| n.to_lowercase().ends_with(".xyz")).collect();
    if lists.is_empty() {
        return Err("no XYZ in the archive".into());
    }
    let mut text = Vec::new();
    for n in lists {
        text.push(String::from_utf8_lossy(&read(&mut zip, n)?).into_owned());
    }
    parse_xyz(&text.join("\n"), label)
}
