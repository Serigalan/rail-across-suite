// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Track optimization, simple curves and compound curves (Korbbögen) —
//! the former Python service's optimize.py and api.py, ported.
//!
//! Model:
//! * variables: lateral shifts s_i of the interior straights (parallel,
//!   bearing fixed); per curve group with n arcs: radii R_1..R_n, cants
//!   u_1..u_n and the free sweep splits θ_1..θ_{n−1} (the last sweep closes the
//!   corner);
//! * objective: maximize the bottleneck speed min v over all optimizable arcs,
//!   v = √(R (u + u_f) / 11.8); the whole group runs at its min v;
//! * constraints: max lateral offset to the existing alignment ≤ corridor,
//!   track ends fixed, and every rule of the catalogue the run was asked for,
//!   at the level it was asked for ([`crate::grenzen`]). None of them is a
//!   number here.
//!
//! `baseline` is the per-curve search for simple groups (s = 0, u grid + R
//! bisection); compound groups enter at their existing geometry.
//! `joint_optimize` sweeps windows along the track — a group plus whoever
//! shares a straight with it — running Differential Evolution over each,
//! warm-started and guarded so it never falls below the baseline. A cant may
//! only change where the ramps on both sides of its arc exist.

use crate::geometry::{
    CANT_DEFICIENCY_COEFF, GroupFit, L_STEP, Pt, R_MIN, R_STEP, Segment, fit_compound_group, fit_s_group, max_dist_to_polyline, sample_arc, sample_transition,
    snap_down, snap_up,
};
use crate::grenzen::{Grenzen, Stufe};
use crate::numerics::{Evolution, NelderMead, differential_evolution, nelder_mead};
use crate::rules::Rules;
use crate::track::{Built, Group, Skipped, build_elements, elements_of, parse_groups};
use serde_json::{Map, Value as Json, json};
use std::collections::{BTreeSet, HashMap};

const PENALTY: f64 = 1000.0;

/// What a run is held to: the corridor [m], the target speed, the limits.
pub struct Params<'a, 'k> {
    pub corridor: f64,
    pub v_max: Option<f64>,
    pub grenzen: &'a Grenzen<'k>,
}

/// A curvature jump that bounds a group's speed.
#[derive(Debug, Clone, Copy)]
pub struct Jump {
    pub slot: usize,
    pub r_w: f64,
    pub soll: f64,
}

/// One group, fitted and feasible.
#[derive(Debug, Clone)]
pub struct Sol {
    pub radii: Vec<f64>,
    pub us: Vec<f64>,
    pub thetas_free: Vec<f64>,
    pub v: f64,
    pub v_design: Option<f64>,
    pub trans_l: Vec<f64>,
    pub ramp_rules: Vec<Option<String>>,
    pub jump: Option<Jump>,
    pub fit: GroupFit,
    pub offset: f64,
}

fn u_max_for(g: &Group, p: &Params) -> f64 {
    p.grenzen.u_max(g.on_switch)
}

fn arc_speed(g: &Group, radius: f64, cant: f64, p: &Params) -> f64 {
    p.grenzen.speed(radius, cant, g.on_switch)
}

/// The curvature jumps of a group: one per slot without a transition, as
/// (slot, comparison radius, inside one turnout).
fn jumps(g: &Group, radii: &[f64]) -> Vec<(usize, f64, bool)> {
    let n = radii.len();
    let mut out = Vec::new();
    for (i, has) in g.has_t.iter().enumerate() {
        if *has {
            continue;
        }
        let k1 = if i == 0 { 0.0 } else { g.signs[i - 1] / radii[i - 1] };
        let k2 = if i == n { 0.0 } else { g.signs[i] / radii[i] };
        let step = (k1 - k2).abs();
        if step < 1e-12 {
            continue;
        }
        out.push((i, 1.0 / step, g.jump_internal.get(i).copied().unwrap_or(false)));
    }
    out
}

/// The speed the group runs at, and what bounds it: the slowest arc, or a
/// curvature jump lower still (LP.KS.01). None where a jump admits no design
/// speed at all. `straights`: a jump onto a bounding straight is judged at
/// the faster of the two design speeds that meet there — left out for the
/// existing alignment.
fn group_speed(g: &Group, radii: &[f64], us: &[f64], p: &Params, straights: bool) -> Option<(f64, Option<Jump>)> {
    let gz = p.grenzen;
    let mut v = radii.iter().zip(us).map(|(r, u)| arc_speed(g, *r, *u, p)).fold(f64::INFINITY, f64::min);
    let mut jump = None;
    let n = radii.len();
    for (slot, r_w, internal) in jumps(g, radii) {
        let own = if slot == 0 {
            g.entry_speed
        } else if slot == n {
            g.exit_speed
        } else {
            0.0
        };
        if straights
            && own != 0.0
            && let Some(v_own) = gz.design_speed(own)
            && gz.jump_need(v_own, internal) > r_w
        {
            return None;
        }
        let v_jump = gz.jump_speed(r_w, v, internal)?;
        if v_jump < v - 1e-9 {
            v = v_jump;
            jump = Some(Jump {
                slot,
                r_w,
                soll: gz.jump_need(v_jump, internal),
            });
        }
    }
    Some((v, jump))
}

