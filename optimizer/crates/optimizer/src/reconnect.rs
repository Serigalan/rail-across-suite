// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reconnecting existing elements — "Bestehende Elemente neu verbinden"
//! (Paket N; services/optimizer's reconnect.py, ported).
//!
//! A stretch of one track is replaced: the app hands over the element before
//! it and the one after it as the two picks of a splice ([`crate::splice`]),
//! the departure joined at its end and the arrival at its start, and the axis
//! the stretch had as points every centimetre. The splice construction joins
//! the two ends again; what is free in it is searched so that the new axis
//! lies within `tolerance` of the old one everywhere and keeps the rules at
//! the design speed (Entscheidung 189):
//!
//! * two straights, an arc and a straight: the radius (whole metres, a grid
//!   from R_MIN to R_MAX, then narrowed around the best), at each radius the
//!   cant the app proposes for it and the transitions at their Regellänge,
//!   their Mindestlänge or none — and from the best at the Regellänge on, the
//!   transitions as long as fits the old axis best, length and radius fitted
//!   together by least squares (Entscheidung 202);
//! * two arcs: a straight between them with transitions at the Regellänge,
//!   the Mindestlänge or none — or one transition straight from arc to arc.
//!
//! Every candidate is the splice's own, checked for loops, measured against
//! the points both ways and, where it lies within the tolerance, judged by
//! the rule catalogue. Where a neighbouring track is named, a candidate within
//! the tolerance is also held to the spacing to it (Entscheidung 201). Best
//! first (Entscheidung 190): within the tolerance and the spacing kept before
//! within the tolerance alone before beyond it, then the lightest finding,
//! then the smallest largest deviation, then the smallest RMS. The answer
//! names the best of every variant.

use crate::clearance::{CantModel, Neighbour, Spacing, auto_cant, check, described};
use crate::geometry::{Profile, Pt, sample_transition, transition_steps};
use crate::numerics::{NelderMead, fminbound, nelder_mead};
use crate::poly::{KdTree, Shape, piece_points, project};
use crate::rules::Rules;
use crate::splice::{ArcJoin, El, End, Mode, Pick, Solution, Spec, centre_of, cost, judge, kept_cant, loops, solve};
use serde_json::{Map, Value as Json, json};
use std::collections::HashMap;

// The radius search: a geometric grid between these [m] …
const R_MIN: f64 = 50.0;
const R_MAX: f64 = 50000.0;
const GRID: usize = 36;
// … then this many radii between the two grid neighbours of the best, so many times.
const ZOOM: usize = 8;
const ZOOM_ROUNDS: usize = 4;
/// Where a transition is asked for at its Regellänge or Mindestlänge, the
/// fixed point of `solve` starts from this length [m].
const LENGTH_START: f64 = 60.0;
// Points between two the search measures with [m]: the grid with the coarse
// set, the narrowing with the fine one — the final measure takes every point.
const COARSE_SPACING: f64 = 1.0;
const SEARCH_SPACING: f64 = 0.5;
/// The deviation band the panel draws: one value per this much station [m].
const BAND_STEP: f64 = 0.5;
/// Far more points than any stretch a dialog replaces (5 km at 1 cm).
const MAX_POINTS: usize = 500_000;
// Fitting the transitions' length to the old axis (Entscheidung 202): at most
// so many tries of the least-squares fit; the length, between two arcs, up to
// three times the Regellänge and this much more [m]; for a whole-metre radius
// the length searched again this far either side [m]; what a try that does not
// fit counts as.
const FIT_EVALS: usize = 160;
const FIT_MARGIN: f64 = 50.0;
const FIT_LENGTH_SPAN: f64 = 20.0;
const FIT_PENALTY: f64 = 1e3;
/// Two circles this close are one [m].
const SAME_CIRCLE: f64 = 1e-3;

fn bad(what: &str) -> String {
    format!("invalid request: {what}")
}

// ── the points ───────────────────────────────────────────────────────────────

