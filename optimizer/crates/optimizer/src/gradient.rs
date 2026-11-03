// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A gradient from the heights of measured axis points — with the alignment
//! fit (AP 12.5; services/optimizer's gradient_fit.py, ported).
//!
//! The axis points of a trace carry the top of rail. Stationed along the
//! fitted chain, they give the vertical alignment of the new track in the
//! app's form: height points at the changes of gradient, each with the radius
//! of the vertical curve rounding it, the first at station 0 and the last at
//! the chain's length (heightUtils.js).
//!
//! 1. The heights are taken as the median in bins of BIN metres.
//! 2. The gradient diagram g(s): the slope of the best-fit line through the
//!    bins within GRADE_CHORD; its own slope over CURVATURE_CHORD is the
//!    curvature 1/R.
//! 3. Constant gradients are the runs of bins where that curvature stays
//!    under 1/R_FLAT (or a few times its scatter on a rough track), at least
//!    MIN_GRADE long, each with its best-fit line — split where a line leaves
//!    the bins by more than the tolerance, merged where two in a row have the
//!    same gradient.
//! 4. Where the points start or end inside a vertical curve, the first or
//!    last END_GRADE metres get a line of their own.
//! 5. Changes of gradient: two neighbouring lines meet at the height point;
//!    the tangent length of the curve between them is fitted to the bins
//!    (Brent, bounded), R = 2T/|Δg|, rounded. Where the bins between two runs
//!    stay off that curve, a short constant gradient hidden there is tried
//!    everywhere in between.
//! 6. Report: the height difference of every point from the gradient.

use crate::numerics::fminbound;
use serde_json::{Value as Json, json};

const BIN: f64 = 1.0;
const GRADE_CHORD: f64 = 15.0;
const CURVATURE_CHORD: f64 = 25.0;
const WINDOW_FILL: f64 = 0.6;
const R_FLAT: f64 = 80000.0;
const FLAT_NOISE: f64 = 4.0;
const MIN_GRADE: f64 = 20.0;
const SAME_GRADE: f64 = 0.0003;
const END_GRADE: f64 = 20.0;
const T_MIN: f64 = 1.0;
const SHORT_GRADE: f64 = 10.0;
const MAX_REFINE: usize = 20;
const MIN_POINTS: usize = 10;

/// Python's round(x, digits): half to even.
pub fn round_to(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round_ties_even() / f
}

/// numpy's median.
pub fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// numpy's searchsorted on ascending `a`: left — the first index with a[i] ≥ x; right — > x.
pub fn search_left(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v < x)
}

pub fn search_right(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v <= x)
}

/// numpy's interp: linear, held at the ends.
pub fn interp(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len();
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    let j = search_right(xs, x).max(1);
    let (x0, x1) = (xs[j - 1], xs[j]);
    if x1 == x0 {
        return ys[j];
    }
    ys[j - 1] + (ys[j] - ys[j - 1]) * (x - x0) / (x1 - x0)
}

/// Radii as a designer states them: whole hundreds from 2000 m, tens below.
fn round_radius(r: f64) -> f64 {
    let step: f64 = if r >= 2000.0 { 100.0 } else { 10.0 };
    step.max((r / step).round_ties_even() * step)
}

/// Median height per bin of BIN metres: (stations, heights), stations the bins' mean.
fn bins(s: &[f64], z: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut order: Vec<usize> = (0..s.len()).collect();
    let key = |i: usize| (s[i] / BIN).floor() as i64;
    order.sort_by_key(|&i| key(i));
    let (mut bs, mut bz) = (Vec::new(), Vec::new());
    let mut k = 0;
    while k < order.len() {
        let mut e = k;
        while e < order.len() && key(order[e]) == key(order[k]) {
            e += 1;
        }
        let group = &order[k..e];
        bs.push(group.iter().map(|&i| s[i]).sum::<f64>() / group.len() as f64);
        bz.push(median(&group.iter().map(|&i| z[i]).collect::<Vec<_>>()));
        k = e;
    }
    (bs, bz)
}