/// What the group permits as it lies — 0 where the catalogue admits no speed for it.
fn bestand_speed(g: &Group, p: &Params) -> f64 {
    let radii: Vec<f64> = g.arcs.iter().map(|a| a.r_alt).collect();
    let us: Vec<f64> = g.arcs.iter().map(|a| a.u_alt).collect();
    group_speed(g, &radii, &us, p, false).map_or(0.0, |(v, _)| v)
}

/// Speed as the objective counts it: nothing above the target, nor above the
/// fastest design speed the catalogue knows.
fn capped(v: f64, p: &Params) -> f64 {
    let v = v.min(p.grenzen.v_top);
    match p.v_max {
        Some(v_max) => v.min(v_max),
        None => v,
    }
}

/// The radius past which a run has nothing to gain at this cant — never
/// below what is already built.
fn radius_cap(g: &Group, u: f64, p: &Params, r_alt: f64) -> f64 {
    match p.v_max {
        Some(v_max) => r_alt.max(p.grenzen.radius_for(v_max, u, g.on_switch)),
        None => f64::INFINITY,
    }
}

/// May the cant of arc i be touched at all? Only where both adjacent ramps
/// exist and the existing cant is within the ceiling.
fn u_variable(g: &Group, i: usize, p: &Params) -> bool {
    g.has_t[i] && g.has_t[i + 1] && g.arcs[i].u_alt <= u_max_for(g, p)
}

/// Transition lengths per slot for design speed V: every length and gradient
/// rule of the transition's form at once, on the length grid rounded up —
/// with the rule that asked for each — or None where no length satisfies all.
fn ramp_lengths(g: &Group, v: f64, radii: &[f64], us: &[f64], p: &Params) -> Option<(Vec<f64>, Vec<Option<String>>)> {
    let n = g.arcs.len();
    // Signed: over the ramp between the two arcs of an S the step is their sum.
    let mut signed_u = vec![0.0];
    signed_u.extend(g.signs.iter().zip(us).map(|(s, u)| s * u));
    signed_u.push(0.0);
    let mut signed_uf = vec![0.0];
    signed_uf.extend(g.signs.iter().zip(radii).zip(us).map(|((s, r), u)| s * (CANT_DEFICIENCY_COEFF * v * v / r - u)));
    signed_uf.push(0.0);
    let mut lengths = Vec::with_capacity(n + 1);
    let mut rules = Vec::with_capacity(n + 1);
    for i in 0..=n {
        if !g.has_t[i] {
            lengths.push(0.0);
            rules.push(None);
            continue;
        }
        let du = (signed_u[i + 1] - signed_u[i]).abs();
        let duf = (signed_uf[i + 1] - signed_uf[i]).abs();
        let bounds = p.grenzen.ramp_bounds(g.types[i].id(), v, du, duf).ok()??;
        let length = snap_up(bounds.shortest, L_STEP);
        if length > bounds.longest + 1e-9 {
            return None;
        }
        lengths.push(length);
        rules.push(bounds.by);
    }
    Some((lengths, rules))
}

fn sample_fit(fit: &GroupFit) -> Vec<Pt> {
    let mut pts: Vec<Pt> = Vec::new();
    for seg in &fit.segments {
        let part = match *seg {
            Segment::Transition {
                start,
                bearing,
                length,
                r1,
                r2,
                profile,
            } => sample_transition(start.0, start.1, bearing, length, r1, r2, profile, None),
            Segment::Arc { start, end, signed_r, .. } => sample_arc(start, end, signed_r, 0.02),
        };
        let skip = usize::from(!pts.is_empty());
        pts.extend(part.into_iter().skip(skip));
    }
    pts
}

/// Max lateral deviation of the fitted curve zone against the original
/// alignment — the second sweep skipped once the first has left `limit`.
fn fit_offset(g: &Group, fit: &GroupFit, p1: Pt, p2: Pt, limit: f64) -> f64 {
    let new_pts = sample_fit(fit);
    let offset = max_dist_to_polyline(&new_pts, &g.ref_poly);
    if offset > limit {
        return offset;
    }
    let mut new_poly = vec![p1];
    new_poly.extend(&new_pts);
    new_poly.push(p2);
    offset.max(max_dist_to_polyline(&g.ref_curve_pts, &new_poly))
}

