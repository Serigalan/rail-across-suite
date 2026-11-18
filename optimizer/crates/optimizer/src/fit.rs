// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An imported track fitted to hold together (decision 0006): what a file
//! states element by element — each from its own point, a length rounded to the
//! millimetre, a bearing that does not quite meet the next — made into one chain
//! the document of core 6.0 takes, with no gap and no kink after a curve.
//!
//! The coordinates are what is fixed; bearings and stations are fitted to them:
//!
//! * a straight runs through its own two points; two straights in a row become
//!   one where the joint between them lies less than [`MERGE_TOL`] off the line
//!   through their outer points — and they carry the same values —, else they
//!   stay two with a kink between them;
//! * a curve group between two straights is fitted between their lines with its
//!   radii, its transitions' lengths and, but for the last, its arcs' sweeps —
//!   any run of arcs and transitions, a transition in pieces too; a reverse curve
//!   by [`fit_s_group`]. An arc's length changes a little.
//!   A compound or reverse curve is only fitted where it stays within
//!   [`GROUP_TOL`] of where it lay;
//! * a track's first and last point stay exactly. Where it begins or ends with a
//!   curve, its first point and bearing stay, and the curve runs into the line of
//!   the next straight: the sweep of its arc nearest the straight is solved so
//!   the straight, still through its far point, leaves the curve tangentially;
//! * the routes of a switch keep their form: they are not fitted, and their ends
//!   are fixed like a track's.
//!
//! What cannot be read so — transitions in a row, a curve between two fixed
//! ends — stays as it lies and is named in the notes.

use crate::geometry::{Profile, Pt, Segment, arc_forward, dir_of, fit_s_group, max_dist_to_polyline, sample_arc, sample_transition, transition_end};
use serde_json::{Map, Value as Json, json};

/// Two straights whose joint lies less than this off the line through their outer points are one [m].
pub const MERGE_TOL: f64 = 0.005;
/// How far a compound or reverse curve may move to be fitted [m].
pub const GROUP_TOL: f64 = 0.001;
/// An element shorter than this is a sliver a cut left: it goes, its neighbour ends where it ended [m].
pub const SLIVER: f64 = 0.001;
/// Radii this close are one [relative].
const RADIUS_TOL: f64 = 1e-3;
/// Two bearings this close are one [°]: no kink.
const SAME_BEARING: f64 = 1e-7;
/// What an element states of its place and shape; the rest is carried as it came.
const GEOMETRY: &[&str] = &[
    "startNode",
    "endNode",
    "bearing",
    "endBearing",
    "length",
    "geometry",
    "renderCoords",
    "coordinates",
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Straight,
    Arc,
    Transition(Profile),
}

/// An element as the fit works on it: its kind and design values, where it begins, and what it came as.
#[derive(Debug, Clone)]
struct El {
    kind: Kind,
    start: Pt,
    /// The stated end point: a straight's line runs through it.
    end: Pt,
    bearing: f64,
    length: f64,
    r1: Option<f64>,
    r2: Option<f64>,
    fixed: bool,
    src: Json,
}

fn num(v: &Json, key: &str) -> Option<f64> {
    v.get(key).and_then(Json::as_f64)
}

fn point(v: &Json, key: &str) -> Option<Pt> {
    let p = v.get(key)?.as_array()?;
    Some((p.first()?.as_f64()?, p.get(1)?.as_f64()?))
}

fn bearing_of(from: Pt, to: Pt) -> f64 {
    (to.0 - from.0).atan2(to.1 - from.1).to_degrees().rem_euclid(360.0)
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn turn(a: f64, b: f64) -> f64 {
    ((a - b + 540.0).rem_euclid(360.0) - 180.0).abs()
}

fn radius(r: Option<f64>) -> Option<f64> {
    r.filter(|r| *r != 0.0 && r.is_finite())
}

fn same_radius(a: Option<f64>, b: Option<f64>) -> bool {
    match (radius(a), radius(b)) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= RADIUS_TOL * a.abs().max(b.abs()),
        _ => false,
    }
}

