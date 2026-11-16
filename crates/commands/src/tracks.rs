// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What the create and connect dialogs commit (`tracks.js`).

use rail_across_geom::kind::is_transition;
use std::collections::HashSet;

use rail_across_geom::element::Utm;
use rail_across_geom::factory::{arc_element, arc_from, straight_element, straight_from, transition_element};
use rail_across_geom::reconstruct::reconstruct_elements;
use rail_across_js::{Object, generate_id, get, js_number, js_string, number, truthy};
use rail_across_rules::cant::cant_sign;
use rail_across_switch::symbol::rebuild_switch_symbol;
use rail_across_track::ids::build_type_fields;
use rail_across_track::model::{port_tracks, rebuild_coords};
use rail_across_track::platforms::rebuild_platform_symbol;
use serde_json::{Value, json};

/// A track's own fields from the dialog's fields and its name (`trackMeta`).
pub fn track_meta(fields: &Value, name: &Value) -> Value {
    let mut o = Object::new();
    o.insert("name".into(), name.clone());
    if let Some(owner) = fields.get("owner") {
        o.insert("owner".into(), owner.clone());
    }
    for (k, v) in build_type_fields(fields).as_object().into_iter().flatten() {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o)
}

/// A track of `elements` in plane `epsg` (`trackOf`).
pub fn track_of(elements: Vec<Value>, epsg: &Value, meta: &Value, id: Option<&Value>) -> Value {
    let mut o = Object::new();
    o.insert("id".into(), id.filter(|v| !v.is_null()).cloned().unwrap_or_else(|| json!(generate_id())));
    for (k, v) in meta.as_object().into_iter().flatten() {
        o.insert(k.clone(), v.clone());
    }
    o.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    o.insert("epsg".into(), epsg.clone());
    o.insert("elements".into(), Value::Array(elements));
    Value::Object(o)
}

fn utm(v: &Value) -> Utm {
    Utm::from_json(v)
}

fn speed_extra(o: &Value) -> Object {
    let mut e = Object::new();
    if let Some(s) = o.get("speed") {
        e.insert("speed".into(), s.clone());
    }
    e
}

/// LineForm: one straight between two points (`buildLineTrack`).
pub fn build_line_track(o: &Value) -> Value {
    let el = straight_element(utm(&o["start"]), utm(&o["end"]), &speed_extra(o)).unwrap_or(Value::Null);
    track_of(vec![el], &o["start"]["zone"], o.get("meta").unwrap_or(&json!({})), o.get("id"))
}

/// The cant an arc of `signed_r` stores (`curveCant`).
pub fn curve_cant(signed_r: f64, cant: Option<&Value>) -> f64 {
    let c = js_number(cant);
    cant_sign(signed_r) * if c != 0.0 && !c.is_nan() { c } else { 0.0 }
}

/// CurvedLineForm: one arc between two points (`buildCurvedLineTrack`).
pub fn build_curved_line_track(o: &Value) -> Value {
    let r = js_number(o.get("signedR"));
    let mut extra = speed_extra(o);
    extra.insert("cant".into(), number(curve_cant(r, o.get("cant"))));
    let el = arc_element(utm(&o["start"]), utm(&o["end"]), r, &extra).unwrap_or(Value::Null);
    track_of(vec![el], &o["start"]["zone"], o.get("meta").unwrap_or(&json!({})), o.get("id"))
}

/// ParallelLineForm: one element offset from a picked one (`buildParallelLineTrack`).
pub fn build_parallel_line_track(o: &Value) -> Value {
    let extra = speed_extra(o);
    let el = match get(o, "signedR").and_then(Value::as_f64) {
        Some(r) => arc_element(utm(&o["start"]), utm(&o["end"]), r, &extra),
        None => straight_element(utm(&o["start"]), utm(&o["end"]), &extra),
    }
    .unwrap_or(Value::Null);
    track_of(vec![el], &o["start"]["zone"], o.get("meta").unwrap_or(&json!({})), o.get("id"))
}

/// ParallelTrackForm: a whole track's elements offset (`buildParallelTrack`).
pub fn build_parallel_track(o: &Value) -> Value {
    track_of(
        o.get("elements").and_then(Value::as_array).cloned().unwrap_or_default(),
        o.get("epsg").unwrap_or(&Value::Null),
        o.get("meta").unwrap_or(&json!({})),
        o.get("id"),
    )
}

fn leading_transition(start: Utm, bearing: f64, transition: Option<&Value>, to_radius: Option<f64>, speed: Option<&Value>) -> Option<(Value, Utm, f64)> {
    let t = transition.filter(|t| truthy(Some(t)))?;
    let length = t.get("length").and_then(Value::as_f64).filter(|l| *l > 0.0)?;
    let tt = get(t, "type").cloned().unwrap_or(json!("clothoid"));
    let mut extra = Object::new();
    if let Some(s) = speed {
        extra.insert("speed".into(), s.clone());
    }
    transition_element(
        start,
        bearing,
        length,
        get(t, "fromRadius").and_then(Value::as_f64),
        to_radius,
        &tt,
        None,
        &extra,
    )
    .ok()
}

