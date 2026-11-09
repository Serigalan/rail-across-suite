// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The elements between the two branches of a switch connection (`midChain.js`).

use rail_across_geom::element::Utm;
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{curvature_of, radius_of_curvature};
use rail_across_js::number;
use serde_json::{Value, json};

use crate::catalogue::STRAIGHT_CURVATURE;
use crate::route::{Route, switch_chain_bearing_at, switch_chain_point_utm};

/// A middle element as the dialog states it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mid {
    pub kind: MidKind,
    pub length: Option<f64>,
    pub radius: Option<f64>,
    pub side: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MidKind {
    Straight,
    Arc,
    Transition,
}

impl Mid {
    pub fn from_json(v: &Value) -> Mid {
        let kind = match v.get("type").and_then(Value::as_str) {
            Some("straight") => MidKind::Straight,
            Some("arc") => MidKind::Arc,
            _ => MidKind::Transition,
        };
        let f = |k: &str| v.get(k).and_then(Value::as_f64);
        Mid {
            kind,
            length: f("length"),
            radius: f("radius"),
            side: f("side"),
        }
    }
    pub fn to_json(self) -> Value {
        let t = match self.kind {
            MidKind::Straight => "straight",
            MidKind::Arc => "arc",
            MidKind::Transition => "transition",
        };
        let mut o = json!({ "type": t, "length": self.length.map(number), "radius": self.radius.map(number) });
        if let Some(s) = self.side {
            o["side"] = number(s);
        }
        o
    }
}

pub fn mids_from(v: &Value) -> Vec<Mid> {
    v.as_array().map(|a| a.iter().map(Mid::from_json).collect()).unwrap_or_default()
}

/// The one element the connection always had (`DEFAULT_MIDS`).
pub const DEFAULT_MIDS: [Mid; 1] = [Mid {
    kind: MidKind::Arc,
    length: None,
    radius: None,
    side: None,
}];

/// Is this the connection as it always was (`isDefaultMids`)?
pub fn is_default_mids(mids: &[Mid]) -> bool {
    mids.is_empty() || (mids.len() == 1 && mids[0].kind == MidKind::Arc && mids[0].length.is_none() && mids[0].radius.is_none())
}

/// An open value: the element and whether it is its length (else its curvature).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unknown {
    pub i: usize,
    pub length: bool,
}

/// The values a spec leaves open (`midUnknowns`).
pub fn mid_unknowns(mids: &[Mid]) -> Vec<Unknown> {
    let mut out = Vec::new();
    for (i, m) in mids.iter().enumerate() {
        if m.length.is_none() {
            out.push(Unknown { i, length: true });
        }
        if m.kind == MidKind::Arc && m.radius.is_none() {
            out.push(Unknown { i, length: false });
        }
    }
    out
}

pub const MID_OPEN: usize = 2;

fn mid_pieces(mids: &[Mid], values: &[f64], signs: &[f64], k_start: f64, k_end: f64) -> Vec<Route> {
    let open = mid_unknowns(mids);
    let value = |i: usize, length: bool| open.iter().position(|u| u.i == i && u.length == length).map(|at| values[at]);
    let mut sign = 0;
    // (length, curvature or None for a transition)
    let parts: Vec<(f64, Option<f64>)> = mids
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let length = m.length.or_else(|| value(i, true)).unwrap_or(f64::NAN);
            match m.kind {
                MidKind::Straight => (length, Some(0.0)),
                MidKind::Arc => {
                    let k = match m.radius {
                        Some(r) => {
                            let s = signs.get(sign).copied().unwrap_or(f64::NAN);
                            sign += 1;
                            s / r
                        }
                        None => value(i, false).unwrap_or(f64::NAN),
                    };
                    (length, Some(k))
                }
                MidKind::Transition => (length, None),
            }
        })
        .collect();
    let k_of = |j: i64| -> f64 {
        if j < 0 {
            k_start
        } else if j as usize >= parts.len() {
            k_end
        } else {
            parts[j as usize].1.unwrap_or(0.0)
        }
    };
    parts
        .iter()
        .enumerate()
        .map(|(i, &(length, k))| match k {
            Some(k) => {
                let r = if k.abs() < STRAIGHT_CURVATURE { None } else { Some(1.0 / k) };
                Route { length, r1: r, r2: r }
            }
            None => Route {
                length,
                r1: radius_of_curvature(k_of(i as i64 - 1)),
                r2: radius_of_curvature(k_of(i as i64 + 1)),
            },
        })
        .collect()
}

