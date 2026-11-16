// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The S-curve between two line tracks: solving the connection for a shift,
//! the range of shifts it stands over, and the commit (`sCurve.js`).

use rail_across_geom::cant::cant_ends;
use rail_across_geom::coords::{plane_to_wgs84, transform_grid_bearing, wgs84_to_plane};
use rail_across_geom::element::{Utm, node_utm};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_js::{Object, generate_id, get, js_number, js_string, number, strict_eq, truthy};
use rail_across_rules::element_length::min_element_length;
use rail_across_switch::connection::{Stem, build_connection_elements, orient_stem_toward, solve_switch_connection};
use rail_across_switch::mid_chain::Mid;
use rail_across_switch::placement::turnout_runs_on;
use rail_across_switch::route::{Route, switch_element_route, switch_route_varies, to_chain};
use rail_across_switch::slider::unique_stations;
use rail_across_switch::symbol::{MainR, compute_switch_geometry_utm};
use rail_across_track::ids::{next_switch_number, switch_designation};
use rail_across_track::model::{next_track_name, rebuild_coords, track_label};
use rail_across_track::split::{carve_switch_route, split_element_at, split_track_at_joint};
use rail_across_track::switch_model::{new_switch_fields, switch_element_mark};
use serde_json::{Value, json};

const STEM_REACH: f64 = 200.0;
const FIT_TOL: f64 = 1e-6;
const CURVATURE_EPS: f64 = 1e-6;

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn curvature(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 && !r.is_nan() => 1.0 / r,
        _ => 0.0,
    }
}

fn is_transition(el: &Value) -> bool {
    el.get("elementType").and_then(Value::as_f64) == Some(2.0)
}

fn same_geometry(a: Option<&Value>, b: &Value) -> bool {
    a.is_some_and(|a| !is_transition(a) && !is_transition(b) && (curvature(f(a, "radius")) - curvature(f(b, "radius"))).abs() < CURVATURE_EPS)
}

/// The run of one curvature beside element `i` (`runBeside`): its length, or None.
pub fn run_beside(elements: &[Value], i: usize, side: &str) -> Option<f64> {
    let el = elements.get(i)?;
    if is_transition(el) {
        return None;
    }
    let step: i64 = if side == "before" { -1 } else { 1 };
    let mut length = 0.0;
    let mut j = i as i64 + step;
    while j >= 0 && (j as usize) < elements.len() && same_geometry(elements.get(j as usize), el) {
        length += f(&elements[j as usize], "length").unwrap_or(0.0);
        j += step;
    }
    (length > 0.0).then_some(length)
}

fn first_last_coord(el: &Value) -> (Option<&Value>, Option<&Value>) {
    let c = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(Value::as_array);
    (c.and_then(|c| c.first()), c.and_then(|c| c.last()))
}

/// A pick as the dialog takes one (`connectionPick`).
pub fn connection_pick(track: &Value, el_idx: usize, along: f64) -> Value {
    let es = els(track);
    let epsg = js_number(track.get("epsg")) as i64;
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    };
    let piece = |i: usize, s0: f64| -> Value {
        let el = &es[i];
        let (c0, c1) = first_last_coord(el);
        let (rs, re) = cant_ends(es, i);
        let speed = f(el, "speed").filter(|s| *s != 0.0 && !s.is_nan()).map(number).unwrap_or(Value::Null);
        json!({
            "elIdx": i, "s0": number(s0),
            "startUtm": node_utm(el.get("startNode"), c0, epsg).unwrap_or(nan).to_json(),
            "endUtm": node_utm(el.get("endNode"), c1, epsg).unwrap_or(nan).to_json(),
            "bearing": el.get("bearing"), "route": switch_element_route(el).to_json(),
            "cantStart": number(rs), "cantEnd": number(re), "speed": speed,
            "trackEnds": [i == 0, i == es.len() - 1],
            "runs": [number(run_beside(es, i, "before").unwrap_or(0.0)), number(run_beside(es, i, "after").unwrap_or(0.0))],
        })
    };
    let own = piece(el_idx, 0.0);
    let own_len = own["route"]["length"].as_f64().unwrap_or(f64::NAN);
    let mut pieces = vec![own.clone()];
    let (mut i, mut s0) = (el_idx, 0.0);
    while s0 > -STEM_REACH && turnout_runs_on(es, i, "before", epsg) {
        s0 -= js_number(es[i - 1].get("length"));
        pieces.insert(0, piece(i - 1, s0));
        i -= 1;
    }
    let (mut i, mut s0) = (el_idx, own_len);
    while s0 < own_len + STEM_REACH && turnout_runs_on(es, i, "after", epsg) {
        pieces.push(piece(i + 1, s0));
        s0 += js_number(es[i + 1].get("length"));
        i += 1;
    }
    let mut out = json!({ "trackId": track.get("id"), "elIdx": el_idx, "along": number(along) });
    for (k, v) in own.as_object().into_iter().flatten() {
        if k != "elIdx" && k != "s0" {
            out[k] = v.clone();
        }
    }
    out["zone"] = track.get("epsg").cloned().unwrap_or(Value::Null);
    let name = get(track, "name")
        .filter(|v| truthy(Some(v)))
        .cloned()
        .unwrap_or_else(|| json!(track_label(Some(track))));
    out["name"] = name;
    out["pieces"] = Value::Array(pieces);
    out
}

/// A turnout may be laid on this element (`isUsableStem`).
pub fn is_usable_stem(el: &Value) -> bool {
    !is_transition(el) || (el.get("transitionType").and_then(Value::as_str) != Some("bloss") && rail_across_js::has(el, "r1"))
}

