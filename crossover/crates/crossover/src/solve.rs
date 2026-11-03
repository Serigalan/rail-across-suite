// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The search: per sequence of middle elements and per start, least squares
//! finds a point that closes within the limits (robust from a start that is
//! neither), SLSQP then moves it along what closes to the best place by the
//! objective. Among what closes, per sequence the one the ranking puts first.

use crate::branch::BranchEnd;
use crate::lm::least_squares;
use crate::problem::{Field, K_MAX, K_SCALE, Kind, MAX_LENGTH, Problem, ROOM, SAME_K, Template};
use serde::Serialize;
use std::cell::{Cell, RefCell};

/// How close a solution has to close [m, rad] — the app closes it to 1e-8 m again.
const CLOSE_POS: f64 = 1e-4;
const CLOSE_ANGLE: f64 = 1e-6;
/// The closure as SLSQP is held to it: within this either way [m, and m for
/// the direction weighed by the chain's length].
const CLOSE_EPS: f64 = 1e-6;
/// Steps of the first phase, evaluations of the second.
const LM_STEPS: usize = 100;
const SLSQP_EVALS: usize = 200;

/// One element of an answer: its kind, its length and an arc's curvature (math-signed).
#[derive(Debug, Clone, Serialize)]
pub struct PieceOut {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub length: f64,
    pub kappa: Option<f64>,
}

/// One answer: where both toes stand, the elements between, and what of
/// LP.KS.01 it keeps only at the Ermessensgrenze.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub form: usize,
    pub label: Option<String>,
    pub label2: Option<String>,
    pub template: Vec<&'static str>,
    pub s1: f64,
    pub s2: f64,
    pub pieces: Vec<PieceOut>,
    /// The comparison radii under the Regelwert [m].
    pub hints: Vec<f64>,
    pub rw_min: Option<f64>,
    pub score: f64,
}

/// What puts one answer before another, at `nearest` the distance of the
/// nearest of them from the click: nearer the click first, counted in whole
/// metres from that one — a few centimetres nearer is not worth a joint under
/// the Regelwert —, then one that keeps the Regelwert, then fewer elements,
/// then the objective.
fn rank(sol: &Solution, nearest: f64) -> (i64, bool, usize, f64) {
    (
        (sol.s1.abs() - nearest).max(0.0).floor() as i64,
        !sol.hints.is_empty(),
        sol.pieces.len(),
        sol.score,
    )
}

fn rank_cmp_at(a: &Solution, b: &Solution, nearest: f64) -> std::cmp::Ordering {
    let (ra, rb) = (rank(a, nearest), rank(b, nearest));
    ra.0.cmp(&rb.0).then(ra.1.cmp(&rb.1)).then(ra.2.cmp(&rb.2)).then(ra.3.total_cmp(&rb.3))
}

/// Is `a` the better of two answers of one sequence?
pub fn better(a: &Solution, b: &Solution) -> bool {
    rank_cmp_at(a, b, a.s1.abs().min(b.s1.abs())).is_lt()
}

/// The answers best first.
pub fn order(sols: &mut [Solution]) {
    let nearest = sols.iter().map(|s| s.s1.abs()).fold(f64::INFINITY, f64::min);
    sols.sort_by(|a, b| rank_cmp_at(a, b, nearest));
}

/// Where the second toe starts: the station whose branch end lies on the ray
/// an arc from B1E would end along — the dialog's closed construction.
pub fn guess_s2(problem: &Problem, s1: f64, interval: (f64, f64)) -> Option<f64> {
    let b1 = problem.first.at(s1);
    let n = (((interval.1 - interval.0) / 0.5) as usize + 1).max(2);
    let mut best: Option<(f64, f64)> = None;
    for i in 0..n {
        let s2 = interval.0 + (interval.1 - interval.0) * i as f64 / (n - 1) as f64;
        let b2 = problem.second.at(s2);
        let (dx, dy) = (b2.x - b1.x, b2.y - b1.y);
        let mid = b1.psi + crate::branch::wrap(b2.psi - b1.psi) / 2.0;
        if dx * mid.cos() + dy * mid.sin() <= 0.0 {
            continue;
        }
        let f = (-dx * mid.sin() + dy * mid.cos()).abs();
        if best.is_none_or(|(_, bf)| f < bf) {
            best = Some((s2, f));
        }
    }
    best.map(|(s2, _)| s2)
}

