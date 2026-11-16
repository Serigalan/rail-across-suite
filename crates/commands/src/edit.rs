// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Geometry of an element edit: plan, apply and merge length, bearing and
//! radius changes (`editGeometry.js`).

use rail_across_geom::kind::is_transition;
use std::collections::{HashSet, VecDeque};

use rail_across_geom::element::{Utm, node_utm};
use rail_across_geom::factory::{arc_from, straight_from, transition_element};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_js::{Object, get, has, js_number, js_string, number, strict_eq};
use rail_across_switch::delete::switch_parts;
use rail_across_track::heights::truncate_heights;
use rail_across_track::model::rebuild_coords;
use serde_json::{Map, Value, json};

use crate::roles::intermediate_span;

pub const MAX_EDIT_SWITCHES: usize = 1;
pub const MAX_EDIT_TRACKS: usize = 3;

fn node(v: Option<&Value>) -> Option<[f64; 2]> {
    let a = v?.as_array()?;
    Some([a.first()?.as_f64().unwrap_or(f64::NAN), a.get(1)?.as_f64().unwrap_or(f64::NAN)])
}

fn nodes_approx_equal(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (node(a), node(b)) {
        (Some(a), Some(b)) => (a[0] - b[0]).abs() < NODE_TOL && (a[1] - b[1]).abs() < NODE_TOL,
        _ => false,
    }
}

fn norm360(d: f64) -> f64 {
    ((d % 360.0) + 360.0) % 360.0
}

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn shifted_kink(el: &Value, new_bearing: f64) -> Option<f64> {
    if get(el, "radius").is_some() || is_transition(el) {
        return None;
    }
    let eb = f(el, "endBearing")?;
    Some(norm360(new_bearing + (eb - f(el, "bearing").unwrap_or(f64::NAN))))
}

fn merge(el: &Value, over: &Value) -> Value {
    let mut o = el.as_object().cloned().unwrap_or_default();
    for (k, v) in over.as_object().into_iter().flatten() {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o)
}

/// What an element is rebuilt with: the given values, else its own.
#[derive(Default, Clone, Copy)]
pub struct Given {
    pub bearing: Option<f64>,
    pub length: Option<f64>,
    /// Some(None) a straight, Some(Some(r)) an arc; None the element's own.
    pub radius: Option<Option<f64>>,
}

fn build_element(el: &Value, start: Utm, g: Given) -> Value {
    let bearing = g.bearing.unwrap_or_else(|| f(el, "bearing").unwrap_or(f64::NAN));
    let length = g.length.unwrap_or_else(|| js_number(el.get("length")));
    let radius = g.radius.unwrap_or_else(|| f(el, "radius"));
    let none = Object::new();
    if is_transition(el) && has(el, "r1") {
        let tt = el.get("transitionType").cloned().unwrap_or(json!("clothoid"));
        let tt = if tt.is_null() { json!("clothoid") } else { tt };
        let built = transition_element(start, bearing, length, f(el, "r1"), f(el, "r2"), &tt, None, &none)
            .map(|t| t.0)
            .unwrap_or(Value::Null);
        return merge(el, &built);
    }
    if let Some(r) = radius {
        let mut out = merge(el, &arc_from(start, bearing, length, r, &none).unwrap_or(Value::Null));
        out["bearing"] = number(bearing);
        out["length"] = number(length);
        return out;
    }
    let mut out = merge(el, &straight_from(start, bearing, length, &none).unwrap_or(Value::Null));
    out["bearing"] = number(bearing);
    out["length"] = number(length);
    out["radius"] = Value::Null;
    let o = out.as_object_mut().expect("an element");
    match shifted_kink(el, bearing) {
        Some(k) => {
            o.insert("endBearing".into(), number(k));
        }
        None => {
            o.remove("endBearing");
        }
    }
    o.remove("renderCoords");
    out
}

fn same_radius(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() < 1e-6,
        _ => false,
    }
}

fn joined_transition<'a>(el: Option<&'a Value>, key: &str, radius: Option<f64>) -> Option<&'a Value> {
    let el = el?;
    (is_transition(el) && has(el, "r1") && get(el, "switchId").is_none() && same_radius(f(el, key), radius)).then_some(el)
}

fn end_utm_of(el: &Value, epsg: i64) -> Utm {
    let n = node(el.get("endNode")).unwrap_or([f64::NAN, f64::NAN]);
    Utm { e: n[0], n: n[1], zone: epsg }
}