fn to_plane(p: Utm, crs: i64) -> Utm {
    if p.zone == crs {
        return p;
    }
    let (lng, lat) = plane_to_wgs84(p.e, p.n, p.zone).unwrap_or((f64::NAN, f64::NAN));
    let (e, n) = wgs84_to_plane(lng, lat, crs).unwrap_or((f64::NAN, f64::NAN));
    Utm { e, n, zone: crs }
}

fn stem_in_plane(g: &Value, crs: i64) -> Value {
    let zone = js_number(g.get("zone")) as i64;
    let keep = ["startUtm", "bearing", "route", "along", "cantStart", "cantEnd"];
    let mut out = json!({});
    for k in keep {
        out[k] = g.get(k).cloned().unwrap_or(Value::Null);
    }
    let pieces: Vec<Value> = g
        .get("pieces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|p| json!({ "startUtm": p.get("startUtm"), "bearing": p.get("bearing"), "route": p.get("route"), "cantStart": p.get("cantStart"), "cantEnd": p.get("cantEnd"), "s0": p.get("s0") }))
        .collect();
    if zone == crs {
        out["pieces"] = Value::Array(pieces);
        return out;
    }
    let in_plane = |p: &Value, b: f64| -> (Value, Value) {
        let u = Utm::from_json(p);
        (
            to_plane(u, crs).to_json(),
            number(transform_grid_bearing(u.e, u.n, b, zone, crs).unwrap_or(f64::NAN)),
        )
    };
    let (s, b) = in_plane(&g["startUtm"], js_number(g.get("bearing")));
    out["startUtm"] = s;
    out["bearing"] = b;
    out["pieces"] = Value::Array(
        pieces
            .into_iter()
            .map(|mut p| {
                let (s, b) = in_plane(&p["startUtm"], js_number(p.get("bearing")));
                p["startUtm"] = s;
                p["bearing"] = b;
                p
            })
            .collect(),
    );
    out
}

/// The two picks as the solver takes them (`stems`).
pub fn stems(picks: &[Value]) -> (Stem, Stem) {
    let zone = js_number(picks[0].get("zone")) as i64;
    let s2 = Stem::from_json(&stem_in_plane(&picks[1], zone));
    let s1 = orient_stem_toward(&Stem::from_json(&stem_in_plane(&picks[0], zone)), &s2);
    (s1, s2)
}

/// What a connection is asked for.
#[derive(Clone)]
pub struct Ask<'a> {
    pub picks: &'a [Value],
    pub speed: f64,
    pub mids: Option<Vec<Mid>>,
    pub seed: Option<(f64, Vec<f64>)>,
    pub form: Option<&'a Value>,
    pub lenient: bool,
}

fn stands(res: &Value, lenient: bool) -> bool {
    !res.is_null() && (truthy(res.get("valid")) || (lenient && truthy(res.get("overridable"))))
}

fn pick_pieces(pick: &Value) -> Vec<Value> {
    match pick.get("pieces").and_then(Value::as_array).filter(|p| !p.is_empty()) {
        Some(p) => p.clone(),
        None => vec![json!({ "s0": 0, "route": pick.get("route"), "speed": pick.get("speed"), "runs": pick.get("runs"), "trackEnds": pick.get("trackEnds") })],
    }
}

fn route_len(p: &Value) -> f64 {
    p["route"]["length"].as_f64().unwrap_or(f64::NAN)
}

fn pick_extent(pick: &Value) -> (f64, f64) {
    let p = pick_pieces(pick);
    let last = &p[p.len() - 1];
    (js_number(p[0].get("s0")), js_number(last.get("s0")) + route_len(last))
}

fn turnout_on_element(pick: &Value, toe: f64, dir: f64, opening: f64, through: f64) -> bool {
    let far = toe + dir * opening * through;
    let (from, to) = pick_extent(pick);
    toe >= -FIT_TOL && toe <= route_len(pick) + FIT_TOL && far >= from - FIT_TOL && far <= to + FIT_TOL
}

fn through_of(res: &Value, k: usize) -> f64 {
    if k == 0 {
        js_number(res.get("throughLength"))
    } else {
        f(res, "throughLength2").unwrap_or_else(|| js_number(res.get("throughLength")))
    }
}

fn turnouts_on_elements(picks: &[Value], dir1: f64, shift: f64, res: &Value) -> bool {
    let Some(s2) = f(res, "s2") else { return false };
    let dir2 = if truthy(res.get("flipped2")) { -1.0 } else { 1.0 };
    let along = |p: &Value| js_number(p.get("along"));
    turnout_on_element(&picks[0], along(&picks[0]) + dir1 * shift, dir1, 1.0, through_of(res, 0))
        && turnout_on_element(&picks[1], along(&picks[1]) + dir2 * s2, dir2, -1.0, through_of(res, 1))
}

/// The connection at a shift (`solveConnection`).
pub fn solve_connection(a: &Ask, shift: f64) -> Value {
    if a.picks.len() < 2 || a.picks[0].is_null() || a.picks[1].is_null() || !(a.speed != 0.0 && !a.speed.is_nan()) {
        return Value::Null;
    }
    let (s1, s2) = stems(a.picks);
    let res = solve_switch_connection(&s1, &s2, a.speed, shift, a.mids.as_deref(), a.seed.clone(), a.form);
    if !stands(&res, true) || turnouts_on_elements(a.picks, s1.dir, shift, &res) {
        return res;
    }
    let mut out = res;
    out["valid"] = json!(false);
    out["overridable"] = json!(false);
    out["reason"] = json!("off_element");
    out
}

