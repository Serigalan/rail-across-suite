// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The alignment fit's bookkeeping: the request, the point file, the report
//! and the track built from the answer (`alignmentFit.js`). The fit itself is
//! the optimizer's.

use rail_across_geom::kind::{kind_of, ARC, TRANSITION};
use std::sync::LazyLock;

use rail_across_geom::coords::crs_name;
use rail_across_geom::reconstruct::reconstruct_elements;
use rail_across_js::{generate_id, get, js_number, js_round, js_trim, number, truthy};
use rail_across_track::model::rebuild_coords;
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::axis::survey_points;

/// The fit's settings as the panel shows them (`ALIGN_DEFAULTS`).
pub fn align_defaults() -> Value {
    json!({ "chord": 10, "window": 6, "toleranceMm": 10, "step": 0.5, "spacing": 6, "sagittaMm": 3, "minLength": 20, "heightToleranceMm": 20 })
}

/// The axis points of a survey as the fit reads them (`surveyAxis`).
pub fn survey_axis(survey: &Value) -> Vec<Value> {
    survey_points(survey)
        .iter()
        .map(|p| {
            let f = |k: &str| p.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
            json!({ "station": p["station"], "easting": p["easting"], "northing": p["northing"],
                "z": number(f("zLeft").min(f("zRight"))), "cant": number(js_round(f("cant") * 10000.0) / 10.0) })
        })
        .collect()
}

// JavaScript's `\b` is ASCII's; `(?-u:\b)` is the same here.
static HEADER_NAMES: LazyLock<[Regex; 5]> = LazyLock::new(|| {
    [
        r"(?i)^(station|stat|km|s)(?-u:\b)",
        r"(?i)^(rechtswert|rechts|easting|ost|east|e|y)(?-u:\b)",
        r"(?i)^(hochwert|hoch|northing|nord|north|n|x)(?-u:\b)",
        r"(?i)^(so|sok|höhe|hoehe|height|h|z)(?-u:\b)",
        r"(?i)^(überhöhung|ueberhoehung|cant|superelevation|u)(?-u:\b)",
    ]
    .map(|p| Regex::new(p).expect("header pattern"))
});
static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[a-zäöü]").expect("letter"));
static BLANKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("blanks"));
static BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s").expect("blank"));
static QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^["']|["']$"#).expect("quotes"));
static EPSG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)EPSG[_ -]?(\d{4,5})").expect("epsg"));

/// `Number(String(text).trim().replace(/\s/g, ''))`; an absent cell is `undefined`, NaN.
fn cell_number(text: Option<&String>) -> f64 {
    match text {
        None => f64::NAN,
        Some(t) => js_number(Some(&Value::String(BLANK.replace_all(js_trim(t), "").into_owned()))),
    }
}

