// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Track spacing of a splice to a neighbouring track — and the largest radius
//! that keeps it (the former Python service's clearance.py, ported).
//!
//! The splice dialog may name a track the new geometry has to keep its
//! distance to (Entscheidung 167). The distance asked for is the user's
//! minimum spacing `dMin`, widened where cant leans the two clearance profiles
//! towards each other (Entscheidung 168, read geometrically): both profiles are
//! turned by their cant about the running circle of the rail that stays down,
//! as the cross section draws them, and the widening is how much further apart
//! the two centre lines then have to be for the turned outlines not to
//! overlap, against the same at no cant. Both tracks are taken at the same
//! height of their lower rail — the new track has no gradient yet.
//!
//! What is checked is what the splice inserts: every element of role 'new',
//! at 1 m steps, each with its cant — linear over a clothoid ramp and the Bloss
//! curve over a Bloss one, the cant of the picked arcs on the elements they
//! keep. The neighbour comes as points along its axis with its cant (the app's
//! sign: positive raises the left rail), in the plane of the splice.

use crate::geometry::{Profile, Pt, sample_arc, sample_transition};
use crate::splice::{El, Role};
use serde_json::{Value as Json, json};
use std::cell::RefCell;
use std::collections::HashMap;

const RUNNING_CIRCLE_DISTANCE: f64 = 1500.0; // mm, the lever of the cant
const HALF_RUNNING: f64 = RUNNING_CIRCLE_DISTANCE / 2.0;
/// Height step the two outlines are compared at [mm].
const Z_STEP: f64 = 10.0;
/// Step along the new elements [m].
const SAMPLE_STEP: f64 = 1.0;
/// How far beyond the two picked elements the neighbour is still read [m].
const REF_MARGIN: f64 = 100.0;
/// The radius search: from this down …
pub const R_MAX: f64 = 50000.0;
/// … to this, in steps of this ratio, then halved to the metre.
pub const R_MIN: f64 = 50.0;
const R_RATIO: f64 = 0.92;

// ── the outline turned by the cant ───────────────────────────────────────────

/// The closed outline from its stated half ([y, z] in mm, y ≥ 0), mirrored at the centre line.
fn profile_ring(half: &[Pt]) -> Vec<Pt> {
    let mut back: Vec<Pt> = half.iter().rev().map(|&(y, z)| (if y.abs() > 1e-9 { -y } else { 0.0 }, z)).collect();
    if half.last().is_some_and(|p| p.0.abs() <= 1e-9) {
        back.remove(0);
    }
    let mut ring: Vec<Pt> = half.to_vec();
    ring.extend(back);
    if ring.first() != ring.last() {
        ring.push(ring[0]);
    }
    ring
}

/// The outline turned by `cant` [mm, app sign] about the running circle that stays down.
fn turned(ring: &[Pt], cant: f64) -> Vec<Pt> {
    let angle = -(cant / RUNNING_CIRCLE_DISTANCE).clamp(-1.0, 1.0).asin();
    let py = if cant < 0.0 { -HALF_RUNNING } else { HALF_RUNNING };
    let (c, s) = (angle.cos(), angle.sin());
    ring.iter().map(|&(y, z)| ((y - py) * c - z * s + py, (y - py) * s + z * c)).collect()
}

/// Per height the outline's [y_min, y_max], or None where it does not reach.
fn spans(ring: &[Pt], zs: &[f64]) -> Vec<Option<(f64, f64)>> {
    zs.iter()
        .map(|&z| {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for w in ring.windows(2) {
                let ((y1, z1), (y2, z2)) = (w[0], w[1]);
                if (z1 <= z && z <= z2) || (z2 <= z && z <= z1) {
                    let ys = if (z2 - z1).abs() < 1e-12 {
                        [y1, y2]
                    } else {
                        [y1 + (y2 - y1) * (z - z1) / (z2 - z1); 2]
                    };
                    for y in ys {
                        lo = lo.min(y);
                        hi = hi.max(y);
                    }
                }
            }
            (lo <= hi).then_some((lo, hi))
        })
        .collect()
}

/// Per height the outline's [y_min, y_max] at one cant.
type Spans = std::rc::Rc<Vec<Option<(f64, f64)>>>;

/// How far apart two centre lines have to be for the two turned outlines to clear each other.
pub struct Spacing {
    ring: Vec<Pt>,
    zs: Vec<f64>,
    cache: RefCell<HashMap<i64, Spans>>,
    base: f64,
}

