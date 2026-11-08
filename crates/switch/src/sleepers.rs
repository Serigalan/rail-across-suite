// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Where the sleepers of a turnout lie, along its bisector (`sleepers.js`).

use rail_across_js::{get, js_number, number, strict_eq};
use rail_across_track::cross_section::superstructure_at;
use rail_across_track::heights::{element_at_station, point_at_station_utm, track_length};
use rail_across_track::platforms::{element_stations, point_on_element};
use rail_across_track::switch_model::{element_belongs_to_switch, port_end, port_track, turnout_diverging_port, turnout_line_port, turnout_line_route};
use serde_json::{Value, json};

use crate::catalogue::{switch_type_by_label, switch_variant_for};

pub const FIRST_SLEEPER: f64 = 0.3;
pub const SLEEPER_SPACING: f64 = 0.6;
const STEP: f64 = 1.0;
const NEWTON: usize = 12;
const EPS: f64 = 1e-9;

/// Where a form is looked up: the catalogue, or a table the caller hands in.
pub type FormOf<'a> = &'a dyn Fn(Option<&str>) -> Option<Value>;

pub fn catalogue_form(label: Option<&str>) -> Option<Value> {
    switch_type_by_label(label).cloned()
}

/// How far behind WE the ldS lies (`switchLds`).
pub fn switch_lds(sw: &Value, form_of: FormOf) -> Option<f64> {
    let lds = form_of(sw.get("label").and_then(Value::as_str)).and_then(|f| f.get("lds").and_then(Value::as_f64))?;
    (lds.is_finite() && lds >= 0.0).then_some(lds)
}

/// Length of the main route, WA to WE (`mainRouteLength`).
pub fn main_route_length(track: &Value, sw: &Value) -> f64 {
    let line = turnout_line_route(sw);
    track
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|el| element_belongs_to_switch(el, sw) && get(el, "switchRoute").and_then(Value::as_str).unwrap_or("main") == line)
        .map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0))
        .sum()
}

fn epsg_of(track: &Value) -> i64 {
    js_number(track.get("epsg")) as i64
}

/// A point [E, N] at a station of a track.
pub fn point_at(track: &Value, station: f64) -> Option<[f64; 2]> {
    let els = track.get("elements").and_then(Value::as_array)?;
    let (i, s) = element_at_station(els, station)?;
    let p = point_at_station_utm(&els[i], s, epsg_of(track))?;
    Some([p.e, p.n])
}

/// The unit tangent at a station of a track.
pub fn tangent_at(track: &Value, station: f64, length: f64) -> Option<[f64; 2]> {
    let h = 0.05;
    let a = point_at(track, 0f64.max(station - h))?;
    let b = point_at(track, length.min(station + h))?;
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let n = dx.hypot(dy);
    (n > 0.0).then(|| [dx / n, dy / n])
}

/// A track as seen from the switch toe (`routeFrom`).
#[derive(Clone)]
pub struct RouteFrom {
    pub track: Value,
    pub length: f64,
    pub from_begin: bool,
    pub sense: f64,
    rows: Vec<(usize, f64, f64)>,
}

/// A point and its direction away from WA.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub p: [f64; 2],
    pub t: [f64; 2],
}

