// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An alignment from measured axis points — phase 12, stage B (AP 12.5;
//! the former Python service's alignment_fit.py, ported).
//!
//! The points come in travel order: easting and northing in one plane, and
//! the height of the top of rail and the cant where they are known. What goes
//! back is a chain of straights, arcs and transitions in the app's element
//! form, and what the user needs to judge and correct it:
//!
//! 1. The curvature diagram κ(s) from mid-ordinates on a chord (κ = 8f/c²),
//!    smoothed. Signed like the radius: > 0 for a right-hand curve.
//! 2. Straights, by the client's procedure: a window from the start; where
//!    every point in it lies within the tolerance of its best-fit line it is a
//!    candidate, grown while the deviation holds; it counts where the circle
//!    through it has a small mid-ordinate and it is long enough. A straight
//!    can also be given by hand.
//! 3. Curves between two straights: the splice construction, its R, L1 and L2
//!    adjusted to the points by least squares (Nelder–Mead). A transition the
//!    points do not show is left out.
//! 4. The ends, where the points start or stop inside a curve.
//! 5. Report: the offset of every point from the chain, per element the
//!    largest and the RMS.
//! 6. The gradient, where the points carry heights ([`crate::gradient`]).
//! 7. The cant, where the points carry it: per straight and arc the median of
//!    the points on it, away from its ends, on the step cant is designed in.

use crate::geometry::{CurveFit, Profile, Pt, arc_forward, dir_of, fit_curve_group, transition_end};
use crate::gradient::{fit_gradient, interp, median, round_to, search_left, search_right};
use crate::numerics::{NelderMead, nelder_mead};
use crate::poly::{Shape, piece_points, project};
use crate::splice::{Role, arc as arc_element, straight as straight_element, transition as transition_element};
use serde_json::{Map, Value as Json, json};

const DEFAULTS: [(&str, f64, f64, f64); 8] = [
    ("window", 6.0, 1.0, 100.0),
    ("tolerance", 0.01, 0.001, 0.2),
    ("step", 0.5, 0.1, 10.0),
    ("spacing", 6.0, 0.0, 100.0),
    ("sagitta", 0.003, 0.0005, 0.1),
    ("minLength", 20.0, 2.0, 1000.0),
    ("chord", 10.0, 2.0, 50.0),
    ("heightTolerance", 0.02, 0.001, 0.5),
];
const MAX_POINTS: usize = 40000;
const GAP_FACTOR: f64 = 5.0;
const GAP_MIN: f64 = 2.0;
const KAPPA_SMOOTH: f64 = 5.0;
const CURVE_MARGIN: f64 = 30.0;
const TRANSITION_EVIDENCE: f64 = 0.0005;
const TRANSITION_TEST_SHIFT: f64 = 0.02;
const EDGE_EXIT_GAIN: f64 = 0.7;
const EDGE_EXIT_NOISE: f64 = 1.5;
const R_MIN: f64 = 20.0;
const FATOL: f64 = 1e-8;
const STRAIGHT_STEP: f64 = 5.0;
const CANT_STEP: f64 = 5.0;
const CANT_TRIM: f64 = 5.0;
const CANT_END: f64 = 5.0;
const CANT_MIN_POINTS: usize = 3;

/// The settings of a fit, each held to its limits.
#[derive(Debug, Clone)]
struct Settings {
    window: f64,
    tolerance: f64,
    step: f64,
    spacing: f64,
    sagitta: f64,
    min_length: f64,
    chord: f64,
    height_tolerance: f64,
}

impl Settings {
    fn from_json(raw: &Json) -> (Settings, Json) {
        let mut out = Map::new();
        for (key, default, lo, hi) in DEFAULTS {
            let v = raw.get(key).and_then(Json::as_f64).map_or(default, |v| v.clamp(lo, hi));
            out.insert(key.into(), json!(v));
        }
        let get = |k: &str| out[k].as_f64().unwrap();
        let s = Settings {
            window: get("window"),
            tolerance: get("tolerance"),
            step: get("step"),
            spacing: get("spacing"),
            sagitta: get("sagitta"),
            min_length: get("minLength"),
            chord: get("chord"),
            height_tolerance: get("heightTolerance"),
        };
        (s, Json::Object(out))
    }
}

fn bearing_of(d: Pt) -> f64 {
    d.0.atan2(d.1).to_degrees().rem_euclid(360.0)
}

fn sub(a: Pt, b: Pt) -> Pt {
    (a.0 - b.0, a.1 - b.1)
}

fn dot(a: Pt, b: Pt) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn add_scaled(a: Pt, t: f64, d: Pt) -> Pt {
    (a.0 + t * d.0, a.1 + t * d.1)
}

// ── the points ───────────────────────────────────────────────────────────────

fn stations_of(pts: &[Pt], station0: f64) -> (Vec<f64>, Vec<f64>) {
    let seg: Vec<f64> = pts.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).collect();
    let mut s = vec![station0];
    for d in &seg {
        s.push(s.last().unwrap() + d);
    }
    (s, seg)
}

/// Indices i where the axis has a gap between point i and i + 1.
fn find_gaps(seg: &[f64]) -> Vec<usize> {
    if seg.is_empty() {
        return Vec::new();
    }
    let limit = GAP_MIN.max(GAP_FACTOR * median(seg));
    (0..seg.len()).filter(|&i| seg[i] > limit).collect()
}

/// κ at every point from the mid-ordinate on a chord centred on it; NaN where
/// the chord leaves the points or spans a gap. Then a running mean.
fn curvature(pts: &[Pt], s: &[f64], gaps: &[usize], chord: f64) -> Vec<f64> {
    let half = chord / 2.0;
    let (xs, ys): (Vec<f64>, Vec<f64>) = pts.iter().copied().unzip();
    let n = s.len();
    let mut k = vec![f64::NAN; n];
    for i in 0..n {
        let (lo, hi) = (s[i] - half, s[i] + half);
        if lo < s[0] || hi > s[n - 1] || gaps.iter().any(|&g| lo < s[g + 1] && hi > s[g]) {
            continue;
        }
        let (ax, ay) = (interp(lo, s, &xs), interp(lo, s, &ys));
        let (bx, by) = (interp(hi, s, &xs), interp(hi, s, &ys));
        let (cx, cy) = (bx - ax, by - ay);
        let c2 = cx * cx + cy * cy;
        // Left of the chord is the outside of a right-hand curve: κ > 0.
        let f = (cx * (pts[i].1 - ay) - cy * (pts[i].0 - ax)) / c2.sqrt();
        k[i] = 8.0 * f / c2;
    }
    smooth(&k, s, KAPPA_SMOOTH)
}