impl Spacing {
    pub fn new(half_profile: &[Pt]) -> Spacing {
        let ring = profile_ring(half_profile);
        let top = ring.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) + RUNNING_CIRCLE_DISTANCE;
        let count = ((top + RUNNING_CIRCLE_DISTANCE) / Z_STEP) as usize + 1;
        let zs = (0..count).map(|i| -RUNNING_CIRCLE_DISTANCE + i as f64 * Z_STEP).collect();
        let mut spacing = Spacing {
            ring,
            zs,
            cache: RefCell::new(HashMap::new()),
            base: 0.0,
        };
        spacing.base = spacing.needed(0.0, 0.0, 1);
        spacing
    }

    fn span_of(&self, cant: f64) -> Spans {
        let key = (cant * 10.0).round() as i64;
        self.cache
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| std::rc::Rc::new(spans(&turned(&self.ring, key as f64 / 10.0), &self.zs)))
            .clone()
    }

    /// Centre distance [mm] at which the outline of a track with `cant_there`,
    /// lying to the right of this one (`side` +1) or to its left (−1) and read
    /// in this track's frame, just clears this one's with `cant_here`.
    fn needed(&self, cant_here: f64, cant_there: f64, side: i32) -> f64 {
        let (a, b) = (self.span_of(cant_here), self.span_of(cant_there));
        let mut worst = f64::NEG_INFINITY;
        for (sa, sb) in a.iter().zip(b.iter()) {
            if let (Some(sa), Some(sb)) = (sa, sb) {
                worst = worst.max(if side > 0 { sa.1 - sb.0 } else { sb.1 - sa.0 });
            }
        }
        worst
    }

    /// What the cant adds to the spacing [m] — never less than nothing.
    pub fn widening(&self, cant_here: f64, cant_there: f64, side: i32) -> f64 {
        (self.needed(cant_here, cant_there, side) - self.base).max(0.0) / 1000.0
    }
}

// ── the new elements, sampled with their cant ────────────────────────────────

fn bloss(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Cant at both ends of every element: arcs their own, straights none,
/// transitions ramping between their neighbours.
fn element_cants(chain: &[El], cant_of_arc: &dyn Fn(&El) -> f64) -> Vec<(f64, f64)> {
    let own: Vec<Option<(f64, f64)>> = chain
        .iter()
        .map(|el| match el.kind {
            1 => {
                let c = cant_of_arc(el);
                Some((c, c))
            }
            0 => Some((0.0, 0.0)),
            _ => None,
        })
        .collect();
    (0..chain.len())
        .map(|i| {
            own[i].unwrap_or_else(|| {
                let before = if i > 0 { own[i - 1].map_or(0.0, |c| c.1) } else { 0.0 };
                let after = own.get(i + 1).copied().flatten().map_or(0.0, |c| c.0);
                (before, after)
            })
        })
        .collect()
}

/// A point of an inserted element: where, its bearing [°], its cant.
#[derive(Debug, Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
    bearing: f64,
    cant: f64,
}

/// Points of the elements the splice inserts.
fn sample_new(chain: &[El], cant_of_arc: &dyn Fn(&El) -> f64) -> Vec<Sample> {
    let ends = element_cants(chain, cant_of_arc);
    let mut out = Vec::new();
    for (el, &(c0, c1)) in chain.iter().zip(&ends) {
        if el.role != Role::New || el.length <= 0.0 {
            continue;
        }
        let n = ((el.length / SAMPLE_STEP).ceil() as usize).max(2);
        let pts: Vec<Pt> = match el.kind {
            0 => (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    (el.start.0 + (el.end.0 - el.start.0) * t, el.start.1 + (el.end.1 - el.start.1) * t)
                })
                .collect(),
            1 => {
                let r = el.radius.unwrap_or(0.0);
                sample_arc(el.start, el.end, r, SAMPLE_STEP / r.abs())
            }
            _ => sample_transition(el.start.0, el.start.1, el.bearing, el.length, el.r1, el.r2, el.profile, Some(n)),
        };
        let is_bloss = el.kind == 2 && el.profile == Profile::Bloss;
        let m = pts.len() - 1;
        for (i, &(x, y)) in pts.iter().enumerate() {
            let (j0, j1) = (i.saturating_sub(1), (i + 1).min(m));
            let bearing = (pts[j1].0 - pts[j0].0).atan2(pts[j1].1 - pts[j0].1).to_degrees().rem_euclid(360.0);
            let t = if m > 0 { i as f64 / m as f64 } else { 0.0 };
            let cant = c0 + (c1 - c0) * if is_bloss { bloss(t) } else { t };
            out.push(Sample { x, y, bearing, cant });
        }
    }
    out
}

