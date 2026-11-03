// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The end of one element laid from a point on a tangent: a straight, an arc,
//! or a transition whose curvature runs linearly — the last integrated by
//! Gauss–Legendre over 16 points, which is exact to far below a micron over
//! the lengths a connection has.

use std::sync::OnceLock;

/// The Gauss–Legendre nodes and weights on [−1, 1].
fn gauss_legendre() -> &'static ([f64; 16], [f64; 16]) {
    static RULE: OnceLock<([f64; 16], [f64; 16])> = OnceLock::new();
    RULE.get_or_init(|| {
        const N: usize = 16;
        let mut x = [0.0; N];
        let mut w = [0.0; N];
        for i in 0..N / 2 {
            // Newton's method on P₁₆ from the Chebyshev guess.
            let mut z = (std::f64::consts::PI * (i as f64 + 0.75) / (N as f64 + 0.5)).cos();
            let mut dp = 0.0;
            for _ in 0..100 {
                let (mut p0, mut p1) = (1.0, 0.0);
                for j in 0..N {
                    let p2 = p1;
                    p1 = p0;
                    p0 = ((2 * j + 1) as f64 * z * p1 - j as f64 * p2) / (j + 1) as f64;
                }
                dp = N as f64 * (z * p0 - p1) / (z * z - 1.0);
                let step = p0 / dp;
                z -= step;
                if step.abs() < 1e-16 {
                    break;
                }
            }
            let weight = 2.0 / ((1.0 - z * z) * dp * dp);
            x[i] = -z;
            x[N - 1 - i] = z;
            w[i] = weight;
            w[N - 1 - i] = weight;
        }
        (x, w)
    })
}

/// The end (x, y, ψ) of a piece of curvature k0 → k1 over `length`, laid
/// from (x, y) on ψ.
pub fn piece_end(x: f64, y: f64, psi: f64, length: f64, k0: f64, k1: f64) -> (f64, f64, f64) {
    if (k1 - k0).abs() < 1e-12 {
        let turn = k0 * length;
        if turn.abs() < 1e-9 {
            return (x + length * psi.cos(), y + length * psi.sin(), psi);
        }
        return (x + ((psi + turn).sin() - psi.sin()) / k0, y - ((psi + turn).cos() - psi.cos()) / k0, psi + turn);
    }
    let (nodes, weights) = gauss_legendre();
    let half = length / 2.0;
    let (mut dx, mut dy) = (0.0, 0.0);
    for (&t, &w) in nodes.iter().zip(weights) {
        let u = (t + 1.0) * half;
        let theta = psi + k0 * u + (k1 - k0) * u * u / (2.0 * length);
        dx += w * half * theta.cos();
        dy += w * half * theta.sin();
    }
    (x + dx, y + dy, psi + (k0 + k1) / 2.0 * length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left_arc(x: f64, y: f64, psi: f64, k: f64, length: f64) -> (f64, f64, f64) {
        (
            x + ((psi + k * length).sin() - psi.sin()) / k,
            y - ((psi + k * length).cos() - psi.cos()) / k,
            psi + k * length,
        )
    }

    #[test]
    fn a_transition_of_no_change_is_the_arc() {
        let (x, y, psi) = piece_end(0.0, 0.0, 0.3, 40.0, 1.0 / 300.0, 1.0 / 300.0 + 2e-12);
        let (ax, ay, apsi) = left_arc(0.0, 0.0, 0.3, 1.0 / 300.0, 40.0);
        assert!((x - ax).hypot(y - ay) < 1e-9);
        assert!((psi - apsi).abs() < 1e-10);
    }

    #[test]
    fn a_transition_turns_by_its_mean_curvature_times_its_length() {
        let (_, _, psi) = piece_end(0.0, 0.0, 0.0, 60.0, 0.0, 1.0 / 300.0);
        assert!((psi - 0.1).abs() < 1e-12);
    }

    #[test]
    fn the_weights_sum_to_the_interval() {
        let (x, w) = gauss_legendre();
        assert!((w.iter().sum::<f64>() - 2.0).abs() < 1e-14);
        // x⁴ integrates to 2/5 over [−1, 1].
        let q: f64 = x.iter().zip(w).map(|(x, w)| w * x.powi(4)).sum();
        assert!((q - 0.4).abs() < 1e-14);
    }
}