/// Running mean over `length` metres, NaNs left out of it.
fn smooth(values: &[f64], s: &[f64], length: f64) -> Vec<f64> {
    let mut cs = vec![0.0];
    let mut cn = vec![0usize];
    for v in values {
        cs.push(cs.last().unwrap() + if v.is_finite() { *v } else { 0.0 });
        cn.push(cn.last().unwrap() + usize::from(v.is_finite()));
    }
    (0..values.len())
        .map(|i| {
            if !values[i].is_finite() {
                return f64::NAN;
            }
            let (lo, hi) = (search_left(s, s[i] - length / 2.0), search_right(s, s[i] + length / 2.0));
            let cnt = cn[hi] - cn[lo];
            if cnt > 0 { (cs[hi] - cs[lo]) / cnt as f64 } else { f64::NAN }
        })
        .collect()
}

// ── straights ────────────────────────────────────────────────────────────────

/// A line: a point on it and its unit direction.
type Line = (Pt, Pt);

/// Best-fit line (total least squares) through `p`: centre, unit direction
/// along the points' order, and the largest perpendicular deviation.
fn fit_line(p: &[Pt]) -> (Pt, Pt, f64) {
    let n = p.len() as f64;
    let c = (p.iter().map(|q| q.0).sum::<f64>() / n, p.iter().map(|q| q.1).sum::<f64>() / n);
    let (mut a, mut b, mut d) = (0.0, 0.0, 0.0);
    for q in p {
        let (x, y) = sub(*q, c);
        a += x * x;
        b += x * y;
        d += y * y;
    }
    // The eigenvector of the larger eigenvalue of [[a, b], [b, d]].
    let lambda = (a + d) / 2.0 + (((a - d) / 2.0).powi(2) + b * b).sqrt();
    let mut v = if b.abs() > 1e-300 {
        (lambda - d, b)
    } else if a >= d {
        (1.0, 0.0)
    } else {
        (0.0, 1.0)
    };
    let len = v.0.hypot(v.1);
    v = (v.0 / len, v.1 / len);
    if dot(v, sub(p[p.len() - 1], p[0])) < 0.0 {
        v = (-v.0, -v.1);
    }
    let dev = p.iter().map(|q| {
        let r = sub(*q, c);
        (r.0 * v.1 - r.1 * v.0).abs()
    });
    (c, v, dev.fold(0.0, f64::max))
}

/// Mid-ordinate over its length of the parabola (≈ circle) through `p`, in the frame of its line.
fn line_sagitta(p: &[Pt], c: Pt, d: Pt) -> f64 {
    let xy: Vec<(f64, f64)> = p
        .iter()
        .map(|q| {
            let r = sub(*q, c);
            (dot(r, d), r.0 * d.1 - r.1 * d.0)
        })
        .collect();
    let (xmin, xmax) = xy.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (x, _)| (lo.min(*x), hi.max(*x)));
    let span = xmax - xmin;
    if p.len() < 4 || span <= 0.0 {
        return 0.0;
    }
    // y = a x² + b x + c by least squares, x centred (a does not change).
    let mx = xy.iter().map(|(x, _)| x).sum::<f64>() / xy.len() as f64;
    let mut m = [[0.0f64; 4]; 3];
    for &(x, y) in &xy {
        let x = x - mx;
        let row = [x * x, x, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += row[i] * row[j];
            }
            m[i][3] += row[i] * y;
        }
    }
    // Gaussian elimination with partial pivoting.
    for col in 0..3 {
        let piv = (col..3).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs())).unwrap();
        m.swap(col, piv);
        if m[col][col].abs() < 1e-300 {
            return 0.0;
        }
        let pivot = m[col];
        for row in m.iter_mut().skip(col + 1) {
            let f = row[col] / pivot[col];
            for (v, p) in row.iter_mut().zip(pivot.iter()).skip(col) {
                *v -= f * p;
            }
        }
    }
    let mut sol = [0.0; 3];
    for i in (0..3).rev() {
        let s: f64 = (i + 1..3).map(|k| m[i][k] * sol[k]).sum();
        sol[i] = (m[i][3] - s) / m[i][i];
    }
    sol[0].abs() * span * span / 4.0
}

fn spans_gap(i: usize, j: usize, gaps: &[usize]) -> bool {
    gaps.iter().any(|&g| i <= g && g < j)
}

/// Best-fit lines over ranges of the points, from running sums.
struct Lines<'a> {
    pts: &'a [Pt],
    sums: [Vec<f64>; 5],
}

impl<'a> Lines<'a> {
    fn new(pts: &'a [Pt]) -> Lines<'a> {
        let cum = |f: &dyn Fn(Pt) -> f64| {
            let mut c = vec![0.0];
            for p in pts {
                c.push(c.last().unwrap() + f(*p));
            }
            c
        };
        Lines {
            pts,
            sums: [cum(&|p| p.0), cum(&|p| p.1), cum(&|p| p.0 * p.0), cum(&|p| p.0 * p.1), cum(&|p| p.1 * p.1)],
        }
    }

    /// Largest distance of points i…j from their best-fit line.
    fn deviation(&self, i: usize, j: usize) -> f64 {
        let n = (j - i + 1) as f64;
        let d = |k: usize| self.sums[k][j + 1] - self.sums[k][i];
        let (mx, my) = (d(0) / n, d(1) / n);
        let (cxx, cxy, cyy) = (d(2) / n - mx * mx, d(3) / n - mx * my, d(4) / n - my * my);
        let theta = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        let (st, ct) = (theta.sin(), theta.cos());
        self.pts[i..=j].iter().map(|q| ((q.0 - mx) * st - (q.1 - my) * ct).abs()).fold(0.0, f64::max)
    }
}

/// The client's procedure (step 2): point ranges of straights.
fn find_straights(pts: &[Pt], s: &[f64], gaps: &[usize], o: &Settings) -> Vec<(usize, usize)> {
    let n = s.len();
    let lines = Lines::new(pts);
    let mut found = Vec::new();
    let mut i = 0;
    while i + 2 < n {
        let mut j = search_left(s, s[i] + o.window);
        if j >= n {
            break;
        }
        if spans_gap(i, j, gaps) {
            i = gaps.iter().find(|&&g| i <= g && g < j).unwrap() + 1;
            continue;
        }
        if lines.deviation(i, j) > o.tolerance {
            i = search_left(s, s[i] + o.step);
            continue;
        }
        // Grow it while the deviation holds.
        loop {
            let k = search_left(s, s[j] + o.step);
            if k >= n || spans_gap(j, k, gaps) || lines.deviation(i, k) > o.tolerance {
                break;
            }
            j = k;
        }
        let (c, d, _) = fit_line(&pts[i..=j]);
        if s[j] - s[i] >= o.min_length && line_sagitta(&pts[i..=j], c, d) < o.sagitta {
            found.push((i, j));
            i = search_left(s, s[j] + o.spacing);
        } else {
            i = search_left(s, s[i] + o.step);
        }
    }
    found
}

/// Straights set by hand, as station ranges: point ranges, in order, each
/// with at least three points, overlaps merged.
fn given_straights(s: &[f64], ranges: &[(f64, f64)]) -> Vec<(usize, usize)> {
    let mut sorted: Vec<(f64, f64)> = ranges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (frm, to) in sorted {
        let i = search_left(s, frm);
        let j = search_right(s, to) as i64 - 1;
        if j - (i as i64) < 2 {
            continue;
        }
        let j = j as usize;
        match out.last_mut() {
            Some(last) if i <= last.1 => last.1 = last.1.max(j),
            _ => out.push((i, j)),
        }
    }
    out
}

// ── chains ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Straight,
    Arc(f64),
    Transition(Option<f64>, Option<f64>),
}

