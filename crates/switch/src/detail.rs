// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A turnout drawn in detail: its sleepers and rails in its own frame (`detail.js`).

use rail_across_geom::element::{Utm, bearing_after, end_point_curved, end_point_straight};
use rail_across_geom::transition::{transition_bearing_at, transition_point_at};
use rail_across_js::{Object, get, js_hypot, number};
use rail_across_track::switch_model::element_belongs_to_switch;
use serde_json::{Value, json};

use crate::catalogue::{switch_form_chain, switch_straight_length, switch_tangents};
use crate::sleepers::{FIRST_SLEEPER, FormOf, Pose, RouteFrom, SLEEPER_SPACING, SleeperOptions, route_cut, sleepers_we_to_lds, table_lds, turnout_sleepers};

pub const RAIL_OFFSET: f64 = 0.75;
pub const SLEEPER_OVERHANG: f64 = 0.55;
const SLEEPER_HALF: f64 = RAIL_OFFSET + SLEEPER_OVERHANG;
const SLEEPER_GAP: f64 = 0.05;
const RAIL_STEP: f64 = 0.5;
const RAIL_BEYOND: f64 = 0.3;

type P = [f64; 2];

fn add(p: P, v: P, k: f64) -> P {
    [p[0] + v[0] * k, p[1] + v[1] * k]
}
fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn left(v: P) -> P {
    [-v[1], v[0]]
}
fn unit(v: P) -> Option<P> {
    let n = js_hypot(&v);
    (n > 1e-12).then(|| [v[0] / n, v[1] / n])
}
fn pj(p: P) -> Value {
    json!([number(p[0]), number(p[1])])
}

fn tangent_length(form: Option<&Value>) -> Option<f64> {
    let form = form?;
    let truthy = |k: &str| rail_across_js::truthy(form.get(k));
    if !truthy("R") || !truthy("ratio") {
        return None;
    }
    if truthy("symmetric") {
        return Some(switch_straight_length(form) / 2.0);
    }
    Some(switch_tangents(Some(form)).0)
}

fn branch_route_length(track: &Value, sw: &Value) -> f64 {
    track
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|el| element_belongs_to_switch(el, sw))
        .map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0))
        .sum()
}

const NAN_POSE: Pose = Pose {
    p: [f64::NAN, f64::NAN],
    t: [f64::NAN, f64::NAN],
};

/// One sleeper of the detail.
#[derive(Clone, Debug)]
struct Sleeper {
    kind: &'static str,
    k: i64,
    m: Option<f64>,
    b: Option<f64>,
    main: Option<f64>,
    branch: Option<f64>,
    ends: [P; 2],
}

impl Sleeper {
    fn to_json(&self) -> Value {
        let mut o = Object::new();
        o.insert("kind".into(), json!(self.kind));
        o.insert("k".into(), json!(self.k));
        for (k, v) in [("m", self.m), ("b", self.b), ("main", self.main), ("branch", self.branch)] {
            if let Some(v) = v {
                o.insert(k.into(), number(v));
            }
        }
        o.insert("ends".into(), json!([pj(self.ends[0]), pj(self.ends[1])]));
        Value::Object(o)
    }
    fn key(&self, main: bool) -> Option<f64> {
        if main { self.m } else { self.b }
    }
}