/// Why a shift has no connection, in the words the panel shows (`REASON_MSG`).
pub fn reason_msg() -> Value {
    json!({
        "no_solution": "scurve_no_solution", "too_short": "scurve_too_short", "too_sharp": "scurve_too_sharp",
        "branch_too_sharp": "scurve_branch_too_sharp", "cant_mismatch": "scurve_cant_mismatch", "cant_over": "scurve_cant_over",
        "off_element": "scurve_off_element", "min_element_length": "scurve_min_element_length", "mid_unknowns": "scurve_mid_unknowns",
        "mid_no_solution": "scurve_mid_no_solution", "mid_element_short": "scurve_mid_element_short", "mid_jump": "scurve_mid_jump",
    })
}

struct Span {
    toe: f64,
    far: f64,
    len: f64,
    node: f64,
}

fn turnout_span(picks: &[Value], k: usize, shift: f64, res: &Value) -> Span {
    let p = &picks[k];
    let dir = if k == 0 {
        stems(picks).0.dir
    } else if truthy(res.get("flipped2")) {
        -1.0
    } else {
        1.0
    };
    let along = js_number(p.get("along"));
    let toe = if k == 0 {
        along + dir * shift
    } else {
        along + dir * js_number(res.get("s2"))
    };
    let far = toe + dir * (if k == 0 { 1.0 } else { -1.0 }) * through_of(res, k);
    let len = route_len(p);
    Span {
        toe,
        far,
        len,
        node: if far > toe { 0.0 } else { len },
    }
}

/// A piece a connection leaves of its elements.
#[derive(Clone, Debug)]
pub struct Remnant {
    pub track: usize,
    pub side: &'static str,
    pub length: f64,
    pub run: f64,
    pub v: Option<f64>,
    pub l_min: Option<f64>,
    pub short: bool,
}

impl Remnant {
    pub fn to_json(&self) -> Value {
        json!({ "track": self.track, "side": self.side, "length": number(self.length), "run": number(self.run), "v": self.v.map(number), "lMin": self.l_min.map(number), "short": self.short })
    }
}

fn we_element(pick: &Value, far: f64, forward: bool) -> (f64, Option<f64>, f64) {
    let pieces = pick_pieces(pick);
    let p = if forward {
        pieces
            .iter()
            .find(|q| far <= js_number(q.get("s0")) + route_len(q) + NODE_TOL)
            .unwrap_or(&pieces[pieces.len() - 1])
    } else {
        pieces.iter().rev().find(|q| far >= js_number(q.get("s0")) - NODE_TOL).unwrap_or(&pieces[0])
    };
    let runs = |i: usize| p.get("runs").and_then(|r| r.get(i)).and_then(Value::as_f64).unwrap_or(0.0);
    let s0 = js_number(p.get("s0"));
    let speed = f(p, "speed").filter(|s| *s != 0.0 && !s.is_nan());
    (
        if forward { s0 + route_len(p) - far } else { far - s0 },
        speed,
        if forward { runs(1) } else { runs(0) },
    )
}

/// The pieces a connection leaves of its two elements (`connectionRemnants`).
pub fn connection_remnants(picks: &[Value], shift: f64, res: &Value, speed: f64) -> Vec<Remnant> {
    let mut out = Vec::new();
    for k in [0usize, 1] {
        let sp = turnout_span(picks, k, shift, res);
        let forward = sp.far > sp.toe;
        let before = if forward { sp.toe } else { sp.len - sp.toe };
        let runs = |i: usize| picks[k].get("runs").and_then(|r| r.get(i)).and_then(Value::as_f64).unwrap_or(0.0);
        let (at_start, at_end) = (runs(0), runs(1));
        let (end_len, end_speed, end_beyond) = we_element(&picks[k], sp.far, forward);
        let speed_v = (speed != 0.0 && !speed.is_nan()).then_some(speed);
        for (side, length, v, beyond) in [
            ("before", before, speed_v, if forward { at_start } else { at_end }),
            ("after", end_len, end_speed, end_beyond),
        ] {
            let l = 0f64.max(length);
            let run = if l > NODE_TOL { l + beyond } else { l };
            let l_min = v.and_then(min_element_length);
            out.push(Remnant {
                track: k,
                side,
                length: l,
                run,
                v,
                l_min,
                short: l_min.is_some_and(|m| l > NODE_TOL && run < m - NODE_TOL),
            });
        }
    }
    out
}

fn shift_for_second_toe(a: &Ask, target: f64, from: f64, lo: f64, hi: f64) -> Option<f64> {
    let off = |s: f64| -> Option<f64> {
        let res = solve_connection(a, s);
        stands(&res, a.lenient).then(|| turnout_span(a.picks, 1, s, &res).toe - target)
    };
    let span = hi - lo;
    if !(span > 0.0) {
        return None;
    }
    let n = (span / 0.5f64.max(span / 400.0)).ceil() as usize;
    let mut best: Option<f64> = None;
    let (mut prev_s, mut prev_f): (Option<f64>, Option<f64>) = (None, None);
    for i in 0..=n {
        let s = lo + span * i as f64 / n as f64;
        let fv = off(s);
        if let (Some(fs), Some(pf), Some(ps)) = (fv, prev_f, prev_s)
            && (pf <= 0.0) != (fs <= 0.0)
        {
            let (mut x, mut y, mut fa) = (ps, s, pf);
            let mut it = 0;
            while it < 60 && y - x > 1e-7 {
                let m = (x + y) / 2.0;
                let Some(fm) = off(m) else { break };
                if (fa <= 0.0) == (fm <= 0.0) {
                    x = m;
                    fa = fm;
                } else {
                    y = m;
                }
                it += 1;
            }
            let root = (x + y) / 2.0;
            if best.is_none_or(|b| (root - from).abs() < (b - from).abs()) {
                best = Some(root);
            }
        }
        prev_s = Some(s);
        prev_f = fv;
    }
    best
}

