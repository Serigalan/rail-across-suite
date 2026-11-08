// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! `validateProject`, `newFindings` and `overlappingTracks`.

use std::collections::{HashMap, HashSet};

use rail_across_geom::chain::{epsg_mismatches, node_gaps, untrue_lengths};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{is_bloss, sample_transition};
use rail_across_js::{cmp_utf16, get, js_string, number, truthy};
use rail_across_survey::axis::survey_defect;
use rail_across_survey::reference::reference_axis_defect;
use rail_across_track::end_marks::end_key;
use rail_across_track::model::track_label;
use rail_across_track::platforms::point_on_element;
use rail_across_track::routes::resolve_route;
use rail_across_track::switch_model::{port_end, port_track, ports_of};
use serde_json::{Value, json};

const GAP_WARNING: f64 = 0.01;
const GAP_ERROR: f64 = 0.5;
const STATION_TOL: f64 = 0.01;
const OVERLAP_DIST: f64 = 0.5;
const OVERLAP_LENGTH: f64 = 10.0;
const SAMPLE_STEP: f64 = 2.0;
const CELL: f64 = 5.0;

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn track_length(t: &Value) -> f64 {
    arr(t, "elements").iter().map(|e| get(e, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum()
}

/// An object of the members that are there (`undefined` left out, as JSON.stringify does).
fn obj(members: Vec<(&str, Option<Value>)>) -> Value {
    Value::Object(members.into_iter().filter_map(|(k, v)| v.map(|v| (k.to_string(), v))).collect())
}

/// `${extra}` where it is truthy.
fn extra_of(v: &Value) -> Option<String> {
    truthy(Some(v)).then(|| js_string(Some(v)))
}

struct Findings {
    errors: Vec<Value>,
    warnings: Vec<Value>,
}

impl Findings {
    #[allow(clippy::too_many_arguments)]
    fn add(&mut self, error: bool, code: &str, collection: &str, id: Option<&Value>, label: Option<Value>, params: Value, extra: Option<String>) {
        let id_text = id.map(|v| js_string(Some(v))).unwrap_or_else(|| "undefined".into());
        let key = match extra {
            Some(x) if !x.is_empty() => format!("{code}:{collection}:{id_text}:{x}"),
            _ => format!("{code}:{collection}:{id_text}"),
        };
        let f = obj(vec![
            ("code", Some(json!(code))),
            ("key", Some(json!(key))),
            ("collection", Some(json!(collection))),
            ("id", id.cloned()),
            ("label", label),
            ("params", Some(params)),
        ]);
        if error { self.errors.push(f) } else { self.warnings.push(f) }
    }
}

/// `a ?? b ?? c` over members.
fn first_of(v: &Value, keys: &[&str]) -> Option<Value> {
    keys.iter()
        .find_map(|k| get(v, k).cloned())
        .or_else(|| keys.last().and_then(|k| v.get(*k).cloned()))
}

/// Whether a project record holds together (`validateProject`): { errors, warnings }.
pub fn validate_project(project: &Value) -> Value {
    let mut fx = Findings {
        errors: vec![],
        warnings: vec![],
    };
    let tracks = arr(project, "tracks");
    let switches = arr(project, "switches");
    let by_id: HashMap<String, &Value> = tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
    let track_of = |id: Option<&Value>| id.filter(|v| !v.is_null()).and_then(|v| by_id.get(&js_string(Some(v))).copied());

    // switches: every port on a track and an end, no end held twice
    let mut held: HashMap<String, (Option<Value>, Option<Value>)> = HashMap::new();
    for sw in switches {
        let label = first_of(sw, &["name", "label", "switchId"]);
        for (port, _) in ports_of(sw) {
            let track_id = port_track(sw, port);
            let endpoint = sw.get("ports").and_then(|p| p.get(port)).and_then(|p| p.get("endpoint")).cloned();
            if !truthy(track_id) || track_of(track_id).is_none() {
                fx.add(
                    true,
                    "port_track_missing",
                    "switches",
                    sw.get("switchId"),
                    label.clone(),
                    obj(vec![("port", Some(json!(port))), ("trackId", track_id.cloned())]),
                    Some(port.into()),
                );
                continue;
            }
            let ep = port_end(sw, port);
            if ep != Some("BEGIN") && ep != Some("END") {
                fx.add(
                    true,
                    "port_endpoint_invalid",
                    "switches",
                    sw.get("switchId"),
                    label.clone(),
                    obj(vec![("port", Some(json!(port))), ("endpoint", endpoint)]),
                    Some(port.into()),
                );
                continue;
            }
            let k = end_key(track_id, endpoint.as_ref());
            match held.get(&k) {
                Some((sid, other)) if !rail_across_js::strict_eq(sid.as_ref(), sw.get("switchId")) => {
                    fx.add(
                        true,
                        "end_held_twice",
                        "switches",
                        sw.get("switchId"),
                        label.clone(),
                        obj(vec![
                            ("port", Some(json!(port))),
                            ("track", Some(json!(track_label(track_of(track_id))))),
                            ("endpoint", endpoint),
                            ("other", other.clone()),
                        ]),
                        Some(port.into()),
                    );
                }
                Some(_) => {}
                None => {
                    held.insert(k, (sw.get("switchId").cloned(), label.clone()));
                }
            }
        }
    }

    // tracks: their switch elements, chains, heights
    let switch_ids: HashSet<String> = switches.iter().map(|sw| js_string(sw.get("switchId"))).collect();
    for t in tracks {
        let label = Some(json!(track_label(Some(t))));
        let els = arr(t, "elements");
        let mut missing: Vec<&Value> = Vec::new();
        for el in els {
            if let Some(sid) = el.get("switchId").filter(|v| truthy(Some(v)))
                && !switch_ids.contains(&js_string(Some(sid)))
                && !missing.iter().any(|m| rail_across_js::strict_eq(Some(m), Some(sid)))
            {
                missing.push(sid);
            }
        }
        for sid in missing {
            fx.add(
                true,
                "element_switch_missing",
                "tracks",
                t.get("id"),
                label.clone(),
                json!({ "switchId": sid }),
                extra_of(sid),
            );
        }
        for (index, gap) in node_gaps(els, GAP_WARNING) {
            fx.add(
                gap > GAP_ERROR,
                "chain_gap",
                "tracks",
                t.get("id"),
                label.clone(),
                json!({ "index": index, "gap": number(gap) }),
                extra_of(&json!(index)),
            );
        }
        for (index, _, expected) in untrue_lengths(els, NODE_TOL) {
            let params = obj(vec![
                ("index", Some(json!(index))),
                ("length", els[index].get("length").cloned()),
                ("expected", Some(number(expected))),
            ]);
            fx.add(false, "chain_length", "tracks", t.get("id"), label.clone(), params, extra_of(&json!(index)));
        }
        for (index, epsg) in epsg_mismatches(t) {
            fx.add(
                true,
                "chain_epsg",
                "tracks",
                t.get("id"),
                label.clone(),
                json!({ "index": index, "epsg": epsg }),
                extra_of(&json!(index)),
            );
        }
        let length = track_length(t);
        let heights = arr(t, "heights");
        for (i, h) in heights.iter().enumerate() {
            let st = h.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN);
            if !(st >= -STATION_TOL && st <= length + STATION_TOL) {
                fx.add(
                    true,
                    "height_out_of_range",
                    "tracks",
                    t.get("id"),
                    label.clone(),
                    obj(vec![("station", h.get("station").cloned()), ("length", Some(number(length)))]),
                    Some(format!("h{i}")),
                );
            }
            if i > 0 && !(st > heights[i - 1].get("station").and_then(Value::as_f64).unwrap_or(f64::NAN)) {
                fx.add(
                    true,
                    "height_not_rising",
                    "tracks",
                    t.get("id"),
                    label.clone(),
                    obj(vec![("station", h.get("station").cloned())]),
                    Some(format!("r{i}")),
                );
            }
        }
    }

    // platforms on their track, within its length
    for pf in arr(project, "platforms") {
        let parts: Vec<String> = ["stationName", "code"]
            .iter()
            .filter_map(|k| pf.get(*k).filter(|v| truthy(Some(v))).map(|v| js_string(Some(v))))
            .collect();
        let joined = parts.join(" ");
        let label = if joined.is_empty() { pf.get("id").cloned() } else { Some(json!(joined)) };
        let Some(t) = track_of(pf.get("trackId")) else {
            fx.add(
                true,
                "platform_track_missing",
                "platforms",
                pf.get("id"),
                label,
                obj(vec![("trackId", pf.get("trackId").cloned())]),
                None,
            );
            continue;
        };
        let length = track_length(t);
        let inside = |k: &str| {
            pf.get(k)
                .and_then(Value::as_f64)
                .is_some_and(|s| s.is_finite() && s >= -STATION_TOL && s <= length + STATION_TOL)
        };
        if !inside("startStation") || !inside("endStation") {
            fx.add(
                true,
                "platform_out_of_range",
                "platforms",
                pf.get("id"),
                label,
                obj(vec![
                    ("track", Some(json!(track_label(Some(t))))),
                    ("start", pf.get("startStation").cloned()),
                    ("end", pf.get("endStation").cloned()),
                    ("length", Some(number(length))),
                ]),
                None,
            );
        }
    }

    // end marks only on free ends
    let mut seen: HashSet<String> = HashSet::new();
    for m in arr(project, "endMarks") {
        let k = end_key(m.get("trackId"), m.get("endpoint"));
        let t = track_of(m.get("trackId"));
        let label = match t {
            Some(t) => Some(json!(format!("{} {}", track_label(Some(t)), js_string(m.get("endpoint"))))),
            None => m.get("id").cloned(),
        };
        if t.is_none() {
            fx.add(
                true,
                "end_mark_track_missing",
                "endMarks",
                m.get("id"),
                label,
                obj(vec![("trackId", m.get("trackId").cloned())]),
                None,
            );
        } else if let Some((_, other)) = held.get(&k) {
            fx.add(
                true,
                "end_mark_not_free",
                "endMarks",
                m.get("id"),
                label,
                obj(vec![("switch", other.clone())]),
                None,
            );
        } else if seen.contains(&k) {
            fx.add(true, "end_mark_twice", "endMarks", m.get("id"), label, json!({}), None);
        }
        seen.insert(k);
    }

    // measured axes
    let qm = json!("?");
    for s in arr(project, "axisSurveys") {
        if let Some(defect) = survey_defect(s) {
            let label = first_of(s, &["name", "id"]);
            fx.add(
                true,
                "axis_survey_malformed",
                "axisSurveys",
                Some(get(s, "id").unwrap_or(&qm)),
                label,
                json!({ "field": defect }),
                None,
            );
        }
    }
    for a in arr(project, "referenceAxes") {
        if let Some(defect) = reference_axis_defect(a) {
            let label = first_of(a, &["name", "id"]);
            fx.add(
                true,
                "reference_axis_malformed",
                "referenceAxes",
                Some(get(a, "id").unwrap_or(&qm)),
                label,
                json!({ "field": defect }),
                None,
            );
        }
    }

    // routes
    for r in arr(project, "routes") {
        let label = first_of(r, &["name", "id"]);
        let ids = r.get("trackIds").and_then(Value::as_array);
        if ids.is_none_or(Vec::is_empty) {
            fx.add(true, "route_malformed", "routes", Some(get(r, "id").unwrap_or(&qm)), label, json!({}), None);
            continue;
        }
        let resolved = resolve_route(r, tracks, switches);
        for g in resolved.get("gaps").and_then(Value::as_array).into_iter().flatten() {
            let missing = g.get("kind").and_then(Value::as_str) == Some("missing");
            let (code, params, extra) = if missing {
                (
                    "route_track_missing",
                    obj(vec![("trackId", g.get("trackId").cloned())]),
                    g.get("trackId").and_then(extra_of),
                )
            } else {
                ("route_gap", json!({}), Some(format!("g{}", js_string(g.get("after")))))
            };
            fx.add(false, code, "routes", r.get("id"), label.clone(), params, extra);
        }
    }

    // tracks drawn twice
    for o in overlapping_tracks(tracks, switches) {
        let (a, b) = (&o["a"], &o["b"]);
        fx.add(
            false,
            "tracks_overlap",
            "tracks",
            Some(a),
            Some(json!(track_label(track_of(Some(a))))),
            json!({ "other": track_label(track_of(Some(b))), "otherId": b, "length": o["length"] }),
            extra_of(b),
        );
    }
    json!({ "errors": fx.errors, "warnings": fx.warnings })
}

/// The findings of `next` that `previous` does not have, by key (`newFindings`).
pub fn new_findings(next: &[Value], previous: &[Value]) -> Vec<Value> {
    let known: HashSet<String> = previous.iter().map(|f| js_string(f.get("key"))).collect();
    next.iter().filter(|f| !known.contains(&js_string(f.get("key")))).cloned().collect()
}

#[derive(Clone, Copy)]
struct Pt {
    x: f64,
    y: f64,
    station: f64,
}

fn sample_plane(track: &Value) -> Vec<Pt> {
    let mut pts: Vec<Pt> = Vec::new();
    let mut station = 0.0;
    let epsg = rail_across_js::js_number(track.get("epsg")) as i64;
    for el in arr(track, "elements") {
        let l = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        let start = el.get("startNode").and_then(Value::as_array);
        let Some(start) = start.filter(|_| l > 0.0) else {
            station += l;
            continue;
        };
        let n = 1usize.max((l / SAMPLE_STEP).ceil() as usize);
        let f = |k: &str| get(el, k).and_then(Value::as_f64);
        let run = (el.get("elementType").and_then(Value::as_f64) == Some(2.0)).then(|| {
            let s = |i: usize| start.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
            sample_transition(
                s(0),
                s(1),
                f("bearing").unwrap_or(f64::NAN),
                l,
                f("r1"),
                f("r2"),
                is_bloss(el.get("transitionType").and_then(Value::as_str)),
                n,
                4,
                l,
            )
        });
        for i in (if pts.is_empty() { 0 } else { 1 })..=n {
            let s = l * i as f64 / n as f64;
            let [x, y] = match &run {
                Some(r) => r[i],
                None => point_on_element(el, epsg, s).map(|(u, _)| [u.e, u.n]).unwrap_or([f64::NAN, f64::NAN]),
            };
            pts.push(Pt { x, y, station: station + s });
        }
        station += l;
    }
    pts
}

fn seg_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let u = if l2 > 0.0 {
        1f64.min(0f64.max(((px - ax) * dx + (py - ay) * dy) / l2))
    } else {
        0.0
    };
    (ax + u * dx - px).hypot(ay + u * dy - py)
}

