// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Editing a switch connection: taken apart in memory and laid again by the
//! connection's own construction (`crossoverEdit.js`).

use std::collections::HashSet;

use rail_across_geom::element::Utm;
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_js::{get, js_number, js_string, number, strict_eq, truthy};
use rail_across_switch::catalogue::SWITCH_PICK_TYPES;
use rail_across_switch::delete::{by_id, plan_switch_deletion, switch_parts};
use rail_across_switch::mid_chain::{DEFAULT_MIDS, Mid, MidKind, mid_unknowns};
use rail_across_track::model::remap_switches;
use rail_across_track::platforms::{element_stations, station_on_track};
use rail_across_track::switch_model::{port_end, port_track};
use serde_json::{Value, json};

use crate::crossover::adopt_crossover;
use crate::scurve::{Ask, connection_pick, settle_connection};

const EDIT_MAX_MIDS: usize = 3;
const TOE_INTO: f64 = 1e-3;
const ROUTE_PORTS: [&str; 2] = ["B1", "B2"];

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn is_turnout(sw: &Value) -> bool {
    !truthy(sw.get("kind")) || sw.get("kind").and_then(Value::as_str) == Some("switch")
}

fn port_at(sw: &Value, track_id: &Value, endpoint: &str) -> Option<&'static str> {
    ["A", "B1", "B2"]
        .into_iter()
        .find(|p| strict_eq(port_track(sw, p), Some(track_id)) && port_end(sw, p) == Some(endpoint))
}

/// The switch connection a track is, or the error key (`switchConnectionOf`).
pub fn switch_connection_of(track_id: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    let none = json!({ "error": "crossover_edit_error_none" });
    let Some(conn) = tracks.iter().find(|t| strict_eq(t.get("id"), Some(track_id))) else {
        return none;
    };
    if els(conn).is_empty() {
        return none;
    }
    let cid = conn.get("id").cloned().unwrap_or(Value::Null);
    let at = |endpoint: &str| switches.iter().find_map(|sw| port_at(sw, &cid, endpoint).map(|p| (sw, p)));
    let ends = [at("BEGIN"), at("END")];
    if ends.iter().any(|e| e.is_none_or(|(sw, p)| !ROUTE_PORTS.contains(&p) || !is_turnout(sw))) {
        return none;
    }
    let ends = [ends[0].expect("an end"), ends[1].expect("an end")];
    if strict_eq(ends[0].0.get("switchId"), ends[1].0.get("switchId")) {
        return none;
    }
    let by = by_id(tracks);
    let turnouts: Vec<Value> = ends
        .iter()
        .map(|(sw, port)| {
            let line_port = ROUTE_PORTS.into_iter().find(|p| p != port).unwrap_or("B2");
            let own = switch_parts(sw, &by).into_iter().find(|p| p.port == *port).map(|p| p.mine).unwrap_or_default();
            json!({ "sw": sw, "port": port, "linePort": line_port, "own": own })
        })
        .collect();
    let own_len = |t: &Value| t["own"].as_array().map(Vec::len).unwrap_or(0);
    let (n1, n2) = (own_len(&turnouts[0]), own_len(&turnouts[1]));
    let ce = els(conn);
    let mids: Vec<Value> = if n1 + n2 <= ce.len() { ce[n1..ce.len() - n2].to_vec() } else { vec![] };
    if n1 == 0 || n2 == 0 || mids.iter().any(|el| truthy(el.get("switchId"))) {
        return none;
    }
    if mids.is_empty() || mids.len() > EDIT_MAX_MIDS {
        return json!({ "error": "crossover_edit_error_mids" });
    }
    let forms: Vec<Option<&Value>> = turnouts
        .iter()
        .map(|t| SWITCH_PICK_TYPES.iter().copied().find(|f| f.get("label") == t["sw"].get("label")))
        .collect();
    if forms.iter().any(Option::is_none) {
        return json!({ "error": "crossover_edit_error_form" });
    }
    let (f1, f2) = (forms[0].expect("a form"), forms[1].expect("a form"));
    let form = if std::ptr::eq(f1, f2) { f1.clone() } else { json!([f1, f2]) };
    let speed = js_number(f1.get("speed")).min(js_number(f2.get("speed")));
    json!({ "conn": conn, "turnouts": turnouts, "mids": mids, "form": form, "forms": [f1, f2], "speed": number(speed) })
}

fn apply_deletion(tracks: &[Value], switches: &[Value], plan: &Value) -> (Vec<Value>, Vec<Value>) {
    let removed: HashSet<String> = plan["removeTrackIds"].as_array().into_iter().flatten().map(|v| js_string(Some(v))).collect();
    let updated: Vec<Value> = plan["updateTracks"].as_array().cloned().unwrap_or_default();
    let rest: Vec<Value> = switches
        .iter()
        .filter(|sw| !strict_eq(sw.get("switchId"), plan.get("switchId")))
        .cloned()
        .collect();
    let tracks: Vec<Value> = tracks
        .iter()
        .filter(|t| !removed.contains(&js_string(t.get("id"))))
        .map(|t| {
            updated
                .iter()
                .find(|u| strict_eq(u.get("id"), t.get("id")))
                .cloned()
                .unwrap_or_else(|| t.clone())
        })
        .collect();
    let remap = plan["remap"].as_array().cloned().unwrap_or_default();
    let switches = if remap.is_empty() { rest } else { remap_switches(&rest, &remap) };
    (tracks, switches)
}

