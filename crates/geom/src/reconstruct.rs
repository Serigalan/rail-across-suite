// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The display geometry of elements from their plane data (`elementReconstruct.js`).

use crate::kind::is_transition;
use rail_across_js::{get, has, number, str_of};
use serde_json::{Value, json};

use crate::coords::plane_to_wgs84;
use crate::element::{Utm, arc_coords_from_radius};
use crate::precision::{SAGITTA_ELEMENT, SAGITTA_TRACK};
use crate::transition::compute_clothoid;

fn node(v: &Value, key: &str) -> Option<[f64; 2]> {
    let a = get(v, key)?.as_array()?;
    Some([
        a.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ])
}

fn reconstruct_one(el: &Value, epsg: i64) -> Option<Value> {
    let [se, sn] = node(el, "startNode")?;
    let [ee, en] = node(el, "endNode")?;
    if epsg == 0 {
        return None;
    }
    let s = Utm { e: se, n: sn, zone: epsg };
    let e = Utm { e: ee, n: en, zone: epsg };
    let wgs = |p: Utm| -> Option<Value> {
        let (lng, lat) = plane_to_wgs84(p.e, p.n, p.zone).ok()?;
        Some(json!([number(lng), number(lat)]))
    };
    let start_wgs = wgs(s)?;
    let end_wgs = wgs(e)?;
    let mut out = el.as_object().cloned().unwrap_or_default();
    let radius = el.get("radius").and_then(Value::as_f64).filter(|r| *r != 0.0 && !r.is_nan());
    if let Some(r) = radius {
        let chord = vec![start_wgs.clone(), end_wgs.clone()];
        let coords = arc_coords_from_radius(s, e, r, SAGITTA_ELEMENT).ok()?.unwrap_or_else(|| chord.clone());
        let render = arc_coords_from_radius(s, e, r, SAGITTA_TRACK).ok()?.unwrap_or(chord);
        out.insert("geometry".into(), json!({ "type": "LineString", "coordinates": coords }));
        out.insert("renderCoords".into(), Value::Array(render));
        return Some(Value::Object(out));
    }
    if is_transition(el) && has(el, "r1") {
        let bearing = el.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let r1 = get(el, "r1").and_then(Value::as_f64);
        let r2 = get(el, "r2").and_then(Value::as_f64);
        let bloss = str_of(el, "transitionType") == Some("bloss");
        let cl = compute_clothoid(se, sn, epsg, bearing, length, r1, r2, SAGITTA_ELEMENT, bloss).ok()?;
        let cl_r = compute_clothoid(se, sn, epsg, bearing, length, r1, r2, SAGITTA_TRACK, bloss).ok()?;
        let values = |pts: &[[f64; 2]]| -> Vec<Value> {
            let mut v: Vec<Value> = pts[..pts.len() - 1].iter().map(|[x, y]| json!([number(*x), number(*y)])).collect();
            v.push(end_wgs.clone());
            v
        };
        out.insert("geometry".into(), json!({ "type": "LineString", "coordinates": values(&cl.coords) }));
        out.insert("renderCoords".into(), Value::Array(values(&cl_r.coords)));
        return Some(Value::Object(out));
    }
    out.insert("geometry".into(), json!({ "type": "LineString", "coordinates": [start_wgs, end_wgs] }));
    out.shift_remove("renderCoords");
    Some(Value::Object(out))
}

/// Every element with its display geometry derived from its plane data (`reconstructElements`).
pub fn reconstruct_elements(elements: &[Value], epsg: i64) -> Vec<Value> {
    elements.iter().map(|el| reconstruct_one(el, epsg).unwrap_or_else(|| el.clone())).collect()
}