/// The connection at the slider's shift, held to the minimum element length (`settleConnection`).
pub struct Settled {
    pub shift: f64,
    pub result: Value,
    pub moved: Vec<usize>,
    pub remnants: Vec<Remnant>,
}

impl Settled {
    pub fn to_json(&self) -> Value {
        json!({ "shift": number(self.shift), "result": self.result, "moved": self.moved, "remnants": self.remnants.iter().map(Remnant::to_json).collect::<Vec<_>>() })
    }
}

pub fn settle_connection(a: &Ask, shift: f64) -> Settled {
    let mut shift = shift;
    let mut res = solve_connection(a, shift);
    if !truthy(res.get("valid")) {
        return Settled {
            shift,
            result: res,
            moved: vec![],
            remnants: vec![],
        };
    }
    let mut moved = Vec::new();
    let short_before = |k: usize, r: &Value, s: f64| {
        connection_remnants(a.picks, s, r, a.speed)
            .into_iter()
            .find(|x| x.track == k && x.side == "before" && x.short)
    };
    let fail = |s: f64, r: &Value, piece: &Remnant, moved: &[usize]| -> Settled {
        let mut result = r.clone();
        result["valid"] = json!(false);
        result["overridable"] = json!(true);
        result["reason"] = json!("min_element_length");
        result["short"] = piece.to_json();
        Settled {
            shift: s,
            result,
            moved: moved.to_vec(),
            remnants: connection_remnants(a.picks, s, r, a.speed),
        }
    };
    let open_end = |k: usize, node: f64| truthy(a.picks[k].get("trackEnds").and_then(|t| t.get(if node == 0.0 { 0 } else { 1 })));
    if let Some(first) = short_before(0, &res, shift) {
        let node = turnout_span(a.picks, 0, shift, &res).node;
        if open_end(0, node) {
            return fail(shift, &res, &first, &moved);
        }
        let s = (node - js_number(a.picks[0].get("along"))) * stems(a.picks).0.dir;
        let r = solve_connection(a, s);
        if !truthy(r.get("valid")) {
            return fail(shift, &res, &first, &moved);
        }
        shift = s;
        res = r;
        moved.push(0);
    }
    if let Some(second) = short_before(1, &res, shift) {
        let node = turnout_span(a.picks, 1, shift, &res).node;
        if open_end(1, node) {
            return fail(shift, &res, &second, &moved);
        }
        let dir1 = stems(a.picks).0.dir;
        let along = js_number(a.picks[0].get("along"));
        let sa = -dir1 * along;
        let sb = dir1 * (route_len(&a.picks[0]) - along);
        let plain = Ask { lenient: false, ..a.clone() };
        let s = shift_for_second_toe(&plain, node, shift, sa.min(sb), sa.max(sb));
        let r = s.map(|s| solve_connection(a, s)).unwrap_or(Value::Null);
        if !truthy(r.get("valid")) {
            return fail(shift, &res, &second, &moved);
        }
        shift = s.unwrap_or(shift);
        res = r;
        moved.push(1);
    }
    let remnants = connection_remnants(a.picks, shift, &res, a.speed);
    if let Some(still) = remnants.iter().find(|x| x.short).cloned() {
        return fail(shift, &res, &still, &moved);
    }
    Settled {
        shift,
        result: res,
        moved,
        remnants,
    }
}

/// The shift range over which the connection stands (`computeShiftBounds`): (min, max).
pub fn compute_shift_bounds(a: &Ask) -> (f64, f64) {
    let p1 = &a.picks[0];
    let dir1 = stems(a.picks).0.dir;
    let along = js_number(p1.get("along"));
    let sa = -dir1 * along;
    let sb = dir1 * (route_len(p1) - along);
    let (lo1, hi1) = (sa.min(sb), sa.max(sb));
    let span = hi1 - lo1;
    if !(span > 0.0) {
        return (0.0, 0.0);
    }
    let n = (span / 1f64.max(span / 200.0)).ceil() as usize;
    let probe = |s: f64| stands(&solve_connection(a, s), a.lenient);
    let edge = |mut bad: f64, mut good: f64| {
        while (good - bad).abs() > 1e-6 {
            let m = (bad + good) / 2.0;
            if probe(m) {
                good = m;
            } else {
                bad = m;
            }
        }
        good
    };
    let c = hi1.min(lo1.max(0.0));
    let miss = |r: (f64, f64)| 0f64.max(r.0 - c).max(c - r.1);
    let mut best: Option<(f64, f64)> = None;
    let mut run: Option<(f64, f64)> = None;
    let close = |bad: Option<f64>, run: &mut Option<(f64, f64)>, best: &mut Option<(f64, f64)>| {
        if let (Some(r), Some(b)) = (run.as_mut(), bad) {
            r.1 = edge(b, r.1);
        }
        if let Some(r) = *run
            && best.is_none_or(|b| miss(r) < miss(b))
        {
            *best = Some(r);
        }
        *run = None;
    };
    let mut prev: Option<f64> = None;
    for i in 0..=n {
        let s = lo1 + span * i as f64 / n as f64;
        if probe(s) {
            run = Some(match run {
                Some(r) => (r.0, s),
                None => (prev.map(|p| edge(p, s)).unwrap_or(s), s),
            });
        } else {
            close(Some(s), &mut run, &mut best);
        }
        prev = Some(s);
    }
    close(None, &mut run, &mut best);
    best.unwrap_or((c, c))
}

