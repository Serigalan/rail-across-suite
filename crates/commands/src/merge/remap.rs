// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Carrying a reference across a change of track ids (`remap.js`).

use std::collections::{HashMap, HashSet, VecDeque};

use rail_across_js::{js_round, js_string, number};
use rail_across_track::platforms::{point_at_station, station_on_track};
use rail_across_track::routes::repair_route_with;
use rail_across_track::switch_model::{port_end, port_track, ports_of, with_port};
use serde_json::{Map, Value, json};

use super::carry::{CARRIED_DOMAINS, Ctx, carry_domain};

const END_TOL: f64 = 0.01;
const ON_TRACK_TOL: f64 = 0.05;

fn from_entries(log: &[Value]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for e in log {
        let (Some(from), Some(to)) = (e.get("from").filter(|v| rail_across_js::truthy(Some(v))), e.get("to").and_then(Value::as_array)) else {
            continue;
        };
        map.entry(js_string(Some(from))).or_default().extend(to.iter().map(|v| js_string(Some(v))));
    }
    map
}

fn descendants(id: &str, log: &[Value]) -> Vec<String> {
    let next = from_entries(log);
    let mut seen: Vec<String> = Vec::new();
    let mut queue: VecDeque<String> = next.get(id).cloned().unwrap_or_default().into();
    while let Some(cur) = queue.pop_front() {
        if seen.contains(&cur) || cur == id {
            continue;
        }
        seen.push(cur.clone());
        queue.extend(next.get(&cur).cloned().unwrap_or_default());
    }
    seen
}

/// Whether the log split or folded a track (`remapKind`).
pub fn remap_kind(id: &str, log: &[Value]) -> Option<&'static str> {
    let entries: Vec<&Value> = log
        .iter()
        .filter(|e| {
            e.get("from").map(|v| js_string(Some(v))) == Some(id.to_string())
                && e.get("to")
                    .and_then(Value::as_array)
                    .is_some_and(|t| t.iter().any(|x| js_string(Some(x)) != id))
        })
        .collect();
    if entries.is_empty() {
        return None;
    }
    Some(if entries.iter().any(|e| e["to"].as_array().map(Vec::len).unwrap_or(0) > 1) {
        "split"
    } else {
        "joined"
    })
}

fn end_point(track: &Value, endpoint: &str) -> Option<[f64; 2]> {
    let es = track.get("elements").and_then(Value::as_array)?;
    let el = if endpoint == "BEGIN" { es.first()? } else { es.last()? };
    let n = el.get(if endpoint == "BEGIN" { "startNode" } else { "endNode" })?.as_array()?;
    let (e, no) = (n.first()?.as_f64()?, n.get(1)?.as_f64()?);
    (e.is_finite() && no.is_finite()).then_some([e, no])
}

fn map_end(old: &Value, endpoint: &str, candidates: &[Value]) -> Option<(Value, &'static str)> {
    let at = end_point(old, endpoint)?;
    let mut hits = Vec::new();
    for tr in candidates {
        if !rail_across_js::strict_eq(tr.get("epsg"), old.get("epsg")) {
            continue;
        }
        for end in ["BEGIN", "END"] {
            if let Some(p) = end_point(tr, end)
                && (p[0] - at[0]).hypot(p[1] - at[1]) < END_TOL
            {
                hits.push((tr.get("id").cloned().unwrap_or(Value::Null), end));
            }
        }
    }
    (hits.len() == 1).then(|| hits.remove(0))
}

fn bearing_gap(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs()
}

fn map_stretch(old: &Value, s0: f64, s1: f64, candidates: &[Value]) -> Option<(Value, f64, f64, bool)> {
    let (p0, b0) = point_at_station(old, s0)?;
    let (p1, _) = point_at_station(old, s1)?;
    let mut hits = Vec::new();
    for tr in candidates {
        if !rail_across_js::strict_eq(tr.get("epsg"), old.get("epsg")) {
            continue;
        }
        let (Some(a), Some(b)) = (station_on_track(tr, p0), station_on_track(tr, p1)) else {
            continue;
        };
        if a.1 > ON_TRACK_TOL || b.1 > ON_TRACK_TOL {
            continue;
        }
        hits.push((tr.get("id").cloned().unwrap_or(Value::Null), a.0, b.0, bearing_gap(a.2, b0) > 90.0));
    }
    (hits.len() == 1).then(|| hits.remove(0))
}

