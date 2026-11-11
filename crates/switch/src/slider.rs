// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The position slider of the switch dialogs (`stationSlider.js`).

const EDGE_TOL: f64 = 1e-4;
const UNIQUE_TOL: f64 = 0.5e-4;
/// The grid the stretches' ends are rounded inward onto [m].
const GRID: f64 = 1e-3;

/// The stretches of [from, to] where `valid` holds (`validIntervals`).
pub fn valid_intervals(valid: &dyn Fn(f64) -> bool, from: f64, to: f64, step: f64, points: &[f64]) -> Vec<(f64, f64)> {
    if !(to >= from) {
        return vec![];
    }
    let sampled = sampled_intervals(valid, from, to, step);
    let inside = |p: f64| sampled.iter().any(|&(a, b)| p >= a - 1e-9 && p <= b + 1e-9);
    let mut out = sampled.clone();
    for p in points.iter().copied().filter(|&p| p >= from && p <= to && !inside(p) && valid(p)) {
        // A point just past a stretch's end, which the millimetre grid fell short of, ends it.
        match out.iter_mut().find(|(a, b)| (p > *b && p - *b <= GRID + 1e-9) || (p < *a && *a - p <= GRID + 1e-9)) {
            Some(iv) => *iv = (iv.0.min(p), iv.1.max(p)),
            None => out.push((p, p)),
        }
    }
    out.sort_by(|x, y| (x.0 - y.0).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn sampled_intervals(valid: &dyn Fn(f64) -> bool, from: f64, to: f64, step: f64) -> Vec<(f64, f64)> {
    let n = 1usize.max(((to - from) / step).ceil() as usize);
    let at = |i: usize| from + (to - from) * i as f64 / n as f64;
    let edge = |mut bad: f64, mut good: f64| {
        while (good - bad).abs() > EDGE_TOL {
            let m = (bad + good) / 2.0;
            if valid(m) {
                good = m;
            } else {
                bad = m;
            }
        }
        good
    };
    let inward = |x: f64, up: bool| {
        let r = if up {
            (x * 1000.0 - 1e-6).ceil() / 1000.0
        } else {
            (x * 1000.0 + 1e-6).floor() / 1000.0
        };
        if valid(r) { r } else { x }
    };
    let mut out = Vec::new();
    let (mut prev, mut prev_ok, mut start): (Option<f64>, bool, f64) = (None, false, 0.0);
    for i in 0..=n {
        let s = at(i);
        let ok = valid(s);
        if ok && !prev_ok {
            start = match prev {
                None => s,
                Some(p) => inward(edge(p, s), true),
            };
        }
        if !ok && prev_ok {
            out.push((start, inward(edge(s, prev.unwrap_or(s)), false)));
        }
        prev = Some(s);
        prev_ok = ok;
    }
    if prev_ok {
        out.push((start, to));
    }
    out
}

/// The nearest station inside one of the intervals (`nearestValid`).
pub fn nearest_valid(s: f64, intervals: &[(f64, f64)]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for &(a, b) in intervals {
        let c = b.min(a.max(s));
        if best.is_none_or(|x| (c - s).abs() < (x - s).abs()) {
            best = Some(c);
        }
    }
    best
}

/// Where a dragged station settles (`settleStation`).
pub fn settle_station(s: f64, intervals: &[(f64, f64)], snaps: &[f64], tol: f64) -> Option<f64> {
    let mut snap: Option<f64> = None;
    for &p in snaps {
        if (p - s).abs() <= tol && snap.is_none_or(|x| (p - s).abs() < (x - s).abs()) {
            snap = Some(p);
        }
    }
    let inside = |p: f64| intervals.iter().any(|&(a, b)| p >= a - 1e-9 && p <= b + 1e-9);
    if let Some(p) = snap.filter(|p| inside(*p)) {
        return Some(p);
    }
    nearest_valid(s, intervals)
}

/// Stations once each, ascending (`uniqueStations`): two within a tenth of a
/// millimetre are one, and the first of them is kept as it is — a node's own
/// station, not one rounded off it, so what settles on it leaves no sliver.
pub fn unique_stations(list: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for s in list.iter().copied().filter(|s| s.is_finite()) {
        if !out.iter().any(|o| (o - s).abs() < UNIQUE_TOL) {
            out.push(s);
        }
    }
    out.sort_by(|a, b| (a - b).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}
