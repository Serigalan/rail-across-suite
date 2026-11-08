// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The points of the project's clouds in a cross section (`cloudSection.js`),
//! read through each cloud's tile source; decoded segments are kept in a
//! small cache across calls, counted in bytes.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use rail_across_geom::coords::{transform_grid_bearing, transform_plane_point};
use serde_json::Value;

use crate::crs::{Transform, cloud_to_plane, plane_to_cloud};
use crate::slice::{SlicePoints, slice_frame, slice_segment, tiles_in_slice};
use crate::source::TileSource;
use crate::tiles::{Decoded, Grid, decode_grid_segment, decode_segment, segment_placement};
use crate::{Error, Result};

const CACHE_BYTES: usize = 64 * 1024 * 1024;

/// A cloud as a section reads it: its index (tiles, layout, bounds), its plane,
/// its re-referencing and where its tile file lies.
pub struct Cloud {
    pub id: String,
    /// The level of a server cloud: its levels are clouds of their own in the cache.
    pub level: Option<i64>,
    pub tile_size: f64,
    pub per_metre: Option<f64>,
    pub grid: Option<Grid>,
    pub intensity_shift: u32,
    pub rgb: bool,
    pub crs: Option<i64>,
    pub transform: Option<Transform>,
    /// [minE, minN, minZ, maxE, maxN, maxZ]
    pub bounds: [f64; 6],
    pub tiles: Vec<(i64, i64, Vec<[f64; 4]>)>,
    pub source: Arc<dyn TileSource>,
}

