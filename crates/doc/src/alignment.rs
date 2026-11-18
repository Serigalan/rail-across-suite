// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A track's plan by column (core 6.0; decision 0006): `horizontalAlignment` instead of a list of elements,
//! as the OSRD export states it. Over N elements `stationing`, `coordinates` and `bearingGon` hold N+1 values —
//! each element's start, the last the track's end —, `elementType`, `beginRadius` / `endRadius` and
//! `beginCant` / `endCant` N, `designSpeed` N where an element has one; what few elements carry (the routes of a
//! switch, a role, reasons) stands by row in `elementDetails`, by the element's index.
//!
//! The columns have no room for a gap or for a kink after an arc or a transition: each element begins where the
//! one before ends, and only a straight may leave in another direction than it runs. Each element is held to
//! its own start: its values lead from there to where the next begins, within [`GAP_TOLERANCE`] — a length
//! rounded to the millimetre is not carried on from element to element. What a record states further apart is
//! refused.

use rail_across_geom::element::{Utm, bearing_after, end_point_curved, end_point_straight};
use rail_across_geom::transition::{sample_transition, transition_bearing_at};
use rail_across_js::{Object, js_round, num, number, str_of};
use serde_json::{Value, json};

/// How far an element may end from where the next begins [m]: by the end it states, or by its values.
pub const GAP_TOLERANCE: f64 = 0.001;
/// How far an arc's or a transition's end bearing may lie from the next element's start [gon], beyond what a
/// length [`GAP_TOLERANCE`] off makes of it on the element's sharpest curvature.
pub const BEARING_TOLERANCE_GON: f64 = 1e-5;

const DEG_PER_GON: f64 = 0.9;
/// Two bearings this close are one [°]: no kink after a straight (persistence's `SAME_END_BEARING`).
const SAME_BEARING: f64 = 1e-7;
/// What the columns state of a row; every other member goes to `elementDetails`.
const STATED: &[&str] = &[
    "elementType",
    "startNode",
    "endNode",
    "bearing",
    "endBearing",
    "length",
    "radius",
    "r1",
    "r2",
    "transitionType",
    "cant",
    "cantStart",
    "cantEnd",
    "designSpeed",
];

fn at(x: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    js_round(x * f) / f
}
fn position(x: f64) -> f64 {
    at(x, rail_across_geom::tolerances::POSITION_DIGITS)
}
fn gon_of(deg: f64) -> f64 {
    at(deg / DEG_PER_GON, 9)
}
fn deg_of(gon: f64) -> f64 {
    at(gon * DEG_PER_GON, 9)
}
fn point(v: Option<&Value>) -> Option<[f64; 2]> {
    let p = v?.as_array()?;
    Some([p.first()?.as_f64()?, p.get(1)?.as_f64()?])
}
/// A point as JSON states it: whole metres as integers, as a parsed document has them.
fn point_value(p: [f64; 2]) -> Value {
    Value::Array(vec![number(p[0]), number(p[1])])
}
fn opt(v: Option<f64>) -> Value {
    v.map_or(Value::Null, number)
}

/// An element's kind in the columns: `straight`, `arc`, `clothoid` or `bloss`.
fn column_type(el: &Value) -> &'static str {
    match str_of(el, "elementType") {
        Some("arc") => "arc",
        Some("transition") if str_of(el, "transitionType") == Some("bloss") => "bloss",
        Some("transition") => "clothoid",
        _ => "straight",
    }
}

/// Where an element of `kind` that leaves `start` on `bearing` [°] ends, and on which bearing.
fn walk(kind: &str, start: [f64; 2], bearing: f64, length: f64, r1: Option<f64>, r2: Option<f64>) -> ([f64; 2], f64) {
    let s = Utm {
        e: start[0],
        n: start[1],
        zone: 0,
    };
    match kind {
        "arc" if r1.is_some_and(|r| r != 0.0) => {
            let r = r1.unwrap_or_default();
            let e = end_point_curved(s, bearing, length, r);
            ([e.e, e.n], bearing_after(bearing, length, Some(r)))
        }
        "clothoid" | "bloss" => {
            let bloss = kind == "bloss";
            let pts = sample_transition(start[0], start[1], bearing, length, r1, r2, bloss, 64, 8, length);
            (pts[pts.len() - 1], transition_bearing_at(bearing, length, r1, r2, bloss, length))
        }
        _ => {
            let e = end_point_straight(s, bearing, length);
            ([e.e, e.n], bearing)
        }
    }
}

