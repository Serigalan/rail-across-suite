// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Measuring points against a polyline — what reconnecting and the alignment
//! fit compare a chain of elements with the measured or old axis by
//! (alignment_fit.py's `piece_points` and `project`, with a k-d tree for
//! scipy's cKDTree).

use crate::geometry::{Profile, Pt, sample_transition};

/// Metres between the points a fitted chain is drawn with …
const SAMPLE_STEP: f64 = 1.0;
/// … or closer, where a tight arc needs it …
const SAMPLE_SAGITTA: f64 = 0.0002;
/// … and on a straight.
pub const STRAIGHT_STEP: f64 = 5.0;
/// Vertices whose segments a point is measured against.
const NEAR_VERTICES: usize = 3;

/// A static k-d tree over points in the plane, for the nearest vertices.
pub struct KdTree {
    pts: Vec<Pt>,
    /// Point indices in tree order: each node the median of its range.
    order: Vec<usize>,
}

impl KdTree {
    pub fn new(pts: &[Pt]) -> KdTree {
        let mut order: Vec<usize> = (0..pts.len()).collect();
        fn build(pts: &[Pt], idx: &mut [usize], depth: usize) {
            if idx.len() <= 1 {
                return;
            }
            let mid = idx.len() / 2;
            let axis = depth % 2;
            let key = |i: &usize| if axis == 0 { pts[*i].0 } else { pts[*i].1 };
            idx.select_nth_unstable_by(mid, |a, b| key(a).total_cmp(&key(b)));
            let (left, right) = idx.split_at_mut(mid);
            build(pts, left, depth + 1);
            build(pts, &mut right[1..], depth + 1);
        }
        build(pts, &mut order, 0);
        KdTree { pts: pts.to_vec(), order }
    }

    /// The `k` points nearest to `q`, nearest first.
    pub fn nearest(&self, q: Pt, k: usize) -> Vec<usize> {
        let mut best: Vec<(f64, usize)> = Vec::with_capacity(k + 1);
        self.search(q, k, 0, self.order.len(), 0, &mut best);
        best.into_iter().map(|(_, i)| i).collect()
    }

    fn search(&self, q: Pt, k: usize, lo: usize, hi: usize, depth: usize, best: &mut Vec<(f64, usize)>) {
        if lo >= hi {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        let i = self.order[mid];
        let p = self.pts[i];
        let d2 = (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2);
        if best.len() < k || d2 < best[best.len() - 1].0 {
            let at = best.partition_point(|(d, j)| (*d, *j) < (d2, i));
            best.insert(at, (d2, i));
            best.truncate(k);
        }
        let diff = if depth % 2 == 0 { q.0 - p.0 } else { q.1 - p.1 };
        let (first, second) = if diff < 0.0 { ((lo, mid), (mid + 1, hi)) } else { ((mid + 1, hi), (lo, mid)) };
        self.search(q, k, first.0, first.1, depth + 1, best);
        if best.len() < k || diff * diff < best[best.len() - 1].0 {
            self.search(q, k, second.0, second.1, depth + 1, best);
        }
    }
}

fn linspace(a: f64, b: f64, n: usize) -> impl Iterator<Item = f64> {
    (0..n).map(move |i| if i + 1 == n { b } else { a + (b - a) * i as f64 / (n - 1) as f64 })
}

/// What a piece of a chain is, for drawing it.
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Straight,
    Arc(f64),
    Transition(Option<f64>, Option<f64>, Profile),
}

