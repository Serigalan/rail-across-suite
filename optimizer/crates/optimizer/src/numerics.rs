// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The searches the Python service took from scipy, written as scipy has
//! them so that a run follows the same path: Nelder–Mead
//! (`minimize(method="Nelder-Mead")`), Brent's bounded search
//! (`minimize_scalar(method="bounded")`, after Forsythe, Malcolm and Moler)
//! and Differential Evolution (`differential_evolution`, best1bin with
//! dithering and a seeded generator of its own).

/// What Nelder–Mead is asked for.
#[derive(Debug, Clone)]
pub struct NelderMead {
    /// The simplex to start from; None: scipy's default around x0 (5 % of
    /// each coordinate, 0.00025 where it is zero).
    pub initial_simplex: Option<Vec<Vec<f64>>>,
    pub xatol: f64,
    pub fatol: f64,
    pub maxfev: usize,
    pub maxiter: Option<usize>,
}

impl Default for NelderMead {
    fn default() -> NelderMead {
        NelderMead {
            initial_simplex: None,
            xatol: 1e-4,
            fatol: 1e-4,
            maxfev: usize::MAX,
            maxiter: None,
        }
    }
}

/// The minimum Nelder–Mead found: where, and the value there.
#[derive(Debug, Clone)]
pub struct Minimum {
    pub x: Vec<f64>,
    pub fun: f64,
    pub nfev: usize,
}

fn sort_simplex(sim: &mut Vec<Vec<f64>>, fsim: &mut Vec<f64>) {
    let mut order: Vec<usize> = (0..fsim.len()).collect();
    order.sort_by(|&a, &b| fsim[a].total_cmp(&fsim[b]));
    *sim = order.iter().map(|&i| sim[i].clone()).collect();
    *fsim = order.iter().map(|&i| fsim[i]).collect();
}

/// Minimize `f` from `x0` by Nelder–Mead — scipy's `_minimize_neldermead`
/// without bounds and without the adaptive coefficients.
pub fn nelder_mead(mut f: impl FnMut(&[f64]) -> f64, x0: &[f64], opts: &NelderMead) -> Minimum {
    let (rho, chi, psi, sigma) = (1.0, 2.0, 0.5, 0.5);
    let n = x0.len();
    let maxiter = opts.maxiter.unwrap_or(n * 200);
    let mut sim: Vec<Vec<f64>> = match &opts.initial_simplex {
        Some(s) => s.clone(),
        None => {
            let mut s = vec![x0.to_vec()];
            for k in 0..n {
                let mut y = x0.to_vec();
                y[k] = if y[k] != 0.0 { 1.05 * y[k] } else { 0.00025 };
                s.push(y);
            }
            s
        }
    };
    let mut nfev = 0;
    let mut call = |x: &[f64], nfev: &mut usize| {
        *nfev += 1;
        f(x)
    };
    let mut fsim: Vec<f64> = Vec::with_capacity(n + 1);
    for x in &sim {
        if nfev >= opts.maxfev {
            fsim.push(f64::INFINITY);
            continue;
        }
        fsim.push(call(x, &mut nfev));
    }
    sort_simplex(&mut sim, &mut fsim);
    let combine = |a: f64, xa: &[f64], b: f64, xb: &[f64]| -> Vec<f64> { xa.iter().zip(xb).map(|(p, q)| a * p + b * q).collect() };

    let mut iterations = 1;
    while nfev < opts.maxfev && iterations < maxiter {
        let x_spread = sim[1..]
            .iter()
            .flat_map(|x| x.iter().zip(&sim[0]).map(|(a, b)| (a - b).abs()))
            .fold(0.0, f64::max);
        let f_spread = fsim[1..].iter().map(|v| (fsim[0] - v).abs()).fold(0.0, f64::max);
        if x_spread <= opts.xatol && f_spread <= opts.fatol {
            break;
        }
        let mut xbar = vec![0.0; n];
        for x in &sim[..n] {
            for (s, v) in xbar.iter_mut().zip(x) {
                *s += v / n as f64;
            }
        }
        let worst = sim[n].clone();
        let xr = combine(1.0 + rho, &xbar, -rho, &worst);
        let fxr = call(&xr, &mut nfev);
        let mut shrink = false;
        if fxr < fsim[0] {
            let xe = combine(1.0 + rho * chi, &xbar, -rho * chi, &worst);
            let fxe = call(&xe, &mut nfev);
            if fxe < fxr {
                sim[n] = xe;
                fsim[n] = fxe;
            } else {
                sim[n] = xr;
                fsim[n] = fxr;
            }
        } else if fxr < fsim[n - 1] {
            sim[n] = xr;
            fsim[n] = fxr;
        } else if fxr < fsim[n] {
            let xc = combine(1.0 + psi * rho, &xbar, -psi * rho, &worst);
            let fxc = call(&xc, &mut nfev);
            if fxc <= fxr {
                sim[n] = xc;
                fsim[n] = fxc;
            } else {
                shrink = true;
            }
        } else {
            let xcc = combine(1.0 - psi, &xbar, psi, &worst);
            let fxcc = call(&xcc, &mut nfev);
            if fxcc < fsim[n] {
                sim[n] = xcc;
                fsim[n] = fxcc;
            } else {
                shrink = true;
            }
        }
        if shrink {
            for j in 1..=n {
                sim[j] = combine(1.0 - sigma, &sim[0].clone(), sigma, &sim[j]);
                fsim[j] = call(&sim[j].clone(), &mut nfev);
            }
        }
        iterations += 1;
        sort_simplex(&mut sim, &mut fsim);
    }
    Minimum {
        x: sim[0].clone(),
        fun: fsim[0],
        nfev,
    }
}