fn round(s: f64) -> f64 {
    js_round(s * 1000.0) / 1000.0
}

/// A reference carried: { collection, id, from, to }.
pub struct Carried {
    pub collection: String,
    pub id: Value,
    pub from: String,
    pub to: String,
}

impl Carried {
    pub fn to_json(&self) -> Value {
        json!({ "collection": self.collection, "id": self.id, "from": self.from, "to": self.to })
    }
}

/// Repoint the references that name a track the record no longer has (`carryReferences`).
pub fn carry_references(project: &Value, logs: &[&[Value]], old_tracks: &dyn Fn(&str) -> Option<Value>) -> (Value, Vec<Carried>) {
    let tracks: Vec<Value> = project.get("tracks").and_then(Value::as_array).cloned().unwrap_or_default();
    let present: Map<String, Value> = tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
    let mut carried = Vec::new();
    let log: Vec<Value> = logs.iter().flat_map(|l| l.iter().cloned()).collect();
    let candidates_for = |id: &str| -> Vec<Value> { descendants(id, &log).into_iter().filter_map(|d| present.get(&d).cloned()).collect() };
    let list = |k: &str| project.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let switches: Vec<Value> = list("switches")
        .into_iter()
        .map(|sw| {
            let mut out = sw.clone();
            for (port, _) in ports_of(&sw) {
                let Some(id) = port_track(&sw, port).filter(|v| rail_across_js::truthy(Some(v))).map(|v| js_string(Some(v))) else {
                    continue;
                };
                if present.contains_key(&id) {
                    continue;
                }
                let (Some(old), cands) = (old_tracks(&id), candidates_for(&id)) else {
                    continue;
                };
                if cands.is_empty() {
                    continue;
                }
                let Some((tid, end)) = map_end(&old, port_end(&sw, port).unwrap_or(""), &cands) else {
                    continue;
                };
                carried.push(Carried {
                    collection: "switches".into(),
                    id: sw.get("switchId").cloned().unwrap_or(Value::Null),
                    from: id.clone(),
                    to: js_string(Some(&tid)),
                });
                out = with_port(&out, port, &json!({ "trackId": tid, "endpoint": end }));
            }
            out
        })
        .collect();
    let platforms: Vec<Value> = list("platforms")
        .into_iter()
        .map(|pf| {
            let Some(id) = pf.get("trackId").filter(|v| rail_across_js::truthy(Some(v))).map(|v| js_string(Some(v))) else {
                return pf;
            };
            if present.contains_key(&id) {
                return pf;
            }
            let (Some(old), cands) = (old_tracks(&id), candidates_for(&id)) else {
                return pf;
            };
            if cands.is_empty() {
                return pf;
            }
            let num = |k: &str| pf.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
            let Some((tid, s0, s1, flip)) = map_stretch(&old, num("startStation"), num("endStation"), &cands) else {
                return pf;
            };
            carried.push(Carried {
                collection: "platforms".into(),
                id: pf.get("id").cloned().unwrap_or(Value::Null),
                from: id,
                to: js_string(Some(&tid)),
            });
            let mut out = pf.clone();
            out["trackId"] = tid;
            out["startStation"] = number(round(if flip { s1 } else { s0 }));
            out["endStation"] = number(round(if flip { s0 } else { s1 }));
            if flip && let Some(side) = pf.get("side").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                out["side"] = json!(if side == "left" { "right" } else { "left" });
            }
            out
        })
        .collect();
    let end_marks: Vec<Value> = list("endMarks")
        .into_iter()
        .map(|m| {
            let Some(id) = m.get("trackId").filter(|v| rail_across_js::truthy(Some(v))).map(|v| js_string(Some(v))) else {
                return m;
            };
            if present.contains_key(&id) {
                return m;
            }
            let (Some(old), cands) = (old_tracks(&id), candidates_for(&id)) else {
                return m;
            };
            if cands.is_empty() {
                return m;
            }
            let Some((tid, end)) = map_end(&old, m.get("endpoint").and_then(Value::as_str).unwrap_or(""), &cands) else {
                return m;
            };
            carried.push(Carried {
                collection: "endMarks".into(),
                id: m.get("id").cloned().unwrap_or(Value::Null),
                from: id,
                to: js_string(Some(&tid)),
            });
            let mut out = m.clone();
            out["trackId"] = tid;
            out["endpoint"] = json!(end);
            out
        })
        .collect();
    let old_v = |id: &Value| old_tracks(&js_string(Some(id)));
    let cands_v = |id: &Value| candidates_for(&js_string(Some(id)));
    let routes: Vec<Value> = list("routes")
        .into_iter()
        .map(|r| {
            let next = repair_route_with(&r, &tracks, &switches, &old_v, &cands_v);
            if next == r {
                return r;
            }
            let old_ids: Vec<Value> = r["trackIds"].as_array().cloned().unwrap_or_default();
            let new_ids: Vec<Value> = next["trackIds"].as_array().cloned().unwrap_or_default();
            for id in old_ids.iter().filter(|x| !present.contains_key(&js_string(Some(x)))) {
                let to = new_ids
                    .iter()
                    .filter(|x| !old_ids.contains(x))
                    .map(|v| js_string(Some(v)))
                    .collect::<Vec<_>>()
                    .join(",");
                carried.push(Carried {
                    collection: "routes".into(),
                    id: r.get("id").cloned().unwrap_or(Value::Null),
                    from: js_string(Some(id)),
                    to,
                });
            }
            next
        })
        .collect();
    let changed = |id: &str| !present.contains_key(id);
    let ctx = Ctx {
        present: present.keys().cloned().collect::<HashSet<String>>(),
        changed: &changed,
        old_track: old_tracks,
        candidates: &candidates_for,
        tracks: tracks.clone(),
        switches: switches.clone(),
    };
    let mut out = project.clone();
    for name in CARRIED_DOMAINS {
        let Some(d) = project
            .get(name)
            .filter(|d| d.get("collections").is_some_and(|c| rail_across_js::truthy(Some(c))))
        else {
            continue;
        };
        let (domain, moved) = carry_domain(d, &ctx);
        if domain == *d {
            continue;
        }
        out[name] = domain;
        for m in moved {
            carried.push(Carried {
                collection: format!("{name}.{}", m.collection),
                id: m.id,
                from: m.from,
                to: m.to,
            });
        }
    }
    // Every other collection of the record that holds locations — the civil structures among them, and any
    // to come — is carried as a layer's are (decision 0006).
    for name in super::collections::collections_in(&[project]).into_iter().filter(|n| !n.contains('.')) {
        if ["tracks", "switches", "platforms", "endMarks", "routes"].contains(&name.as_str()) {
            continue;
        }
        let d = json!({ "fields": {}, "collections": { name.clone(): project[&name] } });
        let (domain, moved) = carry_domain(&d, &ctx);
        if domain == d {
            continue;
        }
        out[&name] = domain["collections"][&name].clone();
        for m in moved {
            carried.push(Carried {
                collection: name.clone(),
                id: m.id,
                from: m.from,
                to: m.to,
            });
        }
    }
    if rail_across_js::truthy(project.get("routes")) {
        out["routes"] = Value::Array(routes);
    }
    if rail_across_js::truthy(project.get("switches")) {
        out["switches"] = Value::Array(switches);
    }
    if rail_across_js::truthy(project.get("platforms")) {
        out["platforms"] = Value::Array(platforms);
    }
    if rail_across_js::truthy(project.get("endMarks")) {
        out["endMarks"] = Value::Array(end_marks);
    }
    (out, carried)
}
