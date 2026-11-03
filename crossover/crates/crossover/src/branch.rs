// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! One turnout's branch end over its toe station, interpolated from the app's
//! samples: position and tangent by a cubic spline (one element under the
//! turnout, so they are smooth), the curvature, the end run and the cant at
//! the ldS piecewise linear, and the stretches where the turnout keeps its own
//! checks.

use serde::Deserialize;

/// A sample as the app's `crossoverRequest` makes it: at B1E (first turnout)
/// or B2A (second), in the direction the connection runs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    /// The toe station [m], 0 at the pick.
    pub s: f64,
    pub x: f64,
    pub y: f64,
    /// The math angle of the tangent [rad].
    pub psi: f64,
    /// The curvature, math-signed [1/m].
    pub kappa: f64,
    /// The run of one curvature the branch ends with [m].
    #[serde(default)]
    pub tail: Option<f64>,
    /// The cant at the turnout's ldS [mm].
    #[serde(default)]
    pub cant_lds: Option<f64>,
    /// The turnout keeps its own checks there.
    #[serde(default)]
    pub ok: bool,
}

/// The values of a branch end at one toe station.
#[derive(Debug, Clone, Copy)]
pub struct End {
    pub x: f64,
    pub y: f64,
    pub psi: f64,
    pub kappa: f64,
    pub tail: f64,
    pub cant: f64,
}

/// A cubic spline through (s, v) with not-a-knot ends, as scipy's
/// `CubicSpline` makes one by default: per interval the coefficients of
/// v(s) = a + b·d + c·d² + e·d³, d = s − s_i.
#[derive(Debug, Clone)]
struct Spline {
    coeffs: Vec<[f64; 4]>,
}

impl Spline {
    fn new(s: &[f64], v: &[f64]) -> Spline {
        let n = s.len() - 1;
        let h: Vec<f64> = (0..n).map(|i| s[i + 1] - s[i]).collect();
        let d: Vec<f64> = (0..n).map(|i| (v[i + 1] - v[i]) / h[i]).collect();
        // The second derivatives at the knots.
        let m = if n < 2 {
            vec![0.0; n + 1]
        } else if n == 2 {
            // Three points: not-a-knot is the one parabola through them.
            let c = (d[1] - d[0]) / (h[0] + h[1]) * 2.0;
            vec![c; 3]
        } else {
            not_a_knot(&h, &d)
        };
        let coeffs = (0..n)
            .map(|i| {
                let hi = h[i];
                [v[i], d[i] - hi * (2.0 * m[i] + m[i + 1]) / 6.0, m[i] / 2.0, (m[i + 1] - m[i]) / (6.0 * hi)]
            })
            .collect();
        Spline { coeffs }
    }

    fn at(&self, i: usize, d: f64) -> f64 {
        let [a, b, c, e] = self.coeffs[i];
        ((e * d + c) * d + b) * d + a
    }
}

/// The second derivatives of a not-a-knot spline: the interior equations
/// with M₀ and Mₙ written by M₁, M₂ and Mₙ₋₁, Mₙ₋₂ — a tridiagonal system.
fn not_a_knot(h: &[f64], d: &[f64]) -> Vec<f64> {
    let n = h.len();
    let k = n - 1; // unknowns M₁ … Mₙ₋₁
    let mut lower = vec![0.0; k];
    let mut diag = vec![0.0; k];
    let mut upper = vec![0.0; k];
    let mut rhs = vec![0.0; k];
    for j in 0..k {
        let i = j + 1;
        lower[j] = h[i - 1];
        diag[j] = 2.0 * (h[i - 1] + h[i]);
        upper[j] = h[i];
        rhs[j] = 6.0 * (d[i] - d[i - 1]);
    }
    // M₀ = ((h₀ + h₁) M₁ − h₀ M₂) / h₁
    diag[0] += h[0] * (h[0] + h[1]) / h[1];
    upper[0] -= h[0] * h[0] / h[1];
    // Mₙ = ((hₙ₋₁ + hₙ₋₂) Mₙ₋₁ − hₙ₋₁ Mₙ₋₂) / hₙ₋₂
    let (a, b) = (h[n - 1], h[n - 2]);
    diag[k - 1] += a * (a + b) / b;
    lower[k - 1] -= a * a / b;
    let inner = thomas(&lower, &diag, &upper, &rhs);
    let mut m = Vec::with_capacity(n + 1);
    m.push(((h[0] + h[1]) * inner[0] - h[0] * inner[1]) / h[1]);
    m.extend_from_slice(&inner);
    m.push(((a + b) * inner[k - 1] - a * inner[k - 2]) / b);
    m
}

