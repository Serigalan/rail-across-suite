// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Writing a project as a PlanPro 1.10 plan: its net, its switches and end
//! marks, and its signalling — the objects and attributes `planpro.rs` reads,
//! so that what is written reads back as the project (the tests check it).
//!
//! Every track is an edge (TOP_Kante) along its elements, one GEO_Kante each,
//! in the DB_REF Gauss-Krüger strip the project lies in; a node (TOP_Knoten)
//! is where track ends meet — a turnout's tip, left and right, a buffer stop
//! or boundary, or two tracks joining. A point object stands at `Abstand`
//! along its edge. What is existing is in both states of the plan, what is
//! new only in the target, what goes only in the start.
//!
//! Ids become GUIDs, as PlanPro has them: an id that is one stays, any other
//! is turned into one by name (`planpro_id`), the same every time.
//!
//! Not written, and said so in the report: what lies on no track (a location
//! in the plane), crossings and slips, logical signals after the first, the
//! balises of systems other than PZB, station boundaries, stopping places and
//! itineraries.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use rail_across_geom::coords::{plane_to_wgs84, system_code_at, transform_grid_bearing, transform_plane_point};
use serde_json::{Map, Value, json};

use crate::mapping as m;

const NS_PLANPRO: &str = "http://www.plan-pro.org/modell/PlanPro/1.10.0.3";
const NS_ASPECTS: &str = "http://www.plan-pro.org/modell/Signalbegriffe_Ril_301/1.10.0.3";
const XSD_VERSION: &str = "1.10.0.3";
const GON: f64 = 0.9;

// ── ids ─────────────────────────────────────────────────────────────────────

fn is_guid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

