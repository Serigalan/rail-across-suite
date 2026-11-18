// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! How the tracks hang together (`topology.js`), the joints between them
//! (`trackLinkUtils.js`) and kilometrage along them (`kmLines.js`).

use std::collections::{HashMap, HashSet, VecDeque};

use rail_across_geom::coords::{crs_name, plane_to_wgs84, transform_grid_bearing, transform_plane_point};
use rail_across_geom::element::resolve_end_bearing;
use rail_across_js::{Object, cmp_utf16, js_round, js_string, number, strict_eq, truthy};
use serde_json::{Value, json};

use crate::end_marks::{BOUNDARY, BUFFER_STOP, end_key};
use crate::switch_model::{LINK_KIND, is_link_switch, joined_ports, new_switch_fields, port_end, port_track, ports_of, switch_route_ports};
use crate::util::{elements, epsg, text};

const JOINT_EXACT: f64 = 0.01;
const CELL: f64 = 2e-5;
pub const JOINT_TOL: f64 = 1.5;
const BEARING_TOL: f64 = 2.0;
const LINK_BODY_HALF: f64 = 2.0;

fn bearing_delta(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs()
}

/// A track end: where it is and which way it leaves (`trackEndAt`).
#[derive(Clone, Debug)]
pub struct End {
    pub track_id: Value,
    pub track_name: Value,
    pub endpoint: &'static str,
    pub epsg: i64,
    pub easting: f64,
    pub northing: f64,
    pub outward: f64,
}

