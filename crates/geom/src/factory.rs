// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The element factory (`elementFactory.js`, R4.1): every element made from
//! plane points and design scalars, with the display geometry derived from
//! them — `geometry` (SAGITTA_ELEMENT) and, for arcs and transitions,
//! `renderCoords` (SAGITTA_TRACK). `extra` is merged in last.

use rail_across_js::{Object, number};
use serde_json::{Value, json};

use crate::coords::{Unsupported, plane_to_wgs84};
use crate::element::{Utm, arc_coords_from_radius, curved_values, end_point_curved, end_point_straight, straight_values};
use crate::precision::{SAGITTA_ELEMENT, SAGITTA_TRACK};
use crate::transition::compute_clothoid;

fn wgs(p: Utm) -> Result<Value, Unsupported> {
    let (lng, lat) = plane_to_wgs84(p.e, p.n, p.zone)?;
    Ok(json!([number(lng), number(lat)]))
}

fn line(coords: Vec<Value>) -> Value {
    json!({ "type": "LineString", "coordinates": coords })
}

fn with_extra(mut o: Object, extra: &Object) -> Value {
    for (k, v) in extra {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o)
}

/// A straight from `s` to `e` (`straightElement`).
pub fn straight_element(s: Utm, e: Utm, extra: &Object) -> Result<Value, Unsupported> {
    let (length, bearing) = straight_values(s, e);
    let mut o = Object::new();
    o.insert("elementType".into(), json!(0));
    o.insert("startNode".into(), json!([number(s.e), number(s.n)]));
    o.insert("endNode".into(), json!([number(e.e), number(e.n)]));
    o.insert("bearing".into(), number(bearing));
    o.insert("length".into(), number(length));
    o.insert("geometry".into(), line(vec![wgs(s)?, wgs(e)?]));
    Ok(with_extra(o, extra))
}

/// A straight of `length` from `s` at `bearing` (`straightFrom`).
pub fn straight_from(s: Utm, bearing: f64, length: f64, extra: &Object) -> Result<Value, Unsupported> {
    straight_element(s, end_point_straight(s, bearing, length), extra)
}

/// An arc from `s` to `e` with signed radius (`arcElement`).
pub fn arc_element(s: Utm, e: Utm, signed_r: f64, extra: &Object) -> Result<Value, Unsupported> {
    let (length, bearing, end_bearing) = curved_values(s, e, signed_r);
    let chord = vec![wgs(s)?, wgs(e)?];
    let mut o = Object::new();
    o.insert("elementType".into(), json!(1));
    o.insert("startNode".into(), json!([number(s.e), number(s.n)]));
    o.insert("endNode".into(), json!([number(e.e), number(e.n)]));
    o.insert("bearing".into(), number(bearing));
    o.insert("endBearing".into(), number(end_bearing));
    o.insert("length".into(), number(length));
    o.insert("radius".into(), number(signed_r));
    o.insert(
        "geometry".into(),
        line(arc_coords_from_radius(s, e, signed_r, SAGITTA_ELEMENT)?.unwrap_or_else(|| chord.clone())),
    );
    o.insert(
        "renderCoords".into(),
        Value::Array(arc_coords_from_radius(s, e, signed_r, SAGITTA_TRACK)?.unwrap_or(chord)),
    );
    Ok(with_extra(o, extra))
}

/// An arc of `length` from `s` at `bearing` (`arcFrom`).
pub fn arc_from(s: Utm, bearing: f64, length: f64, signed_r: f64, extra: &Object) -> Result<Value, Unsupported> {
    arc_element(s, end_point_curved(s, bearing, length, signed_r), signed_r, extra)
}

/// A transition of `length` from `s` at `bearing` (`transitionElement`): the
/// element, its end and its end bearing. `end_fixed` names an end the curve
/// has to reach exactly.
#[allow(clippy::too_many_arguments)]
pub fn transition_element(
    s: Utm,
    bearing: f64,
    length: f64,
    r1: Option<f64>,
    r2: Option<f64>,
    transition_type: &Value,
    end_fixed: Option<Utm>,
    extra: &Object,
) -> Result<(Value, Utm, f64), Unsupported> {
    let bloss = transition_type.as_str() == Some("bloss");
    let cl = compute_clothoid(s.e, s.n, s.zone, bearing, length, r1, r2, SAGITTA_ELEMENT, bloss)?;
    let cl_r = compute_clothoid(s.e, s.n, s.zone, bearing, length, r1, r2, SAGITTA_TRACK, bloss)?;
    let end = end_fixed.unwrap_or(Utm {
        e: cl.end[0],
        n: cl.end[1],
        zone: s.zone,
    });
    let end_wgs = match end_fixed {
        Some(p) => Some(wgs(p)?),
        None => None,
    };
    let as_values = |pts: &[[f64; 2]]| -> Vec<Value> { pts.iter().map(|[x, y]| json!([number(*x), number(*y)])).collect() };
    let mut geometry = as_values(&cl.coords);
    let mut render = as_values(&cl_r.coords);
    if let Some(w) = &end_wgs {
        geometry.pop();
        geometry.push(w.clone());
        render.pop();
        render.push(w.clone());
    }
    let opt = |r: Option<f64>| r.map(number).unwrap_or(Value::Null);
    let mut o = Object::new();
    o.insert("elementType".into(), json!(2));
    o.insert("transitionType".into(), transition_type.clone());
    o.insert("r1".into(), opt(r1));
    o.insert("r2".into(), opt(r2));
    o.insert("startNode".into(), json!([number(s.e), number(s.n)]));
    o.insert("endNode".into(), json!([number(end.e), number(end.n)]));
    o.insert("bearing".into(), number(bearing));
    o.insert("endBearing".into(), number(cl.end_bearing));
    o.insert("length".into(), number(length));
    o.insert("geometry".into(), line(geometry));
    o.insert("renderCoords".into(), Value::Array(render));
    Ok((with_extra(o, extra), Utm { zone: s.zone, ..end }, cl.end_bearing))
}
