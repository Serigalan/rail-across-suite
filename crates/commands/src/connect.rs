// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! "Weiche / Kreuzung an Gleis anschließen" and what can be changed where
//! the rules reject the refitted element (`connectSwitch.js`,
//! `connectCrossing.js`, `connectRemedies.js`).

use std::sync::LazyLock;

use rail_across_geom::element::{Utm, reverse_element};
use rail_across_geom::transition::curvature_of;
use rail_across_js::{Object, get, js_hypot, js_number, js_string, number, strict_eq, truthy};
use rail_across_rules::bundled::{katalog, severity_rank};
use rail_across_rules::check::check_track;
use rail_across_rules::element::Element;
use rail_across_rules::grenzen::transition_lengths;
use rail_across_rules::pruefung::Context;
use rail_across_rules::rules::Rules;
use rail_across_rules::speed::SPEED_STEP;
use rail_across_switch::catalogue::{switch_branch_length, switch_straight_length};
use rail_across_switch::connect::{
    Anchor, Fit, PortAt, connection_roots, fit_connecting_element, join_route_into_track, refit_count, refit_track_end, solve_end_chain,
};
use rail_across_switch::crossing::{crossing_angle, crossing_end_distance};
use rail_across_switch::mid_chain::{End, Mid};
use rail_across_switch::placement::place_switch_on_track;
use rail_across_switch::route::switch_route_point_utm;
use rail_across_track::heights::{element_at_station, track_length};
use rail_across_track::model::next_track_name;
use rail_across_track::switch_model::{port_track, with_port};
use serde_json::{Value, json};

use crate::switches::{
    CrossingAsk, OnTrack, build_crossing_on_track, build_switch_on_track, crossing_on_track_placement, crossing_on_track_port, placement_json,
    switch_on_track_placement, switch_on_track_remnant, switch_on_track_route_end,
};

const SCAN_STEP: f64 = 2.0;

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// The point of the host at `station`, and one `ahead` further (`hostPointAt`).
pub fn host_point_at(track: &Value, station: f64, ahead: f64) -> Option<Utm> {
    let es = els(track);
    let (i, s) = element_at_station(es, track_length(track).min(0f64.max(station)))?;
    let el = &es[i];
    let route = rail_across_switch::route::switch_element_route(el);
    let route = rail_across_switch::route::Route {
        length: js_number(el.get("length")),
        ..route
    };
    let sn = el.get("startNode")?;
    let start = Utm {
        e: sn[0].as_f64().unwrap_or(f64::NAN),
        n: sn[1].as_f64().unwrap_or(f64::NAN),
        zone: js_number(track.get("epsg")) as i64,
    };
    Some(switch_route_point_utm(start, js_number(el.get("bearing")), &route, route.length.min(s + ahead)))
}

fn side_of(track: &Value, station: f64, reversed: bool, point: Utm) -> &'static str {
    let (Some(p), Some(q)) = (host_point_at(track, station, 0.0), host_point_at(track, station, 0.1)) else {
        return "left";
    };
    let sign = (if reversed { -1.0 } else { 1.0 }) * ((q.e - p.e) * (point.n - p.n) - (q.n - p.n) * (point.e - p.e));
    if sign > 0.0 { "left" } else { "right" }
}

/// The turnout's new route's end at toe `x` as the search needs it.
fn route_end_at(track: &Value, reversed: bool, sw: &Value, side: &str, x: f64) -> Option<PortAt> {
    let a = OnTrack {
        track,
        reversed,
        sw,
        side,
        straight_len: switch_straight_length(sw),
        speed: None,
    };
    let (utm, bearing, k) = switch_on_track_route_end(&a, x)?;
    Some(PortAt {
        port: utm,
        bearing_in: (bearing + 180.0) % 360.0,
        extra: json!({ "kIn": number(-k) }),
    })
}