/// How far the first toe stands from the element change behind it (`waOffset`).
pub fn wa_offset(picks: &[Value]) -> f64 {
    let dir1 = stems(picks).0.dir;
    let along = js_number(picks[0].get("along"));
    if dir1 > 0.0 { along } else { route_len(&picks[0]) - along }
}

fn second_toe_on_node(a: &Ask, bounds: (f64, f64)) -> Option<f64> {
    if bounds.1 <= bounds.0 {
        return None;
    }
    let mut res = solve_connection(a, bounds.0);
    if res.is_null() {
        res = solve_connection(a, bounds.1);
    }
    if !stands(&res, a.lenient) {
        return None;
    }
    let node = turnout_span(a.picks, 1, bounds.0, &res).node;
    if truthy(a.picks[1].get("trackEnds").and_then(|t| t.get(if node == 0.0 { 0 } else { 1 }))) {
        return None;
    }
    for s in [bounds.0, bounds.1] {
        let r = solve_connection(a, s);
        if stands(&r, a.lenient) && (turnout_span(a.picks, 1, s, &r).toe - node).abs() < NODE_TOL {
            return Some(s);
        }
    }
    shift_for_second_toe(a, node, 0.0, bounds.0, bounds.1)
}

fn second_toe_crossings(a: &Ask, targets: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    let toe = |s: f64| -> Option<f64> {
        let res = solve_connection(a, s);
        stands(&res, a.lenient).then(|| turnout_span(a.picks, 1, s, &res).toe)
    };
    let span = hi - lo;
    if !(span > 0.0) {
        return vec![];
    }
    let n = (span / 0.5f64.max(span / 200.0)).ceil() as usize;
    let mut out = Vec::new();
    let (mut prev_s, mut prev_t): (f64, Option<f64>) = (f64::NAN, None);
    for i in 0..=n {
        let s = lo + span * i as f64 / n as f64;
        let t = toe(s);
        if let (Some(tv), Some(pt)) = (t, prev_t) {
            for &target in targets {
                if (pt - target <= 0.0) == (tv - target <= 0.0) {
                    continue;
                }
                let (mut x, mut y, mut fa) = (prev_s, s, pt - target);
                let mut it = 0;
                while it < 60 && y - x > 1e-7 {
                    let m = (x + y) / 2.0;
                    let Some(tm) = toe(m) else { break };
                    if (fa <= 0.0) == (tm - target <= 0.0) {
                        x = m;
                        fa = tm - target;
                    } else {
                        y = m;
                    }
                    it += 1;
                }
                out.push((x + y) / 2.0);
            }
        }
        prev_s = s;
        prev_t = t;
    }
    out
}

/// Is the element between the branches a straight (`hasStraightMid`)?
pub fn has_straight_mid(res: &Value) -> bool {
    !res.is_null() && res.get("mid").is_none_or(Value::is_null) && f(res, "Lg").is_some_and(f64::is_finite) && get(res, "signedRg").is_none()
}

/// The shifts at which the element between the branches is a straight (`straightShifts`).
pub fn straight_shifts(a: &Ask, bounds: (f64, f64)) -> Vec<f64> {
    let span = bounds.1 - bounds.0;
    if !(span > 0.0) {
        return vec![];
    }
    let sign = |s: f64| -> Option<i8> {
        let res = solve_connection(a, s);
        if !stands(&res, a.lenient) || !res.get("mid").is_none_or(Value::is_null) || !f(&res, "Lg").is_some_and(f64::is_finite) {
            return None;
        }
        if has_straight_mid(&res) {
            return Some(0);
        }
        let d = js_number(res.get("delta"));
        Some(if d > 0.0 {
            1
        } else if d < 0.0 {
            -1
        } else {
            0
        })
    };
    let n = (span / 0.5f64.max(span / 200.0)).ceil() as usize;
    let at: Vec<f64> = (0..=n).map(|i| bounds.0 + span * i as f64 / n as f64).collect();
    let signs: Vec<Option<i8>> = at.iter().map(|s| sign(*s)).collect();
    let mut out = Vec::new();
    let nonzero = |x: Option<i8>| x != Some(0);
    for i in 0..at.len() {
        let before = if i > 0 { signs[i - 1] } else { None };
        let after = signs.get(i + 1).copied().flatten();
        let after_raw = if i + 1 < signs.len() { signs[i + 1] } else { None };
        let _ = after;
        if signs[i] == Some(0) && nonzero(before) && nonzero(after_raw) {
            out.push(at[i]);
        }
        let truthy_sign = |x: Option<i8>| x.is_some_and(|v| v != 0);
        if i == 0 || !truthy_sign(signs[i - 1]) || !truthy_sign(signs[i]) || signs[i - 1] == signs[i] {
            continue;
        }
        let (mut x, mut y, mut root) = (at[i - 1], at[i], None);
        let mut it = 0;
        while it < 80 && y - x > 1e-9 {
            let m = (x + y) / 2.0;
            let Some(fm) = sign(m) else { break };
            if fm == 0 {
                root = Some(m);
                break;
            }
            if Some(fm) == signs[i - 1] {
                x = m;
            } else {
                y = m;
            }
            it += 1;
        }
        let root = root.unwrap_or((x + y) / 2.0);
        if sign(root) == Some(0) {
            out.push(root);
        }
    }
    out
}

