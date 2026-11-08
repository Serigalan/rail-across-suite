// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The slice of a point cloud a cross section shows (`cloudSlice.js`): the
//! points within half a thickness of the section plane and the drawing's
//! width, as (y across, z up) — y to the right of the running direction.

use serde_json::{Value, json};

use crate::crs::Chain;
use crate::tiles::Decoded;

/// The frame of a section in a plane.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub e: f64,
    pub n: f64,
    pub along: [f64; 2],
    pub right: [f64; 2],
    pub half_width: f64,
    pub half: f64,
}

/// The frame through (easting, northing) square to `bearing` (`sliceFrame`).
pub fn slice_frame(easting: f64, northing: f64, bearing: f64, half_width: f64, thickness: f64) -> Frame {
    let rad = bearing * std::f64::consts::PI / 180.0;
    Frame {
        e: easting,
        n: northing,
        along: [rad.sin(), rad.cos()],
        right: [rad.cos(), -rad.sin()],
        half_width,
        half: thickness / 2.0,
    }
}

impl Frame {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        json!({ "e": n(self.e), "n": n(self.n), "along": self.along.map(n), "right": self.right.map(n), "halfWidth": n(self.half_width), "half": n(self.half) })
    }
}

/// The tiles of an index the slice can reach (`tilesInSlice`): indices into `tiles`.
pub fn tiles_in_slice(tile_size: f64, tiles: &[(i64, i64)], f: &Frame) -> Vec<usize> {
    let size = tile_size;
    let ext = |a: [f64; 2]| (a[0].abs() + a[1].abs()) * size / 2.0;
    let (ext_along, ext_right) = (ext(f.along), ext(f.right));
    let bx = f.right[0].abs() * f.half_width + f.along[0].abs() * f.half;
    let by = f.right[1].abs() * f.half_width + f.along[1].abs() * f.half;
    (0..tiles.len())
        .filter(|&k| {
            let (tx, ty) = tiles[k];
            let cx = (tx as f64 + 0.5) * size - f.e;
            let cy = (ty as f64 + 0.5) * size - f.n;
            if cx.abs() > bx + size / 2.0 || cy.abs() > by + size / 2.0 {
                return false;
            }
            if (cx * f.along[0] + cy * f.along[1]).abs() > f.half + ext_along {
                return false;
            }
            (cx * f.right[0] + cy * f.right[1]).abs() <= f.half_width + ext_right
        })
        .collect()
}

/// The points of a slice: y as the JavaScript's Float32Array holds it, z, intensity, colour.
#[derive(Clone, Debug, Default)]
pub struct SlicePoints {
    pub y: Vec<f32>,
    pub z: Vec<f64>,
    pub i: Vec<u8>,
    pub rgb: Option<Vec<u8>>,
    pub tiles: usize,
}

impl SlicePoints {
    pub fn new(rgb: bool) -> Self {
        SlicePoints {
            rgb: rgb.then(Vec::new),
            ..SlicePoints::default()
        }
    }
    pub fn count(&self) -> usize {
        self.y.len()
    }
    pub fn push(&mut self, y: f64, z: f64, i: u8, c: [u8; 3]) {
        self.y.push(y as f32);
        self.z.push(z);
        self.i.push(i);
        if let Some(rgb) = self.rgb.as_mut() {
            rgb.extend_from_slice(&c);
        }
    }
    pub fn to_json(&self) -> Value {
        let mut v = json!({
            "count": self.count(),
            "y": self.y.iter().map(|y| rail_across_js::number(f64::from(*y))).collect::<Vec<_>>(),
            "z": self.z.iter().map(|z| rail_across_js::number(*z)).collect::<Vec<_>>(),
            "i": self.i,
        });
        if let Some(rgb) = &self.rgb {
            v["rgb"] = json!(rgb);
        }
        v
    }
}

/// Add the points of one decoded segment that lie in the slice (`sliceSegment`).
pub fn slice_segment(out: &mut SlicePoints, seg: &Decoded, place: [f64; 6], f: &Frame, to_plane: Option<&Chain>) {
    let [ox, oy, oz, sx, sy, sz] = place;
    let color = seg.rgb.is_some() && out.rgb.is_some();
    for k in 0..seg.x.len() {
        let (mut pe, mut pn, mut pz) = (ox + f64::from(seg.x[k]) * sx, oy + f64::from(seg.y[k]) * sy, oz + f64::from(seg.z[k]) * sz);
        if let Some(t) = to_plane {
            let (e, n, z) = t.apply(pe, pn, pz);
            pe = e;
            pn = n;
            if t.has_height() {
                pz = z;
            }
        }
        let (de, dn) = (pe - f.e, pn - f.n);
        let d = de * f.along[0] + dn * f.along[1];
        if d > f.half || d < -f.half {
            continue;
        }
        let q = de * f.right[0] + dn * f.right[1];
        if q > f.half_width || q < -f.half_width {
            continue;
        }
        let c = if color {
            let rgb = seg.rgb.as_ref().unwrap();
            [rgb[0][k], rgb[1][k], rgb[2][k]]
        } else {
            [0; 3]
        };
        out.push(q, pz, seg.i[k], c);
    }
}
