// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What the switch dialogs commit (`switches.js`).

use rail_across_geom::cant::with_cant_ramp;
use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, bearing_after, curved_values, straight_values};
use rail_across_geom::factory::{arc_element, straight_element};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::curvature_of;
use rail_across_js::{Object, generate_id, get, js_number, js_string, number, strict_eq, truthy};
use rail_across_rules::cant::{cant_exception_fields, worst_cant_of};
use rail_across_rules::element_length::min_element_length;
use rail_across_switch::catalogue::{Kind, switch_branch_length, switch_branch_sections};
use rail_across_switch::crossing::{compute_crossing_geometry_utm, crossing_elements, crossing_leg_radius, crossing_leg_signed_radius, pieces_on_radius};
use rail_across_switch::placement::{Placement, place_switch_on_track};
use rail_across_switch::route::{
    Route, neg_r, switch_abreast, switch_branch_chain, switch_chain_bearing_at, switch_chain_joints, switch_chain_point_utm, switch_chain_to,
    switch_route_varies, utm_end_route,
};
use rail_across_switch::slider::{unique_stations, valid_intervals};
use rail_across_switch::symbol::{MainR, compute_switch_geometry_utm};
use rail_across_track::heights::{element_at_station, track_length};
use rail_across_track::ids::build_type_fields;
use rail_across_track::model::reverse_track;
use rail_across_track::split::{carve_switch_route, split_element_at, split_track_at_joint};
use rail_across_track::switch_model::{element_belongs_to_switch, new_switch_fields, switch_element_mark};
use serde_json::{Value, json};

use crate::scurve::run_beside;

const MIN_BEHIND: f64 = 0.5;
pub const SLIDER_REACH: f64 = 50.0;

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// What a turnout laid into a track is asked with.
pub struct OnTrack<'a> {
    pub track: &'a Value,
    pub reversed: bool,
    pub sw: &'a Value,
    pub side: &'a str,
    pub straight_len: f64,
    pub speed: Option<f64>,
    /// What the slider snaps onto an element change: `toe` (WA), `end` (WE) or
    /// `technical` (the technical end); both WA and WE where it is not said.
    pub snap: Option<&'a str>,
}