/// One element in the making.
#[derive(Debug, Clone, Copy)]
struct Piece {
    kind: Kind,
    start: Pt,
    bearing: f64,
    length: f64,
}

fn piece_end(pc: &Piece) -> (Pt, f64) {
    match pc.kind {
        Kind::Straight => (add_scaled(pc.start, pc.length, dir_of(pc.bearing)), pc.bearing),
        Kind::Arc(r) => arc_forward(pc.start.0, pc.start.1, pc.bearing, r, pc.length / r.abs()),
        Kind::Transition(r1, r2) => {
            let (e, n, eb) = transition_end(pc.start.0, pc.start.1, pc.bearing, pc.length, r1, r2, Profile::Clothoid);
            ((e, n), eb)
        }
    }
}

fn points_of(pc: &Piece) -> Vec<Pt> {
    let shape = match pc.kind {
        Kind::Straight => Shape::Straight,
        Kind::Arc(r) => Shape::Arc(r),
        Kind::Transition(r1, r2) => Shape::Transition(r1, r2, Profile::Clothoid),
    };
    piece_points(pc.start, pc.bearing, pc.length, shape)
}

/// Pieces one after the other from `start`.
fn chain_from(start: Pt, bearing: f64, specs: &[(Kind, f64)]) -> Vec<Piece> {
    let mut out = Vec::new();
    let (mut p, mut b) = (start, bearing);
    for &(kind, length) in specs {
        if length <= 1e-9 {
            continue;
        }
        let pc = Piece {
            kind,
            start: p,
            bearing: b,
            length,
        };
        out.push(pc);
        (p, b) = piece_end(&pc);
    }
    out
}

/// The chain as a polyline, and for every vertex the piece it belongs to.
fn chain_points(pieces: &[Piece]) -> (Vec<Pt>, Vec<usize>) {
    let (mut pts, mut owner) = (Vec::new(), Vec::new());
    for (k, pc) in pieces.iter().enumerate() {
        let q = points_of(pc);
        let skip = usize::from(!pts.is_empty());
        for p in q.into_iter().skip(skip) {
            pts.push(p);
            owner.push(k);
        }
    }
    (pts, owner)
}

/// The line from a to b with a vertex every STRAIGHT_STEP.
fn segment(a: Pt, b: Pt) -> Vec<Pt> {
    let k = (((b.0 - a.0).hypot(b.1 - a.1) / STRAIGHT_STEP).ceil() as usize).max(1);
    (0..=k).map(|i| add_scaled(a, i as f64 / k as f64, sub(b, a))).collect()
}

fn sse(points: &[Pt], poly: &[Pt]) -> f64 {
    if points.is_empty() || poly.len() < 2 {
        return 0.0;
    }
    project(points, poly, None).0.iter().map(|o| o * o).sum()
}