/// The vectors the search starts from at the toes (s1, s2): every element of
/// an even share of the chord, the arcs turning it — and for reverse curves
/// an S flatter or steeper than the branches, for one arc a straight.
pub fn starts(tpl: &Template, problem: &Problem, s1: f64, s2: f64) -> Vec<Vec<f64>> {
    let b1 = problem.first.at(s1);
    let b2 = problem.second.at(s2);
    let chord = (b2.x - b1.x).hypot(b2.y - b1.y);
    let turn = crate::branch::wrap(b2.psi - b1.psi);
    let each = (chord / tpl.kinds.len() as f64).max(ROOM * 10.0);
    let k_turn = turn / chord.max(1.0);
    let arcs: Vec<usize> = tpl.kinds.iter().enumerate().filter(|(_, k)| **k == Kind::Arc).map(|(i, _)| i).collect();
    let gentle = 1.0 / 1000.0;
    let mut patterns: Vec<Vec<(usize, f64)>> = vec![arcs.iter().map(|&i| (i, k_turn)).collect()];
    if arcs.len() >= 2 {
        for sign in [1.0, -1.0] {
            patterns.push(
                arcs.iter()
                    .enumerate()
                    .map(|(j, &i)| (i, if j % 2 == 0 { sign } else { -sign } * gentle + k_turn))
                    .collect(),
            );
        }
    } else if !arcs.is_empty() {
        patterns.push(arcs.iter().map(|&i| (i, 0.0)).collect());
    }
    patterns
        .into_iter()
        .map(|pattern| {
            let mut z = vec![s1, s2];
            for (j, &(i, field)) in tpl.slots.iter().enumerate() {
                z.push(match (tpl.fixed[j], field) {
                    (Some(v), _) => v,
                    (None, Field::Length) => each,
                    (None, Field::K) => pattern.iter().find(|(a, _)| *a == i).map_or(0.0, |(_, k)| k * K_SCALE),
                });
            }
            z
        })
        .collect()
}

/// Does z close the connection and keep every limit?
pub fn feasible(tpl: &Template, problem: &Problem, z: &[f64]) -> bool {
    let st = problem.state(tpl, z);
    let r = problem.residual(&st);
    let scale = st.pieces.iter().map(|p| p.0.abs()).sum::<f64>().max(10.0);
    if r[0].hypot(r[1]) > CLOSE_POS || r[2].abs() / scale > CLOSE_ANGLE {
        return false;
    }
    let (g, _) = problem.inequalities(tpl, &st);
    g.iter().all(|&v| v >= -1e-7)
}

/// The objective, the constraints as SLSQP takes them (≤ 0: the closure
/// either way within CLOSE_EPS, every limit) and their derivatives, by
/// forward differences from one evaluation per variable.
struct Bundle {
    f: f64,
    df: Vec<f64>,
    c: Vec<f64>,
    dc: Vec<Vec<f64>>,
}

fn constraints(tpl: &Template, problem: &Problem, z: &[f64]) -> (f64, Vec<f64>) {
    let st = problem.state(tpl, z);
    let f = problem.objective(tpl, z, &st);
    let r = problem.residual(&st);
    let (g, _) = problem.inequalities(tpl, &st);
    let mut c = Vec::with_capacity(6 + g.len());
    for v in r {
        c.push(v - CLOSE_EPS);
        c.push(-v - CLOSE_EPS);
    }
    c.extend(g.iter().map(|v| -v));
    (f, c)
}

/// A constraint as SLSQP calls it: its value at a point, its gradient where asked for.
type Constraint<'a> = dyn Fn(&[f64], Option<&mut [f64]>, &mut ()) -> f64 + 'a;