/// One group fitted and judged, or None. `trans_l`: the ramps to fit with —
/// the existing ones reproduce the group as it lies, and then the length rules
/// are not applied to it (an existing alignment is a fact, not a proposal).
#[allow(clippy::too_many_arguments)]
fn evaluate_group(
    g: &Group,
    radii: &[f64],
    us: &[f64],
    thetas_free: &[f64],
    p: &Params,
    p1: Option<Pt>,
    p2: Option<Pt>,
    trans_l: Option<&[f64]>,
) -> Option<Sol> {
    let gz = p.grenzen;
    let (p1, p2) = (p1.unwrap_or(g.p1), p2.unwrap_or(g.p2));
    let proposed = trans_l.is_none();
    let (v, jump) = match group_speed(g, radii, us, p, proposed) {
        Some(found) => found,
        None if proposed => return None,
        None => (0.0, None),
    };
    let design = gz.design_speed(v);
    let (trans_l, ramp_rules) = match trans_l {
        Some(t) => (t.to_vec(), vec![None; g.has_t.len()]),
        None => ramp_lengths(g, design?, radii, us, p)?,
    };
    let fit = if g.s_curve {
        let signed: Vec<f64> = g.signs.iter().zip(radii).map(|(s, r)| s * r).collect();
        fit_s_group(p1, g.d1, g.b1, p2, g.d2, g.b2, &signed, &trans_l, &g.types, g.s_entry)
    } else {
        fit_compound_group(p1, g.d1, g.b1, p2, g.d2, g.b2, radii, thetas_free, &trans_l, &g.types)
    }?;
    if proposed {
        // The straights either side are elements of their own, stated at
        // their own design speed — the faster of it and the curve's counts.
        for (length, own) in [(fit.entry_len, g.entry_speed), (fit.exit_len, g.exit_speed)] {
            let at = gz.design_speed(v.max(own))?;
            if length < gz.min_length("straight", at, false).ok()? {
                return None;
            }
        }
        let arc_min = gz.min_length("circular_arc", design?, false).ok()?;
        for seg in &fit.segments {
            if let Segment::Arc { sweep, signed_r, .. } = seg
                && (sweep * signed_r).abs() < arc_min
            {
                return None;
            }
        }
    }
    let offset = fit_offset(g, &fit, p1, p2, p.corridor);
    if offset > p.corridor {
        return None;
    }
    Some(Sol {
        radii: radii.to_vec(),
        us: us.to_vec(),
        thetas_free: thetas_free.to_vec(),
        v,
        v_design: design,
        trans_l,
        ramp_rules,
        jump,
        fit,
        offset,
    })
}

/// Which rule a group's accepted solution sits against (AP R.4), read off its
/// own numbers in the order a designer would check them: target speed,
/// curvature jump, cant ceiling, corridor, ramp rules. None where nothing is tight.
fn binding_reason(g: &Group, sol: &Sol, p: &Params) -> Json {
    let gz = p.grenzen;
    let v = sol.v;
    if let Some(v_max) = p.v_max
        && v >= v_max - 1e-6
    {
        return json!({"regel": "zielgeschwindigkeit", "ist": v, "soll": v_max});
    }
    if let Some(j) = sol.jump {
        return json!({"regel": "kruemmungssprung", "slot": j.slot + 1, "ist": j.r_w, "soll": j.soll, "regelId": "LP.KS.01"});
    }
    let u_max = u_max_for(g, p);
    if let Some(i) = sol.us.iter().position(|u| (u - u_max).abs() < 1e-6) {
        let regel = if g.on_switch { "weiche" } else { "ueberhoehung" };
        return json!({"regel": regel, "arc": i + 1, "ist": sol.us[i], "soll": u_max});
    }
    let corridor_eps = 1e-4_f64.max(p.corridor * 0.02);
    if sol.offset >= p.corridor - corridor_eps {
        return json!({"regel": "korridor", "ist": sol.offset, "soll": p.corridor});
    }
    let design = sol.v_design.or_else(|| gz.design_speed(v));
    let floor_len = design
        .and_then(|d| gz.min_length("transition_curve", d, false).ok())
        .map_or(0.0, |l| snap_up(l, L_STEP));
    if let Some(i) = sol.trans_l.iter().position(|l| *l > floor_len + 1e-9) {
        let mut reason = json!({"regel": "rampenregel", "slot": i + 1, "ist": sol.trans_l[i], "soll": floor_len});
        if let Some(Some(by)) = sol.ramp_rules.get(i) {
            reason["regelId"] = json!(by);
        }
        return reason;
    }
    Json::Null
}

