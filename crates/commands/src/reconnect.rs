// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reconnecting existing elements: a stretch taken out and its neighbours
//! joined again by the splice construction (`reconnect.js`).

use rail_across_geom::kind::{is_arc, is_transition};
use rail_across_geom::element::{Utm, end_point_curved, end_point_straight, node_utm, resolve_end_bearing};
use rail_across_geom::transition::{is_bloss, sample_transition};
use rail_across_js::{Object, get, js_number, js_round, number, truthy};
use rail_across_rules::cant::{auto_cant_model, cant_sign};
use rail_across_rules::element_length::min_element_length;
use rail_across_track::heights::height_at;
use rail_across_track::model::{rebuild_coords, track_label};
use serde_json::{Value, json};

use crate::splice::{pick_runs, picks_at_speed, splice_from_answer};

pub const AXIS_SPACING: f64 = 0.01;

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn length_of(es: &[Value]) -> f64 {
    es.iter().map(|e| f(e, "length").unwrap_or(0.0)).sum()
}

fn in_switch(el: &Value) -> bool {
    get(el, "switchId").is_some() || el.get("switchBranch") == Some(&json!(true))
}

/// The stretch two clicks choose (`reconnectRange`).
pub fn reconnect_range(track: &Value, a: usize, b: usize) -> Value {
    let es = els(track);
    if es.get(a).is_none() || es.get(b).is_none() {
        return json!({ "error": "reconnect_error_pick" });
    }
    let (mut from, mut to) = (a.min(b), a.max(b));
    let mut widened = Vec::new();
    while from > 0 && is_transition(&es[from - 1]) && !in_switch(&es[from - 1]) {
        from -= 1;
        widened.push(from);
    }
    while to < es.len() - 1 && is_transition(&es[to + 1]) && !in_switch(&es[to + 1]) {
        to += 1;
        widened.push(to);
    }
    if es[from..=to].iter().any(in_switch) {
        return json!({ "error": "reconnect_error_switch" });
    }
    if from == 0 || to == es.len() - 1 {
        return json!({ "error": "reconnect_error_open_end" });
    }
    let side = |idx: usize| json!({ "idx": idx, "fixed": in_switch(&es[idx]) || is_transition(&es[idx]) });
    widened.sort_unstable();
    json!({ "from": from, "to": to, "dep": side(from - 1), "arr": side(to + 1), "widened": widened })
}

/// The elements the commit replaces (`replacedSpan`): (first, last).
pub fn replaced_span(range: &Value) -> (usize, usize) {
    let u = |v: &Value| v.as_u64().unwrap_or(0) as usize;
    let first = if truthy(range["dep"].get("fixed")) {
        u(&range["from"])
    } else {
        u(&range["dep"]["idx"])
    };
    let last = if truthy(range["arr"].get("fixed")) {
        u(&range["to"])
    } else {
        u(&range["arr"]["idx"])
    };
    (first, last)
}

/// A neighbour as the splice takes it (`reconnectPick`).
pub fn reconnect_pick(track: &Value, idx: usize, join_at: &str, fixed: bool) -> Value {
    let es = els(track);
    let el = &es[idx];
    let c = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(Value::as_array);
    let epsg = js_number(track.get("epsg")) as i64;
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    };
    let start = node_utm(el.get("startNode"), c.and_then(|c| c.first()), epsg).unwrap_or(nan);
    let end = node_utm(el.get("endNode"), c.and_then(|c| c.last()), epsg).unwrap_or(nan);
    let mut base = json!({
        "trackId": track.get("id"), "elIdx": idx, "epsg": track.get("epsg"), "label": track_label(Some(track)), "joinAt": join_at,
        "cant": get(el, "cant").cloned().unwrap_or(json!(0)), "speed": get(el, "speed").cloned().unwrap_or(json!(0)),
        "before": number(length_of(&es[..idx])), "after": number(length_of(&es[idx + 1..])),
    });
    if fixed {
        let at_end = join_at == "end";
        let r = if is_transition(el) {
            get(el, if at_end { "r2" } else { "r1" })
        } else {
            get(el, "radius")
        };
        let at = if at_end { end } else { start };
        for (k, v) in [
            ("virtual", json!(join_at)),
            ("startUtm", at.to_json()),
            ("endUtm", at.to_json()),
            ("length", json!(0)),
            (
                "bearing",
                number(if at_end {
                    resolve_end_bearing(el, epsg)
                } else {
                    js_number(el.get("bearing"))
                }),
            ),
            ("signedR", r.cloned().unwrap_or(Value::Null)),
        ] {
            base[k] = v;
        }
        return base;
    }
    let (rb, ra) = pick_runs(es, idx);
    for (k, v) in [
        ("startUtm", start.to_json()),
        ("endUtm", end.to_json()),
        ("length", get(el, "length").cloned().unwrap_or(json!(0))),
        ("bearing", number(resolve_end_bearing(el, epsg))),
        ("signedR", get(el, "radius").cloned().unwrap_or(Value::Null)),
        ("runBefore", number(rb)),
        ("runAfter", number(ra)),
    ] {
        base[k] = v;
    }
    base
}