fn cumulative(poly: &[Pt]) -> Vec<f64> {
    let mut along = vec![0.0];
    for w in poly.windows(2) {
        along.push(along.last().unwrap() + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
    }
    along
}

/// How far along the chain `point` projects [m].
fn chain_station(pieces: &[Piece], point: Pt) -> f64 {
    let (poly, _) = chain_points(pieces);
    let along = cumulative(&poly);
    let (_, seg, t) = project(&[point], &poly, None);
    along[seg[0]] + t[0] * (along[seg[0] + 1] - along[seg[0]])
}

/// The chain up to where `point` projects onto it.
fn cut_chain(pieces: &[Piece], point: Pt) -> Vec<Piece> {
    let at = chain_station(pieces, point);
    let mut out = Vec::new();
    let mut start = 0.0;
    for pc in pieces {
        if start + pc.length <= at + 1e-9 {
            out.push(*pc);
            start += pc.length;
            continue;
        }
        let into = at - start;
        if into > 1e-6 {
            let mut cut = *pc;
            if let Kind::Transition(r1, r2) = pc.kind {
                // A clothoid cut short is a clothoid to the curvature it has there.
                let k1 = r1.filter(|r| *r != 0.0).map_or(0.0, |r| 1.0 / r);
                let k2 = r2.filter(|r| *r != 0.0).map_or(0.0, |r| 1.0 / r);
                let kc = k1 + (k2 - k1) * into / pc.length;
                cut.kind = Kind::Transition(r1, if kc.abs() > 1e-12 { Some(1.0 / kc) } else { None });
            }
            cut.length = into;
            out.push(cut);
        }
        break;
    }
    out
}

/// The app's element for a piece (the splice's form, without a role).
fn piece_element(pc: &Piece) -> Json {
    let (end, end_b) = piece_end(pc);
    let el = match pc.kind {
        Kind::Straight => Some(straight_element(pc.start, end, Role::New)),
        Kind::Arc(r) => arc_element(pc.start, end, r, Role::New).ok().map(|mut el| {
            el.length = pc.length;
            el
        }),
        Kind::Transition(r1, r2) => Some(transition_element(
            pc.start,
            pc.bearing,
            pc.length,
            r1,
            r2,
            Profile::Clothoid,
            Some(end),
            Some(end_b),
        )),
    };
    let mut json = el.map_or(Json::Null, |e| e.to_json());
    if let Some(o) = json.as_object_mut() {
        o.remove("role");
    }
    json
}

// ── a curve between two straights ────────────────────────────────────────────

fn corner_pieces(fit: &CurveFit, b_in: f64, l1: f64, l2: f64) -> Vec<Piece> {
    let sr = fit.signed_r;
    chain_from(
        fit.cl_start,
        b_in,
        &[
            (Kind::Transition(None, Some(sr)), l1),
            (Kind::Arc(sr), fit.arc_len),
            (Kind::Transition(Some(sr), None), l2),
        ],
    )
}

/// How far a clothoid of `length` moves an arc of `r` inwards (L²/24R) [m].
fn shift(length: f64, r: f64) -> f64 {
    if r != 0.0 { length * length / (24.0 * r.abs()) } else { 0.0 }
}

fn corner(line_a: &Line, line_b: &Line, p_from: Pt, p_to: Pt, r: f64, l1: f64, l2: f64) -> Option<CurveFit> {
    fit_curve_group(p_from, line_a.1, p_to, line_b.1, r, l1, l2, Profile::Clothoid, Profile::Clothoid).filter(|f| f.entry_len >= 0.0 && f.exit_len >= 0.0)
}

/// A first guess from κ between two stations: R from its plateau, L1 and L2
/// from the ramps, and the stations where κ passes half the plateau on the
/// way up and down. None where κ has no plateau.
#[derive(Debug, Clone, Copy)]
struct Guess {
    r: f64,
    sign: f64,
    l1: f64,
    l2: f64,
    rise: Option<f64>,
    fall: Option<f64>,
}

fn start_values(kappa: &[f64], s: &[f64], frm: f64, to: f64) -> Option<Guess> {
    let (ss, k): (Vec<f64>, Vec<f64>) = s
        .iter()
        .zip(kappa)
        .filter(|(st, k)| **st >= frm && **st <= to && k.is_finite())
        .map(|(a, b)| (*a, *b))
        .unzip();
    if k.len() < 3 {
        return None;
    }
    let a: Vec<f64> = k.iter().map(|v| v.abs()).collect();
    let top = a.iter().copied().fold(0.0, f64::max);
    let plateau = if top > 0.0 {
        median(&a.iter().copied().filter(|v| *v >= 0.7 * top).collect::<Vec<_>>())
    } else {
        0.0
    };
    if plateau <= 1e-7 {
        return None;
    }
    let over = |f: f64| -> Vec<usize> { (0..a.len()).filter(|&i| a[i] >= f * plateau).collect() };
    let (rise, full, half) = (over(0.2), over(0.8), over(0.5));
    let mean = k.iter().sum::<f64>() / k.len() as f64;
    Some(Guess {
        r: 1.0 / plateau,
        sign: if mean >= 0.0 { 1.0 } else { -1.0 },
        l1: ((ss[full[0]] - ss[rise[0]]) / 0.6 - 5.0).max(0.0),
        l2: ((ss[*rise.last().unwrap()] - ss[*full.last().unwrap()]) / 0.6 - 5.0).max(0.0),
        rise: (half[0] > 0).then(|| ss[half[0]]),
        fall: (*half.last().unwrap() < a.len() - 1).then(|| ss[*half.last().unwrap()]),
    })
}

/// R, L1, L2 of the curve between two lines adjusted to `pts_fit` by least
/// squares. `extent`: a point on line a before it and one on line b after it,
/// the stretch of line the points are also measured against.
fn fit_corner(
    pts_fit: &[Pt],
    line_a: &Line,
    line_b: &Line,
    p_from: Pt,
    p_to: Pt,
    guess: (f64, f64, f64),
    extent: (Pt, Pt),
) -> Option<(CurveFit, f64, f64, f64)> {
    let b_in = bearing_of(line_a.1);
    let build = |r: f64, l1: f64, l2: f64| -> Option<(CurveFit, Vec<Pt>)> {
        let fit = corner(line_a, line_b, p_from, p_to, r, l1, l2)?;
        let (poly, _) = chain_points(&corner_pieces(&fit, b_in, l1, l2));
        let mut full: Vec<Pt> = segment(extent.0, poly[0]);
        full.pop();
        full.extend(&poly);
        full.extend(segment(*poly.last().unwrap(), extent.1).into_iter().skip(1));
        Some((fit, full))
    };
    let cost = |x: &[f64], fixed: (bool, bool)| -> f64 {
        let r = x[0].abs();
        let ls: Vec<f64> = x[1..].iter().map(|v| v.abs()).collect();
        let l1 = if fixed.0 { 0.0 } else { ls[0] };
        let l2 = if fixed.1 { 0.0 } else { *ls.last().unwrap() };
        if r < R_MIN {
            return 1e12;
        }
        build(r, l1, l2).map_or(1e12, |(_, poly)| sse(pts_fit, &poly))
    };
    let run = |r0: f64, l1: f64, l2: f64, fixed: (bool, bool)| -> (f64, f64, f64, f64) {
        let mut x0 = vec![r0];
        if !fixed.0 {
            x0.push(l1);
        }
        if !fixed.1 {
            x0.push(l2);
        }
        let mut simplex = vec![x0.clone()];
        for i in 0..x0.len() {
            let mut x = x0.clone();
            x[i] += if i == 0 { (0.1 * r0).max(5.0) } else { 10.0 };
            simplex.push(x);
        }
        let m = nelder_mead(
            |x| cost(x, fixed),
            &x0,
            &NelderMead {
                initial_simplex: Some(simplex),
                xatol: 0.01,
                fatol: FATOL,
                maxfev: usize::MAX,
                maxiter: Some(800),
            },
        );
        let mut it = m.x[1..].iter().map(|v| v.abs());
        let l1v = if fixed.0 { 0.0 } else { it.next().unwrap() };
        let l2v = if fixed.1 { 0.0 } else { it.next().unwrap() };
        (m.x[0].abs(), l1v, l2v, m.fun)
    };
    let (mut r0, mut l1, mut l2) = guess;
    // A start the construction cannot build is no start: shorten the ramps,
    // then widen the curve, until it can.
    let mut built = false;
    for _ in 0..40 {
        if build(r0, l1, l2).is_some() {
            built = true;
            break;
        }
        if l1 + l2 > 1.0 {
            l1 /= 2.0;
            l2 /= 2.0;
        } else {
            l1 = 0.0;
            l2 = 0.0;
            r0 *= 0.8;
        }
    }
    if !built {
        return None;
    }
    let full = run(r0, l1, l2, (false, false));
    let n = pts_fit.len().max(1) as f64;
    let mut best = full;
    // A transition the points do not show is not there: the simplest fit
    // that is as good as the full one wins.
    let weak = [shift(full.1, full.0) < TRANSITION_TEST_SHIFT, shift(full.2, full.0) < TRANSITION_TEST_SHIFT];
    for fixed in [(true, true), (true, false), (false, true)] {
        if (fixed.0 && !weak[0]) || (fixed.1 && !weak[1]) {
            continue;
        }
        let cand = run(full.0, full.1, full.2, fixed);
        if (cand.3 / n).sqrt() <= (full.3 / n).sqrt() + TRANSITION_EVIDENCE {
            best = cand;
            break;
        }
    }
    let (r, l1, l2, f) = best;
    let (fit, _) = build(r, l1, l2)?;
    Some((fit, l1, l2, (f / n).sqrt()))
}

// ── an open end: a curve the points stop in ──────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct EdgeGuess {
    r: f64,
    sign: f64,
    l1: f64,
    l2: f64,
    t: f64,
    arc: Option<f64>,
}

struct Edge {
    pieces: Vec<Piece>,
    tangent: Pt,
    radius: f64,
    l1: f64,
    l2: f64,
}

