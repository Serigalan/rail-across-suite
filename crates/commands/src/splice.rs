// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Splicing two tracks into one: what a picked element offers, the request,
//! and the commit around the chain the optimizer answers with (`splice.js`).

use rail_across_geom::coords::transform_plane_point;
use rail_across_geom::element::{Utm, node_utm, resolve_end_bearing, reverse_element};
use rail_across_geom::kind::{is_arc, is_transition};
use rail_across_geom::reconstruct::reconstruct_elements;
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_js::{Object, generate_id, get, js_number, js_round, number, strict_eq, truthy};
use rail_across_rules::cant::{auto_cant_model, cant_sign};
use rail_across_rules::element_length::min_element_length;
use rail_across_rules::length_formula::length_rule_formula;
use rail_across_track::cross_section::section_at_station;
use rail_across_track::heights::{reverse_heights, split_heights, track_length};
use rail_across_track::model::{rebuild_coords, track_label};
use rail_across_track::platforms::{element_stations, point_on_element};
use serde_json::{Value, json};

use crate::scurve::run_beside;

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn length_of(es: &[Value]) -> f64 {
    es.iter().map(|e| f(e, "length").unwrap_or(0.0)).sum()
}

fn coords(el: &Value) -> (Option<&Value>, Option<&Value>) {
    let c = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(Value::as_array);
    (c.and_then(|c| c.first()), c.and_then(|c| c.last()))
}

/// The runs of an element's own curvature beside it (`pickRuns`).
pub fn pick_runs(elements: &[Value], i: usize) -> (f64, f64) {
    (
        run_beside(elements, i, "before").unwrap_or(0.0),
        run_beside(elements, i, "after").unwrap_or(0.0),
    )
}

/// What the splice needs of a picked element (`splicePick`).
pub fn splice_pick(track: &Value, el_idx: usize) -> Value {
    let es = els(track);
    let Some(el) = es.get(el_idx) else { return Value::Null };
    let last = el_idx == es.len() - 1;
    let transition = is_transition(el);
    if transition && !last && el_idx != 0 {
        return json!({ "error": "splice_hint_transition_end" });
    }
    let epsg = js_number(track.get("epsg")) as i64;
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    };
    let (c0, c1) = coords(el);
    let mut base = json!({
        "trackId": track.get("id"), "elIdx": el_idx, "epsg": track.get("epsg"), "label": track_label(Some(track)),
        "cant": get(el, "cant").cloned().unwrap_or(json!(0)), "speed": get(el, "speed").cloned().unwrap_or(json!(0)),
    });
    if transition {
        let side = if last { "end" } else { "start" };
        let at = if side == "end" {
            node_utm(el.get("endNode"), c1, epsg)
        } else {
            node_utm(el.get("startNode"), c0, epsg)
        }
        .unwrap_or(nan);
        let total = length_of(es);
        for (k, v) in [
            ("virtual", json!(side)),
            ("joinAt", json!(side)),
            ("startUtm", at.to_json()),
            ("endUtm", at.to_json()),
            (
                "bearing",
                number(if side == "end" {
                    resolve_end_bearing(el, epsg)
                } else {
                    js_number(el.get("bearing"))
                }),
            ),
            ("signedR", get(el, if side == "end" { "r2" } else { "r1" }).cloned().unwrap_or(Value::Null)),
            ("length", json!(0)),
            ("before", number(if side == "end" { total } else { 0.0 })),
            ("after", number(if side == "end" { 0.0 } else { total })),
        ] {
            base[k] = v;
        }
        return base;
    }
    let (rb, ra) = pick_runs(es, el_idx);
    for (k, v) in [
        ("endUtm", node_utm(el.get("endNode"), c1, epsg).unwrap_or(nan).to_json()),
        ("startUtm", node_utm(el.get("startNode"), c0, epsg).unwrap_or(nan).to_json()),
        ("bearing", number(resolve_end_bearing(el, epsg))),
        ("signedR", get(el, "radius").cloned().unwrap_or(Value::Null)),
        ("length", get(el, "length").cloned().unwrap_or(json!(0))),
        ("before", number(length_of(&es[..el_idx]))),
        ("after", number(length_of(&es[el_idx + 1..]))),
        ("runBefore", number(rb)),
        ("runAfter", number(ra)),
    ] {
        base[k] = v;
    }
    base
}

