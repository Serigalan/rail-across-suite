// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! How the element table may change each element (`elementRoles.js`).

use rail_across_js::get;
use serde_json::{Value, json};

fn is_transition(el: Option<&Value>) -> bool {
    el.and_then(|e| e.get("elementType")).and_then(Value::as_f64) == Some(2.0)
}

fn in_switch(el: Option<&Value>) -> bool {
    el.and_then(|e| get(e, "switchId")).is_some()
}

/// The elements a Zwischenelement is worked out between (`intermediateSpan`): (a, b) or the error key.
pub fn intermediate_span(elements: &[Value], z: usize) -> Result<(usize, usize), &'static str> {
    let el = elements.get(z);
    if el.is_none() || is_transition(el) || in_switch(el) {
        return Err("table_role_intermediate_kind");
    }
    let mut a = z as i64 - 1;
    while a >= 0 && is_transition(elements.get(a as usize)) {
        a -= 1;
    }
    let mut b = z + 1;
    while b < elements.len() && is_transition(elements.get(b)) {
        b += 1;
    }
    if a < 0 || b >= elements.len() {
        return Err("table_role_intermediate_ends");
    }
    if elements[a as usize..=b].iter().any(|e| in_switch(Some(e))) {
        return Err("table_role_intermediate_switch");
    }
    Ok((a as usize, b))
}

pub fn span_json(r: Result<(usize, usize), &'static str>) -> Value {
    match r {
        Ok((a, b)) => json!({ "a": a, "b": b, "error": null }),
        Err(e) => json!({ "error": e }),
    }
}

fn role(el: &Value) -> Option<&str> {
    el.get("role").and_then(Value::as_str)
}

/// The role each element has (`elementRoles`).
pub fn element_roles(elements: &[Value]) -> Vec<Value> {
    let mut out: Vec<Value> = elements
        .iter()
        .map(|_| json!({ "role": "fixed", "by": null, "length": false, "radius": false, "error": null }))
        .collect();
    let from = elements.iter().position(|e| role(e) == Some("follow")).unwrap_or(elements.len());
    for i in from..elements.len() {
        let own = !in_switch(Some(&elements[i]));
        out[i] = json!({ "role": "follow", "by": if i == from { Value::Null } else { json!(from) }, "error": null, "length": own, "radius": own && !is_transition(Some(&elements[i])) });
    }
    let mut taken: i64 = -1;
    for (z, el) in elements.iter().enumerate() {
        if role(el) != Some("intermediate") || z >= from {
            continue;
        }
        let span = intermediate_span(elements, z);
        let error = match span {
            Err(e) => Some(e),
            Ok((_, b)) if b >= from => Some("table_role_intermediate_follow"),
            Ok((a, _)) if a as i64 <= taken => Some("table_role_intermediate_overlap"),
            _ => None,
        };
        if let Some(e) = error {
            out[z]["error"] = json!(e);
            continue;
        }
        let (a, b) = span.expect("a span");
        for (i, slot) in out.iter_mut().enumerate().take(b + 1).skip(a) {
            *slot = if i == z {
                json!({ "role": "intermediate", "by": null, "length": false, "radius": true, "error": null, "span": span_json(span) })
            } else {
                json!({ "role": "follow", "by": z, "length": false, "radius": false, "error": null })
            };
        }
        taken = b as i64;
    }
    out
}

/// The roles an element may be given (`rolesFor`).
pub fn roles_for(el: &Value) -> Vec<&'static str> {
    if is_transition(Some(el)) || in_switch(Some(el)) {
        vec!["fixed", "follow"]
    } else {
        vec!["fixed", "follow", "intermediate"]
    }
}

fn without_role(e: &Value) -> Value {
    let mut o = e.as_object().cloned().unwrap_or_default();
    o.remove("role");
    Value::Object(o)
}

fn with(e: &Value, r: &str) -> Value {
    let mut o = e.clone();
    o["role"] = json!(r);
    o
}

/// The elements with element `i` given `role` (`withRole`): (elements, error).
pub fn with_role(elements: &[Value], i: usize, r: &str) -> (Vec<Value>, Option<&'static str>) {
    if elements.get(i).is_none() {
        return (elements.to_vec(), None);
    }
    if r == "fixed" {
        return (
            elements
                .iter()
                .enumerate()
                .map(|(j, e)| if j == i { without_role(e) } else { e.clone() })
                .collect(),
            None,
        );
    }
    if r == "follow" {
        let out = elements
            .iter()
            .enumerate()
            .map(|(j, e)| {
                if j == i {
                    with(e, "follow")
                } else if j > i || (role(e) == Some("intermediate") && intermediate_span(elements, j).map(|s| s.1 as i64).unwrap_or(-1) >= i as i64) {
                    without_role(e)
                } else {
                    e.clone()
                }
            })
            .collect();
        return (out, None);
    }
    let (a, b) = match intermediate_span(elements, i) {
        Ok(s) => s,
        Err(e) => return (elements.to_vec(), Some(e)),
    };
    let out = elements
        .iter()
        .enumerate()
        .map(|(j, e)| {
            if j == i {
                return with(e, "intermediate");
            }
            if j >= a && j <= b {
                return without_role(e);
            }
            if role(e) == Some("intermediate")
                && let Ok((oa, ob)) = intermediate_span(elements, j)
                && oa <= b
                && ob >= a
            {
                return without_role(e);
            }
            e.clone()
        })
        .collect();
    (out, None)
}
