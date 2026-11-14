// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Whether the signalling domain holds together (`checkSignalling.js`).

use std::collections::{HashMap, HashSet};

use rail_across_js::{get, js_number, js_string, number, strict_eq, truthy};
use rail_across_track::topology::tracks_on_from;
use serde_json::{Map, Value, json};

use crate::catalogue::{
    SIGNAL_CATALOGUE, aspect_fits_frame, aspect_of, balise_kind_of, board_aspect, known_system, protection_system_of, settings_problems, signal_kind_of,
    symbol_fits,
};
use crate::overlaps::{branch_speed_on_run, overlap_length, route_speed_of};
use crate::sections::track_section_extents;

const OVERLAP_TOL: f64 = 0.01;
const STOP_TOL: f64 = 0.5;
const STATION_TOL: f64 = 0.01;

struct Out {
    errors: Vec<Value>,
    warnings: Vec<Value>,
}

impl Out {
    fn say(list: &mut Vec<Value>, code: &str, collection: &str, id: &Value, params: Value, extra: &str) {
        let idt = js_string(Some(id));
        let key = if extra.is_empty() {
            format!("{code}:{collection}:{idt}")
        } else {
            format!("{code}:{collection}:{idt}:{extra}")
        };
        let mut o = Map::new();
        o.insert("code".into(), json!(code));
        o.insert("key".into(), json!(key));
        o.insert("collection".into(), json!(collection));
        o.insert("id".into(), id.clone());
        o.insert("params".into(), params);
        list.push(Value::Object(o));
    }
    fn error(&mut self, code: &str, collection: &str, id: &Value, params: Value, extra: &str) {
        Out::say(&mut self.errors, code, collection, id, params, extra);
    }
    fn warning(&mut self, code: &str, collection: &str, id: &Value, params: Value, extra: &str) {
        Out::say(&mut self.warnings, code, collection, id, params, extra);
    }
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn coll<'a>(domain: &'a Value, name: &str) -> &'a [Value] {
    domain
        .get("collections")
        .and_then(|c| c.get(name))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn id_of(v: &Value) -> Value {
    v.get("id").cloned().unwrap_or(Value::Null)
}

fn ids(list: &[Value]) -> HashSet<String> {
    list.iter().map(|o| js_string(o.get("id"))).collect()
}

fn s(v: Option<&Value>) -> String {
    js_string(v)
}

fn track_length(track: &Value) -> f64 {
    arr(track, "elements")
        .iter()
        .map(|e| get(e, "length").and_then(Value::as_f64).unwrap_or(0.0))
        .sum()
}

/// Whether the signalling domain holds together, on its own and on the core (`checkSignalling`).
pub fn check_signalling(core: &Value, signalling: &Value) -> Value {
    check_signalling_on(core, signalling, &Value::Null)
}

