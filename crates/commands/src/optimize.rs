// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What the optimize dialog sends and keeps (`optimizeApply.js`).

use rail_across_js::{get, js_number, truthy};
use rail_across_track::heights::truncate_heights;
use rail_across_track::model::rebuild_coords;
use serde_json::{Value, json};

/// The request for a run of the optimizer (`optimizeRequest`).
pub fn optimize_request(track: &Value, o: &Value) -> Value {
    let mut out = json!({ "track": track, "corridorCm": o.get("corridorCm"), "grenzwert": o.get("grenzwert"), "uebergang": "auto", "maxiter": 100 });
    let v = js_number(o.get("vMax"));
    if v > 0.0 {
        out["vMax"] = rail_across_js::number(v);
    }
    if truthy(o.get("regelwerkId")) {
        out["regelwerk"] = o["regelwerkId"].clone();
    }
    if let Some(i) = get(o, "elementIdx") {
        out["targetElementIdx"] = i.clone();
    }
    out
}

fn len(el: Option<&Value>) -> f64 {
    el.and_then(|e| get(e, "length")).and_then(Value::as_f64).unwrap_or(0.0)
}

/// The heights the optimized track keeps (`reshapedHeights`).
pub fn reshaped_heights(track: &Value, elements: &[Value]) -> Option<Value> {
    let old: &[Value] = track.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    let i = elements.iter().enumerate().position(|(k, el)| len(Some(el)) != len(old.get(k)));
    if i.is_none() && elements.len() == old.len() {
        return track.get("heights").cloned();
    }
    let cut_at: f64 = elements.iter().take(i.unwrap_or(0)).map(|e| len(Some(e))).sum();
    truncate_heights(get(track, "heights").and_then(Value::as_array).map(Vec::as_slice), cut_at).map(Value::Array)
}

/// The track as the result has it (`optimizedTrack`).
pub fn optimized_track(track: &Value, result: &Value) -> Value {
    let elements: Vec<Value> = result["elements"].as_array().cloned().unwrap_or_default();
    let mut out = track.clone();
    out["coordinates"] = Value::Array(rebuild_coords(&elements));
    let heights = reshaped_heights(track, &elements);
    out["elements"] = Value::Array(elements);
    let o = out.as_object_mut().expect("a track");
    match heights {
        Some(h) => {
            o.insert("heights".into(), h);
        }
        None => {
            o.remove("heights");
        }
    }
    out
}