/// A point file of axis points (`parseAxisPointFile`): `{ points, epsg }` or `{ error }`.
pub fn parse_axis_point_file(text: &str, file_name: &str) -> Value {
    let lines: Vec<&str> = text
        .split('\n')
        .map(|l| js_trim(l.strip_suffix('\r').unwrap_or(l)))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if lines.is_empty() {
        return json!({ "error": "align_file_empty" });
    }
    let sep = [";", "\t", ","].into_iter().find(|s| lines[0].contains(s));
    let split = |line: &str| -> Vec<String> {
        let cells: Vec<&str> = match sep {
            Some(s) => line.split(s).collect(),
            None => BLANKS.split(line).collect(),
        };
        cells
            .into_iter()
            .map(|c| js_trim(&if sep == Some(";") { c.replacen(',', ".", 1) } else { c.to_string() }).to_string())
            .collect()
    };
    let mut rows: Vec<Vec<String>> = lines.iter().map(|l| split(l)).collect();
    let is_header = rows[0].iter().any(|c| LETTER.is_match(c) && !cell_number(Some(c)).is_finite());
    // (station, easting, northing, height, cant)
    let col: [i64; 5] = if is_header {
        let header = rows.remove(0);
        let find = |re: &Regex| {
            header
                .iter()
                .position(|h| re.is_match(&QUOTES.replace_all(h, "")))
                .map(|i| i as i64)
                .unwrap_or(-1)
        };
        let c = HEADER_NAMES.each_ref().map(find);
        if c[1] < 0 || c[2] < 0 {
            return json!({ "error": "align_file_columns" });
        }
        c
    } else {
        let first = cell_number(rows[0].first());
        let counter = rows.len() > 1 && rows.iter().take(5).enumerate().all(|(i, r)| cell_number(r.first()) == first + i as f64);
        if counter { [-1, 1, 2, 3, -1] } else { [-1, 0, 1, 2, -1] }
    };
    let at = |r: &Vec<String>, c: i64| -> f64 { if c < 0 { f64::NAN } else { cell_number(r.get(c as usize)) } };
    rows.retain(|r| at(r, col[1]).is_finite() && at(r, col[2]).is_finite());
    if rows.len() < 10 {
        return json!({ "error": "align_file_few_points" });
    }
    let mut along = 0.0;
    let mut points = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let (easting, northing) = (at(r, col[1]), at(r, col[2]));
        if i > 0 {
            along += (easting - at(&rows[i - 1], col[1])).hypot(northing - at(&rows[i - 1], col[2]));
        }
        let station = if col[0] >= 0 { at(r, col[0]) } else { along };
        let optional = |c: i64| {
            if c >= 0 && r.get(c as usize).is_some_and(|v| !v.is_empty()) {
                at(r, c)
            } else {
                f64::NAN
            }
        };
        let (z, cant) = (optional(col[3]), optional(col[4]));
        let mut p = Map::new();
        p.insert("station".into(), number(if station.is_finite() { station } else { along }));
        p.insert("easting".into(), number(easting));
        p.insert("northing".into(), number(northing));
        if z.is_finite() {
            p.insert("z".into(), number(z));
        }
        if cant.is_finite() {
            p.insert("cant".into(), number(cant));
        }
        points.push(Value::Object(p));
    }
    let code = EPSG.captures(file_name).and_then(|c| c[1].parse::<i64>().ok());
    let epsg = code.filter(|c| crs_name(*c).is_some()).map(|c| json!(c)).unwrap_or(Value::Null);
    json!({ "points": points, "epsg": epsg })
}

/// The request for the service's `POST /align` (`alignRequest`).
pub fn align_request(points: &[Value], straights: Option<&Value>, settings: Option<&Value>) -> Value {
    let defaults = align_defaults();
    let s = settings.filter(|v| !v.is_null()).unwrap_or(&defaults);
    let n = |k: &str| js_number(s.get(k));
    let finite = |p: &Value, k: &str| p.get(k).and_then(Value::as_f64).filter(|x| x.is_finite());
    json!({
        "points": points.iter().map(|p| {
            let (e, nn) = (p.get("easting").cloned().unwrap_or(Value::Null), p.get("northing").cloned().unwrap_or(Value::Null));
            let z = finite(p, "z");
            if let Some(c) = finite(p, "cant") {
                return json!([e, nn, z.map(number), number(c)]);
            }
            match z { Some(z) => json!([e, nn, number(z)]), None => json!([e, nn]) }
        }).collect::<Vec<_>>(),
        "station0": points.first().and_then(|p| get(p, "station")).cloned().unwrap_or(json!(0)),
        "straights": straights.cloned().unwrap_or(Value::Null),
        "settings": {
            "chord": number(n("chord")), "window": number(n("window")), "step": number(n("step")), "spacing": number(n("spacing")),
            "minLength": number(n("minLength")),
            "tolerance": number(n("toleranceMm") / 1000.0), "sagitta": number(n("sagittaMm") / 1000.0),
            "heightTolerance": number(n("heightToleranceMm") / 1000.0),
        },
    })
}

fn curvature_of(r: Option<&Value>) -> f64 {
    if truthy(r) { 1.0 / js_number(r) } else { 0.0 }
}