impl El {
    fn read(src: &Json) -> Option<El> {
        let kind = match src.get("elementType").and_then(Json::as_str)? {
            "straight" => Kind::Straight,
            "arc" => Kind::Arc,
            "transition" => Kind::Transition(Profile::parse(src.get("transitionType").and_then(Json::as_str).unwrap_or("clothoid"))),
            _ => return None,
        };
        let (start, end) = (point(src, "startNode")?, point(src, "endNode")?);
        let (r1, r2) = match kind {
            Kind::Arc => (radius(num(src, "radius")), radius(num(src, "radius"))),
            Kind::Transition(_) => (radius(num(src, "r1")), radius(num(src, "r2"))),
            Kind::Straight => (None, None),
        };
        if kind == Kind::Arc && r1.is_none() {
            return None;
        }
        Some(El {
            kind,
            start,
            end,
            bearing: num(src, "bearing").unwrap_or_else(|| bearing_of(start, end)),
            length: num(src, "length")?,
            r1,
            r2,
            fixed: src.get("switchId").is_some_and(|v| !v.is_null()),
            src: src.clone(),
        })
    }

    fn is_straight(&self) -> bool {
        self.kind == Kind::Straight
    }

    /// Where the element's values lead from its start, and on which bearing.
    fn walk(&self) -> (Pt, f64) {
        walk(self.kind, self.start, self.bearing, self.length, self.r1, self.r2)
    }

    /// What the element carries beyond its place: two straights are one only where this is the same.
    fn values(&self) -> Map<String, Json> {
        self.src
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, _)| !GEOMETRY.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

fn walk(kind: Kind, start: Pt, bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>) -> (Pt, f64) {
    match kind {
        Kind::Straight => {
            let d = dir_of(bearing);
            ((start.0 + length * d.0, start.1 + length * d.1), bearing)
        }
        Kind::Arc => {
            let r = r1.unwrap_or(f64::INFINITY);
            arc_forward(start.0, start.1, bearing, r, length / r.abs())
        }
        Kind::Transition(profile) => {
            let (e, n, b) = transition_end(start.0, start.1, bearing, length, r1, r2, profile);
            ((e, n), b)
        }
    }
}

/// The element run the other way: from its end, on its end bearing turned round, curving the other way.
fn reversed(el: &El) -> El {
    let (end, end_bearing) = el.walk();
    El {
        start: end,
        end: el.start,
        bearing: (end_bearing + 180.0).rem_euclid(360.0),
        r1: el.r2.map(|r| -r),
        r2: el.r1.map(|r| -r),
        ..el.clone()
    }
}

/// Points along a chain, close enough to measure millimetres against.
fn sample(els: &[El]) -> Vec<Pt> {
    let mut pts: Vec<Pt> = Vec::new();
    for el in els {
        let (end, _) = el.walk();
        let part: Vec<Pt> = match el.kind {
            Kind::Straight => vec![el.start, end],
            Kind::Arc => sample_arc(el.start, end, el.r1.unwrap_or(f64::INFINITY), 0.002),
            Kind::Transition(profile) => sample_transition(el.start.0, el.start.1, el.bearing, el.length, el.r1, el.r2, profile, None),
        };
        let skip = usize::from(!pts.is_empty());
        pts.extend(part.into_iter().skip(skip));
    }
    pts
}

/// The stated chain as points: each element from its own point.
fn stated(els: &[El]) -> Vec<Pt> {
    sample(els)
}

// ── Straights ───────────────────────────────────────────────────────────────

/// The elements without slivers: one shorter than SLIVER goes, the element before it ending where it ended —
/// the first one's place taken by the element after it. Its direction, read off two points a hair apart, is
/// noise, and would be a kink.
fn without_slivers(els: Vec<El>, notes: &mut Vec<String>) -> Vec<El> {
    if els.len() < 2 {
        return els;
    }
    let mut out: Vec<El> = Vec::with_capacity(els.len());
    let mut dropped = 0;
    let mut carry: Option<Pt> = None;
    for mut el in els {
        if el.length < SLIVER {
            dropped += 1;
            match out.last_mut() {
                Some(prev) => {
                    prev.end = el.end;
                    if prev.is_straight() {
                        prev.bearing = bearing_of(prev.start, prev.end);
                        prev.length = dist(prev.start, prev.end);
                    }
                }
                None => carry = Some(el.start),
            }
            continue;
        }
        if let Some(start) = carry.take() {
            el.start = start;
        }
        out.push(el);
    }
    if out.is_empty() {
        return out;
    }
    if dropped > 0 {
        notes.push(format!("{dropped} Elemente unter {:.0} mm Länge entfernt", SLIVER * 1000.0));
    }
    out
}

/// Straights in a row made one where their joint lies within MERGE_TOL of the line through their outer points
/// and they carry the same values.
fn merged(els: Vec<El>, notes: &mut Vec<String>) -> Vec<El> {
    let mut out: Vec<El> = Vec::with_capacity(els.len());
    let mut joined = 0;
    for el in els {
        if let Some(prev) = out.last_mut()
            && prev.is_straight()
            && el.is_straight()
            && !prev.fixed
            && !el.fixed
            && prev.values() == el.values()
        {
            let d = dir_of(bearing_of(prev.start, el.end));
            let off = ((prev.end.0 - prev.start.0) * d.1 - (prev.end.1 - prev.start.1) * d.0).abs();
            if off < MERGE_TOL {
                prev.end = el.end;
                prev.bearing = bearing_of(prev.start, prev.end);
                prev.length = dist(prev.start, prev.end);
                joined += 1;
                continue;
            }
        }
        out.push(el);
    }
    if joined > 0 {
        notes.push(format!(
            "{joined} Geradenstöße unter {:.0} mm zu einer Geraden zusammengelegt",
            MERGE_TOL * 1000.0
        ));
    }
    out
}

// ── Curve groups ────────────────────────────────────────────────────────────

/// A run of curves as the fit walks it: each element in its order with its values, an arc by its sweep. Any
/// sequence of arcs and transitions — a transition in pieces too —, as long as it holds an arc.
struct Run {
    pieces: Vec<El>,
    /// The arc whose sweep closes the turn: the last one.
    last_arc: usize,
}

/// A run of curves, or why not.
fn read_run(els: &[El], run: std::ops::Range<usize>) -> Result<Run, &'static str> {
    let pieces: Vec<El> = els[run].to_vec();
    if pieces.iter().any(|e| e.fixed) {
        return Err("Weichenstrang");
    }
    if pieces.iter().any(El::is_straight) {
        return Err("Gerade in der Bogenfolge");
    }
    let last_arc = pieces.iter().rposition(|e| e.kind == Kind::Arc).ok_or("kein Bogen")?;
    Ok(Run { pieces, last_arc })
}

