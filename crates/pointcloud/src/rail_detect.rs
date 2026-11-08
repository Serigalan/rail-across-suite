// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The two rail heads of a track in one slice of a point cloud
//! (`railDetect.js`, decisions 128–131): found by their flangeways, each
//! head's inner flank measured 20–35 mm under its top, the axis the middle
//! between the head centres.

use rail_across_js::js_round;
use rail_across_track::cross_section::{TRACK_GAUGE, rail};
use serde_json::{Value, json};

const COLUMN: f64 = 0.01;
const BAND_ABOVE_GROUND: f64 = 0.8;
const FLANGEWAY: [f64; 2] = [0.015, 0.05];
const MIN_DROP: f64 = 0.03;
const HEAD_FLATNESS: f64 = 0.02;
const GAUGE_BAND: [f64; 2] = [0.020, 0.035];
const MIN_FLANK_POINTS: usize = 3;
const GAUGE_NARROW: f64 = 0.015;
const GAUGE_WIDE: f64 = 0.025;
const COLUMN_SLACK: f64 = 0.015;
const HEIGHT_SLACK: f64 = 0.01;
const MAX_CANT: f64 = 0.2;

fn quantile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let k = js_round(p * (sorted.len() - 1) as f64).max(0.0).min((sorted.len() - 1) as f64);
    sorted[k as usize]
}

fn sort(v: &mut [f64]) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
}

/// The highest point of every 1-cm column (`topSurface`).
pub struct Surface {
    pub from: f64,
    pub tops: Vec<f64>,
    pub ground: f64,
}

impl Surface {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        json!({ "from": n(self.from), "tops": self.tops.iter().map(|t| n(*t)).collect::<Vec<_>>(), "ground": n(self.ground) })
    }
}

pub fn top_surface(y: &[f32], z: &[f64], from: f64, to: f64) -> Surface {
    let mut zs: Vec<f64> = (0..y.len())
        .filter(|&k| f64::from(y[k]) >= from && f64::from(y[k]) <= to)
        .map(|k| z[k])
        .collect();
    sort(&mut zs);
    let ground = quantile(&zs, 0.05);
    let ceiling = ground + BAND_ABOVE_GROUND;
    let len = ((to - from) / COLUMN).ceil() + 1.0;
    let mut tops = vec![f64::NAN; if len.is_finite() && len > 0.0 { len as usize } else { 0 }];
    for k in 0..y.len() {
        let (yy, zz) = (f64::from(y[k]), z[k]);
        if yy < from || yy > to || zz > ceiling {
            continue;
        }
        let c = js_round((yy - from) / COLUMN);
        if c < 0.0 || c as usize >= tops.len() {
            continue;
        }
        let c = c as usize;
        if !(tops[c] >= zz) {
            tops[c] = zz;
        }
    }
    Surface { from, tops, ground }
}

fn columns(s: &Surface, a: f64, b: f64) -> Vec<f64> {
    let c0 = ((a - s.from) / COLUMN - 1e-9).ceil().max(0.0);
    let c1 = ((b - s.from) / COLUMN + 1e-9).floor().min(s.tops.len() as f64 - 1.0);
    let mut out = Vec::new();
    let mut c = c0;
    while c <= c1 {
        let t = s.tops[c as usize];
        if !t.is_nan() {
            out.push(t);
        }
        c += 1.0;
    }
    sort(&mut out);
    out
}

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub y: f64,
    pub z: f64,
    pub drop: f64,
}