/// The slope of the best-fit line through the values within `half` of every x;
/// NaN where the window holds too few of them.
fn window_slope(x: &[f64], y: &[f64], half: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; x.len()];
    let (xs, ys): (Vec<f64>, Vec<f64>) = x.iter().zip(y).filter(|(_, y)| y.is_finite()).map(|(a, b)| (*a, *b)).unzip();
    if xs.len() < 3 {
        return out;
    }
    let cum = |f: &dyn Fn(usize) -> f64| {
        let mut c = vec![0.0];
        for i in 0..xs.len() {
            c.push(c[i] + f(i));
        }
        c
    };
    let (c, cy, cxx, cxy) = (cum(&|i| xs[i]), cum(&|i| ys[i]), cum(&|i| xs[i] * xs[i]), cum(&|i| xs[i] * ys[i]));
    let need = 3.max((WINDOW_FILL * 2.0 * half / BIN) as usize);
    for (k, &xk) in x.iter().enumerate() {
        let (lo, hi) = (search_left(&xs, xk - half), search_right(&xs, xk + half));
        let n = (hi - lo) as f64;
        let (sx, sy, sxx, sxy) = (c[hi] - c[lo], cy[hi] - cy[lo], cxx[hi] - cxx[lo], cxy[hi] - cxy[lo]);
        let den = n * sxx - sx * sx;
        if hi - lo >= need && den > 0.0 {
            out[k] = (n * sxy - sx * sy) / den;
        }
    }
    out
}

/// Best-fit line z = a + g·s: (a, g).
fn line(s: &[f64], z: &[f64]) -> (f64, f64) {
    let ptp = s.iter().copied().fold(f64::NEG_INFINITY, f64::max) - s.iter().copied().fold(f64::INFINITY, f64::min);
    let n = s.len() as f64;
    let mean_z = z.iter().sum::<f64>() / n;
    if s.len() < 2 || ptp == 0.0 {
        return (mean_z, 0.0);
    }
    let mean_s = s.iter().sum::<f64>() / n;
    let sxy: f64 = s.iter().zip(z).map(|(a, b)| (a - mean_s) * (b - mean_z)).sum();
    let sxx: f64 = s.iter().map(|a| (a - mean_s).powi(2)).sum();
    let g = sxy / sxx;
    (mean_z - g * mean_s, g)
}