/// The bearing change of a piece [°], + to the right.
fn turn_of(el: &El) -> f64 {
    let k = |r: Option<f64>| r.map_or(0.0, |r| 1.0 / r);
    match el.kind {
        Kind::Arc => (el.length * k(el.r1)).to_degrees(),
        Kind::Transition(_) => (el.length * (k(el.r1) + k(el.r2)) / 2.0).to_degrees(),
        Kind::Straight => 0.0,
    }
}

/// The run walked from `from` on `bearing`, its last arc this long: each piece placed, and where it ends.
fn walked(run: &Run, from: Pt, bearing: f64, last_len: f64) -> (Vec<El>, Pt, f64) {
    let (mut at, mut brg) = (from, bearing);
    let mut out = Vec::with_capacity(run.pieces.len());
    for (k, piece) in run.pieces.iter().enumerate() {
        let mut el = piece.clone();
        if k == run.last_arc {
            el.length = last_len;
        }
        el.start = at;
        el.bearing = brg;
        let (end, b) = el.walk();
        el.end = end;
        (at, brg) = (end, b);
        out.push(el);
    }
    (out, at, brg)
}

/// How far a run moved: its new points from the old chain and the old points from the new one.
fn moved(before: &[El], after: &[El], with: (Pt, Pt)) -> f64 {
    let new_pts = sample(after);
    let mut new_poly = vec![with.0];
    new_poly.extend(&new_pts);
    new_poly.push(with.1);
    let mut old_poly = vec![with.0];
    old_poly.extend(stated(before));
    old_poly.push(with.1);
    max_dist_to_polyline(&new_pts, &old_poly).max(max_dist_to_polyline(&stated(before), &new_poly))
}

/// A two-arc reverse curve in the S solver's form: its signed radii, transition lengths and profiles.
fn s_form(run: &Run) -> Option<(Vec<f64>, Vec<f64>, Vec<Profile>)> {
    let (mut signed, mut trans_l, mut types) = (vec![], vec![0.0], vec![Profile::Clothoid]);
    for el in &run.pieces {
        match el.kind {
            Kind::Arc => {
                signed.push(el.r1?);
                trans_l.push(0.0);
                types.push(Profile::Clothoid);
            }
            Kind::Transition(profile) => {
                let slot = signed.len();
                if trans_l[slot] > 0.0 || !same_radius(el.r1, signed.last().copied()) {
                    return None;
                }
                trans_l[slot] = el.length;
                types[slot] = profile;
            }
            Kind::Straight => return None,
        }
    }
    (signed.len() == 2 && (signed[0] > 0.0) != (signed[1] > 0.0)).then_some((signed, trans_l, types))
}

