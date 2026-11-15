// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Ids, track types and switch numbers (`identifierUtils.js`, `heightDatums.js`).

use std::collections::HashSet;

use rail_across_js::{get, js_number, js_string, js_trim, number, truthy};
use regex::Regex;
use serde_json::{Value, json};

use crate::util::text;

pub use rail_across_js::generate_id;

/// Vertical datum heights are stated in unless the track says otherwise: DHHN2016.
pub const DEFAULT_HEIGHT_EPSG: i64 = 7837;
/// Height points: an element up to this long carries one at each end [m].
pub const HEIGHT_SPLIT_MIN: f64 = 200.0;
/// … a longer one evenly spaced points at most this far apart [m].
pub const HEIGHT_POINT_SPACING: f64 = 100.0;

/// The vertical datums heights may be stated in.
pub const HEIGHT_DATUMS: &[(i64, &str)] = &[
    (7837, "DHHN2016"),
    (5783, "DHHN92"),
    (7699, "DHHN12"),
    (5784, "DHHN85"),
    (5785, "SNN76 (HN 76)"),
    (5773, "EGM96"),
    (3855, "EGM2008"),
];

/// A height datum as the dialogs name it (`heightDatumLabel`).
pub fn height_datum_label(epsg: Option<&Value>) -> String {
    let code = epsg.map(|v| js_string(Some(v))).unwrap_or_else(|| DEFAULT_HEIGHT_EPSG.to_string());
    match HEIGHT_DATUMS.iter().find(|(e, _)| e.to_string() == code) {
        Some((_, label)) => format!("EPSG {code} – {label}"),
        None => format!("EPSG {code}"),
    }
}

/// The type and the fields of it a track form gives (`buildTypeFields`).
pub fn build_type_fields(fields: &Value) -> Value {
    let present = |k: &str| get(fields, k).filter(|v| !js_trim(&js_string(Some(v))).is_empty());
    let s = |k: &str| present(k).map(|v| json!(js_trim(&js_string(Some(v))))).unwrap_or(Value::Null);
    let n = |k: &str| present(k).map(|v| number(js_number(Some(v)))).unwrap_or(Value::Null);
    let height = n("heightEpsg")
        .as_f64()
        .filter(|x| *x != 0.0 && !x.is_nan())
        .map(number)
        .unwrap_or(json!(DEFAULT_HEIGHT_EPSG));
    let siding = text(fields, "trackUse") == Some("siding");
    let mut o = rail_across_js::Object::new();
    if text(fields, "type") == Some("line_track") {
        o.insert("trackType".into(), json!("line"));
        o.insert("lineNumber".into(), n("lineNumber"));
        o.insert("lineName".into(), s("lineName"));
        let side = if truthy(fields.get("side")) {
            match fields.get("side").and_then(Value::as_str) {
                Some("sorting") => json!("regular"),
                Some("non_sorting") => json!("opposite"),
                _ => Value::Null,
            }
        } else {
            Value::Null
        };
        o.insert("side".into(), side);
    } else {
        o.insert("trackType".into(), json!("station"));
        o.insert("stationName".into(), s("stationName"));
        o.insert("uicStation".into(), s("uicStation"));
        o.insert("trackNumber".into(), n("trackNumber"));
    }
    if siding {
        o.insert("trackUse".into(), json!("siding"));
    }
    o.insert("heightEpsg".into(), height);
    Value::Object(o)
}

/// `switch.008`, or `crossing.008` for a crossing kind (`switchDesignation`).
pub fn switch_designation(number: &Value, kind: Option<&str>) -> String {
    let prefix = if kind.is_some_and(|k| !k.is_empty() && k != "turnout") {
        "crossing"
    } else {
        "switch"
    };
    format!("{prefix}.{:0>3}", js_string(Some(number)))
}

/// The number a switch carries, else the one its designation states (`switchNumberOf`).
pub fn switch_number_of(sw: &Value) -> Option<f64> {
    if let Some(n) = get(sw, "number").and_then(Value::as_f64).filter(|n| n.is_finite()) {
        return Some(n);
    }
    let re = Regex::new(r"^(?:switch|crossing)\.(\d+)$").expect("pattern");
    let name = text(sw, "name").unwrap_or("");
    re.captures(name).and_then(|c| c[1].parse::<f64>().ok())
}

/// The numbers a project has given out (`switchNumbersInUse`).
pub fn switch_numbers_in_use(switches: &[Value]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for n in switches.iter().filter_map(switch_number_of) {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

/// The lowest number no switch holds yet (`nextSwitchNumber`).
pub fn next_switch_number(switches: &[Value], also_taken: &[f64]) -> f64 {
    let mut used: HashSet<u64> = switch_numbers_in_use(switches).iter().map(|n| n.to_bits()).collect();
    for n in also_taken {
        used.insert(n.to_bits());
    }
    for i in 1..=999 {
        if !used.contains(&f64::from(i).to_bits()) {
            return f64::from(i);
        }
    }
    used.len() as f64 + 1.0
}
