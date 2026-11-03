// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Least squares in a box: Levenberg–Marquardt with the Jacobian by forward
//! differences, its damping scaled by the Jacobian's own columns (scipy's
//! `x_scale="jac"`), each step clipped into the box. A handful of variables
//! and a few dozen residuals — dense and small, and written with indices as
//! the matrices are.

#![allow(clippy::needless_range_loop)]

/// Solves the n×n system `a x = b` by elimination with partial pivoting; None where it is singular.
pub fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-300 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            if f == 0.0 {
                continue;
            }
            for k in col..n {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let s: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - s) / a[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// The damped Gauss–Newton step, a variable on a bound that it would leave
/// the box by held there and the step solved again for the others — a
/// step only clipped would stall on the bound.
fn damped_step(jtj: &[Vec<f64>], jtr: &[f64], lambda: f64, x: &[f64], lo: &[f64], hi: &[f64]) -> Option<Vec<f64>> {
    let n = x.len();
    let mut held = vec![false; n];
    for _ in 0..=n {
        let free: Vec<usize> = (0..n).filter(|&i| !held[i]).collect();
        let mut step = vec![0.0; n];
        if !free.is_empty() {
            let m = free
                .iter()
                .map(|&a| {
                    free.iter()
                        .map(|&b| jtj[a][b] + if a == b { lambda * jtj[a][a].max(1e-12) } else { 0.0 })
                        .collect()
                })
                .collect();
            let sol = solve_dense(m, free.iter().map(|&a| -jtr[a]).collect())?;
            for (k, &i) in free.iter().enumerate() {
                step[i] = sol[k];
            }
        }
        let leaving: Vec<usize> = free
            .into_iter()
            .filter(|&i| (x[i] <= lo[i] && step[i] < 0.0) || (x[i] >= hi[i] && step[i] > 0.0))
            .collect();
        if leaving.is_empty() {
            return Some(step);
        }
        for i in leaving {
            held[i] = true;
        }
    }
    None
}

fn cost(r: &[f64]) -> f64 {
    r.iter().map(|v| v * v).sum::<f64>() / 2.0
}

/// The point in [lo, hi] that brings `f`'s residuals nearest to nought, from
/// `x0`, with at most `max_iter` steps. None where `f` gives no finite value.
pub fn least_squares(f: &dyn Fn(&[f64]) -> Vec<f64>, x0: &[f64], lo: &[f64], hi: &[f64], max_iter: usize) -> Option<Vec<f64>> {
    let n = x0.len();
    let mut x: Vec<f64> = (0..n).map(|i| x0[i].clamp(lo[i], hi[i])).collect();
    let mut r = f(&x);
    if r.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut c = cost(&r);
    let mut lambda = 1e-3;
    for _ in 0..max_iter {
        // The Jacobian, stepping into the box where a variable stands on its upper bound.
        let mut jac = vec![vec![0.0; n]; r.len()];
        for j in 0..n {
            let mut h = 1e-7 * x[j].abs().max(1.0);
            if x[j] + h > hi[j] {
                h = -h;
            }
            let mut xj = x.clone();
            xj[j] += h;
            let rj = f(&xj);
            for (row, (a, b)) in jac.iter_mut().zip(rj.iter().zip(&r)) {
                row[j] = (a - b) / h;
            }
        }
        let mut jtj = vec![vec![0.0; n]; n];
        let mut jtr = vec![0.0; n];
        for (row, ri) in jac.iter().zip(&r) {
            for a in 0..n {
                jtr[a] += row[a] * ri;
                for b in a..n {
                    jtj[a][b] += row[a] * row[b];
                }
            }
        }
        for a in 0..n {
            for b in 0..a {
                jtj[a][b] = jtj[b][a];
            }
        }
        if jtr.iter().map(|v| v.abs()).fold(0.0, f64::max) < 1e-14 {
            break;
        }
        let mut improved = false;
        for _ in 0..12 {
            let Some(step) = damped_step(&jtj, &jtr, lambda, &x, lo, hi) else {
                lambda *= 10.0;
                continue;
            };
            let xn: Vec<f64> = (0..n).map(|i| (x[i] + step[i]).clamp(lo[i], hi[i])).collect();
            let rn = f(&xn);
            let cn = if rn.iter().all(|v| v.is_finite()) { cost(&rn) } else { f64::INFINITY };
            if cn < c {
                let moved = (0..n).map(|i| (xn[i] - x[i]).abs() / x[i].abs().max(1.0)).fold(0.0, f64::max);
                let gain = (c - cn) / c.max(1e-300);
                x = xn;
                r = rn;
                c = cn;
                lambda = (lambda / 3.0).max(1e-12);
                improved = gain > 1e-12 && moved > 1e-12;
                break;
            }
            lambda *= 4.0;
        }
        if !improved {
            break;
        }
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rosenbrocks_minimum() {
        let f = |x: &[f64]| vec![10.0 * (x[1] - x[0] * x[0]), 1.0 - x[0]];
        let x = least_squares(&f, &[-1.2, 1.0], &[-5.0, -5.0], &[5.0, 5.0], 200).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-6 && (x[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn stays_in_its_box() {
        let f = |x: &[f64]| vec![x[0] - 3.0, x[1] + 2.0];
        let x = least_squares(&f, &[0.0, 0.0], &[-1.0, -1.0], &[1.0, 1.0], 50).unwrap();
        assert_eq!(x, vec![1.0, -1.0]);
    }
}