impl Cloud {
    /// A cloud from its index body (as `importLevels` writes it) and a source.
    pub fn from_index(id: &str, level: Option<i64>, index: &Value, crs: Option<i64>, transform: Option<Transform>, source: Arc<dyn TileSource>) -> Cloud {
        let f = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64);
        let b = index.get("bounds").cloned().unwrap_or(Value::Null);
        let tiles = index
            .get("tiles")
            .and_then(Value::as_array)
            .map(|ts| {
                ts.iter()
                    .map(|t| {
                        let segs = t[2]
                            .as_array()
                            .map(|s| s.iter().map(|q| std::array::from_fn(|i| q[i].as_f64().unwrap_or(0.0))).collect())
                            .unwrap_or_default();
                        (t[0].as_i64().unwrap_or(0), t[1].as_i64().unwrap_or(0), segs)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Cloud {
            id: id.to_string(),
            level,
            tile_size: f(index, "tileSize").unwrap_or(2.0),
            per_metre: f(index, "perMetre"),
            grid: index.get("grid").and_then(Grid::from_json),
            intensity_shift: f(index, "intensityShift").unwrap_or(0.0) as u32,
            rgb: rail_across_js::truthy(index.get("rgb")),
            crs,
            transform,
            bounds: ["minE", "minN", "minZ", "maxE", "maxN", "maxZ"].map(|k| f(&b, k).unwrap_or(f64::NAN)),
            tiles,
            source,
        }
    }

    fn key(&self) -> String {
        match self.level {
            Some(l) => format!("{}@L{l}", self.id),
            None => self.id.clone(),
        }
    }

    fn decode(&self, bytes: &[u8], count: usize) -> Result<Decoded> {
        match &self.grid {
            Some(_) => decode_grid_segment(bytes, count, self.intensity_shift, self.rgb),
            None => decode_segment(bytes, count, self.rgb),
        }
    }
}

#[derive(Default)]
struct Cache {
    map: HashMap<String, Arc<Decoded>>,
    order: VecDeque<String>,
    bytes: usize,
}

impl Cache {
    fn touch(&mut self, key: &str) {
        if let Some(p) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(p).unwrap();
            self.order.push_back(k);
        }
    }
    fn forget(&mut self, key: &str) {
        if let Some(seg) = self.map.remove(key) {
            self.bytes -= seg.bytes();
            if let Some(p) = self.order.iter().position(|k| k == key) {
                self.order.remove(p);
            }
        }
    }
    fn remember(&mut self, key: String, seg: Arc<Decoded>) {
        self.forget(&key);
        self.bytes += seg.bytes();
        self.map.insert(key.clone(), seg);
        self.order.push_back(key);
        while self.bytes > CACHE_BYTES && self.map.len() > 1 {
            let oldest = self.order.front().cloned().unwrap();
            self.forget(&oldest);
        }
    }
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn with_cache<T>(f: impl FnOnce(&mut Cache) -> T) -> T {
    let mut guard = CACHE.lock().unwrap();
    f(guard.get_or_insert_with(Cache::default))
}

/// Forget what was read of a cloud (`forgetCloud`).
pub fn forget_cloud(id: &str) {
    with_cache(|c| {
        let keys: Vec<String> = c
            .map
            .keys()
            .filter(|k| k.starts_with(&format!("{id}|")) || k.starts_with(&format!("{id}@")))
            .cloned()
            .collect();
        for k in keys {
            c.forget(&k);
        }
    });
}

/// Forget what was read of every cloud (`forgetAllClouds`).
pub fn forget_all_clouds() {
    with_cache(|c| *c = Cache::default());
}

fn segments(cloud: &Cloud, wanted: &[[f64; 4]]) -> Result<HashMap<u64, Arc<Decoded>>> {
    let key = cloud.key();
    let mut got = HashMap::new();
    let mut missing = Vec::new();
    with_cache(|c| {
        for s in wanted {
            let k = format!("{key}|{}", s[0]);
            match c.map.get(&k).cloned() {
                Some(hit) => {
                    c.touch(&k);
                    got.insert(s[0] as u64, hit);
                }
                None => missing.push(*s),
            }
        }
    });
    if missing.is_empty() {
        return Ok(got);
    }
    let bytes = cloud
        .source
        .read_many(&missing.iter().map(|s| (s[0] as u64, s[1] as usize)).collect::<Vec<_>>())?;
    for (s, b) in missing.iter().zip(bytes) {
        let seg = Arc::new(cloud.decode(&b, s[2] as usize)?);
        with_cache(|c| c.remember(format!("{key}|{}", s[0]), seg.clone()));
        got.insert(s[0] as u64, seg);
    }
    Ok(got)
}

/// The points of `cloud` in the section through `origin` (in plane `crs`)
/// square to `bearing` (`cloudSectionPoints`).
pub fn cloud_section_points(cloud: &Cloud, origin: (f64, f64), bearing: f64, crs: i64, half_width: f64, thickness: f64) -> Result<SlicePoints> {
    let mut out = SlicePoints::new(cloud.rgb);
    let frame = slice_frame(origin.0, origin.1, bearing, half_width, thickness);
    let to_plane = cloud_to_plane(cloud.crs, cloud.transform.as_ref(), crs)?;
    let coords: Vec<(i64, i64)> = cloud.tiles.iter().map(|t| (t.0, t.1)).collect();
    let tiles = match (&to_plane, &cloud.transform) {
        (None, _) => tiles_in_slice(cloud.tile_size, &coords, &frame),
        (Some(_), None) => {
            let from = cloud.crs.unwrap_or(-1);
            let (e, n) = transform_plane_point(origin.0, origin.1, crs, from).map_err(|e| Error::msg(e.0))?;
            let b = transform_grid_bearing(origin.0, origin.1, bearing, crs, from).map_err(|e| Error::msg(e.0))?;
            tiles_in_slice(cloud.tile_size, &coords, &slice_frame(e, n, b, half_width * 1.01 + 0.1, thickness + 0.2))
        }
        (Some(to), Some(t)) => {
            let back = plane_to_cloud(cloud.crs, Some(t), crs)?.unwrap();
            let m = t.matrix;
            let [min_e, min_n, min_z, max_e, max_n, max_z] = cloud.bounds;
            let z_mid = to.apply((min_e + max_e) / 2.0, (min_n + max_n) / 2.0, (min_z + max_z) / 2.0).2;
            let (e, n, _) = back.apply(origin.0, origin.1, z_mid);
            let rad = bearing * std::f64::consts::PI / 180.0;
            let (e2, n2, _) = back.apply(origin.0 + rad.sin(), origin.1 + rad.cos(), z_mid);
            let b = (e2 - e).atan2(n2 - n) * 180.0 / std::f64::consts::PI;
            let slack = rail_across_js::js_hypot(&[m[2], m[6]]) * (max_z - min_z) + 0.1;
            tiles_in_slice(
                cloud.tile_size,
                &coords,
                &slice_frame(e, n, b, half_width * 1.01 + slack, thickness + 2.0 * slack),
            )
        }
    };
    let wanted: Vec<[f64; 4]> = tiles.iter().flat_map(|&k| cloud.tiles[k].2.iter().copied()).collect();
    let decoded = segments(cloud, &wanted)?;
    for &k in &tiles {
        let (tx, ty, segs) = &cloud.tiles[k];
        for s in segs {
            let place = segment_placement(cloud.tile_size, cloud.per_metre, cloud.grid.as_ref(), *tx, *ty, s[3]);
            slice_segment(&mut out, &decoded[&(s[0] as u64)], place, &frame, to_plane.as_ref());
        }
    }
    out.tiles = tiles.len();
    Ok(out)
}