/// A curve after a straight that the points stop inside: from the tangent
/// point on the straight a transition L1, an arc R and — if the points show
/// it — an exit transition L2 and the straight beyond, cut where
/// `last_point` projects.
fn fit_edge(pts_fit: &[Pt], line: &Line, base: Pt, guess: EdgeGuess, last_point: Pt, t_min: f64, noise: f64) -> Edge {
    let d = line.1;
    let b0 = bearing_of(d);
    let sign = guess.sign;
    let reach = 10.0_f64.max(sub(last_point, base).0.hypot(sub(last_point, base).1) + 20.0);
    let pieces_for = |t: f64, r: f64, l1: f64, arc: Option<f64>, l2: f64| -> (Vec<Piece>, Pt) {
        let tp = add_scaled(base, t, d);
        let sr = sign * r;
        let arc = arc.unwrap_or_else(|| reach.min(0.9 * std::f64::consts::PI * r));
        let mut specs = vec![(Kind::Transition(None, Some(sr)), l1), (Kind::Arc(sr), arc)];
        if l2 > 0.0 {
            specs.push((Kind::Transition(Some(sr), None), l2));
            specs.push((Kind::Straight, reach));
        }
        (chain_from(tp, b0, &specs), tp)
    };
    let back = dot(sub(pts_fit[0], base), d).min(0.0) - 1.0;
    let cost = |x: &[f64]| -> f64 {
        let (t, r) = (x[0], x[1].abs());
        if r < R_MIN {
            return 1e12;
        }
        let rest: Vec<f64> = x[2..].iter().map(|v| v.abs()).collect();
        let (pieces, _) = match rest[..] {
            [] => pieces_for(t, r, 0.0, None, 0.0),
            [l1] => pieces_for(t, r, l1, None, 0.0),
            [l1, arc, l2] => pieces_for(t, r, l1, Some(arc), l2),
            _ => return 1e12,
        };
        let (poly, _) = chain_points(&pieces);
        let mut full = segment(add_scaled(base, t.min(back), d), poly[0]);
        full.pop();
        full.extend(poly);
        sse(pts_fit, &full) + 1e6 * (t_min - t).max(0.0).powi(2)
    };
    let nm = |x0: Vec<f64>| -> (Vec<f64>, f64) {
        let mut steps = vec![5.0, (0.1 * x0[1]).max(5.0)];
        steps.extend(std::iter::repeat_n(10.0, x0.len() - 2));
        let mut simplex = vec![x0.clone()];
        for i in 0..x0.len() {
            let mut x = x0.clone();
            x[i] += steps[i];
            simplex.push(x);
        }
        let m = nelder_mead(
            cost,
            &x0,
            &NelderMead {
                initial_simplex: Some(simplex),
                xatol: 0.01,
                fatol: FATOL,
                maxfev: usize::MAX,
                maxiter: Some(1500),
            },
        );
        let mut x = vec![m.x[0]];
        x.extend(m.x[1..].iter().map(|v| v.abs()));
        (x, m.fun)
    };
    let n = pts_fit.len().max(1) as f64;
    let t0 = t_min.max(guess.t);
    // The arc runs on to the end of the points …
    let (mut x, mut f) = nm(vec![t0, guess.r, guess.l1]);
    // … with no transition before it, if the points do not show one …
    if shift(x[2], x[1]) < TRANSITION_TEST_SHIFT {
        let (x0, f0) = nm(vec![x[0] + x[2] / 2.0, x[1]]);
        if (f0 / n).sqrt() <= (f / n).sqrt() + TRANSITION_EVIDENCE {
            x = vec![x0[0], x0[1], 0.0];
            f = f0;
        }
    }
    // (t, r, l1, arc, l2)
    let mut model = (x[0], x[1], x[2], None, 0.0);
    // … or leaves it again before they stop.
    if guess.arc.is_some() || (f / n).sqrt() > EDGE_EXIT_NOISE * noise {
        let arc0 = guess
            .arc
            .unwrap_or_else(|| (chain_station(&pieces_for(x[0], x[1], x[2], None, 0.0).0, last_point) - x[2] - 20.0).max(1.0));
        let (x5, f5) = nm(vec![t0, guess.r, guess.l1, arc0.max(1.0), guess.l2.max(10.0)]);
        if f5 < EDGE_EXIT_GAIN.powi(2) * f {
            model = (x5[0], x5[1], x5[2], Some(x5[3]), x5[4]);
        }
    }
    let (t, r, l1, arc, l2) = model;
    let (pieces, tp) = pieces_for(t, r, l1, arc, l2);
    Edge {
        pieces: cut_chain(&pieces, last_point),
        tangent: tp,
        radius: sign * r,
        l1,
        l2,
    }
}

// ── the whole fit ────────────────────────────────────────────────────────────

fn line_through(pts: &[Pt]) -> Line {
    let (c, d, _) = fit_line(pts);
    (c, d)
}

fn scatter(p: &[Pt], line: &Line) -> f64 {
    let (c, d) = *line;
    let v: f64 = p
        .iter()
        .map(|q| {
            let r = sub(*q, c);
            (r.0 * d.1 - r.1 * d.0).powi(2)
        })
        .sum();
    (v / p.len() as f64).sqrt()
}

fn foot(line: &Line, p: Pt) -> Pt {
    let (c, d) = *line;
    add_scaled(c, dot(sub(p, c), d), d)
}

/// What the curvature along a fitted arc says about it: a reverse or a
/// compound curve, which one arc cannot follow.
fn notes_for(kappa: &[f64], s: &[f64], frm: f64, to: f64, radius: f64, k_noise: f64) -> Vec<&'static str> {
    if to - frm < 15.0 {
        return Vec::new();
    }
    let plateau = 1.0 / radius;
    let mut means = Vec::new();
    for q in 0..3 {
        let (a, b) = (frm + (to - frm) * q as f64 / 3.0, frm + (to - frm) * (q + 1) as f64 / 3.0);
        let k: Vec<f64> = s
            .iter()
            .zip(kappa)
            .filter(|(st, k)| **st >= a && **st <= b && k.is_finite())
            .map(|(_, k)| *k)
            .collect();
        if k.len() < 3 {
            return Vec::new();
        }
        means.push(k.iter().sum::<f64>() / k.len() as f64 * radius.signum());
    }
    let floor = 3.0 * k_noise;
    let (lo, hi) = means.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), m| (l.min(*m), h.max(*m)));
    if lo < -(0.3 * plateau).max(floor) {
        return vec!["reverse"];
    }
    if hi - lo > (0.25 * plateau).max(floor) {
        return vec!["compound"];
    }
    Vec::new()
}

struct Curve {
    kind: &'static str,
    /// The pieces it is, first and one past the last.
    elements: (usize, usize),
    radius: f64,
    l1: f64,
    l2: f64,
}

/// The chain in the making, in travel order, and the curves in it.
#[derive(Default)]
struct Chain {
    pieces: Vec<Piece>,
    curves: Vec<Curve>,
    overlap: Option<(Pt, f64)>,
}

impl Chain {
    /// The straight from `frm` to `to` on a line of direction `d`.
    fn straight(&mut self, frm: Pt, to: Pt, d: Pt) {
        let length = dot(sub(to, frm), d);
        if length < -0.001 && self.overlap.is_none() {
            self.overlap = Some((frm, length));
        }
        if length > 1e-6 {
            self.pieces.push(Piece {
                kind: Kind::Straight,
                start: frm,
                bearing: bearing_of(d),
                length,
            });
        }
    }

