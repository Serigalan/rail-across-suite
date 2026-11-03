// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A track as the optimizer reads and writes it (the former Python service's
//! track_io.py, ported): one object with scalar elements in the app's form —
//! startNode/endNode [E, N] in the track's own CRS (`epsg`), bearings in
//! degrees, lengths in metres. The elements are kept as they came: what a run
//! does not touch goes back exactly as it was.

use crate::geometry::{Profile, Pt, arc_center, arc_sweep, dir_of, sample_arc, sample_transition, transition_end};
use serde_json::{Value as Json, json};

/// Arcs that turn opposite ways need the S-curve solver, and it takes exactly
/// two of them.
const S_ARCS: usize = 2;

fn num(el: &Json, key: &str) -> Option<f64> {
    el.get(key).and_then(Json::as_f64)
}

fn node(el: &Json, key: &str) -> Pt {
    let n = &el[key];
    (n[0].as_f64().unwrap_or(0.0), n[1].as_f64().unwrap_or(0.0))
}

pub fn is_transition(el: &Json) -> bool {
    el.get("elementType").and_then(Json::as_i64) == Some(2)
}

pub fn is_arc(el: &Json) -> bool {
    num(el, "radius").is_some()
}

pub fn is_straight(el: &Json) -> bool {
    !is_arc(el) && !is_transition(el)
}

/// Is this element part of a turnout's own route? (switchModel.js)
fn is_switch_element(el: &Json) -> bool {
    el.get("switchBranch").is_some_and(|b| !b.is_null() && b != false && b != "" && b != 0)
}

/// Do both elements belong to one and the same turnout?
fn same_switch(a: &Json, b: &Json) -> bool {
    match (a.get("switchId"), b.get("switchId")) {
        (Some(x), Some(y)) => !x.is_null() && x == y,
        _ => false,
    }
}

fn bearing_from_nodes(el: &Json) -> f64 {
    let (s, e) = (node(el, "startNode"), node(el, "endNode"));
    (e.0 - s.0).atan2(e.1 - s.1).to_degrees().rem_euclid(360.0)
}

fn profile_of(el: &Json) -> Profile {
    if el.get("transitionType").and_then(Json::as_str) == Some("bloss") {
        Profile::Bloss
    } else {
        Profile::Clothoid
    }
}

/// Sample points of one original element (its own plane).
pub fn element_ref_points(el: &Json) -> Vec<Pt> {
    let (s, e) = (node(el, "startNode"), node(el, "endNode"));
    if let Some(r) = num(el, "radius") {
        return sample_arc(s, e, r, 0.02);
    }
    if is_transition(el) {
        let bearing = num(el, "bearing").unwrap_or_else(|| bearing_from_nodes(el));
        return sample_transition(
            s.0,
            s.1,
            bearing,
            num(el, "length").unwrap_or(0.0),
            num(el, "r1"),
            num(el, "r2"),
            profile_of(el),
            None,
        );
    }
    vec![s, e]
}

fn ref_polyline(els: &[Json], start: usize, end: usize) -> Vec<Pt> {
    let mut pts: Vec<Pt> = Vec::new();
    for el in els.iter().take(end + 1).skip(start) {
        let part = element_ref_points(el);
        let skip = usize::from(!pts.is_empty());
        pts.extend(part.into_iter().skip(skip));
    }
    pts
}

fn arc_sweep_mag(el: &Json) -> f64 {
    let (s, e, r) = (node(el, "startNode"), node(el, "endNode"), num(el, "radius").unwrap_or(0.0));
    arc_center(s, e, r).map_or(0.0, |c| arc_sweep(s, e, c, r).abs())
}

/// One arc of a group as it lies.
#[derive(Debug, Clone)]
pub struct ArcAlt {
    pub r_alt: f64,
    /// Cant is stored signed (following the curve); the run works with
    /// magnitudes and the sign is put back when the elements are built.
    pub u_alt: f64,
    pub sweep_alt: f64,
}