#[derive(Clone, Copy)]
struct Run {
    from: f64,
    to: f64,
    start: Pt,
    end: Pt,
}

/// Pairs of tracks that lie on each other (`overlappingTracks`): [{ a, b, length }].
pub fn overlapping_tracks(tracks: &[Value], switches: &[Value]) -> Vec<Value> {
    let mut together: HashSet<(String, String)> = HashSet::new();
    for sw in switches {
        let ids: Vec<String> = ports_of(sw)
            .into_iter()
            .filter_map(|(p, _)| port_track(sw, p).filter(|v| truthy(Some(v))).map(|v| js_string(Some(v))))
            .collect();
        for a in &ids {
            for b in &ids {
                if a != b {
                    together.insert((a.clone(), b.clone()));
                }
            }
        }
    }
    let id_of = |t: &Value| js_string(t.get("id"));
    let mut samples: HashMap<String, Vec<Pt>> = HashMap::new();
    let mut lengths: HashMap<String, f64> = HashMap::new();
    for t in tracks {
        samples.insert(id_of(t), sample_plane(t));
        lengths.insert(id_of(t), track_length(t));
    }
    let ends: HashMap<String, Vec<Pt>> = samples
        .iter()
        .map(|(id, pts)| (id.clone(), if pts.is_empty() { vec![] } else { vec![pts[0], pts[pts.len() - 1]] }))
        .collect();
    let near = |p: &Pt, q: &Pt| (p.x - q.x).hypot(p.y - q.y) < 1.0;
    let from_shared_end = |a: &str, b: &str, run: &Run| {
        ends[a]
            .iter()
            .any(|ea| ends[b].iter().any(|eb| near(ea, eb) && (near(ea, &run.start) || near(ea, &run.end))))
    };
    let keep = |a: &str, b: &str, run: &Run| !from_shared_end(a, b, run) || run.to - run.from >= 0.5 * lengths[a].min(lengths[b]);
    let mut grid: HashMap<(String, i64, i64), Vec<(String, Pt, Pt)>> = HashMap::new();
    for t in tracks {
        let pts = &samples[&id_of(t)];
        let epsg = js_string(t.get("epsg"));
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let c = |v: f64| (v / CELL).floor() as i64;
            for cx in c(a.x.min(b.x))..=c(a.x.max(b.x)) {
                for cy in c(a.y.min(b.y))..=c(a.y.max(b.y)) {
                    grid.entry((epsg.clone(), cx, cy)).or_default().push((id_of(t), a, b));
                }
            }
        }
    }
    // Found pairs in the order first found, by key.
    let mut found: Vec<(String, Value, Value, f64)> = Vec::new();
    let id_value: HashMap<String, Value> = tracks.iter().map(|t| (id_of(t), t.get("id").cloned().unwrap_or(Value::Null))).collect();
    let record = |found: &mut Vec<(String, Value, Value, f64)>, a: &str, b: &str, run: &Run| {
        let length = run.to - run.from;
        if !(length > OVERLAP_LENGTH) {
            return;
        }
        let (x, y) = if cmp_utf16(a, b).is_lt() { (a, b) } else { (b, a) };
        let k = format!("{x}|{y}");
        match found.iter_mut().find(|f| f.0 == k) {
            Some(f) => {
                if f.3 < length {
                    f.3 = length;
                }
            }
            None => found.push((k, id_value[x].clone(), id_value[y].clone(), length)),
        }
    };
    for t in tracks {
        let tid = id_of(t);
        let epsg = js_string(t.get("epsg"));
        let mut runs: Vec<(String, Run)> = Vec::new();
        for p in &samples[&tid] {
            let (cx, cy) = ((p.x / CELL).floor() as i64, (p.y / CELL).floor() as i64);
            let mut close: Vec<(String, f64)> = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for (sid, a, b) in grid.get(&(epsg.clone(), cx + dx, cy + dy)).map(Vec::as_slice).unwrap_or(&[]) {
                        if *sid == tid || together.contains(&(tid.clone(), sid.clone())) {
                            continue;
                        }
                        let d = seg_dist(p.x, p.y, a.x, a.y, b.x, b.y);
                        match close.iter_mut().find(|c| c.0 == *sid) {
                            Some(c) => {
                                if d < c.1 {
                                    c.1 = d;
                                }
                            }
                            None => {
                                if d < f64::INFINITY {
                                    close.push((sid.clone(), d));
                                }
                            }
                        }
                    }
                }
            }
            let mut i = 0;
            while i < runs.len() {
                let other = runs[i].0.clone();
                let d = close.iter().find(|c| c.0 == other).map(|c| c.1).unwrap_or(f64::INFINITY);
                if !(d < OVERLAP_DIST) {
                    let run = runs[i].1;
                    if keep(&tid, &other, &run) {
                        record(&mut found, &tid, &other, &run);
                    }
                    runs.remove(i);
                } else {
                    i += 1;
                }
            }
            for (other, d) in &close {
                if *d >= OVERLAP_DIST {
                    continue;
                }
                match runs.iter_mut().find(|r| r.0 == *other) {
                    Some(r) => {
                        r.1.to = p.station;
                        r.1.end = *p;
                    }
                    None => runs.push((
                        other.clone(),
                        Run {
                            from: p.station,
                            to: p.station,
                            start: *p,
                            end: *p,
                        },
                    )),
                }
            }
        }
        for (other, run) in &runs {
            if keep(&tid, other, run) {
                record(&mut found, &tid, other, run);
            }
        }
    }
    found
        .into_iter()
        .map(|(_, a, b, length)| json!({ "a": a, "b": b, "length": number(length) }))
        .collect()
}