// ── the neighbour ────────────────────────────────────────────────────────────

/// The axis of the track kept clear of: points with the cant there, cut to
/// what lies near the splice.
pub struct Neighbour {
    pts: Vec<(f64, f64, f64)>,
}

/// The nearest place of the neighbour to a sample.
struct Hit {
    dist: f64,
    side: i32,
    cant: f64,
    at: Pt,
}

impl Neighbour {
    /// `points` [e, n, cant?] along its axis, `bbox` (e0, n0, e1, n1) of the splice.
    pub fn new(points: &[(f64, f64, f64)], bbox: (f64, f64, f64, f64)) -> Neighbour {
        let (e0, n0, e1, n1) = bbox;
        let near = |p: &(f64, f64, f64)| e0 - REF_MARGIN <= p.0 && p.0 <= e1 + REF_MARGIN && n0 - REF_MARGIN <= p.1 && p.1 <= n1 + REF_MARGIN;
        let (Some(first), Some(last)) = (points.iter().position(near), points.iter().rposition(near)) else {
            return Neighbour { pts: Vec::new() };
        };
        let (lo, hi) = (first.saturating_sub(1), (last + 2).min(points.len()));
        Neighbour { pts: points[lo..hi].to_vec() }
    }

    /// The neighbour's axis from a request: [[e, n, cant], …], the cant optional.
    pub fn points_of(raw: &Json) -> Vec<(f64, f64, f64)> {
        raw.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        let p = p.as_array()?;
                        Some((p.first()?.as_f64()?, p.get(1)?.as_f64()?, p.get(2).and_then(Json::as_f64).unwrap_or(0.0)))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn nearest(&self, s: &Sample) -> Option<Hit> {
        if self.pts.len() < 2 {
            return None;
        }
        let mut best = (f64::INFINITY, 0, 0.0);
        for k in 0..self.pts.len() - 1 {
            let ((ax, ay, _), (bx, by, _)) = (self.pts[k], self.pts[k + 1]);
            let (dx, dy) = (bx - ax, by - ay);
            let den = dx * dx + dy * dy;
            let den = if den == 0.0 { 1.0 } else { den };
            let t = (((s.x - ax) * dx + (s.y - ay) * dy) / den).clamp(0.0, 1.0);
            let (ex, ey) = (s.x - ax - t * dx, s.y - ay - t * dy);
            let d2 = ex * ex + ey * ey;
            if d2 < best.0 {
                best = (d2, k, t);
            }
        }
        let (_, k, t) = best;
        let ((ax, ay, ca), (bx, by, cb)) = (self.pts[k], self.pts[k + 1]);
        let (qx, qy) = (ax + t * (bx - ax), ay + t * (by - ay));
        let b = s.bearing.to_radians();
        let right = (b.cos(), -b.sin());
        let lateral = (qx - s.x) * right.0 + (qy - s.y) * right.1;
        // Read the other way round, the neighbour's left is this track's
        // right: its cant changes sign in this frame.
        let along = b.sin() * (bx - ax) + b.cos() * (by - ay);
        let cant = ca + (cb - ca) * t;
        Some(Hit {
            dist: (qx - s.x).hypot(qy - s.y),
            side: if lateral >= 0.0 { 1 } else { -1 },
            cant: if along >= 0.0 { cant } else { -cant },
            at: (qx, qy),
        })
    }
}

// ── the check and the search ─────────────────────────────────────────────────

/// Math.round: halves up.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// How the app proposes the cant for a radius (rules/cant computeAutoC).
#[derive(Debug, Clone, Default)]
pub struct CantModel {
    pub coeff: f64,
    pub def_coeff: f64,
    pub def_min: f64,
    pub max: f64,
    pub step: f64,
}

impl CantModel {
    pub fn from_json(raw: &Json) -> Option<CantModel> {
        Some(CantModel {
            coeff: raw.get("coeff")?.as_f64()?,
            def_coeff: raw.get("defCoeff")?.as_f64()?,
            def_min: raw.get("defMin").and_then(Json::as_f64).unwrap_or(60.0),
            max: raw.get("max")?.as_f64()?,
            step: raw.get("step").and_then(Json::as_f64).unwrap_or(5.0),
        })
    }
}

/// The cant the app proposes for `radius` at `speed`, unsigned [mm].
pub fn auto_cant(speed: f64, radius: f64, model: &CantModel) -> f64 {
    let r = radius.abs();
    if r.is_nan() || r <= 0.0 || speed == 0.0 {
        return 0.0;
    }
    if js_round(model.def_coeff * speed * speed / r) < model.def_min {
        return 0.0;
    }
    model.max.min(js_round(model.coeff * speed * speed / r / model.step) * model.step)
}

/// The tightest place of the inserted elements — margin < 0 where the spacing is not kept.
#[derive(Debug, Clone)]
pub struct Worst {
    pub margin: f64,
    pub distance: f64,
    pub required: f64,
    pub cant_new: f64,
    pub cant_ref: f64,
    pub at: Pt,
    pub reference: Pt,
}

impl Worst {
    pub fn to_json(&self) -> serde_json::Map<String, Json> {
        let Json::Object(map) = json!({
            "margin": self.margin, "distance": self.distance, "required": self.required,
            "cantNew": self.cant_new, "cantRef": self.cant_ref,
            "at": [self.at.0, self.at.1], "ref": [self.reference.0, self.reference.1],
        }) else {
            unreachable!()
        };
        map
    }
}

/// The tightest place of the inserted elements, or None where no part of the neighbour lies near.
pub fn check(chain: &[El], neighbour: &Neighbour, spacing: &Spacing, d_min: f64, cant_of_arc: &dyn Fn(&El) -> f64) -> Option<Worst> {
    let mut worst: Option<Worst> = None;
    for sample in sample_new(chain, cant_of_arc) {
        let Some(hit) = neighbour.nearest(&sample) else { continue };
        let required = d_min + spacing.widening(sample.cant, hit.cant, hit.side);
        let margin = hit.dist - required;
        if worst.as_ref().is_none_or(|w| margin < w.margin) {
            worst = Some(Worst {
                margin,
                distance: hit.dist,
                required,
                cant_new: sample.cant,
                cant_ref: hit.cant,
                at: (sample.x, sample.y),
                reference: hit.at,
            });
        }
    }
    worst
}

/// What the dialog is told of the spacing a solution keeps: dMin, near, kept
/// and — where the neighbour lies near — the tightest place.
pub fn described(worst: Option<Worst>, d_min: f64) -> serde_json::Map<String, Json> {
    let mut info = serde_json::Map::new();
    info.insert("dMin".into(), json!(d_min));
    info.insert("near".into(), json!(worst.is_some()));
    match worst {
        Some(w) => {
            let kept = w.margin >= -1e-6;
            info.extend(w.to_json());
            info.insert("kept".into(), json!(kept));
        }
        None => {
            info.insert("kept".into(), json!(true));
        }
    }
    info
}

/// The largest whole-metre radius whose splice `build(r)` (None where it does
/// not fit) `keeps` the spacing — searched down from `r_max` on a geometric
/// grid, then halved between the last radius that failed and the first that
/// held. (radius, result) or None.
pub fn largest_radius<T>(mut build: impl FnMut(f64) -> Option<T>, keeps: impl Fn(&T) -> bool, r_max: f64) -> Option<(f64, T)> {
    let mut failed: Option<f64> = None;
    let mut r = r_max;
    while r >= R_MIN {
        if let Some(res) = build(r).filter(&keeps) {
            let Some(failed) = failed else {
                // The largest of all keeps it: in whole metres, as built.
                return Some(match build(r.floor()) {
                    Some(at) => (r.floor(), at),
                    None => (r, res),
                });
            };
            let mut ok = |r: f64| build(r).filter(&keeps);
            let (mut lo, mut hi, mut best) = (r, failed, res);
            while hi - lo > 0.5 {
                let mid = (lo + hi) / 2.0;
                match ok(mid) {
                    Some(got) => {
                        lo = mid;
                        best = got;
                    }
                    None => hi = mid,
                }
            }
            let found = lo.floor();
            return Some(match ok(found) {
                Some(final_) => (found, final_),
                None => (lo, best),
            });
        }
        failed = Some(r);
        r *= R_RATIO;
    }
    None
}