/// A curve group: straight – [T] – arc (– [T] – arc)* – [T] – straight.
#[derive(Debug, Clone)]
pub struct Group {
    pub entry_idx: usize,
    pub arc_idxs: Vec<usize>,
    pub t_idxs: Vec<Option<usize>>,
    pub exit_idx: usize,
    pub p1: Pt,
    pub d1: Pt,
    pub b1: f64,
    pub p2: Pt,
    pub d2: Pt,
    pub b2: f64,
    /// Which way each arc turns: for a same-side group the corner says it,
    /// an S-curve has to be told.
    pub signs: Vec<f64>,
    pub s_curve: bool,
    /// Where along the entry line the curve begins (the S-curve solver keeps it).
    pub s_entry: f64,
    pub arcs: Vec<ArcAlt>,
    pub types: Vec<Profile>,
    pub has_t: Vec<bool>,
    /// The ramps as they lie: reproducing the group means fitting it with these.
    pub t_len: Vec<f64>,
    /// The curve part runs through a turnout: the turnout's cant ceiling holds.
    pub on_switch: bool,
    /// Per slot: a curvature jump there lies inside one turnout (LP.KS.01 `switch_internal`).
    pub jump_internal: Vec<bool>,
    /// The design speed the bounding straights state for themselves.
    pub entry_speed: f64,
    pub exit_speed: f64,
    pub ref_poly: Vec<Pt>,
    pub ref_curve_pts: Vec<Pt>,
}