/// Points along one element in equal steps of at most `step` (`elementPoints`).
pub fn element_points(el: &Value, epsg: i64, step: f64) -> Vec<[f64; 2]> {
    let len = f(el, "length").unwrap_or(0.0);
    let c = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0));
    let start = node_utm(el.get("startNode"), c, epsg).unwrap_or(Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    });
    let n = 1usize.max((len / step - 1e-9).ceil() as usize);
    let bearing = js_number(el.get("bearing"));
    if is_transition(el) {
        return sample_transition(
            start.e,
            start.n,
            bearing,
            len,
            f(el, "r1"),
            f(el, "r2"),
            is_bloss(el.get("transitionType").and_then(Value::as_str)),
            n,
            1,
            len,
        );
    }
    (0..=n)
        .map(|i| {
            let s = len * i as f64 / n as f64;
            let p = match get(el, "radius").and_then(Value::as_f64) {
                Some(r) => end_point_curved(start, bearing, s, r),
                None => end_point_straight(start, bearing, s),
            };
            [p.e, p.n]
        })
        .collect()
}

/// The old axis of some elements every `step` (`axisPoints`).
pub fn axis_points(track: &Value, first: usize, last: usize, step: f64) -> Value {
    let epsg = js_number(track.get("epsg")) as i64;
    let mut coords: Vec<[f64; 2]> = Vec::new();
    for el in &els(track)[first..=last] {
        let pts = element_points(el, epsg, step);
        let skip = usize::from(!coords.is_empty());
        coords.extend(pts.into_iter().skip(skip));
    }
    let [e0, n0] = coords[0];
    json!({
        "e0": number(e0), "n0": number(n0),
        "de": coords.iter().map(|p| number(js_round((p[0] - e0) * 1000.0))).collect::<Vec<_>>(),
        "dn": coords.iter().map(|p| number(js_round((p[1] - n0) * 1000.0))).collect::<Vec<_>>(),
        "coords": coords.iter().map(|p| json!([number(p[0]), number(p[1])])).collect::<Vec<_>>(),
    })
}

/// What the panel starts with (`stretchDefaults`).
pub fn stretch_defaults(track: &Value, range: &Value) -> Value {
    let u = |v: &Value| v.as_u64().unwrap_or(0) as usize;
    let es = &els(track)[u(&range["from"])..=u(&range["to"])];
    let tr = es.iter().find(|e| is_transition(e));
    json!({
        "speed": number(es.iter().map(|e| f(e, "speed").unwrap_or(0.0)).fold(0.0, f64::max)),
        "transitionType": if tr.and_then(|t| t.get("transitionType")).and_then(Value::as_str) == Some("bloss") { "bloss" } else { "clothoid" },
    })
}

fn num_or_zero(v: Option<&Value>) -> f64 {
    let x = js_number(v);
    if x.is_nan() { 0.0 } else { x }
}

/// The request for `POST /reconnect` (`reconnectRequest`).
pub fn reconnect_request(picks: &[Value], points: &Value, s: &Value, clearance: Option<&Value>) -> Value {
    let at = picks_at_speed(picks, s.get("speed"));
    let pick = |p: &Value| -> Value {
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
        o["joinAt"] = p.get("joinAt").cloned().unwrap_or(Value::Null);
        o
    };
    let speed = num_or_zero(s.get("speed"));
    let mut out = json!({
        "dep": pick(&at[0]), "arr": pick(&at[1]),
        "points": { "e0": points.get("e0"), "n0": points.get("n0"), "de": points.get("de"), "dn": points.get("dn") },
        "tolerance": number(num_or_zero(s.get("tolerance"))),
        "speed": number(speed),
        "transition": get(s, "transitionType").cloned().unwrap_or(json!("clothoid")),
        "radius": number(num_or_zero(s.get("radius")).abs()),
        "cantModel": auto_cant_model(),
        "lMin": number(min_element_length(js_number(s.get("speed"))).unwrap_or(0.0)),
    });
    if let Some(c) = clearance.filter(|c| truthy(Some(c))) {
        out["clearance"] = c.clone();
    }
    out
}

/// The optimizer module's answer as the panel reads it (`reconnectFromAnswer`).
pub fn reconnect_from_answer(answer: &Value, picks: &[Value], request: Option<&Value>) -> Value {
    if answer.get("solutions").and_then(Value::as_array).is_none_or(Vec::is_empty) {
        return json!({ "error": get(answer, "error").cloned().unwrap_or(json!("reconnect_error_no_fit")) });
    }
    let read = splice_from_answer(answer, picks, request);
    let solutions: Vec<Value> = read["solutions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, sol)| {
            let mut s = sol.clone();
            s["reconnect"] = answer["solutions"][i].get("reconnect").cloned().unwrap_or(Value::Null);
            s
        })
        .collect();
    json!({ "tolerance": answer.get("tolerance"), "solutions": solutions })
}

/// Where a station of the old track lies on the new one (`stationMap`).
pub fn station_map(s0: f64, old_length: f64, new_length: f64, s: f64) -> f64 {
    if s <= s0 {
        return s;
    }
    if s >= s0 + old_length {
        return s + new_length - old_length;
    }
    s0 + (s - s0) * new_length / old_length
}