impl RouteFrom {
    pub fn station(&self, d: f64) -> f64 {
        if self.from_begin { d } else { self.length - d }
    }
    pub fn distance(&self, station: f64) -> f64 {
        if self.from_begin { station } else { self.length - station }
    }
    pub fn pose(&self, d: f64) -> Option<Pose> {
        let st = self.station(0f64.max(self.length.min(d)));
        let rows = &self.rows;
        if rows.is_empty() {
            return None;
        }
        let (mut lo, mut hi) = (0usize, rows.len() - 1);
        while lo < hi {
            let mid = (lo + hi) >> 1;
            if st <= rows[mid].2 + 1e-9 {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let (idx, start, _) = rows[lo];
        let el = &self.track["elements"][idx];
        let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        let (u, bearing) = point_on_element(el, epsg_of(&self.track), 0f64.max(len.min(st - start)))?;
        let b = bearing * std::f64::consts::PI / 180.0;
        Some(Pose {
            p: [u.e, u.n],
            t: [b.sin() * self.sense, b.cos() * self.sense],
        })
    }
    pub fn to_json(&self) -> Value {
        json!({ "trackId": self.track.get("id"), "length": number(self.length), "sense": number(self.sense) })
    }
}

pub fn route_from(tracks: &[Value], sw: &Value, port: &str) -> Option<RouteFrom> {
    let id = port_track(sw, port);
    let track = tracks.iter().find(|t| strict_eq(t.get("id"), id))?.clone();
    let length = track_length(&track);
    let from_begin = port_end(sw, port) != Some("END");
    let rows = element_stations(&track);
    Some(RouteFrom {
        track,
        length,
        from_begin,
        sense: if from_begin { 1.0 } else { -1.0 },
        rows,
    })
}

fn add(p: [f64; 2], v: [f64; 2], k: f64) -> [f64; 2] {
    [p[0] + v[0] * k, p[1] + v[1] * k]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let n = v[0].hypot(v[1]);
    (n > EPS).then(|| [v[0] / n, v[1] / n])
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn foot(route: &RouteFrom, x: [f64; 2], guess: f64) -> Option<f64> {
    let mut d = guess;
    for _ in 0..NEWTON {
        let Pose { p, t } = route.pose(d)?;
        let step = dot(sub(x, p), t);
        d = 0f64.max(route.length.min(d + step));
        if step.abs() < 1e-7 {
            break;
        }
    }
    Some(d)
}

/// Where a route crosses the line through `x` square to `dir` (`routeCut`).
pub fn route_cut(route: &RouteFrom, x: [f64; 2], dir: [f64; 2], guess: f64) -> Option<f64> {
    let mut d = guess;
    for _ in 0..NEWTON {
        let Pose { p, t } = route.pose(d)?;
        let (f, df) = (dot(sub(p, x), dir), dot(t, dir));
        if df.abs() < EPS {
            return None;
        }
        let step = f / df;
        d -= step;
        if d < -1e-6 || d > route.length + 1e-6 {
            return None;
        }
        if step.abs() < 1e-7 {
            break;
        }
    }
    Some(0f64.max(route.length.min(d)))
}

/// A traced sample of the bisector.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub s: f64,
    pub x: Option<[f64; 2]>,
    pub dir: Option<[f64; 2]>,
    pub m: f64,
    pub b: f64,
}

impl Sample {
    pub fn to_json(self) -> Value {
        let mut o = serde_json::Map::new();
        o.insert("s".into(), number(self.s));
        if let Some(x) = self.x {
            o.insert("x".into(), json!([number(x[0]), number(x[1])]));
        }
        if let Some(d) = self.dir {
            o.insert("dir".into(), json!([number(d[0]), number(d[1])]));
        }
        o.insert("m".into(), number(self.m));
        o.insert("b".into(), number(self.b));
        Value::Object(o)
    }
}

fn trace_bisector(main: &RouteFrom, branch: &RouteFrom, lds_main: f64) -> Option<Vec<Sample>> {
    let start = main.pose(0.0)?;
    let (start_p, t0) = (start.p, start.t);
    let mut samples = vec![Sample {
        s: 0.0,
        x: Some(start_p),
        dir: Some(t0),
        m: 0.0,
        b: 0.0,
    }];
    let (mut x, mut dir, mut s, mut fm, mut fb, mut m) = (start_p, t0, 0.0, 0.0, 0.0, 0.0);
    let max_steps = ((lds_main * 2.0 + 10.0) / STEP).ceil() as usize + 20;
    let direction = |p: [f64; 2], gm: f64, gb: f64| -> Option<([f64; 2], f64, f64)> {
        let am = foot(main, p, gm)?;
        let ab = foot(branch, p, gb)?;
        let (tm, tb) = (main.pose(am).map(|q| q.t), branch.pose(ab).map(|q| q.t));
        let d = match (tm, tb) {
            (Some(tm), Some(tb)) => unit([tm[0] + tb[0], tm[1] + tb[1]]),
            _ => None,
        }?;
        Some((d, am, ab))
    };
    let mut i = 0;
    while i < max_steps && m < lds_main {
        let h = STEP.min(0.05f64.max(lds_main - m + 0.02));
        let (hd, ham, hab) = direction(add(x, dir, h / 2.0), fm, fb)?;
        let mut next = add(x, hd, h);
        for _ in 0..2 {
            let am = foot(main, next, ham)?;
            let ab = foot(branch, next, hab)?;
            let pm = main.pose(am)?.p;
            let pb = branch.pose(ab)?.p;
            let Some(u) = unit(sub(pb, pm)) else { break };
            let (dm, db) = (sub(next, pm), sub(next, pb));
            let (dm, db) = (dm[0].hypot(dm[1]), db[0].hypot(db[1]));
            if (db - dm).abs() < 1e-7 {
                break;
            }
            next = add(next, u, (db - dm) / 2.0);
        }
        let (d, am, ab) = direction(next, ham, hab)?;
        let step = sub(next, x);
        s += step[0].hypot(step[1]);
        x = next;
        dir = d;
        fm = am;
        fb = ab;
        let cm = route_cut(main, x, dir, fm)?;
        let cb = route_cut(branch, x, dir, fb)?;
        m = cm;
        samples.push(Sample {
            s,
            x: Some(x),
            dir: Some(dir),
            m,
            b: cb,
        });
        i += 1;
    }
    (m >= lds_main).then_some(samples)
}

#[derive(Clone, Copy)]
enum Key {
    S,
    M,
    B,
}

fn key_of(s: &Sample, k: Key) -> f64 {
    match k {
        Key::S => s.s,
        Key::M => s.m,
        Key::B => s.b,
    }
}

fn sample_at(samples: &[Sample], k: Key, value: f64) -> Option<Sample> {
    if value <= key_of(&samples[0], k) {
        return Some(samples[0]);
    }
    for i in 1..samples.len() {
        let (a, c) = (&samples[i - 1], &samples[i]);
        if value <= key_of(c, k) + EPS {
            let t = if key_of(c, k) - key_of(a, k) > EPS {
                (value - key_of(a, k)) / (key_of(c, k) - key_of(a, k))
            } else {
                0.0
            };
            return Some(Sample {
                s: lerp(a.s, c.s, t),
                x: None,
                dir: None,
                m: lerp(a.m, c.m, t),
                b: lerp(a.b, c.b, t),
            });
        }
    }
    None
}

/// A sleeper: its number (None for the ldS), where it lies and how it sits.
#[derive(Clone, Copy, Debug)]
pub struct Sleeper {
    pub k: Option<usize>,
    pub s: f64,
    pub m: f64,
    pub b: f64,
    pub main: f64,
    pub branch: f64,
    pub a: f64,
    pub y: f64,
}

impl Sleeper {
    pub fn to_json(self) -> Value {
        json!({
            "k": self.k.map(|k| json!(k)).unwrap_or(json!("lds")), "s": number(self.s), "m": number(self.m), "b": number(self.b),
            "main": number(self.main), "branch": number(self.branch), "a": number(self.a), "y": number(self.y),
        })
    }
}

/// The sleepers of one turnout (`turnoutSleepers`).
pub struct Frame {
    pub main: RouteFrom,
    pub branch: RouteFrom,
    pub variant: Option<Value>,
    pub we: f64,
    pub lds: f64,
    pub lds_main: f64,
    pub lds_branch: f64,
    pub lds_branch_distance: f64,
    pub we_bisector: Option<f64>,
    pub samples: Vec<Sample>,
    pub sleepers: Vec<Sleeper>,
    pub grid: Vec<Sleeper>,
}

impl Frame {
    pub fn to_json(&self) -> Value {
        json!({
            "main": self.main.to_json(), "branch": self.branch.to_json(), "variant": self.variant,
            "we": number(self.we), "lds": number(self.lds), "ldsMain": number(self.lds_main), "ldsBranch": number(self.lds_branch),
            "ldsBranchDistance": number(self.lds_branch_distance), "weBisector": self.we_bisector.map(number),
            "samples": self.samples.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
            "sleepers": self.sleepers.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
            "grid": self.grid.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
        })
    }
}

fn is_turnout(sw: &Value) -> bool {
    get(sw, "kind").and_then(Value::as_str).unwrap_or("turnout") == "turnout"
}

/// The options of `turnoutSleepers`.
pub struct SleeperOptions<'a> {
    pub form_of: FormOf<'a>,
    pub rail: Option<Value>,
    pub lds_unknown: Option<f64>,
}

pub fn turnout_sleepers(tracks: &[Value], sw: &Value, o: &SleeperOptions) -> Option<Frame> {
    if !is_turnout(sw) {
        return None;
    }
    let behind_we = switch_lds(sw, o.form_of).or(o.lds_unknown)?;
    let main = route_from(tracks, sw, turnout_line_port(sw))?;
    let branch = route_from(tracks, sw, turnout_diverging_port(sw))?;
    if strict_eq(main.track.get("id"), branch.track.get("id")) {
        return None;
    }
    if js_number(main.track.get("epsg")) != js_number(branch.track.get("epsg")) {
        return None;
    }
    let form = (o.form_of)(sw.get("label").and_then(Value::as_str));
    let profile = o
        .rail
        .clone()
        .unwrap_or_else(|| superstructure_at(&main.track, main.station(0.0))["rail"].clone());
    let variant = form.as_ref().and_then(|f| switch_variant_for(Some(f), Some(&profile)));
    build_geometry(sw, main, branch, behind_we, variant)
}

/// How far behind WE the ldS lies for the sleepers (`tableLds`).
pub fn table_lds(variant: Option<&Value>, behind_we: f64) -> f64 {
    variant
        .and_then(|v| v.get("masse"))
        .and_then(|m| m.get("lds"))
        .and_then(Value::as_f64)
        .filter(|x| x.is_finite())
        .unwrap_or(behind_we)
}

/// The sleepers from WE to the ldS (`sleepersWeToLds`).
pub fn sleepers_we_to_lds(lds: f64) -> Vec<f64> {
    if !(lds > 0.0) {
        return vec![];
    }
    let mut out = Vec::new();
    let mut j = 0;
    while FIRST_SLEEPER + SLEEPER_SPACING * (j as f64) < lds - FIRST_SLEEPER + EPS {
        out.push(FIRST_SLEEPER + SLEEPER_SPACING * j as f64);
        j += 1;
    }
    out.push(lds);
    out
}

fn sleepers_wa_to_we(variant: Option<&Value>, behind_we: f64, s_we: f64) -> Option<usize> {
    let total = variant.and_then(|v| v.get("schwellenzahl")).and_then(Value::as_f64).filter(|t| *t != 0.0)?;
    let n = total - sleepers_we_to_lds(table_lds(variant, behind_we)).len() as f64;
    let d = if n > 1.0 { (s_we - 2.0 * FIRST_SLEEPER) / (n - 1.0) } else { 0.0 };
    (d >= 0.5 && d <= 0.7).then_some(n as usize)
}

/// The grid positions along the bisector (`sleeperGrid`).
pub fn sleeper_grid(s_we: f64, s_end: f64, count: Option<usize>) -> Vec<f64> {
    let mut out = Vec::new();
    let inner = s_we - 2.0 * FIRST_SLEEPER;
    if !(inner >= SLEEPER_SPACING) {
        let mut k = 0;
        while FIRST_SLEEPER + SLEEPER_SPACING * k as f64 <= s_end + EPS {
            out.push(FIRST_SLEEPER + SLEEPER_SPACING * k as f64);
            k += 1;
        }
        return out;
    }
    let n = count.unwrap_or_else(|| rail_across_js::js_round(inner / SLEEPER_SPACING) as usize + 1);
    let d = inner / (n as f64 - 1.0);
    for k in 0..n {
        out.push(FIRST_SLEEPER + d * k as f64);
    }
    let mut j = 0;
    while s_we + FIRST_SLEEPER + SLEEPER_SPACING * j as f64 <= s_end + EPS {
        out.push(s_we + FIRST_SLEEPER + SLEEPER_SPACING * j as f64);
        j += 1;
    }
    out.into_iter().filter(|s| *s <= s_end + EPS).collect()
}

fn build_geometry(sw: &Value, main: RouteFrom, branch: RouteFrom, behind_we: f64, variant: Option<Value>) -> Option<Frame> {
    let we = main_route_length(&main.track, sw);
    if !(we > 0.0) {
        return None;
    }
    let lds = we + behind_we;
    if lds > main.length {
        return None;
    }
    let beyond = main.length.min(lds + SLEEPER_SPACING / 2.0);
    let samples = (if beyond > lds + EPS { trace_bisector(&main, &branch, beyond) } else { None }).or_else(|| trace_bisector(&main, &branch, lds))?;
    let last = sample_at(&samples, Key::M, lds)?;
    if !(last.b > 0.0) {
        return None;
    }
    let we_at = sample_at(&samples, Key::M, we);
    let at = |s: f64, k: usize| {
        sample_at(&samples, Key::S, s).map(|p| Sleeper {
            k: Some(k),
            s,
            m: p.m,
            b: p.b,
            main: main.station(p.m),
            branch: branch.station(p.b),
            a: 0.0,
            y: 0.0,
        })
    };
    let s_we = we_at.map(|w| w.s).unwrap_or(0.0);
    let mut grid: Vec<Sleeper> = sleeper_grid(s_we, last.s + SLEEPER_SPACING / 2.0, sleepers_wa_to_we(variant.as_ref(), behind_we, s_we))
        .into_iter()
        .enumerate()
        .filter_map(|(k, s)| at(s, k))
        .collect();
    let mut sleepers: Vec<Sleeper> = grid.iter().copied().filter(|sl| sl.s < last.s - EPS).collect();
    sleepers.push(Sleeper {
        k: None,
        s: last.s,
        m: lds,
        b: last.b,
        main: main.station(lds),
        branch: branch.station(last.b),
        a: 0.0,
        y: 0.0,
    });
    for sl in grid.iter_mut().chain(sleepers.iter_mut()) {
        let (a, y) = sleeper_offsets(&main, &branch, sl.main, sl.branch).unwrap_or((0.0, 0.0));
        sl.a = a;
        sl.y = y;
    }
    Some(Frame {
        lds_main: main.station(lds),
        lds_branch: branch.station(last.b),
        lds_branch_distance: last.b,
        we_bisector: we_at.map(|w| w.s),
        main,
        branch,
        variant,
        we,
        lds,
        samples,
        sleepers,
        grid,
    })
}

/// The sleeper nearest to a station of one of the turnout's tracks (`sleeperNear`).
pub fn sleeper_near(frame: &Frame, side: &str, station: f64, tol: f64) -> Option<Sleeper> {
    let (route, main) = if side == "main" { (&frame.main, true) } else { (&frame.branch, false) };
    let key = |s: &Sleeper| if main { s.m } else { s.b };
    let d = route.distance(station);
    let end = key(frame.sleepers.last()?);
    if d < tol || d > end + tol {
        return None;
    }
    let mut best: Option<Sleeper> = None;
    for sl in &frame.sleepers {
        if best.is_none_or(|b| (key(sl) - d).abs() < (key(&b) - d).abs()) {
            best = Some(*sl);
        }
    }
    best
}

/// The sleeper laid through a station (`sleeperThrough`): the sample and its two stations.
pub fn sleeper_through(frame: &Frame, side: &str, station: f64) -> Option<(Sample, f64, f64)> {
    let main = side == "main";
    let d = (if main { &frame.main } else { &frame.branch }).distance(station);
    let last = frame.sleepers.last()?;
    let end = if main { last.m } else { last.b };
    if d < -1e-6 || d > end + 1e-6 {
        return None;
    }
    let p = sample_at(&frame.samples, if main { Key::M } else { Key::B }, d)?;
    Some((p, frame.main.station(p.m), frame.branch.station(p.b)))
}

/// How a sleeper sits in the main track's frame (`sleeperOffsets`): (a, y).
pub fn sleeper_offsets(main: &RouteFrom, branch: &RouteFrom, at_main: f64, at_branch: f64) -> Option<(f64, f64)> {
    let p = point_at(&main.track, at_main)?;
    let q = point_at(&branch.track, at_branch)?;
    let t = tangent_at(&main.track, at_main, main.length)?;
    let v = sub(q, p);
    Some((dot(v, t), t[0] * v[1] - t[1] * v[0]))
}

/// A form table handed in by the caller, by label.
pub fn forms_from(v: Option<&Value>) -> Option<serde_json::Map<String, Value>> {
    v.and_then(Value::as_object).cloned()
}