/// A stretch that cannot be fitted between two tangents, and why not.
struct Unreadable(&'static str);

fn next_straight(els: &[Json], mut i: usize, last: usize) -> usize {
    while i <= last && !is_straight(&els[i]) {
        i += 1;
    }
    i
}

fn read_group(els: &[Json], entry_idx: usize, mut i: usize, last: usize) -> Result<(Group, usize), Unreadable> {
    let mut arc_idxs = Vec::new();
    let mut t_idxs = Vec::new();
    if is_transition(&els[i]) {
        t_idxs.push(Some(i));
        i += 1;
    } else {
        t_idxs.push(None);
    }
    loop {
        if i > last || !is_arc(&els[i]) {
            return Err(Unreadable("Ein Übergangsbogen muss in einen Bogen laufen."));
        }
        arc_idxs.push(i);
        i += 1;
        if i <= last && is_transition(&els[i]) {
            t_idxs.push(Some(i));
            i += 1;
            if i <= last && is_arc(&els[i]) {
                continue;
            }
            break;
        }
        t_idxs.push(None);
        if i <= last && is_arc(&els[i]) {
            continue;
        }
        break;
    }
    if i > last || !is_straight(&els[i]) {
        return Err(Unreadable("Bögen müssen zwischen Geraden liegen."));
    }
    let right = |k: &usize| num(&els[*k], "radius").unwrap_or(0.0) > 0.0;
    let mixed = arc_idxs.iter().any(right) && !arc_idxs.iter().all(right);
    if mixed && arc_idxs.len() != S_ARCS {
        return Err(Unreadable("Mehr als zwei gegensinnige Bögen ohne Zwischengerade werden nicht unterstützt."));
    }
    Ok((build_group(els, entry_idx, arc_idxs, t_idxs, i), i))
}

/// A stretch the parser stepped over: from, to, why.
pub type Skipped = (usize, usize, String);

/// Split a track into curve groups. Compound curves (Korbbögen) form one
/// group. Read is the stretch between the first straight and the last; what
/// cannot be read there is stepped over to the next straight and stays
/// exactly as it lies (`skipped`), while the curves around it are optimized
/// as usual. An error only where nothing at all can be read.
pub fn parse_groups(els: &[Json], skipped: &mut Vec<Skipped>) -> Result<Vec<Group>, String> {
    let straights: Vec<usize> = (0..els.len()).filter(|&i| is_straight(&els[i])).collect();
    if straights.len() < 2 {
        return Err("Track braucht mindestens zwei Geraden mit einem Bogen dazwischen.".into());
    }
    let (first, last) = (straights[0], *straights.last().unwrap());
    let mut groups = Vec::new();
    let mut gaps: Vec<Skipped> = Vec::new();
    let mut entry_idx = first;
    let mut i = first + 1;
    while i <= last {
        if is_straight(&els[i]) {
            entry_idx = i;
            i += 1;
            continue;
        }
        match read_group(els, entry_idx, i, last) {
            Ok((group, exit_idx)) => {
                groups.push(group);
                entry_idx = exit_idx;
                i = exit_idx + 1;
            }
            Err(Unreadable(why)) => {
                // On to the next straight; that one starts the next group.
                let next = next_straight(els, i, last);
                gaps.push((i, next - 1, why.to_string()));
                entry_idx = next;
                i = next + 1;
            }
        }
    }
    skipped.extend(gaps.iter().cloned());
    if groups.is_empty() {
        return Err(format!(
            "Keine optimierbaren Bögen gefunden.{}",
            gaps.first().map_or(String::new(), |g| format!(" {}", g.2))
        ));
    }
    Ok(groups)
}

fn build_group(els: &[Json], entry_idx: usize, arc_idxs: Vec<usize>, t_idxs: Vec<Option<usize>>, exit_idx: usize) -> Group {
    let (entry, exit) = (&els[entry_idx], &els[exit_idx]);
    let (b1, b2) = (bearing_from_nodes(entry), bearing_from_nodes(exit));
    let signs: Vec<f64> = arc_idxs
        .iter()
        .map(|&k| if num(&els[k], "radius").unwrap_or(0.0) > 0.0 { 1.0 } else { -1.0 })
        .collect();
    let mut slots = vec![entry_idx];
    slots.extend(&arc_idxs);
    let mut ends = arc_idxs.clone();
    ends.push(exit_idx);
    let (es, ee) = (node(entry, "startNode"), node(entry, "endNode"));
    Group {
        p1: es,
        d1: dir_of(b1),
        b1,
        p2: node(exit, "endNode"),
        d2: dir_of(b2),
        b2,
        s_curve: signs.iter().any(|s| *s > 0.0) && signs.iter().any(|s| *s < 0.0),
        signs,
        s_entry: (ee.0 - es.0).hypot(ee.1 - es.1),
        arcs: arc_idxs
            .iter()
            .map(|&k| ArcAlt {
                r_alt: num(&els[k], "radius").unwrap_or(0.0).abs(),
                u_alt: num(&els[k], "cant").unwrap_or(0.0).abs(),
                sweep_alt: arc_sweep_mag(&els[k]),
            })
            .collect(),
        types: t_idxs.iter().map(|t| t.map_or(Profile::Clothoid, |k| profile_of(&els[k]))).collect(),
        has_t: t_idxs.iter().map(Option::is_some).collect(),
        t_len: t_idxs.iter().map(|t| t.and_then(|k| num(&els[k], "length")).unwrap_or(0.0)).collect(),
        on_switch: els[entry_idx + 1..exit_idx].iter().any(is_switch_element),
        jump_internal: slots.iter().zip(&ends).map(|(&a, &b)| same_switch(&els[a], &els[b])).collect(),
        entry_speed: num(entry, "speed").unwrap_or(0.0),
        exit_speed: num(exit, "speed").unwrap_or(0.0),
        ref_poly: ref_polyline(els, entry_idx, exit_idx),
        ref_curve_pts: ref_polyline(els, entry_idx + 1, exit_idx - 1),
        entry_idx,
        arc_idxs,
        t_idxs,
        exit_idx,
    }
}

// ── building the optimized element chain ─────────────────────────────────────

fn straight_element(frm: Pt, to: Pt, epsg: &Json, speed: &Json, cant: Option<&Json>) -> Json {
    let mut el = json!({
        "elementType": 0, "epsg": epsg,
        "startNode": [frm.0, frm.1], "endNode": [to.0, to.1],
        "bearing": (to.0 - frm.0).atan2(to.1 - frm.1).to_degrees().rem_euclid(360.0),
        "length": (to.0 - frm.0).hypot(to.1 - frm.1), "speed": speed,
    });
    if let Some(c) = cant.filter(|c| !c.is_null()) {
        el["cant"] = c.clone();
    }
    el
}

/// A segment of a fitted group as the app's element.
pub fn segment_element(seg: &crate::geometry::Segment, epsg: &Json, speed: &Json, cant: f64) -> Json {
    use crate::geometry::Segment;
    match *seg {
        Segment::Transition {
            start,
            bearing,
            length,
            r1,
            r2,
            profile,
        } => {
            let (e, n, end_bearing) = transition_end(start.0, start.1, bearing, length, r1, r2, profile);
            json!({
                "elementType": 2, "transitionType": profile.id(), "r1": r1, "r2": r2, "epsg": epsg,
                "startNode": [start.0, start.1], "endNode": [e, n],
                "bearing": bearing, "endBearing": end_bearing, "length": length, "speed": speed,
            })
        }
        Segment::Arc {
            start,
            end,
            signed_r,
            sweep,
            bearing,
            end_bearing,
        } => json!({
            "elementType": 1, "epsg": epsg, "radius": signed_r,
            "startNode": [start.0, start.1], "endNode": [end.0, end.1],
            "bearing": bearing, "endBearing": end_bearing,
            "length": (sweep * signed_r).abs(), "speed": speed,
            "cant": if signed_r < 0.0 { -cant.abs() } else { cant.abs() },
        }),
    }
}

/// What a group's solution hands the chain: its fit, the design speed it is
/// stated at, the cant of each arc.
pub struct Built<'a> {
    pub fit: &'a crate::geometry::GroupFit,
    pub speed: Json,
    pub us: &'a [f64],
}

