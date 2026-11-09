// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The clearance check of a cloud slice and along a whole track
//! (`clearanceCheck.js`, `clearanceScan.js`, AP 11.5).

use rail_across_switch::sleepers::{SleeperOptions, catalogue_form};
use rail_across_track::cross_section::{cant_angle, cant_pivot, rotate_point, section_at_station, section_origin};
use serde_json::{Value, json};

use crate::section::{Cloud, cloud_section_points};
use crate::{Error, Result};

/// Height over the running plane below which points are not checked [mm].
pub const BOTTOM_BAND: f64 = 50.0;
const NEAR: f64 = 2000.0;
pub const INSIDE: u8 = 1;
pub const ALLOWED: u8 = 2;
const SCAN_STEP: f64 = 0.5;

pub type Ring = Vec<[f64; 2]>;

pub fn ring_of(v: &Value) -> Ring {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| [p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)])
                .collect()
        })
        .unwrap_or_default()
}

fn inside(ring: &Ring, y: f64, z: f64) -> bool {
    let mut hit = false;
    let n = ring.len();
    let mut b = n.wrapping_sub(1);
    for a in 0..n {
        let ([ya, za], [yb, zb]) = (ring[a], ring[b]);
        if (za > z) != (zb > z) && y < (yb - ya) * (z - za) / (zb - za) + ya {
            hit = !hit;
        }
        b = a;
    }
    hit
}

