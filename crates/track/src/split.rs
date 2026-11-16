// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Parting a track: at a joint, inside an element, and carving a turnout's
//! route out of it (`trackSplitUtils.js`).

use rail_across_geom::kind::is_transition;
use std::collections::HashSet;

use rail_across_geom::cant::{transition_cant_ends, with_cant_ramp};
use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, arc_coords_from_radius, curved_values, node_utm, straight_values};
use rail_across_geom::precision::{SAGITTA_ELEMENT, SAGITTA_TRACK};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{clothoid_radius_at, compute_clothoid, transition_bearing_at};
use rail_across_js::{generate_id, get, has, js_string, number, truthy};
use serde_json::{Value, json};

use crate::heights::split_heights;
use crate::model::{next_track_name, rebuild_coords};
use crate::util::{elements, epsg, list, num, obj, text, without};

const DEG2RAD: f64 = std::f64::consts::PI / 180.0;

fn make_split_track(src: &Value, els: Vec<Value>, id: String, name: String, heights: Option<Vec<Value>>) -> Value {
    let mut out = without(src, &["elements", "coordinates", "id", "name", "heights"]);
    out.insert("id".into(), json!(id));
    out.insert("name".into(), json!(name));
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&els)));
    out.insert("elements".into(), Value::Array(els));
    if let Some(h) = heights.filter(|h| !h.is_empty()) {
        out.insert("heights".into(), Value::Array(h));
    }
    Value::Object(out)
}

fn wgs(p: Utm) -> Value {
    let (lng, lat) = plane_to_wgs84(p.e, p.n, p.zone).unwrap_or((f64::NAN, f64::NAN));
    json!([number(lng), number(lat)])
}

/// The two halves of an element cut at `cut`, and their values.
pub struct Halves {
    pub cut_wgs: Value,
    pub a: Value,
    pub b: Value,
    pub a_length: f64,
    pub a_bearing: f64,
    pub a_end_bearing: Option<f64>,
    pub b_length: f64,
    pub b_bearing: f64,
}

/// An element cut at a plane point, a transition at its station (`splitElement`).
pub fn split_element(el: &Value, epsg: i64, cut: &Value, ramp: Option<(f64, f64)>) -> Option<Halves> {
    if is_transition(el) && cut.get("station").and_then(Value::as_f64).is_some_and(f64::is_finite) {
        return split_clothoid(el, epsg, cut, ramp);
    }
    let radius = num(el, "radius").filter(|r| *r != 0.0);
    let coords = el.get("geometry")?.get("coordinates")?.as_array()?;
    let (g_start, g_end) = (coords.first()?.clone(), coords.last()?.clone());
    let s = node_utm(el.get("startNode"), Some(&g_start), epsg).ok()?;
    let e = node_utm(el.get("endNode"), Some(&g_end), epsg).ok()?;
    let j = Utm {
        e: cut["easting"].as_f64().unwrap_or(f64::NAN),
        n: cut["northing"].as_f64().unwrap_or(f64::NAN),
        zone: epsg,
    };
    let cut_wgs = wgs(j);
    let values = |a: Utm, b: Utm| -> (f64, f64, Option<f64>) {
        match radius {
            Some(r) => {
                let (l, b0, eb) = curved_values(a, b, r);
                (l, b0, Some(eb))
            }
            None => {
                let (l, b0) = straight_values(a, b);
                (l, b0, None)
            }
        }
    };
    let half_coords = |a: Utm, b: Utm, sw: &Value, ew: &Value| -> Vec<Value> {
        let Some(r) = radius else { return vec![sw.clone(), ew.clone()] };
        match arc_coords_from_radius(a, b, r, SAGITTA_ELEMENT).ok().flatten() {
            Some(c) if c.len() >= 2 => {
                let mut v = vec![sw.clone()];
                v.extend(c[1..c.len() - 1].iter().cloned());
                v.push(ew.clone());
                v
            }
            _ => vec![sw.clone(), ew.clone()],
        }
    };
    let mk = |a: Utm, b: Utm, v: (f64, f64, Option<f64>), c: Vec<Value>| -> Value {
        let mut o = without(el, &["epsg"]);
        o.insert("startNode".into(), json!([number(a.e), number(a.n)]));
        o.insert("endNode".into(), json!([number(b.e), number(b.n)]));
        o.insert("bearing".into(), number(v.1));
        o.insert("length".into(), number(v.0));
        if let Some(eb) = v.2 {
            o.insert("endBearing".into(), number(eb));
        }
        o.insert("geometry".into(), json!({ "type": "LineString", "coordinates": c }));
        o.shift_remove("renderCoords");
        Value::Object(o)
    };
    let va = values(s, j);
    let vb = values(j, e);
    let mut a = mk(s, j, va, half_coords(s, j, &g_start, &cut_wgs));
    let mut b = mk(j, e, vb, half_coords(j, e, &cut_wgs, &g_end));
    if get(el, "cantStart").is_some() || get(el, "cantEnd").is_some() {
        let c = num(el, "cant");
        let u1 = num(el, "cantStart").or(c).unwrap_or(0.0);
        let u2 = num(el, "cantEnd").or(c).unwrap_or(0.0);
        let length = num(el, "length").unwrap_or(f64::NAN);
        let u_cut = if length > 0.0 { u1 + (u2 - u1) * va.0 / length } else { u1 };
        a = with_cant_ramp(&a, u1, u_cut);
        b = with_cant_ramp(&b, u_cut, u2);
    }
    Some(Halves {
        cut_wgs,
        a,
        b,
        a_length: va.0,
        a_bearing: va.1,
        a_end_bearing: va.2,
        b_length: vb.0,
        b_bearing: vb.1,
    })
}