fn fnv(s: &str, basis: u64) -> u64 {
    s.bytes().fold(basis, |h, c| (h ^ u64::from(c)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// The GUID an id is written as: itself in lower case where it is one, else
/// one made from it by name (two FNV-1a hashes, marked as version 8).
pub fn planpro_id(id: &str) -> String {
    if is_guid(id) {
        return id.to_lowercase();
    }
    let hi = fnv(id, 0xcbf2_9ce4_8422_2325);
    let lo = fnv(id, 0x8422_2325_cbf2_9ce4) ^ hi.rotate_left(29);
    let hi = (hi & 0xffff_ffff_ffff_0fff) | 0x0000_0000_0000_8000;
    let lo = (lo & 0x3fff_ffff_ffff_ffff) | 0x8000_0000_0000_0000;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        hi >> 32,
        (hi >> 16) & 0xffff,
        hi & 0xffff,
        lo >> 48,
        lo & 0xffff_ffff_ffff
    )
}

// ── XML ─────────────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// An attribute of PlanPro: `<name><Wert>v</Wert></name>`, nothing without a value.
fn w(name: &str, v: Option<String>) -> String {
    match v {
        Some(v) => format!("<{name}><Wert>{}</Wert></{name}>", esc(&v)),
        None => String::new(),
    }
}

/// An element around what is inside it, nothing when that is nothing.
fn group(name: &str, inner: String) -> String {
    if inner.is_empty() {
        String::new()
    } else {
        format!("<{name}>{inner}</{name}>")
    }
}

fn id_el(id: &str) -> String {
    w("Identitaet", Some(id.to_string()))
}

fn f3(x: f64) -> String {
    format!("{x:.3}")
}

fn f4(x: f64) -> String {
    format!("{x:.4}")
}

fn yes(b: bool) -> String {
    (if b { "true" } else { "false" }).to_string()
}

// ── reading the project ────────────────────────────────────────────────────

fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn num_of(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn point(v: Option<&Value>) -> Option<(f64, f64)> {
    let a = v?.as_array()?;
    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
}

/// The states of the plan a thing is in, from its lifespan (start, target).
fn states_of_lifespan(v: &Value) -> (bool, bool) {
    let l = v.get("lifespan");
    if l.and_then(|l| l.get("from")).is_some_and(|x| !x.is_null()) {
        (false, true)
    } else if l.and_then(|l| l.get("to")).is_some_and(|x| !x.is_null()) {
        (true, false)
    } else {
        (true, true)
    }
}

/// …and from a status of the net (`new`, `removal`).
fn states_of_status(v: &Value) -> (bool, bool) {
    match str_of(v, "status") {
        Some("new") => (false, true),
        Some("removal") => (true, false),
        _ => (true, true),
    }
}

// ── the plan being written ─────────────────────────────────────────────────

struct Plan {
    start: Vec<String>,
    target: Vec<String>,
    counts: BTreeMap<String, usize>,
    notes: Vec<Value>,
}

impl Plan {
    fn put(&mut self, tag: &str, inner: String, (start, target): (bool, bool)) {
        let xml = format!("<{tag}>{inner}</{tag}>");
        if start {
            self.start.push(xml.clone());
        }
        if target {
            self.target.push(xml);
        }
        *self.counts.entry(tag.to_string()).or_default() += 1;
    }

    fn note(&mut self, code: &str, mut params: Map<String, Value>) {
        params.insert("code".into(), json!(code));
        self.notes.push(Value::Object(params));
    }

    fn not_written(&mut self, collection: &str, id: &str, why: &str) {
        let mut p = Map::new();
        p.insert("collection".into(), json!(collection));
        p.insert("id".into(), json!(id));
        p.insert("why".into(), json!(why));
        self.note("not_written", p);
    }
}

/// A track as written: its edge, its length in the plan's plane and how its
/// stations scale into it, its two nodes and the points beside its ends.
struct Edge {
    guid: String,
    length: f64,
    scale: f64,
    /// The first and last point of the track, and the point at the far end of
    /// its first and last element (where a branch is told from a straight route).
    ends: [(f64, f64); 2],
    beside: [(f64, f64); 2],
    nodes: [usize; 2],
}

struct Node {
    guid: String,
    geo: String,
    at: (f64, f64),
    ends: Vec<(String, usize)>,
    /// TOP_Anschluss by end, where it is no plain end or joint.
    conn: HashMap<(String, usize), &'static str>,
}

fn endpoint_index(e: Option<&str>) -> Option<usize> {
    match e {
        Some("BEGIN") => Some(0),
        Some("END") => Some(1),
        _ => None,
    }
}

const ENDPOINTS: [&str; 2] = ["BEGIN", "END"];

/// The DB_REF strip the project is written in: the one its first track lies
/// in, or the strip nearest it.
fn strip_of(tracks: &[Value]) -> Option<i64> {
    for t in tracks {
        let Some(epsg) = num_of(t, "epsg").map(|e| e as i64) else { continue };
        if m::coordinate_system_key(epsg).is_some() {
            return Some(epsg);
        }
        let first = list(t, "elements").first().and_then(|e| point(e.get("startNode")));
        if let Some((e, n)) = first {
            if let Ok((lng, _)) = plane_to_wgs84(e, n, epsg) {
                return system_code_at("DBREF_GK", lng);
            }
        }
    }
    None
}

/// One element of a track as a GEO_Kante between two GEO_Knoten.
#[allow(clippy::too_many_arguments)]
fn geo_edge(plan: &mut Plan, el: &Value, edge: &str, id: &str, a: &str, b: &str, epsg: i64, strip: i64, state: (bool, bool)) -> Result<f64, String> {
    let fail = |e: rail_across_geom::coords::Unsupported| e.0;
    let (s, e) = (point(el.get("startNode")), point(el.get("endNode")));
    let (Some(s), Some(e)) = (s, e) else {
        return Err(format!("planpro: an element of track {edge} has no end points"));
    };
    let sg = transform_plane_point(s.0, s.1, epsg, strip).map_err(fail)?;
    let eg = transform_plane_point(e.0, e.1, epsg, strip).map_err(fail)?;
    let chord = (e.0 - s.0).hypot(e.1 - s.1);
    let scale = if chord > 1e-9 { (eg.0 - sg.0).hypot(eg.1 - sg.1) / chord } else { 1.0 };
    let length = num_of(el, "length").unwrap_or(chord) * scale;
    let bearing = transform_grid_bearing(s.0, s.1, num_of(el, "bearing").unwrap_or(0.0), epsg, strip).map_err(fail)?;
    let gon = ((bearing / GON) % 400.0 + 400.0) % 400.0;
    let kind = num_of(el, "elementType").unwrap_or(0.0) as i64;
    let radius = |k: &str| num_of(el, k).filter(|r| *r != 0.0).map(|r| f3(r * scale));
    let (form, ra, rb) = match kind {
        1 => ("Bogen", radius("radius"), radius("radius")),
        2 => (
            if str_of(el, "transitionType") == Some("bloss") {
                "Blosskurve"
            } else {
                "Klothoide"
            },
            radius("r1"),
            radius("r2"),
        ),
        _ => ("Gerade", None, None),
    };
    let allg = [
        w("GEO_Form", Some(form.into())),
        w("GEO_Laenge", Some(f3(length))),
        w("GEO_Radius_A", ra),
        w("GEO_Radius_B", rb),
        w("GEO_Richtungswinkel", Some(format!("{gon:.7}"))),
    ]
    .concat();
    let inner = [
        id_el(id),
        group("GEO_Kante_Allg", allg),
        w("ID_GEO_Art", Some(edge.into())),
        w("ID_GEO_Knoten_A", Some(a.into())),
        w("ID_GEO_Knoten_B", Some(b.into())),
    ]
    .concat();
    plan.put("GEO_Kante", inner, state);
    Ok(length)
}

fn geo_node(plan: &mut Plan, id: &str, (e, n): (f64, f64), system: &str, state: (bool, bool)) {
    plan.put("GEO_Knoten", id_el(id), state);
    let allg = [w("GEO_Koordinatensystem", Some(system.into())), w("GK_X", Some(f4(e))), w("GK_Y", Some(f4(n)))].concat();
    let inner = [
        id_el(&planpro_id(&format!("point:{id}"))),
        group("GEO_Punkt_Allg", allg),
        w("ID_GEO_Knoten", Some(id.into())),
    ]
    .concat();
    plan.put("GEO_Punkt", inner, state);
}

/// Where a point object stands: `Punkt_Objekt_TOP_Kante`.
fn place(edge: &Edge, station: f64, wirk: Option<&str>, side: Option<&str>, lateral: Option<f64>) -> String {
    group(
        "Punkt_Objekt_TOP_Kante",
        [
            w("Abstand", Some(f3(station * edge.scale))),
            w("ID_TOP_Kante", Some(edge.guid.clone())),
            w("Wirkrichtung", wirk.map(String::from)),
            w("Seitliche_Lage", side.map(String::from)),
            w("Seitlicher_Abstand", lateral.map(f3)),
        ]
        .concat(),
    )
}

/// A part of an edge between two stations: `Bereich_Objekt_Teilbereich`.
fn part(edge: &Edge, from: f64, to: f64) -> String {
    group(
        "Bereich_Objekt_Teilbereich",
        [
            w("Begrenzung_A", Some(f3(from.min(to) * edge.scale))),
            w("Begrenzung_B", Some(f3(from.max(to) * edge.scale))),
            w("ID_TOP_Kante", Some(edge.guid.clone())),
        ]
        .concat(),
    )
}

/// The whole of an edge.
fn whole(edge: &Edge) -> String {
    part(edge, 0.0, edge.length / edge.scale)
}

/// A location of the domain on a written track.
struct At<'a> {
    edge: &'a Edge,
    track: String,
    station: f64,
    side: Option<&'static str>,
    offset: Option<f64>,
    direction: Option<&'static str>,
}

fn at_of<'a>(v: Option<&Value>, edges: &'a HashMap<String, Edge>) -> Option<At<'a>> {
    let v = v?;
    if str_of(v, "anchor").is_some_and(|a| a != "station") {
        return None;
    }
    let track = str_of(v, "trackId")?;
    Some(At {
        edge: edges.get(track)?,
        track: track.to_string(),
        station: num_of(v, "station")?,
        side: str_of(v, "side").and_then(m::side_key),
        offset: num_of(v, "offset"),
        direction: match str_of(v, "direction") {
            Some("up") => Some("in"),
            Some("down") => Some("gegen"),
            _ => None,
        },
    })
}

fn place_at(at: &At, wirk: Option<&str>) -> String {
    place(at.edge, at.station, wirk.or(at.direction), at.side, at.offset)
}

/// The written name of a thing: `Bezeichnung` with its short name and area number.
fn naming(v: &Value) -> String {
    group(
        "Bezeichnung",
        [
            w("Bezeichnung_Lageplan_Kurz", str_of(v, "name").map(String::from)),
            w("Kennzahl", str_of(v, "areaNumber").map(String::from)),
        ]
        .concat(),
    )
}

fn gid(v: &Value) -> String {
    planpro_id(str_of(v, "id").unwrap_or(""))
}

fn gref(v: &Value, k: &str) -> Option<String> {
    str_of(v, k).map(planpro_id)
}