/// A run between two straights fitted between their lines: its elements and where it meets each line.
///
/// The last arc's sweep closes the turn the two lines make, the other pieces keep theirs; the run is then
/// slid along the first line until it ends on the second. A reverse curve, whose two straights run nearly
/// parallel, is solved by the S solver instead. A run of more than one arc is fitted only within GROUP_TOL.
fn fit_between(els: &[El], range: std::ops::Range<usize>, a: &El, b: &El) -> Result<(Vec<El>, Pt, Pt), String> {
    let run = read_run(els, range.clone()).map_err(str::to_string)?;
    let (b1, b2) = (bearing_of(a.start, a.end), bearing_of(b.start, b.end));
    let (d1, d2) = (dir_of(b1), dir_of(b2));
    let arcs = run.pieces.iter().filter(|e| e.kind == Kind::Arc).count();
    let reverse = run.pieces.iter().any(|e| e.r1.is_some_and(|r| r > 0.0) || e.r2.is_some_and(|r| r > 0.0))
        && run.pieces.iter().any(|e| e.r1.is_some_and(|r| r < 0.0) || e.r2.is_some_and(|r| r < 0.0));
    let (out, cl_start, cl_end) = if let (true, Some((signed, trans_l, types))) = (reverse, s_form(&run)) {
        let fit = fit_s_group(a.start, d1, b1, b.end, d2, b2, &signed, &trans_l, &types, dist(a.start, a.end)).ok_or("keine Lösung zwischen den Geraden")?;
        let mut pieces = run.pieces.clone();
        for (el, seg) in pieces.iter_mut().zip(&fit.segments) {
            match *seg {
                Segment::Arc {
                    start,
                    signed_r,
                    sweep,
                    bearing,
                    ..
                } => {
                    el.start = start;
                    el.bearing = bearing;
                    el.length = (sweep * signed_r).abs();
                }
                Segment::Transition { start, bearing, .. } => {
                    el.start = start;
                    el.bearing = bearing;
                }
            }
            el.end = el.walk().0;
        }
        (pieces, fit.cl_start, fit.cl_end)
    } else {
        // The turn the lines make, less what every piece but the last arc turns.
        let wanted = (b2 - b1 + 540.0).rem_euclid(360.0) - 180.0;
        let fixed: f64 = run.pieces.iter().enumerate().filter(|(k, _)| *k != run.last_arc).map(|(_, e)| turn_of(e)).sum();
        let arc = &run.pieces[run.last_arc];
        let r = arc.r1.unwrap_or(f64::INFINITY);
        let last_len = ((wanted - fixed).to_radians() * r).max(0.0);
        if last_len <= 0.0 || last_len.is_nan() || (wanted - fixed).signum() != r.signum() {
            return Err("die Bögen passen nicht zwischen die Geraden".into());
        }
        let (_, rel_end, _) = walked(&run, (0.0, 0.0), b1, last_len);
        // Slide along the first line until the end lies on the second.
        let n2 = (-d2.1, d2.0);
        let den = d1.0 * n2.0 + d1.1 * n2.1;
        if den.abs() < 1e-9 {
            return Err("die Geraden laufen parallel".into());
        }
        let s = ((b.end.0 - a.start.0 - rel_end.0) * n2.0 + (b.end.1 - a.start.1 - rel_end.1) * n2.1) / den;
        let anchor = (a.start.0 + s * d1.0, a.start.1 + s * d1.1);
        let (pieces, end, _) = walked(&run, anchor, b1, last_len);
        (pieces, anchor, end)
    };
    let entry = (cl_start.0 - a.start.0) * d1.0 + (cl_start.1 - a.start.1) * d1.1;
    let exit = (b.end.0 - cl_end.0) * d2.0 + (b.end.1 - cl_end.1) * d2.1;
    if entry <= 0.0 || exit <= 0.0 {
        return Err("die Bögen passen nicht zwischen die Geraden".into());
    }
    if arcs > 1 {
        let off = moved(&els[range], &out, (a.start, b.end));
        if off > GROUP_TOL {
            return Err(format!(
                "{} würde sich um {:.1} mm verschieben",
                if reverse { "S-Kurve" } else { "Korbbogen" },
                off * 1000.0
            ));
        }
    }
    Ok((out, cl_start, cl_end))
}

