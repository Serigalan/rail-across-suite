// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! How a rule on a transition's length writes the bound it sets, in the
//! Ril's symbols (`transitionLength.js` `lengthRuleFormula`).

use rail_across_js::{es_number, js_string};
use serde_json::Value as Json;

use crate::bundled::{katalog_json, severity_rank};
use crate::expr::{Basic, Value, eval_expr};

fn rule_json(id: &str) -> Option<&'static Json> {
    katalog_json()["rules"].as_array()?.iter().find(|r| r["id"].as_str() == Some(id))
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `new RegExp(`\\b${name}\\b`).test(text)`.
fn has_word(text: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(i) = text[from..].find(name) {
        let at = from + i;
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        let starts = !name.starts_with(is_word) || before.is_none_or(|c| !is_word(c));
        let ends = !name.ends_with(is_word) || after.is_none_or(|c| !is_word(c));
        if starts && ends {
            return true;
        }
        from = at + name.len().max(1);
    }
    false
}

fn bound_name(rule: &Json, worst: &str) -> Option<String> {
    let mut entries: Vec<&Json> = rule["evaluation"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e.get("if").is_some() && severity_rank(e["severity"].as_str()) <= severity_rank(Some(worst)))
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(severity_rank(e["severity"].as_str())));
    let cond = entries.first().and_then(|e| e["if"].as_str()).unwrap_or("");
    rule["thresholds"].as_object()?.keys().find(|n| has_word(cond, n)).cloned()
}

fn is_slope(rule: &Json) -> bool {
    rule["inputs"]
        .as_object()
        .is_some_and(|i| i.values().any(|v| v["from"].as_str() == Some("physics.ramp_slope")))
}

fn value_for_form(rule: &Json, expr: &str, form: Option<&str>) -> Option<f64> {
    let names: Vec<&String> = rule["inputs"]
        .as_object()?
        .iter()
        .filter(|(_, v)| v["from"].as_str() == Some("element.transition_form"))
        .map(|(n, _)| n)
        .collect();
    let form = form.filter(|f| !f.is_empty())?;
    if names.is_empty() {
        return None;
    }
    let scope: Vec<(&str, Value)> = names.iter().map(|n| (n.as_str(), Value::str(form))).collect();
    eval_expr(expr, &scope, &Basic).ok()?.as_num().filter(|v| v.is_finite())
}

/// The formula `*` and `/` written as the Ril writes them.
fn pretty(expr: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '*' || c == '/' {
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            out.push(if c == '*' { '·' } else { '/' });
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out.replace("delta_u_f", "Δu_f").replace("delta_u", "Δu")
}

/// The bound a rule on the length sets at `worst`, as a formula — or None (`lengthRuleFormula`).
pub fn length_rule_formula(id: &str, worst: &str, form: Option<&str>) -> Option<String> {
    let rule = rule_json(id)?;
    let name = bound_name(rule, worst)?;
    let expr_v = &rule["thresholds"][&name]["expr"];
    let expr = js_string(Some(expr_v));
    let shown = value_for_form(rule, &expr, form).map(es_number).unwrap_or_else(|| expr.clone());
    if is_slope(rule) {
        return Some(format!("{}·Δu/1000", pretty(&shown)));
    }
    if expr.contains("lookup(") { None } else { Some(pretty(&shown)) }
}
