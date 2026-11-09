// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A turnout in detail with what lies around it (`detailScene.js`).

use std::collections::HashSet;

use rail_across_js::{get, js_number, js_string, number, strict_eq};
use rail_across_track::platforms::{element_stations, point_on_element};
use rail_across_track::switch_model::{is_link_switch, port_end, port_track, ports_of, turnout_diverging_port};
use serde_json::{Value, json};

use crate::detail::{RAIL_OFFSET, switch_detail};
use crate::sleepers::catalogue_form;

const MARGIN: f64 = 15.0;
const TRACK_STEP: f64 = 1.0;
const MAX_HOPS: usize = 4;
const MAX_REACH: f64 = 250.0;

type P = [f64; 2];

fn is_turnout(sw: &Value) -> bool {
    get(sw, "kind").and_then(Value::as_str).unwrap_or("turnout") == "turnout"
}

fn other(end: Option<&str>) -> &'static str {
    if end == Some("END") { "BEGIN" } else { "END" }
}

fn switches_at<'a>(switches: &'a [Value], track_id: &Value, end: &str) -> Vec<&'a Value> {
    switches
        .iter()
        .filter(|s| {
            ports_of(s)
                .iter()
                .any(|(p, _)| strict_eq(port_track(s, p), Some(track_id)) && port_end(s, p) == Some(end))
        })
        .collect()
}

fn along_branch<'a>(length_of: &dyn Fn(&Value) -> f64, switches: &'a [Value], sw: &Value) -> Vec<&'a Value> {
    let port = turnout_diverging_port(sw);
    let mut track_id = port_track(sw, port).cloned();
    let mut end = other(port_end(sw, port));
    let mut found = Vec::new();
    let mut seen: HashSet<String> = HashSet::from([js_string(sw.get("switchId"))]);
    let mut run = 0.0;
    let mut hop = 0;
    while let Some(tid) = track_id.clone().filter(|t| rail_across_js::truthy(Some(t))) {
        if hop > MAX_HOPS {
            break;
        }
        run += length_of(&tid);
        if run > MAX_REACH {
            break;
        }
        let there: Vec<&Value> = switches_at(switches, &tid, end)
            .into_iter()
            .filter(|s| !seen.contains(&js_string(s.get("switchId"))))
            .collect();
        if let Some(t) = there.iter().find(|s| is_turnout(s)) {
            found.push(*t);
            break;
        }
        let Some(link) = there.iter().find(|s| is_link_switch(s)) else { break };
        seen.insert(js_string(link.get("switchId")));
        let next = ports_of(link)
            .into_iter()
            .map(|(p, _)| (port_track(link, p).cloned(), port_end(link, p)))
            .find(|(t, e)| !(strict_eq(t.as_ref(), Some(&tid)) && *e == Some(end)));
        let Some((Some(nt), ne)) = next else { break };
        track_id = Some(nt);
        end = other(ne);
        hop += 1;
    }
    found
}

/// The turnouts a turnout is joined to by a branch (`connectionPartners`).
pub fn connection_partners<'a>(tracks: &[Value], switches: &'a [Value], sw: &Value) -> Vec<&'a Value> {
    let length_of = |id: &Value| -> f64 {
        tracks
            .iter()
            .find(|t| js_string(t.get("id")) == js_string(Some(id)))
            .map(|t| {
                t.get("elements")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0))
                    .sum()
            })
            .unwrap_or(f64::INFINITY)
    };
    let mut out = along_branch(&length_of, switches, sw);
    for s in switches {
        if !strict_eq(s.get("switchId"), sw.get("switchId"))
            && is_turnout(s)
            && !out.iter().any(|o| std::ptr::eq(*o, s))
            && along_branch(&length_of, switches, s)
                .iter()
                .any(|p| strict_eq(p.get("switchId"), sw.get("switchId")))
        {
            out.push(s);
        }
    }
    out
}

fn pt(v: &Value) -> P {
    [
        v.get(0).and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ]
}
fn pj(p: P) -> Value {
    json!([number(p[0]), number(p[1])])
}

struct FrameOf {
    origin: P,
    x: P,
    y: P,
}

fn frame_of(d: &Value) -> FrameOf {
    FrameOf {
        origin: pt(&d["origin"]),
        x: pt(&d["axes"]["x"]),
        y: pt(&d["axes"]["y"]),
    }
}