/// A run of curves from a fixed point and bearing into the line of the next straight, which keeps its far
/// point `far`: the length of the run's last arc solved so the straight leaves the curve tangentially. The
/// run's elements and where the straight now begins.
fn fit_from(els: &[El], range: std::ops::Range<usize>, from: Pt, bearing: f64, far: Pt) -> Result<(Vec<El>, Pt), String> {
    let run = read_run(els, range).map_err(str::to_string)?;
    let lateral = |len: f64| -> (f64, Vec<El>, Pt, f64) {
        let (pieces, end, brg) = walked(&run, from, bearing, len);
        let d = dir_of(brg);
        ((far.0 - end.0) * d.1 - (far.1 - end.1) * d.0, pieces, end, brg)
    };
    // Bracket around the length the arc had, then bisect.
    let len0 = run.pieces[run.last_arc].length;
    let r = run.pieces[run.last_arc].r1.unwrap_or(f64::INFINITY).abs();
    let span = (len0 * 0.5).max(0.02 * r);
    let steps = 64;
    let mut bracket = None;
    let mut prev: Option<(f64, f64)> = None;
    for k in 0..=steps {
        let len = (len0 - span + 2.0 * span * f64::from(k) / f64::from(steps)).max(1e-6);
        let f = lateral(len).0;
        if let Some((l, pf)) = prev
            && (f < 0.0) != (pf < 0.0)
        {
            bracket = Some((l, len));
            break;
        }
        prev = Some((len, f));
    }
    let (mut lo, mut hi) = bracket.ok_or("der Bogen läuft nicht in die Gerade")?;
    let f_lo = lateral(lo).0;
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if (lateral(mid).0 < 0.0) == (f_lo < 0.0) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (f, pieces, end, brg) = lateral(0.5 * (lo + hi));
    if f.abs() > 1e-6 {
        return Err("der Bogen läuft nicht in die Gerade".into());
    }
    // The straight still has to run forward from the curve to its far point.
    let d = dir_of(brg);
    if (far.0 - end.0) * d.0 + (far.1 - end.1) * d.1 <= 0.0 {
        return Err("die Gerade wäre zu kurz".into());
    }
    Ok((pieces, end))
}

/// The same as fit_from, from the run's far end backwards: a fixed point and the bearing the track ends on.
fn fit_to(els: &[El], run: std::ops::Range<usize>, to: Pt, bearing: f64, far: Pt) -> Result<(Vec<El>, Pt), String> {
    let back: Vec<El> = els[run].iter().rev().map(reversed).collect();
    let (fitted, start) = fit_from(&back, 0..back.len(), to, (bearing + 180.0).rem_euclid(360.0), far)?;
    Ok((fitted.iter().rev().map(reversed).collect(), start))
}

// ── A track ─────────────────────────────────────────────────────────────────

/// One stretch between fixed ends — a track's ends, a switch's routes — fitted in place; notes added.
fn fit_stretch(els: &mut [El], from: Option<(Pt, f64)>, to: Option<(Pt, f64)>, notes: &mut Vec<String>) {
    let straights: Vec<usize> = (0..els.len()).filter(|&i| els[i].is_straight()).collect();
    let (Some(&first), Some(&last)) = (straights.first(), straights.last()) else {
        if els.iter().any(|e| !e.is_straight()) {
            notes.push("Bögen ohne Gerade zwischen zwei festen Enden – unverändert".into());
        }
        return;
    };
    // The ends first: they turn the first and last straight about their far points.
    let mut lead: Option<Vec<El>> = None;
    let mut trail: Option<Vec<El>> = None;
    for _ in 0..20 {
        let before = (els[first].start, els[last].end);
        if first > 0
            && let Some((p, b)) = from
        {
            match fit_from(els, 0..first, p, b, els[first].end) {
                Ok((run, start)) => {
                    els[first].start = start;
                    lead = Some(run);
                }
                Err(why) => {
                    notes.push(format!("Bögen am Anfang unverändert: {why}"));
                    break;
                }
            }
        }
        if last + 1 < els.len()
            && let Some((p, b)) = to
        {
            match fit_to(els, last + 1..els.len(), p, b, els[last].start) {
                Ok((run, end)) => {
                    els[last].end = end;
                    trail = Some(run);
                }
                Err(why) => {
                    notes.push(format!("Bögen am Ende unverändert: {why}"));
                    break;
                }
            }
        }
        // Only one straight between both: each end turns it, so turn again until it stands.
        if first != last || dist(before.0, els[first].start) + dist(before.1, els[last].end) < 1e-9 {
            break;
        }
    }
    if let Some(run) = lead {
        els[..first].clone_from_slice(&run);
    }
    if let Some(run) = trail {
        els[last + 1..].clone_from_slice(&run);
    }
    // Then every group between two straights; a kink between two straights meets where the first ends.
    for w in straights.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b == a + 1 {
            els[b].start = els[a].end;
            continue;
        }
        let (sa, sb) = (els[a].clone(), els[b].clone());
        match fit_between(els, a + 1..b, &sa, &sb) {
            Ok((run, cl_start, cl_end)) => {
                els[a + 1..b].clone_from_slice(&run);
                els[a].end = cl_start;
                els[b].start = cl_end;
            }
            Err(why) => notes.push(format!("Bögen nach {:.1} m unverändert: {why}", station_of(els, a + 1))),
        }
    }
    for i in straights {
        els[i].bearing = bearing_of(els[i].start, els[i].end);
        els[i].length = dist(els[i].start, els[i].end);
    }
}

