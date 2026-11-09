// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The speeds the rules let an element carry (`rules/speed.js`).

use std::collections::BTreeSet;

use rail_across_geom::cant::transition_cant_ends;
use rail_across_js::{get, number};
use serde_json::{Value as Json, json};

use crate::alignment_check::Context;
use crate::bundled::catalog_speed_range;
use crate::check::verdict_errors;
use crate::element::Element;

/// Speed is designed in 5 km/h steps.
pub const SPEED_STEP: f64 = 5.0;

/// Radius and signed cant that govern an element's cant physics (`governing`): None for a straight.
pub fn governing(elements: &[Json], i: usize) -> Option<(f64, f64)> {
    let el = &elements[i];
    let n = |k: &str| get(el, k).and_then(Json::as_f64);
    if let Some(r) = n("radius").filter(|r| *r != 0.0 && !r.is_nan()) {
        return Some((r, n("cant").unwrap_or(0.0)));
    }
    if el.get("elementType").and_then(Json::as_f64) == Some(2.0) {
        let r1 = n("r1").filter(|r| *r != 0.0).map(f64::abs).unwrap_or(f64::INFINITY);
        let r2 = n("r2").filter(|r| *r != 0.0).map(f64::abs).unwrap_or(f64::INFINITY);
        if !r1.min(r2).is_finite() {
            return None;
        }
        let (start, end) = transition_cant_ends(elements, i);
        return Some(if r1 <= r2 {
            (n("r1").unwrap_or(f64::NAN), start)
        } else {
            (n("r2").unwrap_or(f64::NAN), end)
        });
    }
    None
}

/// The highest design speed the catalogue lets element `i` carry (`ruleMaxSpeed`): (speed, fixed).
pub fn rule_max_speed(elements: &[Element], i: usize, cap: Option<f64>, context: Context) -> (Option<f64>, Vec<String>) {
    if i >= elements.len() {
        return (None, Vec::new());
    }
    let mut view = elements.to_vec();
    let near: Vec<usize> = [i.wrapping_sub(1), i, i + 1].into_iter().filter(|j| *j < elements.len()).collect();
    let (min, max) = catalog_speed_range();
    let lo = (min / SPEED_STEP).ceil() * SPEED_STEP;
    let hi = (max / SPEED_STEP).floor() * SPEED_STEP;
    let mut errors: Vec<(f64, BTreeSet<String>)> = Vec::new();
    let mut v = hi;
    while v >= lo {
        for &j in &near {
            view[j] = Element {
                speed: Some(v),
                ..elements[j].clone()
            };
        }
        let ids: BTreeSet<String> = verdict_errors(&view, i, context).unwrap_or_default().into_iter().collect();
        errors.push((v, ids));
        v -= SPEED_STEP;
    }
    let Some(last) = errors.last() else { return (None, Vec::new()) };
    let fixed: Vec<String> = last.1.iter().filter(|id| errors.iter().all(|e| e.1.contains(*id))).cloned().collect();
    let free = errors
        .iter()
        .find(|(v, ids)| cap.is_none_or(|c| *v <= c) && ids.iter().all(|id| fixed.contains(id)));
    let mut fixed_ids: Vec<String> = Vec::new();
    for id in &fixed {
        let rule = id[id.find(':').map(|p| p + 1).unwrap_or(0)..].to_string();
        if !fixed_ids.contains(&rule) {
            fixed_ids.push(rule);
        }
    }
    (free.map(|f| f.0), fixed_ids)
}

/// `ruleMaxSpeed` as the JavaScript returns it.
pub fn rule_max_speed_json(elements: &[Element], i: usize, cap: Option<f64>, context: Context) -> Json {
    let (speed, fixed) = rule_max_speed(elements, i, cap, context);
    json!({ "speed": speed.map(number), "fixed": fixed })
}

/// Every element's speed raised to what the catalogue lets it carry under `cap` (`maxSpeeds`).
pub fn max_speeds(raw: &[Json], elements: &[Element], cap: Option<f64>) -> Vec<Json> {
    raw.iter()
        .enumerate()
        .map(|(i, el)| {
            let (speed, _) = rule_max_speed(elements, i, cap, Context::default());
            let current = get(el, "speed").and_then(Json::as_f64);
            match speed {
                None => match (cap, current) {
                    (Some(c), Some(s)) if s > c => {
                        let mut o = el.as_object().cloned().unwrap_or_default();
                        o.insert("speed".into(), number(c));
                        Json::Object(o)
                    }
                    _ => el.clone(),
                },
                Some(v) if current == Some(v) => el.clone(),
                Some(v) => {
                    let mut o = el.as_object().cloned().unwrap_or_default();
                    o.insert("speed".into(), number(v));
                    Json::Object(o)
                }
            }
        })
        .collect()
}