/// The minimum of `f` on [x1, x2] by Brent's bounded search — scipy's
/// `_minimize_scalar_bounded`: golden sections and parabolic steps, to
/// `xatol`, at most `maxfun` evaluations.
pub fn fminbound(mut f: impl FnMut(f64) -> f64, x1: f64, x2: f64, xatol: f64, maxfun: usize) -> f64 {
    let sqrt_eps = 2.2e-16_f64.sqrt();
    let golden_mean = 0.5 * (3.0 - 5.0_f64.sqrt());
    let (mut a, mut b) = (x1, x2);
    let mut fulc = a + golden_mean * (b - a);
    let (mut nfc, mut xf) = (fulc, fulc);
    let (mut rat, mut e) = (0.0_f64, 0.0_f64);
    let mut fx = f(xf);
    let mut num = 1;
    let (mut ffulc, mut fnfc) = (fx, fx);
    let mut xm = 0.5 * (a + b);
    let mut tol1 = sqrt_eps * xf.abs() + xatol / 3.0;
    let mut tol2 = 2.0 * tol1;
    let sign = |v: f64| {
        if v > 0.0 {
            1.0
        } else if v < 0.0 {
            -1.0
        } else {
            0.0
        }
    };

    while (xf - xm).abs() > tol2 - 0.5 * (b - a) {
        let mut golden = true;
        if e.abs() > tol1 {
            golden = false;
            let mut r = (xf - nfc) * (fx - ffulc);
            let mut q = (xf - fulc) * (fx - fnfc);
            let mut p = (xf - fulc) * q - (xf - nfc) * r;
            q = 2.0 * (q - r);
            if q > 0.0 {
                p = -p;
            }
            q = q.abs();
            r = e;
            e = rat;
            if p.abs() < (0.5 * q * r).abs() && p > q * (a - xf) && p < q * (b - xf) {
                rat = p / q;
                let x = xf + rat;
                if (x - a) < tol2 || (b - x) < tol2 {
                    let si = sign(xm - xf) + if xm - xf == 0.0 { 1.0 } else { 0.0 };
                    rat = tol1 * si;
                }
            } else {
                golden = true;
            }
        }
        if golden {
            e = if xf >= xm { a - xf } else { b - xf };
            rat = golden_mean * e;
        }
        let si = sign(rat) + if rat == 0.0 { 1.0 } else { 0.0 };
        let x = xf + si * rat.abs().max(tol1);
        let fu = f(x);
        num += 1;
        if fu <= fx {
            if x >= xf {
                a = xf;
            } else {
                b = xf;
            }
            (fulc, ffulc) = (nfc, fnfc);
            (nfc, fnfc) = (xf, fx);
            (xf, fx) = (x, fu);
        } else {
            if x < xf {
                a = x;
            } else {
                b = x;
            }
            if fu <= fnfc || nfc == xf {
                (fulc, ffulc) = (nfc, fnfc);
                (nfc, fnfc) = (x, fu);
            } else if fu <= ffulc || fulc == xf || fulc == nfc {
                (fulc, ffulc) = (x, fu);
            }
        }
        xm = 0.5 * (a + b);
        tol1 = sqrt_eps * xf.abs() + xatol / 3.0;
        tol2 = 2.0 * tol1;
        if num >= maxfun {
            break;
        }
    }
    xf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nelder_mead_finds_the_rosenbrock_valley() {
        let rosen = |x: &[f64]| 100.0 * (x[1] - x[0] * x[0]).powi(2) + (1.0 - x[0]).powi(2);
        let m = nelder_mead(
            rosen,
            &[-1.2, 1.0],
            &NelderMead {
                xatol: 1e-8,
                fatol: 1e-8,
                ..NelderMead::default()
            },
        );
        assert!((m.x[0] - 1.0).abs() < 1e-4 && (m.x[1] - 1.0).abs() < 1e-4, "{:?}", m.x);
    }

    #[test]
    fn fminbound_finds_a_bounded_minimum() {
        let x = fminbound(|x| (x - 2.0).powi(2) + 1.0, 0.0, 5.0, 1e-5, 500);
        assert!((x - 2.0).abs() < 1e-4, "{x}");
        // At the bound when the minimum lies beyond it.
        let x = fminbound(|x| x, 1.0, 3.0, 1e-5, 500);
        assert!((x - 1.0).abs() < 1e-3, "{x}");
    }
}