fn split_clothoid(el: &Value, epsg: i64, cut: &Value, ramp: Option<(f64, f64)>) -> Option<Halves> {
    if text(el, "transitionType") == Some("bloss") || !has(el, "r1") {
        return None;
    }
    let l = num(el, "length").unwrap_or(f64::NAN);
    let (r1, r2) = (num(el, "r1"), num(el, "r2"));
    let s = l.min(0f64.max(cut["station"].as_f64().unwrap_or(f64::NAN)));
    let coords = el.get("geometry")?.get("coordinates")?.as_array()?;
    let (g_start, g_end) = (coords.first()?.clone(), coords.last()?.clone());
    let su = node_utm(el.get("startNode"), Some(&g_start), epsg).ok()?;
    let eu = node_utm(el.get("endNode"), Some(&g_end), epsg).ok()?;
    let j = Utm {
        e: cut["easting"].as_f64().unwrap_or(f64::NAN),
        n: cut["northing"].as_f64().unwrap_or(f64::NAN),
        zone: epsg,
    };
    let cut_wgs = wgs(j);
    let bearing = num(el, "bearing").unwrap_or(f64::NAN);
    let r_cut = clothoid_radius_at(r1, r2, l, s);
    let b_cut = transition_bearing_at(bearing, l, r1, r2, false, s);
    let b_end = num(el, "endBearing").unwrap_or_else(|| transition_bearing_at(bearing, l, r1, r2, false, l));
    let u_cut = ramp.filter(|_| l > 0.0).map(|(a, b)| a + (b - a) * s / l);
    #[allow(clippy::too_many_arguments)]
    let piece = |from: Utm,
                 from_wgs: &Value,
                 to_node: [f64; 2],
                 to_wgs: &Value,
                 b0: f64,
                 eb: f64,
                 length: f64,
                 ra: Option<f64>,
                 rb: Option<f64>,
                 ua: Option<f64>,
                 ub: Option<f64>|
     -> Value {
        let line = |sagitta: f64| -> Vec<Value> {
            if length > 0.0 {
                let mut v = vec![from_wgs.clone()];
                if let Ok(c) = compute_clothoid(from.e, from.n, epsg, b0, length, ra, rb, sagitta, false) {
                    v.extend(c.coords[1..c.coords.len() - 1].iter().map(|[a, b]| json!([number(*a), number(*b)])));
                }
                v.push(to_wgs.clone());
                v
            } else {
                vec![from_wgs.clone(), to_wgs.clone()]
            }
        };
        let opt = |r: Option<f64>| r.map(number).unwrap_or(Value::Null);
        let mut o = without(el, &["epsg"]);
        o.insert("r1".into(), opt(ra));
        o.insert("r2".into(), opt(rb));
        o.insert("startNode".into(), json!([number(from.e), number(from.n)]));
        o.insert("endNode".into(), json!([number(to_node[0]), number(to_node[1])]));
        o.insert("bearing".into(), number(b0));
        o.insert("endBearing".into(), number(eb));
        o.insert("length".into(), number(length));
        if ramp.is_some() {
            o.insert("cantStart".into(), opt(ua));
            o.insert("cantEnd".into(), opt(ub));
        }
        o.insert("geometry".into(), json!({ "type": "LineString", "coordinates": line(SAGITTA_ELEMENT) }));
        o.insert("renderCoords".into(), Value::Array(line(SAGITTA_TRACK)));
        Value::Object(o)
    };
    let a = piece(su, &g_start, [j.e, j.n], &cut_wgs, bearing, b_cut, s, r1, r_cut, ramp.map(|r| r.0), u_cut);
    let b = piece(j, &cut_wgs, [eu.e, eu.n], &g_end, b_cut, b_end, l - s, r_cut, r2, u_cut, ramp.map(|r| r.1));
    Some(Halves {
        cut_wgs,
        a,
        b,
        a_length: s,
        a_bearing: bearing,
        a_end_bearing: Some(b_cut),
        b_length: l - s,
        b_bearing: b_cut,
    })
}

