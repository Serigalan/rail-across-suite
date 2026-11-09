// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Cant, cant deficiency and the speeds they allow (`rules/cant.js`, LP.KB):
//! every limit read out of the bundled catalogue, as the dialogs read it.

use std::sync::OnceLock;

use rail_across_js::{get, js_round, js_trim};
use serde_json::{Value as Json, json};

use crate::bundled::{catalog_limit, catalog_speed_range};
use crate::expr::Value;
use crate::physics::CANT_DEFICIENCY_COEFF;

const CANT_COEFF: f64 = 6.5;
const AUTO_CANT_MIN_DEF: f64 = 60.0;

fn limit(id: &str, name: &str, scope: &[(&str, Value)], in_switch: bool) -> f64 {
    catalog_limit(id, name, scope, in_switch).unwrap_or(f64::NAN)
}

macro_rules! cached {
    ($name:ident, $body:expr) => {
        pub fn $name() -> f64 {
            static V: OnceLock<f64> = OnceLock::new();
            *V.get_or_init(|| $body)
        }
    };
}

cached!(max_cant, limit("LP.KB.01", "max", &[("element.cant", 0.0.into())], false));
cached!(cant_step, limit("LP.KB.03", "step", &[("element.cant", 0.0.into())], false));
cached!(max_switch_cant, limit("LP.KB.05", "reg", &[("element.cant", 0.0.into())], true));
cached!(max_switch_cant_def, limit("LP.KB.06", "max", &[("physics.u_f", 0.0.into())], true));
cached!(
    max_switch_cant_exception,
    limit("LP.KB.05", "discretion", &[("element.cant", 0.0.into())], true)
);

/// The deficiency a line element may be built with at `speed` — LP.KB.02's Ermessensgrenze (`cantDefLimit`).
pub fn cant_def_limit(speed: Option<f64>) -> f64 {
    limit(
        "LP.KB.02",
        "discretion",
        &[("element.design_speed", speed.unwrap_or(0.0).into()), ("physics.u_f", 0.0.into())],
        false,
    )
}

fn cant_def_steps() -> &'static [(f64, f64)] {
    static STEPS: OnceLock<Vec<(f64, f64)>> = OnceLock::new();
    STEPS.get_or_init(|| {
        let (min, max) = catalog_speed_range();
        let mut steps: Vec<(f64, f64)> = Vec::new();
        let mut v = min;
        while v <= max {
            let l = cant_def_limit(Some(v));
            if steps.last().is_none_or(|s| s.1 != l) {
                steps.push((v, l));
            }
            v += 1.0;
        }
        steps
    })
}

fn truthy_member(el: &Json, key: &str) -> bool {
    rail_across_js::truthy(el.get(key))
}

/// The deficiency this element may be built with (`limitCantDef`).
pub fn limit_cant_def(el: &Json) -> f64 {
    if truthy_member(el, "switchBranch") {
        max_switch_cant_def()
    } else {
        cant_def_limit(get(el, "speed").and_then(Json::as_f64))
    }
}

/// `'over'` where a deficiency is past what the element may be built with (`cantDefLevel`).
pub fn cant_def_level(el: &Json, cant_def: f64) -> Option<&'static str> {
    if cant_def > limit_cant_def(el) { Some("over") } else { None }
}

/// The sign cant takes from the curve (`cantSign`).
pub fn cant_sign(radius: f64) -> f64 {
    if radius < 0.0 { -1.0 } else { 1.0 }
}

/// `Math.sign`.
fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// A cant snapped to the design step; the magnitude decides (`roundCant`).
pub fn round_cant(mm: f64) -> f64 {
    js_sign(mm) * js_round(mm.abs() / cant_step()) * cant_step()
}

/// computeAutoC's figures (`AUTO_CANT_MODEL`).
pub fn auto_cant_model() -> Json {
    json!({ "coeff": CANT_COEFF, "defCoeff": CANT_DEFICIENCY_COEFF, "defMin": AUTO_CANT_MIN_DEF, "max": max_cant(), "step": cant_step() })
}