/// A tridiagonal system solved by elimination (diagonally dominant here).
fn thomas(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64]) -> Vec<f64> {
    let n = diag.len();
    let mut c = vec![0.0; n];
    let mut r = vec![0.0; n];
    c[0] = upper[0] / diag[0];
    r[0] = rhs[0] / diag[0];
    for i in 1..n {
        let q = diag[i] - lower[i] * c[i - 1];
        c[i] = upper[i] / q;
        r[i] = (rhs[i] - lower[i] * r[i - 1]) / q;
    }
    let mut out = vec![0.0; n];
    out[n - 1] = r[n - 1];
    for i in (0..n - 1).rev() {
        out[i] = r[i] - c[i] * out[i + 1];
    }
    out
}

/// A branch end over its toe station.
#[derive(Debug, Clone)]
pub struct BranchEnd {
    pub s: Vec<f64>,
    x: Spline,
    y: Spline,
    psi: Spline,
    kappa: Vec<f64>,
    tail: Vec<f64>,
    cant: Vec<f64>,
    /// The stretches [from, to] of toe stations where the turnout keeps its checks.
    pub intervals: Vec<(f64, f64)>,
}

impl BranchEnd {
    /// None where there are fewer than two samples or two at one station.
    pub fn new(samples: &[Sample]) -> Option<BranchEnd> {
        let mut pts: Vec<&Sample> = samples.iter().filter(|p| p.s.is_finite()).collect();
        pts.sort_by(|a, b| a.s.total_cmp(&b.s));
        if pts.len() < 2 || pts.windows(2).any(|w| w[1].s <= w[0].s) {
            return None;
        }
        let s: Vec<f64> = pts.iter().map(|p| p.s).collect();
        // The angle unwrapped, as numpy's unwrap does.
        let mut psi = Vec::with_capacity(pts.len());
        for p in &pts {
            let v = match psi.last() {
                None => p.psi,
                Some(&prev) => prev + wrap(p.psi - prev),
            };
            psi.push(v);
        }
        let xs: Vec<f64> = pts.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.y).collect();
        let mut intervals = Vec::new();
        let mut start: Option<usize> = None;
        for i in 0..=pts.len() {
            let good = i < pts.len() && pts[i].ok;
            match (good, start) {
                (true, None) => start = Some(i),
                (false, Some(from)) => {
                    intervals.push((s[from], s[i - 1]));
                    start = None;
                }
                _ => {}
            }
        }
        Some(BranchEnd {
            x: Spline::new(&s, &xs),
            y: Spline::new(&s, &ys),
            psi: Spline::new(&s, &psi),
            kappa: pts.iter().map(|p| p.kappa).collect(),
            tail: pts.iter().map(|p| p.tail.unwrap_or(0.0)).collect(),
            cant: pts.iter().map(|p| p.cant_lds.unwrap_or(0.0)).collect(),
            s,
            intervals,
        })
    }

    /// The first and last toe station sampled.
    pub fn whole(&self) -> (f64, f64) {
        (self.s[0], self.s[self.s.len() - 1])
    }

    /// The branch end at toe station `s`; beyond the samples the end
    /// intervals carry on.
    pub fn at(&self, s: f64) -> End {
        let n = self.s.len();
        let i = self.s.partition_point(|&v| v <= s).saturating_sub(1).min(n - 2);
        let d = s - self.s[i];
        let u = (d / (self.s[i + 1] - self.s[i])).clamp(0.0, 1.0);
        let lin = |v: &[f64]| v[i] + (v[i + 1] - v[i]) * u;
        End {
            x: self.x.at(i, d),
            y: self.y.at(i, d),
            psi: self.psi.at(i, d),
            kappa: lin(&self.kappa),
            tail: lin(&self.tail),
            cant: lin(&self.cant),
        }
    }
}

/// An angle brought into [−π, π).
pub fn wrap(a: f64) -> f64 {
    (a + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spline_reproduces_a_cubic() {
        let s: Vec<f64> = (0..12).map(|i| i as f64 * 0.5 + if i % 3 == 0 { 0.1 } else { 0.0 }).collect();
        let f = |t: f64| 0.3 * t * t * t - 2.0 * t * t + t - 4.0;
        let v: Vec<f64> = s.iter().map(|&t| f(t)).collect();
        let sp = Spline::new(&s, &v);
        for i in 0..s.len() - 1 {
            let d = (s[i + 1] - s[i]) * 0.37;
            assert!((sp.at(i, d) - f(s[i] + d)).abs() < 1e-9);
        }
    }

    #[test]
    fn an_angle_is_unwrapped_across_the_half_turn() {
        let samples: Vec<Sample> = (0..5)
            .map(|i| Sample {
                s: i as f64,
                x: 0.0,
                y: 0.0,
                psi: wrap(3.1 + 0.02 * i as f64),
                kappa: 0.0,
                tail: None,
                cant_lds: None,
                ok: i != 2,
            })
            .collect();
        let end = BranchEnd::new(&samples).unwrap();
        assert!((end.at(3.5).psi - 3.17).abs() < 1e-9);
        assert_eq!(end.intervals, vec![(0.0, 1.0), (3.0, 4.0)]);
    }
}