fn toward_start(track: &Value, pick: &Value) -> Vec<Value> {
    let idx = pick["elIdx"].as_u64().unwrap_or(0) as usize + usize::from(truthy(pick.get("virtual")));
    els(track)[..idx.min(els(track).len())].to_vec()
}

fn toward_end(track: &Value, pick: &Value) -> Vec<Value> {
    let idx = pick["elIdx"].as_u64().unwrap_or(0) as usize + usize::from(!truthy(pick.get("virtual")));
    els(track).get(idx..).map(<[Value]>::to_vec).unwrap_or_default()
}

/// May `pick` be the second of a splice (`secondPickRefusal`)?
pub fn second_pick_refusal(first: &Value, pick: &Value) -> Option<&'static str> {
    if strict_eq(first.get("trackId"), pick.get("trackId")) && strict_eq(first.get("elIdx"), pick.get("elIdx")) {
        return Some("same");
    }
    if strict_eq(first.get("trackId"), pick.get("trackId")) {
        return Some("splice_error_same_track");
    }
    if js_number(first.get("epsg")) != js_number(pick.get("epsg")) {
        return Some("splice_error_crs");
    }
    None
}

/// The picks at the dialog's speed (`picksAtSpeed`).
pub fn picks_at_speed(picks: &[Value], speed: Option<&Value>) -> Vec<Value> {
    let v = js_number(speed);
    let v = if v.is_nan() { 0.0 } else { v };
    picks
        .iter()
        .map(|p| {
            let ps = f(p, "speed").unwrap_or(0.0);
            if v > 0.0 && ps > v {
                let mut q = p.clone();
                q["speedBefore"] = p.get("speed").cloned().unwrap_or(Value::Null);
                q["speed"] = number(v);
                q
            } else {
                p.clone()
            }
        })
        .collect()
}

/// The cant proposed for an inserted arc (`spliceKeptCant`).
pub fn splice_kept_cant(picks: &[Value], transition_on: Option<&Value>) -> Option<f64> {
    let k = [0usize, 1].into_iter().find(|i| !truthy(transition_on.and_then(|t| t.get(*i))))?;
    Some(picks.get(k).and_then(|p| f(p, "cant")).unwrap_or(0.0).abs())
}

fn num_or_zero(v: Option<&Value>) -> f64 {
    let x = js_number(v);
    if x.is_nan() || x == 0.0 { 0.0 } else { x }
}

/// The request for the optimizer module's splice (`spliceRequest`).
pub fn splice_request(first: &Value, second: &Value, s: &Value, clearance: Option<&Value>) -> Value {
    let ends = |p: &Value| -> Value {
        let mut o = json!({
            "start": [p["startUtm"].get("easting"), p["startUtm"].get("northing")],
            "end": [p["endUtm"].get("easting"), p["endUtm"].get("northing")],
            "bearing": p.get("bearing"), "radius": p.get("signedR"),
            "cant": number(f(p, "cant").unwrap_or(0.0).abs()), "speed": get(p, "speed").cloned().unwrap_or(json!(0)),
            "length": get(p, "length").cloned().unwrap_or(json!(0)), "before": get(p, "before").cloned().unwrap_or(json!(0)), "after": get(p, "after").cloned().unwrap_or(json!(0)),
        });
        if f(p, "runBefore").is_some_and(|r| r > 0.0) {
            o["runBefore"] = p["runBefore"].clone();
        }
        if f(p, "runAfter").is_some_and(|r| r > 0.0) {
            o["runAfter"] = p["runAfter"].clone();
        }
        if truthy(p.get("joinAt")) {
            o["joinAt"] = p["joinAt"].clone();
        }
        o
    };
    let on = |i: usize| truthy(s.get("transitionOn").and_then(|t| t.get(i)));
    let arr_at = |k: &str, i: usize| s.get(k).and_then(|a| a.get(i)).cloned();
    let or = |k: &str, d: &str| get(s, k).cloned().unwrap_or(json!(d));
    let radius = js_number(s.get("radius")).abs();
    let mut out = json!({
        "dep": ends(first), "arr": ends(second),
        "radius": number(if radius.is_nan() || radius == 0.0 { 0.0 } else { radius }),
        "speed": number(num_or_zero(s.get("speed"))),
        "cant": number(num_or_zero(s.get("cant")).abs()),
        "lDep": number(if on(0) { num_or_zero(arr_at("transitions", 0).as_ref()) } else { 0.0 }),
        "lArr": number(if on(1) { num_or_zero(arr_at("transitions", 1).as_ref()) } else { 0.0 }),
        "modeDep": if on(0) { arr_at("modes", 0).unwrap_or(json!("fixed")) } else { json!("fixed") },
        "modeArr": if on(1) { arr_at("modes", 1).unwrap_or(json!("fixed")) } else { json!("fixed") },
        "transition": or("transitionType", "clothoid"),
        "transitionDep": arr_at("transitionTypes", 0).filter(|v| !v.is_null()).unwrap_or(json!("clothoid")),
        "transitionArr": arr_at("transitionTypes", 1).filter(|v| !v.is_null()).unwrap_or(json!("clothoid")),
        "arcJoin": or("arcJoin", "straight"),
    });
    if let Some(c) = clearance.filter(|c| truthy(Some(c))) {
        out["clearance"] = c.clone();
    }
    out
}