fn wrap_deg(a: f64) -> f64 {
    (((a + 180.0) % 360.0) + 360.0) % 360.0 - 180.0
}

const CLOSE_TOL: f64 = 1e-8;

/// Where a chain leaves or arrives: the point, its bearing and the curvature there.
#[derive(Clone, Copy, Debug)]
pub struct End {
    pub utm: Utm,
    pub bearing: f64,
    pub k: f64,
}

/// The sides the given radii may bend to (`midSignSets`).
pub fn mid_sign_sets(mids: &[Mid], turn: f64) -> Vec<Vec<f64>> {
    let fixed: Vec<Option<f64>> = mids.iter().filter(|m| m.kind == MidKind::Arc && m.radius.is_some()).map(|m| m.side).collect();
    let mut out: Vec<Vec<f64>> = Vec::new();
    for bits in 0..(1usize << fixed.len()) {
        let set: Vec<f64> = fixed
            .iter()
            .enumerate()
            .map(|(b, side)| side.unwrap_or(if (bits >> b) & 1 == 1 { -turn } else { turn }))
            .collect();
        if !out.contains(&set) {
            out.push(set);
        }
    }
    out
}

/// What `solveMidChain` is asked.
pub struct MidAsk<'a> {
    pub mids: &'a [Mid],
    pub start: End,
    pub end_at: &'a dyn Fn(f64) -> Option<End>,
    pub s2_start: f64,
    pub lg_start: Option<f64>,
    pub delta_start: f64,
    pub seed: Option<(f64, Vec<f64>)>,
}

/// The open values that close the connection (`solveMidChain`): (s2, pieces).
pub fn solve_mid_chain(a: &MidAsk) -> Option<(f64, Vec<Route>)> {
    let open = mid_unknowns(a.mids);
    if open.len() != MID_OPEN {
        return None;
    }
    let turn = if a.delta_start > 0.0 {
        1.0
    } else if a.delta_start < 0.0 {
        -1.0
    } else {
        1.0
    };
    let sign_sets = mid_sign_sets(a.mids, turn);
    let given: f64 = a.mids.iter().map(|m| m.length.unwrap_or(0.0)).sum();
    let open_lengths = open.iter().filter(|u| u.length).count();
    let lg = a.lg_start.filter(|l| l.is_finite() && *l > 0.0).unwrap_or(20.0);
    let each = if open_lengths > 0 {
        (lg - given).max(lg / 2.0) / open_lengths as f64
    } else {
        0.0
    };
    let k_guess = if lg > 0.0 { a.delta_start / lg } else { 0.0 };
    let mut starts: Vec<Vec<f64>> = [k_guess, 0.0, -k_guess, 2.0 * k_guess]
        .iter()
        .map(|&k| {
            std::iter::once(a.s2_start)
                .chain(open.iter().map(|u| if u.length { each } else { k }))
                .collect()
        })
        .collect();
    if let Some((s2, values)) = &a.seed
        && values.len() == open.len()
        && s2.is_finite()
    {
        starts.insert(0, std::iter::once(*s2).chain(values.iter().copied()).collect());
    }
    for signs in &sign_sets {
        for init in &starts {
            if let Some(found) = chain_newton(a.mids, signs, a.start, a.end_at, init, None, None) {
                return Some(found);
            }
        }
    }
    None
}