/// The shifts the position slider snaps to (`shiftSnaps`).
pub fn shift_snaps(a: &Ask, bounds: (f64, f64)) -> Vec<f64> {
    if !(bounds.1 > bounds.0) {
        return vec![];
    }
    let at_min = solve_connection(a, bounds.0);
    let res = if stands(&at_min, a.lenient) { at_min } else { solve_connection(a, bounds.1) };
    if !stands(&res, a.lenient) {
        return vec![];
    }
    let (p1, p2) = (&a.picks[0], &a.picks[1]);
    let dir1 = stems(a.picks).0.dir;
    let dir2 = if truthy(res.get("flipped2")) { -1.0 } else { 1.0 };
    let nodes = |p: &Value| -> Vec<f64> {
        let mut v: Vec<f64> = pick_pieces(p).iter().map(|q| js_number(q.get("s0"))).collect();
        v.push(pick_extent(p).1);
        v
    };
    let along1 = js_number(p1.get("along"));
    let mut out = Vec::new();
    for node in [0.0, route_len(p1)] {
        out.push((node - along1) * dir1);
    }
    for node in nodes(p1) {
        out.push((node - dir1 * through_of(&res, 0) - along1) * dir1);
    }
    let mut targets = vec![0.0, route_len(p2)];
    targets.extend(nodes(p2).into_iter().map(|n| n + dir2 * through_of(&res, 1)));
    out.extend(second_toe_crossings(a, &targets, bounds.0, bounds.1));
    out.extend(straight_shifts(a, bounds));
    unique_stations(&out)
        .into_iter()
        .filter(|s| *s >= bounds.0 - 1e-9 && *s <= bounds.1 + 1e-9)
        .collect()
}

/// Where the dialog lays the connection first (`preferredShift`).
pub fn preferred_shift(a: &Ask, bounds: (f64, f64)) -> f64 {
    let ok = |s: f64| stands(&settle_connection(a, s).result, a.lenient);
    let straight = |s: f64| {
        let st = settle_connection(a, s);
        stands(&st.result, a.lenient) && (st.shift - s).abs() < 1e-6 && has_straight_mid(&st.result)
    };
    let nearest = |x: &f64, y: &f64| (x.abs() - y.abs()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal);
    let mut candidates = Vec::new();
    let d0 = wa_offset(a.picks);
    let node1 = if stems(a.picks).0.dir > 0.0 { 0 } else { 1 };
    if !truthy(a.picks[0].get("trackEnds").and_then(|t| t.get(node1))) && -d0 >= bounds.0 - 1e-6 {
        candidates.push(-d0);
    }
    if let Some(s2) = second_toe_on_node(a, bounds) {
        candidates.push(s2);
    }
    let mut on_node: Vec<f64> = candidates.iter().copied().filter(|s| straight(*s)).collect();
    on_node.sort_by(nearest);
    if let Some(s) = on_node.first() {
        return *s;
    }
    let mut straights: Vec<f64> = straight_shifts(a, bounds).into_iter().filter(|s| straight(*s)).collect();
    straights.sort_by(nearest);
    if let Some(s) = straights.first() {
        return *s;
    }
    let mut fit: Vec<f64> = candidates.into_iter().filter(|s| ok(*s)).collect();
    fit.sort_by(nearest);
    fit.first().copied().unwrap_or_else(|| bounds.1.min(bounds.0.max(0.0)))
}

fn pt(v: &Value) -> [f64; 2] {
    [
        v.get(0).and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ]
}

struct Junction {
    line_route: &'static str,
    conn_route: &'static str,
    line_end: Utm,
    line_length: f64,
    record: Value,
}

fn build_junction_switch(o: &JunctionAsk) -> Junction {
    let j = Utm {
        e: o.node[0],
        n: o.node[1],
        zone: o.zone,
    };
    let geom = compute_switch_geometry_utm(j, o.bearing, o.form, o.side, false, Some(o.wgs), &o.stem_r);
    let swapped = if o.branch_straight {
        geom.get("swapped").filter(|v| !v.is_null()).cloned()
    } else {
        None
    };
    let shown = swapped.clone().unwrap_or_else(|| geom.clone());
    let (b1, b2) = if swapped.is_some() { (&o.ahead, &o.conn) } else { (&o.conn, &o.ahead) };
    let mut record = o.identity.as_object().cloned().unwrap_or_default();
    record.insert("number".into(), o.number.clone());
    record.insert("trailing".into(), json!(false));
    record.insert("speed".into(), number(o.speed));
    if swapped.is_some() {
        record.insert("swapped".into(), json!(true));
    } else if let Some(r) = get(&geom, "stemAtToe").and_then(Value::as_f64).filter(|r| *r != 0.0) {
        record.insert("mainRadius".into(), number(r));
    }
    record.insert(
        "ports".into(),
        json!({
            "A": { "trackId": o.behind.0, "endpoint": o.behind.1 },
            "B1": { "trackId": b1.0, "endpoint": b1.1 },
            "B2": { "trackId": b2.0, "endpoint": b2.1 },
        }),
    );
    for k in ["fillCoords", "lcsCoords", "labelCoords", "bauform"] {
        record.insert(k.into(), shown.get(k).cloned().unwrap_or(Value::Null));
    }
    let (line_end, line_length) = match &swapped {
        Some(s) => (Utm::from_json(&s["curvedUtm"]), js_number(s.get("arcLen"))),
        None => (Utm::from_json(&geom["straightUtm"]), js_number(geom.get("straightLen"))),
    };
    Junction {
        line_route: if swapped.is_some() { "branch" } else { "main" },
        conn_route: if swapped.is_some() { "main" } else { "branch" },
        line_end,
        line_length,
        record: Value::Object(record),
    }
}