fn line_end(sw: &Value, line_port: &str, tracks: &[Value]) -> Option<Utm> {
    let parts = switch_parts(sw, &by_id(tracks));
    let part = parts.into_iter().find(|p| p.port == line_port)?;
    let track = part.track.as_ref()?;
    if part.mine.is_empty() {
        return None;
    }
    let at_end = part.endpoint.as_deref() == Some("END");
    let el = if at_end { &part.mine[0] } else { &part.mine[part.mine.len() - 1] };
    let n = el.get(if at_end { "startNode" } else { "endNode" })?;
    Some(Utm {
        e: n[0].as_f64().unwrap_or(f64::NAN),
        n: n[1].as_f64().unwrap_or(f64::NAN),
        zone: js_number(track.get("epsg")) as i64,
    })
}

fn pick_where_it_stood(toe: Utm, end: Option<Utm>, candidates: &[Value]) -> Option<Value> {
    let mut best: Option<(&Value, f64, f64, f64)> = None;
    for track in candidates {
        let a = station_on_track(track, toe);
        let b = end.and_then(|e| station_on_track(track, e));
        let (Some(a), Some(b)) = (a, b) else { continue };
        if a.1 > NODE_TOL || b.1 > 1e-2 {
            continue;
        }
        if best.is_none_or(|bb| a.1 + b.1 < bb.3) {
            best = Some((track, a.0, b.0, a.1 + b.1));
        }
    }
    let (track, toe_s, end_s, _) = best?;
    let ahead = if end_s > toe_s { 1.0 } else { -1.0 };
    let at = toe_s + ahead * TOE_INTO;
    let (idx, start, _) = element_stations(track).into_iter().find(|r| at >= r.1 - 1e-9 && at <= r.2 + 1e-9)?;
    let len = js_number(els(track)[idx].get("length"));
    Some(connection_pick(track, idx, len.min(0f64.max(toe_s - start))))
}

/// The connection taken apart in memory (`takeApart`).
pub fn take_apart(found: &Value, tracks: &[Value], switches: &[Value]) -> Value {
    let conn = &found["conn"];
    let turnouts = found["turnouts"].as_array().cloned().unwrap_or_default();
    let ends: Vec<Option<Utm>> = turnouts
        .iter()
        .map(|t| line_end(&t["sw"], t["linePort"].as_str().unwrap_or("B2"), tracks))
        .collect();
    let epsg = js_number(conn.get("epsg")) as i64;
    let ce = els(conn);
    let node = |el: &Value, k: &str| {
        let n = &el[k];
        Utm {
            e: n[0].as_f64().unwrap_or(f64::NAN),
            n: n[1].as_f64().unwrap_or(f64::NAN),
            zone: epsg,
        }
    };
    let toes = [node(&ce[0], "startNode"), node(&ce[ce.len() - 1], "endNode")];
    let mut state_tracks: Vec<Value> = tracks.iter().filter(|t| !strict_eq(t.get("id"), conn.get("id"))).cloned().collect();
    let mut state_switches: Vec<Value> = switches.to_vec();
    let mut plans = Vec::new();
    let mut touched: HashSet<String> = HashSet::new();
    for t in &turnouts {
        let now = state_switches
            .iter()
            .find(|s| strict_eq(s.get("switchId"), t["sw"].get("switchId")))
            .cloned()
            .unwrap_or(Value::Null);
        let plan = plan_switch_deletion(&now, &state_tracks);
        if !truthy(plan.get("joined")) {
            return json!({ "error": "crossover_edit_error_line" });
        }
        for u in plan["updateTracks"].as_array().into_iter().flatten() {
            touched.insert(js_string(u.get("id")));
        }
        let (nt, ns) = apply_deletion(&state_tracks, &state_switches, &plan);
        state_tracks = nt;
        state_switches = ns;
        plans.push(plan);
    }
    let candidates: Vec<Value> = state_tracks.iter().filter(|t| touched.contains(&js_string(t.get("id")))).cloned().collect();
    let picks: Vec<Option<Value>> = toes
        .iter()
        .enumerate()
        .map(|(k, toe)| pick_where_it_stood(*toe, ends[k], &candidates))
        .collect();
    if picks.iter().any(Option::is_none)
        || strict_eq(
            picks[0].as_ref().and_then(|p| p.get("trackId")),
            picks[1].as_ref().and_then(|p| p.get("trackId")),
        )
    {
        return json!({ "error": "crossover_edit_error_line" });
    }
    json!({ "tracks": state_tracks, "switches": state_switches, "plans": plans, "picks": picks })
}