fn reframe(from: &FrameOf, to: &FrameOf, q: P) -> P {
    let p = [
        from.origin[0] + q[0] * from.x[0] + q[1] * from.y[0],
        from.origin[1] + q[0] * from.x[1] + q[1] * from.y[1],
    ];
    let v = [p[0] - to.origin[0], p[1] - to.origin[1]];
    [v[0] * to.x[0] + v[1] * to.x[1], v[0] * to.y[0] + v[1] * to.y[1]]
}

fn in_frame(detail: &Value, frame: &FrameOf) -> Value {
    let from = frame_of(detail);
    let f = |v: &Value| pj(reframe(&from, frame, pt(v)));
    let pair = |v: &Value| if v.is_null() { Value::Null } else { f(v) };
    let mut out = detail.clone();
    out["sleepers"] = Value::Array(
        detail["sleepers"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| {
                let mut s2 = s.clone();
                s2["ends"] = Value::Array(s["ends"].as_array().into_iter().flatten().map(&f).collect());
                s2
            })
            .collect(),
    );
    out["rails"] = Value::Array(
        detail["rails"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| {
                let mut r2 = r.clone();
                r2["points"] = Value::Array(r["points"].as_array().into_iter().flatten().map(&f).collect());
                r2
            })
            .collect(),
    );
    let m = &detail["marks"];
    out["marks"] = json!({
        "WA": f(&m["WA"]), "WM": pair(&m["WM"]),
        "WE": { "main": pair(&m["WE"]["main"]), "branch": pair(&m["WE"]["branch"]) },
        "ldS": { "main": pair(&m["ldS"]["main"]), "branch": pair(&m["ldS"]["branch"]) },
        "joints": {
            "WA": m["joints"]["WA"].as_array().into_iter().flatten().map(&f).collect::<Vec<_>>(),
            "WE": m["joints"]["WE"].as_array().into_iter().flatten().map(|p| p.as_array().into_iter().flatten().map(&f).collect::<Vec<_>>()).collect::<Vec<_>>(),
        },
    });
    out
}

