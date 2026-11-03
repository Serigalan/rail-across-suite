// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The connection between two branch ends under the dialog's limits: the
//! chain of middle elements a vector of variables gives, how far it misses
//! the second branch, what it keeps of the limits, and what makes one answer
//! better than another.

use crate::branch::{BranchEnd, End, wrap};
use crate::piece::piece_end;
use serde::Deserialize;

/// Shorter than this an element is none [m] (the app's MID_ROOM).
pub const ROOM: f64 = 1e-3;
/// Curvatures this close are one [1/m] (the app's SAME_CURVATURE).
pub const SAME_K: f64 = 1e-6;
/// An arc's curvature is a variable in 1/km, so it weighs like a length.
pub const K_SCALE: f64 = 1e3;
/// The sharpest arc searched [1/m].
pub const K_MAX: f64 = 1.0 / 50.0;
/// Margins kept inside the limits, so the app's own rounding — and the values
/// it rounds when it adopts an answer — stay on the right side.
const CANT_DEF_MARGIN: f64 = 0.5;
const CANT_TOL_MARGIN: f64 = 0.05;
const LENGTH_MARGIN: f64 = 0.01;
const RW_MARGIN: f64 = 1e-3;
/// The middle elements together at most this long [m]: past it a connection
/// is a stretch of new line, not a crossover.
pub const MAX_LENGTH: f64 = 200.0;

/// What a middle element is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Straight,
    Arc,
    Transition,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Straight => "straight",
            Kind::Arc => "arc",
            Kind::Transition => "transition",
        }
    }
}

/// Which value of an element a variable is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Length,
    K,
}

/// One sequence of middle elements and its variables: s1, s2, then per
/// element its length and, for an arc, its curvature in 1/km — each either
/// sought or given (`fixed`, in the same units).
#[derive(Debug, Clone)]
pub struct Template {
    pub kinds: Vec<Kind>,
    pub slots: Vec<(usize, Field)>,
    pub fixed: Vec<Option<f64>>,
}

impl Template {
    pub fn new(kinds: &[Kind]) -> Template {
        let mut slots = Vec::new();
        for (i, &kind) in kinds.iter().enumerate() {
            slots.push((i, Field::Length));
            if kind == Kind::Arc {
                slots.push((i, Field::K));
            }
        }
        let fixed = vec![None; slots.len()];
        Template {
            kinds: kinds.to_vec(),
            slots,
            fixed,
        }
    }

    /// The template with the value of element `i`'s `field` given.
    pub fn with_fixed(mut self, i: usize, field: Field, value: f64) -> Template {
        if let Some(j) = self.slots.iter().position(|&slot| slot == (i, field)) {
            self.fixed[j] = Some(value);
        }
        self
    }

    /// The elements as (length, k0, k1) for the variables z, math-signed: a
    /// transition runs between the curvatures of its neighbours, the
    /// branches' at either end.
    pub fn pieces(&self, z: &[f64], k_start: f64, k_end: f64) -> Vec<(f64, f64, f64)> {
        let n = self.kinds.len();
        let mut length = vec![0.0; n];
        let mut k = vec![0.0; n];
        for (j, &(i, field)) in self.slots.iter().enumerate() {
            match field {
                Field::Length => length[i] = z[2 + j],
                Field::K => k[i] = z[2 + j] / K_SCALE,
            }
        }
        let k_of = |j: isize| -> f64 {
            if j < 0 {
                k_start
            } else if j as usize >= n {
                k_end
            } else if self.kinds[j as usize] == Kind::Transition {
                0.0
            } else {
                k[j as usize]
            }
        };
        (0..n)
            .map(|i| match self.kinds[i] {
                Kind::Transition => (length[i], k_of(i as isize - 1), k_of(i as isize + 1)),
                _ => (length[i], k[i], k[i]),
            })
            .collect()
    }
}