/// The largest useful radius for a group whose arcs all take the same radius
/// — useful, not largest: past the one that reaches the target speed the
/// curve only moves further off the existing alignment for nothing.
fn max_radius_for(g: &Group, u: f64, p: &Params) -> Option<Sol> {
    let n = g.arcs.len();
    let r_alt = g.arcs.iter().map(|a| a.r_alt).fold(f64::INFINITY, f64::min);
    let cap = radius_cap(g, u, p, r_alt);
    // Every probe is snapped, so the search runs on the radii a run may hand out.
    let feasible = |radius: f64| evaluate_group(g, &vec![R_MIN.max(snap_down(radius, R_STEP)); n], &vec![u; n], &[], p, None, None, None);
    let (mut lo, mut hi): (f64, f64);
    if feasible(r_alt).is_some() {
        lo = r_alt;
        let mut radius = r_alt;
        let mut upper = None;
        for _ in 0..60 {
            radius *= 1.5;
            if radius >= cap {
                if let Some(at_cap) = feasible(cap) {
                    return Some(at_cap); // exactly the target speed, no further
                }
                upper = Some(cap);
                break;
            }
            if radius > 1e6 {
                break;
            }
            if feasible(radius).is_some() {
                lo = radius;
            } else {
                upper = Some(radius);
                break;
            }
        }
        let Some(u_hi) = upper else {
            return feasible(lo);
        };
        hi = u_hi;
    } else {
        let mut radius = r_alt;
        let mut found = None;
        for _ in 0..40 {
            radius *= 0.85;
            if feasible(radius).is_some() {
                found = Some(radius);
                break;
            }
        }
        lo = found?;
        hi = lo / 0.85;
    }
    while hi - lo > R_STEP {
        let mid = (lo + hi) / 2.0;
        if feasible(mid).is_some() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    feasible(lo)
}

/// A group re-fitted at its existing parameters, with its existing ramps.
fn bestand_solution(g: &Group, p: &Params) -> Option<Sol> {
    let radii: Vec<f64> = g.arcs.iter().map(|a| a.r_alt).collect();
    let us: Vec<f64> = g.arcs.iter().map(|a| a.u_alt).collect();
    let thetas: Vec<f64> = if g.s_curve {
        Vec::new()
    } else {
        g.arcs[..g.arcs.len() - 1].iter().map(|a| a.sweep_alt).collect()
    };
    evaluate_group(g, &radii, &us, &thetas, p, None, None, Some(&g.t_len))
}

/// The target group plus the neighbours that share a straight with it.
fn window_for(groups: &[Group], target: usize) -> BTreeSet<usize> {
    let mut window = BTreeSet::from([target]);
    if target > 0 && groups[target - 1].exit_idx == groups[target].entry_idx {
        window.insert(target - 1);
    }
    if target + 1 < groups.len() && groups[target + 1].entry_idx == groups[target].exit_idx {
        window.insert(target + 1);
    }
    window
}

/// Per-curve optimization with the straights on their existing lines.
/// Simple groups: u grid + R bisection; compound groups at their existing
/// geometry; groups without any feasible fit locked (None). `window`,
/// `target` (element mode): groups outside the window stay locked, the
/// target's neighbours enter at their existing geometry.
fn baseline(groups: &[Group], p: &Params, window: Option<&BTreeSet<usize>>, target: Option<usize>) -> Vec<Option<Sol>> {
    let mut solutions = Vec::with_capacity(groups.len());
    for (j, g) in groups.iter().enumerate() {
        if let Some(w) = window {
            if !w.contains(&j) {
                solutions.push(None);
                continue;
            }
            if Some(j) != target {
                solutions.push(bestand_solution(g, p));
                continue;
            }
        }
        if g.arcs.len() > 1 && !g.s_curve {
            solutions.push(bestand_solution(g, p));
            continue;
        }
        let n_arcs = g.arcs.len();
        let u_alt = g.arcs.iter().map(|a| a.u_alt).fold(f64::INFINITY, f64::min);
        // The whole grid, below the existing cant as well as above it; the
        // existing cant always a candidate.
        let u_step = p.grenzen.u_step;
        let mut u_values = vec![u_alt];
        if (0..n_arcs).all(|i| u_variable(g, i, p)) {
            let mut u = 0.0;
            while u <= u_max_for(g, p) {
                if u != u_alt {
                    u_values.push(u);
                }
                u += u_step;
            }
        }
        // A proposal below what the group does today is no proposal.
        let v_alt = bestand_speed(g, p);
        let mut best: Option<(Sol, (f64, f64))> = None;
        for u in u_values {
            let Some(cand) = max_radius_for(g, u, p) else { continue };
            if cand.v < v_alt - 1e-9 {
                continue;
            }
            // Faster wins; between equals the one that disturbs the existing cant least.
            let key = (capped(cand.v, p), -(u - u_alt).abs());
            if best.as_ref().is_none_or(|(_, k)| key > *k) {
                best = Some((cand, key));
            }
        }
        solutions.push(best.map(|(s, _)| s));
    }
    solutions
}

// ── joint optimization (shiftable interior straights) ────────────────────────

/// What one window's search may spend (see optimize.py for the measurements).
const WINDOW_POPSIZE: usize = 8;
const SINGLE_POPSIZE: usize = 15;
const SWEEP_BUDGET: usize = 240;
const WINDOW_MIN_MAXITER: usize = 40;
const SWEEP_MAX_WINDOWS: usize = 6;
const POLISH_STEPS: usize = 200;

/// The straights a run may move sideways: the ones two groups share.
fn interior_straights(groups: &[Group]) -> Vec<usize> {
    let entries: BTreeSet<usize> = groups.iter().map(|g| g.entry_idx).collect();
    let mut out: Vec<usize> = groups.iter().map(|g| g.exit_idx).filter(|e| entries.contains(e)).collect();
    out.sort_unstable();
    out
}

fn shifted_anchor(point: Pt, dir: Pt, s: f64) -> Pt {
    (point.0 - s * dir.1, point.1 + s * dir.0)
}

/// What a group contributes to the search vector.
fn n_vars(g: &Group) -> usize {
    let n = g.arcs.len();
    if g.s_curve { 2 * n } else { 3 * n - 1 }
}

struct Window<'w, 'a, 'k> {
    groups: &'w [Group],
    live: Vec<usize>,
    held: Vec<Option<Sol>>,
    held_shifts: HashMap<usize, f64>,
    straight_ids: Vec<usize>,
    p: &'w Params<'a, 'k>,
    target: Option<usize>,
    v_floors: Vec<f64>,
    pairs: Vec<(usize, usize)>,
}

type Decoded = (HashMap<usize, f64>, Vec<(Vec<f64>, Vec<f64>, Vec<f64>)>);