fn end_bearing_of(el: &Value) -> f64 {
    f(el, "endBearing").or_else(|| f(el, "bearing")).unwrap_or(f64::NAN)
}

struct Touched {
    track_ids: Vec<String>,
    switch_ids: Vec<String>,
}

impl Touched {
    fn track(&mut self, id: String) {
        if !self.track_ids.contains(&id) {
            self.track_ids.push(id);
        }
    }
    fn switch(&mut self, id: String) {
        if !self.switch_ids.contains(&id) {
            self.switch_ids.push(id);
        }
    }
}

/// The tracks of an edit, copied on first write.
struct Cow<'a> {
    tracks: &'a [Value],
    written: std::collections::HashMap<usize, Value>,
}

impl<'a> Cow<'a> {
    fn get(&self, i: usize) -> &Value {
        self.written.get(&i).unwrap_or(&self.tracks[i])
    }
    fn get_mut(&mut self, i: usize) -> &mut Value {
        let tracks = self.tracks;
        self.written.entry(i).or_insert_with(|| tracks[i].clone())
    }
}

fn propagate(tracks: &mut Cow, epsg: f64, old_node: Value, from_el: &Value, touched: &mut Touched) {
    let mut queue: VecDeque<(Value, Utm, f64)> = VecDeque::from([(old_node, end_utm_of(from_el, epsg as i64), end_bearing_of(from_el))]);
    while let Some((nd, start, bearing)) = queue.pop_front() {
        for ti in 0..tracks.tracks.len() {
            if js_number(tracks.get(ti).get("epsg")) != epsg {
                continue;
            }
            let tid = js_string(tracks.get(ti).get("id"));
            let n = tracks.get(ti).get("elements").and_then(Value::as_array).map(Vec::len).unwrap_or(0);
            for i in 0..n {
                if !nodes_approx_equal(tracks.get(ti)["elements"][i].get("startNode"), Some(&nd)) {
                    continue;
                }
                let e = tracks.get(ti)["elements"][i].clone();
                let shifted = build_element(
                    &e,
                    start,
                    Given {
                        bearing: Some(bearing),
                        ..Default::default()
                    },
                );
                queue.push_back((
                    e.get("endNode").cloned().unwrap_or(Value::Null),
                    end_utm_of(&shifted, epsg as i64),
                    end_bearing_of(&shifted),
                ));
                tracks.get_mut(ti)["elements"][i] = shifted;
                touched.track(tid.clone());
                if let Some(sid) = e.get("switchId").filter(|v| rail_across_js::truthy(Some(v))) {
                    touched.switch(js_string(Some(sid)));
                }
            }
        }
    }
}

fn finish(tracks: Vec<Value>) -> Vec<Value> {
    tracks
        .into_iter()
        .map(|mut t| {
            let es = t.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
            t["coordinates"] = Value::Array(rebuild_coords(&es));
            t["elements"] = Value::Array(es);
            t
        })
        .collect()
}

fn first_coord(el: &Value) -> Option<&Value> {
    el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0))
}

fn result(tracks: Vec<Value>, touched: &Touched, error: Option<&str>) -> Value {
    json!({ "tracks": tracks, "touchedTrackIds": touched.track_ids, "touchedSwitchIds": touched.switch_ids, "error": error })
}

/// What changing an element would do (`planElementChange`).
pub fn plan_element_change(tracks: &[Value], switches: &[Value], track_id: &Value, el_idx: usize, change: &Value) -> Value {
    // No such element: the tracks as they are, not finished (the JavaScript returns its copy then).
    let found = tracks
        .iter()
        .find(|t| strict_eq(t.get("id"), Some(track_id)))
        .and_then(|t| t["elements"].get(el_idx));
    if found.is_none() {
        return result(
            tracks.to_vec(),
            &Touched {
                track_ids: vec![],
                switch_ids: vec![],
            },
            None,
        );
    }
    let (written, touched, error) = plan_element_change_written(tracks, switches, track_id, el_idx, change);
    let mut all: Vec<Value> = tracks.to_vec();
    for (i, t) in written {
        all[i] = t;
    }
    result(finish(all), &touched, error)
}

