// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What stays when a switch is deleted (`switchDelete.js`).

use std::collections::HashSet;

use rail_across_js::{get, js_number, js_string, number, truthy};
use rail_across_track::heights::{split_heights, track_length};
use rail_across_track::merge::merge_chain;
use rail_across_track::model::{join_tracks, rebuild_coords, reverse_track};
use rail_across_track::switch_model::{
    element_belongs_to_switch, element_on_switch_route, is_link_switch, port_end, port_track, ports_of, switch_route_ports, unmark_switch_element,
};
use serde_json::{Map, Value, json};

/// What hangs on one port of a switch.
#[derive(Clone, Debug)]
pub struct Port {
    pub port: &'static str,
    pub route: Option<&'static str>,
    pub track_id: Option<Value>,
    pub endpoint: Option<String>,
    pub track: Option<Value>,
    pub mine: Vec<Value>,
    pub beyond: Vec<Value>,
    pub occupied: bool,
}

impl Port {
    pub fn to_json(&self) -> Value {
        json!({
            "port": self.port, "route": self.route, "trackId": self.track_id, "endpoint": self.endpoint, "track": self.track,
            "mine": self.mine, "beyond": self.beyond, "occupied": self.occupied,
        })
    }
    fn id(&self) -> Option<String> {
        self.track_id.as_ref().filter(|v| truthy(Some(v))).map(|v| js_string(Some(v)))
    }
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The tracks by id.
pub fn by_id(tracks: &[Value]) -> Map<String, Value> {
    tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect()
}

/// What hangs on each port (`switchParts`).
pub fn switch_parts(sw: &Value, by: &Map<String, Value>) -> Vec<Port> {
    ports_of(sw)
        .into_iter()
        .map(|(port, route)| {
            let track_id = port_track(sw, port).cloned();
            let endpoint = port_end(sw, port).map(str::to_string);
            let track = track_id
                .as_ref()
                .filter(|v| truthy(Some(v)))
                .and_then(|id| by.get(&js_string(Some(id))))
                .cloned();
            let e: Vec<Value> = track.as_ref().map(|t| els(t).to_vec()).unwrap_or_default();
            let at_end = endpoint.as_deref() == Some("END");
            let ordered: Vec<&Value> = if at_end { e.iter().rev().collect() } else { e.iter().collect() };
            let n = ordered.iter().take_while(|el| element_on_switch_route(el, sw, route.unwrap_or(""))).count();
            let (mine, beyond) = if at_end {
                (e[e.len() - n..].to_vec(), e[..e.len() - n].to_vec())
            } else {
                (e[..n].to_vec(), e[n..].to_vec())
            };
            let occupied = track.is_some() && (at_end || endpoint.as_deref() == Some("BEGIN")) && e.len() > n;
            Port {
                port,
                route,
                track_id: Some(track_id.unwrap_or(Value::Null)),
                endpoint,
                track,
                mine,
                beyond,
                occupied,
            }
        })
        .collect()
}

pub fn parts_json(parts: &[Port]) -> Value {
    json!({
        "ports": parts.iter().map(Port::to_json).collect::<Vec<_>>(),
        "byPort": Value::Object(parts.iter().map(|p| (p.port.to_string(), p.to_json())).collect()),
    })
}

fn find<'a>(parts: &'a [Port], port: &str) -> Option<&'a Port> {
    parts.iter().find(|p| p.port == port)
}

/// The routes that stay as ordinary track (`keptRoutes`).
pub fn kept_routes(sw: &Value, parts: &[Port]) -> Vec<&'static str> {
    let mut claimed: HashSet<&str> = HashSet::new();
    let mut kept = Vec::new();
    let mut routes = switch_route_ports(get(sw, "kind").and_then(Value::as_str));
    if truthy(sw.get("swapped")) {
        routes.reverse();
    }
    for (route, ports) in routes {
        if ports.iter().any(|p| claimed.contains(p)) {
            continue;
        }
        if !ports.iter().all(|p| find(parts, p).is_some_and(|x| x.occupied)) {
            continue;
        }
        claimed.extend(ports);
        kept.push(route);
    }
    kept
}

fn trim_track(track: &Value, endpoint: Option<&str>, drop: &[Value]) -> Value {
    let e = els(track);
    let keep: Vec<Value> = if endpoint == Some("END") {
        e[..e.len() - drop.len()].to_vec()
    } else {
        e[drop.len()..].to_vec()
    };
    let cut: f64 = drop.iter().map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum();
    let hs = get(track, "heights").and_then(Value::as_array).map(Vec::as_slice);
    let heights = if endpoint == Some("END") {
        split_heights(hs, 0f64.max(track_length(track) - cut)).0
    } else {
        split_heights(hs, cut).1
    };
    let mut rest = track.as_object().cloned().unwrap_or_default();
    rest.remove("heights");
    rest.insert("coordinates".into(), Value::Array(rebuild_coords(&keep)));
    rest.insert("elements".into(), Value::Array(keep));
    if let Some(h) = heights.filter(|h| !h.is_empty()) {
        rest.insert("heights".into(), Value::Array(h));
    }
    Value::Object(rest)
}