impl Window<'_, '_, '_> {
    fn layout(&self) -> Vec<(usize, usize)> {
        let mut pos = self.straight_ids.len();
        self.live
            .iter()
            .map(|&j| {
                let g = &self.groups[j];
                let at = (pos, g.arcs.len());
                pos += n_vars(g);
                at
            })
            .collect()
    }

    fn decode(&self, x: &[f64]) -> Decoded {
        let p = self.p;
        // The straights this window does not own keep the offset an earlier window gave them.
        let mut shifts = self.held_shifts.clone();
        for (i, idx) in self.straight_ids.iter().enumerate() {
            shifts.insert(*idx, x[i].clamp(-p.corridor, p.corridor));
        }
        let u_step = p.grenzen.u_step;
        let per_group = self
            .live
            .iter()
            .zip(self.layout())
            .map(|(&j, (pos, n))| {
                let g = &self.groups[j];
                // Radii on the metre grid, cants on the 5 mm one — quantized
                // inside the objective, so every candidate is directly usable.
                let radii: Vec<f64> = (0..n).map(|i| R_MIN.max(snap_down(x[pos + i], R_STEP))).collect();
                let us: Vec<f64> = (0..n)
                    .map(|i| {
                        if u_variable(g, i, p) {
                            snap_down(x[pos + n + i].clamp(0.0, u_max_for(g, p)), u_step)
                        } else {
                            g.arcs[i].u_alt
                        }
                    })
                    .collect();
                let thetas: Vec<f64> = if g.s_curve {
                    Vec::new()
                } else {
                    (0..n - 1).map(|i| x[pos + 2 * n + i].max(1e-4)).collect()
                };
                (radii, us, thetas)
            })
            .collect();
        (shifts, per_group)
    }

    /// How long the straight between two neighbouring groups has to stay.
    fn straight_need(&self, ga: &Group, gb: &Group, a: &Sol, b: &Sol) -> f64 {
        let gz = self.p.grenzen;
        let Some(v) = gz.design_speed(a.v.max(b.v).max(ga.exit_speed)) else {
            return 0.0;
        };
        let reverse = ga.signs.last() != gb.signs.first();
        gz.min_length("straight", v, reverse).unwrap_or(0.0)
    }

    /// The whole chain: the live groups fitted from `x`, the rest as held.
    fn evaluate(&self, x: &[f64]) -> (Vec<Option<Sol>>, f64) {
        let (shifts, per_group) = self.decode(x);
        let mut solutions = self.held.clone();
        let mut penalty = 0.0;
        for ((radii, us, thetas), &j) in per_group.iter().zip(&self.live) {
            let g = &self.groups[j];
            let p1 = shifted_anchor(g.p1, g.d1, shifts.get(&g.entry_idx).copied().unwrap_or(0.0));
            let p2 = shifted_anchor(g.p2, g.d2, shifts.get(&g.exit_idx).copied().unwrap_or(0.0));
            let sol = evaluate_group(g, radii, us, thetas, self.p, Some(p1), Some(p2), None);
            if sol.is_none() {
                penalty += 1.0;
            }
            solutions[j] = sol;
        }
        // Only the joints a live group sits on can have moved.
        for &(j, k) in &self.pairs {
            let (Some(a), Some(b)) = (&solutions[j], &solutions[k]) else { continue };
            let (ga, gb) = (&self.groups[j], &self.groups[k]);
            if ga.exit_idx != gb.entry_idx {
                continue;
            }
            let d = ga.d2;
            let remaining = (b.fit.cl_start.0 - a.fit.cl_end.0) * d.0 + (b.fit.cl_start.1 - a.fit.cl_end.1) * d.1;
            let need = self.straight_need(ga, gb, a, b);
            if remaining < need {
                penalty += (need - remaining) / need;
            }
        }
        (solutions, penalty)
    }

    fn objective(&self, x: &[f64]) -> f64 {
        let (solutions, penalty) = self.evaluate(x);
        let p = self.p;
        let missing = self.live.iter().filter(|&&j| solutions[j].is_none()).count() as f64;
        if let Some(t) = self.target {
            // Element mode: the target's v; neighbours may be re-shaped but
            // not fall below their existing speed (a soft floor).
            let Some(ts) = &solutions[t] else {
                return PENALTY * (1.0 + penalty + missing);
            };
            if missing > 0.0 {
                return PENALTY * (1.0 + penalty + missing);
            }
            let short: f64 = self
                .live
                .iter()
                .filter(|&&j| j != t)
                .filter_map(|&j| solutions[j].as_ref().map(|s| (self.v_floors[j] - s.v).max(0.0)))
                .sum();
            return -capped(ts.v, p) + PENALTY * (penalty + short);
        }
        let vs: Vec<f64> = self.live.iter().filter_map(|&j| solutions[j].as_ref().map(|s| capped(s.v, p))).collect();
        if missing > 0.0 || vs.is_empty() {
            return PENALTY * (1.0 + penalty + missing);
        }
        -vs.iter().copied().fold(f64::INFINITY, f64::min) + PENALTY * penalty
    }
}