/// The detail of one turnout (`switchDetail`).
pub fn switch_detail(tracks: &[Value], sw: &Value, form_of: FormOf, rail: Option<Value>) -> Option<Value> {
    let frame = turnout_sleepers(
        tracks,
        sw,
        &SleeperOptions {
            form_of,
            rail,
            lds_unknown: None,
        },
    )?;
    let (main, branch) = (&frame.main, &frame.branch);
    let variant = frame.variant.clone();
    let form = form_of(sw.get("label").and_then(Value::as_str));
    let klammer = |i: usize| {
        variant
            .as_ref()
            .and_then(|v| v.get("klammer"))
            .and_then(|k| k.get(i))
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as i64
    };
    let (before, behind_main, behind_branch) = (klammer(0), klammer(1), klammer(2));
    let wa = main.pose(0.0)?;
    let main_pose = |d: f64| -> Option<Pose> {
        if d < 0.0 {
            Some(Pose {
                p: add(wa.p, wa.t, d),
                t: wa.t,
            })
        } else {
            main.pose(d)
        }
    };
    let lmp = main.pose(frame.lds).unwrap_or(NAN_POSE);
    let lbp = branch.pose(frame.lds_branch_distance).unwrap_or(NAN_POSE);
    let sd = dot(sub(lbp.p, lmp.p), left(lmp.t));
    let side = if sd > 0.0 {
        1.0
    } else if sd < 0.0 {
        -1.0
    } else {
        1.0
    };
    let toward = move |t: P| -> P {
        let n = left(t);
        [n[0] * side, n[1] * side]
    };
    let away = move |t: P| -> P {
        let n = toward(t);
        [-n[0], -n[1]]
    };
    let origin = wa.p;
    let x = wa.t;
    let y = toward(x);
    let local = |p: P| -> P {
        let v = sub(p, origin);
        [dot(v, x), dot(v, y)]
    };
    let reach = |n: P, normal: P| SLEEPER_HALF / 0.5f64.max(dot(n, normal).abs());
    let mut sleepers: Vec<Sleeper> = Vec::new();
    let s_we = frame.we_bisector.unwrap_or(f64::INFINITY);
    let grid = if frame.grid.is_empty() { &frame.sleepers } else { &frame.grid };
    for sl in grid.iter().filter(|s| s.k.is_some() && s.s < s_we - 1e-6) {
        let (Some(pp), Some(qq)) = (main.pose(sl.m), branch.pose(sl.b)) else {
            continue;
        };
        let n = toward(unit(add(pp.t, qq.t, 1.0)).unwrap_or(pp.t));
        sleepers.push(Sleeper {
            kind: "grid",
            k: sleepers.len() as i64,
            m: Some(sl.m),
            b: Some(sl.b),
            main: Some(sl.main),
            branch: Some(sl.branch),
            ends: [local(add(pp.p, n, -reach(n, toward(pp.t)))), local(add(qq.p, n, reach(n, toward(qq.t))))],
        });
    }
    let we_branch = branch_route_length(&branch.track, sw);
    let lds = table_lds(variant.as_ref(), frame.lds - frame.we);
    let mut lds_dir: Option<P> = None;
    for d in sleepers_we_to_lds(lds) {
        let (m, b) = (frame.we + d, we_branch + d);
        if m > main.length || b > branch.length {
            break;
        }
        let (pp, qq) = (main.pose(m), branch.pose(b));
        let n = match (pp, qq) {
            (Some(pp), Some(qq)) => unit(sub(qq.p, pp.p)),
            _ => None,
        };
        let (Some(n), Some(pp), Some(qq)) = (n, pp, qq) else { break };
        sleepers.push(Sleeper {
            kind: "grid",
            k: sleepers.len() as i64,
            m: Some(m),
            b: Some(b),
            main: Some(main.station(m)),
            branch: Some(branch.station(b)),
            ends: [local(add(pp.p, n, -reach(n, toward(pp.t)))), local(add(qq.p, n, reach(n, toward(qq.t))))],
        });
        lds_dir = Some(n);
    }
    let lds_index = sleepers.len().checked_sub(1);
    if let Some(i) = lds_index {
        sleepers[i].kind = "lds";
        if lds_dir.is_none() {
            let [e0, e1] = sleepers[i].ends;
            if let Some(v) = unit(sub(e1, e0)) {
                lds_dir = Some([v[0] * x[0] + v[1] * y[0], v[0] * x[1] + v[1] * y[1]]);
            }
        }
    }
    for j in 1..=before {
        let d = FIRST_SLEEPER - SLEEPER_SPACING * j as f64;
        let pp = main_pose(d).unwrap_or(NAN_POSE);
        sleepers.push(Sleeper {
            kind: "before",
            k: -j,
            m: Some(d),
            b: None,
            main: None,
            branch: None,
            ends: [local(add(pp.p, y, -SLEEPER_HALF)), local(add(pp.p, y, SLEEPER_HALF))],
        });
    }
    let lds_m = lds_index.and_then(|i| sleepers[i].m).unwrap_or(frame.lds);
    let lds_b = lds_index.and_then(|i| sleepers[i].b).unwrap_or(frame.lds_branch_distance);
    let behind = |sleepers: &mut Vec<Sleeper>,
                  own: &RouteFrom,
                  other: &RouteFrom,
                  start: f64,
                  other_start: f64,
                  count: i64,
                  kind: &'static str,
                  inward: &dyn Fn(P) -> P,
                  dir_at_lds: Option<P>| {
        let Some(dir) = dir_at_lds else { return };
        let p0 = own.pose(start).unwrap_or(NAN_POSE);
        let n0 = inward(p0.t);
        let turn = (n0[0] * dir[1] - n0[1] * dir[0]).atan2(dot(n0, dir));
        for j in 1..=count {
            let d = start + SLEEPER_SPACING * j as f64;
            if d > own.length {
                break;
            }
            let Some(pp) = own.pose(d) else { break };
            let normal = inward(pp.t);
            let a = turn * (1.0 - j as f64 / count as f64);
            let n = [normal[0] * a.cos() - normal[1] * a.sin(), normal[0] * a.sin() + normal[1] * a.cos()];
            let hit = route_cut(other, pp.p, left(n), other_start + SLEEPER_SPACING * j as f64);
            let q = hit.and_then(|h| other.pose(h));
            let gap = q.map(|q| js_hypot(&sub(q.p, pp.p))).unwrap_or(f64::INFINITY);
            let full = reach(n, normal);
            let inner = 0f64.max(full.min(gap / 2.0 - SLEEPER_GAP));
            let is_main = kind == "main";
            sleepers.push(Sleeper {
                kind,
                k: j,
                m: is_main.then_some(d),
                b: (!is_main).then_some(d),
                main: is_main.then(|| own.station(d)),
                branch: (!is_main).then(|| own.station(d)),
                ends: [local(add(pp.p, n, -full)), local(add(pp.p, n, inner))],
            });
        }
    };
    behind(&mut sleepers, main, branch, lds_m, lds_b, behind_main, "main", &toward, lds_dir);
    behind(
        &mut sleepers,
        branch,
        main,
        lds_b,
        lds_m,
        behind_branch,
        "branch",
        &away,
        lds_dir.map(|d| [-d[0], -d[1]]),
    );
    let rails_of = |track: &str, from: f64, to: f64, pose: &dyn Fn(f64) -> Option<Pose>, inward: &dyn Fn(P) -> P| -> [Value; 2] {
        let (mut outer, mut inner) = (Vec::new(), Vec::new());
        let n = 1usize.max(((to - from) / RAIL_STEP).ceil() as usize);
        for i in 0..=n {
            let Some(pp) = pose(from + (to - from) * i as f64 / n as f64) else { continue };
            let k = inward(pp.t);
            outer.push(pj(local(add(pp.p, k, -RAIL_OFFSET))));
            inner.push(pj(local(add(pp.p, k, RAIL_OFFSET))));
        }
        [
            json!({ "track": track, "side": "outer", "points": outer }),
            json!({ "track": track, "side": "inner", "points": inner }),
        ]
    };
    let main_from = if before > 0 {
        FIRST_SLEEPER - SLEEPER_SPACING * before as f64 - RAIL_BEYOND
    } else {
        0.0
    };
    let main_to = main.length.min(frame.we.max(lds_m) + SLEEPER_SPACING * behind_main as f64 + RAIL_BEYOND);
    let branch_to = branch.length.min(we_branch.max(lds_b) + SLEEPER_SPACING * behind_branch as f64 + RAIL_BEYOND);
    let mut rails: Vec<Value> = rails_of("main", main_from, main_to, &main_pose, &toward).into_iter().collect();
    rails.extend(rails_of("branch", 0.0, branch_to, &|d| branch.pose(d), &toward));
    let a = tangent_length(form.as_ref());
    let we_main = main.pose(frame.we).unwrap_or(NAN_POSE);
    let we_branch_pose = if we_branch > 0.0 { branch.pose(we_branch) } else { None };
    let on_rails = |pp: Pose, d: f64, main_key: bool| -> Value {
        let near: Vec<&Sleeper> = sleepers.iter().filter(|s| s.key(main_key).is_some()).collect();
        let k = |s: &Sleeper| s.key(main_key).unwrap_or(f64::NAN);
        let before = near
            .iter()
            .filter(|s| k(s) < d)
            .fold(None::<&&Sleeper>, |acc, s| if acc.is_none_or(|a| k(s) > k(a)) { Some(s) } else { acc });
        let after = near
            .iter()
            .filter(|s| k(s) > d)
            .fold(None::<&&Sleeper>, |acc, s| if acc.is_none_or(|a| k(s) < k(a)) { Some(s) } else { acc });
        let tl = [dot(pp.t, x), dot(pp.t, y)];
        Value::Array(
            [-RAIL_OFFSET, RAIL_OFFSET]
                .iter()
                .map(|off| {
                    let r = local(add(pp.p, toward(pp.t), *off));
                    let cross = |s: Option<&&Sleeper>| -> Option<f64> {
                        let s = s?;
                        let [e0, e1] = s.ends;
                        let e = sub(e1, e0);
                        let den = tl[0] * e[1] - tl[1] * e[0];
                        if den.abs() < 1e-12 {
                            return None;
                        }
                        Some(((e0[0] - r[0]) * e[1] - (e0[1] - r[1]) * e[0]) / den)
                    };
                    let (ub, ua) = (cross(before), cross(after));
                    let u = match (ub, ua) {
                        (Some(b), Some(a)) => (b + a) / 2.0,
                        (None, Some(a)) => a - FIRST_SLEEPER,
                        (Some(b), None) => b + FIRST_SLEEPER,
                        _ => 0.0,
                    };
                    pj(add(r, tl, u))
                })
                .collect(),
        )
    };
    let wm = a
        .filter(|a| *a <= main.length)
        .and_then(|a| main.pose(a))
        .map(|p| pj(local(p.p)))
        .unwrap_or(Value::Null);
    let mut we_joints = vec![on_rails(we_main, frame.we, true)];
    if let Some(wb) = we_branch_pose {
        we_joints.push(on_rails(wb, we_branch, false));
    }
    let marks = json!({
        "WA": [0, 0],
        "WM": wm,
        "WE": { "main": pj(local(we_main.p)), "branch": we_branch_pose.map(|p| pj(local(p.p))).unwrap_or(Value::Null) },
        "ldS": { "main": pj(local(main.pose(lds_m).unwrap_or(NAN_POSE).p)), "branch": pj(local(branch.pose(lds_b).unwrap_or(NAN_POSE).p)) },
        "joints": { "WA": on_rails(wa, 0.0, true), "WE": we_joints },
    });
    let counted = |kind: &str| sleepers.iter().filter(|s| s.kind == kind).count();
    Some(json!({
        "sw": sw, "form": form, "variant": variant, "origin": pj(origin), "axes": { "x": pj(x), "y": pj(y) },
        "sleepers": sleepers.iter().map(Sleeper::to_json).collect::<Vec<_>>(), "rails": rails, "marks": marks,
        "counts": { "throughSleepers": counted("grid") + counted("lds"), "before": counted("before"), "behindMain": counted("main"), "behindBranch": counted("branch") },
    }))
}