/// The variables a search moves (their box has room) and the full vector from them.
struct Reduced {
    lo: Vec<f64>,
    hi: Vec<f64>,
    free: Vec<usize>,
}

impl Reduced {
    fn full(&self, v: &[f64]) -> Vec<f64> {
        let mut z = self.lo.clone();
        for (k, &i) in self.free.iter().enumerate() {
            z[i] = v[k];
        }
        z
    }
}

/// One start of one sequence: the polished point where it holds, else the
/// first phase's, else None. `evals` counts what the search has spent.
pub fn solve_one(tpl: &Template, problem: &Problem, i1: (f64, f64), i2: (f64, f64), start: &[f64], evals: &Cell<usize>) -> Option<Vec<f64>> {
    let mut bounds = vec![i1, i2];
    for (j, &(_, field)) in tpl.slots.iter().enumerate() {
        bounds.push(match (tpl.fixed[j], field) {
            (Some(v), _) => (v, v),
            (None, Field::Length) => (ROOM, MAX_LENGTH),
            (None, Field::K) => (-K_MAX * K_SCALE, K_MAX * K_SCALE),
        });
    }
    let lo: Vec<f64> = bounds.iter().map(|b| b.0).collect();
    let hi: Vec<f64> = bounds.iter().map(|b| b.1).collect();
    let free: Vec<usize> = (0..lo.len()).filter(|&i| hi[i] - lo[i] >= 1e-9).collect();
    if free.is_empty() {
        return None;
    }
    let red = Reduced { lo, hi, free };
    let vlo: Vec<f64> = red.free.iter().map(|&i| red.lo[i]).collect();
    let vhi: Vec<f64> = red.free.iter().map(|&i| red.hi[i]).collect();
    let v0: Vec<f64> = red.free.iter().map(|&i| start[i].clamp(red.lo[i], red.hi[i])).collect();

    let penalty = |v: &[f64]| {
        evals.set(evals.get() + 1);
        problem.penalty(tpl, &red.full(v))
    };
    let v1 = least_squares(&penalty, &v0, &vlo, &vhi, LM_STEPS)?;
    let z1 = red.full(&v1);
    let st = problem.state(tpl, &z1);
    let r = problem.residual(&st);
    if r[0].hypot(r[1]) > 1e-2 {
        return None;
    }

    // The second phase shares every evaluation between the objective and
    // the constraints: one bundle per point, kept until the point changes.
    let cache: RefCell<Option<(Vec<f64>, Bundle)>> = RefCell::new(None);
    let bundle_at = |v: &[f64]| {
        let mut slot = cache.borrow_mut();
        if slot.as_ref().is_some_and(|(at, _)| at.as_slice() == v) {
            return;
        }
        let z = red.full(v);
        let (f0, c0) = constraints(tpl, problem, &z);
        let n = v.len();
        let mut df = vec![0.0; n];
        let mut dc = vec![vec![0.0; n]; c0.len()];
        for k in 0..n {
            let mut h = 1e-7 * v[k].abs().max(1.0);
            if v[k] + h > vhi[k] {
                h = -h;
            }
            let mut vk = v.to_vec();
            vk[k] += h;
            let (f1, c1) = constraints(tpl, problem, &red.full(&vk));
            df[k] = (f1 - f0) / h;
            for (row, (a, b)) in dc.iter_mut().zip(c1.iter().zip(&c0)) {
                row[k] = (a - b) / h;
            }
        }
        evals.set(evals.get() + n + 1);
        *slot = Some((v.to_vec(), Bundle { f: f0, df, c: c0, dc }));
    };
    let objective = |v: &[f64], grad: Option<&mut [f64]>, _: &mut ()| {
        bundle_at(v);
        let slot = cache.borrow();
        let b = &slot.as_ref().unwrap().1;
        if let Some(g) = grad {
            g.copy_from_slice(&b.df);
        }
        b.f
    };
    let n_cons = constraints(tpl, problem, &z1).1.len();
    let cons_fns: Vec<Box<Constraint>> = (0..n_cons)
        .map(|i| {
            let bundle_at = &bundle_at;
            let cache = &cache;
            Box::new(move |v: &[f64], grad: Option<&mut [f64]>, _: &mut ()| {
                bundle_at(v);
                let slot = cache.borrow();
                let b = &slot.as_ref().unwrap().1;
                if let Some(g) = grad {
                    g.copy_from_slice(&b.dc[i]);
                }
                b.c[i]
            }) as Box<Constraint>
        })
        .collect();
    let cons: Vec<&Constraint> = cons_fns.iter().map(|f| f.as_ref()).collect();
    let box_bounds: Vec<(f64, f64)> = vlo.iter().zip(&vhi).map(|(&a, &b)| (a, b)).collect();
    let tol = slsqp::StopTols {
        ftol_abs: 1e-10,
        ..Default::default()
    };
    let polished = match slsqp::minimize(objective, &v1, &box_bounds, &cons, (), SLSQP_EVALS, Some(tol)) {
        Ok((_, v, _)) => Some(v),
        Err((_, v, _)) => Some(v),
    };
    if let Some(v) = polished {
        let z = red.full(&v);
        if z.iter().all(|x| x.is_finite()) && feasible(tpl, problem, &z) {
            return Some(z);
        }
    }
    feasible(tpl, problem, &z1).then_some(z1)
}