const AXIS_STEP: f64 = 1.0;
const AXIS_MARGIN: f64 = 150.0;

/// The axis of a neighbour near some points (`neighbourAxisNear`).
pub fn neighbour_axis_near(track: &Value, epsg: f64, corners: &[Utm]) -> Vec<Value> {
    let e0 = corners.iter().map(|p| p.e).fold(f64::INFINITY, f64::min) - AXIS_MARGIN;
    let e1 = corners.iter().map(|p| p.e).fold(f64::NEG_INFINITY, f64::max) + AXIS_MARGIN;
    let n0 = corners.iter().map(|p| p.n).fold(f64::INFINITY, f64::min) - AXIS_MARGIN;
    let n1 = corners.iter().map(|p| p.n).fold(f64::NEG_INFINITY, f64::max) + AXIS_MARGIN;
    let te = js_number(track.get("epsg"));
    let es = els(track);
    let mut out = Vec::new();
    for (k, (idx, start, _)) in element_stations(track).into_iter().enumerate() {
        let el = &es[idx];
        let len = f(el, "length").unwrap_or(0.0);
        let n = 1usize.max((len / AXIS_STEP).ceil() as usize);
        for i in (if k == 0 { 0 } else { 1 })..=n {
            let s = len * i as f64 / n as f64;
            let Some((u, _)) = point_on_element(el, te as i64, s) else { continue };
            let (e, no) = if te == epsg {
                (u.e, u.n)
            } else {
                transform_plane_point(u.e, u.n, te as i64, epsg as i64).unwrap_or((f64::NAN, f64::NAN))
            };
            if e < e0 || e > e1 || no < n0 || no > n1 {
                continue;
            }
            let cant = section_at_station(track, start + s).get("cant").and_then(Value::as_f64).unwrap_or(0.0);
            out.push(json!([
                number(js_round(e * 1000.0) / 1000.0),
                number(js_round(no * 1000.0) / 1000.0),
                number(js_round(cant * 10.0) / 10.0)
            ]));
        }
    }
    out
}

/// The axis near the two picks (`neighbourAxis`).
pub fn neighbour_axis(track: &Value, epsg: f64, dep: &Value, arr: &Value) -> Vec<Value> {
    let u = |p: &Value, k: &str| Utm::from_json(&p[k]);
    neighbour_axis_near(track, epsg, &[u(dep, "startUtm"), u(dep, "endUtm"), u(arr, "startUtm"), u(arr, "endUtm")])
}

/// The spacing a splice is held to (`clearanceRequest`).
pub fn clearance_request(o: &Value) -> Value {
    let speed = num_or_zero(o.get("speed"));
    json!({
        "ref": o.get("axis"), "dMin": number(js_number(o.get("dMin"))), "profile": o.get("profile"),
        "cant": number(num_or_zero(o.get("cant")).abs()), "maximize": truthy(o.get("maximize")), "speed": number(speed),
        "cantModel": auto_cant_model(), "lMin": number(min_element_length(js_number(o.get("speed"))).unwrap_or(0.0)),
    })
}