impl End {
    /// An end as the JavaScript hands one over.
    pub fn from_json(v: &Value) -> End {
        let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
        End {
            track_id: v.get("trackId").cloned().unwrap_or(Value::Null),
            track_name: v.get("trackName").cloned().unwrap_or(Value::Null),
            endpoint: if v.get("endpoint").and_then(Value::as_str) == Some("END") {
                "END"
            } else {
                "BEGIN"
            },
            epsg: rail_across_js::js_number(v.get("epsg")) as i64,
            easting: f("easting"),
            northing: f("northing"),
            outward: f("outward"),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({ "trackId": self.track_id, "trackName": self.track_name, "endpoint": self.endpoint, "epsg": self.epsg,
                "easting": number(self.easting), "northing": number(self.northing), "outward": number(self.outward) })
    }
}

/// A track's end (`trackEndAt`).
pub fn track_end_at(track: Option<&Value>, endpoint: &str) -> Option<End> {
    let track = track?;
    let els = elements(track);
    let el = if endpoint == "END" { els.last()? } else { els.first()? };
    let node = el
        .get(if endpoint == "END" { "endNode" } else { "startNode" })
        .filter(|v| truthy(Some(v)))?
        .as_array()?;
    let outward = if endpoint == "END" {
        (resolve_end_bearing(el, epsg(track)) + 180.0) % 360.0
    } else {
        el.get("bearing").and_then(Value::as_f64).unwrap_or(0.0)
    };
    Some(End {
        track_id: track.get("id").cloned().unwrap_or(Value::Null),
        track_name: rail_across_js::get(track, "name").cloned().unwrap_or(Value::Null),
        endpoint: if endpoint == "END" { "END" } else { "BEGIN" },
        epsg: epsg(track),
        easting: node.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
        northing: node.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
        outward,
    })
}

fn has_plane(track: &Value) -> bool {
    crs_name(epsg(track)).is_some() && track.get("epsg").is_some()
}

/// The switch nodes, where their ports meet (`switchNodes`).
pub fn switch_nodes(tracks: &[Value], switches: &[Value]) -> Value {
    let by_id: HashMap<String, &Value> = tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
    let mut nodes = Vec::new();
    for sw in switches {
        let mut points = Vec::new();
        for (p, _) in ports_of(sw) {
            let Some(track) = port_track(sw, p).and_then(|id| by_id.get(&js_string(Some(id)))).copied() else {
                continue;
            };
            if !has_plane(track) {
                continue;
            }
            if let Some(end) = track_end_at(Some(track), port_end(sw, p).unwrap_or(""))
                && let Ok(ll) = plane_to_wgs84(end.easting, end.northing, end.epsg)
            {
                points.push(ll);
            }
        }
        if points.is_empty() {
            continue;
        }
        let n = points.len() as f64;
        let lng = points.iter().map(|p| p.0).sum::<f64>() / n;
        let lat = points.iter().map(|p| p.1).sum::<f64>() / n;
        nodes.push(json!({ "switchId": sw.get("switchId"), "name": rail_across_js::get(sw, "name").cloned().unwrap_or(Value::Null), "kind": sw.get("kind"), "link": is_link_switch(sw), "lngLat": [number(lng), number(lat)] }));
    }
    Value::Array(nodes)
}

fn port_held(switches: &[Value]) -> HashMap<String, bool> {
    let mut held = HashMap::new();
    for sw in switches {
        for (p, _) in ports_of(sw) {
            if let (Some(t), Some(e)) = (port_track(sw, p).filter(|v| truthy(Some(v))), port_end(sw, p)) {
                held.insert(end_key(Some(t), Some(&json!(e))), is_link_switch(sw));
            }
        }
    }
    held
}

/// Every track end and what it is (`classifyTrackEnds`).
pub fn classify_track_ends(tracks: &[Value], switches: &[Value], end_marks: &[Value]) -> Value {
    let held = port_held(switches);
    let marks: HashMap<String, &Value> = end_marks.iter().map(|m| (end_key(m.get("trackId"), m.get("endpoint")), m)).collect();
    struct E {
        end: End,
        lng_lat: (f64, f64),
        state: Option<String>,
        mark_id: Option<Value>,
        cx: i64,
        cy: i64,
    }
    let mut ends: Vec<E> = Vec::new();
    for track in tracks {
        if !has_plane(track) {
            continue;
        }
        for endpoint in ["BEGIN", "END"] {
            let Some(end) = track_end_at(Some(track), endpoint) else { continue };
            let lng_lat = plane_to_wgs84(end.easting, end.northing, end.epsg).unwrap_or((f64::NAN, f64::NAN));
            let key = end_key(Some(&end.track_id), Some(&json!(endpoint)));
            let mark = marks.get(&key);
            let (state, mark_id) = if let Some(link) = held.get(&key) {
                (Some(if *link { "link" } else { "switch" }.to_string()), None)
            } else if mark.is_some_and(|m| text(m, "kind") == Some(BUFFER_STOP)) {
                (Some("buffer_stop".to_string()), mark.and_then(|m| m.get("id").cloned()))
            } else if mark.is_some_and(|m| text(m, "kind") == Some(BOUNDARY)) {
                (Some("boundary".to_string()), mark.and_then(|m| m.get("id").cloned()))
            } else {
                (None, None)
            };
            let cx = js_round(lng_lat.0 / CELL) as i64;
            let cy = js_round(lng_lat.1 / CELL) as i64;
            ends.push(E {
                end,
                lng_lat,
                state,
                mark_id,
                cx,
                cy,
            });
        }
    }
    let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, e) in ends.iter().enumerate() {
        cells.entry((e.cx, e.cy)).or_default().push(i);
    }
    for i in 0..ends.len() {
        if ends[i].state.is_some() {
            continue;
        }
        let (mut partners, mut crowded) = (0, false);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &j in cells.get(&(ends[i].cx + dx, ends[i].cy + dy)).map(Vec::as_slice).unwrap_or(&[]) {
                    if j == i {
                        continue;
                    }
                    let (a, b) = (&ends[i].end, &ends[j].end);
                    if strict_eq(Some(&b.track_id), Some(&a.track_id)) || b.epsg != a.epsg {
                        continue;
                    }
                    if (a.easting - b.easting).hypot(a.northing - b.northing) > JOINT_EXACT {
                        continue;
                    }
                    if held.contains_key(&end_key(Some(&b.track_id), Some(&json!(b.endpoint)))) {
                        crowded = true;
                    } else {
                        partners += 1;
                    }
                }
            }
        }
        ends[i].state = Some(if partners == 1 && !crowded { "joint" } else { "open" }.to_string());
    }
    Value::Array(
        ends.into_iter()
            .map(|e| {
                let mut o = e.end.to_json().as_object().cloned().unwrap_or_default();
                o.insert("lngLat".into(), json!([number(e.lng_lat.0), number(e.lng_lat.1)]));
                if let Some(id) = e.mark_id {
                    o.insert("markId".into(), id);
                }
                o.insert("state".into(), json!(e.state));
                // The JavaScript set `state` before `markId`; members compare in any order.
                Value::Object(o)
            })
            .collect(),
    )
}

