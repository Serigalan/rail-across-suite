// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The signal aspects of DB Ril 301, the signalling and train protection
//! systems (`catalogue.js`).

use std::sync::LazyLock;

use rail_across_js::{js_number, js_string};
use serde_json::{Value, json};

const FILE: &str = include_str!("../../../packages/core/constraints/db-ril-301.json");

/// The catalogue itself (`SIGNAL_CATALOGUE`).
pub static SIGNAL_CATALOGUE: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(FILE).expect("db-ril-301.json"));

/// The catalogue as the domain names it (`CATALOGUE_REF`).
pub fn catalogue_ref() -> Value {
    json!({ "id": SIGNAL_CATALOGUE["katalog"]["id"], "version": SIGNAL_CATALOGUE["katalog"]["katalog_version"] })
}

fn has_own<'a>(obj: &'a Value, key: &str) -> Option<&'a Value> {
    obj.as_object().and_then(|o| o.get(key))
}

/// The aspect of a key (`aspectOf`).
pub fn aspect_of(key: &str) -> Option<&'static Value> {
    SIGNAL_CATALOGUE["begriffe"].as_array()?.iter().find(|b| b["id"].as_str() == Some(key))
}

/// An entry's title in a language (`titleIn`).
pub fn title_in(entry: Option<&Value>, language: &str) -> Value {
    let Some(e) = entry.filter(|e| rail_across_js::truthy(Some(e))) else {
        return Value::Null;
    };
    if language == "de" {
        return e.get("title").cloned().unwrap_or(Value::Null);
    }
    rail_across_js::get(e, &format!("title_{language}"))
        .or_else(|| e.get("title"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// What a kind of signal shows in a system (`signalKindOf`).
pub fn signal_kind_of(system: Option<&str>, kind: Option<&str>) -> Option<&'static Value> {
    let system = system.filter(|s| !s.is_empty())?;
    let sys = has_own(&SIGNAL_CATALOGUE["systeme"], system)?;
    has_own(&sys["signalarten"], kind?)
}

/// Whether a system is one of the catalogue (`knownSystem`).
pub fn known_system(system: Option<&str>) -> bool {
    system.is_some_and(|s| has_own(&SIGNAL_CATALOGUE["systeme"], s).is_some())
}

/// What a signal's screen must and may show (`screenOf`).
pub fn screen_of(logical: &[Value]) -> Value {
    let kinds: Vec<&Value> = logical
        .iter()
        .filter_map(|l| signal_kind_of(l.get("system").and_then(Value::as_str), l.get("type").and_then(Value::as_str)))
        .collect();
    if kinds.is_empty() {
        return Value::Null;
    }
    let union = |pick: &dyn Fn(&Value) -> Vec<Value>| -> Vec<Value> {
        let mut out: Vec<Value> = Vec::new();
        for k in &kinds {
            for v in pick(k) {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
        out
    };
    let arr = |v: &Value| v.as_array().cloned().unwrap_or_default();
    json!({
        "kinds": kinds,
        "pflicht": union(&|k| arr(&k["schirm"]["pflicht"])),
        "fahrt": kinds.iter().map(|k| arr(&k["schirm"]["fahrt"])).filter(|f| !f.is_empty()).collect::<Vec<_>>(),
        "erlaubt": union(&|k| arr(&k["schirm"]["erlaubt"])),
        "zusatz": union(&|k| arr(&k["zusatz"])),
    })
}

/// A train protection system (`protectionSystemOf`).
pub fn protection_system_of(system: Option<&str>) -> Option<&'static Value> {
    has_own(&SIGNAL_CATALOGUE["zugbeeinflussung"]["systeme"], system?)
}

/// A kind of balise or magnet (`baliseKindOf`).
pub fn balise_kind_of(system: Option<&str>, kind: Option<&str>) -> Option<&'static Value> {
    let arten = protection_system_of(system)?.get("arten").filter(|a| rail_across_js::truthy(Some(a)))?;
    has_own(arten, kind?)
}

/// Whether a value fits a setting (`settingFits`).
pub fn setting_fits(setting: &Value, value: &Value) -> bool {
    match setting["typ"].as_str() {
        Some("auswahl") => value.as_str().is_some_and(|v| has_own(&setting["werte"], v).is_some()),
        Some("zahl") => value.as_f64().is_some_and(|v| {
            v.is_finite()
                && setting.get("min").and_then(Value::as_f64).is_none_or(|m| v >= m)
                && setting.get("max").and_then(Value::as_f64).is_none_or(|m| v <= m)
        }),
        Some("wahrheitswert") => value.is_boolean(),
        Some("text") => value.as_str().is_some_and(|v| !v.is_empty()),
        _ => false,
    }
}

/// How the settings of an object stand to what its system declares (`settingsProblems`): (problem, setting).
pub fn settings_problems(settings: Option<&Value>, declared: Option<&Value>, pflicht: &[Value], erlaubt: Option<&[Value]>) -> Vec<(&'static str, String)> {
    let mut problems = Vec::new();
    let empty = json!({});
    let given = settings.filter(|s| s.is_object()).unwrap_or(&empty);
    for name in pflicht {
        let n = js_string(Some(name));
        if given.get(&n).is_none() {
            problems.push(("missing", n));
        }
    }
    for (name, value) in given.as_object().into_iter().flatten() {
        let setting = declared.and_then(|d| has_own(d, name));
        let not_allowed = erlaubt.is_some_and(|e| !e.iter().any(|x| x.as_str() == Some(name)));
        match setting {
            Some(s) if !not_allowed => {
                if !setting_fits(s, value) {
                    problems.push(("invalid", name.clone()));
                }
            }
            _ => problems.push(("unknown", name.clone())),
        }
    }
    problems
}

/// Whether an aspect may stand on a plate of its own (`boardAspect`).
pub fn board_aspect(key: &str) -> bool {
    SIGNAL_CATALOGUE["ohne_system"]["tafeln"]
        .as_array()
        .is_some_and(|t| t.iter().any(|x| x.as_str() == Some(key)))
}

/// Whether the symbol is what the aspect asks for (`symbolFits`).
pub fn symbol_fits(aspect: &Value, symbol: Option<&Value>) -> bool {
    let truthy = |k: &str| rail_across_js::truthy(aspect.get(k));
    if !truthy("symbole") && !truthy("symbol_frei") {
        return symbol.is_none();
    }
    let Some(sym) = symbol else { return false };
    aspect.get("symbol_frei") == Some(&json!(true)) || aspect["symbole"].as_array().is_some_and(|s| s.contains(sym))
}

/// Whether an aspect may stand on a frame of that kind (`aspectFitsFrame`).
pub fn aspect_fits_frame(aspect: &Value, frame: Option<&str>) -> bool {
    match aspect.get("rahmen").filter(|r| rail_across_js::truthy(Some(r))) {
        None => true,
        Some(r) => r.as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == frame && frame.is_some())),
    }
}

/// The speed an aspect's symbol shows [km/h] (`aspectSpeed`).
pub fn aspect_speed(aspect: Option<&Value>, symbol: Option<&Value>) -> Option<f64> {
    if aspect.and_then(|a| a.get("symbol_bedeutung")).and_then(Value::as_str) != Some("speed") {
        return None;
    }
    let text = match symbol {
        None | Some(Value::Null) => String::new(),
        Some(v) => js_string(Some(v)),
    };
    let n = js_number(Some(&Value::String(text.replacen(',', ".", 1))));
    n.is_finite().then_some(n * 10.0)
}