fn put(m: &mut Map<String, Value>, k: &str, v: Option<&Value>) {
    if let Some(v) = v {
        m.insert(k.into(), v.clone());
    }
}

fn elements_of(answer: &Value) -> &[Value] {
    answer.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The curvature of the fitted chain element by element (`fittedCurvature`).
pub fn fitted_curvature(answer: &Value) -> Vec<Value> {
    let Some(at) = get(answer, "elementStations") else { return vec![] };
    if !truthy(answer.get("elements")) {
        return vec![];
    }
    elements_of(answer)
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let t = kind_of(el);
            let k1 = match t {
                Some(ARC) => curvature_of(el.get("radius")),
                Some(TRANSITION) => curvature_of(el.get("r1")),
                _ => 0.0,
            };
            let k2 = match t {
                Some(ARC) => k1,
                Some(TRANSITION) => curvature_of(el.get("r2")),
                _ => 0.0,
            };
            let mut m = Map::new();
            put(&mut m, "from", at.get(i));
            put(&mut m, "to", at.get(i + 1));
            m.insert("k1".into(), number(k1));
            m.insert("k2".into(), number(k2));
            Value::Object(m)
        })
        .collect()
}

/// What the report shows per element (`elementReport`).
pub fn element_report(answer: &Value, tolerance: f64) -> Vec<Value> {
    let empty = json!({});
    elements_of(answer)
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let st = answer
                .get("elementStats")
                .and_then(|s| get(s, &i.to_string()).or_else(|| s.get(i)))
                .filter(|v| !v.is_null())
                .unwrap_or(&empty);
            let or_null = |v: Option<&Value>| v.cloned().unwrap_or(Value::Null);
            let mut m = Map::new();
            m.insert("index".into(), json!(i));
            put(&mut m, "elementType", el.get("elementType"));
            put(&mut m, "length", el.get("length"));
            m.insert("radius".into(), or_null(get(el, "radius")));
            m.insert("r1".into(), or_null(get(el, "r1")));
            m.insert("r2".into(), or_null(get(el, "r2")));
            let stations = answer.get("elementStations");
            put(&mut m, "from", stations.and_then(|s| s.get(i)));
            put(&mut m, "to", stations.and_then(|s| s.get(i + 1)));
            m.insert("n".into(), get(st, "n").cloned().unwrap_or(json!(0)));
            m.insert("max".into(), or_null(get(st, "max")));
            m.insert("rms".into(), or_null(get(st, "rms")));
            m.insert("over".into(), json!(get(st, "max").is_some_and(|mx| js_number(Some(mx)) > tolerance)));
            m.insert(
                "cant".into(),
                or_null(answer.get("elementCants").and_then(|c| c.get(i)).and_then(|c| get(c, "cant"))),
            );
            Value::Object(m)
        })
        .collect()
}