/// A project as a PlanPro plan (`writePlanPro`): `{ text, report: { notes },
/// counts }` — the XML, what could not be written and why, and how many
/// objects of each kind were. `record` holds `tracks`, `switches`,
/// `endMarks`, `title` and `id`; `options`: `signalling` (the domain, else the
/// record's), `now` (the time stamp of the file), `version` (the suite's).
pub fn write_planpro(record: &Value, options: &Value) -> Result<Value, String> {
    let mut plan = Plan {
        start: vec![],
        target: vec![],
        counts: BTreeMap::new(),
        notes: vec![],
    };
    let tracks = list(record, "tracks");
    let strip = strip_of(tracks).ok_or("planpro: the project has no track in a plane")?;
    let system = m::coordinate_system_key(strip).ok_or("planpro: no DB_REF strip for the project")?;

    // ── the net: an edge per track, a GEO_Kante per element ──
    let mut edges: HashMap<String, Edge> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for t in tracks {
        let id = str_of(t, "id").unwrap_or("").to_string();
        let els = list(t, "elements");
        let Some(epsg) = num_of(t, "epsg").map(|e| e as i64) else {
            plan.not_written("tracks", &id, "no_plane");
            continue;
        };
        if els.is_empty() {
            plan.not_written("tracks", &id, "no_elements");
            continue;
        }
        let fail = |e: rail_across_geom::coords::Unsupported| e.0;
        let gk = |p: Option<(f64, f64)>| -> Result<(f64, f64), String> {
            let (e, n) = p.ok_or_else(|| format!("planpro: an element of track {id} has no end points"))?;
            transform_plane_point(e, n, epsg, strip).map_err(fail)
        };
        let first = &els[0];
        let last = &els[els.len() - 1];
        let edge = Edge {
            guid: planpro_id(&id),
            length: 0.0,
            scale: 1.0,
            ends: [gk(point(first.get("startNode")))?, gk(point(last.get("endNode")))?],
            beside: [gk(point(first.get("endNode")))?, gk(point(last.get("startNode")))?],
            nodes: [usize::MAX; 2],
        };
        edges.insert(id.clone(), edge);
        order.push(id);
    }

    // ── nodes: switches, end marks, then where ends meet ──
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_of: HashMap<(String, usize), usize> = HashMap::new();
    let new_node = |nodes: &mut Vec<Node>, id: &str, at: (f64, f64)| {
        let guid = planpro_id(id);
        nodes.push(Node {
            geo: planpro_id(&format!("geo:{guid}")),
            guid,
            at,
            ends: vec![],
            conn: HashMap::new(),
        });
        nodes.len() - 1
    };
    let mut turnouts: Vec<(usize, &Value)> = Vec::new();
    for sw in list(record, "switches") {
        let id = str_of(sw, "switchId").or(str_of(sw, "id")).unwrap_or("");
        let port = |k: &str| {
            let p = sw.get("ports")?.get(k)?;
            let track = str_of(p, "trackId")?;
            let end = endpoint_index(str_of(p, "endpoint"))?;
            edges.contains_key(track).then(|| (track.to_string(), end))
        };
        let kind = str_of(sw, "kind").unwrap_or("turnout");
        let (Some(a), Some(b1), Some(b2)) = (port("A"), port("B1"), port("B2")) else {
            plan.not_written("switches", id, "ports_not_on_tracks");
            continue;
        };
        if kind != "turnout" {
            plan.not_written("switches", id, "kind_not_written");
            continue;
        }
        if [&a, &b1, &b2].iter().any(|e| node_of.contains_key(*e)) {
            plan.not_written("switches", id, "port_taken");
            continue;
        }
        let at = edges[&a.0].ends[a.1];
        let k = new_node(&mut nodes, id, at);
        for e in [&a, &b1, &b2] {
            node_of.insert(e.clone(), k);
            nodes[k].ends.push(e.clone());
        }
        // Which side the branch leaves to, seen from the tip: the far ends of
        // the first elements of both routes against each other.
        let far = |e: &(String, usize)| edges[&e.0].beside[e.1];
        let (s, d) = (far(&b2), far(&b1));
        let cross = (s.0 - at.0) * (d.1 - at.1) - (s.1 - at.1) * (d.0 - at.0);
        let (branch, straight) = if cross > 0.0 { ("Links", "Rechts") } else { ("Rechts", "Links") };
        nodes[k].conn.insert(a.clone(), "Spitze");
        nodes[k].conn.insert(b1.clone(), branch);
        nodes[k].conn.insert(b2.clone(), straight);
        turnouts.push((k, sw));
    }
    let mut buffer_stops: Vec<(String, usize, &Value)> = Vec::new();
    for mark in list(record, "endMarks") {
        let id = str_of(mark, "id").unwrap_or("");
        let at = mark.get("at").unwrap_or(mark);
        let end = str_of(at, "trackId").zip(endpoint_index(str_of(at, "endpoint")));
        let Some((track, end)) = end.filter(|(t, _)| edges.contains_key(*t)) else {
            plan.not_written("endMarks", id, "no_track");
            continue;
        };
        let key = (track.to_string(), end);
        if node_of.contains_key(&key) {
            continue;
        }
        let k = new_node(&mut nodes, id, edges[track].ends[end]);
        node_of.insert(key.clone(), k);
        nodes[k].ends.push(key);
        if str_of(mark, "kind") == Some("buffer_stop") {
            buffer_stops.push((track.to_string(), end, mark));
        }
    }
    // Ends meeting within a millimetre are one node.
    let cell = |(e, n): (f64, f64)| ((e * 1000.0).round() as i64, (n * 1000.0).round() as i64);
    let mut by_cell: HashMap<(i64, i64), usize> = nodes.iter().enumerate().map(|(k, n)| (cell(n.at), k)).collect();
    for id in &order {
        for (end, name) in ENDPOINTS.iter().enumerate() {
            let key = (id.clone(), end);
            if node_of.contains_key(&key) {
                continue;
            }
            let at = edges[id].ends[end];
            let k = match by_cell.get(&cell(at)) {
                Some(&k) => k,
                None => {
                    let k = new_node(&mut nodes, &format!("node:{id}:{name}"), at);
                    by_cell.insert(cell(at), k);
                    k
                }
            };
            node_of.insert(key.clone(), k);
            nodes[k].ends.push(key);
        }
    }
    for (key, &k) in &node_of {
        edges.get_mut(&key.0).unwrap().nodes[key.1] = k;
    }

    // The GEO nodes of the nodes, and the TOP nodes.
    for n in &nodes {
        geo_node(&mut plan, &n.geo, n.at, system, (true, true));
        plan.put("TOP_Knoten", [id_el(&n.guid), w("ID_GEO_Knoten", Some(n.geo.clone()))].concat(), (true, true));
    }

    // The edges with their geometry.
    for t in tracks {
        let id = str_of(t, "id").unwrap_or("");
        let Some(edge_nodes) = edges.get(id).map(|e| e.nodes) else { continue };
        let epsg = num_of(t, "epsg").unwrap_or(0.0) as i64;
        let state = states_of_status(t);
        let guid = planpro_id(id);
        let els = list(t, "elements");
        let mut length = 0.0;
        let mut plain = 0.0;
        for (i, el) in els.iter().enumerate() {
            let a = if i == 0 {
                nodes[edge_nodes[0]].geo.clone()
            } else {
                planpro_id(&format!("geo:{guid}:{i}"))
            };
            let b = if i + 1 == els.len() {
                nodes[edge_nodes[1]].geo.clone()
            } else {
                let b = planpro_id(&format!("geo:{guid}:{}", i + 1));
                let end = point(el.get("endNode")).ok_or_else(|| format!("planpro: an element of track {id} has no end points"))?;
                let at = transform_plane_point(end.0, end.1, epsg, strip).map_err(|e| e.0)?;
                geo_node(&mut plan, &b, at, system, state);
                b
            };
            let g = planpro_id(&format!("geo-edge:{guid}:{i}"));
            length += geo_edge(&mut plan, el, &guid, &g, &a, &b, epsg, strip, state)?;
            plain += num_of(el, "length").unwrap_or(0.0);
        }
        let conn = |end: usize| {
            let n = &nodes[edge_nodes[end]];
            n.conn
                .get(&(id.to_string(), end))
                .copied()
                .unwrap_or(if n.ends.len() == 1 { "Ende" } else { "Verbindung" })
        };
        let allg = [
            w("TOP_Anschluss_A", Some(conn(0).into())),
            w("TOP_Anschluss_B", Some(conn(1).into())),
            w("TOP_Laenge", Some(f3(length))),
        ]
        .concat();
        let inner = [
            id_el(&guid),
            w("ID_TOP_Knoten_A", Some(nodes[edge_nodes[0]].guid.clone())),
            w("ID_TOP_Knoten_B", Some(nodes[edge_nodes[1]].guid.clone())),
            group("TOP_Kante_Allg", allg),
        ]
        .concat();
        plan.put("TOP_Kante", inner, state);
        let e = edges.get_mut(id).unwrap();
        e.length = length;
        e.scale = if plain > 0.0 { length / plain } else { 1.0 };
    }
    let end_station = |e: &Edge, end: usize| if end == 0 { 0.0 } else { e.length / e.scale };

    // Track names and lines.
    let mut lines: BTreeMap<String, Vec<&Edge>> = BTreeMap::new();
    for t in tracks {
        let id = str_of(t, "id").unwrap_or("");
        let Some(edge) = edges.get(id) else { continue };
        if let Some(name) = str_of(t, "trackNumber").or(str_of(t, "name")) {
            let inner = [
                id_el(&planpro_id(&format!("name:{}", edge.guid))),
                whole(edge),
                group("Bezeichnung", w("Bez_Gleis_Bezeichnung", Some(name.into()))),
            ]
            .concat();
            plan.put("Gleis_Bezeichnung", inner, states_of_status(t));
        }
        if let Some(line) = t
            .get("lineNumber")
            .filter(|l| !l.is_null())
            .map(|l| l.as_str().map(String::from).unwrap_or_else(|| l.to_string()))
        {
            lines.entry(line).or_default().push(edge);
        }
    }
    for (line, list) in &lines {
        let parts: String = list.iter().map(|e| whole(e)).collect();
        let inner = [
            id_el(&planpro_id(&format!("line:{line}"))),
            parts,
            group("Bezeichnung", w("Bezeichnung_Strecke", Some(line.clone()))),
        ]
        .concat();
        plan.put("Strecke", inner, (true, true));
    }

    // Buffer stops.
    for (track, end, mark) in &buffer_stops {
        let e = &edges[track];
        let inner = [
            id_el(&planpro_id(&format!("stop:{}", str_of(mark, "id").unwrap_or("")))),
            place(e, end_station(e, *end), Some(if *end == 0 { "gegen" } else { "in" }), None, None),
            w("Gleis_Abschluss_Art", Some("Festprellbock".into())),
        ]
        .concat();
        plan.put("Gleis_Abschluss", inner, (true, true));
    }

    // ── signalling ──
    let empty = json!({});
    let domain = options
        .get("signalling")
        .filter(|d| d.is_object())
        .or(record.get("signalling"))
        .unwrap_or(&empty);
    let col = |k: &str| domain.get("collections").map(|c| list(c, k)).unwrap_or(&[]);

    // Switches and their elements: every turnout has one.
    let mut element_of_switch: HashMap<&str, &Value> = HashMap::new();
    for el in col("switchElements") {
        if let Some(sw) = str_of(el, "switchId") {
            element_of_switch.entry(sw).or_insert(el);
        }
    }
    let mut written_elements: Vec<String> = Vec::new();
    for &(k, sw) in &turnouts {
        let sid = str_of(sw, "switchId").or(str_of(sw, "id")).unwrap_or("");
        let node = &nodes[k];
        let anlage = planpro_id(&format!("installation:{}", node.guid));
        let state = states_of_status(sw);
        let allg = [w("W_Kr_Art", Some("EW".into())), w("W_Kr_Grundform", str_of(sw, "label").map(String::from))].concat();
        plan.put("W_Kr_Anlage", [id_el(&anlage), group("W_Kr_Anlage_Allg", allg)].concat(), state);
        let element = element_of_switch.get(sid).copied();
        let eid = element.map(gid).unwrap_or_else(|| planpro_id(&format!("element:{}", node.guid)));
        let estate = element.map(states_of_lifespan).unwrap_or(state);
        let bez = group(
            "Bezeichnung",
            [
                w("Bezeichnung_Lageplan_Kurz", element.and_then(|e| str_of(e, "name")).map(String::from)),
                w("Bezeichnung_Tabelle", str_of(sw, "name").map(String::from)),
                w("Kennzahl", element.and_then(|e| str_of(e, "areaNumber")).map(String::from)),
            ]
            .concat(),
        );
        let allg = element.map_or(String::new(), |e| {
            [
                w("W_Kr_Gsp_Stellart", str_of(e, "operation").and_then(m::operation_key).map(String::from)),
                w("Vorzugslage_Automatik", e.get("preferredAutomatic").and_then(Value::as_bool).map(yes)),
            ]
            .concat()
        });
        let weiche = element.map_or(String::new(), |e| {
            [
                w("Weiche_Vorzugslage", str_of(e, "preferredPosition").and_then(m::position_key).map(String::from)),
                w("Auffahrortung", e.get("trailable").and_then(Value::as_bool).map(yes)),
                w("ID_Grenzzeichen", gref(e, "clearanceMarkerId")),
            ]
            .concat()
        });
        let inner = [
            id_el(&eid),
            bez,
            w("ID_W_Kr_Anlage", Some(anlage)),
            group("W_Kr_Gsp_Element_Allg", allg),
            format!("<Weiche_Element>{weiche}</Weiche_Element>"),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Element", inner, estate);
        written_elements.push(element.map(|e| str_of(e, "id").unwrap_or("").to_string()).unwrap_or_default());
        // Its component at the tip, and on which side the straight route goes on.
        let (tip, end) = node.ends.iter().find(|e| node.conn.get(*e) == Some(&"Spitze")).cloned().unwrap();
        let te = &edges[&tip];
        let b2 = sw.get("ports").and_then(|p| p.get("B2")).and_then(|p| {
            let key = (str_of(p, "trackId")?.to_string(), endpoint_index(str_of(p, "endpoint"))?);
            node.conn.get(&key).copied()
        });
        let trunk = if b2 == Some("Links") { "links" } else { "rechts" };
        let inner = [
            id_el(&planpro_id(&format!("component:{eid}"))),
            place(te, end_station(te, end), None, None, None),
            w("ID_W_Kr_Gsp_Element", Some(eid.clone())),
            group("Zungenpaar", w("Stammgleis", Some(trunk.into()))),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Komponente", inner, estate);
        if let Some(z) = element.and_then(|e| e.get("twofoldProtection")).filter(|z| z.is_object()) {
            let measure = |k: &str| str_of(z, k).and_then(m::twofold_measure_key).map(String::from);
            let inner = [
                id_el(&planpro_id(&format!("twofold:{eid}"))),
                w("ID_W_Element", Some(eid.clone())),
                w("Zwieschutz_Art", str_of(z, "kind").and_then(m::twofold_kind_key).map(String::from)),
                group(
                    "Fla_Zwieschutz_Element",
                    [
                        w("Massnahme_L", measure("left")),
                        w("Massnahme_R", measure("right")),
                        w("Nachlaufverhinderung", z.get("preventsRunOn").and_then(Value::as_bool).map(yes)),
                    ]
                    .concat(),
                ),
            ]
            .concat();
            plan.put("Fla_Zwieschutz", inner, estate);
        }
    }
    // Derailers, and elements whose switch was not written.
    for e in col("switchElements") {
        let id = str_of(e, "id").unwrap_or("");
        if written_elements.iter().any(|w| w == id) {
            continue;
        }
        let Some(d) = e.get("derailer").filter(|d| d.is_object()) else {
            plan.not_written("switchElements", id, "no_switch");
            continue;
        };
        let Some(at) = at_of(d.get("at"), &edges) else {
            plan.not_written("switchElements", id, "no_track");
            continue;
        };
        let eid = gid(e);
        let state = states_of_lifespan(e);
        let allg = [
            w("W_Kr_Gsp_Stellart", str_of(e, "operation").and_then(m::operation_key).map(String::from)),
            w("Vorzugslage_Automatik", e.get("preferredAutomatic").and_then(Value::as_bool).map(yes)),
        ]
        .concat();
        let inner = [
            id_el(&eid),
            naming(e),
            group("W_Kr_Gsp_Element_Allg", allg),
            format!(
                "<Gleissperre_Element>{}</Gleissperre_Element>",
                w(
                    "Gleissperre_Vorzugslage",
                    str_of(d, "preferredPosition").and_then(m::derailer_position_key).map(String::from)
                )
            ),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Element", inner, state);
        let inner = [
            id_el(&planpro_id(&format!("component:{eid}"))),
            place_at(&at, None),
            w("ID_W_Kr_Gsp_Element", Some(eid)),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Komponente", inner, state);
    }

    // Signals, their frames and aspects; what a route shows is found by aspect.
    let mut aspect_ids: HashMap<(String, String, Option<String>), String> = HashMap::new();
    let mut speeds: HashMap<String, f64> = HashMap::new();
    let mut signal_at: HashMap<String, (String, f64, Option<&'static str>)> = HashMap::new();
    for sig in col("signals") {
        let id = str_of(sig, "id").unwrap_or("").to_string();
        let Some(at) = at_of(sig.get("at"), &edges) else {
            plan.not_written("signals", &id, "no_track");
            continue;
        };
        let guid = gid(sig);
        let state = states_of_lifespan(sig);
        signal_at.insert(id.clone(), (at.track.clone(), at.station, at.direction));
        let logical = list(sig, "logicalSignals");
        let first = logical.first();
        if logical.len() > 1 {
            plan.not_written("logicalSignals", &id, "only_the_first");
        }
        for s in first.map(|l| list(l, "signallingSpeeds")).unwrap_or(&[]) {
            if let (Some(route), Some(v)) = (str_of(s, "routeId"), num_of(s, "speed")) {
                speeds.insert(route.to_string(), v);
            }
        }
        let both = sig.get("bothDirections").and_then(Value::as_bool) == Some(true);
        let mut places = place_at(&at, both.then_some("beide"));
        for also in list(sig, "alsoAt") {
            if let Some(a) = at_of(Some(also), &edges) {
                places.push_str(&place_at(&a, None));
            }
        }
        let real = str_of(sig, "kind") != Some("virtual");
        let body = if real {
            let function = first.and_then(|l| str_of(l, "function")).or(str_of(sig, "function"));
            let system = first.and_then(|l| str_of(l, "system"));
            let kind = first.and_then(|l| str_of(l, "type"));
            let screen = [
                w("Signal_Art", kind.and_then(m::signal_type_key).map(String::from)),
                w("Signalsystem", system.and_then(m::signal_system_key).map(String::from)),
            ]
            .concat();
            let sight = sig.get("sight");
            let sight = |k: &str| sight.and_then(|s| num_of(s, k)).map(|v| format!("{v}"));
            let inner = [
                w("Signal_Befestigungsart", str_of(sig, "mounting").and_then(m::mounting_key).map(String::from)),
                w("Signal_Funktion", function.and_then(m::signal_function_key).map(String::from)),
                group("Signal_Real_Aktiv_Schirm", screen),
                w("Signalsicht_Erreichbar", sight("achievable")),
                w("Signalsicht_Mindest", sight("minimum")),
                w("Signalsicht_Soll", sight("required")),
            ]
            .concat();
            format!("<Signal_Real>{inner}</Signal_Real>")
        } else {
            let f = str_of(sig, "virtualFunction").and_then(m::virtual_function_key).unwrap_or("sonstige");
            format!("<Signal_Fiktiv>{}</Signal_Fiktiv>", w("Fiktives_Signal_Funktion", Some(f.into())))
        };
        plan.put("Signal", [id_el(&guid), naming(sig), places, body].concat(), state);
        if !real {
            continue;
        }
        let shown = first.map(|l| list(l, "frames")).unwrap_or(&[]);
        for (i, frame) in shown.iter().chain(list(sig, "frames")).enumerate() {
            let fid = planpro_id(&format!("frame:{guid}:{i}"));
            let art = str_of(frame, "kind").and_then(m::frame_key).unwrap_or("sonstige");
            plan.put(
                "Signal_Rahmen",
                [id_el(&fid), w("ID_Signal", Some(guid.clone())), w("Rahmen_Art", Some(art.into()))].concat(),
                state,
            );
            for (j, a) in list(frame, "aspects").iter().enumerate() {
                let Some(aspect) = str_of(a, "aspect") else { continue };
                let aid = planpro_id(&format!("aspect:{guid}:{i}:{j}"));
                let symbol = a
                    .get("symbol")
                    .filter(|s| !s.is_null())
                    .map(|s| s.as_str().map(String::from).unwrap_or_else(|| s.to_string()));
                aspect_ids
                    .entry((id.clone(), aspect.to_string(), symbol.clone()))
                    .or_insert_with(|| aid.clone());
                let allg = [
                    w(
                        "Beleuchtet",
                        a.get("lit").and_then(Value::as_bool).map(|b| (if b { "ja" } else { "nein" }).to_string()),
                    ),
                    w("Geschaltet", a.get("switched").and_then(Value::as_bool).map(yes)),
                ]
                .concat();
                let mut sid = format!("<Signalbegriff_ID xsi:type=\"nsSignalbegriffe_Ril_301:{}\">", esc(aspect));
                if let Some(s) = &symbol {
                    let _ = write!(sid, "<Symbol>{}</Symbol>", esc(s));
                }
                sid.push_str("</Signalbegriff_ID>");
                let inner = [
                    id_el(&aid),
                    w("ID_Signal_Rahmen", Some(fid.clone())),
                    sid,
                    group("Signal_Signalbegriff_Allg", allg),
                ]
                .concat();
                plan.put("Signal_Signalbegriff", inner, state);
            }
        }
    }

    // Train detection: the sections each point bounds.
    let mut bounds: HashMap<&str, Vec<String>> = HashMap::new();
    for s in col("trackSections") {
        let id = str_of(s, "id").unwrap_or("");
        for p in list(s, "boundedBy").iter().filter_map(Value::as_str) {
            bounds.entry(p).or_default().push(planpro_id(id));
        }
    }
    for p in col("detectionPoints") {
        let id = str_of(p, "id").unwrap_or("");
        let Some(at) = at_of(p.get("at"), &edges) else {
            plan.not_written("detectionPoints", id, "no_track");
            continue;
        };
        let limits: String = bounds
            .get(id)
            .map(|l| l.iter().map(|s| w("ID_FMAgrenze", Some(s.clone()))).collect())
            .unwrap_or_default();
        let inner = [
            id_el(&gid(p)),
            naming(p),
            place_at(&at, Some("beide")),
            limits,
            w("FMA_Komponente_Art", str_of(p, "kind").and_then(m::detection_point_key).map(String::from)),
        ]
        .concat();
        plan.put("FMA_Komponente", inner, states_of_lifespan(p));
    }
    for s in col("trackSections") {
        let guid = gid(s);
        let state = states_of_lifespan(s);
        let abschnitt = planpro_id(&format!("section:{guid}"));
        plan.put("Gleis_Abschnitt", [id_el(&abschnitt), naming(s)].concat(), state);
        let allg = [
            w("FMA_Art", str_of(s, "kind").and_then(m::section_kind_key).map(String::from)),
            w(
                "FMA_Hilffreimeldung",
                (s.get("auxiliaryClearing").and_then(Value::as_bool) == Some(true)).then(|| yes(true)),
            ),
        ]
        .concat();
        plan.put(
            "FMA_Anlage",
            [id_el(&guid), group("FMA_Anlage_Allg", allg), w("ID_Gleis_Abschnitt", Some(abschnitt))].concat(),
            state,
        );
    }
    let section_refs = |v: &Value, k: &str, tag: &str| -> String { list(v, k).iter().filter_map(Value::as_str).map(|s| w(tag, Some(planpro_id(s)))).collect() };

    // A run along tracks: from where it starts to where it ends.
    let run_parts = |ids: &[Value], from: Option<(&str, f64, Option<&str>)>, to: Option<(&str, f64)>| -> String {
        let ids: Vec<&str> = ids.iter().filter_map(Value::as_str).filter(|t| edges.contains_key(*t)).collect();
        let n = ids.len();
        ids.iter()
            .enumerate()
            .map(|(i, t)| {
                let e = &edges[*t];
                let full = (0.0, e.length / e.scale);
                let (mut a, mut b) = full;
                if i == 0 {
                    if let Some((_, s, dir)) = from.filter(|f| f.0 == *t) {
                        (a, b) = if dir == Some("gegen") { (0.0, s) } else { (s, full.1) };
                    }
                }
                if i + 1 == n {
                    if let Some((_, s)) = to.filter(|x| x.0 == *t) {
                        if n == 1 && from.is_some_and(|f| f.0 == *t) {
                            let s0 = from.unwrap().1;
                            (a, b) = (s0.min(s), s0.max(s));
                        } else if (s - a).abs() <= (b - s).abs() {
                            a = s;
                        } else {
                            b = s;
                        }
                    }
                }
                part(e, a, b)
            })
            .collect()
    };
    let start_of = |sig: Option<&str>| sig.and_then(|s| signal_at.get(s)).map(|(t, s, d)| (t.as_str(), *s, *d));

    // Overlaps: a run from the route's target signal to a marked point.
    let mut overlaps_written: Vec<&str> = Vec::new();
    for o in col("overlaps") {
        let id = str_of(o, "id").unwrap_or("");
        let end = at_of(o.get("end"), &edges);
        let path = o.get("path").map(|p| list(p, "trackIds")).unwrap_or(&[]);
        let signal = str_of(o, "signalId");
        let (Some(end), Some(start)) = (end, start_of(signal)) else {
            plan.not_written("overlaps", id, "no_run");
            continue;
        };
        if path.is_empty() {
            plan.not_written("overlaps", id, "no_run");
            continue;
        }
        overlaps_written.push(id);
        let guid = gid(o);
        let state = states_of_lifespan(o);
        let point_id = planpro_id(&format!("danger-point:{guid}"));
        let mark = planpro_id(&format!("mark:{guid}"));
        plan.put("Sonstiger_Punkt", [id_el(&point_id), place_at(&end, None)].concat(), state);
        plan.put("Markanter_Punkt", [id_el(&mark), w("ID_Markante_Stelle", Some(point_id))].concat(), state);
        let run = planpro_id(&format!("run:{guid}"));
        let inner = [
            id_el(&run),
            run_parts(path, Some(start), Some((end.track.as_str(), end.station))),
            w("ID_Start", signal.map(planpro_id)),
            w("ID_Ziel", Some(mark.clone())),
        ]
        .concat();
        plan.put("Fstr_Fahrweg", inner, state);
        let allg = [
            w("Laenge_Soll", num_of(o, "length").map(|v| format!("{v}"))),
            w("Massgebende_Neigung", num_of(o, "gradient").map(|v| format!("{v:.2}"))),
        ]
        .concat();
        let specific = [
            w("Aufloesung_Verzoegerung", num_of(o, "releaseDelay").map(|v| format!("{v}"))),
            w("DWeg_V", num_of(o, "speed").map(|v| format!("{v}"))),
        ]
        .concat();
        let inner = [
            id_el(&guid),
            group("Bezeichnung", w("Bezeichnung_Fstr_DWeg", str_of(o, "name").map(String::from))),
            group("Fstr_DWeg_Allg", allg),
            group("Fstr_DWeg_Spezifisch", specific),
            section_refs(o, "clearSections", "ID_FMA_Anlage_Freimeldung"),
            w("ID_Fstr_Fahrweg", Some(run)),
            w("ID_PZB_Gefahrpunkt", Some(mark)),
        ]
        .concat();
        plan.put("Fstr_DWeg", inner, state);
        for (k, s) in list(o, "switches").iter().enumerate() {
            let inner = [
                id_el(&planpro_id(&format!("overlap-switch:{guid}:{k}"))),
                w("Element_Verschluss", Some(yes(s.get("locked").and_then(Value::as_bool) == Some(true)))),
                w("ID_Fstr_DWeg", Some(guid.clone())),
                w("ID_W_Kr_Gsp_Element", gref(s, "switchElementId")),
            ]
            .concat();
            plan.put("Fstr_DWeg_W_Kr", inner, state);
        }
    }

    // Routes, and what each shows.
    for r in col("routes") {
        let id = str_of(r, "id").unwrap_or("");
        let path = r.get("path").map(|p| list(p, "trackIds")).unwrap_or(&[]);
        let (start, target) = (str_of(r, "startSignalId"), str_of(r, "targetSignalId"));
        let (Some(from), Some(to)) = (start_of(start), start_of(target)) else {
            plan.not_written("routes", id, "no_signals");
            continue;
        };
        let guid = gid(r);
        let state = states_of_lifespan(r);
        let run = planpro_id(&format!("run:{guid}"));
        let inner = [
            id_el(&run),
            run_parts(path, Some(from), Some((to.0, to.1))),
            w("ID_Start", start.map(planpro_id)),
            w("ID_Ziel", target.map(planpro_id)),
            w("Fstr_V_Hg", speeds.get(id).map(|v| format!("{v}"))),
        ]
        .concat();
        plan.put("Fstr_Fahrweg", inner, state);
        let kind = str_of(r, "kind").unwrap_or("train");
        let body = match kind {
            "shunting" => format!(
                "<Fstr_Rangier>{}</Fstr_Rangier>",
                [
                    w("Fstr_Rangier_Art", str_of(r, "shuntingType").map(String::from)),
                    section_refs(r, "clearSections", "ID_FMA_Anlage_Rangier_Frei"),
                ]
                .concat()
            ),
            "middle" => format!("<Fstr_Mittel>{}</Fstr_Mittel>", w("Fstr_Mittel_Art", str_of(r, "middleType").map(String::from))),
            _ => {
                let ovl: String = list(r, "overlaps")
                    .iter()
                    .filter(|x| str_of(x, "overlapId").is_some_and(|o| overlaps_written.contains(&o)))
                    .map(|x| {
                        group(
                            "Fstr_Zug_DWeg",
                            [
                                w("DWeg_Vorzug", x.get("preferred").and_then(Value::as_bool).map(yes)),
                                w("ID_Fstr_DWeg", gref(x, "overlapId")),
                            ]
                            .concat(),
                        )
                    })
                    .collect();
                format!(
                    "<Fstr_Zug>{}</Fstr_Zug>",
                    [
                        w(
                            "Automatische_Einstellung",
                            (r.get("autoSetting").and_then(Value::as_bool) == Some(true)).then(|| "ZL".into())
                        ),
                        w(
                            "Fstr_Vsigabstand_Verkuerzt",
                            (r.get("shortDistantSpacing").and_then(Value::as_bool) == Some(true)).then(|| yes(true))
                        ),
                        w("Fstr_Zug_Art", str_of(r, "trainType").map(String::from)),
                        ovl,
                    ]
                    .concat()
                )
            }
        };
        let inner = [
            id_el(&guid),
            group("Fstr_Zug_Rangier_Allg", w("Fstr_Reihenfolge", num_of(r, "rank").map(|v| format!("{v}")))),
            w("ID_Fstr_Fahrweg", Some(run)),
            body,
        ]
        .concat();
        plan.put("Fstr_Zug_Rangier", inner, state);
        for (k, a) in list(r, "aspects").iter().enumerate() {
            let symbol = a
                .get("symbol")
                .filter(|s| !s.is_null())
                .map(|s| s.as_str().map(String::from).unwrap_or_else(|| s.to_string()));
            let key = (
                str_of(a, "signalId").unwrap_or("").to_string(),
                str_of(a, "aspect").unwrap_or("").to_string(),
                symbol,
            );
            let Some(aid) = aspect_ids.get(&key) else {
                plan.not_written("routes.aspects", id, "aspect_not_on_signal");
                continue;
            };
            let inner = [
                id_el(&planpro_id(&format!("shows:{guid}:{k}"))),
                w("ID_Fstr_Zug_Rangier", Some(guid.clone())),
                w("ID_Signal_Signalbegriff", Some(aid.clone())),
            ]
            .concat();
            plan.put("Fstr_Signalisierung", inner, state);
        }
    }

    // Flank protection.
    for f in col("flankProtection") {
        let id = str_of(f, "id").unwrap_or("");
        let guid = gid(f);
        let state = states_of_lifespan(f);
        let req = f.get("requester").unwrap_or(&Value::Null);
        let p = f.get("protection").unwrap_or(&Value::Null);
        let protection = if let Some(sig) = gref(p, "signalId") {
            group(
                "Fla_Schutz_Signal",
                [
                    w("Fla_Signal_Zielsperrung", p.get("targetLock").and_then(Value::as_bool).map(yes)),
                    w("ID_Fla_Signal", Some(sig)),
                ]
                .concat(),
            )
        } else if let Some(el) = gref(p, "switchElementId") {
            group(
                "Fla_Schutz_W_Gsp",
                [
                    w("Fla_W_Lage", str_of(p, "position").and_then(m::position_key).map(String::from)),
                    w("ID_Fla_W_Gsp_Element", Some(el)),
                ]
                .concat(),
            )
        } else if p.get("waived").and_then(Value::as_bool) == Some(true) {
            w("Fla_Verzicht", Some(yes(true)))
        } else {
            plan.not_written("flankProtection", id, "no_protection");
            continue;
        };
        let pass = f.get("passOn").unwrap_or(&Value::Null);
        let inner = [
            id_el(&guid),
            group(
                "Fla_Schutz_Anforderer",
                [
                    w("Fahrt_Ueber", str_of(req, "via").and_then(m::via_key).map(String::from)),
                    w("ID_Anforderer_Element", gref(req, "switchElementId")),
                ]
                .concat(),
            ),
            protection,
            group(
                "Fla_Schutz_Weitergabe",
                [w("ID_Fla_Weitergabe_L", gref(pass, "left")), w("ID_Fla_Weitergabe_R", gref(pass, "right"))].concat(),
            ),
        ]
        .concat();
        plan.put("Fla_Schutz", inner, state);
        for (k, s) in list(f, "clearSections").iter().filter_map(Value::as_str).enumerate() {
            let inner = [
                id_el(&planpro_id(&format!("flank-clear:{guid}:{k}"))),
                w("ID_FMA_Anlage", Some(planpro_id(s))),
                w("ID_Fla_Schutz", Some(guid.clone())),
            ]
            .concat();
            plan.put("Fla_Freimelde_Zuordnung", inner, state);
        }
    }

    // PZB: a balise of the system pzb, its assignment to signal and routes.
    for b in col("balises") {
        let id = str_of(b, "id").unwrap_or("");
        if str_of(b, "system") != Some("pzb") {
            plan.not_written("balises", id, "system_not_written");
            continue;
        }
        let Some(at) = at_of(b.get("at"), &edges) else {
            plan.not_written("balises", id, "no_track");
            continue;
        };
        let guid = gid(b);
        let state = states_of_lifespan(b);
        let z = planpro_id(&format!("assignment:{guid}"));
        let settings = b.get("settings").unwrap_or(&Value::Null);
        let gue = (str_of(b, "kind") == Some("speedCheck")).then(|| {
            format!(
                "<PZB_Element_GUE>{}</PZB_Element_GUE>",
                [
                    w(
                        "GUE_Anordnung",
                        str_of(settings, "arrangement").and_then(m::gue_arrangement_key).map(String::from)
                    ),
                    w("GUE_Messstrecke", num_of(settings, "measuringLength").map(|v| format!("{v}"))),
                    w("Pruefgeschwindigkeit", num_of(settings, "testSpeed").map(|v| format!("{v}"))),
                ]
                .concat()
            )
        });
        let ina = settings.get("ina").and_then(Value::as_bool) == Some(true);
        let inner = [
            id_el(&guid),
            place_at(&at, None),
            w("ID_PZB_Element_Zuordnung", Some(z.clone())),
            w("PZB_Art", str_of(settings, "frequency").and_then(m::pzb_frequency_key).map(String::from)),
            gue.unwrap_or_else(|| group("PZB_Element_GM", w("PZB_INA", Some(yes(ina))))),
        ]
        .concat();
        plan.put("PZB_Element", inner, state);
        let routes: String = list(b, "routes")
            .iter()
            .map(|r| {
                group(
                    "PZB_Element_Zuordnung_Fstr",
                    [
                        w("ID_Fstr_Zug_Rangier", gref(r, "routeId")),
                        w("Wirksamkeit_Fstr", str_of(r, "effect").and_then(m::pzb_route_effect_key).map(String::from)),
                    ]
                    .concat(),
                )
            })
            .collect();
        let bp = group(
            "PZB_Element_Zuordnung_BP",
            [
                w("ID_PZB_Element_Bezugspunkt", gref(b, "signalId")),
                w("Wirksamkeit", str_of(b, "effect").and_then(m::pzb_effect_key).map(String::from)),
            ]
            .concat(),
        );
        plan.put("PZB_Element_Zuordnung", [id_el(&z), bp, routes].concat(), state);
    }

    for k in ["stationBoundaries", "stoppingPlaces", "itineraries"] {
        for v in col(k) {
            plan.not_written(k, str_of(v, "id").unwrap_or(""), "kind_not_written");
        }
    }

    // ── the file ──
    let title = str_of(record, "title").map(String::from);
    let tool = [
        w("Erzeugung_Zeitstempel", str_of(options, "now").map(String::from)),
        w("Werkzeug_Name", Some("Rail-Across-Suite".into())),
        w("Werkzeug_Version", str_of(options, "version").map(String::from)),
    ]
    .concat();
    let project = [
        group("Planung_P_Allg", w("Bezeichnung_Planung_Projekt", title.clone())),
        group(
            "LST_Planung_Gruppe",
            [
                group("Planung_G_Allg", w("PlanPro_XSD_Version", Some(XSD_VERSION.into()))),
                group("Planung_G_Schriftfeld", w("Bezeichnung_Anlage", title)),
            ]
            .concat(),
        ),
    ]
    .concat();
    let mut text = String::new();
    let _ = writeln!(text, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        text,
        "<nsPlanPro:PlanPro_Schnittstelle xmlns:nsPlanPro=\"{NS_PLANPRO}\" xmlns:nsSignalbegriffe_Ril_301=\"{NS_ASPECTS}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
    );
    let _ = writeln!(text, "{}", id_el(&planpro_id(&format!("plan:{}", str_of(record, "id").unwrap_or("")))));
    let _ = writeln!(text, "<PlanPro_Schnittstelle_Allg>{tool}</PlanPro_Schnittstelle_Allg>");
    let _ = writeln!(text, "<LST_Planung><Fachdaten><Ausgabe_Fachdaten>");
    for (name, objects) in [("LST_Zustand_Start", &plan.start), ("LST_Zustand_Ziel", &plan.target)] {
        let _ = writeln!(text, "<{name}><Container>");
        for o in objects {
            let _ = writeln!(text, "{o}");
        }
        let _ = writeln!(text, "</Container></{name}>");
    }
    let _ = writeln!(text, "</Ausgabe_Fachdaten></Fachdaten>");
    let _ = writeln!(
        text,
        "<Objektmanagement><LST_Planung_Projekt>{project}</LST_Planung_Projekt></Objektmanagement>"
    );
    let _ = writeln!(text, "</LST_Planung>");
    let _ = writeln!(text, "</nsPlanPro:PlanPro_Schnittstelle>");
    Ok(json!({
        "text": text,
        "report": { "notes": plan.notes },
        "counts": plan.counts,
    }))
}