/// The same, with the platforms of `civil` beside the core (infra-data-2.0,
/// decision 0025). The stopping places are the core's; the record keeps them in
/// its signalling, each naming its platform, and either is read: the
/// signalling's where it holds them, else the core's, whose platforms name them.
pub fn check_signalling_on(core: &Value, signalling: &Value, civil: &Value) -> Value {
    let mut out = Out {
        errors: vec![],
        warnings: vec![],
    };
    let c = |name: &str| coll(signalling, name);
    let tracks: HashMap<String, &Value> = coll(core, "tracks").iter().map(|t| (s(t.get("id")), t)).collect();
    let switches = ids(coll(core, "switches"));
    let points = ids(coll(core, "operationalPoints"));
    let civil_platforms: Vec<&Value> = coll(core, "platforms").iter().chain(coll(civil, "platforms").iter()).collect();
    let platforms: HashSet<String> = civil_platforms.iter().map(|p| s(p.get("id"))).collect();
    let stopping_places: &[Value] = if c("stoppingPlaces").is_empty() { coll(core, "stoppingPlaces") } else { c("stoppingPlaces") };
    // The platform a stopping place is at: the one it names, or the one naming it.
    let platform_of = |h: &Value| -> Option<Value> {
        if let Some(p) = h.get("platformId").filter(|v| truthy(Some(v))) {
            return Some(p.clone());
        }
        let id = s(h.get("id"));
        civil_platforms
            .iter()
            .find(|p| arr(p, "stoppingPlaceIds").iter().any(|x| s(Some(x)) == id))
            .and_then(|p| p.get("id").cloned())
    };
    let signals: HashMap<String, &Value> = c("signals").iter().map(|x| (s(x.get("id")), x)).collect();
    let known: HashMap<&str, HashSet<String>> = [
        "detectionPoints",
        "trackSections",
        "switchElements",
        "routes",
        "overlaps",
        "flankProtection",
    ]
    .into_iter()
    .map(|n| (n, ids(c(n))))
    .collect();
    let stopping_ids = ids(stopping_places);
    let overlaps: HashMap<String, &Value> = c("overlaps").iter().map(|o| (s(o.get("id")), o)).collect();

    let reference = |out: &mut Out, collection: &str, id: &Value, target: &str, value: Option<&Value>, field: &str| {
        let Some(value) = value.filter(|v| !v.is_null()) else { return };
        let v = js_string(Some(value));
        let there = match target {
            "core.tracks" => tracks.contains_key(&v),
            "core.switches" => switches.contains(&v),
            "core.operationalPoints" => points.contains(&v),
            "civil.platforms" => platforms.contains(&v),
            "core.stoppingPlaces" => stopping_ids.contains(&v),
            "signalling.signals" => signals.contains_key(&v),
            t => known.get(t.trim_start_matches("signalling.")).is_some_and(|k| k.contains(&v)),
        };
        if !there {
            out.error(
                "reference_missing",
                collection,
                id,
                json!({ "field": field, "target": target, "ref": value }),
                &format!("{field}:{v}"),
            );
        }
    };
    let at = |out: &mut Out, collection: &str, obj: &Value, loc: Option<&Value>, field: &str| {
        let Some(loc) = loc.filter(|l| truthy(Some(l))) else { return };
        if loc.get("anchor").and_then(Value::as_str) == Some("plane") {
            out.warning("orphaned", collection, &id_of(obj), json!({ "field": field }), field);
            return;
        }
        reference(out, collection, &id_of(obj), "core.tracks", loc.get("trackId"), &format!("{field}.trackId"));
        if let Some(track) = tracks.get(&s(loc.get("trackId")))
            && let Some(st) = get(loc, "station").and_then(Value::as_f64)
        {
            let len = track_length(track);
            if st < -STATION_TOL || st > len + STATION_TOL {
                out.error(
                    "off_track",
                    collection,
                    &id_of(obj),
                    json!({ "field": field, "station": loc.get("station"), "length": number(len) }),
                    field,
                );
            }
        }
    };
    let path = |out: &mut Out, collection: &str, obj: &Value, p: Option<&Value>, field: &str| {
        let list = p.map(|p| arr(p, "trackIds")).unwrap_or(&[]);
        for t in list {
            reference(out, collection, &id_of(obj), "core.tracks", Some(t), &format!("{field}.trackIds"));
        }
        if list.is_empty() {
            out.error("path_empty", collection, &id_of(obj), json!({ "field": field }), field);
        }
    };
    let track_of = |signal_id: Option<&Value>| -> Option<String> {
        signal_id
            .and_then(|id| signals.get(&js_string(Some(id))))
            .and_then(|sg| sg.get("at"))
            .and_then(|a| get(a, "trackId"))
            .map(|v| js_string(Some(v)))
    };
    let core_switches = coll(core, "switches");
    let core_tracks = coll(core, "tracks");
    let beside = |a: &Value, b: &Value| -> bool {
        core_switches.iter().any(|sw| {
            let held: Vec<Value> = sw
                .get("ports")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(_, p)| p.get("trackId").cloned().unwrap_or(Value::Null))
                .collect();
            held.iter().any(|h| strict_eq(Some(h), Some(a))) && held.iter().any(|h| strict_eq(Some(h), Some(b)))
        }) || ["BEGIN", "END"].iter().any(|end| {
            tracks_on_from(a, end, core_tracks, core_switches)
                .iter()
                .any(|(t, _)| strict_eq(Some(t), Some(b)))
        })
    };

    // signals
    let routes_by_id: HashMap<String, &Value> = c("routes").iter().map(|r| (s(r.get("id")), r)).collect();
    let is_screen = |f: &Value| f.get("kind").and_then(Value::as_str) == Some("screen");
    let frame_checks = |out: &mut Out, sg: &Value, frames: &[Value], field: &str, kind: Option<&Value>, any_screen: bool| {
        for (i, frame) in frames.iter().enumerate() {
            let fk = frame.get("kind").and_then(Value::as_str);
            let aspects = arr(frame, "aspects");
            if aspects.is_empty() && fk != Some("nameplate") {
                out.warning(
                    "frame_empty",
                    "signals",
                    &id_of(sg),
                    json!({ "frame": frame.get("kind") }),
                    &format!("{field}.{i}"),
                );
            }
            for a in aspects {
                let key = s(a.get("aspect"));
                let where_ = format!("{field}.{i}:{key}");
                let Some(aspect) = aspect_of(&key) else {
                    out.error("aspect_unknown", "signals", &id_of(sg), json!({ "aspect": a.get("aspect") }), &where_);
                    continue;
                };
                if !aspect_fits_frame(aspect, fk) {
                    out.error(
                        "aspect_not_on_frame",
                        "signals",
                        &id_of(sg),
                        json!({ "aspect": a.get("aspect"), "frame": frame.get("kind") }),
                        &where_,
                    );
                    continue;
                }
                if !symbol_fits(aspect, a.get("symbol")) {
                    out.error(
                        "symbol_invalid",
                        "signals",
                        &id_of(sg),
                        json!({ "aspect": a.get("aspect"), "symbol": a.get("symbol").cloned().unwrap_or(Value::Null) }),
                        &where_,
                    );
                }
                if fk == Some("indicator")
                    && let Some(k) = kind
                    && !arr(k, "zusatz").iter().any(|z| z.as_str() == Some(&key))
                {
                    out.error("indicator_not_allowed", "signals", &id_of(sg), json!({ "aspect": a.get("aspect") }), &where_);
                }
                if fk == Some("plate") && !any_screen && !board_aspect(&key) {
                    out.error("board_not_allowed", "signals", &id_of(sg), json!({ "aspect": a.get("aspect") }), &where_);
                }
            }
        }
    };
    for sg in c("signals") {
        at(&mut out, "signals", sg, sg.get("at"), "at");
        for (i, also) in arr(sg, "alsoAt").iter().enumerate() {
            at(&mut out, "signals", sg, Some(also), &format!("alsoAt.{i}"));
        }
        reference(
            &mut out,
            "signals",
            &id_of(sg),
            "core.operationalPoints",
            sg.get("operationalPointId"),
            "operationalPointId",
        );
        let own = arr(sg, "frames");
        let logical = arr(sg, "logicalSignals");
        if sg.get("kind").and_then(Value::as_str) == Some("virtual") {
            if !own.is_empty() || !logical.is_empty() {
                out.error("virtual_with_frames", "signals", &id_of(sg), json!({}), "");
            }
            if !truthy(sg.get("virtualFunction")) {
                out.warning("virtual_without_function", "signals", &id_of(sg), json!({}), "");
            }
            continue;
        }
        let any_screen = own.iter().any(is_screen) || logical.iter().any(|l| arr(l, "frames").iter().any(is_screen));
        if !logical.is_empty() {
            for (i, f) in own.iter().enumerate() {
                let k = f.get("kind").and_then(Value::as_str);
                if k == Some("screen") || k == Some("indicator") {
                    out.error(
                        "frame_of_logical_signal",
                        "signals",
                        &id_of(sg),
                        json!({ "frame": f.get("kind") }),
                        &format!("frames.{i}"),
                    );
                }
            }
        }
        let own_screens = own.iter().filter(|f| is_screen(f)).count();
        if own_screens > 1 {
            out.error("screens_many", "signals", &id_of(sg), json!({ "n": own_screens }), "");
        }
        frame_checks(&mut out, sg, own, "frames", None, any_screen);
        let mut seen: Vec<String> = Vec::new();
        for (i, l) in logical.iter().enumerate() {
            let where_ = format!("logicalSignals.{i}");
            let system = l.get("system").and_then(Value::as_str);
            let system_key = s(l.get("system"));
            if seen.contains(&system_key) {
                out.error("system_twice", "signals", &id_of(sg), json!({ "system": l.get("system") }), &where_);
            }
            seen.push(system_key.clone());
            if !known_system(system) {
                out.error("system_unknown", "signals", &id_of(sg), json!({ "system": l.get("system") }), &where_);
                continue;
            }
            let typ = l.get("type").and_then(Value::as_str);
            if truthy(l.get("type")) && typ != Some("other") && signal_kind_of(system, typ).is_none() {
                out.error(
                    "type_not_in_system",
                    "signals",
                    &id_of(sg),
                    json!({ "type": l.get("type"), "system": l.get("system") }),
                    &where_,
                );
            }
            let sys = &SIGNAL_CATALOGUE["systeme"][&system_key];
            for (problem, setting) in settings_problems(l.get("settings"), sys.get("einstellungen"), &[], None) {
                out.error(
                    &format!("setting_{problem}"),
                    "signals",
                    &id_of(sg),
                    json!({ "setting": setting, "system": l.get("system") }),
                    &format!("{where_}:{setting}"),
                );
            }
            for next in arr(l, "nextSystems") {
                if !known_system(next.as_str()) {
                    out.error(
                        "system_unknown",
                        "signals",
                        &id_of(sg),
                        json!({ "system": next }),
                        &format!("{where_}:next:{}", s(Some(next))),
                    );
                }
            }
            let mut parameter_sets: Vec<Option<&Value>> = vec![l.get("defaultParameters")];
            parameter_sets.extend(arr(l, "conditionalParameters").iter().map(|p| p.get("parameters")));
            for (k, params) in parameter_sets.iter().enumerate() {
                for (problem, setting) in settings_problems(*params, sys.get("parameter"), &[], None) {
                    out.error(
                        &format!("parameter_{problem}"),
                        "signals",
                        &id_of(sg),
                        json!({ "parameter": setting, "system": l.get("system") }),
                        &format!("{where_}:{k}:{setting}"),
                    );
                }
            }
            for p in arr(l, "conditionalParameters") {
                reference(
                    &mut out,
                    "signals",
                    &id_of(sg),
                    "signalling.routes",
                    p.get("routeId"),
                    &format!("{where_}.conditionalParameters"),
                );
            }
            let frames = arr(l, "frames");
            let screens: Vec<&Value> = frames.iter().filter(|f| is_screen(f)).collect();
            let kind = signal_kind_of(system, typ);
            if screens.len() > 1 {
                out.error("screens_many", "signals", &id_of(sg), json!({ "n": screens.len() }), &where_);
            }
            if !screens.is_empty() && !truthy(l.get("type")) {
                out.error("type_missing", "signals", &id_of(sg), json!({ "system": l.get("system") }), &where_);
            }
            frame_checks(&mut out, sg, frames, &format!("{where_}.frames"), kind, any_screen);
            if let (Some(k), Some(screen)) = (kind, screens.first()) {
                let shown: Vec<String> = arr(screen, "aspects").iter().map(|a| s(a.get("aspect"))).collect();
                let mut shown_set: Vec<String> = Vec::new();
                for x in &shown {
                    if !shown_set.contains(x) {
                        shown_set.push(x.clone());
                    }
                }
                for must in arr(&k["schirm"], "pflicht") {
                    let m = s(Some(must));
                    if !shown_set.contains(&m) {
                        out.error(
                            "screen_lacks",
                            "signals",
                            &id_of(sg),
                            json!({ "aspect": must, "system": l.get("system") }),
                            &format!("{where_}:{m}"),
                        );
                    }
                }
                let proceed = arr(&k["schirm"], "fahrt");
                if !proceed.is_empty() && !proceed.iter().any(|a| shown_set.contains(&s(Some(a)))) {
                    out.warning(
                        "screen_no_proceed",
                        "signals",
                        &id_of(sg),
                        json!({ "aspects": proceed, "system": l.get("system") }),
                        &where_,
                    );
                }
                for has in &shown_set {
                    if !arr(&k["schirm"], "erlaubt").iter().any(|x| x.as_str() == Some(has)) {
                        out.error(
                            "screen_not_allowed",
                            "signals",
                            &id_of(sg),
                            json!({ "aspect": has, "system": l.get("system") }),
                            &format!("{where_}:{has}"),
                        );
                    }
                }
            }
            let mut speeds: Vec<String> = Vec::new();
            for v in arr(l, "signallingSpeeds") {
                reference(
                    &mut out,
                    "signals",
                    &id_of(sg),
                    "signalling.routes",
                    v.get("routeId"),
                    &format!("{where_}.signallingSpeeds"),
                );
                let rid = s(v.get("routeId"));
                if let Some(route) = routes_by_id.get(&rid)
                    && !strict_eq(route.get("startSignalId"), sg.get("id"))
                {
                    out.error(
                        "speed_route_elsewhere",
                        "signals",
                        &id_of(sg),
                        json!({ "route": v.get("routeId") }),
                        &format!("{where_}:{rid}"),
                    );
                }
                if speeds.contains(&rid) {
                    out.error(
                        "speed_route_twice",
                        "signals",
                        &id_of(sg),
                        json!({ "route": v.get("routeId") }),
                        &format!("{where_}:{rid}:twice"),
                    );
                }
                speeds.push(rid);
            }
        }
    }

    // train detection, switch elements
    let mut bounding: HashSet<String> = HashSet::new();
    for d in c("detectionPoints") {
        at(&mut out, "detectionPoints", d, d.get("at"), "at");
    }
    for t in c("trackSections") {
        for p in arr(t, "detectionPointIds") {
            reference(&mut out, "trackSections", &id_of(t), "signalling.detectionPoints", Some(p), "detectionPointIds");
            bounding.insert(s(Some(p)));
        }
    }
    for d in c("detectionPoints") {
        if !bounding.contains(&s(d.get("id"))) {
            out.warning("detection_point_unused", "detectionPoints", &id_of(d), json!({}), "");
        }
    }
    if !c("trackSections").is_empty() {
        for (id, extent) in track_section_extents(core, signalling) {
            if !extent.matches {
                out.warning("section_extent_differs", "trackSections", &id, json!({ "missing": extent.missing }), "");
            }
        }
    }
    let switch_elements: HashMap<String, &Value> = c("switchElements").iter().map(|e| (s(e.get("id")), e)).collect();
    for e in c("switchElements") {
        reference(&mut out, "switchElements", &id_of(e), "core.switches", e.get("switchId"), "switchId");
        if let Some(der) = e.get("derailer").filter(|d| truthy(Some(d))) {
            at(&mut out, "switchElements", e, der.get("at"), "derailer.at");
        }
        reference(
            &mut out,
            "switchElements",
            &id_of(e),
            "signalling.signals",
            e.get("clearanceMarkerId"),
            "clearanceMarkerId",
        );
    }

    // routes and overlaps
    for r in c("routes") {
        let rid = id_of(r);
        reference(&mut out, "routes", &rid, "signalling.signals", r.get("startSignalId"), "startSignalId");
        reference(&mut out, "routes", &rid, "signalling.signals", r.get("targetSignalId"), "targetSignalId");
        path(&mut out, "routes", r, r.get("path"), "path");
        if strict_eq(r.get("startSignalId"), r.get("targetSignalId")) {
            out.error("route_to_itself", "routes", &rid, json!({}), "");
        }
        let run: Vec<Value> = r.get("path").map(|p| arr(p, "trackIds").to_vec()).unwrap_or_default();
        if let (Some(first), Some(t)) = (run.first(), track_of(r.get("startSignalId")))
            && s(Some(first)) != t
        {
            out.error("route_start_off_path", "routes", &rid, json!({}), "");
        }
        if let (Some(last), Some(t)) = (run.last(), track_of(r.get("targetSignalId")))
            && s(Some(last)) != t
        {
            out.error("route_target_off_path", "routes", &rid, json!({}), "");
        }
        let kind = r.get("kind").and_then(Value::as_str);
        if kind == Some("train") && !truthy(r.get("trainType")) {
            out.warning("route_type_missing", "routes", &rid, json!({}), "");
        }
        if kind == Some("shunting") && !truthy(r.get("shuntingType")) {
            out.warning("route_type_missing", "routes", &rid, json!({}), "");
        }
        if kind == Some("middle") && !truthy(r.get("middleType")) {
            out.warning("route_type_missing", "routes", &rid, json!({}), "");
        }
        for t in arr(r, "clearSectionIds") {
            reference(&mut out, "routes", &rid, "signalling.trackSections", Some(t), "clearSectionIds");
        }
        let ovs = arr(r, "overlaps");
        if kind == Some("shunting") && !ovs.is_empty() {
            out.error("shunting_with_overlap", "routes", &rid, json!({}), "");
        }
        let target_real = r
            .get("targetSignalId")
            .and_then(|t| signals.get(&s(Some(t))))
            .is_some_and(|sg| sg.get("kind").and_then(Value::as_str) == Some("real"));
        if kind == Some("train") && target_real && ovs.is_empty() {
            out.warning("route_without_overlap", "routes", &rid, json!({}), "");
        }
        if ovs.iter().filter(|o| truthy(o.get("preferred"))).count() > 1 {
            out.error("overlap_preferred_many", "routes", &rid, json!({}), "");
        }
        let speed = if kind == Some("train") { route_speed_of(signalling, r, core) } else { None };
        let speed_f = speed.as_ref().and_then(Value::as_f64);
        let branch = if kind == Some("train") {
            branch_speed_on_run(core_switches, &run)
        } else {
            None
        };
        if let Some((Some(allowed), sw_ids)) = &branch {
            if speed.as_ref().is_none_or(Value::is_null) {
                out.warning(
                    "route_speed_over_branch_missing",
                    "routes",
                    &rid,
                    json!({ "allowed": number(*allowed), "switches": sw_ids }),
                    "",
                );
            } else if js_number(speed.as_ref()) > *allowed {
                out.warning(
                    "route_speed_over_branch",
                    "routes",
                    &rid,
                    json!({ "speed": speed, "allowed": number(*allowed), "switches": sw_ids }),
                    "",
                );
            }
        }
        for o in ovs {
            reference(&mut out, "routes", &rid, "signalling.overlaps", o.get("overlapId"), "overlaps");
            let overlap = o.get("overlapId").and_then(|x| overlaps.get(&s(Some(x))));
            if let Some(ov) = overlap
                && !strict_eq(ov.get("signalId"), r.get("targetSignalId"))
            {
                out.error(
                    "overlap_not_at_target",
                    "routes",
                    &rid,
                    json!({ "overlap": o.get("overlapId") }),
                    &s(o.get("overlapId")),
                );
            }
            if let Some(ov) = overlap {
                let governing = get(ov, "speed").and_then(Value::as_f64).or(speed_f);
                let governing_v = get(ov, "speed").cloned().or_else(|| speed.clone()).unwrap_or(Value::Null);
                let needed = overlap_length(governing, get(ov, "gradient").and_then(Value::as_f64).unwrap_or(0.0));
                if let Some(n) = needed
                    && js_number(ov.get("length")) < n * (1.0 - OVERLAP_TOL)
                {
                    out.warning(
                        "overlap_short",
                        "overlaps",
                        &id_of(ov),
                        json!({ "length": ov.get("length"), "needed": number(n), "speed": governing_v, "route": rid }),
                        &s(Some(&rid)),
                    );
                }
            }
        }
        for a in arr(r, "aspects") {
            reference(&mut out, "routes", &rid, "signalling.signals", a.get("signalId"), "aspects");
            if let Some(sg) = a.get("signalId").and_then(|x| signals.get(&s(Some(x)))) {
                let shown: Vec<&Value> = arr(sg, "frames")
                    .iter()
                    .chain(arr(sg, "logicalSignals").iter().flat_map(|l| arr(l, "frames").iter()))
                    .collect();
                let has = shown.iter().any(|f| {
                    arr(f, "aspects")
                        .iter()
                        .any(|x| strict_eq(x.get("aspect"), a.get("aspect")) && (a.get("symbol").is_none() || strict_eq(x.get("symbol"), a.get("symbol"))))
                });
                if !has {
                    out.error(
                        "route_aspect_not_on_signal",
                        "routes",
                        &rid,
                        json!({ "signal": a.get("signalId"), "aspect": a.get("aspect"), "symbol": a.get("symbol").cloned().unwrap_or(Value::Null) }),
                        &format!("{}:{}", s(a.get("signalId")), s(a.get("aspect"))),
                    );
                }
            }
        }
    }
    for o in c("overlaps") {
        let oid = id_of(o);
        reference(&mut out, "overlaps", &oid, "signalling.signals", o.get("signalId"), "signalId");
        path(&mut out, "overlaps", o, o.get("path"), "path");
        at(&mut out, "overlaps", o, o.get("end"), "end");
        let run: Vec<Value> = o.get("path").map(|p| arr(p, "trackIds").to_vec()).unwrap_or_default();
        if let (Some(first), Some(t)) = (run.first(), track_of(o.get("signalId")))
            && s(Some(first)) != t
        {
            out.error("overlap_start_off_path", "overlaps", &oid, json!({}), "");
        }
        if let Some(end) = o.get("end").filter(|e| e.get("anchor").and_then(Value::as_str) == Some("station"))
            && !run.is_empty()
            && !run.iter().any(|t| strict_eq(Some(t), end.get("trackId")))
            && !beside(&run[run.len() - 1], end.get("trackId").unwrap_or(&Value::Null))
        {
            out.error("overlap_end_off_path", "overlaps", &oid, json!({}), "");
        }
        for t in arr(o, "clearSectionIds") {
            reference(&mut out, "overlaps", &oid, "signalling.trackSections", Some(t), "clearSectionIds");
        }
        for sw in arr(o, "switches") {
            reference(&mut out, "overlaps", &oid, "signalling.switchElements", sw.get("switchElementId"), "switches");
        }
    }

    // flank protection
    let mut asked: HashMap<String, Value> = HashMap::new();
    for f in c("flankProtection") {
        let fid = id_of(f);
        let req = &f["requester"];
        let se = req.get("switchElementId");
        reference(&mut out, "flankProtection", &fid, "signalling.switchElements", se, "requester");
        let key = format!("{}:{}", s(se), s(req.get("via")));
        if let Some(other) = asked.get(&key) {
            out.error("flank_asked_twice", "flankProtection", &fid, json!({ "other": other }), &key);
        } else {
            asked.insert(key, fid.clone());
        }
        let prot = &f["protection"];
        if truthy(prot.get("signalId")) {
            reference(&mut out, "flankProtection", &fid, "signalling.signals", prot.get("signalId"), "protection");
        }
        if truthy(prot.get("switchElementId")) {
            reference(
                &mut out,
                "flankProtection",
                &fid,
                "signalling.switchElements",
                prot.get("switchElementId"),
                "protection",
            );
            if strict_eq(prot.get("switchElementId"), se) {
                out.error("flank_by_itself", "flankProtection", &fid, json!({}), "");
            }
            let protecting = switch_elements.get(&s(prot.get("switchElementId")));
            if protecting.is_some_and(|p| truthy(p.get("switchId"))) && !truthy(prot.get("position")) {
                out.error("flank_position_missing", "flankProtection", &fid, json!({}), "");
            }
        }
        for side in ["leftId", "rightId"] {
            reference(
                &mut out,
                "flankProtection",
                &fid,
                "signalling.flankProtection",
                f.get("passOn").and_then(|p| p.get(side)),
                &format!("passOn.{side}"),
            );
        }
        for t in arr(f, "clearSectionIds") {
            reference(&mut out, "flankProtection", &fid, "signalling.trackSections", Some(t), "clearSectionIds");
        }
    }

    // balises and magnets
    for b in c("balises") {
        let bid = id_of(b);
        at(&mut out, "balises", b, b.get("at"), "at");
        reference(&mut out, "balises", &bid, "signalling.signals", b.get("signalId"), "signalId");
        for r in arr(b, "routes") {
            reference(&mut out, "balises", &bid, "signalling.routes", r.get("routeId"), "routes");
        }
        if b.get("effect").and_then(Value::as_str) == Some("switchedBySignal") && !truthy(b.get("signalId")) {
            out.error("balise_switched_without_signal", "balises", &bid, json!({}), "");
        }
        let system = b.get("system").and_then(Value::as_str);
        let Some(sys) = protection_system_of(system) else {
            out.error("system_unknown", "balises", &bid, json!({ "system": b.get("system") }), "");
            continue;
        };
        let Some(kind) = balise_kind_of(system, b.get("kind").and_then(Value::as_str)) else {
            out.error(
                "kind_not_in_system",
                "balises",
                &bid,
                json!({ "kind": b.get("kind"), "system": b.get("system") }),
                "",
            );
            continue;
        };
        let pflicht = kind.get("pflicht").and_then(Value::as_array).cloned().unwrap_or_default();
        let erlaubt = kind.get("erlaubt").and_then(Value::as_array).cloned();
        for (problem, setting) in settings_problems(b.get("settings"), sys.get("einstellungen"), &pflicht, erlaubt.as_deref()) {
            out.error(
                &format!("setting_{problem}"),
                "balises",
                &bid,
                json!({ "setting": setting, "system": b.get("system") }),
                &setting,
            );
        }
    }

    // station boundaries
    for b in c("stationBoundaries") {
        let bid = id_of(b);
        reference(
            &mut out,
            "stationBoundaries",
            &bid,
            "core.operationalPoints",
            b.get("operationalPointId"),
            "operationalPointId",
        );
        for m in arr(b, "marks") {
            if truthy(m.get("switchId")) {
                reference(&mut out, "stationBoundaries", &bid, "core.switches", m.get("switchId"), "marks");
                continue;
            }
            if truthy(m.get("stoppingPlaceId")) {
                reference(&mut out, "stationBoundaries", &bid, "core.stoppingPlaces", m.get("stoppingPlaceId"), "marks");
                continue;
            }
            if truthy(m.get("platformId")) {
                reference(&mut out, "stationBoundaries", &bid, "civil.platforms", m.get("platformId"), "marks");
                continue;
            }
            reference(&mut out, "stationBoundaries", &bid, "signalling.signals", m.get("signalId"), "marks");
            if let Some(sg) = m.get("signalId").and_then(|x| signals.get(&s(Some(x)))) {
                let fns: Vec<Option<&str>> = std::iter::once(sg.get("function").and_then(Value::as_str))
                    .chain(arr(sg, "logicalSignals").iter().map(|l| l.get("function").and_then(Value::as_str)))
                    .collect();
                let entry = fns.iter().any(|f| matches!(f, Some("entry") | Some("entryExit") | Some("entryBlock")));
                let board = arr(sg, "frames")
                    .iter()
                    .any(|f| arr(f, "aspects").iter().any(|a| a.get("aspect").and_then(Value::as_str) == Some("Ne_1")));
                if !entry && !board {
                    out.warning(
                        "boundary_mark_special",
                        "stationBoundaries",
                        &bid,
                        json!({ "signal": m.get("signalId") }),
                        &s(m.get("signalId")),
                    );
                }
                if !strict_eq(sg.get("operationalPointId"), b.get("operationalPointId")) {
                    out.warning(
                        "boundary_mark_elsewhere",
                        "stationBoundaries",
                        &bid,
                        json!({ "signal": m.get("signalId") }),
                        &s(m.get("signalId")),
                    );
                }
            }
        }
    }

    // stopping places
    let core_platforms: HashMap<String, &Value> = civil_platforms.iter().map(|p| (s(p.get("id")), *p)).collect();
    let shows_screen = |sg: &Value| {
        arr(sg, "frames")
            .iter()
            .chain(arr(sg, "logicalSignals").iter().flat_map(|l| arr(l, "frames").iter()))
            .any(is_screen)
    };
    let mut bounds: Vec<(&Value, Value)> = Vec::new();
    for sg in c("signals") {
        if sg.get("kind").and_then(Value::as_str) == Some("real") && shows_screen(sg) {
            for loc in std::iter::once(sg.get("at")).chain(arr(sg, "alsoAt").iter().map(Some)).flatten() {
                bounds.push((loc, json!({ "signal": sg.get("id") })));
            }
        }
    }
    for d in c("detectionPoints") {
        if let Some(loc) = d.get("at") {
            bounds.push((loc, json!({ "detectionPoint": d.get("id") })));
        }
    }
    bounds.retain(|(loc, _)| loc.get("anchor").and_then(Value::as_str) == Some("station"));
    let stopping: HashMap<String, &Value> = stopping_places.iter().map(|h| (s(h.get("id")), h)).collect();
    for h in stopping_places {
        let platform = platform_of(h);
        let hid = id_of(h);
        let r = &h["at"];
        reference(&mut out, "stoppingPlaces", &hid, "core.tracks", r.get("trackId"), "at.trackId");
        reference(
            &mut out,
            "stoppingPlaces",
            &hid,
            "core.operationalPoints",
            h.get("operationalPointId"),
            "operationalPointId",
        );
        reference(&mut out, "stoppingPlaces", &hid, "civil.platforms", platform.as_ref(), "platformId");
        let track = tracks.get(&s(r.get("trackId")));
        let (st, en) = (
            r.get("startStation").and_then(Value::as_f64).unwrap_or(f64::NAN),
            r.get("endStation").and_then(Value::as_f64).unwrap_or(f64::NAN),
        );
        if !(en > st) {
            out.error("stopping_place_empty", "stoppingPlaces", &hid, json!({}), "");
        } else if let Some(t) = track {
            let len = track_length(t);
            if st < -STATION_TOL || en > len + STATION_TOL {
                out.error(
                    "off_track",
                    "stoppingPlaces",
                    &hid,
                    json!({ "field": "at", "station": if st < 0.0 { r.get("startStation") } else { r.get("endStation") }, "length": number(len) }),
                    "at",
                );
            }
        }
        if !truthy(h.get("operationalPointId")) && platform.is_none() {
            out.warning("stopping_place_open_line", "stoppingPlaces", &hid, json!({}), "");
        }
        if let Some(pf_at) = platform
            .as_ref()
            .and_then(|p| core_platforms.get(&s(Some(p))))
            .and_then(|p| p.get("at"))
            .filter(|a| truthy(Some(a)))
        {
            let (ps, pe) = (js_number(pf_at.get("startStation")), js_number(pf_at.get("endStation")));
            if !strict_eq(pf_at.get("trackId"), r.get("trackId")) || st < ps - STOP_TOL || en > pe + STOP_TOL {
                out.warning("stopping_place_off_platform", "stoppingPlaces", &hid, json!({}), "");
            }
        }
        for (loc, refj) in &bounds {
            let ls = loc.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN);
            if !strict_eq(loc.get("trackId"), r.get("trackId")) || ls <= st + STOP_TOL || ls >= en - STOP_TOL {
                continue;
            }
            let extra = s(refj.get("signal").or_else(|| refj.get("detectionPoint")));
            out.warning("stopping_place_crossed", "stoppingPlaces", &hid, refj.clone(), &extra);
        }
    }

    // itineraries
    for it in c("itineraries") {
        let iid = id_of(it);
        for field in ["startStoppingPlaceId", "targetStoppingPlaceId"] {
            reference(&mut out, "itineraries", &iid, "core.stoppingPlaces", it.get(field), field);
        }
        for r in arr(it, "routeIds") {
            reference(&mut out, "itineraries", &iid, "signalling.routes", Some(r), "routeIds");
        }
        path(&mut out, "itineraries", it, it.get("path"), "path");
        let run: Vec<Value> = it.get("path").map(|p| arr(p, "trackIds").to_vec()).unwrap_or_default();
        if run.is_empty() {
            continue;
        }
        let start = it.get("startStoppingPlaceId").and_then(|x| stopping.get(&s(Some(x))));
        let target = it.get("targetStoppingPlaceId").and_then(|x| stopping.get(&s(Some(x))));
        if start.is_some_and(|sp| !strict_eq(sp["at"].get("trackId"), Some(&run[0])))
            || target.is_some_and(|tp| !strict_eq(tp["at"].get("trackId"), Some(&run[run.len() - 1])))
        {
            out.error("itinerary_off_stopping_place", "itineraries", &iid, json!({}), "");
        }
        let mut from = 0usize;
        for id in arr(it, "routeIds") {
            let Some(route_ids) = routes_by_id
                .get(&s(Some(id)))
                .and_then(|r| r.get("path"))
                .map(|p| arr(p, "trackIds"))
                .filter(|v| !v.is_empty())
            else {
                continue;
            };
            let mut found: Option<usize> = None;
            let mut i = from.saturating_sub(1);
            while i + route_ids.len() <= run.len() && found.is_none() {
                if route_ids.iter().enumerate().all(|(k, t)| strict_eq(Some(&run[i + k]), Some(t))) {
                    found = Some(i);
                }
                i += 1;
            }
            match found {
                None => out.error("itinerary_route_off_path", "itineraries", &iid, json!({ "route": id }), &s(Some(id))),
                Some(at) => from = at + route_ids.len(),
            }
        }
    }
    json!({ "errors": out.errors, "warnings": out.warnings })
}