/// Cant proposed for a speed and a signed radius (`computeAutoC`).
pub fn compute_auto_c(speed: f64, radius: f64) -> f64 {
    let r = radius.abs();
    if r <= 0.0 {
        return 0.0;
    }
    let def_without = js_round((CANT_DEFICIENCY_COEFF * speed * speed) / r);
    if def_without < AUTO_CANT_MIN_DEF {
        return 0.0;
    }
    cant_sign(radius) * max_cant().min(round_cant((CANT_COEFF * speed * speed) / r))
}

/// The Regelüberhöhung — LP.KB.04's `u_reg` (`standardCant`).
pub fn standard_cant(speed: f64, radius: f64, in_switch: bool) -> f64 {
    let r = radius.abs();
    if !(r > 0.0) {
        return 0.0;
    }
    cant_sign(radius)
        * limit(
            "LP.KB.04",
            "u_reg",
            &[
                ("element.design_speed", speed.into()),
                ("element.radius", r.into()),
                ("element.cant", 0.0.into()),
            ],
            in_switch,
        )
}

/// The justification an element carries, trimmed (`cantExceptionOf`).
pub fn cant_exception_of(el: &Json) -> String {
    el.get("cantException")
        .and_then(Json::as_str)
        .map(|s| js_trim(s).to_string())
        .unwrap_or_default()
}

/// The cant a switch route may carry, given the justification (`switchCantLimit`).
pub fn switch_cant_limit(reason: Option<&str>) -> f64 {
    if reason.is_some_and(|r| !js_trim(r).is_empty()) {
        max_switch_cant_exception()
    } else {
        max_switch_cant()
    }
}

/// The cant this element may carry (`cantLimit`).
pub fn cant_limit(el: &Json) -> f64 {
    if truthy_member(el, "switchBranch") {
        switch_cant_limit(Some(&cant_exception_of(el)))
    } else {
        max_cant()
    }
}

/// A typed cant as it may be stored for `el` (`clampCant`).
pub fn clamp_cant(value: f64, el: &Json) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let magnitude = cant_limit(el).min(round_cant(value.abs()));
    match get(el, "radius").and_then(Json::as_f64).filter(|r| *r != 0.0 && !r.is_nan()) {
        Some(r) => cant_sign(r) * magnitude,
        None => js_sign(value) * magnitude,
    }
}

fn cants_of(el: &Json) -> Vec<f64> {
    let n = |k: &str| get(el, k).and_then(Json::as_f64);
    if n("cantStart").is_some() || n("cantEnd").is_some() {
        vec![n("cantStart").or(n("cant")).unwrap_or(0.0), n("cantEnd").or(n("cant")).unwrap_or(0.0)]
    } else {
        vec![n("cant").unwrap_or(0.0)]
    }
}

/// The greatest cant magnitude anywhere along this element (`worstCantOf`).
pub fn worst_cant_of(el: &Json) -> f64 {
    cants_of(el).into_iter().map(f64::abs).fold(f64::NEG_INFINITY, f64::max)
}

/// Does this element carry more cant than allowed anywhere along it (`cantExceedsLimit`)?
pub fn cant_exceeds_limit(el: &Json) -> bool {
    worst_cant_of(el) > cant_limit(el)
}

/// The justification field an element built with this cant needs (`cantExceptionFields`).
pub fn cant_exception_fields(cant: Option<f64>, reason: Option<&str>) -> Json {
    match reason.map(js_trim).filter(|r| !r.is_empty()) {
        Some(r) if cant.unwrap_or(0.0).abs() > max_switch_cant() => json!({ "cantException": r }),
        _ => json!({}),
    }
}