fn with_formulas(l: &Value, form: &str) -> Value {
    let mut o = l.clone();
    for (key, worst) in [("regularBy", "ok"), ("minimumBy", "warning")] {
        let list: Vec<Value> = l
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|b| {
                let mut x = b.clone();
                x["formula"] = b["id"]
                    .as_str()
                    .and_then(|id| length_rule_formula(id, worst, Some(form)))
                    .map(Value::String)
                    .unwrap_or(Value::Null);
                x
            })
            .collect();
        o[key] = Value::Array(list);
    }
    o
}

fn request_forms(request: Option<&Value>) -> [String; 2] {
    let pick = |k: &str| -> String {
        request
            .and_then(|r| r.get(k).filter(|v| truthy(Some(v))).or_else(|| r.get("transition").filter(|v| truthy(Some(v)))))
            .and_then(Value::as_str)
            .unwrap_or("clothoid")
            .to_string()
    };
    [pick("transitionDep"), pick("transitionArr")]
}

/// The optimizer module's answer as the dialog and the commit read it (`spliceFromAnswer`).
pub fn splice_from_answer(answer: &Value, picks: &[Value], request: Option<&Value>) -> Value {
    let forms = request_forms(request);
    let formulas = |list: Option<&Value>| -> Value {
        Value::Array(
            list.and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(k, l)| with_formulas(l, &forms[k.min(1)]))
                .collect(),
        )
    };
    let sols = answer.get("solutions").and_then(Value::as_array).filter(|s| !s.is_empty());
    if answer.is_null() || truthy(answer.get("error")) || sols.is_none() {
        return json!({
            "error": get(answer, "error").cloned().unwrap_or(json!("splice_error_parallel")),
            "params": get(answer, "params").cloned().unwrap_or(json!({})),
            "lengths": if truthy(answer.get("lengths")) { formulas(answer.get("lengths")) } else { Value::Null },
        });
    }
    let epsg_v = picks[0].get("epsg").cloned().unwrap_or(Value::Null);
    let epsg = js_number(Some(&epsg_v)) as i64;
    let solutions: Vec<Value> = sols
        .into_iter()
        .flatten()
        .map(|sol| {
            let input: Vec<Value> = sol["elements"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|el| {
                    let mut e = el.clone();
                    e["epsg"] = epsg_v.clone();
                    e
                })
                .collect();
            let elements = reconstruct_elements(&input, epsg);
            let mut preview: Vec<Value> = Vec::new();
            for (i, el) in elements.iter().enumerate() {
                let c = el
                    .get("renderCoords")
                    .filter(|v| !v.is_null())
                    .or_else(|| el.get("geometry").and_then(|g| g.get("coordinates")))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if i == 0 {
                    preview.extend(c);
                } else {
                    preview.extend(c.into_iter().skip(1));
                }
            }
            let dep_pick = sol["depPick"].as_u64().unwrap_or(0) as usize;
            let mut result = sol.get("info").and_then(Value::as_object).cloned().unwrap_or_default();
            let set = |r: &mut Object, k: &str, v: Value| {
                r.insert(k.into(), v);
            };
            set(&mut result, "elements", Value::Array(elements));
            set(&mut result, "previewCoords", Value::Array(preview));
            set(&mut result, "depPick", sol["depPick"].clone());
            set(&mut result, "reverseDep", json!(truthy(sol.get("reverseDep"))));
            set(&mut result, "reverseArr", json!(truthy(sol.get("reverseArr"))));
            if let Some(e) = sol.get("ends") {
                set(&mut result, "ends", e.clone());
            }
            if let Some(r) = sol.get("rebuilt") {
                set(&mut result, "rebuilt", r.clone());
            }
            set(&mut result, "findings", get(sol, "findings").cloned().unwrap_or(json!([])));
            set(&mut result, "worst", get(sol, "worst").cloned().unwrap_or(json!("ok")));
            set(&mut result, "judged", json!(truthy(sol.get("judged"))));
            set(&mut result, "arcJoin", get(sol, "arcJoin").cloned().unwrap_or(Value::Null));
            set(&mut result, "alternative", json!(truthy(sol.get("alternative"))));
            set(&mut result, "lengths", formulas(sol.get("lengths")));
            json!({ "dep": picks.get(dep_pick), "arr": picks.get(1 - dep_pick.min(1)), "result": Value::Object(result) })
        })
        .collect();
    json!({ "requested": get(answer, "requested").cloned().unwrap_or(Value::Null), "solutions": solutions })
}