/// The runs of true in `mask`, ends inclusive.
fn runs(mask: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (k, &m) in mask.iter().enumerate() {
        match (m, start) {
            (true, None) => start = Some(k),
            (false, Some(i)) => {
                out.push((i, k - 1));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(i) = start {
        out.push((i, mask.len() - 1));
    }
    out
}

fn max_off(bs: &[f64], bz: &[f64], a: f64, g: f64) -> f64 {
    bs.iter().zip(bz).map(|(s, z)| (z - (a + g * s)).abs()).fold(0.0, f64::max)
}

/// The run i…j as lines within `tol`, split at the worst bin where one does not do.
fn split(bs: &[f64], bz: &[f64], i: usize, j: usize, tol: f64, out: &mut Vec<(usize, usize)>) {
    let (a, g) = line(&bs[i..=j], &bz[i..=j]);
    let res: Vec<f64> = (i..=j).map(|k| (bz[k] - (a + g * bs[k])).abs()).collect();
    let k = (0..res.len()).fold(0, |b, q| if res[q] > res[b] { q } else { b });
    if res[k] <= tol || bs[j] - bs[i] < 2.0 * MIN_GRADE || k < 2 || k + 2 > j - i {
        out.push((i, j));
        return;
    }
    split(bs, bz, i, i + k, tol, out);
    split(bs, bz, i + k, j, tol, out);
}

/// The curvature under which a gradient counts as constant.
fn flat_limit(k: &[f64]) -> f64 {
    let w = ((CURVATURE_CHORD / BIN).round_ties_even() as usize).max(1);
    if k.len() <= w {
        return 1.0 / R_FLAT;
    }
    let d: Vec<f64> = (w..k.len()).map(|i| k[i] - k[i - w]).filter(|v| v.is_finite()).collect();
    if d.len() < 10 {
        return 1.0 / R_FLAT;
    }
    let sigma = 1.4826 * median(&d.iter().map(|v| v.abs()).collect::<Vec<_>>()) / 2f64.sqrt();
    (1.0 / R_FLAT).max(FLAT_NOISE * sigma)
}

/// The runs of constant gradient, as bin ranges in order.
fn stretches(bs: &[f64], bz: &[f64], tol: f64) -> Vec<(usize, usize)> {
    let g = window_slope(bs, bz, GRADE_CHORD / 2.0);
    let k = window_slope(bs, &g, CURVATURE_CHORD / 2.0);
    let limit = flat_limit(&k);
    let flat: Vec<bool> = k.iter().map(|v| v.is_finite() && v.abs() < limit).collect();
    let mut split_runs = Vec::new();
    for (i, j) in runs(&flat).into_iter().filter(|&(i, j)| bs[j] - bs[i] >= MIN_GRADE) {
        split(bs, bz, i, j, tol, &mut split_runs);
    }
    // Neighbours of the same gradient whose common line holds are one.
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for r in split_runs {
        if let Some(&p) = merged.last() {
            let (_, g1) = line(&bs[p.0..=p.1], &bz[p.0..=p.1]);
            let (_, g2) = line(&bs[r.0..=r.1], &bz[r.0..=r.1]);
            let (a, gg) = line(&bs[p.0..=r.1], &bz[p.0..=r.1]);
            if (g1 - g2).abs() < SAME_GRADE && max_off(&bs[p.0..=r.1], &bz[p.0..=r.1], a, gg) <= tol {
                merged.last_mut().unwrap().1 = r.1;
                continue;
            }
        }
        merged.push(r);
    }
    if merged.is_empty() {
        return if bs.len() >= 2 { vec![(0, bs.len() - 1)] } else { Vec::new() };
    }
    let (first, last) = (merged[0], *merged.last().unwrap());
    let (a, gg) = line(&bs[first.0..=first.1], &bz[first.0..=first.1]);
    if first.0 > 0 && max_off(&bs[..first.0], &bz[..first.0], a, gg) > tol {
        let end = search_left(bs, bs[0] + END_GRADE) as i64 - 1;
        if 0 < end && (end as usize) < first.0 {
            merged.insert(0, (0, end as usize));
        }
    }
    let (a, gg) = line(&bs[last.0..=last.1], &bz[last.0..=last.1]);
    let n = bs.len();
    if last.1 < n - 1 && max_off(&bs[last.1 + 1..], &bz[last.1 + 1..], a, gg) > tol {
        let start = search_left(bs, bs[n - 1] - END_GRADE);
        if last.1 < start && start < n - 1 {
            merged.push((start, n - 1));
        }
    }
    merged
}

/// Heights of two gradients meeting at (pvi, z0), rounded over ±t.
fn curve_z(s: f64, pvi: f64, z0: f64, g1: f64, g2: f64, t: f64) -> f64 {
    let x = s - pvi;
    if t > 0.0 && x.abs() < t {
        return z0 + g1 * x + (x + t).powi(2) * (g2 - g1) / (4.0 * t);
    }
    z0 + if x < 0.0 { g1 * x } else { g2 * x }
}

/// A height point of the gradient: its station, height and the radius of its vertical curve.
#[derive(Debug, Clone)]
pub struct Height {
    pub station: f64,
    pub z: f64,
    pub rv: Option<f64>,
}

/// Heights of the gradient at stations `s` — the polygon, and inside a
/// vertical curve its parabola (heightUtils.gradientAt).
fn gradient_at(heights: &[Height], s: &[f64]) -> Vec<f64> {
    let st: Vec<f64> = heights.iter().map(|h| h.station).collect();
    let zz: Vec<f64> = heights.iter().map(|h| h.z).collect();
    let mut out: Vec<f64> = s.iter().map(|&x| interp(x, &st, &zz)).collect();
    for i in 1..heights.len().saturating_sub(1) {
        let Some(rv) = heights[i].rv.filter(|r| *r != 0.0) else { continue };
        let g1 = (zz[i] - zz[i - 1]) / (st[i] - st[i - 1]);
        let g2 = (zz[i + 1] - zz[i]) / (st[i + 1] - st[i]);
        let t = rv.abs() * (g2 - g1).abs() / 2.0;
        for (o, &x0) in out.iter_mut().zip(s) {
            let x = x0 - st[i];
            if x.abs() < t {
                *o = zz[i] + g1 * x + (x + t).powi(2) * (g2 - g1) / (4.0 * t);
            }
        }
    }
    out
}

/// The gradient of the runs: their lines meeting at the height points, a
/// vertical curve fitted at each. Unrounded.
fn build(bs: &[f64], bz: &[f64], runs: &[(usize, usize)], length: f64) -> Vec<Height> {
    let lines: Vec<(f64, f64)> = runs.iter().map(|&(i, j)| line(&bs[i..=j], &bz[i..=j])).collect();
    struct Pvi {
        station: f64,
        z: f64,
        room: (f64, f64),
    }
    let mut pts = Vec::new();
    for k in 1..runs.len() {
        let ((a1, g1), (a2, g2)) = (lines[k - 1], lines[k]);
        let (lo, hi) = (bs[runs[k - 1].1], bs[runs[k].0]);
        let mut pvi = if (g2 - g1).abs() > 1e-9 {
            let p = (a1 - a2) / (g2 - g1);
            if lo - MIN_GRADE <= p && p <= hi + MIN_GRADE { p } else { (lo + hi) / 2.0 }
        } else {
            (lo + hi) / 2.0
        };
        pvi = pvi.max(bs[runs[k - 1].0] + BIN).min(bs[runs[k].1] - BIN);
        pts.push(Pvi {
            station: pvi,
            z: a1 + g1 * pvi,
            room: (pvi - bs[runs[k - 1].0], bs[runs[k].1] - pvi),
        });
    }
    let mut zs = vec![lines[0].0];
    zs.extend(pts.iter().map(|p| p.z));
    zs.push(lines.last().unwrap().0 + lines.last().unwrap().1 * length);
    let mut ss = vec![0.0];
    ss.extend(pts.iter().map(|p| p.station));
    ss.push(length);
    let mut heights = vec![Height {
        station: 0.0,
        z: zs[0],
        rv: None,
    }];
    for (k, p) in pts.iter().enumerate().map(|(i, p)| (i + 1, p)) {
        let g1 = (zs[k] - zs[k - 1]) / (ss[k] - ss[k - 1]).max(1e-6);
        let g2 = (zs[k + 1] - zs[k]) / (ss[k + 1] - ss[k]).max(1e-6);
        let mut h = Height {
            station: ss[k],
            z: zs[k],
            rv: None,
        };
        let dg = (g2 - g1).abs();
        let tmax = p.room.0.min(p.room.1).min(ss[k] - ss[k - 1]).min(ss[k + 1] - ss[k]);
        if dg > 1e-6 && tmax > T_MIN {
            let (x, y): (Vec<f64>, Vec<f64>) = bs
                .iter()
                .zip(bz)
                .filter(|(b, _)| **b > ss[k] - tmax && **b < ss[k] + tmax)
                .map(|(a, b)| (*a, *b))
                .unzip();
            if x.len() >= 3 {
                let cost = |t: f64| {
                    x.iter()
                        .zip(&y)
                        .map(|(xv, yv)| (yv - curve_z(*xv, ss[k], zs[k], g1, g2, t)).powi(2))
                        .sum::<f64>()
                };
                let t = fminbound(cost, 0.0, tmax, 0.05, 500);
                if t >= T_MIN {
                    h.rv = Some(round_radius(2.0 * t / dg));
                }
            }
        }
        heights.push(h);
    }
    heights.push(Height {
        station: length,
        z: *zs.last().unwrap(),
        rv: None,
    });
    // A radius rounded up may reach into its neighbour's curve: then it is cut
    // back to where they touch.
    let mut t_prev = 0.0;
    for k in 1..heights.len() - 1 {
        let Some(mut rv) = heights[k].rv.filter(|r| *r != 0.0) else {
            t_prev = 0.0;
            continue;
        };
        let (a, b, c) = (&heights[k - 1], &heights[k], &heights[k + 1]);
        let g1 = (b.z - a.z) / (b.station - a.station);
        let g2 = (c.z - b.z) / (c.station - b.station);
        let dg = (g2 - g1).abs();
        let tmax = (b.station - a.station - t_prev).min(c.station - b.station);
        if rv * dg / 2.0 > tmax {
            rv = (2.0 * tmax / dg / 10.0).floor() * 10.0;
            if rv * dg / 2.0 < T_MIN {
                heights[k].rv = None;
                t_prev = 0.0;
                continue;
            }
            heights[k].rv = Some(rv);
        }
        t_prev = rv * dg / 2.0;
    }
    heights
}

fn worst_in(res: &[f64], lo: usize, hi: usize) -> f64 {
    res[lo..hi].iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Where the points between two runs stay off the curve joining them, a short
/// constant gradient between two curves was too short for the diagram to see:
/// a run of SHORT_GRADE is tried at every place in between, and the best one
/// kept if it takes the largest difference down by half.
fn refine(bs: &[f64], bz: &[f64], mut runs: Vec<(usize, usize)>, length: f64, tol: f64) -> Vec<Height> {
    for _ in 0..MAX_REFINE {
        let heights = build(bs, bz, &runs, length);
        let at = gradient_at(&heights, bs);
        let res: Vec<f64> = bz.iter().zip(&at).map(|(z, g)| (z - g).abs()).collect();
        let mut worst: Option<(usize, f64)> = None;
        for k in 1..runs.len() {
            let (lo, hi) = (runs[k - 1].1 + 1, runs[k].0);
            if hi < lo + 3 || worst_in(&res, lo, hi) <= tol {
                continue;
            }
            let w = worst_in(&res, lo, hi);
            if worst.is_none_or(|(_, b)| w > b) {
                worst = Some((k, w));
            }
        }
        let Some((k, before)) = worst else { return heights };
        let (lo, hi) = (runs[k - 1].1 + 1, runs[k].0 - 1);
        let mut best: Option<(f64, Vec<(usize, usize)>)> = None;
        let mut c = bs[lo] + SHORT_GRADE / 2.0;
        while c < bs[hi] - SHORT_GRADE / 2.0 + 1e-9 {
            let i = search_left(bs, c - SHORT_GRADE / 2.0);
            let j = search_left(bs, c + SHORT_GRADE / 2.0) as i64 - 1;
            c += BIN * 2.0;
            if j < 0 || (j as usize) < i + 3 || i <= runs[k - 1].1 || j as usize >= runs[k].0 {
                continue;
            }
            let mut trial = runs[..k].to_vec();
            trial.push((i, j as usize));
            trial.extend_from_slice(&runs[k..]);
            let span = runs[k - 1].0..runs[k].1 + 1;
            let at = gradient_at(&build(bs, bz, &trial, length), &bs[span.clone()]);
            let err = bz[span].iter().zip(&at).map(|(z, g)| (z - g).abs()).fold(0.0, f64::max);
            if best.as_ref().is_none_or(|(b, _)| err < *b) {
                best = Some((err, trial));
            }
        }
        match best {
            Some((err, trial)) if err <= before / 2.0 => runs = trial,
            _ => return heights,
        }
    }
    build(bs, bz, &runs, length)
}

/// The gradient of points at stations `s` [m along the chain] with heights
/// `z` [m] for a track `length` long: {heights, offsets, rms, max} — offsets
/// per point in the order given, null where it has no height — or null where
/// there are too few heights to say.
pub fn fit_gradient(s: &[f64], z: &[f64], length: f64, tolerance: f64) -> Json {
    let have: Vec<usize> = (0..s.len()).filter(|&i| s[i].is_finite() && z[i].is_finite()).collect();
    let hs: Vec<f64> = have.iter().map(|&i| s[i]).collect();
    let hz: Vec<f64> = have.iter().map(|&i| z[i]).collect();
    let ptp = hs.iter().copied().fold(f64::NEG_INFINITY, f64::max) - hs.iter().copied().fold(f64::INFINITY, f64::min);
    if have.len() < MIN_POINTS || ptp < MIN_GRADE {
        return Json::Null;
    }
    let (bs, bz) = bins(&hs, &hz);
    let runs = stretches(&bs, &bz, tolerance);
    if runs.is_empty() {
        return Json::Null;
    }
    let mut heights = refine(&bs, &bz, runs, length, tolerance);
    for h in &mut heights {
        h.station = round_to(h.station, 3);
        h.z = round_to(h.z, 4);
    }
    let at = gradient_at(&heights, &hs);
    let mut off = vec![f64::NAN; s.len()];
    for (k, &i) in have.iter().enumerate() {
        off[i] = z[i] - at[k];
    }
    let v: Vec<f64> = have.iter().map(|&i| off[i]).collect();
    json!({
        "heights": heights.iter().map(|h| {
            let mut o = json!({"station": h.station, "z": h.z});
            if let Some(rv) = h.rv { o["rv"] = json!(rv); }
            o
        }).collect::<Vec<_>>(),
        "offsets": off.iter().map(|d| if d.is_finite() { json!(round_to(*d, 4)) } else { Json::Null }).collect::<Vec<_>>(),
        "rms": round_to((v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt(), 4),
        "max": round_to(v.iter().map(|x| x.abs()).fold(0.0, f64::max), 4),
    })
}