const ORIGIN: Utm = Utm {
    e: 500000.0,
    n: 5600000.0,
    zone: 25832,
};

fn chain_elements(start: Utm, bearing: f64, chain: &[Value], fields: &Object) -> (Vec<Value>, Utm, f64) {
    let mut elements = Vec::new();
    let (mut p, mut b) = (start, bearing);
    let node = |u: Utm| json!([number(u.e), number(u.n)]);
    for piece in chain {
        let length = piece.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let signed_r = piece.get("signedR").and_then(Value::as_f64);
        let (mut el, end, end_bearing);
        if let Some(r2v) = piece.get("signedREnd").filter(|e| e.as_f64() != signed_r) {
            let r2 = r2v.as_f64();
            let [e, n] = transition_point_at(p.e, p.n, b, length, signed_r, r2, false, length);
            end = Utm { e, n, zone: p.zone };
            end_bearing = transition_bearing_at(b, length, signed_r, r2, false, length);
            el = json!({ "elementType": "transition", "transitionType": "clothoid", "r1": signed_r.map(number), "r2": r2.map(number), "startNode": node(p), "endNode": node(end), "bearing": number(b), "endBearing": number(end_bearing), "length": number(length) });
        } else if let Some(r) = signed_r.filter(|r| *r != 0.0) {
            end = end_point_curved(p, b, length, r);
            end_bearing = bearing_after(b, length, Some(r));
            el = json!({ "elementType": "arc", "startNode": node(p), "endNode": node(end), "bearing": number(b), "endBearing": number(end_bearing), "radius": number(r), "length": number(length) });
        } else {
            end = end_point_straight(p, b, length);
            end_bearing = b;
            el = json!({ "elementType": "straight", "startNode": node(p), "endNode": node(end), "bearing": number(b), "length": number(length) });
        }
        for (k, v) in fields {
            el[k] = v.clone();
        }
        elements.push(el);
        p = end;
        b = end_bearing;
    }
    (elements, p, b)
}