fn turn_deg(a: f64, b: f64) -> f64 {
    ((((a - b) % 360.0) + 540.0) % 360.0 - 180.0).abs()
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The radii of a row at its two ends: an arc's one, a transition's two (none: a straight end), a straight's none.
fn radii(el: &Value, kind: &str) -> (Option<f64>, Option<f64>) {
    match kind {
        "arc" => (num(el, "radius"), num(el, "radius")),
        "clothoid" | "bloss" => (num(el, "r1"), num(el, "r2")),
        _ => (None, None),
    }
}

/// The cant of a row at its two ends [mm]: a transition's own two, an arc's or a straight's two where it ramps,
/// else its one; none is 0.
fn cants(el: &Value, kind: &str) -> (f64, f64) {
    let (start, end) = (num(el, "cantStart"), num(el, "cantEnd"));
    match (kind, start, end) {
        ("clothoid" | "bloss", s, e) => (s.unwrap_or(0.0), e.unwrap_or(0.0)),
        (_, Some(s), Some(e)) => (s, e),
        _ => {
            let c = num(el, "cant").unwrap_or(0.0);
            (c, c)
        }
    }
}

/// The columns of a track's rows as a document states them (`designSpeed`); none for no rows. Each element keeps
/// its own start point and bearing, a straight's kink the step to the next bearing; a gap of up to
/// [`GAP_TOLERANCE`] is closed — the element ends where the next begins. Refused, with what lies apart, where a
/// row ends further from where the next begins, or its own values lead further from there, or an arc or a
/// transition ends on another bearing than the next begins on (what [`breaches`] finds of columns).
pub fn to_columns(rows: &[Value]) -> Result<Option<Value>, Vec<String>> {
    if rows.is_empty() {
        return Ok(None);
    }
    let mut apart = Vec::new();
    let (mut stationing, mut coordinates, mut bearings) = (vec![number(0.0)], Vec::new(), Vec::new());
    let (mut types, mut begin_radius, mut end_radius, mut begin_cant, mut end_cant, mut speeds, mut details) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    let mut station = 0.0;
    for (i, el) in rows.iter().enumerate() {
        let kind = column_type(el);
        let (Some(start), Some(end)) = (point(el.get("startNode")), point(el.get("endNode"))) else {
            apart.push(format!("element {i}: lacks its start or end point"));
            continue;
        };
        if let Some(next) = rows.get(i + 1).and_then(|n| point(n.get("startNode")))
            && distance(end, next) > GAP_TOLERANCE
        {
            apart.push(format!("element {i}: ends {:.1} mm from where the next begins", distance(end, next) * 1000.0));
        }
        let bearing = match num(el, "bearing") {
            Some(b) => b,
            None if kind == "straight" => (end[0] - start[0]).atan2(end[1] - start[1]).to_degrees().rem_euclid(360.0),
            None => {
                apart.push(format!("element {i}: an {kind} without its bearing"));
                0.0
            }
        };
        let length = num(el, "length").unwrap_or(0.0);
        let (r1, r2) = radii(el, kind);
        coordinates.push(point_value([position(start[0]), position(start[1])]));
        bearings.push(number(gon_of(bearing)));
        station = position(station + length);
        stationing.push(number(station));
        types.push(json!(kind));
        begin_radius.push(opt(r1));
        end_radius.push(opt(r2));
        let (cs, ce) = cants(el, kind);
        begin_cant.push(number(cs));
        end_cant.push(number(ce));
        speeds.push(el.get("designSpeed").cloned().unwrap_or(Value::Null));
        let rest: Object = el
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, _)| !STATED.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !rest.is_empty() {
            let mut row = Object::new();
            row.insert("element".into(), json!(i));
            row.extend(rest);
            details.push(Value::Object(row));
        }
        if i + 1 == rows.len() {
            // The track's end: where its last element ends, on the bearing it ends on — a straight's own, or the
            // one it states after a kink; an arc's or a transition's from its values.
            coordinates.push(point_value([position(end[0]), position(end[1])]));
            let leaves = match kind {
                "straight" => num(el, "endBearing").filter(|b| turn_deg(*b, bearing) >= SAME_BEARING).unwrap_or(bearing),
                _ => walk(kind, start, deg_of(gon_of(bearing)), length, r1, r2).1,
            };
            bearings.push(number(gon_of(leaves)));
        }
    }
    if !apart.is_empty() {
        return Err(apart);
    }
    let mut out = Object::new();
    out.insert("stationing".into(), Value::Array(stationing));
    out.insert("coordinates".into(), Value::Array(coordinates));
    out.insert("bearingGon".into(), Value::Array(bearings));
    out.insert("elementType".into(), Value::Array(types));
    out.insert("beginRadius".into(), Value::Array(begin_radius));
    out.insert("endRadius".into(), Value::Array(end_radius));
    out.insert("beginCant".into(), Value::Array(begin_cant));
    out.insert("endCant".into(), Value::Array(end_cant));
    if speeds.iter().any(|s| !s.is_null()) {
        out.insert("designSpeed".into(), Value::Array(speeds));
    }
    if !details.is_empty() {
        out.insert("elementDetails".into(), Value::Array(details));
    }
    let out = Value::Object(out);
    let found = breaches(&out);
    if found.is_empty() { Ok(Some(out)) } else { Err(found) }
}