    fn curve(&mut self, kind: &'static str, pieces: Vec<Piece>, radius: f64, l1: f64, l2: f64) {
        let first = self.pieces.len();
        self.pieces.extend(pieces.into_iter().filter(|pc| pc.length > 1e-6));
        self.curves.push(Curve {
            kind,
            elements: (first, self.pieces.len()),
            radius,
            l1,
            l2,
        });
    }
}

/// No curve fits a stretch; `pair` the straight before it, where it lies between two.
struct NoFit {
    code: &'static str,
    frm: f64,
    to: f64,
    pair: Option<usize>,
}

fn edge_guess(g: &Guess, at: f64, way: f64) -> EdgeGuess {
    let (l1, l2, rise, fall) = if way > 0.0 {
        (g.l1, g.l2, g.rise, g.fall)
    } else {
        (g.l2, g.l1, g.fall, g.rise)
    };
    let t = match rise {
        Some(r) if way > 0.0 => r - l1 / 2.0 - at,
        Some(r) => at - r - l1 / 2.0,
        None => 0.0,
    };
    let arc = match (rise, fall) {
        (Some(r), Some(f)) => Some(((f - r).abs() - (l1 + l2) / 2.0).max(1.0)),
        _ => None,
    };
    EdgeGuess {
        r: g.r,
        sign: g.sign * way,
        l1,
        l2,
        t,
        arc,
    }
}

fn reverse_pieces(pieces: &[Piece]) -> Vec<Piece> {
    pieces
        .iter()
        .rev()
        .map(|pc| {
            let (end, end_b) = piece_end(pc);
            let kind = match pc.kind {
                Kind::Straight => Kind::Straight,
                Kind::Arc(r) => Kind::Arc(-r),
                Kind::Transition(r1, r2) => Kind::Transition(r2.filter(|r| *r != 0.0).map(|r| -r), r1.filter(|r| *r != 0.0).map(|r| -r)),
            };
            Piece {
                kind,
                start: end,
                bearing: (end_b + 180.0).rem_euclid(360.0),
                length: pc.length,
            }
        })
        .collect()
}

fn off_line(p: &[Pt], line: &Line) -> f64 {
    let (c, d) = *line;
    p.iter().map(|q| dot(sub(*q, c), (d.1, -d.0)).abs()).fold(0.0, f64::max)
}

/// The chain through the straights `ranges`, the curves between them and at the ends fitted.
fn assemble(pts: &[Pt], s: &[f64], kappa: &[f64], ranges: &[(usize, usize)], o: &Settings) -> Result<Chain, NoFit> {
    let lines: Vec<Line> = ranges.iter().map(|&(i, j)| line_through(&pts[i..=j])).collect();
    // How far the points scatter about a line: what a fit cannot do better than.
    let noise = median(&ranges.iter().zip(&lines).map(|(&(i, j), line)| scatter(&pts[i..=j], line)).collect::<Vec<_>>());
    let mut chain = Chain::default();
    let n = pts.len();

    // The start: a curve the points begin in, fitted backwards from the first straight.
    let (i0, j0) = ranges[0];
    let mut junction = foot(&lines[0], pts[0]);
    if i0 > 0 && off_line(&pts[..=i0], &lines[0]) > o.tolerance {
        let (c0, d0) = lines[0];
        let k_hi = search_right(s, s[i0] + CURVE_MARGIN.min((s[j0] - s[i0]) / 2.0));
        let Some(g) = start_values(kappa, s, s[0], s[i0]) else {
            return Err(NoFit {
                code: "align_error_curve",
                frm: s[0],
                to: s[i0],
                pair: None,
            });
        };
        // Run backwards: the curve turns the other way, its exit is the entry.
        let rev: Vec<Pt> = pts[..k_hi].iter().rev().copied().collect();
        let edge = fit_edge(
            &rev,
            &(c0, (-d0.0, -d0.1)),
            foot(&lines[0], pts[i0]),
            edge_guess(&g, s[i0], -1.0),
            pts[0],
            -(s[j0] - s[i0]) / 2.0,
            noise,
        );
        chain.curve("start", reverse_pieces(&edge.pieces), -edge.radius, edge.l2, edge.l1);
        junction = edge.tangent;
    }

    for (k, (&(i, j), line)) in ranges.iter().zip(&lines).enumerate() {
        if k + 1 < ranges.len() {
            let ((ni, nj), nline) = (ranges[k + 1], &lines[k + 1]);
            let lo = search_left(s, s[j] - CURVE_MARGIN.min((s[j] - s[i]) / 2.0));
            let hi = search_right(s, s[ni] + CURVE_MARGIN.min((s[nj] - s[ni]) / 2.0));
            let fitted = start_values(kappa, s, s[j], s[ni]).and_then(|g| {
                fit_corner(
                    &pts[lo..hi],
                    line,
                    nline,
                    junction,
                    foot(nline, pts[nj]),
                    (g.r, g.l1, g.l2),
                    (foot(line, pts[lo]), foot(nline, pts[hi - 1])),
                )
            });
            let Some((fit, l1, l2, _)) = fitted else {
                return Err(NoFit {
                    code: "align_error_curve",
                    frm: s[j],
                    to: s[ni],
                    pair: Some(k),
                });
            };
            chain.straight(junction, fit.cl_start, line.1);
            chain.curve("curve", corner_pieces(&fit, bearing_of(line.1), l1, l2), fit.signed_r, l1, l2);
            junction = fit.cl_end;
            continue;
        }
        // The end: a curve the points stop in, or the straight runs out.
        let d = line.1;
        if j < n - 1 && off_line(&pts[j..], line) > o.tolerance {
            let lo = search_left(s, s[j] - CURVE_MARGIN.min((s[j] - s[i]) / 2.0));
            let base = foot(line, pts[j]);
            let Some(g) = start_values(kappa, s, s[j], s[n - 1]) else {
                return Err(NoFit {
                    code: "align_error_curve",
                    frm: s[j],
                    to: s[n - 1],
                    pair: None,
                });
            };
            let edge = fit_edge(
                &pts[lo..],
                line,
                base,
                edge_guess(&g, s[j], 1.0),
                pts[n - 1],
                dot(sub(junction, base), d),
                noise,
            );
            chain.straight(junction, edge.tangent, d);
            chain.curve("end", edge.pieces, edge.radius, edge.l1, edge.l2);
        } else {
            chain.straight(junction, foot(line, pts[n - 1]), d);
        }
    }

    if let Some((frm, length)) = chain.overlap {
        let nearest = (0..n)
            .min_by(|&a, &b| {
                sub(pts[a], frm)
                    .0
                    .hypot(sub(pts[a], frm).1)
                    .total_cmp(&sub(pts[b], frm).0.hypot(sub(pts[b], frm).1))
            })
            .unwrap();
        let station = s[nearest];
        return Err(NoFit {
            code: "align_error_overlap",
            frm: station + length,
            to: station,
            pair: None,
        });
    }
    Ok(chain)
}