impl<'a> OnTrack<'a> {
    pub fn from_json(o: &'a Value) -> OnTrack<'a> {
        OnTrack {
            track: &o["track"],
            reversed: truthy(o.get("reversed")),
            sw: &o["sw"],
            side: o.get("side").and_then(Value::as_str).unwrap_or("left"),
            straight_len: js_number(o.get("straightLen")),
            speed: f(o, "speed"),
            snap: o.get("snap").and_then(Value::as_str),
        }
    }
}

fn err(key: &str) -> Value {
    json!({ "key": key })
}

fn place_on_track(a: &OnTrack, toe: f64) -> Result<Placement, Value> {
    let total = track_length(a.track);
    if toe < 0.0 || toe > total {
        return Err(err("switch_on_track_outside"));
    }
    if (if a.reversed { total - toe } else { toe }) <= MIN_BEHIND {
        return Err(err("switch_on_track_no_room"));
    }
    let place = place_switch_on_track(a.track, toe, a.reversed, a.straight_len).map_err(err)?;
    if truthy(a.sw.get("symmetric")) {
        let r = js_number(a.sw.get("R"));
        if !pieces_on_radius(&place.pieces, Some(if a.side == "left" { r } else { -r })) {
            return Err(json!({ "key": "switch_on_track_symmetric", "params": { "r": rail_across_js::es_number(r) } }));
        }
    }
    Ok(place)
}

fn all_straight(pieces: &[Route]) -> bool {
    pieces.iter().all(|p| p.r1.is_none() && p.r2.is_none())
}

fn wgs(u: Utm, zone: i64) -> [f64; 2] {
    let (lng, lat) = plane_to_wgs84(u.e, u.n, zone).unwrap_or((f64::NAN, f64::NAN));
    [lng, lat]
}

/// A placement and the geometry it produces (`switchOnTrackPlacement`).
pub struct OnTrackPlacement {
    pub place: Placement,
    pub plain: bool,
    pub geom: Value,
}

pub fn switch_on_track_placement(a: &OnTrack, toe: Option<f64>) -> Result<Option<OnTrackPlacement>, Value> {
    let Some(toe) = toe.filter(|t| t.is_finite()) else { return Ok(None) };
    if a.track.is_null() {
        return Ok(None);
    }
    let place = place_on_track(a, toe)?;
    let plain = truthy(a.sw.get("symmetric")) || all_straight(&place.pieces);
    let epsg = js_number(a.track.get("epsg")) as i64;
    let toe_wgs = wgs(place.toe, epsg);
    let stem = if plain { MainR::Radius(None) } else { MainR::Chain(place.pieces.clone()) };
    let geom = compute_switch_geometry_utm(place.toe, place.bearing, a.sw, a.side, false, Some(toe_wgs), &stem);
    Ok(Some(OnTrackPlacement { place, plain, geom }))
}

pub fn placement_json(r: Result<Option<OnTrackPlacement>, Value>) -> Value {
    match r {
        Err(e) => json!({ "error": e }),
        Ok(None) => json!({ "error": null }),
        Ok(Some(p)) => json!({ "error": null, "place": p.place.to_json(), "plain": p.plain, "geom": p.geom }),
    }
}

/// Where the new route of a turnout ends (`switchOnTrackRouteEnd`): (utm, bearing, k).
pub fn switch_on_track_route_end(a: &OnTrack, toe: f64) -> Option<(Utm, f64, f64)> {
    if a.track.is_null() || !toe.is_finite() {
        return None;
    }
    if truthy(a.sw.get("symmetric")) {
        let p = switch_on_track_placement(a, Some(toe)).ok().flatten()?;
        let segs = p.geom["branchSegments"].as_array().cloned().unwrap_or_default();
        return Some((
            Utm::from_json(&p.geom["curvedUtm"]),
            js_number(p.geom.get("branchEndBearing")),
            curvature_of(segs.last().and_then(|s| f(s, "r2"))),
        ));
    }
    let place = place_on_track(a, toe).ok()?;
    let plain = all_straight(&place.pieces);
    let stem = if plain {
        vec![Route {
            length: a.straight_len,
            r1: None,
            r2: None,
        }]
    } else {
        switch_chain_to(&place.pieces, a.straight_len)
    };
    let fc = rail_across_switch::route::form_chain(&Value::Array(rail_across_switch::catalogue::switch_form_chain(Some(a.sw), a.side)));
    let branch = switch_branch_chain(&fc, &stem);
    if !plain && all_straight(&branch) {
        let straight = [Route {
            length: a.straight_len,
            r1: None,
            r2: None,
        }];
        return Some((switch_chain_point_utm(place.toe, place.bearing, &straight, None)?, place.bearing, 0.0));
    }
    Some((
        switch_chain_point_utm(place.toe, place.bearing, &branch, None)?,
        switch_chain_bearing_at(place.bearing, &branch, None),
        curvature_of(branch.last().and_then(|b| b.r2)),
    ))
}

/// The pieces a turnout leaves of its elements (`switchOnTrackRemnant`): at WA
/// the piece before it and the piece of WA's element under it; given the
/// turnout's `length`, at WE too the piece of WE's element under it and the
/// piece left beyond — each held to the minimum element length of `speed`
/// together with the elements of the same geometry it runs on into.
pub fn switch_on_track_remnant(track: &Value, toe: f64, reversed: bool, speed: Option<f64>, length: Option<f64>) -> Value {
    let es = els(track);
    let Some((idx, s)) = element_at_station(es, toe) else { return Value::Null };
    let len = f(&es[idx], "length").unwrap_or(0.0);
    let on_node = s <= NODE_TOL || s >= len - NODE_TOL;
    let behind_wa = if on_node {
        0.0
    } else if reversed {
        len - s
    } else {
        s
    };
    let ahead = if on_node { 0.0 } else { len - behind_wa };
    let speed = speed.filter(|v| *v != 0.0 && !v.is_nan());
    let l_min = speed.and_then(min_element_length);
    let (back, on) = if reversed { ("after", "before") } else { ("before", "after") };
    let run = |piece: f64, i: usize, side: &str| {
        if piece > NODE_TOL {
            piece + run_beside(es, i, side).unwrap_or(0.0)
        } else {
            piece
        }
    };
    let too_short = |l: f64, r: f64| l_min.is_some_and(|m| l > NODE_TOL && r < m - NODE_TOL);
    let (run_behind, run_ahead) = (run(behind_wa, idx, back), run(ahead, idx, on));
    let mut out = json!({
        "elIdx": idx, "length": number(behind_wa), "ahead": number(ahead), "run": number(run_behind), "runAhead": number(run_ahead),
        "lMin": l_min.map(number), "speed": speed.map(number), "short": too_short(behind_wa, run_behind), "shortAhead": too_short(ahead, run_ahead),
    });
    let we = length.filter(|l| l.is_finite() && *l > 0.0).map(|l| (l, if reversed { toe - l } else { toe + l }));
    if let Some((l, we)) = we
        && let Some((j, s)) = element_at_station(es, we)
    {
        let len = f(&es[j], "length").unwrap_or(0.0);
        let (under, beyond) = if s <= NODE_TOL || s >= len - NODE_TOL {
            (0.0, 0.0)
        } else if reversed {
            ((len - s).min(l), s)
        } else {
            (s.min(l), len - s)
        };
        // A turnout within one element is one piece under it, not two.
        let under = if j == idx && !on_node { 0.0 } else { under };
        let (run_under, run_beyond) = (run(under, j, back), run(beyond, j, on));
        out["end"] = json!({
            "elIdx": j, "under": number(under), "beyond": number(beyond), "runUnder": number(run_under), "runBeyond": number(run_beyond),
            "shortUnder": too_short(under, run_under), "shortBeyond": too_short(beyond, run_beyond),
        });
    }
    out
}

/// May the toe stand here (`switchOnTrackToeValid`)?
pub fn switch_on_track_toe_valid(a: &OnTrack, toe: f64) -> bool {
    if place_on_track(a, toe).is_err() {
        return false;
    }
    let r = switch_on_track_remnant(a.track, toe, a.reversed, a.speed, Some(a.straight_len));
    ["short", "shortAhead"].iter().all(|k| !truthy(r.get(*k))) && ["shortUnder", "shortBeyond"].iter().all(|k| !truthy(r["end"].get(*k)))
}

fn slider_window(track: &Value, around: Option<f64>) -> (f64, f64) {
    let total = track_length(track);
    match around.filter(|a| a.is_finite()) {
        None => (0.0, total),
        Some(a) => (0f64.max(a - SLIDER_REACH), total.min(a + SLIDER_REACH)),
    }
}

fn node_stations(track: &Value) -> Vec<f64> {
    let mut at = 0.0;
    let mut out = vec![0.0];
    for el in els(track) {
        at += f(el, "length").unwrap_or(0.0);
        out.push(at);
    }
    out
}

fn intervals_json(iv: &[(f64, f64)], snaps: &[f64]) -> Value {
    json!({ "intervals": iv.iter().map(|(a, b)| json!([number(*a), number(*b)])).collect::<Vec<_>>(), "snaps": snaps.iter().map(|s| number(*s)).collect::<Vec<_>>() })
}

/// How far the technical end of a turnout lies from WA along its through
/// route: where the branch's last curved section ends — at the same distance on
/// the through route where an element change lies there, as `switch_abreast`
/// reads the stations up to the last change under the turnout (Entscheidung 370)
/// — or WE, for a branch curved to its end.
pub fn switch_technical_end(sw: &Value, straight_len: f64) -> f64 {
    let sections = switch_branch_sections(Some(sw));
    match sections.iter().rposition(|s| s.kind != Kind::Straight) {
        Some(i) if i + 1 < sections.len() => sections[..=i].iter().map(|s| s.length).sum(),
        _ => straight_len,
    }
}

/// What the position slider of "Weiche auf Gleis" runs over (`switchOnTrackSlider`):
/// the stretches where the toe may stand, and the toe stations that put the
/// chosen point of the turnout (`snap`) on an element change — exactly there,
/// so no sliver of an element is left between.
pub fn switch_on_track_slider(a: &OnTrack, around: Option<f64>) -> Value {
    if els(a.track).is_empty() {
        return json!({ "intervals": [], "snaps": [] });
    }
    let nodes = node_stations(a.track);
    let (from, to) = slider_window(a.track, around);
    let offsets: Vec<f64> = match a.snap {
        Some("toe") => vec![0.0],
        Some("end") => vec![a.straight_len],
        Some("technical") => vec![switch_technical_end(a.sw, a.straight_len)],
        _ => vec![0.0, a.straight_len],
    };
    let all: Vec<f64> = offsets
        .iter()
        .flat_map(|d| nodes.iter().map(move |n| if a.reversed { n + d } else { n - d }))
        .collect();
    let snaps = unique_stations(&all);
    let iv = valid_intervals(&|s| switch_on_track_toe_valid(a, s), from, to, 1.0, &snaps);
    intervals_json(&iv, &snaps)
}

fn constant_branch_element(seg: &Value, cant_a: f64, cant_b: f64) -> Value {
    let (s, e) = (Utm::from_json(&seg["startUtm"]), Utm::from_json(&seg["endUtm"]));
    let r = f(seg, "r1").filter(|r| *r != 0.0);
    let node = |u: Utm| json!([number(u.e), number(u.n)]);
    let el = match r {
        Some(r) => {
            let (length, bearing, end_bearing) = curved_values(s, e, r);
            json!({ "elementType": 1, "startNode": node(s), "endNode": node(e), "bearing": number(bearing), "length": number(length), "endBearing": number(end_bearing), "radius": number(r) })
        }
        None => {
            let (length, bearing) = straight_values(s, e);
            json!({ "elementType": 0, "startNode": node(s), "endNode": node(e), "bearing": number(bearing), "length": number(length) })
        }
    };
    with_cant_ramp(&el, cant_a, cant_b)
}

fn merged(a: &Value, b: &Value) -> Value {
    let mut o = a.as_object().cloned().unwrap_or_default();
    for (k, v) in b.as_object().into_iter().flatten() {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o)
}

fn existing_names(tracks: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tracks {
        if let Some(n) = t.get("name").filter(|v| truthy(Some(v))) {
            let s = js_string(Some(n));
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out
}

fn identity_of(kind: &str, name: &Value, label: Option<&Value>) -> Value {
    let mut id = new_switch_fields(kind).as_object().cloned().unwrap_or_default();
    id.insert("name".into(), name.clone());
    id.insert("label".into(), label.cloned().unwrap_or(Value::Null));
    Value::Object(id)
}

fn track_fields(fields: &Value) -> Object {
    let mut o = Object::new();
    if let Some(owner) = fields.get("owner") {
        o.insert("owner".into(), owner.clone());
    }
    for (k, v) in build_type_fields(fields).as_object().into_iter().flatten() {
        o.insert(k.clone(), v.clone());
    }
    o
}

/// The commit of "switch on track" (`buildSwitchOnTrack`).
pub fn build_switch_on_track(o: &Value, place: &Placement) -> Value {
    let track = &o["track"];
    let tracks: Vec<Value> = o["tracks"].as_array().cloned().unwrap_or_default();
    let g = &o["g"];
    let plain = truthy(o.get("plain"));
    let sw = &o["sw"];
    let straight_len = js_number(o.get("straightLen"));
    let cant = js_number(o.get("cant"));
    let cant_reason = o.get("cantReason").and_then(Value::as_str);
    let mut names = existing_names(&tracks);
    let split = match place.joint {
        Some(j) => Some(split_track_at_joint(track, j, place.bearing, &mut names)),
        None => {
            let mut cut = place.toe.to_json();
            if place.cuts_clothoid {
                cut["station"] = number(place.s.unwrap_or(f64::NAN));
            }
            split_element_at(track, place.el_idx.unwrap_or(0), &cut, place.bearing, &mut names)
        }
    };
    let Some(split) = split else { return Value::Null };
    let identity = identity_of("turnout", o.get("switchName").unwrap_or(&Value::Null), sw.get("label"));
    let swapped = g.get("swapped").filter(|v| !v.is_null());
    let shown = swapped.unwrap_or(g);
    let line_route = if swapped.is_some() { "branch" } else { "main" };
    let line_length = if swapped.is_some() { switch_branch_length(Some(sw)) } else { straight_len };
    let cut = match swapped {
        Some(s) => s["curvedUtm"].clone(),
        None => place.end.to_json(),
    };
    let ahead_ep = split["aheadEndpoint"].as_str().unwrap_or("BEGIN");
    let Some(carved) = carve_switch_route(&split["ahead"], ahead_ep, &cut, &switch_element_mark(&identity, line_route), Some(line_length)) else {
        return json!({ "noRoom": number(line_length) });
    };
    let justify = |el: &Value| -> Value { merged(el, &cant_exception_fields(Some(worst_cant_of(el)), cant_reason)) };
    let mut carved_route = carved.clone();
    carved_route["elements"] = Value::Array(
        els(&carved)
            .iter()
            .map(|el| if element_belongs_to_switch(el, &identity) { justify(el) } else { el.clone() })
            .collect(),
    );
    let split_tracks: Vec<Value> = split["tracks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| {
            if strict_eq(t.get("id"), carved.get("id")) {
                carved_route.clone()
            } else {
                t.clone()
            }
        })
        .collect();
    let branch_id = generate_id();
    let new_route = if swapped.is_some() { "main" } else { "branch" };
    let new_segments: Vec<Value> = (match swapped {
        Some(s) => &s["stemSegments"],
        None => &g["branchSegments"],
    })
    .as_array()
    .cloned()
    .unwrap_or_default();
    let new_length: f64 = new_segments.iter().map(|s| js_number(s.get("length"))).sum();
    let joints = switch_chain_joints(&place.pieces);
    let host_at = |u: f64| switch_abreast(u, &joints, new_length, line_length);
    let cant_beside = |u: f64, seg: &Value| {
        let (s0, l) = (js_number(seg.get("s0")), js_number(seg.get("length")));
        let (a, b) = (host_at(s0), host_at(s0 + l));
        place.cant_at(host_at(u), Some((a, b - a)))
    };
    let mark = switch_element_mark(&identity, new_route);
    let branch_els: Vec<Value> = new_segments
        .iter()
        .map(|seg| {
            let (s0, l) = (js_number(seg.get("s0")), js_number(seg.get("length")));
            let route = rail_across_switch::route::to_route(seg);
            let base = if switch_route_varies(&route) {
                let s = Utm::from_json(&seg["startUtm"]);
                let e = Utm::from_json(&seg["endUtm"]);
                json!({
                    "elementType": 2, "transitionType": "clothoid", "r1": seg.get("r1"), "r2": seg.get("r2"),
                    "startNode": [number(s.e), number(s.n)], "endNode": [number(e.e), number(e.n)],
                    "bearing": seg.get("bearing"), "endBearing": seg.get("endBearing"), "length": seg.get("length"),
                    "cantStart": number(cant_beside(s0, seg)), "cantEnd": number(cant_beside(s0 + l, seg)),
                })
            } else if plain {
                constant_branch_element(seg, cant, cant)
            } else {
                constant_branch_element(seg, cant_beside(s0, seg), cant_beside(s0 + l, seg))
            };
            let mut el = merged(&base, &mark);
            el["geometry"] = json!({ "type": "LineString", "coordinates": seg.get("coords") });
            justify(&el)
        })
        .collect();
    let fields = o.get("fields").unwrap_or(&Value::Null);
    let mut branch_track = Object::new();
    branch_track.insert("id".into(), json!(branch_id));
    branch_track.insert("name".into(), o.get("name").cloned().unwrap_or(Value::Null));
    for (k, v) in track_fields(fields) {
        branch_track.insert(k, v);
    }
    branch_track.insert("epsg".into(), track.get("epsg").cloned().unwrap_or(Value::Null));
    branch_track.insert(
        "coordinates".into(),
        match swapped {
            Some(s) => s["straightCoords"].clone(),
            None => g["arcCoords"].clone(),
        },
    );
    branch_track.insert("elements".into(), Value::Array(branch_els));
    let new_port = json!({ "trackId": branch_id, "endpoint": "BEGIN" });
    let line_port = json!({ "trackId": split["ahead"].get("id"), "endpoint": split["aheadEndpoint"] });
    let (b1, b2) = if swapped.is_some() {
        (&line_port, &new_port)
    } else {
        (&new_port, &line_port)
    };
    let mut record = identity.as_object().cloned().unwrap_or_default();
    record.insert("number".into(), o.get("switchNumber").cloned().unwrap_or(Value::Null));
    record.insert("trailing".into(), json!(false));
    record.insert("speed".into(), o.get("speed").cloned().unwrap_or(Value::Null));
    if swapped.is_some() {
        record.insert("swapped".into(), json!(true));
    }
    record.insert(
        "ports".into(),
        json!({ "A": { "trackId": split["behind"].get("id"), "endpoint": split["behindEndpoint"] }, "B1": b1, "B2": b2 }),
    );
    for k in ["fillCoords", "lcsCoords", "labelCoords", "bauform"] {
        record.insert(k.into(), shown.get(k).cloned().unwrap_or(Value::Null));
    }
    let mut add = split_tracks.clone();
    add.push(Value::Object(branch_track));
    json!({
        "removeTrackIds": [track.get("id")], "addTracks": add, "addSwitches": [Value::Object(record)],
        "remap": [{ "oldId": track.get("id"), "newId": split_tracks.iter().map(|t| t.get("id").cloned().unwrap_or(Value::Null)).collect::<Vec<_>>() }],
    })
}

fn route_element(from: Utm, to: Utm, signed_r: Option<f64>, coords: &Value, extra: &Value) -> Value {
    let mut base: Object = extra.as_object().cloned().unwrap_or_default();
    base.insert("geometry".into(), json!({ "type": "LineString", "coordinates": coords }));
    let mut el = match signed_r.filter(|r| *r != 0.0 && !r.is_nan()) {
        Some(r) => arc_element(from, to, r, &base),
        None => straight_element(from, to, &base),
    }
    .unwrap_or(Value::Null);
    if let Some(o) = el.as_object_mut() {
        o.remove("renderCoords");
    }
    el
}

fn cants(cant: f64, reason: Option<&str>) -> Value {
    merged(&json!({ "cant": number(cant) }), &cant_exception_fields(Some(cant), reason))
}

/// The commit of a turnout at a track's end (`buildSwitchAtTrackEnd`).
pub fn build_switch_at_track_end(o: &Value) -> Value {
    let start = Utm::from_json(&o["start"]);
    let bearing = js_number(o.get("bearing"));
    let start_wgs = o
        .get("startWgs")
        .filter(|v| !v.is_null())
        .map(|v| [v[0].as_f64().unwrap_or(f64::NAN), v[1].as_f64().unwrap_or(f64::NAN)]);
    let sw = &o["sw"];
    let side = o.get("side").and_then(Value::as_str).unwrap_or("left");
    let trailing = truthy(o.get("trailing"));
    let stem = MainR::Radius(f(o, "stemSigned"));
    let geom = compute_switch_geometry_utm(start, bearing, sw, side, trailing, start_wgs, &stem);
    let cant = js_number(o.get("cant"));
    let reason = o.get("cantReason").and_then(Value::as_str);
    let source = o.get("sourceTrackId").cloned().unwrap_or(Value::Null);
    let fields = o.get("fields").unwrap_or(&Value::Null);
    let main_fields = o.get("mainFields").unwrap_or(&Value::Null);
    let zone = json!(start.zone);
    let new_track = |name: &Value, fields: &Value, coords: &Value, el: Value| -> Value {
        let mut t = Object::new();
        t.insert("id".into(), json!(generate_id()));
        t.insert("name".into(), name.clone());
        for (k, v) in track_fields(fields) {
            t.insert(k, v);
        }
        t.insert("epsg".into(), zone.clone());
        t.insert("coordinates".into(), coords.clone());
        t.insert("elements".into(), json!([el]));
        Value::Object(t)
    };
    let name = o.get("name").cloned().unwrap_or(Value::Null);
    let main_name = o.get("mainName").cloned().unwrap_or(Value::Null);
    let record_base = |identity: &Value| -> Object {
        let mut r = identity.as_object().cloned().unwrap_or_default();
        r.insert("number".into(), o.get("switchNumber").cloned().unwrap_or(Value::Null));
        r.insert("trailing".into(), json!(trailing));
        r.insert("speed".into(), o.get("speed").cloned().unwrap_or(Value::Null));
        r
    };
    if let Some(sg) = geom.get("swapped").filter(|v| !v.is_null()) {
        let identity = identity_of("turnout", o.get("switchName").unwrap_or(&Value::Null), sw.get("label"));
        let through_el = route_element(
            Utm::from_json(&sg["startUtm"]),
            Utm::from_json(&sg["straightUtm"]),
            None,
            &sg["straightCoords"],
            &merged(&switch_element_mark(&identity, "main"), &cants(cant, reason)),
        );
        let through_track = new_track(&name, fields, &sg["straightCoords"], through_el);
        let branch_el = if trailing {
            let mut coords = vec![o.get("startWgs").filter(|v| !v.is_null()).cloned().unwrap_or_else(|| sg["curvedEnd"].clone())];
            let arc: Vec<Value> = sg["arcCoords"].as_array().cloned().unwrap_or_default();
            coords.extend(arc.into_iter().rev().skip(1));
            route_element(
                start,
                Utm::from_json(&sg["startUtm"]),
                f(o, "stemSigned"),
                &Value::Array(coords),
                &merged(&switch_element_mark(&identity, "branch"), &cants(-cant, reason)),
            )
        } else {
            route_element(
                Utm::from_json(&sg["arcOriginUtm"]),
                Utm::from_json(&sg["curvedUtm"]),
                f(sg, "signedR"),
                &sg["arcCoords"],
                &merged(&switch_element_mark(&identity, "branch"), &cants(cant, reason)),
            )
        };
        let branch_track = (!trailing).then(|| new_track(&main_name, main_fields, &sg["arcCoords"], branch_el.clone()));
        let mut record = record_base(&identity);
        record.insert("swapped".into(), json!(true));
        record.insert(
            "ports".into(),
            json!({
                "A": { "trackId": if trailing { Value::Null } else { source.clone() }, "endpoint": if trailing { Value::Null } else { json!("END") } },
                "B1": { "trackId": if trailing { source.clone() } else { branch_track.as_ref().map(|t| t["id"].clone()).unwrap_or(Value::Null) }, "endpoint": if trailing { "END" } else { "BEGIN" } },
                "B2": { "trackId": through_track["id"], "endpoint": "BEGIN" },
            }),
        );
        for k in ["fillCoords", "labelCoords", "bauform", "lcsCoords"] {
            record.insert(k.into(), sg.get(k).cloned().unwrap_or(Value::Null));
        }
        let mut add: Vec<Value> = branch_track.into_iter().collect();
        add.push(through_track);
        return json!({
            "removeTrackIds": [], "append": if trailing { json!([{ "trackId": source, "elements": [branch_el] }]) } else { json!([]) },
            "addTracks": add, "addSwitches": [Value::Object(record)],
        });
    }
    let main_cant = if trailing { -cant } else { cant };
    let identity = identity_of("turnout", o.get("switchName").unwrap_or(&Value::Null), sw.get("label"));
    let main_signed = f(&geom, "mainSignedR");
    let mut straight_extra = switch_element_mark(&identity, "main");
    if main_signed.is_some_and(|r| r != 0.0) {
        straight_extra = merged(&straight_extra, &cants(main_cant, reason));
    }
    let straight_el = route_element(
        Utm::from_json(&geom["startUtm"]),
        Utm::from_json(&geom["straightUtm"]),
        main_signed,
        &geom["straightCoords"],
        &straight_extra,
    );
    let curved_el = route_element(
        Utm::from_json(&geom["arcOriginUtm"]),
        Utm::from_json(&geom["curvedUtm"]),
        f(&geom, "signedR"),
        &geom["arcCoords"],
        &merged(&switch_element_mark(&identity, "branch"), &cants(cant, reason)),
    );
    let curved_track = new_track(&name, fields, &geom["arcCoords"], curved_el);
    let straight_track = (!trailing).then(|| new_track(&main_name, main_fields, &geom["straightCoords"], straight_el.clone()));
    let mut record = record_base(&identity);
    if let Some(r) = f(&geom, "stemAtToe").filter(|r| *r != 0.0) {
        record.insert("mainRadius".into(), number(r));
    }
    record.insert(
        "ports".into(),
        json!({
            "A": { "trackId": if trailing { Value::Null } else { source.clone() }, "endpoint": if trailing { Value::Null } else { json!("END") } },
            "B1": { "trackId": curved_track["id"], "endpoint": "BEGIN" },
            "B2": { "trackId": if trailing { source.clone() } else { straight_track.as_ref().map(|t| t["id"].clone()).unwrap_or(Value::Null) }, "endpoint": if trailing { "END" } else { "BEGIN" } },
        }),
    );
    record.insert("fillCoords".into(), geom["fillCoords"].clone());
    record.insert("labelCoords".into(), geom["labelCoords"].clone());
    record.insert("bauform".into(), geom["bauform"].clone());
    record.insert("lcsCoords".into(), geom["lcsCoords"].clone());
    let mut add: Vec<Value> = straight_track.into_iter().collect();
    add.push(curved_track);
    json!({
        "removeTrackIds": [], "append": if trailing { json!([{ "trackId": source, "elements": [straight_el] }]) } else { json!([]) },
        "addTracks": add, "addSwitches": [Value::Object(record)],
    })
}

/// The commit of a crossing laid into a track (`buildCrossingOnTrack`).
pub fn build_crossing_on_track(o: &Value, ahead: &Placement) -> Value {
    let (g, track, form) = (&o["g"], &o["track"], &o["form"]);
    let tracks: Vec<Value> = o["tracks"].as_array().cloned().unwrap_or_default();
    let end_dist = js_number(o.get("endDist"));
    let mut names = existing_names(&tracks);
    let identity = identity_of(
        form.get("kind").and_then(Value::as_str).unwrap_or("crossing"),
        o.get("switchName").unwrap_or(&Value::Null),
        form.get("label"),
    );
    let main_mark = switch_element_mark(&identity, "main");
    let split = match ahead.joint {
        Some(j) => Some(split_track_at_joint(track, j, ahead.bearing, &mut names)),
        None => split_element_at(track, ahead.el_idx.unwrap_or(0), &ahead.toe.to_json(), ahead.bearing, &mut names),
    };
    let Some(split) = split else { return Value::Null };
    let carved_ahead = carve_switch_route(
        &split["ahead"],
        split["aheadEndpoint"].as_str().unwrap_or("BEGIN"),
        &g["portC_utm"],
        &main_mark,
        Some(end_dist),
    );
    let carved_behind = carve_switch_route(
        &split["behind"],
        split["behindEndpoint"].as_str().unwrap_or("END"),
        &g["portA_utm"],
        &main_mark,
        Some(end_dist),
    );
    let (Some(ca), Some(cb)) = (carved_ahead, carved_behind) else {
        return json!({ "noRoom": number(end_dist) });
    };
    let built = crossing_elements(g, &identity);
    let fields = o.get("fields").unwrap_or(&Value::Null);
    let track_of = |el: &Value, name: Value| -> Value {
        let mut t = Object::new();
        t.insert("id".into(), json!(generate_id()));
        t.insert("name".into(), name);
        for (k, v) in track_fields(fields) {
            t.insert(k, v);
        }
        t.insert("epsg".into(), track.get("epsg").cloned().unwrap_or(Value::Null));
        t.insert("coordinates".into(), el["geometry"]["coordinates"].clone());
        t.insert("elements".into(), json!([el]));
        Value::Object(t)
    };
    let leg_b = reverse_track(&track_of(&built["B"], Value::Null));
    let leg_d = track_of(&built["D"], o.get("name").cloned().unwrap_or(Value::Null));
    let slips: Vec<Value> = ["slip1", "slip2"]
        .iter()
        .filter_map(|k| built.get(*k).filter(|v| !v.is_null()))
        .map(|el| track_of(el, Value::Null))
        .collect();
    let mut record = identity.as_object().cloned().unwrap_or_default();
    record.insert("number".into(), o.get("switchNumber").cloned().unwrap_or(Value::Null));
    record.insert(
        "ports".into(),
        json!({
            "A": { "trackId": cb.get("id"), "endpoint": split["behindEndpoint"] },
            "B": { "trackId": leg_b.get("id"), "endpoint": "BEGIN" },
            "C": { "trackId": ca.get("id"), "endpoint": split["aheadEndpoint"] },
            "D": { "trackId": leg_d.get("id"), "endpoint": "BEGIN" },
        }),
    );
    record.insert("fillCoords".into(), g["fillCoords"].clone());
    let mut add: Vec<Value> = split["tracks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| {
            if strict_eq(t.get("id"), ca.get("id")) {
                ca.clone()
            } else if strict_eq(t.get("id"), cb.get("id")) {
                cb.clone()
            } else {
                t.clone()
            }
        })
        .collect();
    add.push(leg_b);
    add.push(leg_d);
    add.extend(slips);
    json!({
        "removeTrackIds": [track.get("id")], "addTracks": add, "addSwitches": [Value::Object(record)],
        "remap": [{ "oldId": track.get("id"), "newId": split["tracks"].as_array().into_iter().flatten().map(|t| t.get("id").cloned().unwrap_or(Value::Null)).collect::<Vec<_>>() }],
    })
}

/// What a crossing laid into a track is asked with.
pub struct CrossingAsk<'a> {
    pub track: &'a Value,
    pub end_dist: f64,
    pub form: &'a Value,
    pub cross_angle: f64,
}