fn unmark_on(track: &Value, sw: &Value) -> Value {
    let mut t = track.clone();
    t["elements"] = Value::Array(
        els(track)
            .iter()
            .map(|el| {
                if truthy(el.get("switchBranch")) && element_belongs_to_switch(el, sw) {
                    unmark_switch_element(el)
                } else {
                    el.clone()
                }
            })
            .collect(),
    );
    t
}

/// Updates kept in the order first made, by track id.
#[derive(Default)]
struct Updates(Vec<(String, Value)>);

impl Updates {
    fn get(&self, id: &str) -> Option<&Value> {
        self.0.iter().find(|e| e.0 == id).map(|e| &e.1)
    }
    fn set(&mut self, id: String, v: Value) {
        match self.0.iter_mut().find(|e| e.0 == id) {
            Some(e) => e.1 = v,
            None => self.0.push((id, v)),
        }
    }
    fn delete(&mut self, id: &str) {
        self.0.retain(|e| e.0 != id);
    }
    fn values_except(&self, gone: &[String]) -> Vec<Value> {
        self.0.iter().filter(|e| !gone.contains(&e.0)).map(|e| e.1.clone()).collect()
    }
}

fn name_or_id(track: &Value, id: &str) -> Value {
    get(track, "name").cloned().unwrap_or_else(|| json!(id))
}

fn plan(
    switch_id: &Value,
    reason: &str,
    remove: &[String],
    updates: &Updates,
    remap: Vec<Value>,
    removed_elements: usize,
    removed_tracks: Vec<Value>,
    merged: (f64, f64),
    joined: bool,
) -> Value {
    json!({
        "switchId": switch_id, "reason": reason,
        "removeTrackIds": remove, "updateTracks": updates.values_except(remove), "remap": remap,
        "removedElements": removed_elements, "removedTracks": removed_tracks,
        "mergedElements": number(merged.0), "mergedInto": number(merged.1), "joined": joined,
    })
}

fn plan_crossing_deletion(sw: &Value, by: &Map<String, Value>) -> Value {
    let parts = switch_parts(sw, by);
    let routes = kept_routes(sw, &parts);
    let kept_slips: HashSet<&str> = routes.iter().copied().filter(|r| *r != "main" && *r != "cross").collect();
    let route_ports = switch_route_ports(get(sw, "kind").and_then(Value::as_str));
    let kept_ports: HashSet<&str> = routes
        .iter()
        .filter(|r| **r == "main" || **r == "cross")
        .flat_map(|r| route_ports.iter().find(|x| x.0 == *r).map(|x| x.1.to_vec()).unwrap_or_default())
        .collect();
    let mut updates = Updates::default();
    let mut remove: Vec<String> = Vec::new();
    let mut removed_tracks: Vec<Value> = Vec::new();
    let mut removed_elements = 0;
    for port in &parts {
        let (Some(track), Some(id)) = (&port.track, port.id()) else { continue };
        if port.mine.is_empty() {
            continue;
        }
        let current = updates.get(&id).cloned().unwrap_or_else(|| track.clone());
        if kept_ports.contains(port.port) {
            updates.set(id, unmark_on(&current, sw));
            continue;
        }
        removed_elements += port.mine.len();
        if port.beyond.is_empty() {
            removed_tracks.push(name_or_id(track, &id));
            remove.push(id);
        } else {
            updates.set(id, trim_track(&current, port.endpoint.as_deref(), &port.mine));
        }
    }
    let legs: HashSet<String> = parts.iter().filter_map(Port::id).collect();
    for (tid, track) in by {
        if legs.contains(tid) || remove.contains(tid) {
            continue;
        }
        let mine: Vec<&Value> = els(track)
            .iter()
            .filter(|el| element_belongs_to_switch(el, sw) && matches!(el.get("switchRoute").and_then(Value::as_str), Some("slip1") | Some("slip2")))
            .collect();
        let Some(first) = mine.first() else { continue };
        if kept_slips.contains(first.get("switchRoute").and_then(Value::as_str).unwrap_or("")) {
            updates.set(tid.clone(), unmark_on(track, sw));
            continue;
        }
        removed_elements += mine.len();
        if mine.len() == els(track).len() {
            removed_tracks.push(name_or_id(track, tid));
            remove.push(tid.clone());
        } else {
            let mut t = track.clone();
            let kept: Vec<Value> = els(track).iter().filter(|el| !mine.iter().any(|m| std::ptr::eq(*m, *el))).cloned().collect();
            t["elements"] = Value::Array(kept);
            updates.set(tid.clone(), t);
        }
    }
    let reason = if !kept_slips.is_empty() {
        "crossing_slip"
    } else if routes.len() == 2 {
        "crossing_both"
    } else if routes.len() == 1 {
        "crossing_one"
    } else {
        "all"
    };
    plan(
        sw.get("switchId").unwrap_or(&Value::Null),
        reason,
        &remove,
        &updates,
        vec![],
        removed_elements,
        removed_tracks,
        (0.0, 0.0),
        false,
    )
}