fn marked(el: &Value, mark: &Value) -> Value {
    let mut o = obj(el);
    for (k, v) in obj(mark) {
        o.insert(k, v);
    }
    Value::Object(o)
}

fn length(el: &Value) -> f64 {
    num(el, "length").unwrap_or(f64::NAN)
}

/// A turnout's route given elements of its own at a track end (`carveSwitchRoute`).
pub fn carve_switch_route(track: &Value, endpoint: &str, cut: &Value, mark: &Value, route_length: Option<f64>) -> Option<Value> {
    let Some(route_length) = route_length else {
        return carve_terminal(track, endpoint, cut, mark);
    };
    let els = elements(track);
    let n = els.len();
    let mut covered: HashSet<usize> = HashSet::new();
    let mut total = 0.0;
    for k in 0..n {
        let idx = if endpoint == "BEGIN" { k } else { n - 1 - k };
        let el = &els[idx];
        if !truthy(el.get("geometry")) {
            return None;
        }
        if is_transition(el) && (text(el, "transitionType") == Some("bloss") || !has(el, "r1")) {
            return None;
        }
        let remaining = route_length - total;
        if length(el) < remaining - NODE_TOL {
            covered.insert(idx);
            total += length(el);
            continue;
        }
        let replacement = if length(el) - remaining <= NODE_TOL {
            vec![marked(el, mark)]
        } else {
            let station = if endpoint == "BEGIN" { remaining } else { length(el) - remaining };
            let ramp = if is_transition(el) { Some(transition_cant_ends(els, idx)) } else { None };
            let c = if is_transition(el) {
                let mut c = obj(cut);
                c.insert("station".into(), number(station));
                Value::Object(c)
            } else {
                cut.clone()
            };
            let h = split_element(el, epsg(track), &c, ramp)?;
            let (route, rest) = if endpoint == "BEGIN" { (h.a, h.b) } else { (h.b, h.a) };
            if !(length(&route) > NODE_TOL) {
                return None;
            }
            if length(&rest) <= NODE_TOL {
                vec![marked(el, mark)]
            } else if endpoint == "BEGIN" {
                vec![marked(&route, mark), rest]
            } else {
                vec![rest, marked(&route, mark)]
            }
        };
        let mut out = Vec::new();
        for (i, e) in els.iter().enumerate() {
            if i == idx {
                out.extend(replacement.iter().cloned());
            } else if covered.contains(&i) {
                out.push(marked(e, mark));
            } else {
                out.push(e.clone());
            }
        }
        let mut t = obj(track);
        t.insert("coordinates".into(), Value::Array(rebuild_coords(&out)));
        t.insert("elements".into(), Value::Array(out));
        return Some(Value::Object(t));
    }
    None
}

fn carve_terminal(track: &Value, endpoint: &str, cut: &Value, mark: &Value) -> Option<Value> {
    let els = elements(track);
    let idx = if endpoint == "BEGIN" { 0 } else { els.len().checked_sub(1)? };
    let el = els.get(idx)?;
    if is_transition(el) || !truthy(el.get("geometry")) {
        return None;
    }
    let h = split_element(el, epsg(track), cut, None)?;
    let (route, rest) = if endpoint == "BEGIN" { (h.a, h.b) } else { (h.b, h.a) };
    if !(length(&route) > NODE_TOL) {
        return None;
    }
    let halves = if length(&rest) <= NODE_TOL {
        vec![marked(el, mark)]
    } else if endpoint == "BEGIN" {
        vec![marked(&route, mark), rest]
    } else {
        vec![rest, marked(&route, mark)]
    };
    let mut out = els[..idx].to_vec();
    out.extend(halves);
    out.extend_from_slice(&els[idx + 1..]);
    let mut t = obj(track);
    t.insert("coordinates".into(), Value::Array(rebuild_coords(&out)));
    t.insert("elements".into(), Value::Array(out));
    Some(Value::Object(t))
}