fn restationed_heights(heights: Option<&Value>, map: &dyn Fn(f64) -> f64, length: f64) -> Option<Vec<Value>> {
    let hs = heights.and_then(Value::as_array).filter(|h| !h.is_empty())?;
    let out: Vec<Value> = hs
        .iter()
        .map(|p| {
            let mut q = p.clone();
            q["station"] = number(map(js_number(p.get("station"))));
            q
        })
        .collect();
    let mut inside: Vec<Value> = out.iter().filter(|p| js_number(p.get("station")) <= length + 1e-6).cloned().collect();
    if inside.len() < out.len() && !inside.is_empty() && js_number(inside[inside.len() - 1].get("station")) < length - 1e-6 {
        inside.push(json!({ "station": number(length), "z": height_at(&out, length).map(number) }));
    }
    (inside.len() >= 2).then_some(inside)
}

fn plane(el: &Value) -> Object {
    let mut o = el.as_object().cloned().unwrap_or_default();
    for k in ["role", "speed", "cant"] {
        o.remove(k);
    }
    o
}

/// The track as it is after reconnecting (`buildReconnect`): { track, s0, oldLength, newLength } or None.
pub fn build_reconnect(track: &Value, range: &Value, solution: &Value, speed: Option<&Value>) -> Option<Value> {
    let chain = solution["result"]["elements"].as_array().filter(|c| !c.is_empty())?;
    if truthy(solution["result"].get("reverseDep")) || truthy(solution["result"].get("reverseArr")) {
        return None;
    }
    let es = els(track);
    let (first, last) = replaced_span(range);
    let u = |v: &Value| v.as_u64().unwrap_or(0) as usize;
    let dep_orig = &es[u(&range["dep"]["idx"])];
    let arr_orig = &es[u(&range["arr"]["idx"])];
    let cant = solution.get("reconnect").and_then(|r| f(r, "cant")).unwrap_or(0.0).abs();
    let v = num_or_zero(speed);
    let keep_cant = |el: &Value, orig: &Value, o: &mut Object| {
        if let (Some(c), Some(r)) = (get(orig, "cant").and_then(Value::as_f64), get(el, "radius").and_then(Value::as_f64)) {
            o.insert("cant".into(), number(cant_sign(r) * c.abs()));
        }
    };
    let slowed = |orig: &Value, o: &mut Object| {
        if v > 0.0 && f(orig, "speed").unwrap_or(0.0) > v {
            o.insert("speed".into(), number(v));
        }
    };
    let built_on = |el: &Value, orig: &Value| -> Value {
        let mut o = plane(el);
        o.insert("speed".into(), orig.get("speed").cloned().unwrap_or(Value::Null));
        slowed(orig, &mut o);
        if is_arc(el) {
            o.insert(
                "cant".into(),
                number(cant_sign(js_number(el.get("radius"))) * f(orig, "cant").unwrap_or(0.0).abs()),
            );
        }
        Value::Object(o)
    };
    let own = |el: &Value| -> Object {
        let mut o = el.as_object().cloned().unwrap_or_default();
        for k in ["switchId", "switchBranch", "switchRoute"] {
            o.remove(k);
        }
        o
    };
    let reshaped = |el: &Value, orig: &Value| -> Value {
        let mut o = own(orig);
        for (k, val) in plane(el) {
            o.insert(k, val);
        }
        keep_cant(el, orig, &mut o);
        slowed(orig, &mut o);
        Value::Object(o)
    };
    let mid: Vec<Value> = chain
        .iter()
        .map(|el| match el.get("role").and_then(Value::as_str) {
            Some("dep") if truthy(range["dep"].get("fixed")) => built_on(el, dep_orig),
            Some("dep") => reshaped(el, dep_orig),
            Some("arr") if truthy(range["arr"].get("fixed")) => built_on(el, arr_orig),
            Some("arr") => reshaped(el, arr_orig),
            _ => {
                let mut o = plane(el);
                o.insert("speed".into(), number(v));
                if is_arc(el) {
                    o.insert("cant".into(), number(cant_sign(js_number(el.get("radius"))) * cant));
                }
                Value::Object(o)
            }
        })
        .collect();
    let mut elements: Vec<Value> = es[..first].to_vec();
    elements.extend(mid.iter().cloned());
    elements.extend(es[last + 1..].iter().cloned());
    let s0 = length_of(&es[..first]);
    let (old_length, new_length) = (length_of(&es[first..=last]), length_of(&mid));
    let map = |s: f64| station_map(s0, old_length, new_length, s);
    let length = length_of(&elements);
    let heights = restationed_heights(track.get("heights"), &map, length);
    let mut meta = track.as_object().cloned().unwrap_or_default();
    meta.remove("heights");
    meta.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    meta.insert("elements".into(), Value::Array(elements));
    if let Some(h) = heights {
        meta.insert("heights".into(), Value::Array(h));
    }
    Some(json!({ "track": Value::Object(meta), "s0": number(s0), "oldLength": number(old_length), "newLength": number(new_length) }))
}