/// Where the dialog starts on a connection taken apart (`editStart`).
pub fn edit_start(found: &Value, apart: &Value) -> Value {
    let picks: Vec<Value> = apart["picks"].as_array().cloned().unwrap_or_default();
    let speed = js_number(found.get("speed"));
    let form = found.get("form").filter(|f| !f.is_null());
    let mids_found: Vec<Value> = found["mids"].as_array().cloned().unwrap_or_default();
    let defaults = || Value::Array(DEFAULT_MIDS.iter().map(|m| m.to_json()).collect());
    if mids_found.len() == 1 {
        let ask = Ask {
            picks: &picks,
            speed,
            mids: None,
            seed: None,
            form,
            lenient: false,
        };
        let settled = settle_connection(&ask, 0.0);
        let r = &settled.result;
        let closes = truthy(r.get("valid")) || truthy(r.get("overridable"));
        return json!({ "shift": number(if closes { settled.shift } else { 0.0 }), "mids": defaults(), "seed": null, "closes": closes });
    }
    let kind = |el: &Value| match el.get("elementType").and_then(Value::as_f64) {
        Some(2.0) => "transition",
        _ if get(el, "radius").is_none() => "straight",
        _ => "arc",
    };
    let pieces: Vec<Value> = mids_found
        .iter()
        .map(|el| match kind(el) {
            "transition" => json!({ "type": "transition", "length": el.get("length") }),
            "straight" => json!({ "type": "straight", "length": el.get("length"), "kappa": 0 }),
            _ => json!({ "type": "arc", "length": el.get("length"), "kappa": number(-1.0 / js_number(el.get("radius"))) }),
        })
        .collect();
    if let Some(adopted) = adopt_crossover(&picks, speed, &json!({ "s1": 0, "s2": 0, "pieces": pieces }), form, true, true) {
        let mut out = adopted;
        out["closes"] = json!(true);
        return out;
    }
    let as_is: Vec<Mid> = mids_found
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let k = kind(el);
            let length = if i < 2 { None } else { el.get("length").and_then(Value::as_f64) };
            match k {
                "arc" => {
                    let r = js_number(el.get("radius"));
                    Mid {
                        kind: MidKind::Arc,
                        length,
                        radius: Some(r.abs()),
                        side: Some(if r > 0.0 {
                            1.0
                        } else if r < 0.0 {
                            -1.0
                        } else {
                            r
                        }),
                    }
                }
                "straight" => Mid {
                    kind: MidKind::Straight,
                    length,
                    radius: None,
                    side: None,
                },
                _ => Mid {
                    kind: MidKind::Transition,
                    length,
                    radius: None,
                    side: None,
                },
            }
        })
        .collect();
    let values: Vec<Value> = mid_unknowns(&as_is)
        .iter()
        .map(|u| mids_found[u.i].get("length").cloned().unwrap_or(Value::Null))
        .collect();
    json!({ "shift": 0, "mids": as_is.iter().map(|m| m.to_json()).collect::<Vec<_>>(), "seed": { "ds": 0, "values": values }, "closes": false })
}

fn kept_record(old: &Value, laid: &Value) -> Value {
    let mut out = laid.as_object().cloned().unwrap_or_default();
    for (k, v) in old.as_object().into_iter().flatten() {
        if !out.contains_key(k) && k != "swapped" && k != "mainRadius" {
            out.insert(k.clone(), v.clone());
        }
    }
    // `{ ...out, switchId, name, number }`: a member the old record has not is left out.
    for k in ["switchId", "name", "number"] {
        match old.get(k) {
            Some(v) => {
                out.insert(k.into(), v.clone());
            }
            None => {
                out.remove(k);
            }
        }
    }
    serde_json::Value::Object(out)
}

/// The commit of an edited connection (`editCommit`).
pub fn edit_commit(found: &Value, apart: &Value, commit: &Value) -> Value {
    if commit.is_null() || truthy(commit.get("carveError")) {
        return commit.clone();
    }
    let olds: Vec<Value> = found["turnouts"].as_array().into_iter().flatten().map(|t| t["sw"].clone()).collect();
    let mut connection = commit.clone();
    let mut remove = commit["removeTrackIds"].as_array().cloned().unwrap_or_default();
    remove.push(found["conn"].get("id").cloned().unwrap_or(Value::Null));
    connection["removeTrackIds"] = Value::Array(remove);
    connection["addSwitches"] = Value::Array(
        commit["addSwitches"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|sw| match olds.iter().find(|o| strict_eq(o.get("switchId"), sw.get("switchId"))) {
                Some(old) => kept_record(old, sw),
                None => sw.clone(),
            })
            .collect(),
    );
    json!({ "deletions": apart["plans"], "connection": connection })
}