/// A cant typed into a field, rounded and held in its limits, or None (`cantFromInput`).
pub fn cant_from_input(draft: &Json, min: f64, max: f64) -> Option<f64> {
    let text = rail_across_js::js_string(Some(draft));
    let typed = match draft {
        Json::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Json::String(s) => {
            let t = js_trim(s);
            if t.is_empty() { 0.0 } else { t.parse::<f64>().unwrap_or(f64::NAN) }
        }
        Json::Null => 0.0,
        Json::Bool(b) => f64::from(u8::from(*b)),
        _ => f64::NAN,
    };
    if js_trim(&text).is_empty() || !typed.is_finite() {
        return None;
    }
    Some(round_cant(min.max(max.min(typed))))
}

/// A cant typed into a switch dialog, on the step and under the ceiling (`clampSwitchCant`).
pub fn clamp_switch_cant(value: f64) -> f64 {
    round_cant((-max_switch_cant_exception()).max(max_switch_cant_exception().min(value)))
}

/// What stands between this cant and being built (`switchCantError`).
pub fn switch_cant_error(cant: Option<f64>, reason: Option<&str>) -> Option<&'static str> {
    let magnitude = cant.unwrap_or(0.0).abs();
    if magnitude > max_switch_cant_exception() {
        return Some("over");
    }
    if magnitude > switch_cant_limit(reason) {
        return Some("unjustified");
    }
    None
}

/// Cant proposed for a switch (`computeSwitchCant`).
pub fn compute_switch_cant(speed: f64, radius: f64) -> f64 {
    let r = radius.abs();
    if r <= 0.0 {
        return 0.0;
    }
    let def_at0 = js_round((CANT_DEFICIENCY_COEFF * speed * speed) / r);
    if def_at0 <= max_switch_cant_def() {
        return 0.0;
    }
    let needed = round_cant((CANT_DEFICIENCY_COEFF * speed * speed) / r - max_switch_cant_def());
    cant_sign(radius) * max_switch_cant().min(0f64.max(needed))
}

/// Cant deficiency from speed, radius and cant (`computeCantDef`).
pub fn compute_cant_def(speed: f64, radius: f64, cant: f64) -> f64 {
    let r = radius.abs();
    if r <= 0.0 {
        return 0.0;
    }
    js_round(((CANT_DEFICIENCY_COEFF * speed * speed) / r) - cant.abs())
}

/// Cant deficiency of a route whose cant may lie against it (`computeCantDefSigned`).
pub fn compute_cant_def_signed(speed: f64, radius: f64, cant: Option<f64>) -> f64 {
    let r = radius.abs();
    if r <= 0.0 {
        return 0.0;
    }
    js_round(((CANT_DEFICIENCY_COEFF * speed * speed) / r) - cant.unwrap_or(0.0) * js_sign(radius))
}

/// Highest speed a radius and cant allow under `limit` (`computeMaxSpeed`).
pub fn compute_max_speed(radius: f64, cant: Option<f64>, limit: f64) -> Option<f64> {
    let r = radius.abs();
    if !(r > 0.0) {
        return None;
    }
    let room = cant.unwrap_or(0.0) * js_sign(radius) + limit;
    if room <= 0.0 {
        return Some(0.0);
    }
    let mut v = ((r * room) / CANT_DEFICIENCY_COEFF).sqrt().floor();
    while compute_cant_def_signed(v + 1.0, radius, cant) <= limit {
        v += 1.0;
    }
    Some(v)
}

/// Highest speed this element's geometry allows under the limit that holds there (`maxSpeedFor`).
pub fn max_speed_for(el: &Json, radius: f64, cant: Option<f64>) -> Option<f64> {
    if !(radius.abs() > 0.0) {
        return None;
    }
    let ceiling = catalog_speed_range().1;
    if truthy_member(el, "switchBranch") {
        return compute_max_speed(radius, cant, max_switch_cant_def()).map(|v| v.min(ceiling));
    }
    let steps = cant_def_steps();
    let mut best: f64 = 0.0;
    for (i, (from, l)) in steps.iter().enumerate() {
        let until = steps.get(i + 1).map(|s| s.0 - 1.0).unwrap_or(ceiling);
        let reached = compute_max_speed(radius, cant, *l).unwrap_or(f64::NAN).min(until);
        if reached >= *from {
            best = best.max(reached);
        }
    }
    Some(best)
}
