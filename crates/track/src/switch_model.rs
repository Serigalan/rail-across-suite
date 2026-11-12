// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch record and its ports (`switchModel.js`).

use rail_across_js::{generate_id, get, strict_eq, truthy};
use serde_json::{Value, json};

use crate::util::{obj, text};

pub const LINK_KIND: &str = "link";
/// A construction stage connector (decision 0027): three track ends, A joined to B1 while that exists and to B2
/// from then on — not a switch: never both at once.
pub const STAGE_LINK_KIND: &str = "stage_link";
pub const DEFAULT_SWITCH_KIND: &str = "turnout";
pub const SWITCH_FORM_VERSION: i64 = 1;

/// The routes of a kind and the ports each runs between (`switchRoutePorts`).
pub fn switch_route_ports(kind: Option<&str>) -> Vec<(&'static str, [&'static str; 2])> {
    match kind {
        Some("crossing") => vec![("main", ["A", "C"]), ("cross", ["B", "D"])],
        Some("single_slip") => vec![("main", ["A", "C"]), ("cross", ["B", "D"]), ("slip1", ["A", "D"])],
        Some("double_slip") => vec![("main", ["A", "C"]), ("cross", ["B", "D"]), ("slip1", ["A", "D"]), ("slip2", ["B", "C"])],
        Some("link" | "stage_link") => vec![],
        _ => vec![("main", ["A", "B2"]), ("branch", ["A", "B1"])],
    }
}

/// The route names of a kind (`switchRoutes`).
pub fn switch_routes(kind: Option<&str>) -> Vec<&'static str> {
    switch_route_ports(kind).into_iter().map(|(r, _)| r).collect()
}

pub fn turnout_line_port(sw: &Value) -> &'static str {
    if truthy(sw.get("swapped")) { "B1" } else { "B2" }
}
pub fn turnout_diverging_port(sw: &Value) -> &'static str {
    if truthy(sw.get("swapped")) { "B2" } else { "B1" }
}
pub fn turnout_line_route(sw: &Value) -> &'static str {
    if truthy(sw.get("swapped")) { "branch" } else { "main" }
}

/// The key a kind is named by in the interface (`switchKindLabelKey`).
pub fn switch_kind_label_key(kind: Option<&str>) -> &'static str {
    match kind {
        Some("crossing") => "table_type_crossing",
        Some("single_slip") | Some("double_slip") => "table_type_crossing_switch",
        Some("link") => "table_type_link",
        Some("stage_link") => "table_type_stage_link",
        _ => "table_type_switch",
    }
}

/// The key a route of a kind is named by (`switchRouteLabelKey`).
pub fn switch_route_label_key(kind: Option<&str>, route: &str) -> Option<&'static str> {
    let known = matches!(kind, Some("turnout" | "crossing" | "single_slip" | "double_slip" | "link" | "stage_link"));
    if known && !switch_route_ports(kind).iter().any(|(r, _)| *r == route) {
        return None;
    }
    let crossing_names = kind != Some(DEFAULT_SWITCH_KIND) && known;
    match (crossing_names, route) {
        (false, "main") => Some("table_route_stem"),
        (false, "branch") => Some("table_route_branch"),
        (true, "main") => Some("table_route_through"),
        (true, "cross") => Some("table_route_cross"),
        (true, "slip1" | "slip2") => Some("table_route_slip"),
        _ => None,
    }
}

/// Whether a switch record is a connector, no switch: a link, or a construction stage connector.
pub fn is_link_switch(sw: &Value) -> bool {
    matches!(text(sw, "kind"), Some(LINK_KIND | STAGE_LINK_KIND))
}

/// Whether a switch record is a construction stage connector.
pub fn is_stage_link(sw: &Value) -> bool {
    text(sw, "kind") == Some(STAGE_LINK_KIND)
}

/// The ports a connector joins a port to: every other for a link; for a stage link A to B1 and B2, each of
/// those to A alone — the old and the new track are never joined to each other.
pub fn joined_ports(sw: &Value, port: &str) -> Vec<&'static str> {
    let all = ports_of(sw);
    if is_stage_link(sw) {
        return if port == "A" { vec!["B1", "B2"] } else { vec!["A"] };
    }
    all.into_iter().map(|(p, _)| p).filter(|p| *p != port).collect()
}