fn cant_step(v: f64) -> f64 {
    (v / CANT_STEP).round_ties_even() * CANT_STEP + 0.0 // never −0.0
}

/// Per element the cant the points on it carry: [{cant, measured, n}] in mm —
/// `cant` rounded, `measured` the median, null for a transition and where too
/// few points say — with `cantStart` / `cantEnd` on a transition at an open
/// end. Null where no point carries a cant.
fn element_cants(pieces: &[Piece], along: &[f64], cants: &[f64]) -> Json {
    let have: Vec<usize> = (0..cants.len()).filter(|&i| cants[i].is_finite()).collect();
    if have.len() < CANT_MIN_POINTS {
        return Json::Null;
    }
    let mut starts = vec![0.0];
    for pc in pieces {
        starts.push(starts.last().unwrap() + pc.length);
    }
    let median_in = |lo: f64, hi: f64| -> (Option<f64>, usize) {
        let u: Vec<f64> = have.iter().filter(|&&i| along[i] >= lo && along[i] <= hi).map(|&i| cants[i]).collect();
        (if u.len() >= CANT_MIN_POINTS { Some(median(&u)) } else { None }, u.len())
    };
    let out: Vec<Json> = pieces
        .iter()
        .enumerate()
        .map(|(k, pc)| {
            let (a, b) = (starts[k], starts[k + 1]);
            let mut entry = json!({"cant": null, "measured": null, "n": 0});
            if let Kind::Transition(..) = pc.kind {
                if k == 0
                    && let (Some(v), _) = median_in(a, a + CANT_END)
                {
                    entry["cantStart"] = json!(cant_step(v));
                }
                if k == pieces.len() - 1
                    && let (Some(v), _) = median_in(b - CANT_END, b)
                {
                    entry["cantEnd"] = json!(cant_step(v));
                }
            } else {
                let trim = CANT_TRIM.min((b - a) / 4.0);
                let (v, n) = median_in(a + trim, b - trim);
                entry["n"] = json!(n);
                if let Some(v) = v {
                    entry["cant"] = json!(cant_step(v));
                    entry["measured"] = json!(round_to(v, 1));
                }
            }
            entry
        })
        .collect();
    Json::Array(out)
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().map(|x| x.abs()).fold(0.0, f64::max)
}