/// The answer for the variables z.
pub fn answer(tpl: &Template, problem: &Problem, z: &[f64], form: usize, label: Option<String>, label2: Option<String>) -> Solution {
    let st = problem.state(tpl, z);
    let mut hints = Vec::new();
    let mut rw_min: Option<f64> = None;
    for dk in problem.joints(tpl, &st) {
        if dk.abs() > SAME_K {
            let rw = 1.0 / dk.abs();
            rw_min = Some(rw_min.map_or(rw, |m| m.min(rw)));
            if problem.k_reg.is_some_and(|k| dk.abs() > k + 1e-12) {
                hints.push(rw);
            }
        }
    }
    let pieces = st
        .pieces
        .iter()
        .zip(&tpl.kinds)
        .map(|(&(length, k0, _), &kind)| PieceOut {
            kind: kind.name(),
            length,
            kappa: (kind == Kind::Arc).then_some(k0),
        })
        .collect();
    Solution {
        form,
        label,
        label2,
        template: tpl.kinds.iter().map(|k| k.name()).collect(),
        s1: z[0],
        s2: z[1],
        pieces,
        hints,
        rw_min,
        score: problem.objective(tpl, z, &st),
    }
}

/// A start of the search: the stretches either toe may stand on, and where in them it starts.
pub type Start = ((f64, f64), (f64, f64), f64, f64);

/// Where the search starts for one form: each stretch the first turnout may
/// stand on, nearest the click first, its toe as near the click as it
/// allows, and the stretch of the second turnout that holds the closed
/// construction's s2 there (or lies nearest to it).
pub fn search_starts(problem: &Problem, first: &BranchEnd, second: &BranchEnd) -> Vec<Start> {
    let whole = second.whole();
    let mut ones = first.intervals.clone();
    ones.sort_by(|a, b| a.0.max(-a.1).max(0.0).total_cmp(&b.0.max(-b.1).max(0.0)));
    let mut out = Vec::new();
    for i1 in ones {
        let s1 = 0.0f64.max(i1.0).min(i1.1);
        let Some(s2) = guess_s2(problem, s1, whole) else { continue };
        let Some(&i2) = second
            .intervals
            .iter()
            .min_by(|a, b| (a.0 - s2).max(s2 - a.1).max(0.0).total_cmp(&(b.0 - s2).max(s2 - b.1).max(0.0)))
        else {
            continue;
        };
        out.push((i1, i2, s1, s2.clamp(i2.0, i2.1)));
    }
    out
}