/// The limits the dialog holds the middle to, as the request states them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub l_min: Option<f64>,
    pub rw_reg: Option<f64>,
    pub rw_discretion: Option<f64>,
    pub max_cant_def: Option<f64>,
    pub cant_def_coeff: Option<f64>,
    pub cant_tol: Option<f64>,
    pub max_length: Option<f64>,
}

/// What a vector of variables gives: both branch ends, the elements and where the chain ends.
pub struct State {
    pub b1: End,
    pub b2: End,
    pub pieces: Vec<(f64, f64, f64)>,
    pub end: (f64, f64, f64),
}

pub struct Problem<'a> {
    pub first: &'a BranchEnd,
    pub second: &'a BranchEnd,
    speed: f64,
    l_min: f64,
    pub k_reg: Option<f64>,
    k_disc: Option<f64>,
    max_def: f64,
    def_coeff: f64,
    cant_tol: f64,
    max_length: f64,
}

fn positive(v: Option<f64>) -> Option<f64> {
    v.filter(|v| v.is_finite() && *v > 0.0)
}

impl<'a> Problem<'a> {
    pub fn new(first: &'a BranchEnd, second: &'a BranchEnd, limits: &Limits, speed: f64) -> Problem<'a> {
        Problem {
            first,
            second,
            speed,
            l_min: positive(limits.l_min).unwrap_or(0.0),
            k_reg: positive(limits.rw_reg).map(|r| 1.0 / r),
            k_disc: positive(limits.rw_discretion).map(|r| 1.0 / (r * (1.0 + RW_MARGIN))),
            max_def: positive(limits.max_cant_def).unwrap_or(0.0),
            def_coeff: positive(limits.cant_def_coeff).unwrap_or(11.8),
            cant_tol: positive(limits.cant_tol).unwrap_or(1.0),
            max_length: positive(limits.max_length).unwrap_or(MAX_LENGTH),
        }
    }

    pub fn state(&self, tpl: &Template, z: &[f64]) -> State {
        let b1 = self.first.at(z[0]);
        let b2 = self.second.at(z[1]);
        let pieces = tpl.pieces(z, b1.kappa, b2.kappa);
        let mut end = (b1.x, b1.y, b1.psi);
        for &(length, k0, k1) in &pieces {
            end = piece_end(end.0, end.1, end.2, length.max(ROOM), k0, k1);
        }
        State { b1, b2, pieces, end }
    }

    /// The miss at the second branch: position [m] and direction, the latter
    /// weighed by the chain's length.
    pub fn residual(&self, st: &State) -> [f64; 3] {
        let scale = st.pieces.iter().map(|p| p.0.abs()).sum::<f64>().max(10.0);
        let (x, y, psi) = st.end;
        [x - st.b2.x, y - st.b2.y, wrap(psi - st.b2.psi) * scale]
    }

    /// The jumps in curvature at the joints of the middle elements (signed Δκ);
    /// a joint with a transition has none.
    pub fn joints(&self, tpl: &Template, st: &State) -> Vec<f64> {
        let n = st.pieces.len();
        let is_transition = |j: usize| j >= 1 && j <= n && tpl.kinds[j - 1] == Kind::Transition;
        let ks = |j: usize| -> (f64, f64) {
            if j == 0 {
                (st.b1.kappa, st.b1.kappa)
            } else if j == n + 1 {
                (st.b2.kappa, st.b2.kappa)
            } else {
                (st.pieces[j - 1].1, st.pieces[j - 1].2)
            }
        };
        (0..=n)
            .filter(|&j| !is_transition(j) && !is_transition(j + 1))
            .map(|j| ks(j + 1).0 - ks(j).1)
            .collect()
    }

    /// Each element's length as LP.EL.01 reads it: a straight at either end
    /// runs on into a straight end piece of the branch.
    fn runs(&self, tpl: &Template, st: &State) -> Vec<f64> {
        let n = st.pieces.len();
        st.pieces
            .iter()
            .enumerate()
            .map(|(i, &(length, _, _))| {
                let mut run = length;
                if tpl.kinds[i] == Kind::Straight {
                    if i == 0 && st.b1.kappa.abs() < SAME_K {
                        run += st.b1.tail;
                    }
                    if i == n - 1 && st.b2.kappa.abs() < SAME_K {
                        run += st.b2.tail;
                    }
                }
                run
            })
            .collect()
    }

    /// The cant deficiency of curvature k (math-signed) at the signed cant —
    /// the app's computeCantDefSigned with the project radius −1/k.
    fn deficiency(&self, k: f64, cant: f64) -> f64 {
        self.def_coeff * self.speed * self.speed * k.abs() + cant * (k * 1e5).tanh()
    }

    /// The limits as values ≥ 0, each with the weight that brings it to about
    /// one per unit that matters (a metre, a curvature step of 1e-4/m, 10 mm
    /// of deficiency, a millimetre of cant).
    pub fn inequalities(&self, tpl: &Template, st: &State) -> (Vec<f64>, Vec<f64>) {
        let (mut g, mut w) = (Vec::new(), Vec::new());
        if self.l_min > 0.0 {
            for run in self.runs(tpl, st) {
                g.push(run - self.l_min - LENGTH_MARGIN);
                w.push(1.0);
            }
        }
        if let Some(k_disc) = self.k_disc {
            for dk in self.joints(tpl, st) {
                g.extend([k_disc - dk, k_disc + dk]);
                w.extend([1e4, 1e4]);
            }
        }
        let cant = (st.b1.cant + st.b2.cant) / 2.0;
        if self.max_def > 0.0 {
            for &(_, k0, k1) in &st.pieces {
                for k in [k0, k1] {
                    g.push(self.max_def - CANT_DEF_MARGIN - self.deficiency(k, cant));
                    w.push(0.1);
                }
            }
        }
        let diff = st.b1.cant - st.b2.cant;
        g.extend([self.cant_tol - CANT_TOL_MARGIN - diff, self.cant_tol - CANT_TOL_MARGIN + diff]);
        w.extend([1.0, 1.0]);
        g.push(self.max_length - st.pieces.iter().map(|p| p.0).sum::<f64>());
        w.push(0.1);
        (g, w)
    }

    /// ∫κ² over the elements.
    fn bend(st: &State) -> f64 {
        st.pieces.iter().map(|&(length, k0, k1)| (k0 * k0 + k0 * k1 + k1 * k1) / 3.0 * length).sum()
    }

    /// The first phase as least squares: the miss at the second branch, what
    /// the limits are short of (with a margin), and weakly, the wish to stand
    /// near the click on gentle curves.
    pub fn penalty(&self, tpl: &Template, z: &[f64]) -> Vec<f64> {
        let st = self.state(tpl, z);
        let mut out: Vec<f64> = self.residual(&st).iter().map(|r| r * 100.0).collect();
        let (g, w) = self.inequalities(tpl, &st);
        out.extend(g.iter().zip(&w).map(|(g, w)| (0.01 - g * w).max(0.0) * 10.0));
        out.push(z[0] / 500.0);
        out.push((Self::bend(&st).max(0.0) * 10.0).sqrt());
        out
    }

    /// What the second phase minimises along what closes: near the click, on
    /// gentle curves, short — and a jump under the Regelwert only where it must.
    pub fn objective(&self, tpl: &Template, z: &[f64], st: &State) -> f64 {
        let total: f64 = st.pieces.iter().map(|p| p.0).sum();
        let mut out = (z[0] / 50.0).powi(2) + 1e3 * Self::bend(st) + total / 1000.0;
        if let Some(k_reg) = self.k_reg {
            for dk in self.joints(tpl, st) {
                let over = dk.abs() - k_reg;
                if over > 0.0 {
                    out += 1e8 * over * over;
                }
            }
        }
        out
    }
}
