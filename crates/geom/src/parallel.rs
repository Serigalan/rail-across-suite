// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A whole track offset into a continuous parallel (`parallelUtils.js`).

use rail_across_js::{Object, get, has};
use serde_json::{Value, json};

use crate::coords::Unsupported;
use crate::element::{Utm, node_utm};
use crate::factory::{arc_element, straight_element, transition_element};

const DEG: f64 = std::f64::consts::PI / 180.0;

fn kappa(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 => -1.0 / r,
        _ => 0.0,
    }
}

fn right_perp(bearing: f64) -> (f64, f64) {
    let b = bearing * DEG;
    (b.cos(), -b.sin())
}

fn speed_cant(el: &Value) -> Object {
    let mut o = Object::new();
    for k in ["speed", "cant"] {
        if let Some(v) = el.get(k) {
            o.insert(k.into(), v.clone());
        }
    }
    o
}

/// Every element offset by `dist` metres (positive right of travel), or None
/// where a radius collapses (`offsetTrackElements`).
pub fn offset_track_elements(elements: &[Value], dist: f64, epsg: i64) -> Result<Option<Vec<Value>>, Unsupported> {
    if elements.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    for el in elements {
        let coords: Vec<Value> = el
            .get("geometry")
            .and_then(|g| g.get("coordinates"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let o_start = node_utm(get(el, "startNode"), coords.first(), epsg)?;
        let o_end = node_utm(get(el, "endNode"), coords.last(), epsg)?;
        let sb = el.get("bearing").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let eb = get(el, "endBearing").and_then(Value::as_f64).unwrap_or(sb);
        let ps = right_perp(sb);
        let pe = right_perp(eb);
        let start = Utm {
            e: o_start.e + dist * ps.0,
            n: o_start.n + dist * ps.1,
            zone: epsg,
        };
        let end = Utm {
            e: o_end.e + dist * pe.0,
            n: o_end.n + dist * pe.1,
            zone: epsg,
        };
        if let Some(radius) = get(el, "radius").and_then(Value::as_f64) {
            let signed_r = radius - dist;
            if (if radius >= 0.0 { 1.0 } else { -1.0 }) * signed_r < 0.5 {
                return Ok(None);
            }
            out.push(arc_element(start, end, signed_r, &speed_cant(el))?);
        } else if el.get("elementType").and_then(Value::as_f64) == Some(2.0) {
            let r1o = get(el, "r1").and_then(Value::as_f64);
            let r2o = get(el, "r2").and_then(Value::as_f64);
            let r1 = r1o.map(|r| r - dist);
            let r2 = r2o.map(|r| r - dist);
            if r1.is_some_and(|r| r.abs() < 0.5) || r2.is_some_and(|r| r.abs() < 0.5) {
                return Ok(None);
            }
            let k_sum = kappa(r1o) + kappa(r2o);
            let k_sum2 = kappa(r1) + kappa(r2);
            let length = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
            let length = if k_sum2.abs() < 1e-12 { length } else { length * k_sum / k_sum2 };
            let mut extra = Object::new();
            extra.insert("endBearing".into(), json!(eb));
            for (k, v) in speed_cant(el) {
                extra.insert(k, v);
            }
            if !has(el, "speed") {
                extra.shift_remove("speed");
            }
            let tt = el.get("transitionType").cloned().unwrap_or(json!("clothoid"));
            out.push(transition_element(start, sb, length, r1, r2, &tt, Some(end), &extra)?.0);
        } else {
            out.push(straight_element(start, end, &speed_cant(el))?);
        }
    }
    Ok(Some(out))
}