struct JunctionAsk<'a> {
    wgs: [f64; 2],
    node: [f64; 2],
    zone: i64,
    bearing: f64,
    side: &'a str,
    form: &'a Value,
    speed: f64,
    number: Value,
    identity: Value,
    stem_r: MainR,
    branch_straight: bool,
    behind: (Value, Value),
    ahead: (Value, Value),
    conn: (Value, Value),
}

fn bent_onto(chain: Option<&Value>, at_toe: Option<f64>) -> MainR {
    if let Some(c) = chain.and_then(Value::as_array) {
        let routes = to_chain(&Value::Array(c.clone()));
        if routes.len() > 1 || (routes.len() == 1 && switch_route_varies(&routes[0])) {
            return MainR::Chain(routes);
        }
    }
    MainR::Radius(at_toe)
}

fn split_at_toe(track: &Value, el_idx: usize, station: f64, junction: &Value, bearing: f64, existing: &mut Vec<String>) -> Option<Value> {
    let es = els(track);
    let n = es.len();
    let len = js_number(es[el_idx].get("length"));
    let j = if station <= NODE_TOL {
        Some(el_idx)
    } else if station >= len - NODE_TOL {
        Some(el_idx + 1)
    } else {
        None
    };
    let mut cut = junction.clone();
    if is_transition(&es[el_idx]) {
        cut["station"] = number(station);
    }
    match j {
        Some(j) if j > 0 && j < n => Some(split_track_at_joint(track, j, bearing, existing)),
        _ => split_element_at(track, el_idx, &cut, bearing, existing),
    }
}

fn carve_through(split: &Value, cut: Utm, mark: &Value, length: f64) -> Option<Vec<Value>> {
    let ahead = &split["ahead"];
    let endpoint = split["aheadEndpoint"].as_str().unwrap_or("BEGIN");
    if !els(ahead).iter().all(is_usable_stem) {
        // `accepts` refuses a route over a Bloss curve; the carve below refuses it too.
    }
    let carved = carve_switch_route(ahead, endpoint, &cut.to_json(), mark, Some(length))?;
    Some(
        split["tracks"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|t| {
                if strict_eq(t.get("id"), carved.get("id")) {
                    carved.clone()
                } else {
                    t.clone()
                }
            })
            .collect(),
    )
}

fn plane_pt(v: &Value, crs: i64) -> Utm {
    to_plane(Utm::from_json(v), crs)
}