/// The detail of a catalogue form that is not laid (`formDetail`).
pub fn form_detail(form: Option<&Value>, rail: Option<Value>, side: &str) -> Option<Value> {
    let form = form.filter(|f| f.get("art").and_then(Value::as_str) == Some("weiche"))?;
    let sw = json!({
        "switchId": "detail", "kind": "switch", "label": form.get("label"),
        "ports": { "A": { "trackId": "detail-a", "endpoint": "END" }, "B1": { "trackId": "detail-b", "endpoint": "BEGIN" }, "B2": { "trackId": "detail-m", "endpoint": "BEGIN" } },
    });
    let onward = get(form, "lds").and_then(Value::as_f64).unwrap_or(0.0) + SLEEPER_SPACING * 30.0 + 10.0;
    let mut own = Object::new();
    own.insert("switchId".into(), json!("detail"));
    let mut main_fields = own.clone();
    main_fields.insert("switchRoute".into(), json!("main"));
    let mut branch_fields = own;
    branch_fields.insert("switchRoute".into(), json!("branch"));
    let main_chain: Vec<Value> = if rail_across_js::truthy(form.get("symmetric")) {
        switch_form_chain(Some(form), if side == "left" { "right" } else { "left" })
    } else {
        vec![json!({ "length": number(switch_straight_length(form)), "signedR": Value::Null })]
    };
    let none = Object::new();
    let (m, m_end, m_b) = chain_elements(ORIGIN, 90.0, &main_chain, &main_fields);
    let (m_on, _, _) = chain_elements(m_end, m_b, &[json!({ "length": number(onward), "signedR": Value::Null })], &none);
    let (b, b_end, b_b) = chain_elements(ORIGIN, 90.0, &switch_form_chain(Some(form), side), &branch_fields);
    let (b_on, _, _) = chain_elements(b_end, b_b, &[json!({ "length": number(onward), "signedR": Value::Null })], &none);
    let tracks = vec![
        json!({ "id": "detail-m", "epsg": 25832, "elements": m.into_iter().chain(m_on).collect::<Vec<_>>() }),
        json!({ "id": "detail-b", "epsg": 25832, "elements": b.into_iter().chain(b_on).collect::<Vec<_>>() }),
    ];
    let label = form.get("label").and_then(Value::as_str).map(str::to_string);
    let own_form = form.clone();
    let lookup = move |l: Option<&str>| {
        if l.is_some() && l.map(str::to_string) == label {
            Some(own_form.clone())
        } else {
            None
        }
    };
    switch_detail(&tracks, &sw, &lookup, rail)
}