/// The same with the placement the commit builds from.
fn route_end_full(track: &Value, reversed: bool, sw: &Value, side: &str, x: f64) -> Option<PortAt> {
    let a = OnTrack {
        track,
        reversed,
        sw,
        side,
        straight_len: switch_straight_length(sw),
        speed: None,
    };
    let p = switch_on_track_placement(&a, Some(x)).ok().flatten()?;
    let g = &p.geom;
    let (utm, bearing, segments) = match g.get("swapped").filter(|v| !v.is_null()) {
        Some(s) => (Utm::from_json(&s["straightUtm"]), js_number(s.get("mainEndBearing")), s["stemSegments"].clone()),
        None => (
            Utm::from_json(&g["curvedUtm"]),
            js_number(g.get("branchEndBearing")),
            g["branchSegments"].clone(),
        ),
    };
    let k_in = -curvature_of(segments.as_array().and_then(|s| s.last()).and_then(|s| f(s, "r2")));
    Some(PortAt {
        port: utm,
        bearing_in: (bearing + 180.0) % 360.0,
        extra: json!({ "kIn": number(k_in), "placement": placement_json(Ok(Some(p))) }),
    })
}

fn at_json(at: &PortAt) -> Value {
    let mut o = json!({ "port": at.port.to_json(), "bearingIn": number(at.bearing_in) });
    for (k, v) in at.extra.as_object().into_iter().flatten() {
        o[k] = v.clone();
    }
    o
}

fn fit_json(fit: &Fit) -> Value {
    match &fit.pieces {
        Some(p) => json!({ "pieces": rail_across_switch::route::chain_json(p) }),
        None => fit.to_json(),
    }
}

/// One way the turnout connects.
pub struct Solution {
    pub reversed: bool,
    pub side: &'static str,
    pub x: f64,
    pub fit: Fit,
    pub at: PortAt,
}

impl Solution {
    pub fn to_json(&self) -> Value {
        json!({ "reversed": self.reversed, "side": self.side, "x": number(self.x), "fit": fit_json(&self.fit), "at": at_json(&self.at) })
    }
}

fn solutions_json(solutions: &[Solution], preferred: Option<usize>) -> Value {
    json!({ "solutions": solutions.iter().map(Solution::to_json).collect::<Vec<_>>(), "preferred": preferred.map(|i| solutions[i].to_json()).unwrap_or(Value::Null) })
}