/// Damped Newton over three unknowns (`chainNewton`): (s2, pieces).
pub fn chain_newton(
    mids: &[Mid],
    signs: &[f64],
    start: End,
    end_at: &dyn Fn(f64) -> Option<End>,
    x0: &[f64],
    s_fixed: Option<f64>,
    patience: Option<usize>,
) -> Option<(f64, Vec<Route>)> {
    let open = mid_unknowns(mids);
    let free = s_fixed.is_none();
    let length_at: Vec<bool> = (if free { vec![false] } else { vec![] })
        .into_iter()
        .chain(open.iter().map(|u| u.length))
        .collect();
    let s_of = |x: &[f64]| if free { x[0] } else { s_fixed.unwrap_or(f64::NAN) };
    let residual = |x: &[f64]| -> Option<([f64; 3], Vec<Route>)> {
        let end = end_at(s_of(x))?;
        let pieces = mid_pieces(mids, if free { &x[1..] } else { x }, signs, start.k, end.k);
        if pieces.iter().any(|p| !(p.length > 1e-6) && p.r1 != p.r2) {
            return None;
        }
        let scale = 10f64.max(pieces.iter().map(|p| p.length.abs()).sum());
        let p = switch_chain_point_utm(start.utm, start.bearing, &pieces, None)?;
        let b = switch_chain_bearing_at(start.bearing, &pieces, None);
        Some((
            [
                p.e - end.utm.e,
                p.n - end.utm.n,
                wrap_deg(b - end.bearing) * std::f64::consts::PI / 180.0 * scale,
            ],
            pieces,
        ))
    };
    let hyp = |f: &[f64; 3]| rail_across_js::js_hypot(&[f[0], f[1], f[2]]);
    let mut x: Vec<f64> = x0.to_vec();
    let mut r = residual(&x)?;
    let mut misses: Vec<f64> = Vec::new();
    for it in 0..50 {
        if hyp(&r.0) < CLOSE_TOL {
            break;
        }
        misses.push(hyp(&r.0));
        if let Some(p) = patience
            && p > 0
            && it >= p
            && misses[it] > misses[it - p] / 2.0
        {
            return None;
        }
        let mut cols: Vec<[f64; 3]> = Vec::with_capacity(3);
        for j in 0..3 {
            let h = if free && j == 0 {
                1e-4
            } else if length_at[j] {
                1e-5 * 1f64.max(x[j].abs())
            } else {
                1e-8
            };
            let mut xh = x.clone();
            xh[j] += h;
            let (fh, _) = residual(&xh)?;
            cols.push([(fh[0] - r.0[0]) / h, (fh[1] - r.0[1]) / h, (fh[2] - r.0[2]) / h]);
        }
        let step = solve3(&cols, &r.0)?;
        let mut t = 1.0;
        let mut next = None;
        for _ in 0..20 {
            let xt: Vec<f64> = x.iter().enumerate().map(|(j, v)| v - t * step[j]).collect();
            if let Some(rt) = residual(&xt)
                && hyp(&rt.0) < hyp(&r.0)
            {
                next = Some((xt, rt));
                break;
            }
            t /= 2.0;
        }
        let (xn, rn) = next?;
        x = xn;
        r = rn;
    }
    if hyp(&r.0) >= 1e-6 {
        return None;
    }
    if r.1.iter().any(|p| !(p.length > NODE_TOL)) {
        return None;
    }
    Some((s_of(&x), r.1))
}

fn solve3(cols: &[[f64; 3]], f: &[f64; 3]) -> Option<[f64; 3]> {
    let (a, b, c) = (cols[0], cols[1], cols[2]);
    let det =
        |u: [f64; 3], v: [f64; 3], w: [f64; 3]| u[0] * (v[1] * w[2] - v[2] * w[1]) - v[0] * (u[1] * w[2] - u[2] * w[1]) + w[0] * (u[1] * v[2] - u[2] * v[1]);
    let d = det(a, b, c);
    if !d.is_finite() || d.abs() < 1e-18 {
        return None;
    }
    Some([det(*f, b, c) / d, det(a, *f, c) / d, det(a, b, *f) / d])
}

/// Curvature at the end of a chain (`endCurvature`).
pub fn end_curvature(chain: &[Route]) -> f64 {
    curvature_of(chain.last().and_then(|r| r.r2))
}