/// Where a crossing laid into a track lies (`crossingOnTrackPlacement`).
pub fn crossing_on_track_placement(a: &CrossingAsk, point: Option<f64>) -> Result<Option<(Placement, Placement, Value)>, Value> {
    let Some(point) = point.filter(|p| p.is_finite()) else { return Ok(None) };
    if a.track.is_null() {
        return Ok(None);
    }
    if point < 0.0 || point > track_length(a.track) {
        return Err(err("switch_on_track_outside"));
    }
    let why = |e: &str| err(if e == "switch_on_track_no_room" { "crossing_on_track_no_room" } else { e });
    let ahead = place_switch_on_track(a.track, point, false, a.end_dist).map_err(why)?;
    let back = place_switch_on_track(a.track, point, true, a.end_dist).map_err(why)?;
    let leg_r = crossing_leg_signed_radius(Some(a.form), a.cross_angle);
    if !pieces_on_radius(&ahead.pieces, leg_r) || !pieces_on_radius(&back.pieces, neg_r(leg_r)) {
        return Err(match leg_r {
            None => err("crossing_on_track_straight_only"),
            Some(_) => {
                json!({ "key": "crossing_on_track_leg_arc", "params": { "r": crossing_leg_radius(Some(a.form)).map(rail_across_js::es_number).unwrap_or_else(|| "null".into()) } })
            }
        });
    }
    let geom = compute_crossing_geometry_utm(ahead.toe, ahead.bearing, a.form, a.cross_angle, None, None, None);
    Ok(Some((ahead, back, geom)))
}