/// The tracks an element change writes, by index, finished — what a held project needs back.
pub fn plan_element_change_touched(tracks: &[Value], switches: &[Value], track_id: &Value, el_idx: usize, change: &Value) -> Value {
    let (written, touched, error) = plan_element_change_written(tracks, switches, track_id, el_idx, change);
    let mut indices: Vec<usize> = written.keys().copied().collect();
    indices.sort_unstable();
    let mut map = written;
    let out: Vec<Value> = finish(indices.into_iter().filter_map(|i| map.remove(&i)).collect());
    let out: Vec<Value> = out.into_iter().filter(|t| touched.track_ids.contains(&js_string(t.get("id")))).collect();
    result(out, &touched, error)
}

fn plan_element_change_written(
    tracks: &[Value],
    switches: &[Value],
    track_id: &Value,
    el_idx: usize,
    change: &Value,
) -> (std::collections::HashMap<usize, Value>, Touched, Option<&'static str>) {
    let mut cow = Cow {
        tracks,
        written: std::collections::HashMap::new(),
    };
    let mut touched = Touched {
        track_ids: vec![],
        switch_ids: vec![],
    };
    let Some(ti) = tracks.iter().position(|t| strict_eq(t.get("id"), Some(track_id))) else {
        return (cow.written, touched, None);
    };
    let Some(el) = tracks[ti]["elements"].get(el_idx).cloned() else {
        return (cow.written, touched, None);
    };
    let length = change.get("length").and_then(Value::as_f64);
    let bearing = change.get("bearing").and_then(Value::as_f64);
    let radius: Option<Option<f64>> = if has(change, "radius") {
        Some(change.get("radius").and_then(Value::as_f64))
    } else {
        None
    };
    let epsg = js_number(tracks[ti].get("epsg"));
    let el_radius = f(&el, "radius");
    let follows = radius.is_some_and(|r| !same_radius(r, el_radius)) && !is_transition(&el);
    let elements: &[Value] = tracks[ti]["elements"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let before = if follows {
        el_idx.checked_sub(1).and_then(|i| joined_transition(elements.get(i), "r2", el_radius)).cloned()
    } else {
        None
    };
    let after = if follows {
        joined_transition(elements.get(el_idx + 1), "r1", el_radius).cloned()
    } else {
        None
    };
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg as i64,
    };
    let mut start = node_utm(el.get("startNode"), first_coord(&el), epsg as i64).unwrap_or(nan);
    let mut start_bearing = bearing.unwrap_or_else(|| f(&el, "bearing").unwrap_or(f64::NAN));
    let mut reshaped_before: Option<Value> = None;
    if let Some(b) = &before {
        let own_start = node_utm(b.get("startNode"), first_coord(b), epsg as i64).unwrap_or(nan);
        let mut b2 = b.clone();
        b2["r2"] = radius.flatten().map(number).unwrap_or(Value::Null);
        let rb = build_element(&b2, own_start, Given::default());
        start = end_utm_of(&rb, epsg as i64);
        start_bearing = f(&rb, "endBearing").unwrap_or(f64::NAN);
        reshaped_before = Some(rb);
    }
    let new_el = build_element(
        &el,
        start,
        Given {
            bearing: Some(start_bearing),
            length: Some(length.unwrap_or_else(|| js_number(el.get("length")))),
            radius: Some(radius.unwrap_or(el_radius)),
        },
    );
    let cut_at: f64 = elements
        .iter()
        .take(el_idx)
        .map(|e| get(e, "length").and_then(Value::as_f64).unwrap_or(0.0))
        .sum();
    if new_el.get("length") != el.get("length")
        && let Some(hs) = tracks[ti].get("heights").and_then(Value::as_array).cloned()
    {
        let t = cow.get_mut(ti).as_object_mut().expect("a track");
        match truncate_heights(Some(&hs), cut_at) {
            Some(k) => {
                t.insert("heights".into(), Value::Array(k));
            }
            None => {
                t.remove("heights");
            }
        }
    }
    cow.get_mut(ti)["elements"][el_idx] = new_el.clone();
    if let Some(a) = &after {
        let mut a2 = a.clone();
        a2["r1"] = radius.flatten().map(number).unwrap_or(Value::Null);
        cow.get_mut(ti)["elements"][el_idx + 1] = a2;
    }
    touched.track(js_string(tracks[ti].get("id")));
    if let Some(sid) = el.get("switchId").filter(|v| rail_across_js::truthy(Some(v))) {
        touched.switch(js_string(Some(sid)));
    }
    if let (Some(rb), Some(b)) = (&reshaped_before, &before) {
        cow.get_mut(ti)["elements"][el_idx - 1] = rb.clone();
        propagate(&mut cow, epsg, b.get("endNode").cloned().unwrap_or(Value::Null), rb, &mut touched);
    }
    propagate(&mut cow, epsg, el.get("endNode").cloned().unwrap_or(Value::Null), &new_el, &mut touched);
    // A switch also stands on the tracks its ports name — read off its ports alone.
    let by: Map<String, Value> = Map::new();
    for sw in switches {
        let Some(sid) = sw.get("switchId").filter(|v| rail_across_js::truthy(Some(v))) else {
            continue;
        };
        let sid = js_string(Some(sid));
        if touched.switch_ids.contains(&sid) {
            continue;
        }
        let parts = switch_parts(sw, &by);
        if parts.iter().any(|p| {
            p.track_id
                .as_ref()
                .filter(|v| rail_across_js::truthy(Some(v)))
                .is_some_and(|id| touched.track_ids.contains(&js_string(Some(id))))
        }) {
            touched.switch(sid);
        }
    }
    let error = (touched.switch_ids.len() > MAX_EDIT_SWITCHES || touched.track_ids.len() > MAX_EDIT_TRACKS).then_some("table_edit_too_wide");
    (cow.written, touched, error)
}

