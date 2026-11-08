// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The position slider of the switch dialogs (`stationSlider.js`).

use rail_across_js::js_round;

const EDGE_TOL: f64 = 1e-4;

/// The stretches of [from, to] where `valid` holds (`validIntervals`).
pub fn valid_intervals(valid: &dyn Fn(f64) -> bool, from: f64, to: f64, step: f64, points: &[f64]) -> Vec<(f64, f64)> {
    if !(to >= from) {
        return vec![];
    }
    let sampled = sampled_intervals(valid, from, to, step);
    let inside = |p: f64| sampled.iter().any(|&(a, b)| p >= a - 1e-9 && p <= b + 1e-9);
    let mut out = sampled.clone();
    out.extend(
        points
            .iter()
            .copied()
            .filter(|&p| p >= from && p <= to && !inside(p) && valid(p))
            .map(|p| (p, p)),
    );
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

/// Stations to a tenth of a millimetre, once each, ascending (`uniqueStations`).
pub fn unique_stations(list: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for s in list.iter().copied().filter(|s| s.is_finite()).map(|s| js_round(s * 1e4) / 1e4) {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out.sort_by(|a, b| (a - b).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}