fn rms(v: &[f64]) -> f64 {
    (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt()
}

/// Elements in the plane, every point's offset from them, per element the
/// largest offset and the RMS — into `answer`. Returns where every point
/// stands along the chain [m from its start].
fn report(answer: &mut Map<String, Json>, pieces: &[Piece], pts: &[Pt], s: &[f64], origin: Pt) -> Vec<f64> {
    let elements: Vec<Json> = pieces
        .iter()
        .map(|pc| {
            piece_element(&Piece {
                start: (pc.start.0 + origin.0, pc.start.1 + origin.1),
                ..*pc
            })
        })
        .collect();
    let (poly, owner) = chain_points(pieces);
    let n = pts.len();
    let (mut offsets, mut which, mut along) = (vec![0.0; n], vec![0usize; n], vec![0.0; n]);
    // In blocks along the chain, so no block measures every point against every vertex.
    let chain_s = cumulative(&poly);
    let rel: Vec<f64> = s.iter().map(|v| v - s[0]).collect();
    const BLOCK: usize = 400;
    let mut a = 0;
    while a < n {
        let b = (a + BLOCK).min(n);
        let lo = search_left(&chain_s, rel[a] - 50.0).saturating_sub(1);
        let hi = (search_left(&chain_s, rel[b - 1] + 50.0) + 2).min(poly.len());
        let (off, seg, ts) = project(&pts[a..b], &poly[lo..hi], None);
        for q in 0..b - a {
            let g = lo + seg[q];
            offsets[a + q] = off[q];
            along[a + q] = chain_s[g] + ts[q] * (chain_s[g + 1] - chain_s[g]);
            which[a + q] = owner[(g + 1).min(owner.len() - 1)];
        }
        a = b;
    }
    let stats: Vec<Json> = (0..pieces.len())
        .map(|k| {
            let v: Vec<f64> = (0..n).filter(|&i| which[i] == k).map(|i| offsets[i]).collect();
            json!({
                "n": v.len(),
                "max": if v.is_empty() { Json::Null } else { json!(round_to(max_abs(&v), 4)) },
                "rms": if v.is_empty() { Json::Null } else { json!(round_to(rms(&v), 4)) },
            })
        })
        .collect();
    answer.insert("elements".into(), Json::Array(elements));
    answer.insert("offsets".into(), offsets.iter().map(|v| json!(round_to(*v, 4))).collect());
    answer.insert("elementStats".into(), Json::Array(stats));
    answer.insert("rms".into(), json!(round_to(rms(&offsets), 4)));
    answer.insert("max".into(), json!(round_to(max_abs(&offsets), 4)));
    // The polyline is a little shorter than the arcs it is drawn with.
    let length: f64 = pieces.iter().map(|pc| pc.length).sum();
    let total = *chain_s.last().unwrap();
    if total > 0.0 {
        along.iter().map(|v| v * length / total).collect()
    } else {
        along
    }
}

fn stations_json(s: &[f64], ranges: &[(usize, usize)]) -> Json {
    ranges
        .iter()
        .map(|&(i, j)| json!({"from": round_to(s[i], 3), "to": round_to(s[j], 3)}))
        .collect()
}

/// The fit. `points` [[e, n], …], [[e, n, z], …] or [[e, n, z, cant], …]
/// (null where a point has none) in travel order; `straights` stations set by
/// hand, or None to look for them.
fn align(points: &[Vec<Option<f64>>], station0: f64, straights: Option<Vec<(f64, f64)>>, settings: &Json) -> Json {
    let (o, settings_json) = Settings::from_json(settings);
    if points.len() < 10 {
        return json!({"error": "align_error_few_points", "errorParams": {}});
    }
    let column = |i: usize| -> Vec<f64> { points.iter().map(|p| p.get(i).copied().flatten().unwrap_or(f64::NAN)).collect() };
    let (xs, ys, zs, us) = (column(0), column(1), column(2), column(3));
    // Repeated points carry no direction.
    let mut keep = vec![0usize];
    for i in 1..points.len() {
        if (xs[i] - xs[i - 1]).hypot(ys[i] - ys[i - 1]) > 1e-4 {
            keep.push(i);
        }
    }
    let origin = (xs[keep[0]], ys[keep[0]]);
    let pts: Vec<Pt> = keep.iter().map(|&i| (xs[i] - origin.0, ys[i] - origin.1)).collect();
    let heights: Vec<f64> = keep.iter().map(|&i| zs[i]).collect();
    let cants: Vec<f64> = keep.iter().map(|&i| us[i]).collect();
    let (s, seg) = stations_of(&pts, station0);
    let gaps = find_gaps(&seg);
    let kappa = curvature(&pts, &s, &gaps, o.chord);

    let auto = straights.is_none();
    let mut ranges = match &straights {
        None => find_straights(&pts, &s, &gaps, &o),
        Some(given) => given_straights(&s, given),
    };
    let mut answer = Map::new();
    answer.insert("stations".into(), s.iter().map(|v| json!(round_to(*v, 3))).collect());
    answer.insert(
        "kappa".into(),
        kappa.iter().map(|v| if v.is_finite() { json!(round_to(*v, 7)) } else { Json::Null }).collect(),
    );
    answer.insert("straights".into(), stations_json(&s, &ranges));
    answer.insert("auto".into(), json!(auto));
    answer.insert(
        "gaps".into(),
        gaps.iter().map(|&g| json!({"from": round_to(s[g], 3), "to": round_to(s[g + 1], 3)})).collect(),
    );
    answer.insert("settings".into(), settings_json);
    for key in ["elements", "offsets", "elementStats", "gradient", "elementCants"] {
        answer.insert(key.into(), Json::Null);
    }
    answer.insert("curves".into(), json!([]));
    if ranges.is_empty() {
        answer.insert("error".into(), json!("align_error_no_straight"));
        return Json::Object(answer);
    }

    // Two straights no curve fits between are one straight the points stray
    // from (a shift in the measured axis); found automatically, they are taken
    // as one and the merge is named. Set by hand, they are the user's.
    let mut merged = Vec::new();
    let chain = loop {
        match assemble(&pts, &s, &kappa, &ranges, &o) {
            Ok(chain) => break chain,
            Err(e) => {
                let (Some(k), true) = (e.pair, auto) else {
                    answer.insert("error".into(), json!(e.code));
                    answer.insert("errorParams".into(), json!({"from": round_to(e.frm, 1), "to": round_to(e.to, 1)}));
                    return Json::Object(answer);
                };
                merged.push(round_to(s[ranges[k].1], 3));
                let joined = (ranges[k].0, ranges[k + 1].1);
                ranges.splice(k..k + 2, [joined]);
            }
        }
    };
    answer.insert("straights".into(), stations_json(&s, &ranges));
    answer.insert("merged".into(), json!(merged));
    let along = report(&mut answer, &chain.pieces, &pts, &s, origin);
    let length: f64 = chain.pieces.iter().map(|pc| pc.length).sum();
    answer.insert("gradient".into(), fit_gradient(&along, &heights, length, o.height_tolerance));
    answer.insert("elementCants".into(), element_cants(&chain.pieces, &along, &cants));

    // Where every element starts, on the stations of the points.
    let mut starts = vec![s[0]];
    for pc in &chain.pieces {
        starts.push(starts.last().unwrap() + pc.length);
    }
    let blur = o.chord / 2.0 + KAPPA_SMOOTH / 2.0;
    let on_straights: Vec<bool> = (0..s.len())
        .map(|q| ranges.iter().any(|&(i, j)| s[q] >= s[i] + blur && s[q] <= s[j] - blur))
        .collect();
    let k_straight: Vec<f64> = (0..s.len()).filter(|&q| on_straights[q] && kappa[q].is_finite()).map(|q| kappa[q]).collect();
    let k_noise = if k_straight.len() > 10 {
        let m = k_straight.iter().sum::<f64>() / k_straight.len() as f64;
        (k_straight.iter().map(|k| (k - m).powi(2)).sum::<f64>() / k_straight.len() as f64).sqrt()
    } else {
        0.0
    };
    let curves: Vec<Json> = chain
        .curves
        .iter()
        .map(|cv| {
            let (a, end) = cv.elements;
            let arcs: Vec<usize> = (a..end).filter(|&q| matches!(chain.pieces[q].kind, Kind::Arc(_))).collect();
            let notes = match arcs.first() {
                Some(&q) => notes_for(&kappa, &s, starts[q] + blur, starts[q + 1] - blur, cv.radius, k_noise),
                None => Vec::new(),
            };
            json!({
                "kind": cv.kind, "elements": [a as i64, end as i64 - 1],
                "radius": round_to(cv.radius, 2), "l1": round_to(cv.l1, 2), "l2": round_to(cv.l2, 2),
                "from": round_to(starts[a], 3), "to": round_to(starts[end], 3),
                "notes": notes,
            })
        })
        .collect();
    answer.insert("curves".into(), Json::Array(curves));
    answer.insert("elementStations".into(), starts.iter().map(|v| json!(round_to(*v, 3))).collect());
    Json::Object(answer)
}

/// The answer to an alignment fit.
///
/// Body: {"points": [[e, n] | [e, n, z] | [e, n, z, cant], …], "station0": m,
/// "straights": [[from, to], …] | null, "settings": {window, tolerance, step,
/// spacing, sagitta, minLength, chord, heightTolerance}}.
///
/// Answers {stations, kappa, straights, auto, gaps, settings, curves,
/// elements, offsets, elementStats, rms, max, gradient, elementCants} — or
/// with `error` (and `errorParams`) where no chain could be made, the
/// curvature and the straights still there for the user to correct.
pub fn align_payload(payload: &Json) -> Result<Json, String> {
    let bad = |what: &str| format!("invalid request: {what}");
    let raw = payload.get("points").and_then(Json::as_array).ok_or_else(|| bad("points"))?;
    if raw.len() > MAX_POINTS {
        return Err(bad("too many points"));
    }
    let points: Vec<Vec<Option<f64>>> = raw
        .iter()
        .map(|p| {
            let a = p.as_array().filter(|a| a.len() >= 2).ok_or_else(|| bad("point"))?;
            if a[0].as_f64().is_none() || a[1].as_f64().is_none() {
                return Err(bad("point"));
            }
            Ok(a.iter().map(Json::as_f64).collect())
        })
        .collect::<Result<_, String>>()?;
    let straights = match payload.get("straights").filter(|s| !s.is_null()) {
        None => None,
        Some(s) => Some(
            s.as_array()
                .ok_or_else(|| bad("straights"))?
                .iter()
                .map(|r| match (r.get(0).and_then(Json::as_f64), r.get(1).and_then(Json::as_f64)) {
                    (Some(a), Some(b)) => Ok((a, b)),
                    _ => Err(bad("straights")),
                })
                .collect::<Result<Vec<_>, String>>()?,
        ),
    };
    let settings = payload.get("settings").cloned().unwrap_or(Json::Null);
    if !(settings.is_null() || settings.is_object()) {
        return Err(bad("settings"));
    }
    Ok(align(
        &points,
        payload.get("station0").and_then(Json::as_f64).unwrap_or(0.0),
        straights,
        &settings,
    ))
}
