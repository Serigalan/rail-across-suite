// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reading a PlanPro 1.10 plan (`planpro.js`, decision 0009): the net it is
//! drawn on and its signalling, as the suite's record holds them. The suite
//! has no PlanPro import any more — too little of a project is in a plan; the
//! reader stays as the check of the export (`write.rs`, tests/export.rs).
//!
//! Every edge (TOP_Kante) becomes a track along its GEO_Kanten in the edge's
//! direction, every node of tip, left and right a turnout; a point object at
//! `Abstand` along an edge stands at that station on its track. What is in
//! both states of the plan is existing, what only the target has is new, what
//! only the start has is removed.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use rail_across_geom::element::{Utm, reverse_element};
use rail_across_geom::factory::{arc_element, straight_element, transition_element};
use rail_across_js::{Object, cmp_utf16, js_hypot, js_number, js_round, js_string, number};
use serde_json::{Map, Value, json};

use crate::mapping as m;
use crate::xml::{El, child, children_named, value};

/// A map that keeps its keys in the order they came, as a JavaScript Map.
struct OMap<K, V> {
    keys: Vec<K>,
    vals: Vec<V>,
    idx: HashMap<K, usize>,
}

impl<K: Hash + Eq + Clone, V> OMap<K, V> {
    fn new() -> Self {
        OMap {
            keys: vec![],
            vals: vec![],
            idx: HashMap::new(),
        }
    }
    fn get(&self, k: &K) -> Option<&V> {
        self.idx.get(k).map(|&i| &self.vals[i])
    }
    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.idx.get(k).map(|&i| &mut self.vals[i])
    }
    fn has(&self, k: &K) -> bool {
        self.idx.contains_key(k)
    }
    fn set(&mut self, k: K, v: V) {
        match self.idx.get(&k) {
            Some(&i) => self.vals[i] = v,
            None => {
                self.idx.insert(k.clone(), self.keys.len());
                self.keys.push(k);
                self.vals.push(v);
            }
        }
    }
    fn entry(&mut self, k: K, make: impl FnOnce() -> V) -> &mut V {
        if !self.has(&k) {
            self.set(k.clone(), make());
        }
        self.get_mut(&k).unwrap()
    }
    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.vals.iter())
    }
}

fn lower(s: Option<String>) -> Option<String> {
    s.map(|s| s.to_lowercase())
}

fn num(s: Option<String>) -> Option<f64> {
    let s = s.filter(|s| !s.is_empty())?;
    let n = js_number(Some(&Value::String(s.replacen(',', ".", 1))));
    n.is_finite().then_some(n)
}

fn boolean(s: Option<String>) -> Option<bool> {
    match s.as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

/// A string as JavaScript tests it: there and not empty.
fn truthy(s: &Option<String>) -> bool {
    s.as_ref().is_some_and(|s| !s.is_empty())
}

fn val(el: &El, path: &str) -> Option<String> {
    value(Some(el), path)
}

/// An object of the pairs that have a value (`withoutUndefined`).
fn obj(pairs: Vec<(&str, Option<Value>)>) -> Map<String, Value> {
    let mut o = Map::new();
    for (k, v) in pairs {
        if let Some(v) = v {
            o.insert(k.to_string(), v);
        }
    }
    o
}

fn s(v: &str) -> Option<Value> {
    Some(json!(v))
}

fn os(v: &Option<String>) -> Option<Value> {
    v.as_ref().map(|v| json!(v))
}

fn on(v: Option<f64>) -> Option<Value> {
    v.map(number)
}

/// An array or object where it has something (`nonEmpty`).
fn non_empty(v: Value) -> Option<Value> {
    match &v {
        Value::Array(a) if !a.is_empty() => Some(v),
        Value::Object(o) if !o.is_empty() => Some(v),
        _ => None,
    }
}

fn arr(v: Vec<String>) -> Value {
    Value::Array(v.into_iter().map(Value::String).collect())
}

/// One object of a plan: its tag, its element (the target state's where it
/// has one) and the states it is in.
pub struct Obj<'a> {
    pub tag: String,
    pub el: &'a El,
    pub start: bool,
    pub target: bool,
}

/// The objects of a plan by id (in lower case) and its header (`readPlanPro`).
pub struct Model<'a> {
    pub header: Value,
    objects: OMap<String, Obj<'a>>,
}

impl<'a> Model<'a> {
    pub fn objects(&self) -> impl Iterator<Item = (&String, &Obj<'a>)> {
        self.objects.iter()
    }

    fn get(&self, id: Option<&str>) -> Option<&Obj<'a>> {
        self.objects.get(&id?.to_lowercase())
    }

    pub fn to_json(&self) -> Value {
        let mut objects = Map::new();
        for (id, o) in self.objects.iter() {
            objects.insert(id.clone(), json!({ "tag": o.tag, "el": o.el.to_json(), "start": o.start, "target": o.target }));
        }
        json!({ "header": self.header, "objects": objects })
    }
}

/// The objects of a plan (`readPlanPro`), from its document element.
pub fn read_planpro(root: &El) -> Result<Model<'_>, String> {
    if !root.name.ends_with("PlanPro_Schnittstelle") {
        return Err("planpro: not a PlanPro interface file".into());
    }
    let mut objects: OMap<String, Obj> = OMap::new();
    let planung = child(Some(root), "LST_Planung");
    for f in children_named(child(planung, "Fachdaten"), "Ausgabe_Fachdaten") {
        for (state, name) in [("start", "LST_Zustand_Start"), ("target", "LST_Zustand_Ziel")] {
            for c in children_named(child(Some(f), name), "Container") {
                for el in &c.children {
                    let Some(id) = lower(val(el, "Identitaet")).filter(|id| !id.is_empty()) else {
                        continue;
                    };
                    let o = objects.entry(id, || Obj {
                        tag: el.name.clone(),
                        el,
                        start: false,
                        target: false,
                    });
                    if state == "start" {
                        o.start = true;
                    } else {
                        o.target = true;
                        o.el = el;
                    }
                }
            }
        }
    }
    let project = child(child(planung, "Objektmanagement"), "LST_Planung_Projekt");
    let group = child(project, "LST_Planung_Gruppe");
    let allg = child(Some(root), "PlanPro_Schnittstelle_Allg");
    let tool = [value(allg, "Werkzeug_Name"), value(allg, "Werkzeug_Version")]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let header = obj(vec![
        ("title", os(&value(child(project, "Planung_P_Allg"), "Bezeichnung_Planung_Projekt"))),
        ("projectNumber", os(&value(child(project, "Planung_P_Allg"), "Projekt_Nummer"))),
        ("installation", os(&value(child(group, "Planung_G_Schriftfeld"), "Bezeichnung_Anlage"))),
        ("subInstallation", os(&value(child(group, "Planung_G_Schriftfeld"), "Bezeichnung_Unteranlage"))),
        ("xsdVersion", os(&value(child(group, "Planung_G_Allg"), "PlanPro_XSD_Version"))),
        ("tool", (!tool.is_empty()).then(|| json!(tool))),
        ("createdAt", os(&value(allg, "Erzeugung_Zeitstempel"))),
    ]);
    Ok(Model {
        header: Value::Object(header),
        objects,
    })
}

/// What the import of a plan read, left out, and could not place.
struct Report {
    notes: Vec<Value>,
    skipped: OMap<String, usize>,
}