/// The ends nobody holds (`freeEnds`).
pub fn free_ends(ends: &[Value]) -> Vec<Value> {
    ends.iter().filter(|e| text(e, "state") == Some("open")).cloned().collect()
}

/// The track ends one end runs on into (`tracksOnFrom`): (trackId, endpoint).
pub fn tracks_on_from(track_id: &Value, endpoint: &str, tracks: &[Value], switches: &[Value]) -> Vec<(Value, String)> {
    let mut found: Vec<(Value, String)> = Vec::new();
    let mut add = |id: Option<&Value>, ep: Option<&str>| {
        let (Some(id), Some(ep)) = (id.filter(|v| truthy(Some(v))), ep.filter(|e| !e.is_empty())) else {
            return;
        };
        if !found.iter().any(|(f, e)| strict_eq(Some(f), Some(id)) && e == ep) {
            found.push((id.clone(), ep.to_string()));
        }
    };
    let mut held = false;
    for sw in switches {
        let ports = ports_of(sw);
        let mine: Vec<&str> = ports
            .iter()
            .filter(|(p, _)| strict_eq(port_track(sw, p), Some(track_id)) && port_end(sw, p) == Some(endpoint))
            .map(|(p, _)| *p)
            .collect();
        if mine.is_empty() {
            continue;
        }
        held = true;
        let routes: Vec<[&str; 2]> = switch_route_ports(text(sw, "kind")).into_iter().map(|(_, r)| r).collect();
        for port in mine {
            let others: Vec<&str> = if is_link_switch(sw) {
                joined_ports(sw, port)
            } else {
                routes
                    .iter()
                    .filter(|r| r.contains(&port))
                    .map(|r| if r[0] == port { r[1] } else { r[0] })
                    .collect()
            };
            for other in others {
                if ports.iter().any(|(p, _)| *p == other) {
                    add(port_track(sw, other), port_end(sw, other));
                }
            }
        }
    }
    if held {
        return found;
    }
    let Some(here) = track_end_at(tracks.iter().find(|t| strict_eq(t.get("id"), Some(track_id))), endpoint) else {
        return found;
    };
    let claimed: HashSet<String> = port_held(switches).into_keys().collect();
    let mut partners: Vec<End> = Vec::new();
    for track in tracks {
        if strict_eq(track.get("id"), Some(track_id)) {
            continue;
        }
        for ep in ["BEGIN", "END"] {
            let Some(end) = track_end_at(Some(track), ep) else { continue };
            if end.epsg != here.epsg || (end.easting - here.easting).hypot(end.northing - here.northing) > JOINT_EXACT {
                continue;
            }
            if claimed.contains(&end_key(track.get("id"), Some(&json!(ep)))) {
                return found;
            }
            partners.push(end);
        }
    }
    if partners.len() == 1 {
        add(Some(&partners[0].track_id), Some(partners[0].endpoint));
    }
    found
}

/// `tracksOnFrom` as the JavaScript returns it.
pub fn tracks_on_from_json(found: Vec<(Value, String)>) -> Value {
    Value::Array(found.into_iter().map(|(t, e)| json!({ "trackId": t, "endpoint": e })).collect())
}

// ── Joints (trackLinkUtils.js) ──────────────────────────────────────────────

/// The ends a switch port claims (`claimedEnds`).
pub fn claimed_ends(switches: &[Value]) -> HashSet<String> {
    port_held(switches).into_keys().collect()
}