fn turn(a: f64, b: f64) -> f64 {
    (((a - b) % 360.0) + 540.0) % 360.0 - 180.0
}

fn solve3(m: &[[f64; 3]; 3], v: [f64; 3]) -> Option<[f64; 3]> {
    let det = |q: &[[f64; 3]; 3]| {
        q[0][0] * (q[1][1] * q[2][2] - q[1][2] * q[2][1]) - q[0][1] * (q[1][0] * q[2][2] - q[1][2] * q[2][0])
            + q[0][2] * (q[1][0] * q[2][1] - q[1][1] * q[2][0])
    };
    let d = det(m);
    if !d.is_finite() || d.abs() < 1e-12 {
        return None;
    }
    let col = |k: usize| {
        let mut q = *m;
        for r in 0..3 {
            q[r][k] = v[r];
        }
        det(&q) / d
    };
    Some([col(0), col(1), col(2)])
}

const BEARING_WEIGHT: f64 = 1000.0;
const SOLVE_TOL: f64 = 1e-6;

fn restationed(heights: &[Value], from: f64, old_to: f64, new_to: f64) -> Vec<Value> {
    let d = new_to - old_to;
    let span = old_to - from;
    let k = (new_to - from) / if span != 0.0 && !span.is_nan() { span } else { 1.0 };
    heights
        .iter()
        .map(|p| {
            let st = js_number(p.get("station"));
            if st <= from {
                return p.clone();
            }
            let mut q = p.clone();
            q["station"] = number(if st >= old_to { st + d } else { from + (st - from) * k });
            q
        })
        .collect()
}