/// Where a head's inner edge may be (`edgeCandidates`).
pub fn edge_candidates(s: &Surface, from: f64, to: f64, head_width: f64, side: f64) -> Vec<Edge> {
    let mut raw = Vec::new();
    for c in 0..s.tops.len().saturating_sub(1) {
        let y = s.from + (c as f64 + 0.5) * COLUMN;
        if y < from || y > to {
            continue;
        }
        let head = if side > 0.0 {
            columns(s, y - head_width + COLUMN, y)
        } else {
            columns(s, y, y + head_width - COLUMN)
        };
        if (head.len() as f64) < js_round(head_width / COLUMN) - 2.0 {
            continue;
        }
        let z = quantile(&head, 0.5);
        if z - head[0] > HEAD_FLATNESS {
            continue;
        }
        let flange = if side > 0.0 {
            columns(s, y + FLANGEWAY[0], y + FLANGEWAY[1])
        } else {
            columns(s, y - FLANGEWAY[1], y - FLANGEWAY[0])
        };
        if flange.len() < 2 {
            continue;
        }
        let drop = z - flange[flange.len() - 1];
        if drop >= MIN_DROP {
            raw.push(Edge { y, z, drop });
        }
    }
    raw.sort_by(|a, b| b.drop.partial_cmp(&a.drop).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<Edge> = Vec::new();
    for e in raw {
        if out.iter().all(|o| (o.y - e.y).abs() >= 0.05) {
            out.push(e);
        }
    }
    out.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn inner_flank(y: &[f32], z: &[f64], edge: f64, top: f64) -> Option<(f64, usize)> {
    let mut ys: Vec<f64> = (0..y.len())
        .filter(|&k| {
            let (yy, d) = (f64::from(y[k]), top - z[k]);
            (yy - edge).abs() <= 0.025 && d >= GAUGE_BAND[0] && d <= GAUGE_BAND[1]
        })
        .map(|k| f64::from(y[k]))
        .collect();
    if ys.len() < MIN_FLANK_POINTS {
        return None;
    }
    sort(&mut ys);
    Some((quantile(&ys, 0.5), ys.len()))
}

fn head_top(s: &Surface, y: f64, fallback: f64) -> f64 {
    let top = columns(s, y - 0.025, y + 0.025);
    if top.is_empty() { fallback } else { quantile(&top, 0.5) }
}

/// One head: centre, top, inner flank and how many flank points showed it.
#[derive(Clone, Copy, Debug)]
pub struct Head {
    pub y: f64,
    pub z: f64,
    pub edge: f64,
    pub flank_points: usize,
}

fn locate_head(y: &[f32], z: &[f64], s: &Surface, cand: &Edge, side: f64, head_width: f64) -> Head {
    let flank = inner_flank(y, z, cand.y, cand.z);
    let edge = flank.map(|f| f.0).unwrap_or(cand.y);
    let yy = edge - side * head_width / 2.0;
    Head {
        y: yy,
        z: head_top(s, yy, cand.z),
        edge,
        flank_points: flank.map(|f| f.1).unwrap_or(0),
    }
}

/// The track in a slice (`detectTrack`).
#[derive(Clone, Debug)]
pub enum Detection {
    Found {
        left: Head,
        right: Head,
        axis: f64,
        gauge: f64,
        cant: f64,
        ground: f64,
        good: bool,
    },
    Missing {
        reason: &'static str,
        edges: usize,
    },
}

impl Detection {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        let head = |h: &Head| json!({ "y": n(h.y), "z": n(h.z), "edge": n(h.edge), "flankPoints": h.flank_points });
        match self {
            Detection::Found {
                left,
                right,
                axis,
                gauge,
                cant,
                ground,
                good,
            } => json!({
                "left": head(left), "right": head(right), "axis": n(*axis), "gauge": n(*gauge), "cant": n(*cant),
                "ground": n(*ground), "quality": if *good { "good" } else { "doubtful" },
            }),
            Detection::Missing { reason, edges } => json!({ "reason": reason, "edges": edges }),
        }
    }
}

pub fn detect_track(y: &[f32], z: &[f64], around: f64, window: f64, rail_key: Option<&str>) -> Detection {
    let profile = rail(rail_key).or_else(|| rail(Some("54E4"))).unwrap();
    let head_width = profile.head / 1000.0;
    let min_height = profile.height_worn / 1000.0 - HEIGHT_SLACK;
    let gauge_nominal = TRACK_GAUGE / 1000.0;
    let reach = window + gauge_nominal / 2.0 + GAUGE_WIDE + head_width + FLANGEWAY[1] + 0.03;
    let s = top_surface(y, z, around - reach, around + reach);
    let lefts = edge_candidates(&s, around - reach, around + reach, head_width, 1.0);
    let rights = edge_candidates(&s, around - reach, around + reach, head_width, -1.0);
    let edges = lefts.len() + rights.len();
    if lefts.is_empty() || rights.is_empty() {
        return Detection::Missing { reason: "no_heads", edges };
    }
    let mut best: Option<(Edge, Edge, f64)> = None;
    for l in &lefts {
        for r in &rights {
            let dev = r.y - l.y - gauge_nominal;
            if dev < -GAUGE_NARROW - COLUMN_SLACK || dev > GAUGE_WIDE + COLUMN_SLACK {
                continue;
            }
            if (l.z - r.z).abs() > MAX_CANT {
                continue;
            }
            if l.z.min(r.z) - s.ground < min_height {
                continue;
            }
            let axis = (l.y + r.y) / 2.0;
            if (axis - around).abs() > window {
                continue;
            }
            let score = (axis - around).abs() + dev.abs();
            if best.is_none_or(|b| score < b.2) {
                best = Some((*l, *r, score));
            }
        }
    }
    let Some((bl, br, _)) = best else {
        return Detection::Missing { reason: "no_pair", edges };
    };
    let left = locate_head(y, z, &s, &bl, 1.0, head_width);
    let right = locate_head(y, z, &s, &br, -1.0, head_width);
    let gauge = right.edge - left.edge;
    let doubtful = left.flank_points == 0 || right.flank_points == 0 || gauge < gauge_nominal - GAUGE_NARROW || gauge > gauge_nominal + GAUGE_WIDE;
    Detection::Found {
        left,
        right,
        axis: (left.y + right.y) / 2.0,
        gauge,
        cant: left.z - right.z,
        ground: s.ground,
        good: !doubtful,
    }
}