/// How well two ends meet: (gap, bearingOff) (`jointFit`).
pub fn joint_fit(a: &End, b: &End) -> (f64, f64) {
    let (be, bn) = transform_plane_point(b.easting, b.northing, b.epsg, a.epsg).unwrap_or((f64::NAN, f64::NAN));
    let b_out = transform_grid_bearing(b.easting, b.northing, b.outward, b.epsg, a.epsg).unwrap_or(f64::NAN);
    ((be - a.easting).hypot(bn - a.northing), (bearing_delta(a.outward, b_out) - 180.0).abs())
}

fn all_track_ends(tracks: &[Value]) -> Vec<End> {
    tracks
        .iter()
        .filter(|t| has_plane(t))
        .flat_map(|t| ["BEGIN", "END"].into_iter().filter_map(move |e| track_end_at(Some(t), e)))
        .collect()
}

/// The track ends that meet each other in pairs (`findTrackJoints`): (joints, fanned).
pub fn find_track_joints(tracks: &[Value], switches: &[Value], tol: f64, bearing_tol: f64) -> (Vec<(End, End, f64, f64)>, usize) {
    let claimed = claimed_ends(switches);
    let ends: Vec<End> = all_track_ends(tracks)
        .into_iter()
        .filter(|e| !claimed.contains(&end_key(Some(&e.track_id), Some(&json!(e.endpoint)))))
        .collect();
    let cell: Vec<(i64, i64)> = ends
        .iter()
        .map(|e| {
            let (lng, lat) = plane_to_wgs84(e.easting, e.northing, e.epsg).unwrap_or((f64::NAN, f64::NAN));
            (js_round(lng / CELL) as i64, js_round(lat / CELL) as i64)
        })
        .collect();
    let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, c) in cell.iter().enumerate() {
        cells.entry(*c).or_default().push(i);
    }
    let key = |e: &End| format!("{}|{}", js_string(Some(&e.track_id)), e.endpoint);
    let mut meets: Vec<Vec<usize>> = vec![Vec::new(); ends.len()];
    for i in 0..ends.len() {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &j in cells.get(&(cell[i].0 + dx, cell[i].1 + dy)).map(Vec::as_slice).unwrap_or(&[]) {
                    if j == i || strict_eq(Some(&ends[j].track_id), Some(&ends[i].track_id)) {
                        continue;
                    }
                    if cmp_utf16(&key(&ends[j]), &key(&ends[i])) != std::cmp::Ordering::Greater {
                        continue;
                    }
                    let (gap, off) = joint_fit(&ends[i], &ends[j]);
                    if gap > tol || off > bearing_tol {
                        continue;
                    }
                    if !meets[i].contains(&j) {
                        meets[i].push(j);
                    }
                    if !meets[j].contains(&i) {
                        meets[j].push(i);
                    }
                }
            }
        }
    }
    let mut joints = Vec::new();
    let mut seen = vec![false; ends.len()];
    let mut fanned = 0;
    for i in 0..ends.len() {
        if seen[i] || meets[i].is_empty() {
            continue;
        }
        let mut group = vec![i];
        let mut queue = VecDeque::from([i]);
        while let Some(k) = queue.pop_front() {
            for &o in &meets[k] {
                if !group.contains(&o) {
                    group.push(o);
                    queue.push_back(o);
                }
            }
        }
        for &m in &group {
            seen[m] = true;
        }
        if group.len() != 2 {
            fanned += 1;
            continue;
        }
        let (a, b) = (ends[group[0]].clone(), ends[group[1]].clone());
        let (gap, off) = joint_fit(&a, &b);
        joints.push((a, b, gap, off));
    }
    joints.sort_by(|x, y| {
        let cx = x.0.epsg != x.1.epsg;
        let cy = y.0.epsg != y.1.epsg;
        (cy as i32 - cx as i32).cmp(&0).then(x.2.partial_cmp(&y.2).unwrap_or(std::cmp::Ordering::Equal))
    });
    (joints, fanned)
}

fn joint_json(j: &(End, End, f64, f64)) -> Value {
    json!({ "a": j.0.to_json(), "b": j.1.to_json(), "gap": number(j.2), "bearingOff": number(j.3), "crsChange": j.0.epsg != j.1.epsg })
}