/// Everything one window's objective needs, plus its start vector and bounds.
fn window_context<'w, 'a, 'k>(
    groups: &'w [Group],
    p: &'w Params<'a, 'k>,
    held: &[Option<Sol>],
    held_shifts: &HashMap<usize, f64>,
    live: Vec<usize>,
    target: Option<usize>,
    v_floors: Vec<f64>,
) -> (Window<'w, 'a, 'k>, Vec<f64>, Vec<(f64, f64)>) {
    // A straight a held group sits on must not move.
    let mut frozen = BTreeSet::new();
    for (j, g) in groups.iter().enumerate() {
        if !live.contains(&j) {
            frozen.insert(g.entry_idx);
            frozen.insert(g.exit_idx);
        }
    }
    let straight_ids: Vec<usize> = interior_straights(groups).into_iter().filter(|i| !frozen.contains(i)).collect();
    let pairs: Vec<(usize, usize)> = live
        .iter()
        .flat_map(|&j| [(j.wrapping_sub(1), j), (j, j + 1)])
        .filter(|&(a, b)| a < groups.len() && b < groups.len())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut x0: Vec<f64> = straight_ids.iter().map(|i| held_shifts.get(i).copied().unwrap_or(0.0)).collect();
    let mut bounds: Vec<(f64, f64)> = vec![(-p.corridor, p.corridor); straight_ids.len()];
    for &j in &live {
        let g = &groups[j];
        match &held[j] {
            Some(sol) => {
                x0.extend(&sol.radii);
                x0.extend(&sol.us);
                x0.extend(&sol.thetas_free);
            }
            None => {
                x0.extend(g.arcs.iter().map(|a| a.r_alt));
                x0.extend(g.arcs.iter().map(|a| a.u_alt));
                if !g.s_curve {
                    x0.extend(g.arcs[..g.arcs.len() - 1].iter().map(|a| a.sweep_alt));
                }
            }
        }
        bounds.extend(g.arcs.iter().map(|a| (R_MIN.max(0.25 * a.r_alt), 10.0 * a.r_alt)));
        bounds.extend(g.arcs.iter().map(|a| (0.0, a.u_alt.max(u_max_for(g, p)))));
        if !g.s_curve {
            bounds.extend(
                g.arcs[..g.arcs.len() - 1]
                    .iter()
                    .map(|a| (1e-3_f64.max(0.2 * a.sweep_alt), std::f64::consts::PI.min(2.5 * a.sweep_alt.max(1e-3)))),
            );
        }
    }
    // The polish is unconstrained, so a held solution can carry a value from
    // outside the box; the next window may not start there.
    let x0 = x0.iter().zip(&bounds).map(|(v, (lo, hi))| v.clamp(*lo, *hi)).collect();
    let ctx = Window {
        groups,
        live,
        held: held.to_vec(),
        held_shifts: held_shifts.clone(),
        straight_ids,
        p,
        target,
        v_floors,
        pairs,
    };
    (ctx, x0, bounds)
}

/// Differential Evolution over one window, a Nelder–Mead polish after it;
/// the best of start, search and polish. None where it finds nothing usable.
fn run_window(ctx: &Window, x0: Vec<f64>, bounds: &[(f64, f64)], maxiter: usize, seed: u64, popsize: usize) -> Option<WindowResult> {
    let f = |x: &[f64]| ctx.objective(x);
    let searched = differential_evolution(
        f,
        bounds,
        &Evolution {
            maxiter,
            popsize,
            tol: 1e-6,
            seed,
            x0: Some(x0.clone()),
        },
    );
    let steps = POLISH_STEPS * x0.len();
    let polished = nelder_mead(
        f,
        &searched,
        &NelderMead {
            initial_simplex: None,
            xatol: 1e-4,
            fatol: 1e-6,
            maxfev: steps,
            maxiter: Some(steps),
        },
    )
    .x;
    let mut best: Option<(Vec<f64>, f64)> = None;
    for x in [x0, searched, polished] {
        let value = f(&x);
        if best.as_ref().is_none_or(|(_, b)| value < *b) {
            best = Some((x, value));
        }
    }
    let (best_x, _) = best?;
    let (solutions, _) = ctx.evaluate(&best_x);
    if ctx.live.iter().any(|&j| solutions[j].is_none()) {
        return None;
    }
    let (shifts, _) = ctx.decode(&best_x);
    Some((solutions, shifts))
}

type Outcome = (Vec<Option<Sol>>, HashMap<usize, f64>, Vec<Option<Sol>>);
/// What a window's search gives back: every group's solution and the shifts of the straights.
type WindowResult = (Vec<Option<Sol>>, HashMap<usize, f64>);

