// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The topology of a project as a graph (`topologyGraph.js`).

use std::cmp::Ordering;
use std::collections::HashMap;

use rail_across_js::{es_number, get, js_round, js_string, number, truthy};
use serde_json::{Map, Value, json};

use crate::end_marks::end_key;
use crate::groups::collate;
use crate::switch_model::{DEFAULT_SWITCH_KIND, is_link_switch, port_end, port_track, ports_of, turnout_line_port};
use crate::topology::classify_track_ends;

fn key(track_id: &Value, end: &str) -> String {
    end_key(Some(track_id), Some(&json!(end)))
}

/// Nodes (by id, in the order they were made) and edges (`buildTopologyGraph`).
pub fn build_topology_graph(tracks: &[Value], switches: &[Value], end_marks: &[Value]) -> (Map<String, Value>, Vec<Value>) {
    let ends = classify_track_ends(tracks, switches, end_marks);
    let ends = ends.as_array().cloned().unwrap_or_default();
    let mut nodes = Map::new();
    let mut node_of_end: HashMap<String, String> = HashMap::new();
    let mut port_of_end: HashMap<String, &str> = HashMap::new();
    for sw in switches {
        let id = format!("sw:{}", js_string(sw.get("switchId")));
        for (port, _) in ports_of(sw) {
            let (Some(t), Some(e)) = (port_track(sw, port), port_end(sw, port)) else {
                continue;
            };
            if !truthy(Some(t)) || e.is_empty() {
                continue;
            }
            node_of_end.insert(key(t, e), id.clone());
            port_of_end.insert(key(t, e), port);
        }
    }
    let mut at: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    let ll = |e: &Value| {
        let p = &e["lngLat"];
        (p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN))
    };
    for e in &ends {
        let k = end_key(e.get("trackId"), e.get("endpoint"));
        if let Some(sw_node) = node_of_end.get(&k) {
            at.entry(sw_node.clone()).or_default().push(ll(e));
        }
    }
    for sw in switches {
        let id = format!("sw:{}", js_string(sw.get("switchId")));
        let Some(pts) = at.get(&id).filter(|p| !p.is_empty()) else { continue };
        let n = pts.len() as f64;
        let mut node = Map::new();
        node.insert("id".into(), json!(id));
        node.insert("kind".into(), json!(if is_link_switch(sw) { "link" } else { "switch" }));
        node.insert("switchId".into(), sw.get("switchId").cloned().unwrap_or(Value::Null));
        node.insert("switchKind".into(), get(sw, "kind").cloned().unwrap_or(json!(DEFAULT_SWITCH_KIND)));
        node.insert("name".into(), get(sw, "name").cloned().unwrap_or(Value::Null));
        if truthy(sw.get("swapped")) {
            node.insert("linePort".into(), json!(turnout_line_port(sw)));
        }
        let (se, sn) = pts.iter().fold((0.0, 0.0), |(a, b), p| (a + p.0, b + p.1));
        node.insert("lngLat".into(), json!([number(se / n), number(sn / n)]));
        nodes.insert(id, Value::Object(node));
    }
    for e in &ends {
        let k = end_key(e.get("trackId"), e.get("endpoint"));
        if node_of_end.contains_key(&k) {
            continue;
        }
        if e.get("state").and_then(Value::as_str) == Some("joint") {
            let f = |key: &str| e.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN);
            let id = format!(
                "j:{}:{}:{}",
                js_string(e.get("epsg")),
                es_number(js_round(f("easting") * 100.0)),
                es_number(js_round(f("northing") * 100.0))
            );
            node_of_end.insert(k, id.clone());
            if !nodes.contains_key(&id) {
                nodes.insert(id.clone(), json!({ "id": id, "kind": "joint", "lngLat": e["lngLat"] }));
            }
        } else {
            let id = format!("e:{k}");
            node_of_end.insert(k, id.clone());
            nodes.insert(id.clone(), json!({ "id": id, "kind": "end", "state": e.get("state"), "lngLat": e["lngLat"] }));
        }
    }
    let mut edges = Vec::new();
    for tr in tracks {
        let id = tr.get("id").cloned().unwrap_or(Value::Null);
        let (Some(from), Some(to)) = (node_of_end.get(&key(&id, "BEGIN")), node_of_end.get(&key(&id, "END"))) else {
            continue;
        };
        let els = tr.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        let coords = match tr.get("coordinates").and_then(Value::as_array).filter(|c| !c.is_empty()) {
            Some(c) => c.clone(),
            None => els
                .iter()
                .flat_map(|el| {
                    el.get("geometry")
                        .and_then(|g| get(g, "coordinates"))
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect(),
        };
        let port = |end: &str| port_of_end.get(&key(&id, end)).map(|p| json!(p)).unwrap_or(Value::Null);
        edges.push(json!({
            "trackId": id, "name": get(tr, "name").cloned().unwrap_or(Value::Null), "from": from, "to": to,
            "fromPort": port("BEGIN"), "toPort": port("END"),
            "length": number(els.iter().map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum()),
            "coords": coords,
        }));
    }
    (nodes, edges)
}

/// `a.localeCompare(b, undefined, { numeric: true })`: digits by value, then
/// case and accents only where all else is equal, lower case first.
pub fn locale_compare_numeric(a: &str, b: &str) -> Ordering {
    collate(a, b).then_with(|| {
        // Swapping the case puts lower case before upper case, as ICU does.
        let flip = |s: &str| {
            s.chars()
                .map(|c| {
                    if c.is_lowercase() {
                        c.to_uppercase().next().unwrap_or(c)
                    } else {
                        c.to_lowercase().next().unwrap_or(c)
                    }
                })
                .collect::<String>()
        };
        flip(a).cmp(&flip(b))
    })
}

/// The connected parts of the graph and the loose tracks (`topologyClusters`).
pub fn topology_clusters(nodes: &Map<String, Value>, edges: &[Value]) -> Value {
    let mut parent: HashMap<String, String> = nodes.keys().map(|k| (k.clone(), k.clone())).collect();
    fn find(parent: &mut HashMap<String, String>, x: &str) -> String {
        let mut x = x.to_string();
        loop {
            let p = parent.get(&x).cloned().unwrap_or_else(|| x.clone());
            if p == x {
                return x;
            }
            let gp = parent.get(&p).cloned().unwrap_or_else(|| p.clone());
            parent.insert(x.clone(), gp.clone());
            x = gp;
        }
    }
    let s = |e: &Value, k: &str| js_string(e.get(k));
    for e in edges {
        let (a, b) = (find(&mut parent, &s(e, "from")), find(&mut parent, &s(e, "to")));
        parent.insert(a, b);
    }
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (Vec<String>, Vec<Value>)> = HashMap::new();
    for e in edges {
        let root = find(&mut parent, &s(e, "from"));
        let g = groups.entry(root.clone()).or_insert_with(|| {
            order.push(root.clone());
            (Vec::new(), Vec::new())
        });
        g.1.push(e.clone());
        for k in ["from", "to"] {
            let id = s(e, k);
            if !g.0.contains(&id) {
                g.0.push(id);
            }
        }
    }
    let mut clusters: Vec<(usize, Value)> = Vec::new();
    let mut loose: Vec<Value> = Vec::new();
    for root in order {
        let (ids, es) = groups.remove(&root).unwrap_or_default();
        let members: Vec<Value> = ids.iter().map(|id| nodes.get(id).cloned().unwrap_or(Value::Null)).collect();
        if es.len() == 1 && members.iter().all(|n| n.get("kind").and_then(Value::as_str) == Some("end")) {
            loose.push(es[0].clone());
        } else {
            clusters.push((es.len(), json!({ "nodes": members, "edges": es })));
        }
    }
    clusters.sort_by_key(|c| std::cmp::Reverse(c.0));
    let name = |e: &Value| get(e, "name").map(|n| js_string(Some(n))).unwrap_or_default();
    loose.sort_by(|a, b| locale_compare_numeric(&name(a), &name(b)));
    json!({ "clusters": clusters.into_iter().map(|c| c.1).collect::<Vec<_>>(), "loose": loose })
}

pub const TOPOLOGY_TRACK_COLORS: [&str; 6] = ["#ff8c00", "#1f9e3a", "#d0308f", "#0f9fb5", "#7b3fbf", "#a0662b"];

/// The tracks a switch's ports name (`switchTrackIds`).
pub fn switch_track_ids(sw: &Value) -> Vec<Value> {
    ports_of(sw)
        .into_iter()
        .filter_map(|(p, _)| port_track(sw, p).filter(|t| truthy(Some(t))).cloned())
        .collect()
}

/// What a selection in the topology view highlights (`selectionHighlight`).
pub fn selection_highlight(selection: Option<&Value>, switches: &[Value]) -> Value {
    let none = json!({ "trackIds": [], "switchIds": [], "colors": {} });
    let Some(sel) = selection.filter(|s| truthy(Some(s))) else { return none };
    let colored = |ids: &[Value]| -> Value {
        Value::Object(
            ids.iter()
                .enumerate()
                .map(|(k, id)| (js_string(Some(id)), json!(TOPOLOGY_TRACK_COLORS[k % TOPOLOGY_TRACK_COLORS.len()])))
                .collect(),
        )
    };
    let id = sel.get("id").cloned().unwrap_or(Value::Null);
    if sel.get("kind").and_then(Value::as_str) == Some("switch") {
        let Some(sw) = switches.iter().find(|s| rail_across_js::strict_eq(s.get("switchId"), Some(&id))) else {
            return none;
        };
        let ids = switch_track_ids(sw);
        return json!({ "trackIds": ids, "switchIds": [sw.get("switchId")], "colors": colored(&ids) });
    }
    let ids = vec![id.clone()];
    json!({
        "trackIds": ids,
        "switchIds": switches.iter().filter(|sw| switch_track_ids(sw).iter().any(|t| rail_across_js::strict_eq(Some(t), Some(&id)))).map(|sw| sw.get("switchId").cloned().unwrap_or(Value::Null)).collect::<Vec<_>>(),
        "colors": colored(&ids),
    })
}