/// The Regellänge set once (`settleLengths`).
pub fn settle_lengths(s: &Value, lengths: Option<&Value>) -> Value {
    let Some(lengths) = lengths.filter(|l| truthy(Some(l))) else {
        return s.clone();
    };
    let on = |k: usize| truthy(s.get("transitionOn").and_then(|t| t.get(k)));
    let due: Vec<usize> = [0usize, 1]
        .into_iter()
        .filter(|k| on(*k) && s["modes"].get(*k).and_then(Value::as_str) == Some("regular") && lengths.get(*k).and_then(|l| get(l, "regular")).is_some())
        .collect();
    if due.is_empty() {
        return s.clone();
    }
    let mut out = s.clone();
    out["transitions"] = Value::Array(
        s["transitions"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(k, v)| if due.contains(&k) { lengths[k]["regular"].clone() } else { v.clone() })
            .collect(),
    );
    out["modes"] = Value::Array(
        s["modes"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(k, m)| if due.contains(&k) { json!("fixed") } else { m.clone() })
            .collect(),
    );
    out
}

fn same_node(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a.and_then(Value::as_array), b.and_then(Value::as_array)) {
        (Some(a), Some(b)) => {
            let g = |v: &[Value], i: usize| v.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
            (g(a, 0) - g(b, 0)).abs() < NODE_TOL && (g(a, 1) - g(b, 1)).abs() < NODE_TOL
        }
        _ => false,
    }
}

fn overlap_of(piece: Option<&Value>, orig: &Value, node: Option<&Value>, piece_node: &str) -> f64 {
    let Some(piece) = piece else { return 0.0 };
    if !same_node(piece.get(piece_node), node) {
        return 0.0;
    }
    if (f(piece, "radius").unwrap_or(0.0).abs() - f(orig, "radius").unwrap_or(0.0).abs()).abs() > 1e-6 {
        return 0.0;
    }
    f(piece, "length").unwrap_or(0.0).min(f(orig, "length").unwrap_or(0.0))
}

fn plain_point(p: &Value) -> Value {
    let mut o = p.as_object().cloned().unwrap_or_default();
    o.remove("rv");
    o.remove("la");
    Value::Object(o)
}

struct Kept {
    dep: f64,
    arr: f64,
}

fn kept_lengths(dep_track: &Value, arr_track: &Value, dep: &Value, arr: &Value, chain: &[Value], reverse_dep: bool, reverse_arr: bool) -> Kept {
    let idx = |p: &Value| p["elIdx"].as_u64().unwrap_or(0) as usize;
    let dep_orig = &els(dep_track)[idx(dep)];
    let arr_orig = &els(arr_track)[idx(arr)];
    let dep_piece = chain.iter().find(|e| e.get("role").and_then(Value::as_str) == Some("dep"));
    let arr_piece = chain.iter().rev().find(|e| e.get("role").and_then(Value::as_str) == Some("arr"));
    let dep_rest = if reverse_dep {
        toward_end(dep_track, dep)
    } else {
        toward_start(dep_track, dep)
    };
    let dep_keep = length_of(&dep_rest)
        + if truthy(dep.get("virtual")) {
            0.0
        } else {
            overlap_of(
                dep_piece,
                dep_orig,
                dep_orig.get(if reverse_dep { "endNode" } else { "startNode" }),
                "startNode",
            )
        };
    let arr_rest = if reverse_arr {
        toward_start(arr_track, arr)
    } else {
        toward_end(arr_track, arr)
    };
    let arr_keep = length_of(&arr_rest)
        + if truthy(arr.get("virtual")) {
            0.0
        } else {
            overlap_of(arr_piece, arr_orig, arr_orig.get(if reverse_arr { "startNode" } else { "endNode" }), "endNode")
        };
    Kept { dep: dep_keep, arr: arr_keep }
}

fn heights_of(t: &Value) -> Option<&[Value]> {
    get(t, "heights").and_then(Value::as_array).map(Vec::as_slice)
}

