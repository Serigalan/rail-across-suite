// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch form tables grouped for the viewer (`weichenRegelwerk.js`).

use rail_across_js::{get, number};
use serde_json::{Value, json};

use crate::catalogue::{CROSSING_TYPES, SWITCH_CATALOGUE, SWITCH_TYPES, SWITCH_TYPES_SPECIAL, switch_mark, switch_variants};

/// What the rulebook is (`SWITCH_RULEBOOK`).
pub fn switch_rulebook() -> Value {
    SWITCH_CATALOGUE["katalog"].clone()
}

fn or_null(v: &Value, k: &str) -> Value {
    get(v, k).cloned().unwrap_or(Value::Null)
}

fn straight_end_piece(form: &Value) -> f64 {
    form.get("branch")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|s| s.get("type").and_then(Value::as_str) == Some("straight"))
        .map(|s| s["length"].as_f64().unwrap_or(f64::NAN))
        .sum()
}

fn turnout_row(form: &Value) -> Value {
    json!({
        "label": form.get("label"), "radius": form.get("R"), "neigung": form.get("ratio"), "speed": form.get("speed"),
        "marke": switch_mark(Some(form)).map(number), "markeBerechnet": get(form, "dLcs").is_none(),
        "minl": or_null(form, "minl"), "gerade": number(straight_end_piece(form)), "lds": or_null(form, "lds"),
        "symmetrisch": form.get("symmetric") == Some(&json!(true)), "herzstueck": or_null(form, "herzstueck"),
        "varianten": switch_variants(Some(form)),
    })
}

fn crossing_row(form: &Value) -> Value {
    json!({
        "label": form.get("label"), "kind": form.get("kind"), "neigung": form.get("ratio"), "radius": or_null(form, "R"),
        "tangente": get(form, "lt").or_else(|| get(form, "lb")).cloned().unwrap_or(Value::Null),
        "speed": or_null(form, "speed"), "marke": or_null(form, "dLcs"), "routen": or_null(form, "routen"),
        "varianten": switch_variants(Some(form)),
    })
}

/// The tables in the catalogue's order, empty groups left out (`switchGroups`).
pub fn switch_groups() -> Vec<Value> {
    let group = |key: &str, art: &str, formen: Vec<Value>| json!({ "key": key, "art": art, "titleKey": format!("constraints_weichen_{key}"), "hintKey": format!("constraints_weichen_{key}_hint"), "formen": formen });
    let crossings = |klasse: &str, slip: bool| -> Vec<Value> {
        CROSSING_TYPES
            .iter()
            .filter(|f| f.get("klasse").and_then(Value::as_str) == Some(klasse) && (f.get("kind").and_then(Value::as_str) != Some("crossing")) == slip)
            .map(|f| crossing_row(f))
            .collect()
    };
    [
        group("regel_weichen", "weiche", SWITCH_TYPES.iter().map(|f| turnout_row(f)).collect()),
        group("regel_kreuzungen", "kreuzung", crossings("regel", false)),
        group("regel_kreuzungsweichen", "kreuzung", crossings("regel", true)),
        group("sonder_weichen", "weiche", SWITCH_TYPES_SPECIAL.iter().map(|f| turnout_row(f)).collect()),
        group("sonder_kreuzungen", "kreuzung", crossings("sonderbauform", false)),
        group("sonder_kreuzungsweichen", "kreuzung", crossings("sonderbauform", true)),
    ]
    .into_iter()
    .filter(|g| g["formen"].as_array().is_some_and(|f| !f.is_empty()))
    .collect()
}