fn distance_to_ring(ring: &Ring, y: f64, z: f64) -> f64 {
    let mut best = f64::INFINITY;
    for a in 0..ring.len().saturating_sub(1) {
        let ([ya, za], [yb, zb]) = (ring[a], ring[a + 1]);
        if za <= 0.0 && zb <= 0.0 {
            continue;
        }
        let (dy, dz) = (yb - ya, zb - za);
        let len2 = dy * dy + dz * dz;
        let t = if len2 != 0.0 {
            (((y - ya) * dy + (z - za) * dz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        best = best.min(rail_across_js::js_hypot(&[y - (ya + t * dy), z - (za + t * dz)]));
    }
    best
}

/// What a check found: flags per point, how many are inside, the deepest and the nearest.
#[derive(Clone, Debug)]
pub struct Check {
    pub flags: Vec<u8>,
    pub inside: usize,
    pub deepest: Option<(f64, usize)>,
    pub nearest: Option<(f64, usize)>,
}

impl Check {
    pub fn to_json(&self) -> Value {
        let at = |p: Option<(f64, usize)>| p.map(|(d, k)| json!({ "distance": rail_across_js::number(d), "index": k }));
        json!({ "flags": self.flags, "inside": self.inside, "deepest": at(self.deepest), "nearest": at(self.nearest) })
    }
}

/// Check points (y [m] across, z [m] absolute) against the outline (`checkClearance`).
pub fn check_clearance(y: &[f32], z: &[f64], z_track: f64, cant: f64, ring: &Ring, areas: &[Ring]) -> Check {
    let count = y.len();
    let mut flags = vec![0u8; count];
    let angle = -cant_angle(cant);
    let [py, pz] = cant_pivot(cant);
    let (mut y_min, mut y_max, mut z_max) = (f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &[ry, rz] in ring {
        y_min = y_min.min(ry);
        y_max = y_max.max(ry);
        z_max = z_max.max(rz);
    }
    let mut n_inside = 0;
    let (mut deepest, mut nearest): (Option<(f64, usize)>, Option<(f64, usize)>) = (None, None);
    for k in 0..count {
        let [ry, rz] = rotate_point([f64::from(y[k]) * 1000.0 - py, (z[k] - z_track) * 1000.0 - pz], angle);
        let (yy, zz) = (ry + py, rz + pz);
        if zz < BOTTOM_BAND {
            continue;
        }
        if yy < y_min - NEAR || yy > y_max + NEAR || zz > z_max + NEAR {
            continue;
        }
        if inside(ring, yy, zz) {
            if areas.iter().any(|a| inside(a, yy, zz)) {
                flags[k] = ALLOWED;
                continue;
            }
            flags[k] = INSIDE;
            n_inside += 1;
            let depth = distance_to_ring(ring, yy, zz);
            if deepest.is_none_or(|d| depth > d.0) {
                deepest = Some((depth, k));
            }
        } else {
            let d = distance_to_ring(ring, yy, zz);
            if d <= NEAR && nearest.is_none_or(|n| d < n.0) {
                nearest = Some((d, k));
            }
        }
    }
    Check {
        flags,
        inside: n_inside,
        deepest,
        nearest,
    }
}

/// Stations with intrusions grouped into stretches (`stretchesOf`): (station, inside, depth).
pub fn stretches_of(hits: &[(f64, usize, f64)], step: f64) -> Vec<Value> {
    let mut out: Vec<(f64, f64, usize, f64, f64)> = Vec::new();
    for &(station, inside, depth) in hits {
        match out.last_mut() {
            Some(last) if station - last.1 <= step * 1.5 => {
                last.1 = station;
                last.2 += inside;
                if depth > last.3 {
                    last.3 = depth;
                    last.4 = station;
                }
            }
            _ => out.push((station, station, inside, depth, station)),
        }
    }
    let n = rail_across_js::number;
    out.into_iter()
        .map(|(from, to, inside, depth, at)| json!({ "from": n(from), "to": n(to), "inside": inside, "depth": n(depth), "deepestAt": n(at) }))
        .collect()
}

/// The clearance check along a track (`scanClearance`): { stretches, checked }.
pub fn scan_clearance(
    clouds: &[Cloud],
    track: &Value,
    ring: &Ring,
    areas: &[Ring],
    from: f64,
    to: Option<f64>,
    tracks: &[Value],
    switches: &[Value],
    step: Option<f64>,
    on_progress: &mut dyn FnMut(f64),
    aborted: &dyn Fn() -> bool,
) -> Result<Value> {
    let step = step.unwrap_or(SCAN_STEP);
    if !track.get("heights").and_then(Value::as_array).is_some_and(|h| !h.is_empty()) {
        return Ok(json!({ "noGradient": true, "stretches": [], "checked": 0 }));
    }
    let end = to.unwrap_or_else(|| rail_across_track::heights::track_length(track));
    let half_width = ring.iter().map(|[y, _]| y.abs()).fold(f64::NEG_INFINITY, f64::max) / 1000.0 + 0.5;
    let epsg = rail_across_js::js_number(track.get("epsg")) as i64;
    let opts = SleeperOptions {
        form_of: &catalogue_form,
        rail: None,
        lds_unknown: None,
    };
    let own = [track.clone()];
    let tracks: &[Value] = if tracks.is_empty() { &own } else { tracks };
    let mut hits = Vec::new();
    let mut checked = 0;
    let mut s = from + step / 2.0;
    while s < end {
        if aborted() {
            return Err(Error::Aborted);
        }
        let origin = section_origin(track, s);
        let z_track = rail_across_switch::gradient::track_height_at(tracks, switches, track, s, &opts);
        if let (Some((utm, bearing)), Some(z_track)) = (origin, z_track) {
            let cant = section_at_station(track, s).get("cant").and_then(Value::as_f64).unwrap_or(0.0);
            let (mut n_inside, mut depth) = (0, 0.0f64);
            for cloud in clouds {
                let points = cloud_section_points(cloud, (utm.e, utm.n), bearing, epsg, half_width, step)?;
                let r = check_clearance(&points.y, &points.z, z_track, cant, ring, areas);
                n_inside += r.inside;
                if let Some((d, _)) = r.deepest {
                    depth = depth.max(d);
                }
            }
            if n_inside > 0 {
                hits.push((s, n_inside, depth));
            }
            checked += 1;
        }
        on_progress(((s - from) / (end - from)).min(1.0));
        s += step;
    }
    Ok(json!({ "stretches": stretches_of(&hits, step), "checked": checked }))
}
