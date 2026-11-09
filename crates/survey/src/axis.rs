// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A measured axis, kept in whole millimetres from its first point (`axisSurvey.js`).

use rail_across_js::{js_number, js_round, number, truthy};
use serde_json::{Value, json};

fn mm(v: f64) -> Value {
    number(js_round(v * 1000.0))
}

fn f(p: &Value, k: &str) -> f64 {
    p.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// A survey from a trace of the point cloud (`surveyFromTrace`).
pub fn survey_from_trace(meta: &Value, trace: &Value) -> Value {
    let points = trace.get("points").and_then(Value::as_array).cloned().unwrap_or_default();
    let first = points.first();
    let e0 = first.map(|p| js_round(f(p, "easting"))).unwrap_or(0.0);
    let n0 = first.map(|p| js_round(f(p, "northing"))).unwrap_or(0.0);
    let z0 = first.map(|p| js_round(f(p, "zLeft").min(f(p, "zRight")))).unwrap_or(0.0);
    let col = |g: &dyn Fn(&Value) -> Value| Value::Array(points.iter().map(g).collect());
    let gaps: Vec<Value> = trace
        .get("gaps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|g| json!({ "from": g.get("from"), "to": g.get("to") }))
        .collect();
    let mut out = serde_json::Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            out.insert(k.into(), v);
        }
    };
    let m = |k: &str| meta.get(k).cloned();
    put("id", m("id"));
    put("name", m("name"));
    put("epsg", trace.get("epsg").cloned());
    put("rail", m("rail"));
    put("guide", m("guide"));
    put("cloudNames", Some(m("cloudNames").unwrap_or(json!([]))));
    put("cloudRefs", Some(m("cloudRefs").unwrap_or(json!([]))));
    put("createdAt", m("createdAt"));
    put("step", m("step"));
    put("gaps", Some(Value::Array(gaps)));
    put(
        "points",
        Some(json!({
                "e0": number(e0), "n0": number(n0), "z0": number(z0),
                "st": col(&|p| mm(f(p, "station"))),
                "de": col(&|p| mm(f(p, "easting") - e0)),
                "dn": col(&|p| mm(f(p, "northing") - n0)),
                "zl": col(&|p| mm(f(p, "zLeft") - z0)),
                "zr": col(&|p| mm(f(p, "zRight") - z0)),
                "ga": col(&|p| mm(f(p, "gauge"))),
                "q": col(&|p| json!(if p.get("quality").and_then(Value::as_str) == Some("good") { 1 } else { 0 })),
        })),
    );
    Value::Object(out)
}

fn arr<'a>(p: &'a Value, k: &str) -> &'a [Value] {
    p.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The points of a survey (`surveyPoints`).
pub fn survey_points(survey: &Value) -> Vec<Value> {
    let Some(p) = survey.get("points").filter(|p| truthy(p.get("st"))) else {
        return vec![];
    };
    let at = |k: &str, i: usize| arr(p, k).get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let (e0, n0, z0) = (f(p, "e0"), f(p, "n0"), f(p, "z0"));
    (0..arr(p, "st").len())
        .map(|i| {
            let (zl, zr) = (z0 + at("zl", i) / 1000.0, z0 + at("zr", i) / 1000.0);
            json!({
                "station": number(at("st", i) / 1000.0), "easting": number(e0 + at("de", i) / 1000.0), "northing": number(n0 + at("dn", i) / 1000.0),
                "zLeft": number(zl), "zRight": number(zr), "cant": number((at("zl", i) - at("zr", i)) / 1000.0), "gauge": number(at("ga", i) / 1000.0),
                "quality": if rail_across_js::truthy(arr(p, "q").get(i)) { "good" } else { "doubtful" },
            })
        })
        .collect()
}

/// A survey without some of its points (`withoutPoints`).
pub fn without_points(survey: &Value, indices: &[usize]) -> Value {
    let p = &survey["points"];
    let keep = |k: &str| {
        Value::Array(
            arr(p, k)
                .iter()
                .enumerate()
                .filter(|(i, _)| !indices.contains(i))
                .map(|(_, v)| v.clone())
                .collect(),
        )
    };
    let mut points = p.as_object().cloned().unwrap_or_default();
    for k in ["st", "de", "dn", "zl", "zr", "ga", "q"] {
        points.insert(k.into(), keep(k));
    }
    let mut out = survey.as_object().cloned().unwrap_or_default();
    out.insert("points".into(), Value::Object(points));
    Value::Object(out)
}

/// How many points, how many good, from where to where (`surveyStats`).
pub fn survey_stats(survey: &Value) -> Value {
    let p = survey.get("points").unwrap_or(&Value::Null);
    let st = arr(p, "st");
    let n = st.len();
    let good: f64 = arr(p, "q").iter().map(|q| q.as_f64().unwrap_or(0.0)).sum();
    json!({
        "points": n, "good": if n > 0 { number(good) } else { json!(0) },
        "from": if n > 0 { number(st[0].as_f64().unwrap_or(f64::NAN) / 1000.0) } else { json!(0) },
        "to": if n > 0 { number(st[n - 1].as_f64().unwrap_or(f64::NAN) / 1000.0) } else { json!(0) },
    })
}

/// What is wrong with a survey as kept, or None (`surveyDefect`).
pub fn survey_defect(survey: &Value) -> Option<&'static str> {
    if !truthy(survey.get("id")) {
        return Some("id");
    }
    if !js_number(survey.get("epsg")).is_finite() {
        return Some("epsg");
    }
    let Some(p) = survey.get("points").filter(|p| truthy(Some(p))) else {
        return Some("points");
    };
    let Some(st) = p.get("st").and_then(Value::as_array) else {
        return Some("points");
    };
    for k in ["de", "dn", "zl", "zr", "ga", "q"] {
        if p.get(k).and_then(Value::as_array).is_none_or(|a| a.len() != st.len()) {
            return Some(match k {
                "de" => "de",
                "dn" => "dn",
                "zl" => "zl",
                "zr" => "zr",
                "ga" => "ga",
                _ => "q",
            });
        }
    }
    None
}