/// The marks an element of a switch route carries (`switchElementMark`).
pub fn switch_element_mark(sw: &Value, route: &str) -> Value {
    let mut o = rail_across_js::Object::new();
    o.insert("switchBranch".into(), json!(true));
    o.insert("switchRoute".into(), json!(route));
    if let Some(id) = sw.get("switchId").filter(|v| truthy(Some(v))) {
        o.insert("switchId".into(), id.clone());
    }
    Value::Object(o)
}

pub fn element_belongs_to_switch(el: &Value, sw: &Value) -> bool {
    get(el, "switchId").is_some() && strict_eq(el.get("switchId"), sw.get("switchId"))
}

/// A new switch's identity (`newSwitchFields`).
pub fn new_switch_fields(kind: &str) -> Value {
    json!({ "switchId": generate_id(), "kind": kind, "formVersion": SWITCH_FORM_VERSION })
}

pub fn is_modelled_switch(sw: &Value) -> bool {
    truthy(sw.get("switchId")) && truthy(sw.get("kind")) && get(sw, "formVersion").is_some()
}

/// The ports of a kind, each with the route it lies on (`switchPorts`).
pub fn switch_ports(kind: Option<&str>) -> Vec<(&'static str, Option<&'static str>)> {
    match kind {
        Some("crossing" | "single_slip" | "double_slip") => vec![("A", Some("main")), ("B", Some("cross")), ("C", Some("main")), ("D", Some("cross"))],
        Some("link") => vec![("A", None), ("B", None)],
        Some("stage_link") => vec![("A", None), ("B1", None), ("B2", None)],
        _ => vec![("A", Some("main")), ("B1", Some("branch")), ("B2", Some("main"))],
    }
}

/// The ports of a switch record (`portsOf`).
pub fn ports_of(sw: &Value) -> Vec<(&'static str, Option<&'static str>)> {
    switch_ports(text(sw, "kind"))
}

/// The ports as the JavaScript lists them: [{ port, route }].
pub fn ports_json(ports: &[(&str, Option<&str>)]) -> Value {
    Value::Array(ports.iter().map(|(p, r)| json!({ "port": p, "route": r })).collect())
}

fn port<'a>(sw: &'a Value, port: &str) -> Option<&'a Value> {
    sw.get("ports").and_then(|p| p.get(port))
}

/// The track a port names (`portTrack`).
pub fn port_track<'a>(sw: &'a Value, p: &str) -> Option<&'a Value> {
    port(sw, p).and_then(|v| get(v, "trackId"))
}

/// The track a port names, as a string.
pub fn port_track_id<'a>(sw: &'a Value, p: &str) -> Option<&'a str> {
    port_track(sw, p).and_then(Value::as_str)
}

/// Which end of its track a port sits at (`portEnd`).
pub fn port_end<'a>(sw: &'a Value, p: &str) -> Option<&'a str> {
    port(sw, p).and_then(|v| get(v, "endpoint")).and_then(Value::as_str)
}

/// A switch with one port patched (`withPort`).
pub fn with_port(sw: &Value, p: &str, patch: &Value) -> Value {
    let mut out = obj(sw);
    let mut ports = sw.get("ports").map(obj).unwrap_or_default();
    let mut one = port(sw, p).map(obj).unwrap_or_default();
    for (k, v) in obj(patch) {
        one.insert(k, v);
    }
    ports.insert(p.to_string(), Value::Object(one));
    out.insert("ports".into(), Value::Object(ports));
    Value::Object(out)
}

pub fn element_on_switch_route(el: &Value, sw: &Value, route: &str) -> bool {
    truthy(el.get("switchBranch")) && element_belongs_to_switch(el, sw) && (!truthy(el.get("switchRoute")) || text(el, "switchRoute") == Some(route))
}

/// An element without its switch marks (`unmarkSwitchElement`).
pub fn unmark_switch_element(el: &Value) -> Value {
    Value::Object(crate::util::without(el, &["switchBranch", "switchRoute", "switchId", "cantException"]))
}
