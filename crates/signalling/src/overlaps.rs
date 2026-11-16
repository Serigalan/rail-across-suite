// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What an overlap needs, and the speed a turnout allows a run (`overlaps.js`, `switchSpeeds.js`).

use rail_across_js::{js_round, number, strict_eq, truthy};
use rail_across_switch::catalogue::switch_branch_speed;
use rail_across_track::heights::{height_at, track_length};
use rail_across_track::switch_model::port_track;
use serde_json::{Value, json};

use crate::catalogue::SIGNAL_CATALOGUE;

/// The speed a route permits (`signallingSpeedOf`).
pub fn signalling_speed_of(signalling: &Value, route: &Value) -> Option<Value> {
    let signals = signalling.get("collections").and_then(|c| c.get("signals")).and_then(Value::as_array)?;
    let start = signals.iter().find(|s| strict_eq(s.get("id"), route.get("startSignalId")))?;
    for l in start.get("logicalSignals").and_then(Value::as_array).into_iter().flatten() {
        if let Some(hit) = l
            .get("signallingSpeeds")
            .and_then(Value::as_array)
            .and_then(|v| v.iter().find(|x| strict_eq(x.get("routeId"), route.get("id"))))
        {
            return Some(hit.get("speed").cloned().unwrap_or(Value::Null));
        }
    }
    None
}

/// How far from a planning boundary a virtual signal may stand and still be its start [m].
const AT_BOUNDARY: f64 = 20.0;

/// The speed trains enter the planning area at where a virtual signal stands
/// at a boundary acting inward (`boundaryStartSpeed`): the boundary's, as the
/// layout sets it; None elsewhere. `core` is the core domain — its end marks
/// at a track end (`at`) or of a record (`trackId`, `endpoint`).
pub fn boundary_start_speed(core: &Value, signal: &Value) -> Option<f64> {
    if signal.get("kind").and_then(Value::as_str) != Some("virtual") {
        return None;
    }
    let at = signal.get("at")?;
    let (track_id, station) = (at.get("trackId")?, at.get("station")?.as_f64()?);
    let list = |k: &str| core.get("collections").and_then(|c| c.get(k)).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    let track = list("tracks").iter().find(|t| strict_eq(t.get("id"), Some(track_id)))?;
    let length = track_length(track);
    list("endMarks").iter().find_map(|m| {
        let end = m.get("at").unwrap_or(m);
        if m.get("kind").and_then(Value::as_str) != Some("boundary") || !strict_eq(end.get("trackId"), Some(track_id)) {
            return None;
        }
        let (edge, inward) = if end.get("endpoint").and_then(Value::as_str) == Some("END") { (length, "down") } else { (0.0, "up") };
        let acts = truthy(signal.get("bothDirections")) || at.get("direction").and_then(Value::as_str).unwrap_or("up") == inward;
        (acts && (station - edge).abs() <= AT_BOUNDARY).then(|| m.get("speed").and_then(Value::as_f64).filter(|v| *v > 0.0)).flatten()
    })
}

/// The speed a route is run at from its start (`routeSpeedOf`): what its start
/// signal signals for it, else — a virtual start at a planning boundary — the
/// boundary's speed.
pub fn route_speed_of(signalling: &Value, route: &Value, core: &Value) -> Option<Value> {
    signalling_speed_of(signalling, route).or_else(|| {
        let signals = signalling.get("collections").and_then(|c| c.get("signals")).and_then(Value::as_array)?;
        let start = signals.iter().find(|s| strict_eq(s.get("id"), route.get("startSignalId")))?;
        boundary_start_speed(core, start).map(number)
    })
}

/// The length an overlap needs [m] (`overlapLength`).
pub fn overlap_length(speed: Option<f64>, gradient: f64) -> Option<f64> {
    let speed = speed.filter(|s| s.is_finite())?;
    let table = &SIGNAL_CATALOGUE["durchrutschweg"];
    let stufen = table["stufen"].as_array()?;
    let base = stufen
        .iter()
        .find(|s| s.get("bis").is_none_or(Value::is_null) || s["bis"].as_f64().is_some_and(|b| speed <= b))
        .or(stufen.last())?["laenge"]
        .as_f64()?;
    let n = &table["neigung"];
    let f = |k: &str| n[k].as_f64().unwrap_or(f64::NAN);
    let (from, a, g, step) = (f("ab_gefaelle"), f("verzoegerung"), f("schwere"), f("runden"));
    let fall = -gradient;
    if fall <= from {
        return Some(base);
    }
    let left = a - (g * fall) / 1000.0;
    if left <= 0.0 {
        return None;
    }
    Some(((base * a) / left / step).ceil() * step)
}

/// The gradient over a run [‰] (`gradientAlong`).
pub fn gradient_along(tracks: &[Value], start: &Value, end: &Value, length: f64) -> Option<f64> {
    if !(length > 0.0) {
        return None;
    }
    let height_of = |loc: &Value| -> Option<f64> {
        let t = tracks.iter().find(|t| strict_eq(t.get("id"), loc.get("trackId")))?;
        let hs = t.get("heights").and_then(Value::as_array)?;
        height_at(hs, loc.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN))
    };
    let (a, b) = (height_of(start)?, height_of(end)?);
    Some(js_round(((b - a) / length) * 1000.0 * 10.0) / 10.0)
}

/// The turnouts a run passes on their branch and the speed they allow (`branchSpeedOnRun`).
pub fn branch_speed_on_run(switches: &[Value], track_ids: &[Value]) -> Option<(Option<f64>, Vec<Value>)> {
    let mut ids: Vec<Value> = Vec::new();
    let mut speeds: Vec<f64> = Vec::new();
    for i in 0..track_ids.len().saturating_sub(1) {
        let pair = [&track_ids[i], &track_ids[i + 1]];
        for sw in switches {
            if rail_across_js::get(sw, "kind").and_then(Value::as_str).unwrap_or("switch") != "switch" {
                continue;
            }
            let (Some(toe), Some(branch)) = (port_track(sw, "A"), port_track(sw, "B1")) else {
                continue;
            };
            if strict_eq(Some(toe), Some(branch))
                || !pair.iter().any(|p| strict_eq(Some(p), Some(toe)))
                || !pair.iter().any(|p| strict_eq(Some(p), Some(branch)))
            {
                continue;
            }
            let id = rail_across_js::get(sw, "id").or_else(|| sw.get("switchId")).cloned().unwrap_or(Value::Null);
            if ids.iter().any(|x| strict_eq(Some(x), Some(&id))) {
                continue;
            }
            ids.push(id);
            if let Some(v) = switch_branch_speed(sw).filter(|v| *v > 0.0) {
                speeds.push(v);
            }
        }
    }
    if ids.is_empty() {
        return None;
    }
    let speed = (!speeds.is_empty()).then(|| speeds.iter().copied().fold(f64::INFINITY, f64::min));
    Some((speed, ids))
}

pub fn branch_speed_json(r: Option<(Option<f64>, Vec<Value>)>) -> Value {
    match r {
        None => Value::Null,
        Some((speed, ids)) => json!({ "speed": speed.map(rail_across_js::number), "switchIds": ids }),
    }
}