/// Where an end of the cross route lies (`crossingOnTrackPort`): (utm, bearing).
pub fn crossing_on_track_port(a: &CrossingAsk, point: f64, port: &str) -> Option<(Utm, f64)> {
    if a.track.is_null() || !point.is_finite() || point < 0.0 || point > track_length(a.track) {
        return None;
    }
    let ahead = place_switch_on_track(a.track, point, false, a.end_dist).ok()?;
    let back = place_switch_on_track(a.track, point, true, a.end_dist).ok()?;
    let leg_r = crossing_leg_signed_radius(Some(a.form), a.cross_angle);
    if !pieces_on_radius(&ahead.pieces, leg_r) || !pieces_on_radius(&back.pieces, neg_r(leg_r)) {
        return None;
    }
    let cross_bearing = (ahead.bearing + a.cross_angle + 360.0) % 360.0;
    let (from, r) = if port == "B" {
        ((cross_bearing + 180.0) % 360.0, neg_r(leg_r))
    } else {
        (cross_bearing, leg_r)
    };
    Some((utm_end_route(ahead.toe, from, a.end_dist, r), bearing_after(from, a.end_dist, r)))
}

/// What the position slider of "Kreuzung auf Gleis" runs over (`crossingOnTrackSlider`).
pub fn crossing_on_track_slider(a: &CrossingAsk, around: Option<f64>) -> Value {
    if els(a.track).is_empty() {
        return json!({ "intervals": [], "snaps": [] });
    }
    let nodes = node_stations(a.track);
    let (from, to) = slider_window(a.track, around);
    let all: Vec<f64> = nodes.iter().flat_map(|n| [n - a.end_dist, n + a.end_dist]).collect();
    let snaps = unique_stations(&all);
    let iv = valid_intervals(&|s| crossing_on_track_port(a, s, "D").is_some(), from, to, 1.0, &snaps);
    intervals_json(&iv, &snaps)
}