impl Report {
    fn note(&mut self, code: &str, params: Vec<(&str, Option<Value>)>) {
        let mut o = obj(params);
        o.insert("code".into(), json!(code));
        self.notes.push(Value::Object(o));
    }
    fn missed(&mut self, collection: &str, id: &str, why: &str) {
        self.note("not_placed", vec![("collection", s(collection)), ("id", s(id)), ("why", s(why))]);
    }
    fn summary(&self) -> Value {
        let mut counts: Vec<(&String, &usize)> = self.skipped.iter().collect();
        counts.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        let mut skipped = Map::new();
        for (tag, n) in counts {
            skipped.insert(tag.clone(), json!(n));
        }
        json!({ "notes": self.notes, "skipped": skipped })
    }
}

/// Where a point object stands (PlanPro Punkt_Objekt_TOP_Kante).
#[derive(Clone, Debug)]
struct Place {
    edge: Option<String>,
    abstand: Option<f64>,
    wirk: Option<String>,
    side: Option<String>,
    lateral: Option<f64>,
}

/// The places a point object stands at, in their order (`placesOf`).
fn places_of(el: &El) -> Vec<Place> {
    children_named(Some(el), "Punkt_Objekt_TOP_Kante")
        .into_iter()
        .map(|p| Place {
            edge: lower(val(p, "ID_TOP_Kante")),
            abstand: num(val(p, "Abstand")),
            wirk: val(p, "Wirkrichtung"),
            side: val(p, "Seitliche_Lage"),
            lateral: num(val(p, "Seitlicher_Abstand")),
        })
        .collect()
}

struct Edge<'a> {
    o: &'a Obj<'a>,
    a: Option<String>,
    b: Option<String>,
    conn_a: Option<String>,
    conn_b: Option<String>,
    length: Option<f64>,
}

#[derive(Clone)]
struct End {
    edge: String,
    endpoint: &'static str,
    conn: Option<String>,
}

struct Comp {
    element: Option<String>,
    place: Option<Place>,
    /// The node it stands at: none for null, `Some(None)` for a node the edge does not name.
    node: Option<Option<String>>,
    trunk: Option<String>,
}

struct Frame {
    signal: Option<String>,
    kind: &'static str,
    aspects: Vec<Value>,
}

struct Run {
    start: Option<String>,
    target: Option<String>,
    speed: Option<f64>,
    track_ids: Vec<String>,
}

const GON: f64 = 0.9;

/// One GEO_Kante as an element of the core, in its own direction (node A to B).
fn geo_element(g: &El, points: &HashMap<String, Utm>, report: &mut Report) -> Result<Option<Value>, String> {
    let point = |path: &str| lower(val(g, path)).and_then(|n| points.get(&n).copied());
    let (Some(a), Some(b)) = (point("ID_GEO_Knoten_A"), point("ID_GEO_Knoten_B")) else {
        return Ok(None);
    };
    let form_name = val(g, "GEO_Kante_Allg/GEO_Form");
    let form = form_name.as_deref().and_then(m::geo_form);
    if form.is_none() {
        report.note("geo_form_as_straight", vec![("form", os(&form_name))]);
    }
    let (kind, transition) = form.unwrap_or((0, None));
    let length = num(val(g, "GEO_Kante_Allg/GEO_Laenge"));
    let bearing = num(val(g, "GEO_Kante_Allg/GEO_Richtungswinkel")).unwrap_or(0.0) * GON;
    let radius = |path: &str| num(val(g, path)).filter(|r| *r != 0.0);
    let (ra, rb) = (radius("GEO_Kante_Allg/GEO_Radius_A"), radius("GEO_Kante_Allg/GEO_Radius_B"));
    let gap = |x: f64, y: f64| (((x - y + 540.0) % 360.0) - 180.0).abs();
    let fail = |e: rail_across_geom::coords::Unsupported| e.0;
    let none = Object::new();
    let length = match length {
        Some(l) if kind != 0 && l > 0.0 => l,
        _ => return straight_element(a, b, &none).map(Some).map_err(fail),
    };
    if kind == 1 {
        if let Some(ra) = ra {
            let right = arc_element(a, b, ra.abs(), &none).map_err(fail)?;
            let left = arc_element(a, b, -ra.abs(), &none).map_err(fail)?;
            let bearing_of = |e: &Value| e["bearing"].as_f64().unwrap_or(f64::NAN);
            return Ok(Some(if gap(bearing_of(&right), bearing) <= gap(bearing_of(&left), bearing) {
                right
            } else {
                left
            }));
        }
    }
    // A transition: the signs that bring it, from its bearing, nearest its end point.
    let tt = json!(transition.unwrap_or("clothoid"));
    let mut best: Option<(Option<f64>, Option<f64>, f64)> = None;
    for (sa, sb) in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        let r1 = ra.map(|r| sa * r.abs());
        let r2 = rb.map(|r| sb * r.abs());
        let (_, end, _) = transition_element(a, bearing, length, r1, r2, &tt, None, &none).map_err(fail)?;
        let miss = js_hypot(&[end.e - b.e, end.n - b.n]);
        if best.is_none_or(|(_, _, m)| miss < m) {
            best = Some((r1, r2, miss));
        }
        if (ra.is_none() || rb.is_none()) && sa == -1.0 {
            break;
        }
    }
    let (r1, r2, miss) = best.unwrap();
    if miss > 0.05 {
        report.note(
            "geo_transition_misses_end",
            vec![
                ("id", os(&lower(val(g, "Identitaet")))),
                ("miss", Some(number(js_round(miss * 1000.0) / 1000.0))),
            ],
        );
    }
    let (el, _, _) = transition_element(a, bearing, length, r1, r2, &tt, Some(b), &none).map_err(fail)?;
    Ok(Some(el))
}