fn joint_optimize(groups: &[Group], p: &Params, maxiter: usize, seed: u64, target: Option<usize>) -> Outcome {
    let window = target.map(|t| window_for(groups, t));
    let base = baseline(groups, p, window.as_ref(), target);
    if base.iter().all(Option::is_none) {
        return (base.clone(), HashMap::new(), base);
    }
    let v_floors: Vec<f64> = groups.iter().map(|g| bestand_speed(g, p)).collect();

    if let (Some(t), Some(window)) = (target, &window) {
        let live: Vec<usize> = window.iter().copied().filter(|&j| base[j].is_some()).collect();
        let (ctx, x0, bounds) = window_context(groups, p, &base, &HashMap::new(), live, Some(t), v_floors);
        return match run_window(&ctx, x0, &bounds, maxiter, seed, SINGLE_POPSIZE) {
            Some((sols, shifts)) => (sols, shifts, base),
            None => (base.clone(), HashMap::new(), base),
        };
    }

    // Track mode: work on the slowest group, in its own window, until it will
    // not move — then nothing else will move the bottleneck either.
    let mut solutions = base.clone();
    let mut shifts: HashMap<usize, f64> = HashMap::new();
    let reachable: Vec<usize> = (0..base.len()).filter(|&j| base[j].is_some()).collect();
    let rounds = (reachable.len() + 2).min(SWEEP_MAX_WINDOWS);
    let window_maxiter = WINDOW_MIN_MAXITER.max(maxiter.min(SWEEP_BUDGET / rounds));
    for _ in 0..rounds {
        let speed = |j: usize, sols: &[Option<Sol>]| capped(sols[j].as_ref().map_or(f64::INFINITY, |s| s.v), p);
        let target = *reachable
            .iter()
            .min_by(|&&a, &&b| speed(a, &solutions).total_cmp(&speed(b, &solutions)))
            .unwrap();
        if let Some(v_max) = p.v_max
            && speed(target, &solutions) >= v_max
        {
            break; // the slowest curve is already fast enough
        }
        let live: Vec<usize> = window_for(groups, target).into_iter().filter(|&j| solutions[j].is_some()).collect();
        let before = live.iter().map(|&j| speed(j, &solutions)).fold(f64::INFINITY, f64::min);
        let (ctx, x0, bounds) = window_context(groups, p, &solutions, &shifts, live.clone(), None, v_floors.clone());
        let Some((sols, new_shifts)) = run_window(&ctx, x0, &bounds, window_maxiter, seed, WINDOW_POPSIZE) else {
            break;
        };
        let after = live.iter().map(|&j| speed(j, &sols)).fold(f64::INFINITY, f64::min);
        if after <= before + 1e-9 {
            break;
        }
        solutions = sols;
        shifts = new_shifts;
    }
    (solutions, shifts, base)
}

/// A request a run cannot take: said to the user as it is.
fn unsupported(message: impl Into<String>) -> Json {
    json!({"error": "unsupported_topology", "message": message.into()})
}

/// The transition profiles a run tries: the ramps as they lie, and all of
/// them Bloss — the latter only where the level admits a Bloss transition.
fn variants_for(uebergang: &str, gz: &Grenzen) -> Result<Vec<&'static str>, String> {
    let bloss_ok = gz.forms.iter().any(|(f, ok)| *f == "bloss" && *ok);
    match uebergang {
        "auto" => Ok(if bloss_ok { vec!["bestand", "bloss"] } else { vec!["bestand"] }),
        "bestand" => Ok(vec!["bestand"]),
        "bloss" if bloss_ok => Ok(vec!["bloss"]),
        "bloss" => Err("Blossbögen schlägt ein Lauf nur an der Ermessensgrenze vor (LP.UB.02: die Überhöhungsrampe ist in dieser App immer gerade).".into()),
        other => Err(format!("unbekanntes Übergangsbogen-Profil: {other}")),
    }
}

/// The largest number of generations a request may ask for.
const MAX_ITER: usize = 150;
/// The most elements a track may bring.
const MAX_ELEMENTS: usize = 2000;

fn design_speed_json(sol: &Sol) -> Json {
    match sol.v_design {
        Some(v) if v == v.trunc() => json!(v as i64),
        Some(v) => json!(v),
        None => json!(sol.v.floor() as i64),
    }
}