#[derive(Clone, Copy)]
struct Box2 {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

impl Box2 {
    fn to_json(self) -> Value {
        json!({ "x0": number(self.x0), "x1": number(self.x1), "y0": number(self.y0), "y1": number(self.y1) })
    }
}

fn box_of(d: &Value) -> Box2 {
    let mut pts: Vec<P> = d["sleepers"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|s| s["ends"].as_array().cloned().unwrap_or_default())
        .map(|p| pt(&p))
        .collect();
    pts.extend(
        d["rails"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|r| r["points"].as_array().cloned().unwrap_or_default())
            .map(|p| pt(&p)),
    );
    let fold = |i: usize, max: bool| {
        pts.iter().map(|p| p[i]).fold(
            if max { f64::NEG_INFINITY } else { f64::INFINITY },
            |a, b| if max { a.max(b) } else { a.min(b) },
        )
    };
    Box2 {
        x0: fold(0, false),
        x1: fold(0, true),
        y0: fold(1, false),
        y1: fold(1, true),
    }
}

fn union(a: Box2, b: Box2) -> Box2 {
    Box2 {
        x0: a.x0.min(b.x0),
        x1: a.x1.max(b.x1),
        y0: a.y0.min(b.y0),
        y1: a.y1.max(b.y1),
    }
}
fn grow(b: Box2, d: f64) -> Box2 {
    Box2 {
        x0: b.x0 - d,
        x1: b.x1 + d,
        y0: b.y0 - d,
        y1: b.y1 + d,
    }
}
fn inside(b: Box2, p: P) -> bool {
    p[0] >= b.x0 && p[0] <= b.x1 && p[1] >= b.y0 && p[1] <= b.y1
}
fn meets(a: Box2, b: Box2) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

/// The scene of one laid turnout (`switchDetailScene`).
pub fn switch_detail_scene(tracks: &[Value], switches: &[Value], sw: &Value, margin: Option<f64>) -> Option<Value> {
    let margin = margin.unwrap_or(MARGIN);
    let detail = switch_detail(tracks, sw, &catalogue_form, None)?;
    let frame = frame_of(&detail);
    let to_local = |p: P| -> P {
        let v = [p[0] - frame.origin[0], p[1] - frame.origin[1]];
        [v[0] * frame.x[0] + v[1] * frame.x[1], v[0] * frame.y[0] + v[1] * frame.y[1]]
    };
    let partner_sws = connection_partners(tracks, switches, sw);
    let partners: Vec<Value> = partner_sws
        .iter()
        .filter_map(|p| switch_detail(tracks, p, &catalogue_form, None))
        .map(|d| in_frame(&d, &frame))
        .collect();
    let focus = std::iter::once(&detail).chain(partners.iter()).map(box_of).reduce(union)?;
    let bx = grow(focus, margin);
    let node = |s: &Value, port: &str| -> Option<P> {
        let id = port_track(s, port)?;
        let t = tracks.iter().find(|t| js_string(t.get("id")) == js_string(Some(id)))?;
        let els = t.get("elements").and_then(Value::as_array)?;
        let at_end = port_end(s, port) == Some("END");
        let el = if at_end { els.last() } else { els.first() }?;
        let n = el.get(if at_end { "endNode" } else { "startNode" }).filter(|v| !v.is_null())?;
        Some(to_local(pt(n)))
    };
    let mut taken: HashSet<String> = HashSet::from([js_string(sw.get("switchId"))]);
    taken.extend(partners.iter().map(|p| js_string(p["sw"].get("switchId"))));
    let reach = grow(bx, 120.0);
    let nearby: Vec<Value> = switches
        .iter()
        .filter(|s| is_turnout(s) && !taken.contains(&js_string(s.get("switchId"))))
        .filter(|s| ports_of(s).iter().any(|(p, _)| node(s, p).is_some_and(|q| inside(reach, q))))
        .filter_map(|s| switch_detail(tracks, s, &catalogue_form, None))
        .map(|d| in_frame(&d, &frame))
        .filter(|d| meets(box_of(d), bx))
        .collect();
    let mut runs: Vec<Value> = Vec::new();
    for t in tracks {
        let els = t.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
        let nodes: Vec<P> = els
            .iter()
            .flat_map(|el| [el.get("startNode"), el.get("endNode")])
            .flatten()
            .filter(|v| !v.is_null())
            .map(|v| to_local(pt(v)))
            .collect();
        if nodes.is_empty() {
            continue;
        }
        let tb = nodes.iter().fold(
            Box2 {
                x0: f64::INFINITY,
                x1: f64::NEG_INFINITY,
                y0: f64::INFINITY,
                y1: f64::NEG_INFINITY,
            },
            |b, p| {
                union(
                    b,
                    Box2 {
                        x0: p[0],
                        x1: p[0],
                        y0: p[1],
                        y1: p[1],
                    },
                )
            },
        );
        if !meets(grow(tb, 5.0), bx) {
            continue;
        }
        let epsg = js_number(t.get("epsg")) as i64;
        let mut run: Option<(Vec<Value>, Vec<Value>)> = None;
        let close = |run: &mut Option<(Vec<Value>, Vec<Value>)>, runs: &mut Vec<Value>| {
            if let Some((a, b)) = run.take()
                && a.len() > 1
            {
                runs.push(json!({ "trackId": t.get("id"), "name": get(t, "name").cloned().unwrap_or(Value::Null), "rails": [a, b] }));
            }
        };
        for (idx, _, _) in element_stations(t) {
            let el = &els[idx];
            let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
            if let (Some(s), Some(e)) = (el.get("startNode").filter(|v| !v.is_null()), el.get("endNode").filter(|v| !v.is_null())) {
                let (a, c) = (to_local(pt(s)), to_local(pt(e)));
                let eb = Box2 {
                    x0: a[0].min(c[0]),
                    x1: a[0].max(c[0]),
                    y0: a[1].min(c[1]),
                    y1: a[1].max(c[1]),
                };
                if !meets(grow(eb, len / 5.0 + 2.0), bx) {
                    close(&mut run, &mut runs);
                    continue;
                }
            }
            let n = 1usize.max((len / TRACK_STEP).ceil() as usize);
            for i in 0..=n {
                let Some((u, bearing)) = point_on_element(el, epsg, len * i as f64 / n as f64) else {
                    continue;
                };
                let c = to_local([u.e, u.n]);
                if !inside(bx, c) {
                    close(&mut run, &mut runs);
                    continue;
                }
                let b = bearing * std::f64::consts::PI / 180.0;
                let nl = to_local([u.e - b.cos(), u.n + b.sin()]);
                let dn = [nl[0] - c[0], nl[1] - c[1]];
                let r = run.get_or_insert_with(|| (Vec::new(), Vec::new()));
                r.0.push(pj([c[0] + dn[0] * RAIL_OFFSET, c[1] + dn[1] * RAIL_OFFSET]));
                r.1.push(pj([c[0] - dn[0] * RAIL_OFFSET, c[1] - dn[1] * RAIL_OFFSET]));
            }
        }
        close(&mut run, &mut runs);
    }
    Some(json!({ "detail": detail, "partners": partners, "nearby": nearby, "tracks": runs, "box": bx.to_json(), "focus": focus.to_json() }))
}