fn splice_heights(dep_track: &Value, arr_track: &Value, kept: &Kept, reverse_dep: bool, reverse_arr: bool, merged_length: f64) -> Option<Vec<Value>> {
    let offset = merged_length - kept.arr;
    let dep_heights: Option<Vec<Value>> = if reverse_dep {
        reverse_heights(dep_track.get("heights"), track_length(dep_track)).and_then(|v| v.as_array().cloned())
    } else {
        heights_of(dep_track).map(<[Value]>::to_vec)
    };
    let dep_h = split_heights(dep_heights.as_deref(), kept.dep).0;
    let arr_h: Option<Vec<Value>> = if reverse_arr {
        let part = split_heights(heights_of(arr_track), kept.arr).0;
        part.and_then(|p| reverse_heights(Some(&Value::Array(p)), kept.arr))
            .and_then(|v| v.as_array().cloned())
    } else {
        split_heights(heights_of(arr_track), track_length(arr_track) - kept.arr).1
    };
    let arr_h: Option<Vec<Value>> = arr_h.map(|v| {
        v.into_iter()
            .map(|p| {
                let mut q = p.clone();
                q["station"] = number(js_number(p.get("station")) + offset);
                q
            })
            .collect()
    });
    let dep_ok = dep_h.as_ref().is_some_and(|h| h.len() >= 2);
    let arr_ok = arr_h.as_ref().is_some_and(|h| h.len() >= 2);
    match (dep_ok, arr_ok) {
        (true, true) => {
            let (d, a) = (dep_h.expect("dep"), arr_h.expect("arr"));
            let mut out: Vec<Value> = d[..d.len() - 1].to_vec();
            out.push(plain_point(&d[d.len() - 1]));
            out.push(plain_point(&a[0]));
            out.extend(a[1..].iter().cloned());
            Some(out)
        }
        (true, false) => dep_h,
        (false, true) => arr_h,
        _ => None,
    }
}

fn plane(el: &Value) -> Object {
    let mut o = el.as_object().cloned().unwrap_or_default();
    for k in ["role", "speed", "cant"] {
        o.remove(k);
    }
    o
}

fn over(base: &Value, layers: &[Object]) -> Value {
    let mut o = base.as_object().cloned().unwrap_or_default();
    for l in layers {
        for (k, v) in l {
            o.insert(k.clone(), v.clone());
        }
    }
    Value::Object(o)
}

fn keep_cant(el: &Value, orig: &Value) -> Object {
    let mut o = Object::new();
    if let (Some(c), Some(r)) = (get(orig, "cant").and_then(Value::as_f64), get(el, "radius").and_then(Value::as_f64)) {
        o.insert("cant".into(), number(cant_sign(r) * c.abs()));
    }
    o
}

fn merged_chain(solution: &Value, dep_track: &Value, arr_track: &Value, speed: &Value, cant: f64) -> Vec<Value> {
    let (dep, arr, result) = (&solution["dep"], &solution["arr"], &solution["result"]);
    let chain = result["elements"].as_array().cloned().unwrap_or_default();
    let (reverse_dep, reverse_arr) = (truthy(result.get("reverseDep")), truthy(result.get("reverseArr")));
    let idx = |p: &Value| p["elIdx"].as_u64().unwrap_or(0) as usize;
    let dep_orig = &els(dep_track)[idx(dep)];
    let arr_orig = &els(arr_track)[idx(arr)];
    let dep_base = if reverse_dep { reverse_element(dep_orig) } else { dep_orig.clone() };
    let arr_base = if reverse_arr { reverse_element(arr_orig) } else { arr_orig.clone() };
    let lowered = |p: &Value| -> Object {
        let mut o = Object::new();
        if get(p, "speedBefore").is_some() {
            o.insert("speed".into(), p.get("speed").cloned().unwrap_or(Value::Null));
        }
        o
    };
    let built_on = |el: &Value, p: &Value| -> Value {
        let mut o = plane(el);
        o.insert("speed".into(), p.get("speed").cloned().unwrap_or(Value::Null));
        if is_arc(el) {
            o.insert(
                "cant".into(),
                number(cant_sign(js_number(el.get("radius"))) * f(p, "cant").unwrap_or(0.0).abs()),
            );
        }
        Value::Object(o)
    };
    let mid: Vec<Value> = chain
        .iter()
        .map(|el| match el.get("role").and_then(Value::as_str) {
            Some("dep") if truthy(dep.get("virtual")) => built_on(el, dep),
            Some("dep") => over(&dep_base, &[plane(el), keep_cant(el, dep_orig), lowered(dep)]),
            Some("arr") if truthy(arr.get("virtual")) => built_on(el, arr),
            Some("arr") => over(&arr_base, &[plane(el), keep_cant(el, arr_orig), lowered(arr)]),
            _ => {
                let mut o = plane(el);
                o.insert("speed".into(), speed.clone());
                if is_arc(el) {
                    o.insert("cant".into(), number(cant_sign(js_number(el.get("radius"))) * cant.abs()));
                }
                Value::Object(o)
            }
        })
        .collect();
    let mut out: Vec<Value> = if reverse_dep {
        toward_end(dep_track, dep).iter().rev().map(reverse_element).collect()
    } else {
        toward_start(dep_track, dep)
    };
    out.extend(mid);
    out.extend(if reverse_arr {
        toward_start(arr_track, arr).iter().rev().map(reverse_element).collect::<Vec<_>>()
    } else {
        toward_end(arr_track, arr)
    });
    out
}