/// `findTrackJoints` as the JavaScript returns it.
pub fn find_track_joints_json(tracks: &[Value], switches: &[Value], options: &Value) -> Value {
    let tol = options.get("tol").and_then(Value::as_f64).unwrap_or(JOINT_TOL);
    let btol = options.get("bearingTol").and_then(Value::as_f64).unwrap_or(BEARING_TOL);
    let (joints, fanned) = find_track_joints(tracks, switches, tol, btol);
    json!({ "joints": joints.iter().map(joint_json).collect::<Vec<_>>(), "fanned": fanned })
}

/// The links a project holds, how well each fits (`existingLinks`).
pub fn existing_links(switches: &[Value], tracks: &[Value]) -> Value {
    let end_of = |id: Option<&Value>, ep: Option<&str>| -> Option<End> {
        let track = tracks.iter().find(|t| strict_eq(t.get("id"), id))?;
        if !has_plane(track) {
            return None;
        }
        track_end_at(Some(track), ep.unwrap_or(""))
    };
    let mut out: Vec<(Option<f64>, Value)> = switches
        .iter()
        .filter(|sw| text(sw, "kind") == Some(LINK_KIND))
        .map(|sw| {
            let a = end_of(port_track(sw, "A"), port_end(sw, "A"));
            let b = end_of(port_track(sw, "B"), port_end(sw, "B"));
            match (a, b) {
                (Some(a), Some(b)) => {
                    let (gap, off) = joint_fit(&a, &b);
                    (
                        Some(gap),
                        json!({ "sw": sw, "a": a.to_json(), "b": b.to_json(), "gap": number(gap), "bearingOff": number(off), "crsChange": a.epsg != b.epsg }),
                    )
                }
                (a, b) => (
                    None,
                    json!({ "sw": sw, "a": a.map(|e| e.to_json()), "b": b.map(|e| e.to_json()), "gap": null, "bearingOff": null, "crsChange": false }),
                ),
            }
        })
        .collect();
    out.sort_by(|x, y| match (x.0, y.0) {
        (None, _) => std::cmp::Ordering::Less,
        (_, None) => std::cmp::Ordering::Greater,
        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal),
    });
    Value::Array(out.into_iter().map(|(_, v)| v).collect())
}

/// A link record for a joint (`linkRecord`).
pub fn link_record(a: &Value, b: &Value, name: Option<&str>) -> Value {
    let mut o = new_switch_fields(LINK_KIND).as_object().cloned().unwrap_or_default();
    if let Some(n) = name.filter(|n| !n.is_empty()) {
        o.insert("name".into(), json!(n));
    }
    o.insert(
        "ports".into(),
        json!({ "A": { "trackId": a["trackId"], "endpoint": a["endpoint"] }, "B": { "trackId": b["trackId"], "endpoint": b["endpoint"] } }),
    );
    Value::Object(o)
}

/// Links for joints, named `link.001`… (`linksForJoints`).
pub fn links_for_joints(joints: &[Value], tracks: &[Value], taken: &[String]) -> Vec<Value> {
    let mut taken: HashSet<String> = taken.iter().cloned().collect();
    let mut n = 0;
    joints
        .iter()
        .map(|j| {
            let name = loop {
                n += 1;
                let candidate = format!("link.{n:03}");
                if !taken.contains(&candidate) {
                    break candidate;
                }
            };
            taken.insert(name.clone());
            link_symbol(&link_record(&j["a"], &j["b"], Some(&name)), tracks)
        })
        .collect()
}

/// Every pair of meeting ends linked (`linkAllJoints`).
pub fn link_all_joints(tracks: &[Value], switches: &[Value], taken: &[String]) -> Value {
    let found = find_track_joints_json(tracks, switches, &Value::Null);
    let joints = found["joints"].as_array().cloned().unwrap_or_default();
    json!({ "links": links_for_joints(&joints, tracks, taken), "joints": joints, "fanned": found["fanned"] })
}