/// Every way the turnout connects (`switchConnectSolutions`).
pub fn switch_connect_solutions(o: &Value) -> Value {
    let track = &o["track"];
    let anchor = Anchor::from_json(&o["anchor"]);
    let sw = &o["sw"];
    let lead = f(o, "lead").unwrap_or(0.0);
    let only = o.get("only").filter(|v| truthy(Some(v)));
    let near = js_number(o.get("clickStation"));
    let total = track_length(track);
    let click = host_point_at(track, near, 0.0).unwrap_or(anchor.fixed);
    let reach = f(o, "reach").unwrap_or_else(|| {
        2.0 * js_hypot(&[click.e - anchor.fixed.e, click.n - anchor.fixed.n]) + 2.0 * switch_branch_length(Some(sw)) + anchor.old_length + 100.0
    });
    let (from, to) = (0f64.max(near - reach), total.min(near + reach));
    let mut solutions: Vec<Solution> = Vec::new();
    for reversed in [false, true] {
        for side in ["left", "right"] {
            if let Some(on) = only
                && (truthy(on.get("reversed")) != reversed || on.get("side").and_then(Value::as_str) != Some(side))
            {
                continue;
            }
            for root in connection_roots(&anchor, &|x| route_end_at(track, reversed, sw, side, x), from, to, SCAN_STEP, Some(near), lead) {
                let Some(at) = route_end_full(track, reversed, sw, side, root.x) else {
                    continue;
                };
                if let Some(fit) = fit_connecting_element(&anchor, at.port, at.bearing_in, lead)
                    && fit.miss.abs() < 1e-5
                {
                    solutions.push(Solution {
                        reversed,
                        side,
                        x: root.x,
                        fit,
                        at,
                    });
                }
            }
        }
    }
    solutions.sort_by(|a, b| ((a.x - near).abs() - (b.x - near).abs()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    let preferred = solutions
        .iter()
        .position(|s| s.side == side_of(track, s.x, s.reversed, anchor.fixed))
        .or((!solutions.is_empty()).then_some(0));
    solutions_json(&solutions, preferred)
}

/// The same with the elements at the open end laid anew (`switchConnectChainSolutions`).
pub fn switch_connect_chain_solutions(o: &Value, mids: &[Mid]) -> Value {
    let track = &o["track"];
    let anchor = Anchor::from_json(&o["anchor"]);
    let sw = &o["sw"];
    let click = js_number(o.get("clickStation"));
    let station = f(o, "station");
    let only = o.get("only").filter(|v| truthy(Some(v)));
    let solve_way = |reversed: bool, side: &'static str| -> Result<Option<Solution>, &'static str> {
        let end_at = |x: f64| -> Option<End> {
            let at = route_end_at(track, reversed, sw, side, x)?;
            Some(End {
                utm: at.port,
                bearing: at.bearing_in,
                k: at.extra["kIn"].as_f64().unwrap_or(0.0),
            })
        };
        let Some((x, pieces)) = solve_end_chain(&anchor, mids, &end_at, station, click)? else {
            return Ok(None);
        };
        let Some(at) = route_end_full(track, reversed, sw, side, x) else {
            return Ok(None);
        };
        Ok(Some(Solution {
            reversed,
            side,
            x,
            fit: Fit {
                route: pieces[0],
                lead: None,
                end_bearing: f64::NAN,
                miss: 0.0,
                pieces: Some(pieces),
            },
            at,
        }))
    };
    let own = |reversed: bool| side_of(track, click, reversed, anchor.fixed);
    let other = |reversed: bool| if own(reversed) == "left" { "right" } else { "left" };
    let ways = |sides: &dyn Fn(bool) -> &'static str| -> Vec<Result<Option<Solution>, &'static str>> {
        [false, true].into_iter().map(|r| solve_way(r, sides(r))).collect()
    };
    let mut found = match only {
        Some(on) => {
            let side = if on.get("side").and_then(Value::as_str) == Some("right") {
                "right"
            } else {
                "left"
            };
            vec![solve_way(truthy(on.get("reversed")), side)]
        }
        None => ways(&own),
    };
    if let Some(Err(e)) = found.iter().find(|f| f.is_err()) {
        return json!({ "error": e, "solutions": [], "preferred": null });
    }
    if only.is_none() && !found.iter().any(|f| matches!(f, Ok(Some(_)))) {
        found = ways(&other);
    }
    let mut solutions: Vec<Solution> = found.into_iter().filter_map(|f| f.ok().flatten()).collect();
    solutions.sort_by(|a, b| {
        ((a.x - click).abs() - (b.x - click).abs())
            .partial_cmp(&0.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let preferred = (!solutions.is_empty()).then_some(0);
    solutions_json(&solutions, preferred)
}

fn refit_check_chain(refitted: &Value, a: &Anchor, count: usize) -> Vec<Value> {
    let es = els(refitted);
    if a.endpoint == "END" {
        return es[a.el_idx.saturating_sub(1)..(a.el_idx + count).min(es.len())].to_vec();
    }
    es.iter().take(count + 1).rev().map(reverse_element).collect()
}

fn per_element(chain: &[Value]) -> Vec<Value> {
    let parsed: Vec<Element> = chain.iter().map(|e| serde_json::from_value(e.clone()).unwrap_or_default()).collect();
    check_track(&parsed, Context { before: None, after: None })
        .ok()
        .and_then(|c| c["perElement"].as_array().cloned())
        .unwrap_or_default()
}

fn refit_rule_error(chain: &[Value], count: usize) -> bool {
    let per = per_element(chain);
    let judged = chain.len().saturating_sub(count)..chain.len();
    judged
        .into_iter()
        .any(|i| severity_rank(per.get(i).and_then(|p| p["severity"].as_str())) >= severity_rank(Some("error")))
}

/// The connected track refitted and what the rules judge (`refitOutcome`).
pub fn refit_outcome(target: &Value, anchor: &Anchor, fit: &Fit, port: Utm, speed_limit: Option<f64>) -> Value {
    let refitted = refit_track_end(target, anchor, fit, port, speed_limit);
    let count = refit_count(fit);
    let chain = refit_check_chain(&refitted, anchor, count);
    let judged: Vec<usize> = (chain.len().saturating_sub(count)..chain.len()).collect();
    json!({
        "refitted": refitted, "chain": chain, "judged": judged,
        "element": chain.last().cloned().unwrap_or(Value::Null),
        "lead": if count > 1 { chain[chain.len() - 2].clone() } else { Value::Null },
        "ruleError": refit_rule_error(&chain, count),
        "delta": number(track_length(&refitted) - track_length(target)),
    })
}

fn fit_of(solution: &Value) -> Fit {
    let fit = &solution["fit"];
    if fit.get("pieces").and_then(Value::as_array).is_some() {
        let pieces = rail_across_switch::route::to_chain(&fit["pieces"]);
        return Fit {
            route: pieces[0],
            lead: None,
            end_bearing: f64::NAN,
            miss: 0.0,
            pieces: Some(pieces),
        };
    }
    Fit::from_json(fit)
}

/// What the solution means before anything is written (`switchConnectOutcome`).
pub fn switch_connect_outcome(o: &Value) -> Value {
    let s = &o["solution"];
    let mut out = refit_outcome(
        &o["target"],
        &Anchor::from_json(&o["anchor"]),
        &fit_of(s),
        Utm::from_json(&s["at"]["port"]),
        f(o, "speedLimit"),
    );
    out["remnant"] = switch_on_track_remnant(&o["track"], js_number(s.get("x")), truthy(s.get("reversed")), f(o, "speed"));
    out
}

/// The commit: the turnout laid into the host, its new route joined into the refitted track (`buildSwitchConnect`).
pub fn build_switch_connect(o: &Value) -> Value {
    let (track, target, solution, sw) = (&o["track"], &o["target"], &o["solution"], &o["sw"]);
    let anchor = Anchor::from_json(&o["anchor"]);
    let straight_len = switch_straight_length(sw);
    let side = if solution.get("side").and_then(Value::as_str) == Some("right") {
        "right"
    } else {
        "left"
    };
    let reversed = truthy(solution.get("reversed"));
    let x = js_number(solution.get("x"));
    let a = OnTrack {
        track,
        reversed,
        sw,
        side,
        straight_len,
        speed: None,
    };
    let Some(p) = switch_on_track_placement(&a, Some(x)).ok().flatten() else {
        return Value::Null;
    };
    let mut args = json!({
        "track": track, "tracks": o["tracks"], "g": p.geom, "plain": p.plain, "sw": sw, "straightLen": number(straight_len),
        "cant": o["cant"], "cantReason": o["cantReason"], "speed": o["speed"], "switchName": o["switchName"], "switchNumber": o["switchNumber"],
        "name": target.get("name"), "fields": {},
    });
    if args["cantReason"].is_null() {
        args.as_object_mut().expect("args").remove("cantReason");
    }
    let laid = build_switch_on_track(&args, &p.place);
    if laid.get("noRoom").is_some() || laid.is_null() {
        return laid;
    }
    let record = laid["addSwitches"][0].clone();
    let added = laid["addTracks"].as_array().cloned().unwrap_or_default();
    let route = added.last().cloned().unwrap_or(Value::Null);
    let route_id = route.get("id").cloned().unwrap_or(Value::Null);
    let port = Utm::from_json(&solution["at"]["port"]);
    let refitted = refit_track_end(target, &anchor, &fit_of(solution), port, f(o, "speedLimit"));
    let (joined, shift) = join_route_into_track(&refitted, &anchor, &route);
    let moved = if shift != 0.0 {
        shift + track_length(&refitted) - track_length(target)
    } else {
        0.0
    };
    let port_key = ["B1", "B2"]
        .into_iter()
        .find(|p| strict_eq(port_track(&record, p), Some(&route_id)))
        .unwrap_or("B1");
    let mut remove = laid["removeTrackIds"].as_array().cloned().unwrap_or_default();
    remove.push(target.get("id").cloned().unwrap_or(Value::Null));
    let mut add: Vec<Value> = added.into_iter().filter(|t| !strict_eq(t.get("id"), Some(&route_id))).collect();
    add.push(joined);
    json!({
        "removeTrackIds": remove, "addTracks": add,
        "addSwitches": [with_port(&record, port_key, &json!({ "trackId": target.get("id"), "endpoint": anchor.endpoint }))],
        "remap": laid["remap"],
        "restation": if moved != 0.0 { json!([{ "trackId": target.get("id"), "shift": number(moved) }]) } else { json!([]) },
    })
}

/// The crossing's cross angle for a side [deg] (`crossAngleOf`).
pub fn cross_angle_of(form: &Value, cross_side: &str) -> f64 {
    let alpha = crossing_angle(form) * 180.0 / std::f64::consts::PI;
    if cross_side == "right" { alpha } else { -alpha }
}

fn port_side(port: &str, cross_side: &str) -> &'static str {
    let right = cross_side == "right";
    if port == "D" {
        if right { "right" } else { "left" }
    } else if right {
        "left"
    } else {
        "right"
    }
}

/// Every way the crossing connects (`crossingConnectSolutions`).
pub fn crossing_connect_solutions(o: &Value) -> Value {
    let track = &o["track"];
    let anchor = Anchor::from_json(&o["anchor"]);
    let form = &o["form"];
    let lead = f(o, "lead").unwrap_or(0.0);
    let only = o.get("only").filter(|v| truthy(Some(v)));
    let near = js_number(o.get("clickStation"));
    let total = track_length(track);
    let end_dist = crossing_end_distance(form);
    let click = host_point_at(track, near, 0.0).unwrap_or(anchor.fixed);
    let reach =
        f(o, "reach").unwrap_or_else(|| 2.0 * js_hypot(&[click.e - anchor.fixed.e, click.n - anchor.fixed.n]) + 2.0 * end_dist + anchor.old_length + 100.0);
    let (from, to) = (0f64.max(near - reach), total.min(near + reach));
    let mut solutions: Vec<(Value, f64, &'static str, &'static str)> = Vec::new();
    for cross_side in ["right", "left"] {
        let ask = CrossingAsk {
            track,
            end_dist,
            form,
            cross_angle: cross_angle_of(form, cross_side),
        };
        for port in ["B", "D"] {
            if let Some(on) = only
                && (on.get("crossSide").and_then(Value::as_str) != Some(cross_side) || on.get("port").and_then(Value::as_str) != Some(port))
            {
                continue;
            }
            let port_at = |x: f64| {
                crossing_on_track_port(&ask, x, port).map(|(u, b)| PortAt {
                    port: u,
                    bearing_in: (b + 180.0) % 360.0,
                    extra: json!({}),
                })
            };
            for root in connection_roots(&anchor, &port_at, from, to, SCAN_STEP, Some(near), lead) {
                let Ok(Some((ahead, back, geom))) = crossing_on_track_placement(&ask, Some(root.x)) else {
                    continue;
                };
                let port_utm = Utm::from_json(&geom[format!("port{port}_utm")]);
                let placement = json!({ "error": null, "ahead": ahead.to_json(), "back": back.to_json(), "geom": geom });
                let at = json!({ "port": port_utm.to_json(), "bearingIn": number(root.at.bearing_in), "placement": placement });
                if let Some(fit) = fit_connecting_element(&anchor, port_utm, root.at.bearing_in, lead)
                    && fit.miss.abs() < 1e-5
                {
                    solutions.push((
                        json!({ "port": port, "crossSide": cross_side, "x": number(root.x), "fit": fit.to_json(), "at": at }),
                        root.x,
                        port,
                        cross_side,
                    ));
                }
            }
        }
    }
    solutions.sort_by(|a, b| ((a.1 - near).abs() - (b.1 - near).abs()).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    let preferred = solutions
        .iter()
        .find(|s| port_side(s.2, s.3) == side_of(track, s.1, false, anchor.fixed))
        .or(solutions.first())
        .map(|s| s.0.clone())
        .unwrap_or(Value::Null);
    json!({ "solutions": solutions.into_iter().map(|s| s.0).collect::<Vec<_>>(), "preferred": preferred })
}

fn describe(track: &Value) -> Object {
    track
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| !matches!(k.as_str(), "id" | "name" | "elements" | "coordinates" | "heights" | "epsg"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// The commit of a crossing connected to a track (`buildCrossingConnect`).
pub fn build_crossing_connect(o: &Value) -> Value {
    let (track, target, solution, form) = (&o["track"], &o["target"], &o["solution"], &o["form"]);
    let anchor = Anchor::from_json(&o["anchor"]);
    let end_dist = crossing_end_distance(form);
    let cross_side = solution.get("crossSide").and_then(Value::as_str).unwrap_or("right");
    let x = js_number(solution.get("x"));
    let ask = CrossingAsk {
        track,
        end_dist,
        form,
        cross_angle: cross_angle_of(form, cross_side),
    };
    let Ok(Some((ahead, _, geom))) = crossing_on_track_placement(&ask, Some(x)) else {
        return Value::Null;
    };
    let tracks: Vec<Value> = o["tracks"].as_array().cloned().unwrap_or_default();
    let args = json!({ "g": geom, "track": track, "tracks": tracks, "form": form, "endDist": number(end_dist), "switchName": o["switchName"], "switchNumber": o["switchNumber"], "name": null, "fields": {} });
    let laid = build_crossing_on_track(&args, &ahead);
    if laid.get("noRoom").is_some() || laid.is_null() {
        return laid;
    }
    let record = laid["addSwitches"][0].clone();
    let port = solution.get("port").and_then(Value::as_str).unwrap_or("D");
    let leg_id = port_track(&record, port).cloned().unwrap_or(Value::Null);
    let added = laid["addTracks"].as_array().cloned().unwrap_or_default();
    let leg = added.iter().find(|t| strict_eq(t.get("id"), Some(&leg_id))).cloned().unwrap_or(Value::Null);
    let refitted = refit_track_end(target, &anchor, &fit_of(solution), Utm::from_json(&solution["at"]["port"]), f(o, "speedLimit"));
    let (joined, shift) = join_route_into_track(&refitted, &anchor, &leg);
    let moved = if shift != 0.0 {
        shift + track_length(&refitted) - track_length(target)
    } else {
        0.0
    };
    let host_ids: Vec<String> = laid["remap"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r["newId"].as_array().cloned().unwrap_or_default())
        .map(|v| js_string(Some(&v)))
        .collect();
    let own = describe(target);
    let other_id = port_track(&record, if port == "B" { "D" } else { "B" }).cloned().unwrap_or(Value::Null);
    let names: Vec<String> = tracks
        .iter()
        .chain(added.iter())
        .filter_map(|t| t.get("name").filter(|v| truthy(Some(v))))
        .map(|v| js_string(Some(v)))
        .collect();
    let target_name = get(target, "name").map(|v| js_string(Some(v))).unwrap_or_default();
    let stem = {
        let trimmed = strip_number_suffix(&target_name);
        if trimmed.is_empty() { "track".to_string() } else { trimmed }
    };
    let other_name = next_track_name(&stem, &names);
    let mut add: Vec<Value> = added
        .into_iter()
        .filter(|t| !strict_eq(t.get("id"), Some(&leg_id)))
        .map(|t| {
            if host_ids.contains(&js_string(t.get("id"))) {
                return t;
            }
            let mut o2 = t.as_object().cloned().unwrap_or_default();
            for (k, v) in &own {
                o2.insert(k.clone(), v.clone());
            }
            if strict_eq(t.get("id"), Some(&other_id)) {
                o2.insert("name".into(), json!(other_name));
            }
            Value::Object(o2)
        })
        .collect();
    add.push(joined);
    let mut remove = laid["removeTrackIds"].as_array().cloned().unwrap_or_default();
    remove.push(target.get("id").cloned().unwrap_or(Value::Null));
    json!({
        "removeTrackIds": remove, "addTracks": add,
        "addSwitches": [with_port(&record, port, &json!({ "trackId": target.get("id"), "endpoint": anchor.endpoint }))],
        "remap": laid["remap"],
        "restation": if moved != 0.0 { json!([{ "trackId": target.get("id"), "shift": number(moved) }]) } else { json!([]) },
    })
}

/// `name.replace(/\.\d+$/, '')`.
fn strip_number_suffix(name: &str) -> String {
    if let Some(dot) = name.rfind('.') {
        let tail = &name[dot + 1..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return name[..dot].to_string();
        }
    }
    name.to_string()
}

const LEAD_START: f64 = 60.0;
const LEAD_ROUNDS: usize = 6;

static RULES: LazyLock<Rules<'static>> = LazyLock::new(|| Rules::new(katalog()).expect("the bundled catalogue's limits"));

fn remaining_findings(outcome: &Value) -> Vec<Value> {
    let chain = outcome["chain"].as_array().cloned().unwrap_or_default();
    let per = per_element(&chain);
    outcome["judged"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|i| {
            per.get(i.as_u64().unwrap_or(0) as usize)
                .and_then(|p| p["results"].as_array().cloned())
                .unwrap_or_default()
        })
        .filter(|r| severity_rank(r["severity"].as_str()) > severity_rank(Some("hint")))
        .collect()
}

fn regular_of(anchor: &Anchor, o: &Value) -> Option<f64> {
    let chain = o["chain"].as_array().cloned().unwrap_or_default();
    let prev: Option<Element> = (chain.len() > 2).then(|| serde_json::from_value(chain[0].clone()).unwrap_or_default());
    let next: Element = serde_json::from_value(o["element"].clone()).unwrap_or_default();
    let speed = f(&o["element"], "speed").unwrap_or(f64::NAN);
    transition_lengths(prev.as_ref(), Some(&next), anchor.r0, "clothoid", speed, &RULES.reg, &RULES.discretion)
        .ok()
        .and_then(|t| t.regular)
}

/// The remedies an outcome with a rule error may take (`connectRemedies`).
pub fn connect_remedies(anchor: &Anchor, current: &Value, outcome: &Value, outcome_for: &dyn Fn(&Value) -> Value) -> Vec<Value> {
    if !truthy(outcome.get("ruleError")) {
        return vec![];
    }
    let mut out = Vec::new();
    let clear = |o: &Value| !o.is_null() && !truthy(o.get("ruleError"));
    let with = |patch: &[(&str, Value)]| -> Value {
        let mut c = current.clone();
        for (k, v) in patch {
            c[*k] = v.clone();
        }
        c
    };
    let slowest = |o: &Value, with_limit: &dyn Fn(f64) -> Value| -> Option<(f64, Value)> {
        let fastest = o["chain"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|el| f(el, "speed").filter(|s| *s != 0.0).unwrap_or(0.0))
            .fold(0.0, f64::max);
        let mut v = (fastest / SPEED_STEP).ceil() * SPEED_STEP - SPEED_STEP;
        while v >= SPEED_STEP {
            let found = with_limit(v);
            if clear(&found) {
                return Some((v, found));
            }
            v -= SPEED_STEP;
        }
        None
    };
    let merged_remedy = |kind: &str, extra: &[(&str, Value)], o: &Value| -> Value {
        let mut r = json!({ "kind": kind });
        for (k, v) in current.as_object().into_iter().flatten() {
            r[k] = v.clone();
        }
        for (k, v) in extra {
            r[*k] = v.clone();
        }
        r["outcome"] = o.clone();
        r["remaining"] = Value::Array(remaining_findings(o));
        r
    };
    if let Some((v, o)) = slowest(outcome, &|v| outcome_for(&with(&[("speedLimit", number(v))]))) {
        let mut r = json!({ "kind": "speed", "speed": number(v) });
        for (k, val) in current.as_object().into_iter().flatten() {
            r[k] = val.clone();
        }
        r["speedLimit"] = number(v);
        r["outcome"] = o.clone();
        r["remaining"] = Value::Array(remaining_findings(&o));
        out.push(r);
    }
    if !(f(current, "lead").unwrap_or(0.0) > 0.0) && anchor.kind != "transition" {
        let at = |l: f64| -> Option<(f64, Value, f64)> {
            let o = outcome_for(&with(&[("lead", number(l))]));
            if o.is_null() || o["lead"].is_null() {
                return None;
            }
            let h = regular_of(anchor, &o).filter(|r| *r > 0.0).map(|r| r - l).unwrap_or(0.0);
            Some((l, o, h))
        };
        let found = (|| {
            let mut a = at(LEAD_START)?;
            if a.2.abs() < 0.05 {
                return Some(a.1);
            }
            let mut b = at(LEAD_START + a.2);
            let mut k = 0;
            while let Some(bb) = b.clone() {
                if k >= LEAD_ROUNDS || bb.2.abs() < 0.05 {
                    break;
                }
                let slope = (bb.2 - a.2) / (bb.0 - a.0);
                let next = if slope.abs() > 1e-9 { bb.0 - bb.2 / slope } else { bb.0 + bb.2 };
                a = bb;
                b = at(1f64.max(rail_across_js::js_round(next * 10.0) / 10.0));
                k += 1;
            }
            Some(b.unwrap_or(a).1)
        })();
        if let Some(o) = found {
            let lead = js_number(o["lead"].get("length"));
            let slower = if clear(&o) {
                None
            } else {
                slowest(&o, &|v| outcome_for(&with(&[("lead", number(lead)), ("speedLimit", number(v))])))
            };
            let done = if clear(&o) { Some(o.clone()) } else { slower.as_ref().map(|s| s.1.clone()) };
            if let Some(d) = done {
                out.push(merged_remedy(
                    "transition",
                    &[
                        ("length", number(lead)),
                        ("speed", slower.as_ref().map(|s| number(s.0)).unwrap_or(Value::Null)),
                        ("lead", number(lead)),
                        (
                            "speedLimit",
                            slower
                                .as_ref()
                                .map(|s| number(s.0))
                                .unwrap_or_else(|| current.get("speedLimit").cloned().unwrap_or(Value::Null)),
                        ),
                    ],
                    &d,
                ));
            }
        }
    }
    out
}

/// A turnout's toe placed on a track, as the placement that connectSwitch's search lays (for the ops).
pub fn placement_at(track: &Value, x: f64, reversed: bool, length: f64) -> Value {
    match place_switch_on_track(track, x, reversed, length) {
        Ok(p) => p.to_json(),
        Err(e) => json!({ "error": e }),
    }
}
