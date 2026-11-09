// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The gradient of a turnout is one: the branch lies in the plane of the
//! main route from WA to the ldS (`switchGradient.js`).

use std::collections::HashSet;

use rail_across_js::{get, js_round, js_string, number, strict_eq};
use rail_across_track::cross_section::{RUNNING_CIRCLE_DISTANCE, section_at_station};
use rail_across_track::heights::gradient_at;
use rail_across_track::platforms::point_at_station;
use rail_across_track::switch_model::{port_track, turnout_line_port};
use serde_json::{Value, json};

use crate::sleepers::{Frame, Sleeper, SleeperOptions, main_route_length, route_from, sleeper_offsets, sleeper_through, switch_lds, turnout_sleepers};

pub const STATION_TOL: f64 = 0.01;
pub const Z_TOL: f64 = 0.00005;

pub fn round_mm(x: f64) -> f64 {
    js_round(x * 1000.0) / 1000.0
}

fn heights(track: &Value) -> &[Value] {
    get(track, "heights").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// How far from WA the body reaches (`ldsFromToe`).
pub fn lds_from_toe(tracks: &[Value], sw: &Value, o: &SleeperOptions) -> Option<f64> {
    let main = route_from(tracks, sw, turnout_line_port(sw));
    let we = main.map(|m| main_route_length(&m.track, sw)).unwrap_or(0.0);
    if !(we > 0.0) {
        return None;
    }
    Some(we + switch_lds(sw, o.form_of).unwrap_or(0.0))
}

fn slope_at(heights: &[Value], station: f64) -> f64 {
    let h = 0.05;
    match (gradient_at(heights, station - h), gradient_at(heights, station + h)) {
        (Some(a), Some(b)) => (b - a) / (2.0 * h),
        _ => 0.0,
    }
}

fn cant_at(track: &Value, station: f64) -> f64 {
    section_at_station(track, station).get("cant").and_then(Value::as_f64).unwrap_or(0.0)
}

/// How a turnout couples its branch to its main route (`switchCoupling`).
pub struct Coupling {
    pub sw: Value,
    pub frame: Frame,
    pub lds_sleeper: Sleeper,
    pub offset: f64,
    pub cant: f64,
    pub reach: (f64, f64),
    pub end: (f64, f64),
}

impl Coupling {
    pub fn to_json(&self) -> Value {
        let mut o = self.frame.to_json();
        o["sw"] = self.sw.clone();
        // The JavaScript keeps the main route's cant on the ldS sleeper once read (cantOn).
        let mut lds = self.lds_sleeper.to_json();
        lds["cant"] = number(self.cant);
        if let Some(last) = o["sleepers"].as_array_mut().and_then(|s| s.last_mut()) {
            last["cant"] = number(self.cant);
        }
        o["ldsSleeper"] = lds;
        o["offset"] = number(self.offset);
        o["cant"] = number(self.cant);
        o["reach"] = json!({ "main": number(self.reach.0), "branch": number(self.reach.1) });
        o["end"] = json!({ "main": number(self.end.0), "branch": number(self.end.1) });
        o
    }
    fn side(&self, main: bool) -> &crate::sleepers::RouteFrom {
        if main { &self.frame.main } else { &self.frame.branch }
    }
}

pub fn switch_coupling(tracks: &[Value], sw: &Value, o: &SleeperOptions) -> Option<Coupling> {
    let opts = SleeperOptions {
        form_of: o.form_of,
        rail: o.rail.clone(),
        lds_unknown: Some(0.0),
    };
    let frame = turnout_sleepers(tracks, sw, &opts)?;
    let lds = *frame.sleepers.last()?;
    let cant = cant_at(&frame.main.track, lds.main);
    let reach = (frame.lds, frame.lds_branch_distance);
    Some(Coupling {
        sw: sw.clone(),
        offset: lds.y,
        cant,
        lds_sleeper: lds,
        reach,
        end: reach,
        frame,
    })
}

fn couplings_of(tracks: &[Value], switches: &[Value], o: &SleeperOptions) -> Vec<Coupling> {
    let mut cs: Vec<Coupling> = switches.iter().filter_map(|sw| switch_coupling(tracks, sw, o)).collect();
    // On each track, in the order first met: (coupling, main side?).
    let mut on_track: Vec<(String, Vec<(usize, bool)>)> = Vec::new();
    for (i, c) in cs.iter().enumerate() {
        for main in [true, false] {
            let id = js_string(c.side(main).track.get("id"));
            match on_track.iter_mut().find(|e| e.0 == id) {
                Some(e) => e.1.push((i, main)),
                None => on_track.push((id, vec![(i, main)])),
            }
        }
    }
    for (_, entries) in &on_track {
        for (i, &(c1, s1)) in entries.iter().enumerate() {
            for &(c2, s2) in &entries[i + 1..] {
                if cs[c1].side(s1).sense == cs[c2].side(s2).sense {
                    continue;
                }
                let l = cs[c1].side(s1).length;
                let r = |c: &Coupling, main: bool| if main { c.reach.0 } else { c.reach.1 };
                let (r1, r2) = (r(&cs[c1], s1), r(&cs[c2], s2));
                if r1 + r2 <= l {
                    continue;
                }
                let split = (r1 + l - r2) / 2.0;
                let set = |c: &mut Coupling, main: bool, v: f64| if main { c.end.0 = v } else { c.end.1 = v };
                set(&mut cs[c1], s1, split);
                set(&mut cs[c2], s2, l - split);
            }
        }
    }
    cs
}

/// Every coupling a project's turnouts make (`switchCouplings`).
pub fn switch_couplings(tracks: &[Value], switches: &[Value], o: &SleeperOptions) -> Vec<Coupling> {
    couplings_of(tracks, switches, o)
}

/// The coupling of one turnout, with the stretch it owns (`couplingOf`).
pub fn coupling_of(tracks: &[Value], switches: &[Value], sw: &Value, o: &SleeperOptions) -> Option<Coupling> {
    let own = switch_coupling(tracks, sw, o)?;
    let ids: HashSet<String> = [js_string(own.frame.main.track.get("id")), js_string(own.frame.branch.track.get("id"))]
        .into_iter()
        .collect();
    let near: Vec<Value> = switches
        .iter()
        .filter(|s| {
            strict_eq(s.get("switchId"), sw.get("switchId"))
                || ["B1", "B2"]
                    .iter()
                    .any(|p| port_track(s, p).is_some_and(|id| ids.contains(&js_string(Some(id)))))
        })
        .cloned()
        .collect();
    couplings_of(tracks, &near, o)
        .into_iter()
        .find(|c| strict_eq(c.sw.get("switchId"), sw.get("switchId")))
}

fn in_body(c: &Coupling, main: bool, station: f64) -> bool {
    let d = c.side(main).distance(station);
    let end = if main { c.end.0 } else { c.end.1 };
    d > STATION_TOL && d <= end + STATION_TOL
}

/// How far the branch lies above the main route on the ldS sleeper (`ldsLift`).
pub fn lds_lift(c: &Coupling, main_heights: &[Value]) -> f64 {
    let sl = &c.lds_sleeper;
    slope_at(main_heights, sl.main) * sl.a + (c.cant / RUNNING_CIRCLE_DISTANCE) * sl.y
}

/// `lds_lift` of a coupling as `Coupling::to_json` writes it — its ldS
/// sleeper's `main`, `a` and `y`, and its `cant` — on the main route's heights.
pub fn lds_lift_json(c: &Value, main_heights: &[Value]) -> Option<f64> {
    let sl = &c["ldsSleeper"];
    let (main, a, y, cant) = (sl["main"].as_f64()?, sl["a"].as_f64()?, sl["y"].as_f64()?, c["cant"].as_f64().unwrap_or(0.0));
    Some(slope_at(main_heights, main) * a + (cant / RUNNING_CIRCLE_DISTANCE) * y)
}

/// The height a point takes from one it depends on "as an ldS" (`planeFrom`).
pub fn plane_from(src: &Value, src_station: f64, z_src: Option<f64>, dst: &Value, dst_station: f64) -> Option<f64> {
    let (p, pb) = point_at_station(src, src_station)?;
    let (q, _) = point_at_station(dst, dst_station)?;
    let z_src = z_src?;
    let b = pb * std::f64::consts::PI / 180.0;
    let y = b.sin() * (q.n - p.n) - b.cos() * (q.e - p.e);
    Some(z_src + (cant_at(src, src_station) / RUNNING_CIRCLE_DISTANCE) * y)
}

/// Height of a coupled branch at one of its stations (`branchPlaneHeight`).
pub fn branch_plane_height(c: &Coupling, station: f64) -> Option<f64> {
    let (p, main_st, branch_st) = sleeper_through(&c.frame, "branch", station)?;
    let main = &c.frame.main.track;
    let z_p = gradient_at(heights(main), main_st)?;
    let _ = p;
    let (a, y) = sleeper_offsets(&c.frame.main, &c.frame.branch, main_st, branch_st)?;
    Some(z_p + slope_at(heights(main), main_st) * a + (cant_at(main, main_st) / RUNNING_CIRCLE_DISTANCE) * y)
}

/// The height a track is built at (`trackHeightAt`).
pub fn track_height_at(tracks: &[Value], switches: &[Value], track: &Value, station: f64, o: &SleeperOptions) -> Option<f64> {
    for c in switch_couplings(tracks, switches, o) {
        if !strict_eq(c.frame.branch.track.get("id"), track.get("id")) || !in_body(&c, false, station) {
            continue;
        }
        if let Some(z) = branch_plane_height(&c, station) {
            return Some(z);
        }
    }
    gradient_at(heights(track), station)
}

/// What trackHeightAt needs of a project for one track (`heightContext`).
pub fn height_context(tracks: &[Value], switches: &[Value], track: &Value, o: &SleeperOptions) -> Value {
    let tid = js_string(track.get("id"));
    let couplings: Vec<Coupling> = switch_couplings(tracks, switches, o)
        .into_iter()
        .filter(|c| js_string(c.frame.branch.track.get("id")) == tid || js_string(c.frame.main.track.get("id")) == tid)
        .collect();
    let mut ids: HashSet<String> = couplings
        .iter()
        .flat_map(|c| [js_string(c.frame.main.track.get("id")), js_string(c.frame.branch.track.get("id"))])
        .collect();
    ids.remove(&tid);
    let coupled_here = couplings.iter().any(|c| js_string(c.frame.branch.track.get("id")) == tid);
    if !coupled_here {
        return json!({ "tracks": [track], "switches": [] });
    }
    let mut out = vec![track.clone()];
    out.extend(tracks.iter().filter(|t| ids.contains(&js_string(t.get("id")))).cloned());
    json!({ "tracks": out, "switches": couplings.iter().map(|c| c.sw.clone()).collect::<Vec<_>>() })
}

fn ordered(a: f64, b: f64) -> Value {
    json!({ "from": number(a.min(b)), "to": number(a.max(b)) })
}

/// The stretches of a track the Höhenplan treats as the turnout (`switchBodySpans`).
pub fn switch_body_spans(tracks: &[Value], switches: &[Value], track: &Value, o: &SleeperOptions) -> Vec<Value> {
    let mut spans = Vec::new();
    let mut covered: HashSet<String> = HashSet::new();
    let tid = track.get("id");
    for sw in switches {
        let Some(c) = switch_coupling(tracks, sw, o) else { continue };
        if strict_eq(c.frame.main.track.get("id"), tid) {
            spans.push(ordered(c.frame.main.station(0.0), c.frame.lds_main));
            covered.insert(js_string(sw.get("switchId")));
        } else if strict_eq(c.frame.branch.track.get("id"), tid) {
            spans.push(ordered(c.frame.branch.station(0.0), c.frame.lds_branch));
            covered.insert(js_string(sw.get("switchId")));
        }
    }
    let mut start = 0.0;
    let mut open: Option<(String, f64, f64)> = None;
    for el in track.get("elements").and_then(Value::as_array).into_iter().flatten() {
        let end = start + get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        let id = get(el, "switchId").map(|v| js_string(Some(v))).filter(|id| !covered.contains(id));
        match (&mut open, id) {
            (Some(o), Some(id)) if o.0 == id => o.2 = end,
            (_, id) => {
                if let Some(o) = open.take() {
                    spans.push(json!({ "from": number(o.1), "to": number(o.2) }));
                }
                open = id.map(|id| (id, start, end));
            }
        }
        start = end;
    }
    if let Some(o) = open {
        spans.push(json!({ "from": number(o.1), "to": number(o.2) }));
    }
    spans
}