/// What deleting a switch would do (`planSwitchDeletion`).
pub fn plan_switch_deletion(sw: &Value, tracks: &[Value]) -> Value {
    if !truthy(sw.get("switchId")) {
        return Value::Null;
    }
    let sid = sw.get("switchId").cloned().unwrap_or(Value::Null);
    if is_link_switch(sw) {
        return plan(&sid, "link", &[], &Updates::default(), vec![], 0, vec![], (0.0, 0.0), false);
    }
    let by = by_id(tracks);
    if truthy(sw.get("kind")) && sw.get("kind").and_then(Value::as_str) != Some("turnout") {
        return plan_crossing_deletion(sw, &by);
    }
    let parts = switch_parts(sw, &by);
    let (Some(a), Some(b1), Some(b2)) = (find(&parts, "A"), find(&parts, "B1"), find(&parts, "B2")) else {
        return Value::Null;
    };
    let routes = kept_routes(sw, &parts);
    let keep_through = routes.contains(&"main");
    let keep_branch = routes.contains(&"branch");
    let kept = if keep_through {
        Some(b2)
    } else if keep_branch {
        Some(b1)
    } else {
        None
    };
    let reason = if keep_through {
        "through"
    } else if keep_branch {
        "branch"
    } else {
        "all"
    };
    let mut updates = Updates::default();
    let mut remove: Vec<String> = Vec::new();
    let mut removed_tracks: Vec<Value> = Vec::new();
    let mut remap: Vec<Value> = Vec::new();
    let mut removed_elements = 0;
    for port in [b1, b2] {
        if kept.is_some_and(|k| k.port == port.port) {
            continue;
        }
        let (Some(track), Some(id)) = (&port.track, port.id()) else { continue };
        if port.mine.is_empty() || remove.contains(&id) {
            continue;
        }
        removed_elements += port.mine.len();
        if port.beyond.is_empty() {
            removed_tracks.push(name_or_id(track, &id));
            remove.push(id);
        } else {
            let current = updates.get(&id).cloned().unwrap_or_else(|| track.clone());
            updates.set(id, trim_track(&current, port.endpoint.as_deref(), &port.mine));
        }
    }
    let Some(kept) = kept else {
        return plan(&sid, reason, &remove, &updates, remap, removed_elements, removed_tracks, (0.0, 0.0), false);
    };
    let kid = kept.id().unwrap_or_default();
    let current = |updates: &Updates, p: &Port| -> Option<Value> { p.id().and_then(|id| updates.get(&id).cloned()).or_else(|| p.track.clone()) };
    let kc = current(&updates, kept).unwrap_or(Value::Null);
    updates.set(kid.clone(), unmark_on(&kc, sw));
    let mut joined = false;
    let mut keep_id = kid.clone();
    if let (Some(aid), Some(_)) = (a.id(), &a.track)
        && aid != kid
    {
        let kept_track = current(&updates, kept).unwrap_or(Value::Null);
        let a_track = current(&updates, a).unwrap_or(Value::Null);
        let reversed = reverse_track(&kept_track);
        let kept_at_end = kept.endpoint.as_deref() == Some("END");
        let a_at_end = a.endpoint.as_deref() == Some("END");
        let flipped = a_at_end == kept_at_end;
        let head = if a_at_end {
            &a_track
        } else if kept_at_end {
            &kept_track
        } else {
            &reversed
        };
        let tail = if a_at_end {
            if kept_at_end { &reversed } else { &kept_track }
        } else {
            &a_track
        };
        let result = join_tracks(head, tail);
        let rid = js_string(result.get("id"));
        let gone = if rid == aid { kid.clone() } else { aid.clone() };
        updates.delete(&gone);
        updates.set(rid.clone(), result);
        remove.push(gone.clone());
        remap.push(json!({ "oldId": gone, "newId": rid, "flip": gone == kid && flipped }));
        if rid == kid && flipped {
            remap.push(json!({ "oldId": kid, "newId": kid, "flip": true }));
        }
        keep_id = rid;
        joined = true;
    }
    let merging = updates.get(&keep_id).cloned().unwrap_or(Value::Null);
    let chain = merge_chain(els(&merging), js_number(merging.get("epsg")) as i64, false);
    let mut m = merging.clone();
    let ce = chain["elements"].as_array().cloned().unwrap_or_default();
    m["coordinates"] = Value::Array(rebuild_coords(&ce));
    m["elements"] = Value::Array(ce);
    updates.set(keep_id, m);
    let merged = (chain["mergedFrom"].as_f64().unwrap_or(0.0), chain["mergedInto"].as_f64().unwrap_or(0.0));
    plan(&sid, reason, &remove, &updates, remap, removed_elements, removed_tracks, merged, joined)
}