/// ConnectStraightForm: a straight appended at a track's end (`buildConnectStraight`).
pub fn build_connect_straight(o: &Value) -> Vec<Value> {
    let (start, bearing) = (utm(&o["start"]), js_number(o.get("bearing")));
    let tr = o.get("transition").filter(|t| truthy(Some(t)));
    let lead = if tr.and_then(|t| get(t, "fromRadius")).is_some() {
        leading_transition(start, bearing, tr, None, o.get("speed"))
    } else {
        None
    };
    let (from, b) = lead.as_ref().map(|l| (l.1, l.2)).unwrap_or((start, bearing));
    let mut out: Vec<Value> = lead.map(|l| vec![l.0]).unwrap_or_default();
    out.push(straight_from(from, b, js_number(o.get("length")), &speed_extra(o)).unwrap_or(Value::Null));
    out
}

/// The cant an arc appended without a transition is proposed with (`cantAtTrackEnd`).
pub fn cant_at_track_end(last: Option<&Value>) -> Option<f64> {
    let Some(el) = last.filter(|v| truthy(Some(v))) else { return Some(0.0) };
    if is_transition(el) {
        return get(el, "cantEnd").and_then(Value::as_f64);
    }
    Some(get(el, "cant").and_then(Value::as_f64).unwrap_or(0.0))
}

/// ConnectCurvedForm: an arc appended at a track's end (`buildConnectCurved`).
pub fn build_connect_curved(o: &Value) -> Vec<Value> {
    let (start, bearing) = (utm(&o["start"]), js_number(o.get("bearing")));
    let r = js_number(o.get("signedR"));
    let lead = leading_transition(start, bearing, o.get("transition"), Some(r), o.get("speed"));
    let (from, b) = lead.as_ref().map(|l| (l.1, l.2)).unwrap_or((start, bearing));
    let arc_length = js_number(o.get("arcLength"));
    let mut extra = speed_extra(o);
    extra.insert("cant".into(), number(curve_cant(r, o.get("cant"))));
    let mut arc = arc_from(from, b, arc_length, r, &extra).unwrap_or(Value::Null);
    arc["bearing"] = number(b);
    arc["length"] = number(arc_length);
    let mut out: Vec<Value> = lead.map(|l| vec![l.0]).unwrap_or_default();
    out.push(arc);
    out
}

/// MoveTrackForm: the project with some tracks shifted (`translateTracks`).
pub fn translate_tracks(project: &Value, track_ids: &[Value], dx: f64, dy: f64) -> Value {
    let ids: HashSet<String> = track_ids.iter().map(|v| js_string(Some(v))).collect();
    if !truthy(Some(project)) || ids.is_empty() || !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
        return project.clone();
    }
    let shift = |n: Option<&Value>| -> Value {
        match n.and_then(Value::as_array) {
            Some(a) => json!([number(a[0].as_f64().unwrap_or(f64::NAN) + dx), number(a[1].as_f64().unwrap_or(f64::NAN) + dy)]),
            None => n.cloned().unwrap_or(Value::Null),
        }
    };
    let mut moved = false;
    let tracks: Vec<Value> = project
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|t| {
            if !ids.contains(&js_string(t.get("id"))) {
                return t.clone();
            }
            moved = true;
            let els: Vec<Value> = t
                .get("elements")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|el| {
                    let mut e = el.clone();
                    e["startNode"] = shift(el.get("startNode"));
                    e["endNode"] = shift(el.get("endNode"));
                    e
                })
                .collect();
            let elements = reconstruct_elements(&els, js_number(t.get("epsg")) as i64);
            let mut out = t.clone();
            out["coordinates"] = Value::Array(rebuild_coords(&elements));
            out["elements"] = Value::Array(elements);
            out
        })
        .collect();
    if !moved {
        return project.clone();
    }
    let by_id: serde_json::Map<String, Value> = tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
    let mut out = project.clone();
    if let Some(sws) = project.get("switches").and_then(Value::as_array) {
        out["switches"] = Value::Array(
            sws.iter()
                .map(|sw| {
                    if port_tracks(sw).iter().any(|id| ids.contains(&js_string(Some(id)))) {
                        rebuild_switch_symbol(sw, &by_id)
                    } else {
                        sw.clone()
                    }
                })
                .collect(),
        );
    }
    if let Some(pfs) = project.get("platforms").and_then(Value::as_array) {
        out["platforms"] = Value::Array(
            pfs.iter()
                .map(|pf| {
                    if ids.contains(&js_string(pf.get("trackId"))) {
                        rebuild_platform_symbol(pf, &tracks)
                    } else {
                        pf.clone()
                    }
                })
                .collect(),
        );
    }
    out["tracks"] = Value::Array(tracks);
    out
}