/// The columns of a km line run's rows (core 6.0's KmAlignment): a track's, without its cants, design speeds
/// and details, which a km line does not have.
pub fn to_km_columns(rows: &[Value]) -> Result<Option<Value>, Vec<String>> {
    Ok(to_columns(rows)?.map(|mut columns| {
        if let Some(o) = columns.as_object_mut() {
            for k in ["beginCant", "endCant", "designSpeed", "elementDetails"] {
                o.shift_remove(k);
            }
        }
        columns
    }))
}

fn column<'a>(a: &'a Value, key: &str) -> &'a [Value] {
    a.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}
fn f(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// The rows the columns state, as a document of before stated them (`designSpeed`): each element with its start
/// and end point, bearing [°] and length; a straight's end bearing where it leaves in another direction (a kink);
/// its cant as one `cant` where both ends agree on an arc or a straight — one of 0 on a straight not at all —, else
/// as `cantStart` / `cantEnd`, as a transition always.
pub fn to_rows(alignment: &Value) -> Vec<Value> {
    let stationing = column(alignment, "stationing");
    let coordinates = column(alignment, "coordinates");
    let bearings = column(alignment, "bearingGon");
    let types = column(alignment, "elementType");
    let speeds = alignment.get("designSpeed").and_then(Value::as_array);
    // A km line's run has no cants.
    let with_cant = alignment.get("beginCant").is_some();
    let mut rows: Vec<Object> = Vec::new();
    for (i, kind) in types.iter().enumerate() {
        let kind = kind.as_str().unwrap_or("straight");
        let bearing = f(bearings.get(i)).map(deg_of).unwrap_or(0.0);
        let mut o = Object::new();
        o.insert(
            "elementType".into(),
            json!(if kind == "clothoid" || kind == "bloss" { "transition" } else { kind }),
        );
        o.insert("startNode".into(), coordinates.get(i).cloned().unwrap_or(Value::Null));
        o.insert("endNode".into(), coordinates.get(i + 1).cloned().unwrap_or(Value::Null));
        o.insert("bearing".into(), number(bearing));
        let length = match (f(stationing.get(i)), f(stationing.get(i + 1))) {
            (Some(a), Some(b)) => position(b - a),
            _ => 0.0,
        };
        o.insert("length".into(), number(length));
        let (r1, r2) = (f(column(alignment, "beginRadius").get(i)), f(column(alignment, "endRadius").get(i)));
        match kind {
            "arc" => {
                o.insert("radius".into(), opt(r1));
            }
            "clothoid" | "bloss" => {
                o.insert("r1".into(), opt(r1));
                o.insert("r2".into(), opt(r2));
                o.insert("transitionType".into(), json!(kind));
            }
            _ => {
                if let Some(next) = f(bearings.get(i + 1)).map(deg_of)
                    && turn_deg(next, bearing) >= SAME_BEARING
                {
                    o.insert("endBearing".into(), number(next));
                }
            }
        }
        let (cs, ce) = (
            f(column(alignment, "beginCant").get(i)).unwrap_or(0.0),
            f(column(alignment, "endCant").get(i)).unwrap_or(0.0),
        );
        if with_cant && (kind == "clothoid" || kind == "bloss" || cs != ce) {
            o.insert("cantStart".into(), number(cs));
            o.insert("cantEnd".into(), number(ce));
        } else if with_cant && (cs != 0.0 || kind == "arc") {
            o.insert("cant".into(), number(cs));
        }
        if let Some(s) = speeds.and_then(|s| s.get(i)).filter(|s| !s.is_null()) {
            o.insert("designSpeed".into(), s.clone());
        }
        rows.push(o);
    }
    for d in column(alignment, "elementDetails") {
        let Some(row) = d.get("element").and_then(Value::as_u64).and_then(|i| rows.get_mut(i as usize)) else {
            continue;
        };
        for (k, v) in d.as_object().into_iter().flatten().filter(|(k, _)| *k != "element") {
            row.insert(k.clone(), v.clone());
        }
    }
    rows.into_iter().map(Value::Object).collect()
}

/// What the columns of a document's track break that its schema cannot say, as "what" lines: columns of the
/// wrong length, stations that do not run on from 0, an arc without its one radius, a straight with one, an
/// element that does not end where the next begins, an arc or a transition whose end bearing is not the next
/// one's start.
pub fn breaches(alignment: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let n = column(alignment, "elementType").len();
    for key in ["stationing", "coordinates", "bearingGon"] {
        if column(alignment, key).len() != n + 1 {
            out.push(format!("{key}: holds {} values, not {}", column(alignment, key).len(), n + 1));
        }
    }
    for key in ["beginRadius", "endRadius", "beginCant", "endCant", "designSpeed"] {
        let optional = matches!(key, "beginCant" | "endCant" | "designSpeed");
        if (!optional || alignment.get(key).is_some()) && column(alignment, key).len() != n {
            out.push(format!("{key}: holds {} values, not {n}", column(alignment, key).len()));
        }
    }
    for d in column(alignment, "elementDetails") {
        if d.get("element").and_then(Value::as_u64).is_none_or(|i| i as usize >= n) {
            out.push(format!(
                "elementDetails: names element {}, which is not there",
                rail_across_js::js_string(d.get("element"))
            ));
        }
    }
    if !out.is_empty() {
        return out;
    }
    let stationing = column(alignment, "stationing");
    if f(stationing.first()) != Some(0.0) {
        out.push("stationing: does not begin with 0".into());
    }
    let bearings = column(alignment, "bearingGon");
    for i in 0..n {
        let kind = column(alignment, "elementType")[i].as_str().unwrap_or("");
        let (r1, r2) = (f(column(alignment, "beginRadius").get(i)), f(column(alignment, "endRadius").get(i)));
        match kind {
            "arc" if r1.is_none_or(|r| r == 0.0) || r1 != r2 => out.push(format!("element {i}: an arc without its one radius")),
            "straight" if r1.is_some() || r2.is_some() => out.push(format!("element {i}: a straight with a radius")),
            _ => {}
        }
        let (Some(a), Some(b)) = (f(stationing.get(i)), f(stationing.get(i + 1))) else {
            continue;
        };
        if b < a {
            out.push(format!("element {i}: stationing runs back"));
            continue;
        }
        let (Some(start), Some(end), Some(bearing), Some(next)) = (
            point(alignment["coordinates"].get(i)),
            point(alignment["coordinates"].get(i + 1)),
            f(bearings.get(i)),
            f(bearings.get(i + 1)),
        ) else {
            out.push(format!("element {i}: lacks its points or bearings"));
            continue;
        };
        let (reached, end_bearing) = walk(kind, start, deg_of(bearing), b - a, r1, r2);
        let d = distance(reached, end);
        if d > GAP_TOLERANCE {
            out.push(format!("element {i}: ends {:.1} mm from where the next begins", d * 1000.0));
        }
        let sharpest = [r1, r2].into_iter().flatten().filter(|r| *r != 0.0).map(|r| 1.0 / r.abs()).fold(0.0, f64::max);
        let tolerance = BEARING_TOLERANCE_GON + (GAP_TOLERANCE * sharpest).to_degrees() / DEG_PER_GON;
        if kind != "straight" && turn_deg(end_bearing, deg_of(next)) / DEG_PER_GON > tolerance {
            out.push(format!("element {i}: a kink after an {kind}"));
        }
    }
    out
}