/// The gradient of the answer stretch by stretch (`gradientReport`).
pub fn gradient_report(answer: &Value) -> Vec<Value> {
    let heights = answer
        .get("gradient")
        .and_then(|g| get(g, "heights"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    heights
        .windows(2)
        .map(|w| {
            let (a, b) = (&w[0], &w[1]);
            let f = |p: &Value, k: &str| js_number(p.get(k));
            json!({ "from": a.get("station"), "to": b.get("station"),
                "grade": number((f(b, "z") - f(a, "z")) / (f(b, "station") - f(a, "station")) * 1000.0), "end": b })
        })
        .collect()
}

/// The new track the fit is taken over as (`trackFromFit`).
pub fn track_from_fit(answer: &Value, name: Option<&Value>, epsg: Option<&Value>) -> Value {
    let els = elements_of(answer);
    if els.is_empty() {
        return Value::Null;
    }
    let plane = js_number(epsg);
    let input: Vec<Value> = els
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let mut m = el.as_object().cloned().unwrap_or_default();
            m.insert("epsg".into(), number(plane));
            m.insert("speed".into(), json!(0));
            if let Some(c) = answer.get("elementCants").and_then(|c| c.get(i)).filter(|c| truthy(Some(c))) {
                for k in ["cant", "cantStart", "cantEnd"] {
                    if let Some(v) = get(c, k) {
                        m.insert(k.into(), v.clone());
                    }
                }
            }
            Value::Object(m)
        })
        .collect();
    let elements = reconstruct_elements(&input, plane as i64);
    let heights: Option<Vec<Value>> = answer.get("gradient").and_then(|g| get(g, "heights")).and_then(Value::as_array).map(|hs| {
        hs.iter()
            .map(|h| {
                let mut m = Map::new();
                put(&mut m, "station", h.get("station"));
                put(&mut m, "z", h.get("z"));
                if truthy(h.get("rv")) {
                    put(&mut m, "rv", h.get("rv"));
                }
                Value::Object(m)
            })
            .collect()
    });
    let mut out = Map::new();
    out.insert("id".into(), json!(generate_id()));
    out.insert("epsg".into(), number(plane));
    put(&mut out, "name", name);
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    out.insert("elements".into(), Value::Array(elements));
    if let Some(h) = heights.filter(|h| h.len() >= 2) {
        out.insert("heights".into(), Value::Array(h));
    }
    // The JavaScript's order of members: id, epsg, name, elements, coordinates, heights.
    let order = ["id", "epsg", "name", "elements", "coordinates", "heights"];
    Value::Object(order.iter().filter_map(|k| out.remove(*k).map(|v| (k.to_string(), v))).collect())
}

/// Straights as the request takes them (`straightRanges`).
pub fn straight_ranges(straights: &[Value]) -> Vec<Value> {
    straights.iter().map(|s| json!([s.get("from"), s.get("to")])).collect()
}

const MIN_HAND_STRAIGHT: f64 = 2.0;

fn sf(s: &Value, k: &str) -> f64 {
    js_number(s.get(k))
}

/// One end of a straight moved, held off its other end and its neighbours (`moveStraightEnd`).
pub fn move_straight_end(straights: &[Value], i: usize, side: &str, at: f64) -> Vec<Value> {
    let s = &straights[i];
    let prev = i.checked_sub(1).and_then(|p| straights.get(p));
    let next = straights.get(i + 1);
    let value = if side == "from" {
        at.max(prev.map(|p| sf(p, "to") + 1.0).unwrap_or(f64::NEG_INFINITY))
            .min(sf(s, "to") - MIN_HAND_STRAIGHT)
    } else {
        at.min(next.map(|n| sf(n, "from") - 1.0).unwrap_or(f64::INFINITY))
            .max(sf(s, "from") + MIN_HAND_STRAIGHT)
    };
    straights
        .iter()
        .enumerate()
        .map(|(k, x)| {
            if k != i {
                return x.clone();
            }
            let mut m = x.as_object().cloned().unwrap_or_default();
            m.insert(side.into(), number(value));
            Value::Object(m)
        })
        .collect()
}

/// A straight drawn from `a` to `b` added in order (`insertStraight`).
pub fn insert_straight(straights: &[Value], a: f64, b: f64) -> Option<Vec<Value>> {
    let (from, to) = (a.min(b), a.max(b));
    if to - from < MIN_HAND_STRAIGHT {
        return None;
    }
    let apart = |s: &Value| sf(s, "to") < from || sf(s, "from") > to;
    let mut keep: Vec<Value> = straights.iter().filter(|s| apart(s)).cloned().collect();
    let taken: Vec<&Value> = straights.iter().filter(|s| !apart(s)).collect();
    let mf = taken.iter().fold(from, |m, s| m.min(sf(s, "from")));
    let mt = taken.iter().fold(to, |m, s| m.max(sf(s, "to")));
    keep.push(json!({ "from": number(mf), "to": number(mt) }));
    keep.sort_by(|x, y| (sf(x, "from") - sf(y, "from")).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(keep)
}

/// Straight `i` taken out (`deleteStraight`).
pub fn delete_straight(straights: &[Value], i: usize) -> Vec<Value> {
    straights.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, s)| s.clone()).collect()
}