fn station_of(els: &[El], i: usize) -> f64 {
    els[..i].iter().map(|e| e.length).sum()
}

/// A track's elements fitted (see the module): the elements and what was left as it lay.
pub fn fit_elements(given: &[Json]) -> Result<(Vec<Json>, Vec<String>), String> {
    let read: Option<Vec<El>> = given.iter().map(El::read).collect();
    let Some(read) = read else {
        return Err("ein Element ohne Lage oder Werte".into());
    };
    let mut notes = Vec::new();
    let mut els = merged(without_slivers(read, &mut notes), &mut notes);
    // The stretches between the routes of switches, each between two fixed ends.
    let mut i = 0;
    while i < els.len() {
        if els[i].fixed {
            i += 1;
            continue;
        }
        let a = i;
        while i < els.len() && !els[i].fixed {
            i += 1;
        }
        let from = if a == 0 {
            Some((els[0].start, els[0].bearing))
        } else {
            let (_, b) = els[a - 1].walk();
            Some((els[a - 1].end, b))
        };
        let to = if i == els.len() {
            let (_, b) = els[i - 1].walk();
            Some((els[i - 1].end, b))
        } else {
            Some((els[i].start, els[i].bearing))
        };
        fit_stretch(&mut els[a..i], from, to, &mut notes);
    }
    Ok((written(&els), notes))
}

/// The elements as the app takes them: each as it came, at its new place.
fn written(els: &[El]) -> Vec<Json> {
    els.iter()
        .enumerate()
        .map(|(i, el)| {
            let mut o = el.values();
            let end = el.end;
            o.insert("startNode".into(), json!([el.start.0, el.start.1]));
            o.insert("endNode".into(), json!([end.0, end.1]));
            o.insert("bearing".into(), json!(el.bearing));
            o.insert("length".into(), json!(el.length));
            if el.is_straight()
                && let Some(next) = els.get(i + 1).filter(|n| n.is_straight())
                && turn(next.bearing, el.bearing) >= SAME_BEARING
            {
                o.insert("endBearing".into(), json!(next.bearing));
            }
            Json::Object(o)
        })
        .collect()
}

/// The answer to a fit request. Body: `{"tracks": [{"id", "elements"}]}` — the elements in the app's form,
/// each track in its own plane. Answers `{"tracks": [{"id", "elements", "notes", "moved"}]}`: the fitted
/// elements, what stayed as it lay, and how far the chain moved from the points the file stated [m]; a track
/// that cannot be read comes back with its elements as they were and why.
pub fn fit_payload(payload: &Json) -> Result<Json, String> {
    let tracks = payload.get("tracks").and_then(Json::as_array).ok_or("invalid request: no tracks")?;
    let out: Vec<Json> = tracks
        .iter()
        .map(|t| {
            let given = t.get("elements").and_then(Json::as_array).cloned().unwrap_or_default();
            match fit_elements(&given) {
                Ok((elements, notes)) => {
                    let before: Vec<El> = given.iter().filter_map(El::read).collect();
                    let after: Vec<El> = elements.iter().filter_map(El::read).collect();
                    let shift = max_dist_to_polyline(&before.iter().flat_map(|e| [e.start, e.end]).collect::<Vec<_>>(), &sample(&after));
                    json!({ "id": t.get("id"), "elements": elements, "notes": notes, "moved": shift })
                }
                Err(why) => json!({ "id": t.get("id"), "elements": given, "notes": [why], "moved": 0.0 }),
            }
        })
        .collect();
    Ok(json!({ "tracks": out }))
}