/// The old axis from the request: {e0, n0, de, dn} with de, dn in whole
/// millimetres from (e0, n0), in travel order.
fn points(raw: &Json) -> Result<Vec<Pt>, String> {
    let (Some(e0), Some(n0)) = (raw.get("e0").and_then(Json::as_f64), raw.get("n0").and_then(Json::as_f64)) else {
        return Err(bad("points"));
    };
    let (Some(de), Some(dn)) = (raw.get("de").and_then(Json::as_array), raw.get("dn").and_then(Json::as_array)) else {
        return Err(bad("points"));
    };
    if de.len() != dn.len() || de.len() < 2 {
        return Err(bad("points"));
    }
    if de.len() > MAX_POINTS {
        return Err(bad("points: too many"));
    }
    de.iter()
        .zip(dn)
        .map(|(e, n)| match (e.as_f64(), n.as_f64()) {
            (Some(e), Some(n)) => Ok((e0 + e / 1000.0, n0 + n / 1000.0)),
            _ => Err(bad("points")),
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Level {
    Coarse,
    Fine,
    All,
}

/// The old axis: its points, their stations, a tree to find the nearest, and
/// the thinned sets the search measures with.
struct Reference {
    pts: Vec<Pt>,
    station: Vec<f64>,
    coarse: Vec<Pt>,
    fine: Vec<Pt>,
    tree: KdTree,
}

impl Reference {
    fn new(pts: Vec<Pt>) -> Reference {
        let steps: Vec<f64> = pts.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).collect();
        let mut station = vec![0.0];
        for s in &steps {
            station.push(station.last().unwrap() + s);
        }
        let median = {
            let mut s = steps.clone();
            s.sort_by(f64::total_cmp);
            let n = s.len();
            if n == 0 {
                0.01
            } else if n % 2 == 1 {
                s[n / 2]
            } else {
                (s[n / 2 - 1] + s[n / 2]) / 2.0
            }
        };
        let spacing = median.max(1e-6);
        let thinned = |every: f64| {
            let k = ((every / spacing).round_ties_even() as usize).max(1);
            let mut out: Vec<Pt> = pts.iter().step_by(k).copied().collect();
            if (pts.len() - 1) % k != 0 {
                out.push(*pts.last().unwrap());
            }
            out
        };
        Reference {
            coarse: thinned(COARSE_SPACING),
            fine: thinned(SEARCH_SPACING),
            tree: KdTree::new(&pts),
            station,
            pts,
        }
    }

    fn set(&self, level: Level) -> &[Pt] {
        match level {
            Level::Coarse => &self.coarse,
            Level::Fine => &self.fine,
            Level::All => &self.pts,
        }
    }
}

// ── a chain as a line ────────────────────────────────────────────────────────

/// The chain of a solution as a polyline, dense enough to measure millimetres against.
pub fn chain_polyline(els: &[El]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for el in els {
        let q = if el.length <= 1e-9 {
            vec![el.start]
        } else {
            match el.kind {
                0 => piece_points(el.start, el.bearing, el.length, Shape::Straight),
                1 => piece_points(el.start, el.bearing, el.length, Shape::Arc(el.radius.unwrap())),
                _ => {
                    let steps = transition_steps(el.length, el.r1, el.r2).max(el.length.ceil() as usize);
                    sample_transition(el.start.0, el.start.1, el.bearing, el.length, el.r1, el.r2, el.profile, Some(steps))
                }
            }
        };
        let skip = usize::from(!out.is_empty());
        out.extend(q.into_iter().skip(skip));
    }
    out
}

struct Deviation {
    max: f64,
    rms: f64,
    /// Every point's offset (new axis against old, > 0 right in travel) — measured with every point only.
    offsets: Option<Vec<f64>>,
}

/// How far the new axis `poly` lies from the old one, measured with the
/// points of `level` — with every point also the other way round.
fn measure(reference: &Reference, poly: &[Pt], level: Level) -> Deviation {
    let (off, _, _) = project(reference.set(level), poly, None);
    let mut worst = off.iter().fold(0.0_f64, |m, o| m.max(o.abs()));
    let full = level == Level::All;
    if full {
        // The other way round: no stretch of the new axis may run away from
        // the old one where no point would notice.
        let (back, _, _) = project(poly, &reference.pts, Some(&reference.tree));
        worst = back.iter().fold(worst, |m, o| m.max(o.abs()));
    }
    let rms = (off.iter().map(|o| o * o).sum::<f64>() / off.len() as f64).sqrt();
    Deviation {
        max: worst,
        rms,
        offsets: full.then(|| off.iter().map(|o| -o).collect()),
    }
}

fn round_to(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (x * f).round_ties_even() / f
}

/// The deviation over the station of the old axis, one value per BAND_STEP —
/// the largest in it, signed.
fn band(reference: &Reference, offsets: &[f64]) -> Json {
    let mut out: Vec<Json> = Vec::new();
    let mut current: Option<(i64, usize)> = None;
    let flush = |c: Option<(i64, usize)>, out: &mut Vec<Json>| {
        if let Some((_, j)) = c {
            out.push(json!([round_to(reference.station[j], 2), round_to(offsets[j], 4)]));
        }
    };
    for (j, st) in reference.station.iter().enumerate() {
        let bin = (st / BAND_STEP).floor() as i64;
        match current {
            Some((b, best)) if b == bin => {
                if offsets[j].abs() > offsets[best].abs() {
                    current = Some((b, j));
                }
            }
            _ => {
                flush(current, &mut out);
                current = Some((bin, j));
            }
        }
    }
    flush(current, &mut out);
    Json::Array(out)
}

// ── candidates ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Lengths {
    Regular,
    Minimum,
    None,
    Fitted(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Variant {
    lengths: Lengths,
    arc_join: ArcJoin,
}

impl Variant {
    fn to_json(self) -> Json {
        let (mode, l) = match self.lengths {
            Lengths::Regular => ("regular", None),
            Lengths::Minimum => ("minimum", None),
            Lengths::None => ("none", None),
            Lengths::Fitted(l) => ("fitted", Some(l)),
        };
        let mut out = json!({"lengths": mode, "arcJoin": if self.arc_join == ArcJoin::Straight { "straight" } else { "transition" }});
        if let Some(l) = l {
            out["L"] = json!(l);
        }
        out
    }
}

struct Base {
    speed: f64,
    profile: Profile,
    model: CantModel,
}

/// The splice settings for one candidate. The arc inserted takes the cant
/// the app proposes for its radius, without transitions the one of the
/// element before it (Entscheidung 315).
fn spec_of(base: &Base, variant: Variant, radius: Option<f64>, picks: &[Pick; 2]) -> Spec {
    let on = variant.lengths != Lengths::None && variant.arc_join != ArcJoin::Transition;
    let (start, modes) = match (on, variant.lengths) {
        (false, _) => (0.0, [Mode::Fixed; 2]),
        (true, Lengths::Fitted(l)) => (l, [Mode::Fixed; 2]),
        (true, Lengths::Minimum) => (LENGTH_START, [Mode::Minimum; 2]),
        (true, _) => (LENGTH_START, [Mode::Regular; 2]),
    };
    let mut spec = Spec {
        radius: radius.unwrap_or(0.0),
        lengths: [start; 2],
        modes,
        profile: base.profile,
        profiles: [base.profile; 2],
        arc_join: variant.arc_join,
        speed: base.speed,
        cant: 0.0,
    };
    if let Some(r) = radius.filter(|r| *r != 0.0) {
        spec.cant = kept_cant(picks, &spec).unwrap_or_else(|| auto_cant(base.speed, r, &base.model));
    }
    spec
}

struct Cand {
    sol: Solution,
    spec: Spec,
    variant: Variant,
    radius: Option<f64>,
    dev: Deviation,
    poly: Vec<Pt>,
    within: bool,
    /// The spacing to the neighbour, once assessed.
    spacing: Option<Map<String, Json>>,
}

type Key = (i32, i32, f64, f64);

/// Builds and measures the candidates of one request.
struct Search<'a, 'k> {
    rules: &'a Rules<'k>,
    picks: [Pick; 2],
    reference: Reference,
    base: Base,
    tol: f64,
    l_min: f64,
    clearance: Option<(Neighbour, Spacing, f64)>,
}

impl Search<'_, '_> {
    /// The tightest place of what a solution inserts to the neighbour, every
    /// element with the cant the splice gave it.
    fn spacing(&self, sol: &Solution) -> Option<Map<String, Json>> {
        let (neighbour, spacing, d_min) = self.clearance.as_ref()?;
        Some(described(
            check(&sol.elements, neighbour, spacing, *d_min, &|el: &El| el.cant.unwrap_or(0.0)),
            *d_min,
        ))
    }

    /// The splice of one variant at `radius` (None for two arcs), measured
    /// with the points of `level` — or None where it does not fit.
    fn build(&self, variant: Variant, radius: Option<f64>, level: Level) -> Option<Cand> {
        let spec = spec_of(&self.base, variant, radius, &self.picks);
        let sol = solve(self.rules, &self.picks, [End::End, End::Start], &spec).ok()?;
        if loops(&sol, &self.picks) {
            return None;
        }
        // The new arc an element of its own (Entscheidung 169): else the old
        // arc runs on and a sliver of any radius closes the gap.
        if radius.is_some_and(|r| r != 0.0) && sol.info.arc_length.unwrap_or(0.0) < self.l_min {
            return None;
        }
        let poly = chain_polyline(&sol.elements);
        if poly.len() < 2 {
            return None;
        }
        let dev = measure(&self.reference, &poly, level);
        let within = dev.max <= self.tol;
        Some(Cand {
            sol,
            spec,
            variant,
            radius,
            dev,
            poly,
            within,
            spacing: None,
        })
    }

    /// What the catalogue finds on a candidate within the tolerance, and the
    /// spacing it keeps — once, and only when it is ranked.
    fn assess(&self, cand: &mut Cand) {
        if cand.within && cand.sol.worst.is_none() {
            judge(self.rules, &mut cand.sol, &self.picks);
            cand.spacing = self.spacing(&cand.sol);
        }
    }

    fn kept(&self, cand: &Cand) -> bool {
        self.clearance.is_none() || cand.spacing.as_ref().and_then(|s| s.get("kept")).and_then(Json::as_bool).unwrap_or(false)
    }

    /// Best first: within the tolerance with the spacing kept, within the
    /// tolerance alone, beyond it — then the lightest finding, the smallest
    /// largest deviation, the smallest RMS.
    fn key(&self, cand: &mut Option<Cand>) -> Key {
        let Some(cand) = cand else {
            return (3, 0, f64::INFINITY, f64::INFINITY);
        };
        self.assess(cand);
        let rank = if cand.within {
            self.rules.katalog.rank(cand.sol.worst.as_deref().unwrap_or("ok"))
        } else {
            0
        };
        let class = if cand.within { if self.kept(cand) { 0 } else { 1 } } else { 2 };
        (class, rank, round_to(cand.dev.max, 4), cand.dev.rms)
    }

    /// The best candidate of a variant with a radius of its own to choose: on
    /// a geometric grid, then narrowed between the grid neighbours of the
    /// best, in whole metres at the end.
    fn best_radius(&self, variant: Variant, fixed: Option<f64>) -> Option<Cand> {
        if let Some(r) = fixed {
            return self.build(variant, Some(r), Level::Fine);
        }
        let grid: Vec<f64> = (0..GRID).map(|i| R_MIN * (R_MAX / R_MIN).powf(i as f64 / (GRID - 1) as f64)).collect();
        let mut tried: HashMap<(u64, Level), Option<Cand>> = HashMap::new();
        let key_at = |r: f64, level: Level, tried: &mut HashMap<(u64, Level), Option<Cand>>| -> Key {
            let entry = tried.entry((r.to_bits(), level)).or_insert_with(|| self.build(variant, Some(r), level));
            self.key(entry)
        };
        // The grid with the coarse points, from there on the fine ones.
        let keys: Vec<Key> = grid.iter().map(|&r| key_at(r, Level::Coarse, &mut tried)).collect();
        if grid.iter().all(|r| tried[&(r.to_bits(), Level::Coarse)].is_none()) {
            return None;
        }
        let i = (0..grid.len())
            .min_by(|&a, &b| keys[a].partial_cmp(&keys[b]).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let (mut lo, mut hi) = (grid[i.saturating_sub(1)], grid[(i + 1).min(grid.len() - 1)]);
        let mut best_r = grid[i];
        for _ in 0..ZOOM_ROUNDS {
            for j in 0..ZOOM {
                let r = lo + (hi - lo) * j as f64 / (ZOOM - 1) as f64;
                if key_at(r, Level::Fine, &mut tried) < key_at(best_r, Level::Fine, &mut tried) {
                    best_r = r;
                }
            }
            let step = (hi - lo) / (ZOOM - 1) as f64;
            lo = R_MIN.max(best_r - step);
            hi = R_MAX.min(best_r + step);
        }
        // Whole metres, the better of the two around it.
        let mut best: Option<(Key, f64)> = None;
        for r in [best_r.floor(), best_r.ceil()].into_iter().filter(|r| *r >= 1.0) {
            let k = key_at(r, Level::Fine, &mut tried);
            if best.is_none_or(|(bk, _)| k < bk) {
                best = Some((k, r));
            }
        }
        let (_, r) = best?;
        tried.remove(&(r.to_bits(), Level::Fine)).flatten()
    }

    /// The best candidate with the transitions as long as fits the old axis
    /// best (Entscheidung 202): from `start` — the best at the Regellänge —
    /// length and radius fitted together to the points by least squares
    /// (Nelder–Mead; the two are coupled in a narrow valley), then the radius
    /// in whole metres and for it the length in decimetres. A radius given
    /// stays; between two arcs only the length is free.
    fn best_fitted(&self, start: &Cand, fixed: Option<f64>) -> Option<Cand> {
        let reg = start
            .sol
            .lengths
            .as_ref()
            .map(|sides| {
                sides
                    .iter()
                    .map(|s| s.rules.as_ref().and_then(|r| r.regular).unwrap_or(0.0))
                    .fold(0.0, f64::max)
            })
            .filter(|r| *r != 0.0)
            .unwrap_or(LENGTH_START);
        let r0 = start.radius;
        let free_r = r0.is_some() && fixed.is_none();
        let cand = |length: f64, radius: Option<f64>, level: Level| -> Option<Cand> {
            if length.is_nan() || length < 1.0 {
                return None;
            }
            self.build(
                Variant {
                    lengths: Lengths::Fitted(length),
                    arc_join: ArcJoin::Straight,
                },
                radius,
                level,
            )
        };
        let rms = |length: f64, radius: Option<f64>| cand(length, radius, Level::Coarse).map_or(FIT_PENALTY + (length - reg).abs(), |c| c.dev.rms);

        let (length, radii) = if free_r {
            let r0 = r0.unwrap();
            let m = nelder_mead(
                |x| rms(x[0], Some(x[1])),
                &[reg, r0],
                &NelderMead {
                    initial_simplex: Some(vec![vec![reg, r0], vec![reg * 1.4, r0], vec![reg, r0 * 1.02]]),
                    xatol: 0.05,
                    fatol: 1e-5,
                    maxfev: FIT_EVALS,
                    maxiter: None,
                },
            );
            (m.x[0], vec![Some(m.x[1].floor()), Some(m.x[1].ceil())])
        } else {
            (fminbound(|x| rms(x, r0), 0.5 * reg, 3.0 * reg + FIT_MARGIN, 0.05, FIT_EVALS), vec![r0])
        };
        let mut found: Vec<Option<Cand>> = Vec::new();
        for radius in radii {
            // For a whole-metre radius, the length that fits it best.
            let lr = if free_r {
                fminbound(|x| rms(x, radius), (length - FIT_LENGTH_SPAN).max(1.0), length + FIT_LENGTH_SPAN, 0.02, 500)
            } else {
                length
            };
            for l in [(lr * 10.0).floor() / 10.0, (lr * 10.0).ceil() / 10.0] {
                if let Some(c) = cand(l, radius, Level::Fine) {
                    found.push(Some(c));
                }
            }
        }
        let mut best: Option<(Key, usize)> = None;
        for (i, cand) in found.iter_mut().enumerate() {
            let k = self.key(cand);
            if best.is_none_or(|(bk, _)| k < bk) {
                best = Some((k, i));
            }
        }
        let (_, i) = best?;
        found.swap_remove(i)
    }

    /// A candidate measured with every point, judged and as the answer has it.
    fn finish(&self, mut cand: Cand) -> Solution {
        let dev = measure(&self.reference, &cand.poly, Level::All);
        let within = dev.max <= self.tol;
        if cand.sol.worst.is_none() {
            judge(self.rules, &mut cand.sol, &self.picks);
        }
        if self.clearance.is_some() && cand.spacing.is_none() {
            cand.spacing = self.spacing(&cand.sol);
        }
        let offsets = dev.offsets.unwrap();
        let j = (0..offsets.len()).fold(0, |b, i| if offsets[i].abs() > offsets[b].abs() { i } else { b });
        let mut sol = cand.sol;
        sol.rebuilt = cost(&sol, &self.picks);
        sol.alternative = false;
        if self.picks.iter().all(|p| p.radius.is_some()) {
            sol.arc_join = Some(cand.variant.arc_join);
        }
        let worst_at = self.reference.pts[j];
        sol.extra.insert(
            "reconnect".into(),
            json!({
                "variant": cand.variant.to_json(),
                "radius": cand.radius,
                "cant": cand.spec.cant,
                "within": within,
                "spacing": cand.spacing.map(Json::Object),
                "max": round_to(dev.max, 4),
                "rms": round_to(dev.rms, 4),
                "worstAt": [round_to(worst_at.0, 3), round_to(worst_at.1, 3)],
                "worstStation": round_to(self.reference.station[j], 2),
                "band": band(&self.reference, &offsets),
            }),
        );
        sol
    }
}

/// Do both ends lie on one circle, curving the same way? Then there is
/// nothing between them to join — the arc runs on.
fn same_circle(picks: &[Pick; 2]) -> bool {
    let (Some(ra), Some(rb)) = (picks[0].radius, picks[1].radius) else {
        return false;
    };
    if (ra - rb).abs() > SAME_CIRCLE {
        return false;
    }
    let (a, b) = (centre_of(picks[0].end, picks[0].bearing, ra), centre_of(picks[1].end, picks[1].bearing, rb));
    (a.0 - b.0).hypot(a.1 - b.1) < SAME_CIRCLE
}

fn rank_answer(rules: &Rules, sol: &Solution) -> Key {
    let r = &sol.extra["reconnect"];
    let within = r["within"].as_bool().unwrap_or(false);
    let kept = r["spacing"].is_null() || r["spacing"]["kept"] == true;
    let class = if within { if kept { 0 } else { 1 } } else { 2 };
    let rank = if within {
        rules.katalog.rank(sol.worst.as_deref().unwrap_or("ok"))
    } else {
        0
    };
    (
        class,
        rank,
        r["max"].as_f64().unwrap_or(f64::INFINITY),
        r["rms"].as_f64().unwrap_or(f64::INFINITY),
    )
}

/// The answer to a reconnect request.
///
/// Body: {"dep": pick, "arr": pick — the element before the stretch and the
/// one after it, each with `joinAt`; "points": {"e0", "n0", "de": [mm],
/// "dn": [mm]} the old axis in travel order; "tolerance": m; "speed": km/h;
/// "transition": "clothoid"|"bloss"; "radius": m to keep it fixed (0:
/// searched); "cantModel"; "lMin": m the shortest new arc; "clearance":
/// optional {"ref", "dMin", "profile"}}.
///
/// Answers {"solutions": [...], "tolerance"} — the best of every variant,
/// best first, each a splice solution with `reconnect`: {variant, radius,
/// cant, within, spacing, max, rms, worstAt, worstStation, band} — or
/// {"error", "params": {}} where no variant fits at all.
pub fn reconnect_payload(rules: &Rules, payload: &Json) -> Result<Json, String> {
    if !payload.is_object() {
        return Err(bad("payload"));
    }
    let picks = [Pick::from_json(&payload["dep"])?, Pick::from_json(&payload["arr"])?];
    if picks[0].join_at != Some(End::End) || picks[1].join_at != Some(End::Start) {
        return Err(bad("joinAt"));
    }
    let reference = Reference::new(points(payload.get("points").unwrap_or(&Json::Null))?);
    let tolerance = payload
        .get("tolerance")
        .and_then(Json::as_f64)
        .filter(|t| *t > 0.0)
        .ok_or_else(|| bad("tolerance"))?;
    let speed = payload.get("speed").and_then(Json::as_f64).unwrap_or(0.0).max(0.0);
    let profile = match payload.get("transition").and_then(Json::as_str).unwrap_or("clothoid") {
        "clothoid" => Profile::Clothoid,
        "bloss" => Profile::Bloss,
        _ => return Err(bad("transition")),
    };
    let model = CantModel::from_json(payload.get("cantModel").unwrap_or(&Json::Null));
    if speed > 0.0 && model.is_none() {
        return Err(bad("cantModel"));
    }
    let fixed = payload.get("radius").and_then(Json::as_f64).filter(|r| *r != 0.0);
    let clearance = match payload.get("clearance").filter(|c| !c.is_null()) {
        None => None,
        Some(cl) if cl.is_object() => {
            let d_min = cl.get("dMin").and_then(Json::as_f64).filter(|d| *d > 0.0).ok_or_else(|| bad("dMin"))?;
            let half: Vec<Pt> = cl
                .get("profile")
                .and_then(Json::as_array)
                .map(|a| a.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect())
                .filter(|h: &Vec<Pt>| !h.is_empty())
                .ok_or_else(|| bad("profile"))?;
            let pts = &reference.pts;
            let bbox = pts.iter().fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY), |b, p| {
                (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
            });
            Some((
                Neighbour::new(&Neighbour::points_of(cl.get("ref").unwrap_or(&Json::Null)), bbox),
                Spacing::new(&half),
                d_min,
            ))
        }
        Some(_) => return Err(bad("clearance")),
    };
    let search = Search {
        rules,
        picks,
        reference,
        base: Base {
            speed,
            profile,
            model: model.unwrap_or_default(),
        },
        tol: tolerance,
        l_min: payload.get("lMin").and_then(Json::as_f64).unwrap_or(0.0).max(0.0),
        clearance,
    };

    if same_circle(&search.picks) {
        return Ok(json!({"error": "reconnect_error_same_circle", "params": {}}));
    }
    let two_arcs = search.picks.iter().all(|p| p.radius.is_some());
    let mut variants: Vec<Variant> = [Lengths::Regular, Lengths::Minimum, Lengths::None]
        .into_iter()
        .map(|lengths| Variant {
            lengths,
            arc_join: ArcJoin::Straight,
        })
        .collect();
    if two_arcs {
        variants.push(Variant {
            lengths: Lengths::None,
            arc_join: ArcJoin::Transition,
        });
    }
    let mut best: Vec<Solution> = Vec::new();
    let mut regular: Option<Cand> = None;
    for v in variants {
        let cand = if two_arcs {
            search.build(v, None, Level::Fine)
        } else {
            search.best_radius(v, fixed)
        };
        if let Some(cand) = cand {
            if v.lengths == Lengths::Regular && v.arc_join == ArcJoin::Straight {
                // Kept for the fit; the answer gets its own copy.
                let fitted_from = Cand {
                    sol: cand.sol.clone(),
                    spec: cand.spec.clone(),
                    dev: Deviation {
                        max: cand.dev.max,
                        rms: cand.dev.rms,
                        offsets: None,
                    },
                    poly: Vec::new(),
                    spacing: None,
                    ..cand
                };
                regular = Some(fitted_from);
            }
            best.push(search.finish(cand));
        }
    }
    // The transitions as long as fits best, from the best at the Regellänge
    // on — the old ones are often longer than the rules ask (Entscheidung 202).
    if let Some(regular) = regular.filter(|_| speed > 0.0)
        && let Some(cand) = search.best_fitted(&regular, fixed)
    {
        best.push(search.finish(cand));
    }
    if best.is_empty() {
        return Ok(json!({"error": "reconnect_error_no_fit", "params": {}}));
    }
    best.sort_by(|a, b| rank_answer(rules, a).partial_cmp(&rank_answer(rules, b)).unwrap_or(std::cmp::Ordering::Equal));
    Ok(json!({"solutions": best.iter().map(Solution::to_json).collect::<Vec<_>>(), "tolerance": tolerance}))
}