/// A link with the symbol it is drawn with (`linkSymbol`).
pub fn link_symbol(sw: &Value, tracks: &[Value]) -> Value {
    let track = tracks
        .iter()
        .find(|t| strict_eq(t.get("id"), sw.get("ports").and_then(|p| p.get("A")).and_then(|a| a.get("trackId"))));
    let Some(end) = track.and_then(|t| track_end_at(Some(t), port_end(sw, "A").unwrap_or(""))) else {
        return sw.clone();
    };
    let rad = end.outward * std::f64::consts::PI / 180.0;
    let along = [rad.sin(), rad.cos()];
    let across = [along[1], -along[0]];
    let corner = |d: [f64; 2], sign: f64| {
        let (lng, lat) = plane_to_wgs84(
            end.easting + sign * LINK_BODY_HALF * d[0],
            end.northing + sign * LINK_BODY_HALF * d[1],
            end.epsg,
        )
        .unwrap_or((f64::NAN, f64::NAN));
        json!([number(lng), number(lat)])
    };
    let ring = vec![corner(along, 1.0), corner(across, 1.0), corner(along, -1.0), corner(across, -1.0)];
    let mut fill = ring.clone();
    fill.push(ring[0].clone());
    let (lng, lat) = plane_to_wgs84(end.easting, end.northing, end.epsg).unwrap_or((f64::NAN, f64::NAN));
    let mut o: Object = sw.as_object().cloned().unwrap_or_default();
    o.insert("fillCoords".into(), Value::Array(fill));
    o.insert("bodyCentre".into(), json!([number(lng), number(lat)]));
    Value::Object(o)
}

// ── Kilometrage (kmLines.js) ────────────────────────────────────────────────

/// A kilometrage line's runs in a plane as points [easting, northing, km] (`planeRuns`): from either form a
/// project holds a line in (`km_line`), a run of elements sampled.
pub fn plane_runs(km_line: &Value, epsg: i64) -> Vec<Vec<[f64; 3]>> {
    rail_across_geom::km_line::plane_runs(km_line, epsg, rail_across_geom::km_line::DRAW_STEP)
}

/// A kilometrage line's runs in WGS 84 as [lon, lat, km] (`kmLineRuns`): what a map draws, from either form.
pub fn km_line_runs(km_line: &Value) -> Vec<Vec<[f64; 3]>> {
    rail_across_geom::km_line::wgs84_runs(km_line, rail_across_geom::km_line::DRAW_STEP)
}

/// The kilometre nearest a plane point on runs (`kmOnRuns`): (km, offset).
pub fn km_on_runs(runs: &[Vec<[f64; 3]>], point: [f64; 2]) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for run in runs {
        for i in 1..run.len() {
            let [ax, ay, akm] = run[i - 1];
            let [bx, by, bkm] = run[i];
            let (dx, dy) = (bx - ax, by - ay);
            let sq = dx * dx + dy * dy;
            if sq == 0.0 {
                continue;
            }
            let t = (((point[0] - ax) * dx + (point[1] - ay) * dy) / sq).clamp(0.0, 1.0);
            let offset = (point[0] - (ax + t * dx)).hypot(point[1] - (ay + t * dy));
            if best.is_some_and(|b| offset >= b.1) {
                continue;
            }
            best = Some((akm + t * (bkm - akm), offset));
        }
    }
    best
}

/// The kilometre of a point of a track, on its own line where it has one (`kmForTrackPoint`).
pub fn km_for_track_point(km_lines: &[Value], track: &Value, point: [f64; 2]) -> Value {
    let named: Vec<&Value> = km_lines
        .iter()
        .filter(|l| truthy(track.get("lineNumber")) && js_string(l.get("lineNumber")) == js_string(track.get("lineNumber")))
        .collect();
    let candidates: Vec<&Value> = if named.is_empty() { km_lines.iter().collect() } else { named };
    let mut best: Option<(f64, f64, Value)> = None;
    for line in candidates {
        // On a run of elements exactly, on a run of points between them (`km_line::km_at`).
        let Some((km, offset)) = rail_across_geom::km_line::km_at(line, point, epsg(track)) else {
            continue;
        };
        if best.as_ref().is_some_and(|b| offset >= b.1) {
            continue;
        }
        best = Some((km, offset, line.get("lineNumber").cloned().unwrap_or(Value::Null)));
    }
    match best {
        Some((km, offset, line)) => json!({ "km": number(km), "offset": number(offset), "lineNumber": line }),
        None => Value::Null,
    }
}