/// The optimized element chain: per group its solution or None (kept
/// unchanged), `shifts` straight index → lateral shift of its line.
pub fn build_elements(els: &[Json], epsg: &Json, groups: &[Group], solutions: &[Option<Built>], shifts: &std::collections::HashMap<usize, f64>) -> Vec<Json> {
    let mut cuts: std::collections::HashMap<usize, (Option<Pt>, Option<Pt>)> = std::collections::HashMap::new();
    let mut group_first: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut members_of: Vec<Vec<usize>> = Vec::new();
    let mut in_group = std::collections::HashSet::new();
    for (gi, (g, sol)) in groups.iter().zip(solutions).enumerate() {
        let mut members: Vec<usize> = g.arc_idxs.iter().copied().chain(g.t_idxs.iter().flatten().copied()).collect();
        members.sort_unstable();
        group_first.insert(members[0], gi);
        in_group.extend(members.iter().copied());
        members_of.push(members);
        if let Some(sol) = sol {
            cuts.entry(g.entry_idx).or_default().1 = Some(sol.fit.cl_start);
            cuts.entry(g.exit_idx).or_default().0 = Some(sol.fit.cl_end);
        }
    }
    let mut out = Vec::new();
    for (i, el) in els.iter().enumerate() {
        if is_straight(el) {
            let (cut_from, cut_to) = cuts.get(&i).copied().unwrap_or_default();
            let s = shifts.get(&i).copied().unwrap_or(0.0);
            let d = dir_of(bearing_from_nodes(el));
            let normal = (-d.1, d.0);
            let (sn, en) = (node(el, "startNode"), node(el, "endNode"));
            let frm = cut_from.unwrap_or((sn.0 + s * normal.0, sn.1 + s * normal.1));
            let to = cut_to.unwrap_or((en.0 + s * normal.0, en.1 + s * normal.1));
            out.push(straight_element(frm, to, epsg, el.get("speed").unwrap_or(&json!(0)), el.get("cant")));
            continue;
        }
        let Some(&gi) = group_first.get(&i) else {
            if !in_group.contains(&i) {
                out.push(el.clone()); // before the first straight or after the last
            }
            continue; // emitted at its group's first index
        };
        match &solutions[gi] {
            None => out.extend(members_of[gi].iter().map(|&k| els[k].clone())),
            Some(sol) => {
                let mut arc_no = 0;
                for seg in &sol.fit.segments {
                    let cant = match seg {
                        crate::geometry::Segment::Arc { .. } => {
                            arc_no += 1;
                            sol.us[arc_no - 1]
                        }
                        _ => 0.0,
                    };
                    out.push(segment_element(seg, epsg, &sol.speed, cant));
                }
            }
        }
    }
    out
}

/// A track's elements and epsg from a request: the elements as they came.
pub fn elements_of(track: &Json) -> Option<(&[Json], Json)> {
    let els = track.get("elements")?.as_array()?;
    Some((els.as_slice(), track.get("epsg").cloned().unwrap_or(Json::Null)))
}