/// The splice as one commit (`buildSplice`).
pub fn build_splice(o: &Value) -> Value {
    let tracks: Vec<Value> = o["tracks"].as_array().cloned().unwrap_or_default();
    let solution = &o["solution"];
    let dep_track = tracks.iter().find(|t| strict_eq(t.get("id"), solution["dep"].get("trackId")));
    let arr_track = tracks.iter().find(|t| strict_eq(t.get("id"), solution["arr"].get("trackId")));
    let chain = solution["result"]["elements"].as_array().cloned();
    let (Some(dep_track), Some(arr_track), Some(chain)) = (dep_track, arr_track, chain) else {
        return Value::Null;
    };
    let (dep, arr, result) = (&solution["dep"], &solution["arr"], &solution["result"]);
    let (reverse_dep, reverse_arr) = (truthy(result.get("reverseDep")), truthy(result.get("reverseArr")));
    let elements = merged_chain(solution, dep_track, arr_track, o.get("speed").unwrap_or(&Value::Null), js_number(o.get("cant")));
    let before = (if reverse_dep {
        toward_end(dep_track, dep)
    } else {
        toward_start(dep_track, dep)
    })
    .len();
    let id = o.get("newId").filter(|v| !v.is_null()).cloned().unwrap_or_else(|| json!(generate_id()));
    let merged_length = length_of(&elements);
    let kept = kept_lengths(dep_track, arr_track, dep, arr, &chain, reverse_dep, reverse_arr);
    let heights = splice_heights(dep_track, arr_track, &kept, reverse_dep, reverse_arr, merged_length);
    let mut meta = (if reverse_dep { arr_track } else { dep_track }).as_object().cloned().unwrap_or_default();
    meta.remove("heights");
    meta.insert("id".into(), id.clone());
    meta.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    meta.insert("elements".into(), Value::Array(elements.clone()));
    if let Some(h) = heights {
        meta.insert("heights".into(), Value::Array(h));
    }
    let (dep_len, arr_len) = (track_length(dep_track), track_length(arr_track));
    let arr_from = merged_length - kept.arr;
    let carry = json!([
        if reverse_dep {
            json!({ "oldId": dep["trackId"], "newId": id, "from": number(dep_len - kept.dep), "to": number(dep_len), "offset": number(dep_len), "flip": true })
        } else {
            json!({ "oldId": dep["trackId"], "newId": id, "from": 0, "to": number(kept.dep), "offset": 0, "flip": false })
        },
        if reverse_arr {
            json!({ "oldId": arr["trackId"], "newId": id, "from": 0, "to": number(kept.arr), "offset": number(arr_from + kept.arr), "flip": true })
        } else {
            json!({ "oldId": arr["trackId"], "newId": id, "from": number(arr_len - kept.arr), "to": number(arr_len), "offset": number(arr_from - (arr_len - kept.arr)), "flip": false })
        },
    ]);
    json!({
        "removeTrackIds": [dep["trackId"], arr["trackId"]],
        "addTracks": [Value::Object(meta)],
        "remap": [{ "oldId": dep["trackId"], "newId": id, "flip": reverse_dep }, { "oldId": arr["trackId"], "newId": id, "flip": reverse_arr }],
        "carry": carry,
        "consumed": [{ "trackId": dep["trackId"], "endpoint": if reverse_dep { "BEGIN" } else { "END" } }, { "trackId": arr["trackId"], "endpoint": if reverse_arr { "END" } else { "BEGIN" } }],
        "stretch": { "first": before, "last": before + chain.len() - 1, "from": number(length_of(&elements[..before])) },
    })
}
