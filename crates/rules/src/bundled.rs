// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The catalogue the app bundles (`regelkatalog.js`): DB Ril 800.0110, the
//! very file packages/core/constraints holds — compiled into the core's
//! module, as the JavaScript imports it, so the two read one Ril.

use std::sync::OnceLock;

use serde_json::Value as Json;

use crate::expr::Value;
use crate::katalog::{Katalog, Rule};

const FILE: &str = include_str!("../../../packages/core/constraints/db-ril-800-0110.json");

/// The bundled catalogue as JSON.
pub fn katalog_json() -> &'static Json {
    static JSON: OnceLock<Json> = OnceLock::new();
    JSON.get_or_init(|| serde_json::from_str(FILE).expect("the bundled catalogue"))
}

/// The bundled catalogue, read.
pub fn katalog() -> &'static Katalog {
    static KATALOG: OnceLock<Katalog> = OnceLock::new();
    KATALOG.get_or_init(|| crate::load_katalog(katalog_json()).expect("the bundled catalogue reads"))
}

/// A rule by its id (`ruleById`).
pub fn rule_by_id(id: &str) -> Option<&'static Rule> {
    katalog().rule(id).ok()
}

/// The rank of a severity; -1 for none (`severityRank`).
pub fn severity_rank(id: Option<&str>) -> i32 {
    id.map(|s| katalog().rank(s)).unwrap_or(-1)
}

/// The worst of some severities (`worstSeverity`).
pub fn worst_severity<'a>(ids: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    let mut worst: Option<&str> = None;
    for id in ids.into_iter().flatten() {
        if worst.is_none() || severity_rank(Some(id)) > severity_rank(worst) {
            worst = Some(id);
        }
    }
    worst
}

/// What a dialog may ask of a rule (`catalogLimit`): threshold or input `name`
/// of rule `id` for the scope given, in the switch area where asked.
pub fn catalog_limit(id: &str, name: &str, scope: &[(&str, Value)], in_switch: bool) -> Option<f64> {
    let k = katalog();
    let rule = k.rule(id).ok()?;
    let ctx = |c: &str| c == "switch_area";
    let in_context: Option<&dyn Fn(&str) -> bool> = if in_switch { Some(&ctx) } else { None };
    let outcome = k.evaluate_rule(rule, &k.scope(scope), in_context).ok()?;
    let sym = k.symbols().get(name)?;
    outcome.values.get(sym).and_then(Value::as_num).filter(|x| x.is_finite())
}

/// The design speeds the catalogue speaks about, LP.ALL.01 (`catalogSpeedRange`): (min, max).
pub fn catalog_speed_range() -> (f64, f64) {
    static RANGE: OnceLock<(f64, f64)> = OnceLock::new();
    *RANGE.get_or_init(|| {
        let k = katalog();
        let rule = k.rule("LP.ALL.01").expect("LP.ALL.01");
        let keeps = |v: f64| {
            let scope = k.scope(&[("element.design_speed", Value::Num(v))]);
            k.evaluate_rule(rule, &scope, None).ok().and_then(|o| o.severity).as_deref() == Some("ok")
        };
        let (mut min, mut max) = (None, None);
        for v in 1..=1000 {
            if keeps(f64::from(v)) {
                min.get_or_insert(f64::from(v));
                max = Some(f64::from(v));
            }
        }
        (min.unwrap_or(f64::NAN), max.unwrap_or(f64::NAN))
    })
}