/// The edges of a run in the order a train runs them (`inRunningOrder`).
fn in_running_order(parts: Vec<String>, first: Option<&str>, edges: &OMap<String, Edge>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for p in parts {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    let Some(first) = first.filter(|f| unique.len() >= 2 && unique.iter().any(|u| u == f)) else {
        return unique;
    };
    let mut out = vec![first.to_string()];
    let mut left: Vec<String> = unique.iter().filter(|u| *u != first).cloned().collect();
    let mut open: Option<String> = None;
    while !left.is_empty() {
        let last = edges.get(out.last().unwrap()).unwrap();
        let nodes: Vec<Option<String>> = if truthy(&open) {
            vec![open.clone()]
        } else {
            vec![last.a.clone(), last.b.clone()]
        };
        let Some(pos) = left.iter().position(|id| {
            let e = edges.get(id).unwrap();
            nodes.contains(&e.a) || nodes.contains(&e.b)
        }) else {
            return unique;
        };
        let next = left.remove(pos);
        let e = edges.get(&next).unwrap();
        let via = if nodes.contains(&e.a) { e.a.clone() } else { e.b.clone() };
        open = if via == e.a { e.b.clone() } else { e.a.clone() };
        out.push(next);
    }
    out
}

fn logical_signals_of(system: Option<&str>, kind: Option<&str>) -> Option<Value> {
    if let Some(system) = system {
        return Some(json!([obj(vec![("system", s(system)), ("type", kind.map(|k| json!(k)))])]));
    }
    if kind == Some("shunting") {
        return Some(json!([{ "system": "sh", "type": "shunting" }]));
    }
    None
}

/// A signal as the domain holds it: function, screen and indicators its
/// logical signal's, with the speeds it signals; its boards its own (`byLogicalSignal`).
fn by_logical_signal(signal: Map<String, Value>, speeds: Option<&Vec<Value>>) -> Value {
    let Some(logical) = signal.get("logicalSignals").and_then(Value::as_array).filter(|l| !l.is_empty()).cloned() else {
        return Value::Object(signal);
    };
    let mut place = signal;
    let function = place.remove("function");
    let frames = place.remove("frames").and_then(|f| f.as_array().cloned()).unwrap_or_default();
    place.remove("logicalSignals");
    let is_shown = |f: &Value| matches!(f["kind"].as_str(), Some("screen") | Some("indicator"));
    let shown: Vec<Value> = frames.iter().filter(|f| is_shown(f)).cloned().collect();
    let own: Vec<Value> = frames.iter().filter(|f| !is_shown(f)).cloned().collect();
    let mut first = logical[0].as_object().cloned().unwrap_or_default();
    let mut put = |k: &str, v: Option<Value>| match v {
        Some(v) => {
            first.insert(k.into(), v);
        }
        None => {
            first.remove(k);
        }
    };
    put("function", function);
    put("frames", non_empty(Value::Array(shown)));
    put("signallingSpeeds", speeds.and_then(|s| non_empty(Value::Array(s.clone()))));
    let mut list = vec![Value::Object(first)];
    list.extend(logical[1..].iter().cloned());
    place.insert("frames".into(), Value::Array(own));
    place.insert("logicalSignals".into(), Value::Array(list));
    Value::Object(place)
}

const READ: [&str; 28] = [
    "Signal",
    "Signal_Rahmen",
    "Signal_Signalbegriff",
    "FMA_Komponente",
    "FMA_Anlage",
    "Gleis_Abschnitt",
    "W_Kr_Gsp_Element",
    "W_Kr_Gsp_Komponente",
    "W_Kr_Anlage",
    "Fstr_Zug_Rangier",
    "Fstr_Fahrweg",
    "Fstr_DWeg",
    "Fstr_DWeg_W_Kr",
    "Fstr_Signalisierung",
    "Fla_Schutz",
    "Fla_Freimelde_Zuordnung",
    "Fla_Zwieschutz",
    "PZB_Element",
    "PZB_Element_Zuordnung",
    "TOP_Kante",
    "TOP_Knoten",
    "GEO_Kante",
    "GEO_Knoten",
    "GEO_Punkt",
    "Gleis_Abschluss",
    "Gleis_Bezeichnung",
    "Strecke",
    "Markanter_Punkt",
];

const TRAIN_TYPES: [&str; 8] = ["B", "ZH", "ZR", "ZT", "ZTU", "ZU", "ZUH", "ZZ"];
const SHUNTING_TYPES: [&str; 4] = ["RR", "RT", "RTU", "RU"];
const MIDDLE_TYPES: [&str; 2] = ["ZM", "ZUM"];

fn known(set: &[&str], v: Option<String>) -> Option<Value> {
    v.filter(|v| set.contains(&v.as_str())).map(Value::String)
}

/// The net and the signalling of a plan's objects (`convert`): tracks,
/// switches, end marks, the signalling domain and the report.
/// Tracks, switches, end marks, the signalling domain and the report.
type Converted = (Vec<Value>, Vec<Value>, Vec<Value>, Value, Report);

fn convert(model: &Model) -> Result<Converted, String> {
    let mut report = Report {
        notes: vec![],
        skipped: OMap::new(),
    };
    let of = |tag: &str| model.objects.iter().filter(|(_, o)| o.tag == tag).map(|(_, o)| o).collect::<Vec<_>>();
    let get = |id: Option<&str>| model.get(id);
    let id_of = |el: &El| lower(val(el, "Identitaet")).unwrap_or_default();
    // A plan knows two states and no days: what is only in the target is newly built, what is only in the
    // start removed, the day open (decision 0006).
    let lifespan_of = |o: &Obj| -> Option<Value> {
        if o.start && o.target {
            None
        } else if o.target {
            Some(json!({ "from": "open" }))
        } else {
            Some(json!({ "to": "open" }))
        }
    };

    // ── geometry: GEO_Knoten → plane point ──
    let mut point_of_node: HashMap<String, Utm> = HashMap::new();
    let mut epsg: Option<i64> = None;
    for o in of("GEO_Punkt") {
        let Some(node) = lower(val(o.el, "ID_GEO_Knoten")).filter(|n| !n.is_empty()) else {
            continue;
        };
        if point_of_node.contains_key(&node) {
            continue;
        }
        let system = val(o.el, "GEO_Punkt_Allg/GEO_Koordinatensystem");
        let Some(code) = system.as_deref().and_then(m::coordinate_system) else {
            report.note("coordinate_system_unknown", vec![("system", os(&system))]);
            continue;
        };
        let zone = *epsg.get_or_insert(code);
        if code != zone {
            report.note("coordinate_systems_many", vec![("system", os(&system))]);
            continue;
        }
        let easting = num(val(o.el, "GEO_Punkt_Allg/GK_X"));
        let northing = num(val(o.el, "GEO_Punkt_Allg/GK_Y"));
        if let (Some(e), Some(n)) = (easting, northing) {
            point_of_node.insert(node, Utm { e, n, zone: code });
        }
    }
    let mut geo_by_owner: OMap<Option<String>, Vec<&El>> = OMap::new();
    for o in of("GEO_Kante") {
        geo_by_owner.entry(lower(val(o.el, "ID_GEO_Art")), Vec::new).push(o.el);
    }

    // ── the net: an edge is a track ──
    let mut edges: OMap<String, Edge> = OMap::new();
    for o in of("TOP_Kante") {
        edges.set(
            id_of(o.el),
            Edge {
                o,
                a: lower(val(o.el, "ID_TOP_Knoten_A")),
                b: lower(val(o.el, "ID_TOP_Knoten_B")),
                conn_a: val(o.el, "TOP_Kante_Allg/TOP_Anschluss_A"),
                conn_b: val(o.el, "TOP_Kante_Allg/TOP_Anschluss_B"),
                length: num(val(o.el, "TOP_Kante_Allg/TOP_Laenge")),
            },
        );
    }
    let geo_node_of = |top: &Option<String>| get(top.as_deref()).and_then(|o| lower(val(o.el, "ID_GEO_Knoten")));
    let covering = |tag: &str, path: &str| {
        let mut into: HashMap<String, String> = HashMap::new();
        for o in of(tag) {
            let Some(v) = val(o.el, path) else { continue };
            for part in children_named(Some(o.el), "Bereich_Objekt_Teilbereich") {
                if let Some(edge) = lower(val(part, "ID_TOP_Kante")).filter(|e| !e.is_empty()) {
                    into.entry(edge).or_insert_with(|| v.clone());
                }
            }
        }
        into
    };
    let names = covering("Gleis_Bezeichnung", "Bezeichnung/Bez_Gleis_Bezeichnung");
    let lines = covering("Strecke", "Bezeichnung/Bezeichnung_Strecke");

    let mut tracks: Vec<Value> = Vec::new();
    for (id, e) in edges.iter() {
        let geos: Vec<&El> = geo_by_owner.get(&Some(id.clone())).cloned().unwrap_or_default();
        let mut at = geo_node_of(&e.a);
        let end = geo_node_of(&e.b);
        let mut left = geos.clone();
        let mut elements: Vec<Value> = Vec::new();
        while truthy(&at) && at != end && !left.is_empty() {
            let node_a = |g: &El| lower(val(g, "ID_GEO_Knoten_A"));
            let node_b = |g: &El| lower(val(g, "ID_GEO_Knoten_B"));
            let Some(pos) = left.iter().position(|g| node_a(g) == at || node_b(g) == at) else {
                break;
            };
            let next = left.remove(pos);
            let forward = node_a(next) == at;
            let el = geo_element(next, &point_of_node, &mut report)?;
            at = if forward { node_b(next) } else { node_a(next) };
            if let Some(el) = el {
                elements.push(if forward { el } else { reverse_element(&el) });
            }
        }
        if elements.is_empty() || at != end {
            report.note("edge_without_geometry", vec![("id", s(id)), ("geo", Some(json!(geos.len())))]);
            if elements.is_empty() {
                continue;
            }
        }
        let sum = elements.iter().fold(0.0, |s, el| s + js_number(el.get("length")));
        if let Some(top) = e.length {
            if (sum - top).abs() > 0.05 {
                report.note(
                    "edge_length_differs",
                    vec![
                        ("id", s(id)),
                        ("top", Some(number(top))),
                        ("geo", Some(number(js_round(sum * 1000.0) / 1000.0))),
                    ],
                );
            }
        }
        let name = names.get(id).map(|n| json!(n));
        tracks.push(Value::Object(obj(vec![
            ("id", s(id)),
            ("epsg", Some(epsg.map(|z| json!(z)).unwrap_or(Value::Null))),
            ("elements", Some(Value::Array(elements))),
            ("name", name.clone()),
            ("trackNumber", name),
            ("lineNumber", lines.get(id).map(|n| json!(n))),
            ("lifespan", lifespan_of(e.o)),
        ])));
    }
    let track_ids: HashSet<String> = tracks.iter().map(|t| js_string(t.get("id"))).collect();

    // ── switches: a node of tip, left and right; the switch element names it ──
    let mut ends: OMap<Option<String>, Vec<End>> = OMap::new();
    for (id, e) in edges.iter() {
        if !track_ids.contains(id) {
            continue;
        }
        for (node, endpoint, conn) in [(&e.a, "BEGIN", &e.conn_a), (&e.b, "END", &e.conn_b)] {
            ends.entry(node.clone(), Vec::new).push(End {
                edge: id.clone(),
                endpoint,
                conn: conn.clone(),
            });
        }
    }
    let node_at = |place: Option<&Place>| -> Option<Option<String>> {
        let place = place?;
        let e = edges.get(place.edge.as_ref()?)?;
        let abstand = place.abstand?;
        if abstand.abs() < 0.01 {
            return Some(e.a.clone());
        }
        if e.length.is_some_and(|l| (abstand - l).abs() < 0.01) {
            return Some(e.b.clone());
        }
        None
    };
    let mut components: OMap<Option<String>, Vec<usize>> = OMap::new();
    let mut comps: Vec<Comp> = Vec::new();
    let mut component_of_node: HashMap<String, usize> = HashMap::new();
    for o in of("W_Kr_Gsp_Komponente") {
        let element = lower(val(o.el, "ID_W_Kr_Gsp_Element"));
        let place = places_of(o.el).into_iter().next();
        let node = node_at(place.as_ref());
        let k = comps.len();
        comps.push(Comp {
            element: element.clone(),
            place,
            node: node.clone(),
            trunk: val(o.el, "Zungenpaar/Stammgleis"),
        });
        components.entry(element, Vec::new).push(k);
        if let Some(Some(n)) = node.filter(truthy) {
            component_of_node.insert(n, k);
        }
    }
    let installation_of = |element: &Option<String>| get(get(element.as_deref()).and_then(|e| val(e.el, "ID_W_Kr_Anlage")).as_deref());
    let mut switches: Vec<Value> = Vec::new();
    let mut switch_of_node: HashMap<Option<String>, Option<String>> = HashMap::new();
    for (node, list) in ends.iter() {
        let mut by_conn: HashMap<&str, &End> = HashMap::new();
        for x in list {
            if let Some(c) = x.conn.as_deref() {
                by_conn.insert(c, x);
            }
        }
        if list.len() == 1 {
            continue;
        }
        let three = if list.len() == 3 {
            (by_conn.get("Spitze"), by_conn.get("Links"), by_conn.get("Rechts"))
        } else {
            (None, None, None)
        };
        let (Some(tip), Some(l), Some(r)) = three else {
            if list.len() == 2 {
                continue;
            }
            let conns = Value::Array(list.iter().map(|x| os(&x.conn).unwrap_or(Value::Null)).collect());
            report.note("node_not_a_turnout", vec![("id", os(node)), ("ends", Some(conns))]);
            continue;
        };
        let c = node.as_ref().and_then(|n| component_of_node.get(n)).map(|&k| &comps[k]);
        let element = c.and_then(|c| get(c.element.as_deref()));
        let installation = c.and_then(|c| installation_of(&c.element));
        let art = installation.and_then(|i| val(i.el, "W_Kr_Anlage_Allg/W_Kr_Art"));
        // The trunk goes on straight: A–B2 is the straight route of a turnout.
        let straight = if c.and_then(|c| c.trunk.as_deref()) == Some("links") { *l } else { *r };
        let branch = if std::ptr::eq(straight, *l) { *r } else { *l };
        let port = |x: &End| json!({ "trackId": x.edge, "endpoint": x.endpoint });
        let state = get(node.as_deref());
        let sw = obj(vec![
            ("switchId", os(node)),
            ("kind", s(art.as_deref().and_then(m::switch_kind).unwrap_or("switch"))),
            ("formVersion", Some(json!(1))),
            (
                "name",
                element
                    .and_then(|e| val(e.el, "Bezeichnung/Bezeichnung_Tabelle").or_else(|| val(e.el, "Bezeichnung/Bezeichnung_Lageplan_Kurz")))
                    .map(Value::String),
            ),
            (
                "label",
                installation.and_then(|i| val(i.el, "W_Kr_Anlage_Allg/W_Kr_Grundform")).map(Value::String),
            ),
            ("ports", Some(json!({ "A": port(tip), "B1": port(branch), "B2": port(straight) }))),
            ("lifespan", state.and_then(lifespan_of)),
        ]);
        if let Some(art) = art.as_deref().filter(|a| !a.is_empty() && m::switch_kind(a).is_none()) {
            report.note("switch_kind_as_turnout", vec![("id", os(node)), ("kind", s(art))]);
        }
        switches.push(Value::Object(sw));
        switch_of_node.insert(node.clone(), node.clone());
    }

    // ── end marks: a buffer stop where a Gleis_Abschluss stands, else the planning area's boundary ──
    let nearest_end = |place: Option<&Place>| -> Option<String> {
        let place = place?;
        let e = edges.get(place.edge.as_ref()?)?;
        let abstand = place.abstand?;
        let single = |n: &Option<String>| ends.get(n).map(|l| l.len()).unwrap_or(0) == 1;
        let (at_a, at_b) = (single(&e.a), single(&e.b));
        if at_a && (!at_b || abstand < e.length.unwrap_or(f64::INFINITY) / 2.0) {
            return e.a.clone();
        }
        if at_b { e.b.clone() } else { None }
    };
    let mut closures: HashMap<String, Option<String>> = HashMap::new();
    for o in of("Gleis_Abschluss") {
        let places = places_of(o.el);
        let node = node_at(places.first()).flatten().or_else(|| nearest_end(places.first()));
        if let Some(node) = node.filter(|n| !n.is_empty()) {
            closures.insert(node, val(o.el, "Gleis_Abschluss_Art"));
        }
    }
    let mut end_marks: Vec<Value> = Vec::new();
    for (node, list) in ends.iter() {
        if list.len() != 1 {
            continue;
        }
        let kind = node.as_ref().and_then(|n| closures.get(n)).cloned().flatten();
        end_marks.push(Value::Object(obj(vec![
            ("id", os(node)),
            (
                "kind",
                s(if kind.as_deref().is_some_and(m::buffer_stop) {
                    "buffer_stop"
                } else {
                    "boundary"
                }),
            ),
            ("trackId", s(&list[0].edge)),
            ("endpoint", s(list[0].endpoint)),
        ])));
    }

    // ── signalling ──
    let loc = |place: Option<&Place>, directed: bool| -> Option<Map<String, Value>> {
        let place = place?;
        let edge = place.edge.as_ref().filter(|e| track_ids.contains(*e))?;
        let abstand = place.abstand?;
        let lateral = place.lateral;
        let side = place.side.as_deref().and_then(m::side).or(match lateral {
            Some(l) if l < 0.0 => Some("left"),
            Some(l) if l > 0.0 => Some("right"),
            _ => None,
        });
        Some(obj(vec![
            ("anchor", s("station")),
            ("trackId", s(edge)),
            ("station", Some(number(abstand))),
            ("side", side.map(|v| json!(v))),
            ("offset", lateral.filter(|l| *l != 0.0).map(|l| number(l.abs()))),
            (
                "direction",
                (directed || truthy(&place.wirk)).then(|| json!(if place.wirk.as_deref() == Some("gegen") { "down" } else { "up" })),
            ),
        ]))
    };
    let name = |el: &El| {
        val(el, "Bezeichnung/Bezeichnung_Lageplan_Kurz")
            .or_else(|| val(el, "Bezeichnung/Oertlicher_Elementname"))
            .map(Value::String)
    };
    let area_number = |el: &El| val(el, "Bezeichnung/Kennzahl").map(Value::String);

    // Frames and aspects of every signal.
    let mut frames: Vec<Frame> = Vec::new();
    let mut frames_of: HashMap<Option<String>, Vec<usize>> = HashMap::new();
    let mut frame_by_id: HashMap<String, usize> = HashMap::new();
    for o in of("Signal_Rahmen") {
        let signal = lower(val(o.el, "ID_Signal"));
        let kind_name = val(o.el, "Rahmen_Art");
        let kind = kind_name.as_deref().and_then(m::frame);
        if kind.is_none() {
            report.note("frame_kind_other", vec![("kind", os(&kind_name))]);
        }
        let k = frames.len();
        frames.push(Frame {
            signal: signal.clone(),
            kind: kind.unwrap_or("other"),
            aspects: vec![],
        });
        frames_of.entry(signal).or_default().push(k);
        frame_by_id.insert(id_of(o.el), k);
    }
    let mut aspect_by_id: HashMap<String, (Option<String>, Value)> = HashMap::new();
    for o in of("Signal_Signalbegriff") {
        let frame = lower(val(o.el, "ID_Signal_Rahmen")).and_then(|f| frame_by_id.get(&f).copied());
        let id = child(Some(o.el), "Signalbegriff_ID");
        let kind = id.and_then(|i| i.attr("xsi:type")).map(|t| t.rsplit(':').next().unwrap_or("").to_string());
        let (Some(frame), Some(kind)) = (frame, kind.filter(|k| !k.is_empty())) else {
            continue;
        };
        let lit = val(o.el, "Signal_Signalbegriff_Allg/Beleuchtet");
        let aspect = Value::Object(obj(vec![
            ("aspect", s(&kind)),
            (
                "symbol",
                child(id, "Symbol").map(|s| s.text.clone()).filter(|t| !t.is_empty()).map(Value::String),
            ),
            (
                "lit",
                match lit.as_deref() {
                    Some("ja") => Some(json!(true)),
                    Some("nein") => Some(json!(false)),
                    _ => None,
                },
            ),
            ("switched", boolean(val(o.el, "Signal_Signalbegriff_Allg/Geschaltet")).map(Value::Bool)),
        ]));
        frames[frame].aspects.push(aspect.clone());
        aspect_by_id.insert(id_of(o.el), (frames[frame].signal.clone(), aspect));
    }

    let mut signals: Vec<Map<String, Value>> = Vec::new();
    for o in of("Signal") {
        let id = id_of(o.el);
        let places = places_of(o.el);
        let Some(at) = loc(places.first(), true) else {
            report.missed("signals", &id, "no_track");
            continue;
        };
        let real = child(Some(o.el), "Signal_Real");
        let fictive = child(Some(o.el), "Signal_Fiktiv");
        let screen = child(real, "Signal_Real_Aktiv_Schirm");
        let function = value(real, "Signal_Funktion");
        let system = value(screen, "Signalsystem");
        let also_at: Vec<Value> = places[1..].iter().filter_map(|p| loc(Some(p), false)).map(Value::Object).collect();
        let sight = real.map(|r| {
            Value::Object(obj(vec![
                ("required", on(num(val(r, "Signalsicht_Soll")))),
                ("minimum", on(num(val(r, "Signalsicht_Mindest")))),
                ("achievable", on(num(val(r, "Signalsicht_Erreichbar")))),
            ]))
        });
        let own_frames = real.map(|_| {
            let list = frames_of.get(&Some(id.clone())).cloned().unwrap_or_default();
            Value::Array(list.iter().map(|&k| json!({ "kind": frames[k].kind, "aspects": frames[k].aspects })).collect())
        });
        let sig = obj(vec![
            ("id", s(&id)),
            ("name", name(o.el)),
            ("areaNumber", area_number(o.el)),
            ("kind", s(if real.is_some() { "real" } else { "virtual" })),
            ("at", Some(Value::Object(at))),
            ("alsoAt", non_empty(Value::Array(also_at))),
            ("bothDirections", (places[0].wirk.as_deref() == Some("beide")).then(|| json!(true))),
            ("function", real.and(function.as_deref().and_then(m::signal_function)).map(|v| json!(v))),
            (
                "virtualFunction",
                fictive.map(|f| {
                    json!(
                        value(Some(f), "Fiktives_Signal_Funktion")
                            .as_deref()
                            .and_then(m::virtual_function)
                            .unwrap_or("other")
                    )
                }),
            ),
            (
                "logicalSignals",
                screen.and_then(|sc| {
                    logical_signals_of(
                        system.as_deref().and_then(m::signal_system),
                        value(Some(sc), "Signal_Art").as_deref().and_then(m::signal_type),
                    )
                }),
            ),
            (
                "mounting",
                real.and_then(|r| val(r, "Signal_Befestigungsart"))
                    .as_deref()
                    .and_then(m::mounting)
                    .map(|v| json!(v)),
            ),
            ("sight", sight.and_then(non_empty)),
            ("frames", own_frames),
            ("lifespan", lifespan_of(o)),
        ]);
        if let (Some(f), Some(_)) = (function.as_deref().filter(|f| !f.is_empty()), real) {
            if m::signal_function(f).is_none() {
                report.note("signal_function_unknown", vec![("id", s(&id)), ("value", s(f))]);
            }
        }
        signals.push(sig);
    }
    let signal_ids: HashSet<String> = signals.iter().map(|s| js_string(s.get("id"))).collect();
    let is_signal = |id: &Option<String>| id.as_ref().is_some_and(|i| signal_ids.contains(i));

    // Train detection.
    let mut detection_points: Vec<Value> = Vec::new();
    let mut bounded_by: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for o in of("FMA_Komponente") {
        let id = id_of(o.el);
        let Some(mut at) = loc(places_of(o.el).first(), false) else {
            report.missed("detectionPoints", &id, "no_track");
            continue;
        };
        at.remove("direction");
        let art = val(o.el, "FMA_Komponente_Art");
        detection_points.push(Value::Object(obj(vec![
            ("id", s(&id)),
            ("name", name(o.el)),
            ("kind", s(art.as_deref().and_then(m::detection_point).unwrap_or("axleCounter"))),
            ("at", Some(Value::Object(at))),
            ("lifespan", lifespan_of(o)),
        ])));
        for g in children_named(Some(o.el), "ID_FMAgrenze") {
            let section = lower(child(Some(g), "Wert").map(|w| w.text.clone()));
            bounded_by.entry(section).or_default().push(id.clone());
        }
    }
    let mut track_sections: Vec<Value> = Vec::new();
    for o in of("FMA_Anlage") {
        let id = id_of(o.el);
        let section = get(val(o.el, "ID_Gleis_Abschnitt").as_deref());
        let letter = val(o.el, "Bezeichnung/Bezeichnung_Kennbuchstabe").unwrap_or_default();
        let short = section.and_then(|s| val(s.el, "Bezeichnung/Bezeichnung_Lageplan_Kurz"));
        let points = bounded_by.get(&Some(id.clone())).cloned().unwrap_or_default();
        if points.is_empty() {
            report.missed("trackSections", &id, "no_points");
            continue;
        }
        let mut unique: Vec<String> = Vec::new();
        for p in points {
            if !unique.contains(&p) {
                unique.push(p);
            }
        }
        unique.sort_by(|a, b| cmp_utf16(a, b));
        track_sections.push(Value::Object(obj(vec![
            ("id", s(&id)),
            ("name", short.map(|sh| json!(format!("{letter}{sh}")))),
            ("areaNumber", section.and_then(|s| area_number(s.el))),
            (
                "kind",
                s(val(o.el, "FMA_Anlage_Allg/FMA_Art").as_deref().and_then(m::section_kind).unwrap_or("other")),
            ),
            ("detectionPointIds", Some(arr(unique))),
            (
                "auxiliaryClearing",
                (boolean(val(o.el, "FMA_Anlage_Allg/FMA_Hilffreimeldung")) == Some(true)).then(|| json!(true)),
            ),
            ("lifespan", lifespan_of(o)),
        ])));
    }
    let section_ids: HashSet<String> = track_sections.iter().map(|t| js_string(t.get("id"))).collect();
    let section_refs = |el: Option<&El>, name: &str| -> Vec<String> {
        children_named(el, name)
            .into_iter()
            .filter_map(|r| lower(child(Some(r), "Wert").map(|w| w.text.clone())))
            .filter(|id| section_ids.contains(id))
            .collect()
    };

    // Switch elements, their twofold flank protection.
    let mut twofold: HashMap<Option<String>, Option<Value>> = HashMap::new();
    for o in of("Fla_Zwieschutz") {
        let z = child(Some(o.el), "Fla_Zwieschutz_Element");
        let measure = |path: &str| value(z, path).as_deref().and_then(m::twofold_measure).map(|v| json!(v));
        twofold.insert(
            lower(val(o.el, "ID_W_Element")),
            non_empty(Value::Object(obj(vec![
                ("kind", val(o.el, "Zwieschutz_Art").as_deref().and_then(m::twofold_kind).map(|v| json!(v))),
                ("left", measure("Massnahme_L")),
                ("right", measure("Massnahme_R")),
                ("preventsRunOn", boolean(value(z, "Nachlaufverhinderung")).map(Value::Bool)),
            ]))),
        );
    }
    let has_switch = |n: &Option<Option<String>>| n.as_ref().is_some_and(|k| switch_of_node.contains_key(k));
    let mut switch_elements: Vec<Value> = Vec::new();
    for o in of("W_Kr_Gsp_Element") {
        let id = id_of(o.el);
        let mine: Vec<&Comp> = components
            .get(&Some(id.clone()))
            .map(|l| l.iter().map(|&k| &comps[k]).collect())
            .unwrap_or_default();
        let turnout = child(Some(o.el), "Weiche_Element");
        let derailer = child(Some(o.el), "Gleissperre_Element");
        let operation = val(o.el, "W_Kr_Gsp_Element_Allg/W_Kr_Gsp_Stellart")
            .as_deref()
            .and_then(m::operation)
            .unwrap_or("other");
        let mut base = obj(vec![
            ("id", s(&id)),
            ("name", name(o.el)),
            ("areaNumber", area_number(o.el)),
            ("operation", s(operation)),
            (
                "preferredAutomatic",
                boolean(val(o.el, "W_Kr_Gsp_Element_Allg/Vorzugslage_Automatik")).map(Value::Bool),
            ),
            ("lifespan", lifespan_of(o)),
        ]);
        if let Some(derailer) = derailer {
            let Some(at) = loc(mine.first().and_then(|c| c.place.as_ref()), true) else {
                report.missed("switchElements", &id, "no_track");
                continue;
            };
            let d = obj(vec![
                ("at", Some(Value::Object(at))),
                (
                    "preferredPosition",
                    val(derailer, "Gleissperre_Vorzugslage")
                        .as_deref()
                        .and_then(m::derailer_position)
                        .map(|v| json!(v)),
                ),
            ]);
            base.insert("derailer".into(), Value::Object(d));
            switch_elements.push(Value::Object(base));
            continue;
        }
        let node = mine.iter().map(|c| &c.node).find(|n| has_switch(n)).cloned().flatten().flatten();
        let Some(node) = node.filter(|n| !n.is_empty()) else {
            report.missed("switchElements", &id, "no_switch");
            continue;
        };
        if mine.iter().filter(|c| has_switch(&c.node)).count() > 1 {
            report.note("switch_element_many_nodes", vec![("id", s(&id)), ("nodes", Some(json!(mine.len())))]);
        }
        let marker = lower(value(turnout, "ID_Grenzzeichen"));
        let extra = obj(vec![
            ("switchId", os(switch_of_node.get(&Some(node)).unwrap())),
            (
                "preferredPosition",
                value(turnout, "Weiche_Vorzugslage").as_deref().and_then(m::position).map(|v| json!(v)),
            ),
            ("trailable", boolean(value(turnout, "Auffahrortung")).map(Value::Bool)),
            ("clearanceMarkerId", if is_signal(&marker) { os(&marker) } else { None }),
            ("twofoldProtection", twofold.get(&Some(id.clone())).cloned().flatten()),
        ]);
        base.extend(extra);
        switch_elements.push(Value::Object(base));
    }
    let element_ids: HashSet<String> = switch_elements.iter().map(|e| js_string(e.get("id"))).collect();
    let is_element = |id: &Option<String>| id.as_ref().is_some_and(|i| element_ids.contains(i));

    // Runs: a Fstr_Fahrweg's parts on edges, in running order from its start's edge.
    let mut runs: HashMap<String, Run> = HashMap::new();
    for o in of("Fstr_Fahrweg") {
        let start = lower(val(o.el, "ID_Start"));
        let target = lower(val(o.el, "ID_Ziel"));
        let parts: Vec<String> = children_named(Some(o.el), "Bereich_Objekt_Teilbereich")
            .into_iter()
            .filter_map(|p| lower(val(p, "ID_TOP_Kante")))
            .filter(|id| track_ids.contains(id))
            .collect();
        let first = signals
            .iter()
            .find(|s| start.as_deref().is_some_and(|st| s.get("id").and_then(Value::as_str) == Some(st)))
            .and_then(|s| s["at"]["trackId"].as_str());
        runs.insert(
            id_of(o.el),
            Run {
                start: start.clone(),
                target,
                speed: num(val(o.el, "Fstr_V_Hg")),
                track_ids: in_running_order(parts, first, &edges),
            },
        );
    }
    let run_of = |el: &El| lower(val(el, "ID_Fstr_Fahrweg")).and_then(|r| runs.get(&r));

    // Overlaps.
    let mut overlap_switches: HashMap<Option<String>, Vec<Value>> = HashMap::new();
    for o in of("Fstr_DWeg_W_Kr") {
        let list = overlap_switches.entry(lower(val(o.el, "ID_Fstr_DWeg"))).or_default();
        let el = lower(val(o.el, "ID_W_Kr_Gsp_Element"));
        if is_element(&el) {
            list.push(json!({ "switchElementId": el, "locked": boolean(val(o.el, "Element_Verschluss")).unwrap_or(false) }));
        }
    }
    // Where a Markanter_Punkt is: the place of the point object it marks.
    let marked_place = |id: Option<&str>, run: &[String]| -> Option<Place> {
        let mark = get(id);
        let target = mark.and_then(|m| get(val(m.el, "ID_Markante_Stelle").as_deref()));
        let places = target.map(|t| places_of(t.el)).unwrap_or_default();
        let last = run.last().and_then(|l| edges.get(l));
        let meets = |p: &Place| {
            let e = p.edge.as_ref().and_then(|x| edges.get(x));
            p.edge.as_ref().is_some_and(|x| run.contains(x))
                || matches!((e, last), (Some(e), Some(last)) if [&last.a, &last.b].iter().any(|n| **n == e.a || **n == e.b))
        };
        places.iter().find(|p| meets(p)).or(places.first()).cloned()
    };
    let mut overlaps: Vec<Value> = Vec::new();
    for o in of("Fstr_DWeg") {
        let id = id_of(o.el);
        let run = run_of(o.el);
        let empty: Vec<String> = vec![];
        let run_ids = run.map(|r| &r.track_ids).unwrap_or(&empty);
        let end_place =
            marked_place(val(o.el, "ID_PZB_Gefahrpunkt").as_deref(), run_ids).or_else(|| run.and_then(|r| marked_place(r.target.as_deref(), &r.track_ids)));
        let end = loc(end_place.as_ref(), false);
        let (Some(run), Some(mut end)) = (run.filter(|r| is_signal(&r.start) && !r.track_ids.is_empty()), end.clone()) else {
            report.missed("overlaps", &id, if end.is_none() { "no_end" } else { "no_run" });
            continue;
        };
        for k in ["direction", "side", "offset"] {
            end.remove(k);
        }
        overlaps.push(Value::Object(obj(vec![
            ("id", s(&id)),
            ("name", val(o.el, "Bezeichnung/Bezeichnung_Fstr_DWeg").map(Value::String)),
            ("signalId", os(&run.start)),
            ("path", Some(json!({ "trackIds": run.track_ids }))),
            ("end", Some(Value::Object(end))),
            ("length", Some(number(num(val(o.el, "Fstr_DWeg_Allg/Laenge_Soll")).unwrap_or(0.0)))),
            ("gradient", on(num(val(o.el, "Fstr_DWeg_Allg/Massgebende_Neigung")))),
            ("speed", on(num(val(o.el, "Fstr_DWeg_Spezifisch/DWeg_V")))),
            ("releaseDelay", on(num(val(o.el, "Fstr_DWeg_Spezifisch/Aufloesung_Verzoegerung")))),
            ("clearSectionIds", non_empty(arr(section_refs(Some(o.el), "ID_FMA_Anlage_Freimeldung")))),
            (
                "switches",
                overlap_switches.get(&Some(id.clone())).and_then(|l| non_empty(Value::Array(l.clone()))),
            ),
            ("lifespan", lifespan_of(o)),
        ])));
    }
    let overlap_ids: HashSet<String> = overlaps.iter().map(|x| js_string(x.get("id"))).collect();

    // Routes and what they show.
    let mut shown: HashMap<Option<String>, Vec<Value>> = HashMap::new();
    for o in of("Fstr_Signalisierung") {
        let route = lower(val(o.el, "ID_Fstr_Zug_Rangier"));
        let Some((signal, aspect)) = lower(val(o.el, "ID_Signal_Signalbegriff")).and_then(|a| aspect_by_id.get(&a)) else {
            continue;
        };
        if !is_signal(signal) {
            continue;
        }
        shown.entry(route).or_default().push(Value::Object(obj(vec![
            ("signalId", os(signal)),
            ("aspect", aspect.get("aspect").cloned()),
            ("symbol", aspect.get("symbol").cloned()),
        ])));
    }
    let mut routes: Vec<Value> = Vec::new();
    let mut route_speeds: HashMap<String, Vec<Value>> = HashMap::new();
    for o in of("Fstr_Zug_Rangier") {
        let id = id_of(o.el);
        let Some(run) = run_of(o.el).filter(|r| is_signal(&r.start) && is_signal(&r.target)) else {
            report.missed("routes", &id, "no_signals");
            continue;
        };
        let train = child(Some(o.el), "Fstr_Zug");
        let shunting = child(Some(o.el), "Fstr_Rangier");
        let middle = child(Some(o.el), "Fstr_Mittel");
        let ovl: Vec<Value> = children_named(train, "Fstr_Zug_DWeg")
            .into_iter()
            .map(|d| (lower(val(d, "ID_Fstr_DWeg")), boolean(val(d, "DWeg_Vorzug"))))
            .filter(|(ov, _)| ov.as_ref().is_some_and(|x| overlap_ids.contains(x)))
            .map(|(ov, pref)| Value::Object(obj(vec![("overlapId", os(&ov)), ("preferred", pref.map(Value::Bool))])))
            .collect();
        routes.push(Value::Object(obj(vec![
            ("id", s(&id)),
            (
                "kind",
                s(if train.is_some() {
                    "train"
                } else if middle.is_some() {
                    "middle"
                } else {
                    "shunting"
                }),
            ),
            ("trainType", known(&TRAIN_TYPES, value(train, "Fstr_Zug_Art"))),
            ("shuntingType", known(&SHUNTING_TYPES, value(shunting, "Fstr_Rangier_Art"))),
            ("middleType", known(&MIDDLE_TYPES, value(middle, "Fstr_Mittel_Art"))),
            ("rank", on(num(val(o.el, "Fstr_Zug_Rangier_Allg/Fstr_Reihenfolge")))),
            ("startSignalId", os(&run.start)),
            ("targetSignalId", os(&run.target)),
            ("path", Some(json!({ "trackIds": run.track_ids }))),
            (
                "autoSetting",
                (value(train, "Automatische_Einstellung").as_deref() == Some("ZL")).then(|| json!(true)),
            ),
            (
                "shortDistantSpacing",
                (boolean(value(train, "Fstr_Vsigabstand_Verkuerzt")) == Some(true)).then(|| json!(true)),
            ),
            ("clearSectionIds", non_empty(arr(section_refs(shunting, "ID_FMA_Anlage_Rangier_Frei")))),
            ("overlaps", non_empty(Value::Array(ovl))),
            ("aspects", shown.get(&Some(id.clone())).and_then(|l| non_empty(Value::Array(l.clone())))),
            ("lifespan", lifespan_of(o)),
        ])));
        if let (Some(speed), Some(start)) = (run.speed, &run.start) {
            route_speeds
                .entry(start.clone())
                .or_default()
                .push(json!({ "routeId": id, "speed": number(speed) }));
        }
    }
    let route_ids: HashSet<String> = routes.iter().map(|r| js_string(r.get("id"))).collect();

    // Flank protection.
    let mut flank_clear: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for o in of("Fla_Freimelde_Zuordnung") {
        let f = lower(val(o.el, "ID_Fla_Schutz"));
        let Some(sec) = lower(val(o.el, "ID_FMA_Anlage")).filter(|x| section_ids.contains(x)) else {
            continue;
        };
        flank_clear.entry(f).or_default().push(sec);
    }
    let mut flank: Vec<Map<String, Value>> = Vec::new();
    for o in of("Fla_Schutz") {
        let id = id_of(o.el);
        let requester = lower(val(o.el, "Fla_Schutz_Anforderer/ID_Anforderer_Element"));
        let via = val(o.el, "Fla_Schutz_Anforderer/Fahrt_Ueber").as_deref().and_then(m::via);
        let (true, Some(via)) = (is_element(&requester), via) else {
            report.missed("flankProtection", &id, "requester_not_a_switch");
            continue;
        };
        let by_signal = lower(val(o.el, "Fla_Schutz_Signal/ID_Fla_Signal"));
        let by_element = lower(val(o.el, "Fla_Schutz_W_Gsp/ID_Fla_W_Gsp_Element"));
        let protection = if is_signal(&by_signal) {
            Some(Value::Object(obj(vec![
                ("signalId", os(&by_signal)),
                ("targetLock", boolean(val(o.el, "Fla_Schutz_Signal/Fla_Signal_Zielsperrung")).map(Value::Bool)),
            ])))
        } else if is_element(&by_element) {
            Some(Value::Object(obj(vec![
                ("switchElementId", os(&by_element)),
                (
                    "position",
                    val(o.el, "Fla_Schutz_W_Gsp/Fla_W_Lage").as_deref().and_then(m::position).map(|v| json!(v)),
                ),
            ])))
        } else if boolean(val(o.el, "Fla_Verzicht")) == Some(true) {
            Some(json!({ "waived": true }))
        } else {
            None
        };
        let Some(protection) = protection else {
            report.missed("flankProtection", &id, "no_protection");
            continue;
        };
        flank.push(obj(vec![
            ("id", s(&id)),
            ("requester", Some(json!({ "switchElementId": requester, "via": via }))),
            ("protection", Some(protection)),
            (
                "passOn",
                non_empty(Value::Object(obj(vec![
                    ("leftId", os(&lower(val(o.el, "Fla_Schutz_Weitergabe/ID_Fla_Weitergabe_L")))),
                    ("rightId", os(&lower(val(o.el, "Fla_Schutz_Weitergabe/ID_Fla_Weitergabe_R")))),
                ]))),
            ),
            ("clearSectionIds", flank_clear.get(&Some(id.clone())).map(|l| arr(l.clone()))),
            ("lifespan", lifespan_of(o)),
        ]));
    }
    // A pass-on names a flank protection the import has.
    let flank_ids: HashSet<String> = flank.iter().map(|f| js_string(f.get("id"))).collect();
    for f in &mut flank {
        let Some(pass) = f.get_mut("passOn").and_then(Value::as_object_mut) else {
            continue;
        };
        for side in ["leftId", "rightId"] {
            let gone = pass.get(side).is_some_and(|v| {
                let id = js_string(Some(v));
                !id.is_empty() && !flank_ids.contains(&id)
            });
            if gone {
                pass.remove(side);
            }
        }
        if pass.is_empty() {
            f.remove("passOn");
        }
    }

    // PZB, as balises of the system pzb.
    struct Assignment {
        signal_id: Option<String>,
        effect: Option<&'static str>,
        routes: Vec<Value>,
    }
    let mut assignments: HashMap<String, Assignment> = HashMap::new();
    for o in of("PZB_Element_Zuordnung") {
        let bp = child(Some(o.el), "PZB_Element_Zuordnung_BP");
        let routes: Vec<Value> = children_named(Some(o.el), "PZB_Element_Zuordnung_Fstr")
            .into_iter()
            .map(|r| {
                (
                    lower(val(r, "ID_Fstr_Zug_Rangier")),
                    val(r, "Wirksamkeit_Fstr").as_deref().and_then(m::pzb_route_effect).unwrap_or("other"),
                )
            })
            .filter(|(id, _)| id.as_ref().is_some_and(|i| route_ids.contains(i)))
            .map(|(id, effect)| json!({ "routeId": id, "effect": effect }))
            .collect();
        assignments.insert(
            id_of(o.el),
            Assignment {
                signal_id: lower(value(bp, "ID_PZB_Element_Bezugspunkt")),
                effect: value(bp, "Wirksamkeit").as_deref().and_then(m::pzb_effect),
                routes,
            },
        );
    }
    let mut balises: Vec<Value> = Vec::new();
    for o in of("PZB_Element") {
        let id = id_of(o.el);
        let Some(at) = loc(places_of(o.el).first(), true) else {
            report.missed("balises", &id, "no_track");
            continue;
        };
        let z = lower(val(o.el, "ID_PZB_Element_Zuordnung")).and_then(|z| assignments.get(&z));
        let gue = child(Some(o.el), "PZB_Element_GUE");
        let settings = obj(vec![
            ("frequency", s(val(o.el, "PZB_Art").as_deref().and_then(m::pzb_frequency).unwrap_or("1000"))),
            ("ina", (boolean(val(o.el, "PZB_Element_GM/PZB_INA")) == Some(true)).then(|| json!(true))),
            ("testSpeed", gue.map(|g| number(num(val(g, "Pruefgeschwindigkeit")).unwrap_or(0.0)))),
            ("measuringLength", gue.map(|g| number(num(val(g, "GUE_Messstrecke")).unwrap_or(0.0)))),
            (
                "arrangement",
                gue.and_then(|g| val(g, "GUE_Anordnung"))
                    .as_deref()
                    .and_then(m::gue_arrangement)
                    .map(|v| json!(v)),
            ),
        ]);
        let signal = z.and_then(|z| z.signal_id.clone());
        balises.push(Value::Object(obj(vec![
            ("id", s(&id)),
            ("at", Some(Value::Object(at))),
            ("system", s("pzb")),
            ("kind", s(if gue.is_some() { "speedCheck" } else { "magnet" })),
            ("settings", Some(Value::Object(settings))),
            ("signalId", if is_signal(&signal) { os(&signal) } else { None }),
            ("effect", s(z.and_then(|z| z.effect).unwrap_or("other"))),
            ("routes", z.and_then(|z| non_empty(Value::Array(z.routes.clone())))),
            ("lifespan", lifespan_of(o)),
        ])));
    }

    for (_, o) in model.objects.iter() {
        if !READ.contains(&o.tag.as_str()) {
            *report.skipped.entry(o.tag.clone(), || 0) += 1;
        }
    }

    let signalling = json!({
        "fields": { "catalogue": rail_across_signalling::catalogue::catalogue_ref() },
        "collections": {
            "signals": signals.into_iter().map(|s| { let sp = route_speeds.get(&js_string(s.get("id"))); by_logical_signal(s, sp) }).collect::<Vec<_>>(),
            "detectionPoints": detection_points,
            "trackSections": track_sections,
            "switchElements": switch_elements,
            "routes": routes,
            "overlaps": overlaps,
            "flankProtection": flank,
            "balises": balises,
            "stationBoundaries": [],
            "stoppingPlaces": [],
            "itineraries": [],
        },
    });
    Ok((tracks, switches, end_marks, signalling, report))
}

/// A record of the suite from a PlanPro plan (`planproRecord`): { record, report, header }.
pub fn planpro_record(text: &str, id: Option<&str>, title: Option<&str>) -> Result<Value, String> {
    let root = crate::xml::parse_xml(text)?;
    let model = read_planpro(&root)?;
    let project_id = id.unwrap_or("planpro");
    let (tracks, switches, end_marks, signalling, report) = convert(&model)?;
    let title = title
        .map(|t| json!(t))
        .or_else(|| model.header.get("title").cloned())
        .or_else(|| model.header.get("subInstallation").cloned());
    let record = obj(vec![
        ("id", s(project_id)),
        ("title", title),
        ("tracks", Some(Value::Array(tracks))),
        ("switches", Some(Value::Array(switches))),
        ("platforms", Some(json!([]))),
        ("endMarks", Some(Value::Array(end_marks))),
        ("signalling", Some(signalling)),
    ]);
    Ok(json!({ "record": record, "report": report.summary(), "header": model.header }))
}
