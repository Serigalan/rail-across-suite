// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The rule catalogue applied to the app's own elements, answered in the form
//! of `alignmentCheck.js` (`checkTrack`, `elementVerdict`): every result
//! with the values its rule read and computed, per element, per joint and per
//! ramp — what the element table shows. The scopes are [`crate::pruefung`]'s.

use rail_across_js::number;
use serde_json::{Value as Json, json};

use crate::alignment_check::{Context, boundary_scope, element_scope};
use crate::bundled::{catalogue, severity_rank, worst_severity};
use crate::catalogue::{Rule, Source};
use crate::element::Element;
use crate::expr::{Error, Value};

fn to_json(v: &Value) -> Json {
    match v {
        Value::Num(x) => number(*x),
        Value::Bool(b) => json!(b),
        Value::Str(s) => json!(&**s),
    }
}

/// One rule applied as `evaluateRule` applies it: `{ id, applied, severity, values, outOfRange? }`,
/// None where its condition leaves the object alone.
fn evaluate(rule: &Rule, base: &[(&str, Value)], scope: &crate::expr::Scope, in_context: Option<&dyn Fn(&str) -> bool>) -> Result<Option<Json>, Error> {
    let k = catalogue();
    let outcome = match k.evaluate_rule(rule, scope, in_context) {
        Ok(o) => o,
        Err(Error::OutOfRange(severity)) => crate::catalogue::Outcome {
            applied: true,
            severity: Some(severity),
            values: scope.clone(),
            out_of_range: true,
        },
        Err(e) => return Err(e),
    };
    if !outcome.applied {
        return Ok(None);
    }
    let mut values = serde_json::Map::new();
    for (name, v) in base {
        values.insert((*name).to_string(), to_json(v));
    }
    let names = rule.inputs.iter().map(|i| i.name).chain(rule.thresholds.iter().map(|(s, _)| *s));
    for sym in names {
        if let Some(v) = outcome.values.get(sym) {
            values.insert(k.symbols().name(sym).to_string(), to_json(v));
        }
    }
    let _ = Source::Expr;
    let mut out = json!({ "id": rule.id, "applied": true, "severity": outcome.severity, "values": values });
    if outcome.out_of_range {
        out["outOfRange"] = json!(true);
    }
    Ok(Some(out))
}

/// `evaluateRules`: the results of the rules that apply, and their worst severity.
fn evaluate_rules(
    rules: &[&Rule],
    base: &[(&str, Value)],
    in_context: Option<&dyn Fn(&str) -> bool>,
    excluded: &[&str],
) -> Result<(Vec<Json>, Option<String>), Error> {
    let mut results = Vec::new();
    let scope = catalogue().scope(base);
    for rule in rules {
        if rule.excludes.iter().any(|e| excluded.contains(&e.as_str())) {
            continue;
        }
        if let Some(r) = evaluate(rule, base, &scope, in_context)? {
            results.push(r);
        }
    }
    let worst = worst_severity(results.iter().map(|r| r["severity"].as_str())).map(str::to_string);
    Ok((results, worst))
}

fn checkable(el: &Element) -> bool {
    el.speed.is_some_and(|v| v != 0.0 && !v.is_nan())
}

fn entry(index: usize, results: Vec<Json>, severity: Option<String>) -> Json {
    json!({ "index": index, "results": results, "severity": severity })
}

fn element_entry(els: &[Element], i: usize, context: Context) -> Result<Option<Json>, Error> {
    let el = &els[i];
    if !checkable(el) {
        return Ok(None);
    }
    let in_switch = el.in_switch();
    let ctx = move |id: &str| id == "switch_area" && in_switch;
    let k = catalogue();
    let (results, severity) = evaluate_rules(&k.rules_for_element(el.type_id(), el.form()), &element_scope(els, i, context), Some(&ctx), &[])?;
    Ok(Some(entry(i, results, severity)))
}

fn ramp_entry(els: &[Element], i: usize) -> Result<Option<Json>, Error> {
    let el = &els[i];
    if !checkable(el) {
        return Ok(None);
    }
    let du = element_scope(els, i, Context::default())
        .iter()
        .find(|(n, _)| *n == "physics.delta_u")
        .and_then(|(_, v)| v.as_num())
        .unwrap_or(0.0);
    if !(du > 0.0) {
        return Ok(None);
    }
    let base = [
        ("model.ramp_on_transition", el.is_transition().into()),
        ("model.ramp_form_matches", (el.form() == Some("clothoid")).into()),
    ];
    let k = catalogue();
    let (results, severity) = evaluate_rules(&k.rules_for_scope("cant_ramp"), &base, None, &[])?;
    Ok(Some(entry(i, results, severity)))
}

fn boundary_entry(els: &[Element], i: usize) -> Result<Option<Json>, Error> {
    let (Some(a), Some(b)) = (els.get(i), els.get(i + 1)) else { return Ok(None) };
    if !checkable(a) || !checkable(b) {
        return Ok(None);
    }
    let internal = a.in_switch() && b.in_switch() && a.switch_id == b.switch_id;
    let either = a.in_switch() || b.in_switch();
    let ctx = move |id: &str| id == "switch_area" && either;
    let excluded: &[&str] = if internal { &["switch_internal"] } else { &[] };
    let k = catalogue();
    let (results, severity) = evaluate_rules(&k.rules_for_scope("boundary"), &boundary_scope(els, i), Some(&ctx), excluded)?;
    Ok(Some(entry(i, results, severity)))
}

fn placed(entry: Option<&Json>, at: &str) -> Vec<Json> {
    entry
        .and_then(|e| e["results"].as_array())
        .into_iter()
        .flatten()
        .map(|r| {
            let mut r = r.clone();
            r["at"] = json!(at);
            r
        })
        .collect()
}