/// What giving a Zwischenelement a radius would do (`planIntermediateChange`).
pub fn plan_intermediate_change(tracks: &[Value], track_id: &Value, z: usize, radius: Option<f64>) -> Value {
    let refuse = |error: &str| json!({ "tracks": tracks, "touchedTrackIds": [], "touchedSwitchIds": [], "error": error });
    let Some(ti) = tracks.iter().position(|t| strict_eq(t.get("id"), Some(track_id))) else {
        return refuse("table_role_intermediate_kind");
    };
    let mut elements: Vec<Value> = tracks[ti].get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    let (a, b) = match intermediate_span(&elements, z) {
        Ok(s) => s,
        Err(e) => return refuse(e),
    };
    if let Some(r) = radius
        && !(r.is_finite() && r != 0.0)
    {
        return refuse("table_intermediate_no_solution");
    }
    let epsg = js_number(tracks[ti].get("epsg")) as i64;
    let old = elements[z].clone();
    let old_r = f(&old, "radius");
    let nan = Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    };
    let start_a = node_utm(elements[a].get("startNode"), first_coord(&elements[a]), epsg).unwrap_or(nan);
    let target = elements[b].clone();
    let target_bearing = end_bearing_of(&target);
    let target_end = node(target.get("endNode")).unwrap_or([f64::NAN, f64::NAN]);
    let chain = |x: &[f64; 3]| -> Vec<Value> {
        let mut out = Vec::new();
        let mut start = start_a;
        let mut bearing = f(&elements[a], "bearing").unwrap_or(f64::NAN);
        for i in a..=b {
            let mut el = elements[i].clone();
            let mut g = Given {
                bearing: Some(bearing),
                ..Default::default()
            };
            if i == a {
                g.length = Some(x[0]);
            }
            if i == z {
                g.length = Some(x[1]);
                g.radius = Some(radius);
            }
            if i == b {
                g.length = Some(x[2]);
            }
            if i + 1 == z && joined_transition(Some(&el), "r2", old_r).is_some() {
                el["r2"] = radius.map(number).unwrap_or(Value::Null);
            }
            if i == z + 1 && joined_transition(Some(&el), "r1", old_r).is_some() {
                el["r1"] = radius.map(number).unwrap_or(Value::Null);
            }
            let built = build_element(&el, start, g);
            start = end_utm_of(&built, epsg);
            bearing = end_bearing_of(&built);
            out.push(built);
        }
        out
    };
    let residual = |x: &[f64; 3]| -> [f64; 3] {
        let built = chain(x);
        let last = built.last().cloned().unwrap_or(Value::Null);
        let e = node(last.get("endNode")).unwrap_or([f64::NAN, f64::NAN]);
        [
            e[0] - target_end[0],
            e[1] - target_end[1],
            turn(end_bearing_of(&last), target_bearing) * std::f64::consts::PI / 180.0 * BEARING_WEIGHT,
        ]
    };
    let mut x = [
        js_number(elements[a].get("length")),
        js_number(old.get("length")),
        js_number(target.get("length")),
    ];
    let mut ok = false;
    for _ in 0..60 {
        let fx = residual(&x);
        if fx[0].abs().max(fx[1].abs()).max(fx[2].abs() / BEARING_WEIGHT * 1000.0) < SOLVE_TOL {
            ok = true;
            break;
        }
        let h = 1e-4;
        let mut jac = [[0.0; 3]; 3];
        for k in 0..3 {
            let mut xs = x;
            xs[k] += h;
            let fk = residual(&xs);
            for r in 0..3 {
                jac[r][k] = (fk[r] - fx[r]) / h;
            }
        }
        let Some(dx) = solve3(&jac, [-fx[0], -fx[1], -fx[2]]) else { break };
        for k in 0..3 {
            x[k] += dx[k];
        }
        if x.iter().any(|v| !(*v > 0.0) || !v.is_finite()) {
            break;
        }
    }
    if !ok || x.iter().any(|v| !(*v > 1e-6)) {
        return refuse("table_intermediate_no_solution");
    }
    let built = chain(&x);
    let sum_to = |es: &[Value], n: usize| es.iter().take(n).map(|e| get(e, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum::<f64>();
    let old_to = sum_to(&elements, b + 1);
    let from = sum_to(&elements, a);
    for (k, el) in built.into_iter().enumerate() {
        elements[a + k] = el;
    }
    let new_to = sum_to(&elements, b + 1);
    elements[b]["endNode"] = target.get("endNode").cloned().unwrap_or(Value::Null);
    let mut new_tracks = tracks.to_vec();
    if let Some(hs) = new_tracks[ti].get("heights").and_then(Value::as_array).cloned() {
        new_tracks[ti]["heights"] = Value::Array(restationed(&hs, from, old_to, new_to));
    }
    new_tracks[ti]["elements"] = Value::Array(elements);
    json!({ "tracks": finish(new_tracks), "touchedTrackIds": [track_id], "touchedSwitchIds": [], "error": null })
}

/// The track array to commit after edits in the element table (`mergeElementEdits`).
pub fn merge_element_edits(store: &[Value], edited: &[Value], changed: &[Value], reshaped: &[Value]) -> Vec<Value> {
    let changed: HashSet<String> = changed.iter().map(|v| js_string(Some(v))).collect();
    let reshaped: HashSet<String> = reshaped.iter().map(|v| js_string(Some(v))).collect();
    store
        .iter()
        .map(|tr| {
            let id = js_string(tr.get("id"));
            let Some(edit) = changed.contains(&id).then(|| edited.iter().find(|e| js_string(e.get("id")) == id)).flatten() else {
                return tr.clone();
            };
            let mut merged = tr.clone();
            merged["elements"] = edit.get("elements").cloned().unwrap_or(Value::Null);
            merged["coordinates"] = get(edit, "coordinates").or_else(|| tr.get("coordinates")).cloned().unwrap_or(Value::Null);
            if !reshaped.contains(&id) {
                return merged;
            }
            let o = merged.as_object_mut().expect("a track");
            match get(edit, "heights") {
                Some(h) => {
                    o.insert("heights".into(), h.clone());
                }
                None => {
                    o.remove("heights");
                }
            }
            merged
        })
        .collect()
}