fn names(track: &Value, existing: &mut Vec<String>) -> (String, String) {
    let prefix = text(track, "name")
        .map(|n| n.split('.').next().unwrap_or("").to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "track".into());
    let a = next_track_name(&prefix, existing);
    existing.push(a.clone());
    let b = next_track_name(&prefix, existing);
    existing.push(b.clone());
    (a, b)
}

fn split_result(track_a: Value, track_b: Value, junction_wgs: Value, ahead_is_b: bool) -> Value {
    json!({
        "tracks": [track_a, track_b],
        "junctionWgs": junction_wgs,
        "ahead": if ahead_is_b { &track_b } else { &track_a },
        "aheadEndpoint": if ahead_is_b { "BEGIN" } else { "END" },
        "behind": if ahead_is_b { &track_a } else { &track_b },
        "behindEndpoint": if ahead_is_b { "END" } else { "BEGIN" },
    })
}

/// A track parted at the joint before element `j` (`splitTrackAtJoint`).
pub fn split_track_at_joint(track: &Value, j: usize, bearing: f64, existing: &mut Vec<String>) -> Value {
    let els = elements(track);
    let keep_ramp = |el: &Value, i: usize| -> Value {
        if !is_transition(el) {
            return el.clone();
        }
        let (s, e) = transition_cant_ends(els, i);
        let mut o = obj(el);
        o.insert("cantStart".into(), number(s));
        o.insert("cantEnd".into(), number(e));
        Value::Object(o)
    };
    let before: Vec<Value> = els[..j]
        .iter()
        .enumerate()
        .map(|(i, el)| if i + 1 == j { keep_ramp(el, i) } else { el.clone() })
        .collect();
    let after: Vec<Value> = els[j..]
        .iter()
        .enumerate()
        .map(|(i, el)| if i == 0 { keep_ramp(el, j) } else { el.clone() })
        .collect();
    let cut_at: f64 = before.iter().map(|e| num(e, "length").unwrap_or(0.0)).sum();
    let (ha, hb) = split_heights(get(track, "heights").map(|_| list(track, "heights")), cut_at);
    let (na, nb) = names(track, existing);
    let junction = after
        .first()
        .and_then(|e| e.get("geometry"))
        .and_then(|g| g.get("coordinates"))
        .and_then(|c| c.get(0))
        .cloned()
        .unwrap_or(Value::Null);
    let b0 = after.first().and_then(|e| num(e, "bearing")).unwrap_or(f64::NAN);
    let track_a = make_split_track(track, before, generate_id(), na, ha);
    let track_b = make_split_track(track, after, generate_id(), nb, hb);
    split_result(track_a, track_b, junction, ((bearing - b0) * DEG2RAD).cos() > 0.0)
}

/// A track parted inside element `el_idx` at `junction` (`splitElementAt`).
pub fn split_element_at(track: &Value, el_idx: usize, junction: &Value, bearing: f64, existing: &mut Vec<String>) -> Option<Value> {
    let els = elements(track);
    let el = els.get(el_idx)?;
    let ramp = if is_transition(el) && junction.get("station").and_then(Value::as_f64).is_some_and(f64::is_finite) {
        Some(transition_cant_ends(els, el_idx))
    } else {
        None
    };
    let h = split_element(el, epsg(track), junction, ramp)?;
    let before = &els[..el_idx];
    let after = &els[el_idx + 1..];
    let cut_at = before.iter().map(|e| num(e, "length").unwrap_or(0.0)).sum::<f64>() + h.a_length;
    let (ha, hb) = split_heights(get(track, "heights").map(|_| list(track, "heights")), cut_at);
    let (na, nb) = names(track, existing);
    let mut first = before.to_vec();
    first.push(h.a.clone());
    let mut second = vec![h.b.clone()];
    second.extend_from_slice(after);
    let track_a = make_split_track(track, first, generate_id(), na, ha);
    let track_b = make_split_track(track, second, generate_id(), nb, hb);
    let j_bearing = if h.b_length > 1e-9 {
        h.b_bearing
    } else {
        h.a_end_bearing.unwrap_or(h.a_bearing)
    };
    let _ = js_string;
    Some(split_result(track_a, track_b, h.cut_wgs, ((bearing - j_bearing) * DEG2RAD).cos() > 0.0))
}