/// The answer to an optimize request.
///
/// Body: {"track": {elements, epsg, …}, "corridorCm", "grenzwert": "reg" |
/// "discretion", "uebergang": "auto" | "bestand" | "bloss", "perCurve",
/// "maxiter", "seed", "targetElementIdx", "vMax"}.
///
/// Answers {elements, report, variant, vBestand, vBaseline, vNeu, shifts,
/// skipped, regelwerk, regelwerkVersion, grenzwert} — or {"error":
/// "unsupported_topology", "message"} for a track a run cannot take.
pub fn optimize_payload(rules: &Rules, payload: &Json) -> Result<Json, String> {
    let bad = |what: &str| format!("invalid request: {what}");
    let track = payload.get("track").ok_or_else(|| bad("track"))?;
    let (els, epsg) = elements_of(track).ok_or_else(|| bad("track"))?;
    if els.len() > MAX_ELEMENTS {
        return Err(bad("too many elements"));
    }
    let stufe = Stufe::parse(payload.get("grenzwert").and_then(Json::as_str).unwrap_or("reg")).map_err(|e| bad(&e.0))?;
    let gz = rules.at(stufe);
    let p = Params {
        corridor: payload.get("corridorCm").and_then(Json::as_f64).unwrap_or(50.0) / 100.0,
        v_max: payload.get("vMax").and_then(Json::as_f64).filter(|v| *v != 0.0),
        grenzen: gz,
    };
    let maxiter = (payload.get("maxiter").and_then(Json::as_u64).unwrap_or(100) as usize).clamp(1, MAX_ITER);
    let seed = payload.get("seed").and_then(Json::as_u64).unwrap_or(1);
    let per_curve = payload.get("perCurve").and_then(Json::as_bool).unwrap_or(false);
    let names = match variants_for(payload.get("uebergang").and_then(Json::as_str).unwrap_or("auto"), gz) {
        Ok(n) => n,
        Err(message) => return Ok(unsupported(message)),
    };
    let mut skipped: Vec<Skipped> = Vec::new();
    let groups = match parse_groups(els, &mut skipped) {
        Ok(g) => g,
        Err(message) => return Ok(unsupported(message)),
    };
    let target = match payload.get("targetElementIdx").and_then(Json::as_u64) {
        None => None,
        Some(idx) => match groups.iter().position(|g| g.arc_idxs.contains(&(idx as usize))) {
            Some(t) => Some(t),
            None => return Ok(unsupported("Das gewählte Element gehört zu keinem optimierbaren Bogen.")),
        },
    };

    let run = |grps: &[Group]| -> Outcome {
        if per_curve {
            let window = target.map(|t| window_for(grps, t));
            let sols = baseline(grps, &p, window.as_ref(), target);
            return (sols.clone(), HashMap::new(), sols);
        }
        joint_optimize(grps, &p, maxiter, seed, target)
    };
    let score = |sols: &[Option<Sol>]| match target {
        Some(t) => sols[t].as_ref().map_or(f64::NEG_INFINITY, |s| capped(s.v, &p)),
        None => {
            let vs: Vec<f64> = sols.iter().flatten().map(|s| capped(s.v, &p)).collect();
            if vs.is_empty() {
                f64::NEG_INFINITY
            } else {
                vs.into_iter().fold(f64::INFINITY, f64::min)
            }
        }
    };

    let mut best: Option<(f64, &str, Vec<Group>, Outcome)> = None;
    for name in names {
        let grps: Vec<Group> = if name == "bloss" {
            groups
                .iter()
                .map(|g| Group {
                    types: g
                        .types
                        .iter()
                        .zip(&g.has_t)
                        .map(|(t, has)| if *has { crate::geometry::Profile::Bloss } else { *t })
                        .collect(),
                    ..g.clone()
                })
                .collect()
        } else {
            groups.clone()
        };
        let outcome = run(&grps);
        let v = score(&outcome.0);
        if best.as_ref().is_none_or(|(b, ..)| v > *b + 1e-9) {
            best = Some((v, if name == "bloss" { "Bloss" } else { "Bestand" }, grps, outcome));
        }
    }
    let (_, variant, groups_used, (solutions, shifts, base)) = best.ok_or("no variant")?;

    let report_gis: Vec<usize> = match target {
        Some(t) => window_for(&groups_used, t).into_iter().collect(),
        None => (0..groups_used.len()).collect(),
    };
    let score_gis: Vec<usize> = match target {
        Some(t) => vec![t],
        None => report_gis.clone(),
    };
    let v_bestand = score_gis.iter().map(|&j| bestand_speed(&groups_used[j], &p)).fold(f64::INFINITY, f64::min);
    let min_of = |sols: &[Option<Sol>]| {
        let vs: Vec<f64> = score_gis.iter().filter_map(|&j| sols[j].as_ref().map(|s| s.v)).collect();
        if vs.is_empty() {
            v_bestand
        } else {
            vs.into_iter().fold(f64::INFINITY, f64::min)
        }
    };
    let (v_base, v_neu) = (min_of(&base), min_of(&solutions));

    let mut report = Vec::new();
    for &j in &report_gis {
        let (g, sol) = (&groups_used[j], &solutions[j]);
        let reason = sol.as_ref().map(|s| binding_reason(g, s, &p));
        let v_group_alt = bestand_speed(g, &p);
        for (i, arc) in g.arcs.iter().enumerate() {
            let mut row = Map::new();
            row.insert("group".into(), json!(j + 1));
            row.insert("arc".into(), json!(i + 1));
            row.insert("arcs".into(), json!(g.arcs.len()));
            row.insert("rAlt".into(), json!(arc.r_alt));
            row.insert("uAlt".into(), json!(arc.u_alt));
            row.insert("vAlt".into(), json!(arc_speed(g, arc.r_alt, arc.u_alt, &p).min(v_group_alt)));
            row.insert("changed".into(), json!(sol.is_some()));
            row.insert("target".into(), json!(target == Some(j)));
            if let Some(s) = sol {
                row.insert("rNeu".into(), json!(s.radii[i]));
                row.insert("uNeu".into(), json!(s.us[i]));
                row.insert(
                    "vNeu".into(),
                    json!(arc_speed(g, s.radii[i], s.us[i], &p).min(if s.jump.is_some() { s.v } else { f64::INFINITY })),
                );
                row.insert("offsetCm".into(), json!(s.offset * 100.0));
                row.insert("grund".into(), reason.clone().unwrap_or(Json::Null));
            }
            report.push(Json::Object(row));
        }
    }

    let built: Vec<Option<Built>> = solutions
        .iter()
        .map(|s| {
            s.as_ref().map(|s| Built {
                fit: &s.fit,
                speed: design_speed_json(s),
                us: &s.us,
            })
        })
        .collect();
    let mut shift_map = Map::new();
    let mut keys: Vec<&usize> = shifts.keys().collect();
    keys.sort();
    for k in keys {
        shift_map.insert(k.to_string(), json!(shifts[k]));
    }
    Ok(json!({
        "elements": build_elements(els, &epsg, &groups_used, &built, &shifts),
        "report": report,
        "variant": variant,
        "vBestand": v_bestand,
        "vBaseline": v_base,
        "vNeu": v_neu,
        "shifts": shift_map,
        "skipped": skipped.iter().map(|(a, b, why)| json!({"from": a, "to": b, "why": why})).collect::<Vec<_>>(),
        "regelwerk": rules.katalog.id,
        "regelwerkVersion": rules.katalog.version,
        "grenzwert": stufe.id(),
    }))
}