/// Points along one piece from `p` at `bearing`, `length` long: a straight
/// every STRAIGHT_STEP, a curve every metre or closer where it is tight.
pub fn piece_points(p: Pt, bearing: f64, length: f64, shape: Shape) -> Vec<Pt> {
    if length <= 0.0 {
        return vec![p];
    }
    let d = (bearing.to_radians().sin(), bearing.to_radians().cos());
    let radii: Vec<f64> = match shape {
        Shape::Straight => {
            let n = ((length / STRAIGHT_STEP).ceil() as usize + 1).max(2);
            return linspace(0.0, length, n).map(|t| (p.0 + t * d.0, p.1 + t * d.1)).collect();
        }
        Shape::Arc(r) => vec![r.abs()],
        Shape::Transition(r1, r2, _) => [r1, r2].into_iter().flatten().filter(|r| *r != 0.0).map(f64::abs).collect(),
    };
    let tight = radii.iter().copied().fold(f64::INFINITY, f64::min);
    let step = SAMPLE_STEP.min((8.0 * SAMPLE_SAGITTA * tight).sqrt());
    let steps = ((length / step).ceil() as usize).max(2);
    match shape {
        Shape::Arc(r) => {
            let side = if r > 0.0 { 1.0 } else { -1.0 };
            let ar = r.abs();
            let (cx, cy) = (p.0 + ar * side * d.1, p.1 - ar * side * d.0);
            let (rx, ry) = (p.0 - cx, p.1 - cy);
            linspace(0.0, length / ar, steps + 1)
                .map(|a| {
                    let ang = -side * a;
                    (cx + rx * ang.cos() - ry * ang.sin(), cy + rx * ang.sin() + ry * ang.cos())
                })
                .collect()
        }
        Shape::Transition(r1, r2, profile) => sample_transition(p.0, p.1, bearing, length, r1, r2, profile, Some(steps)),
        Shape::Straight => unreachable!(),
    }
}

/// Distance of every point to the polyline, signed (> 0 right of its
/// direction), the segment it is nearest and where along it (0…1).
///
/// Only the segments at the vertices nearest to a point are tried: the points
/// lie millimetres from a polyline that does not fold back on itself. `tree`
/// is the polyline's where the caller measures against it often.
pub fn project(points: &[Pt], poly: &[Pt], tree: Option<&KdTree>) -> (Vec<f64>, Vec<usize>, Vec<f64>) {
    let own;
    let tree = match tree {
        Some(t) => t,
        None => {
            own = KdTree::new(poly);
            &own
        }
    };
    let m = poly.len() - 1;
    let k = NEAR_VERTICES.min(poly.len());
    let mut off = Vec::with_capacity(points.len());
    let mut segs = Vec::with_capacity(points.len());
    let mut ts = Vec::with_capacity(points.len());
    for &p in points {
        let near = tree.nearest(p, k);
        let mut best = (f64::INFINITY, 0usize, 0.0, 0.0);
        for c in near.iter().map(|&v| v.saturating_sub(1)).chain(near.iter().copied()) {
            let c = c.min(m.saturating_sub(1));
            let a = poly[c];
            let b = poly[(c + 1).min(m)];
            let ab = (b.0 - a.0, b.1 - a.1);
            let den = ab.0 * ab.0 + ab.1 * ab.1;
            let den = if den == 0.0 { 1.0 } else { den };
            let ap = (p.0 - a.0, p.1 - a.1);
            let t = ((ap.0 * ab.0 + ap.1 * ab.1) / den).clamp(0.0, 1.0);
            let dv = (ap.0 - t * ab.0, ap.1 - t * ab.1);
            let d2 = dv.0 * dv.0 + dv.1 * dv.1;
            if d2 < best.0 {
                best = (d2, c, t, ab.0 * ap.1 - ab.1 * ap.0);
            }
        }
        let dist = best.0.sqrt();
        off.push(if best.3 > 0.0 { -dist } else { dist });
        segs.push(best.1);
        ts.push(best.2);
    }
    (off, segs, ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_finds_the_nearest() {
        let pts: Vec<Pt> = (0..500).map(|i| ((i as f64 * 0.37).sin() * 100.0, (i as f64 * 0.91).cos() * 100.0)).collect();
        let tree = KdTree::new(&pts);
        for q in [(3.0, -7.0), (99.0, 99.0), (-50.0, 20.0)] {
            let mut brute: Vec<usize> = (0..pts.len()).collect();
            let d = |i: &usize| (pts[*i].0 - q.0).powi(2) + (pts[*i].1 - q.1).powi(2);
            brute.sort_by(|a, b| d(a).total_cmp(&d(b)));
            assert_eq!(tree.nearest(q, 3), brute[..3].to_vec());
        }
    }

    #[test]
    fn points_right_of_a_line_are_positive() {
        let poly = piece_points((0.0, 0.0), 0.0, 100.0, Shape::Straight);
        let (off, _, _) = project(&[(2.0, 50.0), (-3.0, 20.0)], &poly, None);
        assert!((off[0] - 2.0).abs() < 1e-12 && (off[1] + 3.0).abs() < 1e-12, "{off:?}");
    }
}
