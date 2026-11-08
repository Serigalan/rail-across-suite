// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A turnout's two routes carry one cant side by side (`switchCants.js`).

use std::collections::{HashMap, HashSet};

use rail_across_geom::cant::{cant_ends, with_cant_at_end, with_cants_over};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_js::{get, js_string, truthy};
use rail_across_track::switch_model::{port_end, port_track};
use serde_json::Value;

use crate::route::switch_abreast;

const SAME_CANT: f64 = 0.001;
const MAX_PASSES: usize = 24;

fn is_turnout(sw: &Value) -> bool {
    get(sw, "kind").and_then(Value::as_str).unwrap_or("turnout") == "turnout"
}

fn node(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v?.as_array()?;
    Some([
        a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ])
}

fn near(a: Option<[f64; 2]>, b: Option<[f64; 2]>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if (a[0] - b[0]).hypot(a[1] - b[1]) <= NODE_TOL)
}

#[derive(Clone, Debug)]
struct Run {
    track_id: String,
    from: usize,
    to: usize,
}

#[derive(Default, Clone, Debug)]
struct Runs {
    main: Option<Run>,
    branch: Option<Run>,
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn route_runs(tracks: &[Value]) -> HashMap<String, Runs> {
    let mut out: HashMap<String, Runs> = HashMap::new();
    for tr in tracks {
        let tid = js_string(tr.get("id"));
        for (i, el) in els(tr).iter().enumerate() {
            let Some(sid) = el.get("switchId").filter(|v| truthy(Some(v))) else {
                continue;
            };
            let route = el.get("switchRoute").and_then(Value::as_str);
            if route != Some("main") && route != Some("branch") {
                continue;
            }
            let runs = out.entry(js_string(Some(sid))).or_default();
            let slot = if route == Some("main") { &mut runs.main } else { &mut runs.branch };
            match slot {
                None => {
                    *slot = Some(Run {
                        track_id: tid.clone(),
                        from: i,
                        to: i,
                    })
                }
                Some(r) if r.track_id == tid && r.to + 1 == i => r.to = i,
                _ => {}
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
struct Piece {
    index: usize,
    s0: f64,
    length: f64,
    start: f64,
    end: f64,
}

struct Oriented {
    track: Value,
    run: Run,
    reversed: bool,
    sign: f64,
    pieces: Vec<Piece>,
    length: f64,
}

fn oriented(track: &Value, run: &Run, toe: [f64; 2]) -> Oriented {
    let es = els(track);
    let reversed = !near(node(es[run.from].get("startNode")), Some(toe));
    let mut idx: Vec<usize> = (run.from..=run.to).collect();
    if reversed {
        idx.reverse();
    }
    let sign = if reversed { -1.0 } else { 1.0 };
    let mut s = 0.0;
    let pieces = idx
        .into_iter()
        .map(|i| {
            let (a0, b0) = cant_ends(es, i);
            let (a, b) = if reversed { (b0, a0) } else { (a0, b0) };
            let length = get(&es[i], "length").and_then(Value::as_f64).unwrap_or(0.0);
            let p = Piece {
                index: i,
                s0: s,
                length,
                start: sign * a,
                end: sign * b,
            };
            s += length;
            p
        })
        .collect();
    Oriented {
        track: track.clone(),
        run: run.clone(),
        reversed,
        sign,
        pieces,
        length: s,
    }
}

fn cant_at(r: &Oriented, u: f64) -> f64 {
    let p = r.pieces.iter().find(|q| u <= q.s0 + q.length + 1e-9).unwrap_or(&r.pieces[r.pieces.len() - 1]);
    let t = if p.length > 0.0 { 1f64.min(0f64.max((u - p.s0) / p.length)) } else { 0.0 };
    p.start + (p.end - p.start) * t
}

fn joints_of(r: &Oriented) -> Vec<f64> {
    r.pieces.iter().skip(1).map(|p| p.s0).collect()
}

fn toe_of(by_id: &HashMap<String, Value>, runs: &Runs) -> Option<[f64; 2]> {
    let (rm, rb) = (runs.main.as_ref()?, runs.branch.as_ref()?);
    let (m, b) = (by_id.get(&rm.track_id)?, by_id.get(&rb.track_id)?);
    let ends = |tr: &Value, run: &Run| [node(els(tr)[run.from].get("startNode")), node(els(tr)[run.to].get("endNode"))];
    for p in ends(m, rm) {
        for q in ends(b, rb) {
            if near(p, q) {
                return p;
            }
        }
    }
    None
}

fn cant_changed(before: Option<&Value>, after: &Value, run: &Run) -> bool {
    let Some(before) = before else { return false };
    if before == after {
        return false;
    }
    if els(before).len() != els(after).len() {
        return true;
    }
    (run.from..=run.to).any(|i| {
        let (a, b) = (cant_ends(els(before), i), cant_ends(els(after), i));
        (a.0 - b.0).abs() > SAME_CANT || (a.1 - b.1).abs() > SAME_CANT
    })
}

fn laid_onto(from: &Oriented, to: &Oriented, host: &Oriented) -> Option<Value> {
    let joints = joints_of(host);
    let at = |u: f64| to.sign * cant_at(from, switch_abreast(u, &joints, to.length, from.length));
    let by_index: HashMap<usize, (f64, f64)> = to.pieces.iter().map(|p| (p.index, (at(p.s0), at(p.s0 + p.length)))).collect();
    let mut differs = false;
    let mut ends = Vec::new();
    for i in to.run.from..=to.run.to {
        let (a, b) = by_index[&i];
        let (start, end) = if to.reversed { (b, a) } else { (a, b) };
        let e = cant_ends(els(&to.track), i);
        if (e.0 - start).abs() > SAME_CANT || (e.1 - end).abs() > SAME_CANT {
            differs = true;
        }
        ends.push((start, end));
    }
    if !differs {
        return None;
    }
    let mut t = to.track.clone();
    t["elements"] = Value::Array(with_cants_over(els(&to.track), to.run.from, to.run.to, &ends));
    Some(t)
}

fn couple_pass(before: &[Value], tracks: &[Value], switches: &[Value], only: &HashSet<String>) -> (Vec<Value>, HashSet<String>) {
    let was: HashMap<String, &Value> = before.iter().map(|t| (js_string(t.get("id")), t)).collect();
    let mut by_id: HashMap<String, Value> = tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
    let order: Vec<String> = tracks.iter().map(|t| js_string(t.get("id"))).collect();
    let mut changed: HashSet<String> = HashSet::new();
    for sw in switches {
        let sid = js_string(sw.get("switchId"));
        if !only.contains(&sid) || !is_turnout(sw) {
            continue;
        }
        let current: Vec<Value> = order.iter().filter_map(|id| by_id.get(id).cloned()).collect();
        let Some(runs) = route_runs(&current).remove(&sid) else { continue };
        let (Some(rm), Some(rb)) = (runs.main.clone(), runs.branch.clone()) else {
            continue;
        };
        let Some(toe) = toe_of(&by_id, &runs) else { continue };
        let main = oriented(&by_id[&rm.track_id], &rm, toe);
        let branch = oriented(&by_id[&rb.track_id], &rb, toe);
        if rm.track_id == rb.track_id {
            continue;
        }
        let approach_id = port_track(sw, "A").map(|v| js_string(Some(v)));
        let approach = approach_id.as_ref().and_then(|id| by_id.get(id)).cloned();
        let end = if port_end(sw, "A") == Some("BEGIN") { "BEGIN" } else { "END" };
        let frame = if end == "END" { 1.0 } else { -1.0 };
        let toe_cant = |track: &Value| -> Option<f64> {
            let es = els(track);
            if es.is_empty() {
                return None;
            }
            let (a, b) = cant_ends(es, if end == "END" { es.len() - 1 } else { 0 });
            Some(frame * if end == "END" { b } else { a })
        };
        let own_approach = approach.as_ref().is_some_and(|a| {
            let id = js_string(a.get("id"));
            id != rm.track_id && id != rb.track_id
        });
        let main_led = cant_changed(was.get(&rm.track_id).copied(), &main.track, &rm);
        let branch_led = cant_changed(was.get(&rb.track_id).copied(), &branch.track, &rb);
        let mut set = |by_id: &mut HashMap<String, Value>, track: Value| {
            let id = js_string(track.get("id"));
            by_id.insert(id.clone(), track);
            changed.insert(id);
        };
        if !main_led && !branch_led {
            let Some(app) = approach.as_ref() else { continue };
            let before_app = was.get(&js_string(app.get("id"))).copied();
            let v = if own_approach { toe_cant(app) } else { None };
            let Some(v) = v else { continue };
            let Some(b) = before_app else { continue };
            if b == app || ((toe_cant(b).unwrap_or(v)) - v).abs() <= SAME_CANT || (cant_at(&main, 0.0) - v).abs() <= SAME_CANT {
                continue;
            }
            let mut t = main.track.clone();
            t["elements"] = Value::Array(with_cant_at_end(els(&main.track), if main.reversed { "END" } else { "BEGIN" }, main.sign * v));
            set(&mut by_id, t);
            continue;
        }
        let (lead, follow) = if branch_led && !main_led { (&branch, &main) } else { (&main, &branch) };
        let host = if truthy(sw.get("swapped")) { &branch } else { &main };
        if let Some(next) = laid_onto(lead, follow, host) {
            set(&mut by_id, next);
        }
        if own_approach {
            let app = approach.expect("approach");
            let app_now = by_id.get(&js_string(app.get("id"))).cloned().unwrap_or(app);
            let v = cant_at(lead, 0.0);
            if (toe_cant(&app_now).unwrap_or(f64::NAN) - v).abs() > SAME_CANT {
                let mut t = app_now.clone();
                t["elements"] = Value::Array(with_cant_at_end(els(&app_now), end, frame * v));
                set(&mut by_id, t);
            }
        }
    }
    let out = if changed.is_empty() {
        tracks.to_vec()
    } else {
        tracks
            .iter()
            .map(|t| by_id.get(&js_string(t.get("id"))).cloned().unwrap_or_else(|| t.clone()))
            .collect()
    };
    (out, changed)
}

/// The tracks with every turnout's two routes' cant coupled (`coupleSwitchCants`).
pub fn couple_switch_cants(before: &[Value], after: &[Value], switches: &[Value]) -> Vec<Value> {
    if after.is_empty() || switches.is_empty() {
        return after.to_vec();
    }
    let touched = |tracks: &[Value], prev: &HashMap<String, &Value>| -> HashSet<String> {
        let ids: HashSet<String> = tracks
            .iter()
            .filter(|t| prev.get(&js_string(t.get("id"))).is_none_or(|p| *p != *t))
            .map(|t| js_string(t.get("id")))
            .collect();
        let runs = route_runs(tracks);
        switches
            .iter()
            .filter(|sw| {
                let r = runs.get(&js_string(sw.get("switchId")));
                let has = |x: Option<&Run>| x.is_some_and(|x| ids.contains(&x.track_id));
                r.is_some_and(|r| has(r.main.as_ref()) || has(r.branch.as_ref())) || port_track(sw, "A").is_some_and(|a| ids.contains(&js_string(Some(a))))
            })
            .map(|sw| js_string(sw.get("switchId")))
            .collect()
    };
    let was: HashMap<String, &Value> = before.iter().map(|t| (js_string(t.get("id")), t)).collect();
    let mut only = touched(after, &was);
    let mut prev_tracks: Vec<Value> = before.to_vec();
    let mut tracks: Vec<Value> = after.to_vec();
    let mut n = 0;
    while n < MAX_PASSES && !only.is_empty() {
        let (next, changed) = couple_pass(&prev_tracks, &tracks, switches, &only);
        if changed.is_empty() {
            break;
        }
        prev_tracks = tracks;
        let prev: HashMap<String, &Value> = prev_tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
        only = touched(&next, &prev);
        tracks = next;
        n += 1;
    }
    tracks
}