/// Every rule that speaks about element `i` itself, each with `at` (`elementVerdict`).
pub fn element_verdict(els: &[Element], i: usize, context: Context) -> Result<Vec<Json>, Error> {
    let element = element_entry(els, i, context)?;
    let ramp = ramp_entry(els, i)?;
    let start = if i > 0 { boundary_entry(els, i - 1)? } else { None };
    let end = boundary_entry(els, i)?;
    let mut out = placed(element.as_ref(), "element");
    out.extend(placed(ramp.as_ref(), "ramp"));
    out.extend(placed(start.as_ref(), "start"));
    out.extend(placed(end.as_ref(), "end"));
    Ok(out)
}

/// Every rule of the catalogue applied to one element chain (`checkTrack`).
pub fn check_track(els: &[Element], context: Context) -> Result<Json, Error> {
    let mut elements = Vec::new();
    for i in 0..els.len() {
        elements.push(element_entry(els, i, context)?.unwrap_or_else(|| json!({ "index": i, "severity": null, "results": [], "unchecked": true })));
    }
    let mut ramps = Vec::new();
    for i in 0..els.len() {
        if let Some(r) = ramp_entry(els, i)? {
            ramps.push(r);
        }
    }
    let mut boundaries = Vec::new();
    for i in 0..els.len().saturating_sub(1) {
        if let Some(b) = boundary_entry(els, i)? {
            boundaries.push(b);
        }
    }
    let at = |list: &[Json], index: i64| list.iter().find(|e| e["index"].as_i64() == Some(index)).cloned();
    let per_element: Vec<Json> = elements
        .iter()
        .map(|entry| {
            let index = entry["index"].as_i64().unwrap_or(0);
            let mut results = placed(Some(entry), "element");
            results.extend(placed(at(&ramps, index).as_ref(), "ramp"));
            results.extend(placed(at(&boundaries, index - 1).as_ref(), "start"));
            let severity = worst_severity(results.iter().map(|r| r["severity"].as_str())).map(str::to_string);
            json!({ "index": index, "unchecked": entry.get("unchecked").and_then(Json::as_bool).unwrap_or(false), "results": results, "severity": severity })
        })
        .collect();
    let severity = worst_severity(per_element.iter().map(|e| e["severity"].as_str())).map(str::to_string);
    Ok(json!({ "perElement": per_element, "elements": elements, "boundaries": boundaries, "ramps": ramps, "severity": severity }))
}

/// Does the catalogue call anything about this chain an error (`hasRuleError`)?
pub fn has_rule_error(els: &[Element], context: Context) -> Result<bool, Error> {
    let checked = check_track(els, context)?;
    Ok(severity_rank(checked["severity"].as_str()) >= severity_rank(Some("error")))
}

// ── Without the values ───────────────────────────────────────────────────────

/// The errors among the rules that speak about element `i` itself, as
/// "at:id" — what the search for the highest speed compares, without the
/// values each rule read (`elementVerdict` filtered to errors).
pub fn verdict_errors(els: &[Element], i: usize, context: Context) -> Result<Vec<String>, Error> {
    let k = catalogue();
    let error = severity_rank(Some("error"));
    let mut out = Vec::new();
    let mut apply = |rules: &[&Rule], base: &[(&str, Value)], in_context: Option<&dyn Fn(&str) -> bool>, excluded: &[&str], at: &str| -> Result<(), Error> {
        let scope = k.scope(base);
        for rule in rules {
            if rule.excludes.iter().any(|e| excluded.contains(&e.as_str())) {
                continue;
            }
            let severity = match k.evaluate_rule(rule, &scope, in_context) {
                Ok(o) if o.applied => o.severity,
                Ok(_) => continue,
                Err(Error::OutOfRange(s)) => Some(s),
                Err(e) => return Err(e),
            };
            if severity_rank(severity.as_deref()) >= error {
                out.push(format!("{at}:{}", rule.id));
            }
        }
        Ok(())
    };
    let el = &els[i];
    if checkable(el) {
        let in_switch = el.in_switch();
        let ctx = move |id: &str| id == "switch_area" && in_switch;
        let base = element_scope(els, i, context);
        apply(&k.rules_for_element(el.type_id(), el.form()), &base, Some(&ctx), &[], "element")?;
        let du = base.iter().find(|(n, _)| *n == "physics.delta_u").and_then(|(_, v)| v.as_num()).unwrap_or(0.0);
        let du0 = if context.before.is_some() || context.after.is_some() {
            element_scope(els, i, Context::default())
                .iter()
                .find(|(n, _)| *n == "physics.delta_u")
                .and_then(|(_, v)| v.as_num())
                .unwrap_or(0.0)
        } else {
            du
        };
        if du0 > 0.0 {
            let ramp = [
                ("model.ramp_on_transition", el.is_transition().into()),
                ("model.ramp_form_matches", (el.form() == Some("clothoid")).into()),
            ];
            apply(&k.rules_for_scope("cant_ramp"), &ramp, None, &[], "ramp")?;
        }
    }
    for (at, j) in [("start", i.checked_sub(1)), ("end", Some(i))] {
        let Some(j) = j else { continue };
        let (Some(a), Some(b)) = (els.get(j), els.get(j + 1)) else { continue };
        if !checkable(a) || !checkable(b) {
            continue;
        }
        let internal = a.in_switch() && b.in_switch() && a.switch_id == b.switch_id;
        let either = a.in_switch() || b.in_switch();
        let ctx = move |id: &str| id == "switch_area" && either;
        let excluded: &[&str] = if internal { &["switch_internal"] } else { &[] };
        apply(&k.rules_for_scope("boundary"), &boundary_scope(els, j), Some(&ctx), excluded, at)?;
    }
    Ok(out)
}