/// The commit of the S-curve dialog (`buildSCurve`).
pub fn build_s_curve(o: &Value) -> Value {
    let res = &o["result"];
    let force = truthy(o.get("force"));
    if !stands(res, force) {
        return Value::Null;
    }
    let picks: Vec<Value> = o["picks"].as_array().cloned().unwrap_or_default();
    let tracks: Vec<Value> = o["tracks"].as_array().cloned().unwrap_or_default();
    let switches: Vec<Value> = o["switches"].as_array().cloned().unwrap_or_default();
    let speed = js_number(o.get("speed"));
    if picks.len() < 2 {
        return Value::Null;
    }
    let (g1, g2) = (&picks[0], &picks[1]);
    let t1 = tracks.iter().find(|t| strict_eq(t.get("id"), g1.get("trackId")));
    let t2 = tracks.iter().find(|t| strict_eq(t.get("id"), g2.get("trackId")));
    let (Some(t1), Some(t2)) = (t1, t2) else { return Value::Null };
    if strict_eq(t1.get("id"), t2.get("id")) {
        return Value::Null;
    }
    let zone = js_number(res.get("zone")) as i64;
    let sw_type = &res["switchType"];
    let sw_type2 = get(res, "switchType2").unwrap_or(sw_type);
    let side = if js_number(res.get("side")) > 0.0 { "left" } else { "right" };
    let b1 = js_number(res.get("bearing1"));
    let (e1, e2) = (js_number(t1.get("epsg")) as i64, js_number(t2.get("epsg")) as i64);
    let g1_zone = js_number(g1.get("zone")) as i64;
    let tp2 = Utm::from_json(&res["TP2"]);
    let b2_track = if e2 != g1_zone {
        transform_grid_bearing(tp2.e, tp2.n, js_number(res.get("bearing2")), g1_zone, e2).unwrap_or(f64::NAN)
    } else {
        js_number(res.get("bearing2"))
    };
    let mut existing: Vec<String> = tracks.iter().map(|t| js_string(t.get("name"))).collect();
    let dir1 = stems(&picks).0.dir;
    let station1 = js_number(g1.get("along")) + dir1 * js_number(res.get("s1"));
    let station2 = js_number(g2.get("along")) + (if truthy(res.get("flipped2")) { -1.0 } else { 1.0 }) * js_number(res.get("s2"));
    let idx = |g: &Value| g.get("elIdx").and_then(Value::as_u64).unwrap_or(0) as usize;
    let s1 = split_at_toe(t1, idx(g1), station1, &plane_pt(&res["TP1"], e1).to_json(), b1, &mut existing);
    let s2 = split_at_toe(t2, idx(g2), station2, &plane_pt(&res["TP2"], e2).to_json(), b2_track, &mut existing);
    let (Some(s1), Some(s2)) = (s1, s2) else { return Value::Null };
    let identities = o.get("identities").and_then(Value::as_array);
    let ident = |i: usize| identities.and_then(|a| a.get(i)).filter(|v| truthy(Some(v)));
    let no1 = ident(0)
        .and_then(|i| get(i, "number"))
        .cloned()
        .unwrap_or_else(|| number(next_switch_number(&switches, &[])));
    let no2 = ident(1)
        .and_then(|i| get(i, "number"))
        .cloned()
        .unwrap_or_else(|| number(next_switch_number(&switches, &[js_number(Some(&no1))])));
    let identity = |old: Option<&Value>, no: &Value, form: &Value| -> Value {
        let mut id = new_switch_fields("switch").as_object().cloned().unwrap_or_default();
        id.insert("name".into(), json!(switch_designation(no, None)));
        id.insert("label".into(), form.get("label").cloned().unwrap_or(Value::Null));
        if let Some(o) = old {
            id.insert("switchId".into(), o.get("switchId").cloned().unwrap_or(Value::Null));
            id.insert("name".into(), o.get("name").cloned().unwrap_or(Value::Null));
        }
        Value::Object(id)
    };
    let id1 = identity(ident(0), &no1, sw_type);
    let id2 = identity(ident(1), &no2, sw_type2);
    let old_conn = o.get("connTrack").filter(|v| truthy(Some(v)));
    let conn_id = old_conn.and_then(|c| get(c, "id")).cloned().unwrap_or_else(|| json!(generate_id()));
    let straight = |chain: &Value| chain.as_array().into_iter().flatten().all(|p| get(p, "r1").is_none() && get(p, "r2").is_none());
    let port = |t: &Value, e: &Value| (t.get("id").cloned().unwrap_or(Value::Null), e.clone());
    let j1 = build_junction_switch(&JunctionAsk {
        wgs: pt(&res["tp1Wgs"]),
        node: [js_number(res["TP1"].get("easting")), js_number(res["TP1"].get("northing"))],
        zone,
        bearing: b1,
        side,
        form: sw_type,
        speed,
        number: no1.clone(),
        identity: id1.clone(),
        stem_r: bent_onto(res.get("stemRoute1"), f(res, "stemR1")),
        branch_straight: straight(&res["chain1"]),
        behind: port(&s1["behind"], &s1["behindEndpoint"]),
        ahead: port(&s1["ahead"], &s1["aheadEndpoint"]),
        conn: (conn_id.clone(), json!("BEGIN")),
    });
    let j2 = build_junction_switch(&JunctionAsk {
        wgs: pt(&res["tp2Wgs"]),
        node: [tp2.e, tp2.n],
        zone,
        bearing: js_number(res.get("bearing2")),
        side,
        form: sw_type2,
        speed,
        number: no2.clone(),
        identity: id2.clone(),
        stem_r: bent_onto(res.get("stemRoute2"), f(res, "stemR2").map(|r| -r)),
        branch_straight: straight(&res["chain2"]),
        behind: port(&s2["behind"], &s2["behindEndpoint"]),
        ahead: port(&s2["ahead"], &s2["aheadEndpoint"]),
        conn: (conn_id.clone(), json!("END")),
    });
    let built = build_connection_elements(res, speed);
    let mark = |list: &Value, id: &Value, route: &str| -> Vec<Value> {
        let m = switch_element_mark(id, route);
        list.as_array()
            .into_iter()
            .flatten()
            .map(|el| {
                let mut e = el.clone();
                for (k, v) in m.as_object().into_iter().flatten() {
                    e[k] = v.clone();
                }
                e
            })
            .collect()
    };
    let mut conn_elements = mark(&built["branch1"], &id1, j1.conn_route);
    conn_elements.extend(built["midEls"].as_array().cloned().unwrap_or_default());
    conn_elements.extend(mark(&built["branch2"], &id2, j2.conn_route));
    let mut conn: Object = old_conn.and_then(Value::as_object).cloned().unwrap_or_default();
    conn.remove("heights");
    conn.insert("id".into(), conn_id.clone());
    let name = old_conn
        .and_then(|c| get(c, "name"))
        .cloned()
        .unwrap_or_else(|| json!(next_track_name("connection", &existing)));
    conn.insert("name".into(), name);
    conn.insert("epsg".into(), json!(zone));
    conn.insert("coordinates".into(), Value::Array(rebuild_coords(&conn_elements)));
    conn.insert("elements".into(), Value::Array(conn_elements));
    let s1_tracks = carve_through(&s1, to_plane(j1.line_end, e1), &switch_element_mark(&id1, j1.line_route), j1.line_length);
    let s2_tracks = carve_through(&s2, to_plane(j2.line_end, e2), &switch_element_mark(&id2, j2.line_route), j2.line_length);
    let (Some(s1t), Some(s2t)) = (s1_tracks, s2_tracks) else {
        return json!({ "carveError": number(j1.line_length.max(j2.line_length)) });
    };
    let ids = |v: &[Value]| v.iter().map(|t| t.get("id").cloned().unwrap_or(Value::Null)).collect::<Vec<_>>();
    let mut add: Vec<Value> = s1t.clone();
    add.extend(s2t.iter().cloned());
    add.push(Value::Object(conn));
    json!({
        "removeTrackIds": [t1.get("id"), t2.get("id")],
        "addTracks": add,
        "addSwitches": [j1.record, j2.record],
        "remap": [{ "oldId": t1.get("id"), "newId": ids(&s1t) }, { "oldId": t2.get("id"), "newId": ids(&s2t) }],
    })
}

/// A route chain as JSON, for the ops.
pub fn routes_json(r: &[Route]) -> Value {
    rail_across_switch::route::chain_json(r)
}
