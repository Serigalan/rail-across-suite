// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Writing a project as a PlanPro 1.10 plan (XSD 1.10.0.3), valid against
//! PlanPro's schema: the infra-data-1.0 document is the model, and the plan
//! says of it what PlanPro has a place for.
//!
//! The plan has two parts, as PlanPro tools write them: the geodata
//! (Untergewerk Geo) — the net of edges (TOP_Kante, one per track) and nodes,
//! its geometry (GEO_Kante per element, in the DB_REF Gauss-Krüger strip the
//! project lies in), lines, operational points, heights and cant — and the
//! interlocking (ESTW): track names and kinds, buffer stops, platforms,
//! turnouts and derailers, and the signalling domain. Each part has a start
//! and a target state: what exists is in both, what is new only in the
//! target, what goes only in the start. Every object carries its base
//! (`Basis_Objekt_Allg`), the objects of a state stand in the order the
//! schema lists them, and `Objektmanagement` says who planned what when.
//!
//! What PlanPro requires and the model does not say is filled with a stated
//! value and named in the report (`defaulted`); what PlanPro has no place for
//! here is named too (`not_written`). The axle counters and sections need an
//! outside element control: one placeholder (`Gleisfreimelde_Innenanlage`)
//! stands for it.
//!
//! Ids become GUIDs, as PlanPro has them: an id that is one stays, any other
//! is turned into one by name (`planpro_id`), the same every time.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use rail_across_geom::coords::{plane_to_wgs84, system_code_at, transform_grid_bearing, transform_plane_point};
use rail_across_signalling::catalogue::{aspect_of, symbol_fits};
use rail_across_signalling::overlaps::boundary_start_speed;
use serde_json::{Map, Value, json};

use crate::archive;
use crate::mapping as m;

const NS_PLANPRO: &str = "http://www.plan-pro.org/modell/PlanPro/1.10.0.3";
const NS_ASPECTS: &str = "http://www.plan-pro.org/modell/Signalbegriffe_Ril_301/1.10.0.3";
const XSD_VERSION: &str = "1.10.0.3";
const GON: f64 = 0.9;
/// Where the geometry comes from (`Plan_Quelle`): the suite plans.
const SOURCE: &str = "Planung";
/// The longest GEO_Kante PlanPro takes [m]; a longer straight is cut.
const MAX_GEO_LENGTH: f64 = 9999.99;

/// The objects of a Container in the order PlanPro's schema lists them
/// (CContainer), as far as they are written here.
const CONTAINER_ORDER: &[&str] = &[
    "Aussenelementansteuerung",
    "Bahnsteig_Anlage",
    "Bahnsteig_Kante",
    "Fla_Freimelde_Zuordnung",
    "Fla_Schutz",
    "Fla_Zwieschutz",
    "FMA_Anlage",
    "FMA_Komponente",
    "Fstr_DWeg",
    "Fstr_DWeg_W_Kr",
    "Fstr_Fahrweg",
    "Fstr_Signalisierung",
    "Fstr_Zug_Rangier",
    "GEO_Kante",
    "GEO_Knoten",
    "GEO_Punkt",
    "Gleis_Abschluss",
    "Gleis_Abschnitt",
    "Gleis_Art",
    "Gleis_Bezeichnung",
    "Hoehenlinie",
    "Hoehenpunkt",
    "Markanter_Punkt",
    "Oertlichkeit",
    "PZB_Element",
    "PZB_Element_Zuordnung",
    "Signal",
    "Signal_Rahmen",
    "Signal_Signalbegriff",
    "Sonstiger_Punkt",
    "Strecke",
    "TOP_Kante",
    "TOP_Knoten",
    "Ueberhoehung",
    "Ueberhoehungslinie",
    "W_Kr_Anlage",
    "W_Kr_Gsp_Element",
    "W_Kr_Gsp_Komponente",
];

/// The objects of the geodata (Untergewerk Geo); the others are the interlocking's (ESTW).
const GEO_PART: &[&str] = &[
    "GEO_Kante",
    "GEO_Knoten",
    "GEO_Punkt",
    "Hoehenlinie",
    "Hoehenpunkt",
    "Oertlichkeit",
    "Strecke",
    "TOP_Kante",
    "TOP_Knoten",
    "Ueberhoehung",
    "Ueberhoehungslinie",
];

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

/// An element around what is inside it, also when that is nothing.
fn always(name: &str, inner: String) -> String {
    format!("<{name}>{inner}</{name}>")
}

fn id_el(id: &str) -> String {
    w("Identitaet", Some(id.to_string()))
}

/// A decimal with `d` places, never `-0.000`.
fn fixed(x: f64, d: usize) -> String {
    let s = format!("{x:.d$}");
    match s.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => s,
    }
}

/// A whole number, as PlanPro's speeds, distances in whole metres and counts are.
fn int(x: f64) -> String {
    format!("{}", x.round() as i64)
}

/// A whole number within `lo` … `hi`, else none.
fn int_in(x: Option<f64>, lo: f64, hi: f64) -> Option<String> {
    x.map(f64::round).filter(|v| (lo..=hi).contains(v)).map(int)
}

fn yes(b: bool) -> String {
    (if b { "true" } else { "false" }).to_string()
}

/// A text as PlanPro's patterns `.{1,max}` take it: on one line, cut to `max` characters; none when empty.
fn text(s: Option<&str>, max: usize) -> Option<String> {
    let s = s?.trim();
    (!s.is_empty()).then(|| s.chars().map(|c| if c.is_control() { ' ' } else { c }).take(max).collect())
}

// ── reading the document ───────────────────────────────────────────────────

fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn num_of(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// A label of the model, which may be a text or a number (`lineNumber`, `trackNumber`).
fn label_of(v: &Value, k: &str) -> Option<String> {
    match v.get(k)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn point(v: Option<&Value>) -> Option<(f64, f64)> {
    let a = v?.as_array()?;
    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
}

/// The states of the plan an object is in (start, target), from its
/// lifespan: the stages it begins and ends in (core.fields.stages), the start
/// state being the first stage, the target the second.
struct Stages(Vec<String>);

impl Stages {
    fn of(&self, v: &Value) -> (bool, bool) {
        let Some(l) = v.get("lifespan").filter(|l| l.is_object()) else {
            return (true, true);
        };
        let at = |k: &str| -> Option<usize> {
            let s = str_of(l, k)?;
            Some(self.0.iter().position(|x| x == s).unwrap_or(usize::MAX))
        };
        let (from, to) = (at("from"), at("to"));
        let exists = |i: usize| from.is_none_or(|f| f <= i) && to.is_none_or(|t| t > i);
        (exists(0), exists(1))
    }
}

// ── the plan being written ─────────────────────────────────────────────────

struct Item {
    tag: &'static str,
    xml: String,
}

struct Plan {
    start: Vec<Item>,
    target: Vec<Item>,
    counts: BTreeMap<String, usize>,
    notes: Vec<Value>,
    /// `Datum_Regelwerk` of every object: the date of the rules it is planned by.
    rules_date: String,
}

impl Plan {
    /// An object with its identity and base, in the states it is in.
    fn put(&mut self, tag: &'static str, id: &str, body: String, (start, target): (bool, bool)) {
        if !start && !target {
            return;
        }
        let base = group("Basis_Objekt_Allg", w("Datum_Regelwerk", Some(self.rules_date.clone())));
        let xml = format!("<{tag}>{}{base}{body}</{tag}>", id_el(id));
        if start && target {
            self.start.push(Item { tag, xml: xml.clone() });
            self.target.push(Item { tag, xml });
        } else if start {
            self.start.push(Item { tag, xml });
        } else {
            self.target.push(Item { tag, xml });
        }
        *self.counts.entry(tag.to_string()).or_default() += 1;
    }

    fn note(&mut self, code: &str, params: &[(&str, &str)]) {
        let mut p = Map::new();
        p.insert("code".into(), json!(code));
        for (k, v) in params {
            p.insert((*k).into(), json!(v));
        }
        self.notes.push(Value::Object(p));
    }

    /// Something of the model PlanPro has no place for here.
    fn not_written(&mut self, collection: &str, id: &str, why: &str) {
        self.note("not_written", &[("collection", collection), ("id", id), ("why", why)]);
    }

    /// A value PlanPro requires that the model does not say: written as stated.
    fn defaulted(&mut self, collection: &str, id: &str, field: &str, value: &str) {
        self.note("defaulted", &[("collection", collection), ("id", id), ("field", field), ("value", value)]);
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

impl Edge {
    /// The track's own length [m], in its stations.
    fn track_length(&self) -> f64 {
        self.length / self.scale
    }
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

/// A GEO_Kante to write: its form, end points in the plan's plane, length,
/// radii and bearing there.
struct GeoPiece {
    form: &'static str,
    to: (f64, f64),
    length: f64,
    radius_a: Option<String>,
    radius_b: Option<String>,
    gon: f64,
}

/// A radius as PlanPro writes it: signed (+ a right-hand curve, as the model
/// has it), 10 … 99 999.99999 m, `0.00000` for none (a straight end).
fn radius(r: Option<f64>, scale: f64) -> String {
    match r.filter(|r| *r != 0.0 && r.is_finite()).map(|r| r * scale) {
        Some(r) if r.abs() < 100_000.0 => fixed(r.signum() * r.abs().max(10.0), 5),
        _ => "0.00000".into(),
    }
}

/// An element of a track as the GEO_Kanten it is written as — one, or a long
/// straight cut into pieces PlanPro takes.
fn geo_pieces(el: &Value, track: &str, epsg: i64, strip: i64) -> Result<Vec<GeoPiece>, String> {
    let fail = |e: rail_across_geom::coords::Unsupported| e.0;
    let (Some(s), Some(e)) = (point(el.get("startNode")), point(el.get("endNode"))) else {
        return Err(format!("planpro: an element of track {track} has no end points"));
    };
    let sg = transform_plane_point(s.0, s.1, epsg, strip).map_err(fail)?;
    let eg = transform_plane_point(e.0, e.1, epsg, strip).map_err(fail)?;
    let chord = (e.0 - s.0).hypot(e.1 - s.1);
    let scale = if chord > 1e-9 { (eg.0 - sg.0).hypot(eg.1 - sg.1) / chord } else { 1.0 };
    let length = num_of(el, "length").unwrap_or(chord) * scale;
    let bearing = transform_grid_bearing(s.0, s.1, num_of(el, "bearing").unwrap_or(0.0), epsg, strip).map_err(fail)?;
    let gon = ((bearing / GON) % 400.0 + 400.0) % 400.0;
    let kind = num_of(el, "elementType").unwrap_or(0.0) as i64;
    let arc = num_of(el, "radius").filter(|r| *r != 0.0 && (r * scale).abs() < 100_000.0);
    let (form, radius_a, radius_b) = match kind {
        1 if arc.is_some() => ("Bogen", Some(radius(arc, scale)), None),
        2 => (
            if str_of(el, "transitionType") == Some("bloss") {
                "Blosskurve"
            } else {
                "Klothoide"
            },
            Some(radius(num_of(el, "r1"), scale)),
            Some(radius(num_of(el, "r2"), scale)),
        ),
        _ => ("Gerade", None, None),
    };
    if form != "Gerade" || length <= MAX_GEO_LENGTH {
        return Ok(vec![GeoPiece {
            form,
            to: eg,
            length,
            radius_a,
            radius_b,
            gon,
        }]);
    }
    let n = (length / MAX_GEO_LENGTH).ceil() as usize;
    let at = |k: usize| {
        let f = k as f64 / n as f64;
        (sg.0 + (eg.0 - sg.0) * f, sg.1 + (eg.1 - sg.1) * f)
    };
    Ok((0..n)
        .map(|k| GeoPiece {
            form,
            to: at(k + 1),
            length: length / n as f64,
            radius_a: None,
            radius_b: None,
            gon,
        })
        .collect())
}

/// How a point object stands beside its edge: on a rail (`Seitliche_Lage`),
/// at a distance from the track axis — right of the direction A–B positive —
/// (`Seitlicher_Abstand`), or neither.
#[derive(Clone, Copy)]
enum Lateral {
    None,
    Side(&'static str),
    Offset(f64),
}

/// Where a point object stands: `Punkt_Objekt_TOP_Kante`.
fn place(edge: &Edge, station: f64, wirk: Option<&str>, lateral: Lateral) -> String {
    let lateral = match lateral {
        Lateral::None => String::new(),
        Lateral::Side(s) => w("Seitliche_Lage", Some(s.into())),
        Lateral::Offset(o) => w("Seitlicher_Abstand", Some(fixed(o, 3))),
    };
    group(
        "Punkt_Objekt_TOP_Kante",
        [
            w("Abstand", Some(fixed((station * edge.scale).clamp(0.0, edge.length), 3))),
            w("ID_TOP_Kante", Some(edge.guid.clone())),
            w("Wirkrichtung", wirk.map(String::from)),
            lateral,
        ]
        .concat(),
    )
}

/// A part of an edge between two stations: `Bereich_Objekt_Teilbereich`.
fn part(edge: &Edge, from: f64, to: f64) -> String {
    let at = |s: f64| fixed((s * edge.scale).clamp(0.0, edge.length), 3);
    group(
        "Bereich_Objekt_Teilbereich",
        [
            w("Begrenzung_A", Some(at(from.min(to)))),
            w("Begrenzung_B", Some(at(from.max(to)))),
            w("ID_TOP_Kante", Some(edge.guid.clone())),
        ]
        .concat(),
    )
}

/// The whole of an edge.
fn whole(edge: &Edge) -> String {
    part(edge, 0.0, edge.track_length())
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

impl At<'_> {
    /// How it stands beside the track: at its offset on its side where the
    /// model states one, else on its side.
    fn lateral(&self) -> Lateral {
        match (self.side, self.offset) {
            (side, Some(o)) => Lateral::Offset(if side == Some("links") { -o.abs() } else { o.abs() }),
            (Some(s), None) => Lateral::Side(s),
            _ => Lateral::None,
        }
    }

    fn place(&self, wirk: Option<&str>) -> String {
        place(self.edge, self.station, wirk.or(self.direction), self.lateral())
    }
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

/// An area number as PlanPro takes it (`Kennzahl`: 01 … 99).
fn area_number(v: &Value) -> Option<String> {
    str_of(v, "areaNumber")
        .filter(|k| k.len() == 2 && k.bytes().all(|b| b.is_ascii_digit()) && *k != "00")
        .map(String::from)
}

/// The `Bezeichnung` of an element (CBezeichnung_Element): its short name on
/// the plan, its name in the tables — which PlanPro requires — and its area
/// number; nothing without a name.
fn element_naming(short: Option<&str>, table: Option<&str>, area: Option<String>) -> String {
    let Some(table) = text(table.or(short), 36) else { return String::new() };
    group(
        "Bezeichnung",
        [
            w("Bezeichnung_Lageplan_Kurz", text(short, 36)),
            w("Bezeichnung_Tabelle", Some(table)),
            w("Kennzahl", area),
        ]
        .concat(),
    )
}

fn gid(v: &Value) -> String {
    planpro_id(str_of(v, "id").unwrap_or(""))
}

/// A reference to an object that was written, by its id in the model.
fn gref_in(v: &Value, k: &str, written: &HashSet<String>) -> Option<String> {
    str_of(v, k).filter(|id| written.contains(*id)).map(planpro_id)
}

/// The text of a date (`YYYY-MM-DD`) at the start of a time stamp.
fn date_of(stamp: Option<&str>) -> Option<String> {
    let d = stamp?.get(..10)?;
    let ok = d
        .bytes()
        .enumerate()
        .all(|(i, b)| if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() });
    ok.then(|| d.to_string())
}

/// The abbreviation of an operational point as PlanPro takes it (`[A-Z][A-Z0-9 ]{1,4}`).
fn abbreviation(rl100: Option<&str>) -> Option<String> {
    let s = rl100?.trim().to_uppercase();
    let mut chars = s.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && (2..=5).contains(&s.chars().count())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == ' ');
    ok.then_some(s)
}

// ── the extent of track sections ───────────────────────────────────────────

/// The tracks cut at every detection point into pieces, joined where tracks
/// meet: each group of pieces joined is a stretch the points bound, the
/// extent of the track section bounded by just those points.
struct Stretches {
    /// (track, from, to, group) of every piece.
    pieces: Vec<(String, f64, f64, usize)>,
    /// The detection points bounding each group.
    bounds: Vec<Vec<String>>,
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn stretches(order: &[String], edges: &HashMap<String, Edge>, node_count: usize, points: &[(String, String, f64)]) -> Stretches {
    let mut cuts: HashMap<&str, Vec<(f64, &str)>> = HashMap::new();
    for (id, track, station) in points {
        cuts.entry(track.as_str()).or_default().push((*station, id.as_str()));
    }
    // Piece k is node_count + k in the union of pieces and nodes.
    let mut raw: Vec<(String, f64, f64, Vec<String>)> = Vec::new();
    let mut parent: Vec<usize> = (0..node_count).collect();
    for t in order {
        let e = &edges[t];
        let len = e.track_length();
        let mut cs = cuts.get(t.as_str()).cloned().unwrap_or_default();
        cs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut stations: Vec<(f64, Vec<String>)> = Vec::new();
        for (s, id) in cs {
            let s = s.clamp(0.0, len);
            match stations.last_mut() {
                Some(last) if (last.0 - s).abs() < 1e-6 => last.1.push(id.to_string()),
                _ => stations.push((s, vec![id.to_string()])),
            }
        }
        let mut from = (0.0, Vec::new());
        for (s, ids) in stations.into_iter().chain(std::iter::once((len, Vec::new()))) {
            let k = parent.len();
            parent.push(k);
            let mut bounds = from.1.clone();
            bounds.extend(ids.iter().cloned());
            raw.push((t.clone(), from.0, s, bounds));
            if from.0 == 0.0 && from.1.is_empty() {
                let n = find(&mut parent, e.nodes[0]);
                let p = find(&mut parent, k);
                parent[p] = n;
            }
            from = (s, ids);
        }
        // The last piece reaches the END node: a point right there has a piece of no length beyond it.
        let k = parent.len() - 1;
        let n = find(&mut parent, e.nodes[1]);
        let p = find(&mut parent, k);
        parent[p] = n;
    }
    let mut group_of: HashMap<usize, usize> = HashMap::new();
    let mut pieces = Vec::new();
    let mut bounds: Vec<Vec<String>> = Vec::new();
    for (k, (t, a, b, ids)) in raw.into_iter().enumerate() {
        let root = find(&mut parent, node_count + k);
        let n = group_of.len();
        let g = *group_of.entry(root).or_insert(n);
        if g == bounds.len() {
            bounds.push(Vec::new());
        }
        for id in ids {
            if !bounds[g].contains(&id) {
                bounds[g].push(id);
            }
        }
        pieces.push((t, a, b, g));
    }
    Stretches { pieces, bounds }
}

impl Stretches {
    /// The group of pieces a section bounded by these points is: the one
    /// bounded by just them, else the one sharing most of them.
    fn group_of(&self, bounded_by: &[String]) -> Option<usize> {
        let want: HashSet<&String> = bounded_by.iter().collect();
        if want.is_empty() {
            return None;
        }
        let score = |g: usize| {
            let have: HashSet<&String> = self.bounds[g].iter().collect();
            let shared = have.intersection(&want).count() as f64;
            shared / have.union(&want).count().max(1) as f64
        };
        let best = (0..self.bounds.len()).max_by(|a, b| score(*a).total_cmp(&score(*b)))?;
        (score(best) >= 0.5).then_some(best)
    }

    /// The parts of the tracks a group covers.
    fn parts(&self, g: usize) -> Vec<(&str, f64, f64)> {
        let all: Vec<(&str, f64, f64)> = self.pieces.iter().filter(|p| p.3 == g).map(|p| (p.0.as_str(), p.1, p.2)).collect();
        let long: Vec<_> = all.iter().copied().filter(|p| p.2 - p.1 > 1e-6).collect();
        if long.is_empty() { all.into_iter().take(1).collect() } else { long }
    }

    /// The group at a station of a track, inside a piece.
    fn group_at(&self, track: &str, station: f64) -> Option<usize> {
        self.pieces.iter().find(|p| p.0 == track && p.1 < station && station < p.2).map(|p| p.3)
    }
}

// ── planning data ──────────────────────────────────────────────────────────

/// Who planned and how the plan is filed (`options.planning`), with what
/// stands where nothing is given.
struct Planning {
    title: String,
    project_number: String,
    planner: String,
    organisation: String,
    location: Option<String>,
    stage: String,
    index: String,
    number: String,
    informative: bool,
    kind: String,
    date: String,
}

fn two_digits(v: Option<&Value>, default: &str) -> String {
    let n = v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())));
    n.filter(|n| (0.0..=99.0).contains(n))
        .map(|n| format!("{:02}", n as i64))
        .unwrap_or_else(|| default.into())
}

const PLANNING_KINDS: &[&str] = &[
    "Bauzustand",
    "Bestandsdigitalisierung_Geo_Topo",
    "Bestandsdigitalisierung_LST",
    "Bestandskorrektur",
    "Geodaten",
    "Revision",
    "Revision_letzte_Planung_E",
    "sonstige",
    "Verbindungsknoten",
    "Versionshebung",
    "Voraussichtlich",
];

impl Planning {
    fn of(options: &Value, title: &str, project_id: &str, date: &str) -> Planning {
        let p = options.get("planning").filter(|p| p.is_object()).unwrap_or(&Value::Null);
        let given = |k: &str, max: usize| text(str_of(p, k), max);
        let stage = str_of(p, "stage")
            .map(|s| {
                s.chars()
                    .filter(|c| c.is_ascii_alphanumeric() || "äöüÄÖÜ _-,".contains(*c))
                    .take(20)
                    .collect::<String>()
            })
            .filter(|s| !s.trim().is_empty());
        Planning {
            title: text(Some(title), 150).unwrap_or_else(|| "Rail-Across-Suite".into()),
            project_number: given("projectNumber", 20)
                .unwrap_or_else(|| project_id.chars().take(20).collect::<String>())
                .to_string(),
            planner: given("planner", 35).unwrap_or_else(|| "Rail-Across-Suite".into()),
            organisation: given("organisation", 50).unwrap_or_else(|| "Rail-Across-Suite".into()),
            location: abbreviation(str_of(p, "location")),
            stage: stage.unwrap_or_else(|| "Planung".into()),
            index: two_digits(p.get("index"), "01"),
            number: two_digits(p.get("number"), "01"),
            informative: p.get("informative").and_then(Value::as_bool).unwrap_or(true),
            kind: str_of(p, "kind").filter(|k| PLANNING_KINDS.contains(k)).unwrap_or("Bauzustand").into(),
            date: date.into(),
        }
    }

    /// An actor (CAkteur): a person and the organisation they plan for.
    fn actor(&self, id: &str) -> String {
        let short = |n: usize| self.planner.chars().take(n).collect::<String>().trim_end().to_string();
        [
            id_el(&planpro_id(&format!("actor:{id}"))),
            always(
                "Akteur_Allg",
                [
                    w("Name_Akteur", Some(self.planner.clone())),
                    w("Name_Akteur_10", Some(short(10))),
                    w("Name_Akteur_5", Some(short(5))),
                ]
                .concat(),
            ),
            always("Kontaktdaten", self.organisation_xml(id)),
        ]
        .concat()
    }

    fn organisation_xml(&self, id: &str) -> String {
        [
            id_el(&planpro_id(&format!("organisation:{id}"))),
            w("Name_Organisation", Some(self.organisation.clone())),
        ]
        .concat()
    }

    /// A group of the planning for one part of the plan (`LST_Planung_Gruppe`).
    fn group(&self, project: &str, part: &str, data_id: &str, location: &str) -> String {
        let key = format!("{project}:{part}");
        let single = [
            id_el(&planpro_id(&format!("planning-single:{key}"))),
            w("ID_Ausgabe_Fachdaten", Some(data_id.into())),
            always(
                "Planung_E_Allg",
                [
                    w("Bauzustand_Kurzbezeichnung", Some(self.stage.clone())),
                    w("Datum_Abschluss_Einzel", Some(self.date.clone())),
                    w("Datum_Regelwerksstand", Some(self.date.clone())),
                    w("Index_Ausgabe", Some(self.index.clone())),
                    w("Informativ", Some(yes(self.informative))),
                    w("Laufende_Nummer_Ausgabe", Some(self.number.clone())),
                    w("Planung_E_Art", Some(self.kind.clone())),
                ]
                .concat(),
            ),
            always(
                "Planung_E_Handlung",
                always(
                    "Planung_E_Erstellung",
                    [
                        id_el(&planpro_id(&format!("planning-made:{key}"))),
                        w("Datum", Some(self.date.clone())),
                        always("Handelnder", self.actor(&key)),
                    ]
                    .concat(),
                ),
            ),
        ]
        .concat();
        [
            id_el(&planpro_id(&format!("planning-group:{key}"))),
            w("Fuehrende_Oertlichkeit", Some(location.into())),
            always("LST_Planung_Einzel", single),
            always(
                "Planung_G_Allg",
                [
                    w("Datum_Abschluss_Gruppe", Some(self.date.clone())),
                    w("PlanPro_XSD_Version", Some(XSD_VERSION.into())),
                    w("Untergewerk_Art", Some(part.into())),
                ]
                .concat(),
            ),
            always(
                "Planung_G_Schriftfeld",
                [
                    w("Bezeichnung_Anlage", text(Some(&self.title), 35)),
                    w("Bezeichnung_Planung_Gruppe", text(Some(&self.title), 50)),
                    always("Planungsbuero", self.organisation_xml(&key)),
                ]
                .concat(),
            ),
        ]
        .concat()
    }
}

// ── the export ─────────────────────────────────────────────────────────────

/// A project as a PlanPro plan (`writePlanPro`): `{ text, manifest, archive,
/// report: { notes }, counts }` — the plan (`content.xml`), its manifest, the
/// container file of both (`.planpro`, in Base64), what was not written or
/// was filled in and why, and how many objects of each kind were written.
///
/// `project` is an infra-data-1.0 document, or a record of the suite, which is
/// made one first. `options`: `signalling` (the domain as the editor holds it,
/// else the document's), `now` (the time stamp of the file, whose date is the
/// date of the planning), `version` (the suite's), `rulesDate` (the date of the
/// rules every object is planned by, else the planning's), `planning`
/// (`projectNumber`, `planner`, `organisation`, `location` — the leading
/// operational point —, `stage`, `index`, `number`, `informative`, `kind`).
pub fn write_planpro(project: &Value, options: &Value) -> Result<Value, String> {
    let made;
    let doc = if project.get("format").and_then(Value::as_str) == Some("infra-data") {
        project
    } else {
        made = rail_across_doc::door::to_document(project, false, false).map_err(|e| format!("planpro: not a project: {e:?}"))?;
        &made
    };
    let core = &doc["domains"]["core"];
    let fields = &core["fields"];
    let c = |k: &str| core.get("collections").map(|c| list(c, k)).unwrap_or(&[]);
    // The platforms are civil's in infra-data-2.0 (decision 0025); a document of 1.0 had them in the core.
    let civil = &doc["domains"]["civil"];
    let platforms: Vec<&Value> = c("platforms").iter().chain(civil.get("collections").map(|c| list(c, "platforms")).unwrap_or(&[])).collect();
    let stages = Stages(list(fields, "stages").iter().filter_map(|s| str_of(s, "id").map(String::from)).collect());
    let now = str_of(options, "now");
    let date = date_of(now).unwrap_or_else(|| "2026-01-01".into());
    let mut plan = Plan {
        start: vec![],
        target: vec![],
        counts: BTreeMap::new(),
        notes: vec![],
        rules_date: date_of(str_of(options, "rulesDate")).unwrap_or_else(|| date.clone()),
    };
    let tracks = c("tracks");
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
        let (first, last) = (&els[0], &els[els.len() - 1]);
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

    // ── nodes: turnouts, links, end marks, then where ends meet ──
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
    for sw in c("switches") {
        let id = str_of(sw, "id").unwrap_or("");
        let port = |k: &str| {
            let p = sw.get("ports")?.get(k)?;
            let track = str_of(p, "trackId")?;
            let end = endpoint_index(str_of(p, "endpoint"))?;
            edges.contains_key(track).then(|| (track.to_string(), end))
        };
        let kind = str_of(sw, "kind").unwrap_or("turnout");
        if kind == "link" {
            // Two track ends joined: one node, where they meet.
            let ends: Vec<_> = ["A", "B", "B1", "B2"]
                .iter()
                .filter_map(|k| port(k))
                .filter(|e| !node_of.contains_key(e))
                .collect();
            if ends.len() == 2 {
                let k = new_node(&mut nodes, id, edges[&ends[0].0].ends[ends[0].1]);
                for e in ends {
                    node_of.insert(e.clone(), k);
                    nodes[k].ends.push(e);
                }
            }
            continue;
        }
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
    for mark in c("endMarks") {
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
    let geo_node = |plan: &mut Plan, id: &str, (e, n): (f64, f64), state: (bool, bool)| {
        plan.put("GEO_Knoten", id, String::new(), state);
        let allg = [
            w("GEO_Koordinatensystem", Some(system.into())),
            w("GK_X", Some(fixed(e, 5))),
            w("GK_Y", Some(fixed(n, 5))),
            w("Plan_Quelle", Some(SOURCE.into())),
        ]
        .concat();
        plan.put(
            "GEO_Punkt",
            &planpro_id(&format!("point:{id}")),
            [always("GEO_Punkt_Allg", allg), w("ID_GEO_Knoten", Some(id.into()))].concat(),
            state,
        );
    };
    for n in &nodes {
        geo_node(&mut plan, &n.geo, n.at, (true, true));
        plan.put("TOP_Knoten", &n.guid, w("ID_GEO_Knoten", Some(n.geo.clone())), (true, true));
    }

    // The edges with their geometry.
    for t in tracks {
        let id = str_of(t, "id").unwrap_or("");
        let Some(edge_nodes) = edges.get(id).map(|e| e.nodes) else { continue };
        let epsg = num_of(t, "epsg").unwrap_or(0.0) as i64;
        let state = stages.of(t);
        let guid = planpro_id(id);
        let mut pieces = Vec::new();
        let mut plain = 0.0;
        for el in list(t, "elements") {
            pieces.extend(geo_pieces(el, id, epsg, strip)?);
            plain += num_of(el, "length").unwrap_or(0.0);
        }
        let mut length = 0.0;
        for (i, p) in pieces.iter().enumerate() {
            let a = if i == 0 {
                nodes[edge_nodes[0]].geo.clone()
            } else {
                planpro_id(&format!("geo:{guid}:{i}"))
            };
            let b = if i + 1 == pieces.len() {
                nodes[edge_nodes[1]].geo.clone()
            } else {
                let b = planpro_id(&format!("geo:{guid}:{}", i + 1));
                geo_node(&mut plan, &b, p.to, state);
                b
            };
            let gon = fixed(p.gon, 5);
            let allg = [
                w("GEO_Form", Some(p.form.into())),
                w("GEO_Laenge", Some(fixed(p.length, 5))),
                w("GEO_Radius_A", p.radius_a.clone()),
                w("GEO_Radius_B", p.radius_b.clone()),
                w("GEO_Richtungswinkel", Some(if gon == "400.00000" { "0.00000".into() } else { gon })),
                w("Plan_Quelle", Some(SOURCE.into())),
            ]
            .concat();
            let body = [
                always("GEO_Kante_Allg", allg),
                w("ID_GEO_Art", Some(guid.clone())),
                w("ID_GEO_Knoten_A", Some(a)),
                w("ID_GEO_Knoten_B", Some(b)),
            ]
            .concat();
            plan.put("GEO_Kante", &planpro_id(&format!("geo-edge:{guid}:{i}")), body, state);
            length += p.length;
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
            w("TOP_Laenge", Some(fixed(length, 3))),
        ]
        .concat();
        let body = [
            w("ID_TOP_Knoten_A", Some(nodes[edge_nodes[0]].guid.clone())),
            w("ID_TOP_Knoten_B", Some(nodes[edge_nodes[1]].guid.clone())),
            always("TOP_Kante_Allg", allg),
        ]
        .concat();
        plan.put("TOP_Kante", &guid, body, state);
        let e = edges.get_mut(id).unwrap();
        e.length = (length * 1000.0).round() / 1000.0;
        e.scale = if plain > 0.0 { length / plain } else { 1.0 };
    }
    let end_station = |e: &Edge, end: usize| if end == 0 { 0.0 } else { e.track_length() };

    // ── what lies along the tracks: names, kinds, lines, heights, cant ──
    let mut lines: BTreeMap<String, Vec<&Edge>> = BTreeMap::new();
    let mut kinds: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
    for t in tracks {
        let id = str_of(t, "id").unwrap_or("");
        let Some(edge) = edges.get(id) else { continue };
        let state = stages.of(t);
        if let Some(name) = label_of(t, "trackNumber").or_else(|| label_of(t, "name")) {
            let body = [whole(edge), always("Bezeichnung", w("Bez_Gleis_Bezeichnung", Some(name)))].concat();
            plan.put("Gleis_Bezeichnung", &planpro_id(&format!("name:{}", edge.guid)), body, state);
        }
        let kind = match (num_of(t, "trackType").map(|k| k as i64), str_of(t, "trackUse")) {
            (_, Some("siding")) => Some("Nebengleis"),
            (Some(1), _) => Some("Streckengleis"),
            (Some(2), _) => Some("Hauptgleis"),
            _ => None,
        };
        if let Some(k) = kind {
            kinds.entry(k).or_default().push(edge);
        }
        if let Some(line) = label_of(t, "lineNumber") {
            lines.entry(line).or_default().push(edge);
        }
        heights_of(&mut plan, t, edge, state);
        cant_of(&mut plan, t, edge, state);
    }
    for (kind, list) in &kinds {
        let body = [list.iter().map(|e| whole(e)).collect::<String>(), w("Gleisart", Some((*kind).into()))].concat();
        plan.put("Gleis_Art", &planpro_id(&format!("track-kind:{kind}")), body, (true, true));
    }
    for (line, list) in &lines {
        let valid = line.len() >= 4
            && line.bytes().take(4).all(|b| b.is_ascii_digit())
            && (line.len() == 4 || (line.len() == 6 && &line[4..5] == "-" && line[5..].bytes().all(|b| b.is_ascii_alphanumeric())));
        if !valid {
            plan.not_written("lines", line, "line_number_not_planpro");
            continue;
        }
        let body = [
            list.iter().map(|e| whole(e)).collect::<String>(),
            always("Bezeichnung", w("Bezeichnung_Strecke", Some(line.clone()))),
        ]
        .concat();
        plan.put("Strecke", &planpro_id(&format!("line:{line}")), body, (true, true));
    }

    // Operational points: a station where station tracks are of it, a halt where only platforms are.
    let mut leading: Vec<(usize, String)> = Vec::new();
    for p in c("operationalPoints") {
        let id = str_of(p, "id").unwrap_or("");
        let Some(abbr) = abbreviation(str_of(p, "rl100")) else {
            plan.not_written("operationalPoints", id, "no_rl100");
            continue;
        };
        let refs_tracks = tracks
            .iter()
            .filter(|t| str_of(t, "operationalPoint") == Some(id) && num_of(t, "trackType") == Some(2.0))
            .count();
        let refs_platforms = platforms.iter().filter(|x| str_of(x, "operationalPoint") == Some(id)).count();
        let art = if refs_tracks > 0 {
            "Bf"
        } else if refs_platforms > 0 {
            "Hp"
        } else {
            "sonstige"
        };
        leading.push((refs_tracks * 10 + refs_platforms, abbr.clone()));
        let name = str_of(p, "name");
        let body = [
            always(
                "Bezeichnung",
                [
                    w("Oertlichkeit_Abkuerzung", Some(abbr.clone())),
                    w("Oertlichkeit_Kurzname", text(name, 16)),
                    w("Oertlichkeit_Langname", text(name, 40).or(Some(abbr))),
                ]
                .concat(),
            ),
            always("Oertlichkeit_Allg", w("Oertlichkeit_Art", Some(art.into()))),
        ]
        .concat();
        plan.put("Oertlichkeit", &planpro_id(id), body, stages.of(p));
    }

    // Buffer stops: in the middle of the track.
    for (track, end, mark) in &buffer_stops {
        let e = &edges[track];
        let body = [
            place(e, end_station(e, *end), Some(if *end == 0 { "gegen" } else { "in" }), Lateral::Offset(0.0)),
            w("Gleis_Abschluss_Art", Some("Festprellbock".into())),
        ]
        .concat();
        plan.put(
            "Gleis_Abschluss",
            &planpro_id(&format!("stop:{}", str_of(mark, "id").unwrap_or(""))),
            body,
            stages.of(mark),
        );
    }

    // Platforms: an installation per operational point, an edge per platform.
    let mut installations: Vec<(Option<&str>, Vec<&Value>)> = Vec::new();
    for p in platforms.iter().copied() {
        let op = str_of(p, "operationalPoint");
        match installations.iter_mut().find(|(o, _)| *o == op) {
            Some((_, l)) => l.push(p),
            None => installations.push((op, vec![p])),
        }
    }
    let point_name = |id: Option<&str>| {
        c("operationalPoints")
            .iter()
            .find(|p| str_of(p, "id") == id)
            .and_then(|p| str_of(p, "name").or(str_of(p, "rl100")))
    };
    for (op, platforms) in &installations {
        let anlage = planpro_id(&format!("platforms:{}", op.unwrap_or("")));
        let mut written = 0;
        for (k, p) in platforms.iter().enumerate() {
            let id = str_of(p, "id").unwrap_or("");
            let at = p.get("at").unwrap_or(p);
            let (Some(edge), Some(a), Some(b)) = (
                str_of(at, "trackId").and_then(|t| edges.get(t)),
                num_of(at, "startStation"),
                num_of(at, "endStation"),
            ) else {
                plan.not_written("platforms", id, "no_track");
                continue;
            };
            let side = str_of(at, "side").and_then(m::side_key).unwrap_or_else(|| {
                plan.defaulted("platforms", id, "Lage_Zum_Gleis", "rechts");
                "rechts"
            });
            let height = num_of(p, "height").map(|h| match h.round() as i64 {
                380 => "38_cm",
                550 => "55_cm",
                760 => "76_cm",
                960 => "96_cm",
                _ => "sonstige",
            });
            let name = str_of(p, "code").map(String::from).unwrap_or_else(|| format!("{}", k + 1));
            let body = [
                part(edge, a, b),
                always(
                    "Bahnsteig_Kante_Allg",
                    [w("Lage_Zum_Gleis", Some(side.into())), w("Systemhoehe", height.map(String::from))].concat(),
                ),
                always("Bezeichnung", w("Bezeichnung_Bahnsteig_Kante", text(Some(&name), 36))),
                w("ID_Bahnsteig_Anlage", Some(anlage.clone())),
            ]
            .concat();
            plan.put("Bahnsteig_Kante", &planpro_id(id), body, stages.of(p));
            written += 1;
        }
        if written > 0 {
            let body = group("Bezeichnung", w("Bezeichnung_Bahnsteig_Anlage", text(point_name(*op), 20)));
            plan.put("Bahnsteig_Anlage", &anlage, body, (true, true));
        }
    }

    // ── signalling ──
    let empty = json!({});
    let domain = options
        .get("signalling")
        .filter(|d| d.is_object())
        .or(doc["domains"].get("signalling"))
        .unwrap_or(&empty);
    let col = |k: &str| domain.get("collections").map(|c| list(c, k)).unwrap_or(&[]);
    let ids_of = |k: &str| -> HashSet<String> { col(k).iter().filter_map(|v| str_of(v, "id").map(String::from)).collect() };

    // Switches and their elements: every turnout has one.
    let mut element_of_switch: HashMap<&str, &Value> = HashMap::new();
    for el in col("switchElements") {
        if let Some(sw) = str_of(el, "switchId") {
            element_of_switch.entry(sw).or_insert(el);
        }
    }
    let mut written_elements: HashSet<String> = HashSet::new();
    let mut element_guids: Vec<(String, String)> = Vec::new();
    let signal_ids = ids_of("signals");
    for &(k, sw) in &turnouts {
        let sid = str_of(sw, "id").unwrap_or("");
        let node = &nodes[k];
        let anlage = planpro_id(&format!("installation:{}", node.guid));
        let state = stages.of(sw);
        // The form as PlanPro names it (`EW 54-300-1:9`): the label, the kind of turnout before it where it has none.
        let form = str_of(sw, "label").map(|l| {
            let l = l.replace(['–', '—'], "-");
            match l.split_whitespace().next().filter(|k| m::switch_kind(k).is_some()) {
                Some(_) => l,
                None => format!("EW {}", l.split_whitespace().collect::<String>()),
            }
        });
        let form = text(form.as_deref(), 50).unwrap_or_else(|| {
            plan.defaulted("switches", sid, "W_Kr_Grundform", "EW");
            "EW".into()
        });
        let allg = [w("W_Kr_Art", Some("EW".into())), w("W_Kr_Grundform", Some(form))].concat();
        plan.put("W_Kr_Anlage", &anlage, always("W_Kr_Anlage_Allg", allg), state);
        let element = element_of_switch.get(sid).copied();
        let eid = element.map(gid).unwrap_or_else(|| planpro_id(&format!("element:{}", node.guid)));
        let estate = element.map(|e| stages.of(e)).unwrap_or(state);
        let table = str_of(sw, "name")
            .or(element.and_then(|e| str_of(e, "name")))
            .map(String::from)
            .unwrap_or_else(|| {
                let n = num_of(sw, "number")
                    .map(|n| format!("W{}", n as i64))
                    .unwrap_or_else(|| format!("W {}", sid.chars().take(8).collect::<String>()));
                plan.defaulted("switches", sid, "Bezeichnung_Tabelle", &n);
                n
            });
        let operation = element.and_then(|e| str_of(e, "operation"));
        let stellart = operation.and_then(m::operation_key).unwrap_or_else(|| {
            plan.defaulted("switches", sid, "W_Kr_Gsp_Stellart", "sonstige");
            "sonstige"
        });
        let allg = [
            w(
                "Vorzugslage_Automatik",
                element.and_then(|e| e.get("preferredAutomatic")).and_then(Value::as_bool).map(yes),
            ),
            w("W_Kr_Gsp_Stellart", Some(stellart.into())),
        ]
        .concat();
        let weiche = element.map_or(String::new(), |e| {
            [
                w("Auffahrortung", e.get("trailable").and_then(Value::as_bool).map(yes)),
                w("ID_Grenzzeichen", gref_in(e, "clearanceMarkerId", &signal_ids)),
                w("Weiche_Vorzugslage", str_of(e, "preferredPosition").and_then(m::position_key).map(String::from)),
            ]
            .concat()
        });
        let body = [
            element_naming(element.and_then(|e| str_of(e, "name")), Some(&table), element.and_then(area_number)),
            w("ID_W_Kr_Anlage", Some(anlage)),
            always("W_Kr_Gsp_Element_Allg", allg),
            always("Weiche_Element", weiche),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Element", &eid, body, estate);
        if let Some(e) = element {
            written_elements.insert(str_of(e, "id").unwrap_or("").to_string());
        }
        element_guids.push((sid.to_string(), eid.clone()));
        // Its component at the tip, on which side the straight route goes on, the speeds over both.
        let (tip, end) = node.ends.iter().find(|e| node.conn.get(*e) == Some(&"Spitze")).cloned().unwrap();
        let te = &edges[&tip];
        let port = |k: &str| -> Option<(String, usize)> {
            let p = sw.get("ports")?.get(k)?;
            Some((str_of(p, "trackId")?.to_string(), endpoint_index(str_of(p, "endpoint"))?))
        };
        let trunk = if port("B2").and_then(|key| node.conn.get(&key).copied()) == Some("Links") {
            "links"
        } else {
            "rechts"
        };
        let design = |key: Option<(String, usize)>| -> Option<f64> {
            let (t, end) = key?;
            let els = list(tracks.iter().find(|x| str_of(x, "id") == Some(t.as_str()))?, "elements");
            num_of(if end == 0 { els.first()? } else { els.last()? }, "designSpeed")
        };
        let mut speed = |v: Option<f64>, field: &str| -> String {
            int_in(v, 0.0, 500.0).unwrap_or_else(|| {
                plan.defaulted("switches", sid, field, "0");
                "0".into()
            })
        };
        let straight = speed(design(port("B2")).or(design(port("A"))), "Geschwindigkeit_Stammgleis");
        let branch = speed(num_of(sw, "speed").or(design(port("B1"))), "Geschwindigkeit_Zweiggleis");
        let (left, right) = if trunk == "links" { (straight, branch) } else { (branch, straight) };
        let drives = match operation {
            Some("electricRemote" | "electricLocal" | "springBack") => "1",
            Some(_) => "0",
            None => "1",
        };
        let tongues = [
            w("Elektrischer_Antrieb_Anzahl", Some(drives.into())),
            w("Geschwindigkeit_L", Some(left)),
            w("Geschwindigkeit_R", Some(right)),
            w("Stammgleis", Some(trunk.into())),
            w("Zungenpruefkontakt_Anzahl", Some("0".into())),
        ]
        .concat();
        let body = [
            place(te, end_station(te, end), None, Lateral::None),
            w("ID_W_Kr_Gsp_Element", Some(eid.clone())),
            always("Zungenpaar", tongues),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Komponente", &planpro_id(&format!("component:{eid}")), body, estate);
    }
    // Derailers, and elements whose switch was not written.
    for e in col("switchElements") {
        let id = str_of(e, "id").unwrap_or("");
        if written_elements.contains(id) {
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
        let state = stages.of(e);
        let name = str_of(e, "name").map(String::from).unwrap_or_else(|| {
            let n = format!("Gs {}", id.chars().take(8).collect::<String>());
            plan.defaulted("switchElements", id, "Bezeichnung_Tabelle", &n);
            n
        });
        let stellart = str_of(e, "operation").and_then(m::operation_key).unwrap_or_else(|| {
            plan.defaulted("switchElements", id, "W_Kr_Gsp_Stellart", "sonstige");
            "sonstige"
        });
        let allg = [
            w("Vorzugslage_Automatik", e.get("preferredAutomatic").and_then(Value::as_bool).map(yes)),
            w("W_Kr_Gsp_Stellart", Some(stellart.into())),
        ]
        .concat();
        let body = [
            element_naming(str_of(e, "name"), Some(&name), area_number(e)),
            always("W_Kr_Gsp_Element_Allg", allg),
            always(
                "Gleissperre_Element",
                w(
                    "Gleissperre_Vorzugslage",
                    str_of(d, "preferredPosition").and_then(m::derailer_position_key).map(String::from),
                ),
            ),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Element", &eid, body, state);
        written_elements.insert(id.to_string());
        let body = [
            at.place(None),
            w("ID_W_Kr_Gsp_Element", Some(eid.clone())),
            always("Entgleisungsschuh", w("Schutzschiene", Some(yes(false)))),
        ]
        .concat();
        plan.put("W_Kr_Gsp_Komponente", &planpro_id(&format!("component:{eid}")), body, state);
    }

    // Signals, their frames and aspects; what a route shows is found by aspect.
    let mut aspect_ids: HashMap<(String, String, Option<String>), String> = HashMap::new();
    let mut speeds: HashMap<String, f64> = HashMap::new();
    let mut signal_at: HashMap<String, (String, f64, Option<&'static str>)> = HashMap::new();
    let mut written_signals: HashSet<String> = HashSet::new();
    for sig in col("signals") {
        let id = str_of(sig, "id").unwrap_or("").to_string();
        let Some(at) = at_of(sig.get("at"), &edges) else {
            plan.not_written("signals", &id, "no_track");
            continue;
        };
        let guid = gid(sig);
        let state = stages.of(sig);
        signal_at.insert(id.clone(), (at.track.clone(), at.station, at.direction));
        written_signals.insert(id.clone());
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
        let real = str_of(sig, "kind") != Some("virtual");
        let both = sig.get("bothDirections").and_then(Value::as_bool) == Some(true);
        let mut places = place(at.edge, at.station, if both { Some("beide") } else { at.direction }, at.lateral());
        for also in list(sig, "alsoAt") {
            if let Some(a) = at_of(Some(also), &edges) {
                places.push_str(&a.place(None));
            }
        }
        let body = if real {
            let function = first.and_then(|l| str_of(l, "function")).or(str_of(sig, "function"));
            let system = first.and_then(|l| str_of(l, "system"));
            let kind = first.and_then(|l| str_of(l, "type")).and_then(m::signal_type_key);
            let sight = sig.get("sight");
            let sight = |k: &str| sight.and_then(|s| num_of(s, k));
            let screen = kind.map(|kind| {
                let aim = int_in(sight("required"), 1.0, 500.0).unwrap_or_else(|| {
                    plan.defaulted("signals", &id, "Richtpunktentfernung", "200");
                    "200".into()
                });
                [
                    w("Richtpunktentfernung", Some(aim)),
                    w("Signal_Art", Some(kind.into())),
                    w("Signalsystem", Some(system.and_then(m::signal_system_key).unwrap_or("sonstige").into())),
                ]
                .concat()
            });
            let mounting = str_of(sig, "mounting").and_then(m::mounting_key).unwrap_or_else(|| {
                plan.defaulted("signals", &id, "Signal_Befestigungsart", "andere");
                "andere"
            });
            let inner = [
                w("Signal_Befestigungsart", Some(mounting.into())),
                w("Signal_Funktion", function.and_then(m::signal_function_key).map(String::from)),
                group("Signal_Real_Aktiv_Schirm", screen.unwrap_or_default()),
                w("Signalsicht_Erreichbar", int_in(sight("achievable"), 1.0, 500.0)),
                w("Signalsicht_Mindest", int_in(sight("minimum"), 1.0, 300.0)),
                w("Signalsicht_Soll", int_in(sight("required"), 1.0, 500.0)),
            ]
            .concat();
            always("Signal_Real", inner)
        } else {
            let f = str_of(sig, "virtualFunction").and_then(m::virtual_function_key).unwrap_or("sonstige");
            always("Signal_Fiktiv", w("Fiktives_Signal_Funktion", Some(f.into())))
        };
        let naming = element_naming(str_of(sig, "name"), str_of(sig, "name"), area_number(sig));
        plan.put("Signal", &guid, [places, naming, body].concat(), state);
        if !real {
            continue;
        }
        let shown = first.map(|l| list(l, "frames")).unwrap_or(&[]);
        for (i, frame) in shown.iter().chain(list(sig, "frames")).enumerate() {
            let fid = planpro_id(&format!("frame:{guid}:{i}"));
            let art = str_of(frame, "kind").and_then(m::frame_key).unwrap_or("sonstige");
            plan.put(
                "Signal_Rahmen",
                &fid,
                [w("ID_Signal", Some(guid.clone())), w("Rahmen_Art", Some(art.into()))].concat(),
                state,
            );
            for (j, a) in list(frame, "aspects").iter().enumerate() {
                let Some(aspect) = str_of(a, "aspect") else { continue };
                let symbol_value = a
                    .get("symbol")
                    .filter(|s| !s.is_null())
                    .map(|s| s.as_str().map(String::from).unwrap_or_else(|| s.to_string()));
                let Some(entry) = aspect_of(aspect) else {
                    plan.not_written("aspects", &id, "aspect_not_planpro");
                    continue;
                };
                if !symbol_fits(entry, symbol_value.as_ref().map(|s| json!(s)).as_ref()) {
                    plan.not_written("aspects", &id, "symbol_not_planpro");
                    continue;
                }
                let aid = planpro_id(&format!("aspect:{guid}:{i}:{j}"));
                aspect_ids
                    .entry((id.clone(), aspect.to_string(), symbol_value.clone()))
                    .or_insert_with(|| aid.clone());
                let allg = [
                    w(
                        "Beleuchtet",
                        a.get("lit")
                            .and_then(Value::as_bool)
                            .map(|b| (if b { "innenbeleuchtet" } else { "nein" }).to_string()),
                    ),
                    w("Geschaltet", a.get("switched").and_then(Value::as_bool).map(yes)),
                ]
                .concat();
                let mut sid = format!("<Signalbegriff_ID xsi:type=\"nsSignalbegriffe_Ril_301:{}\">", esc(aspect));
                if let Some(s) = &symbol_value {
                    let _ = write!(sid, "<Symbol>{}</Symbol>", esc(s));
                }
                sid.push_str("</Signalbegriff_ID>");
                let body = [w("ID_Signal_Rahmen", Some(fid.clone())), group("Signal_Signalbegriff_Allg", allg), sid].concat();
                plan.put("Signal_Signalbegriff", &aid, body, state);
            }
        }
    }

    // Train detection: the points, cut into the net, give each section its extent.
    let placed: Vec<(String, String, f64)> = col("detectionPoints")
        .iter()
        .filter_map(|p| {
            let at = at_of(p.get("at"), &edges)?;
            Some((str_of(p, "id")?.to_string(), at.track, at.station))
        })
        .collect();
    let stretch = stretches(&order, &edges, nodes.len(), &placed);
    let mut section_group: HashMap<String, usize> = HashMap::new();
    let mut group_section: HashMap<usize, String> = HashMap::new();
    for s in col("trackSections") {
        let id = str_of(s, "id").unwrap_or("");
        let bounded: Vec<String> = list(s, "boundedBy").iter().filter_map(Value::as_str).map(String::from).collect();
        match stretch.group_of(&bounded) {
            Some(g) => {
                section_group.insert(id.to_string(), g);
                group_section.entry(g).or_insert_with(|| id.to_string());
            }
            None => plan.not_written("trackSections", id, "no_extent"),
        }
    }
    let written_sections: HashSet<String> = section_group.keys().cloned().collect();
    let aea = planpro_id(&format!("outside-element-control:{}", str_of(fields, "id").unwrap_or("")));
    let mut needs_aea = false;
    let mut bounds: HashMap<&str, Vec<String>> = HashMap::new();
    for s in col("trackSections") {
        let id = str_of(s, "id").unwrap_or("");
        if !written_sections.contains(id) {
            continue;
        }
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
        let limits = bounds.get(id).cloned().unwrap_or_default();
        if limits.is_empty() {
            plan.not_written("detectionPoints", id, "no_section");
            continue;
        }
        if limits.len() > 4 {
            plan.not_written("detectionPoints.sections", id, "more_than_four");
        }
        let kind = match str_of(p, "kind").and_then(m::detection_point_key) {
            Some(art) => w("FMA_Komponente_Art", Some(art.into())),
            None => {
                needs_aea = true;
                always(
                    "FMA_Komponente_Achszaehlpunkt",
                    [
                        w("FMA_Komponente_Stromversorgung", Some(yes(false))),
                        w("ID_Energie", Some(aea.clone())),
                        w("ID_Information", Some(aea.clone())),
                    ]
                    .concat(),
                )
            }
        };
        let body = [
            at.place(Some("beide")),
            element_naming(str_of(p, "name"), str_of(p, "name"), area_number(p)),
            limits.iter().take(4).map(|s| w("ID_FMAgrenze", Some(s.clone()))).collect(),
            kind,
        ]
        .concat();
        plan.put("FMA_Komponente", &gid(p), body, stages.of(p));
    }
    for s in col("trackSections") {
        let id = str_of(s, "id").unwrap_or("");
        let Some(&g) = section_group.get(id) else { continue };
        let guid = gid(s);
        let state = stages.of(s);
        let abschnitt = planpro_id(&format!("section:{guid}"));
        let name = str_of(s, "name");
        let letter = name
            .filter(|n| n.chars().count() > 1)
            .and_then(|n| n.chars().next())
            .filter(|c| "BGKW".contains(*c));
        let short = match letter {
            Some(_) => name.map(|n| n.chars().skip(1).collect::<String>()),
            None => name.map(String::from),
        };
        let table = name.map(String::from).unwrap_or_else(|| {
            let n = format!("FMA {}", id.chars().take(8).collect::<String>());
            plan.defaulted("trackSections", id, "Bezeichnung_Tabelle", &n);
            n
        });
        let parts: String = stretch.parts(g).into_iter().map(|(t, a, b)| part(&edges[t], a, b)).collect();
        plan.put(
            "Gleis_Abschnitt",
            &abschnitt,
            [parts, element_naming(short.as_deref(), Some(&table), area_number(s))].concat(),
            state,
        );
        let allg = [
            w("FMA_Art", Some(str_of(s, "kind").and_then(m::section_kind_key).unwrap_or("sonstige").into())),
            w(
                "FMA_Hilffreimeldung",
                Some(yes(s.get("auxiliaryClearing").and_then(Value::as_bool) == Some(true))),
            ),
        ]
        .concat();
        needs_aea = true;
        let body = [
            group("Bezeichnung", w("Bezeichnung_Kennbuchstabe", letter.map(String::from))),
            always("FMA_Anlage_Allg", allg),
            w("ID_Gleis_Abschnitt", Some(abschnitt)),
            w("ID_Gleisfreimelde_Innenanlage", Some(aea.clone())),
        ]
        .concat();
        plan.put("FMA_Anlage", &guid, body, state);
    }
    if needs_aea {
        plan.put(
            "Aussenelementansteuerung",
            &aea,
            always("AEA_Allg", w("Aussenelementansteuerung_Art", Some("Gleisfreimelde_Innenanlage".into()))),
            (true, true),
        );
        plan.note(
            "placeholder",
            &[("object", "Aussenelementansteuerung"), ("for", "detectionPoints, trackSections")],
        );
    }
    let section_refs = |v: &Value, k: &str, tag: &str| -> String {
        list(v, k)
            .iter()
            .filter_map(Value::as_str)
            .filter(|s| written_sections.contains(*s))
            .map(|s| w(tag, Some(planpro_id(s))))
            .collect()
    };

    // A run along tracks: from where it starts to where it ends.
    let run_parts = |ids: &[Value], from: Option<(&str, f64, Option<&str>)>, to: Option<(&str, f64)>| -> String {
        let ids: Vec<&str> = ids.iter().filter_map(Value::as_str).filter(|t| edges.contains_key(*t)).collect();
        let n = ids.len();
        ids.iter()
            .enumerate()
            .map(|(i, t)| {
                let e = &edges[*t];
                let full = (0.0, e.track_length());
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
    let mut written_overlaps: HashSet<String> = HashSet::new();
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
        let parts = run_parts(path, Some(start), Some((end.track.as_str(), end.station)));
        if parts.is_empty() {
            plan.not_written("overlaps", id, "no_run");
            continue;
        }
        written_overlaps.insert(id.to_string());
        let guid = gid(o);
        let state = stages.of(o);
        let point_id = planpro_id(&format!("danger-point:{guid}"));
        let mark = planpro_id(&format!("mark:{guid}"));
        plan.put("Sonstiger_Punkt", &point_id, end.place(None), state);
        let mark_name = text(str_of(o, "name"), 30).unwrap_or_else(|| "Gefahrpunkt".into());
        plan.put(
            "Markanter_Punkt",
            &mark,
            [
                always("Bezeichnung", w("Bezeichnung_Markanter_Punkt", Some(mark_name))),
                w("ID_Markante_Stelle", Some(point_id)),
            ]
            .concat(),
            state,
        );
        let run = planpro_id(&format!("run:{guid}"));
        plan.put(
            "Fstr_Fahrweg",
            &run,
            [parts, w("ID_Start", signal.map(planpro_id)), w("ID_Ziel", Some(mark.clone()))].concat(),
            state,
        );
        let length = int_in(num_of(o, "length"), 0.0, 300.0).unwrap_or_else(|| {
            plan.defaulted("overlaps", id, "Laenge_Soll", "0");
            "0".into()
        });
        let gradient = num_of(o, "gradient")
            .filter(|g| (-80.0..=80.0).contains(g))
            .map(|g| fixed(g, 2))
            .unwrap_or_else(|| {
                plan.defaulted("overlaps", id, "Massgebende_Neigung", "0.00");
                "0.00".into()
            });
        // The section the train stands in before the signal: the one it is the target of.
        let before = start.1 + if start.2 == Some("gegen") { 1.0 } else { -1.0 };
        let target_section = stretch.group_at(start.0, before).and_then(|g| group_section.get(&g));
        let specific = target_section.map(|s| {
            [
                w("Aufloesung_Verzoegerung", int_in(num_of(o, "releaseDelay"), 0.0, 150.0)),
                w(
                    "DWeg_V",
                    num_of(o, "speed").filter(|v| [20.0, 30.0, 40.0, 60.0, 100.0, 160.0].contains(v)).map(int),
                ),
                w("ID_FMA_Anlage_Zielgleis", Some(planpro_id(s))),
            ]
            .concat()
        });
        let name = str_of(o, "name").filter(|n| (1..=3).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
        let body = [
            group("Bezeichnung", w("Bezeichnung_Fstr_DWeg", name.map(String::from))),
            always(
                "Fstr_DWeg_Allg",
                [w("Laenge_Soll", Some(length)), w("Massgebende_Neigung", Some(gradient))].concat(),
            ),
            group("Fstr_DWeg_Spezifisch", specific.unwrap_or_default()),
            section_refs(o, "clearSections", "ID_FMA_Anlage_Freimeldung"),
            w("ID_Fstr_Fahrweg", Some(run)),
            w("ID_PZB_Gefahrpunkt", Some(mark)),
        ]
        .concat();
        plan.put("Fstr_DWeg", &guid, body, state);
        for (k, s) in list(o, "switches").iter().enumerate() {
            let Some(element) = gref_in(s, "switchElementId", &written_elements) else {
                continue;
            };
            let body = [
                w("Element_Verschluss", Some(yes(s.get("locked").and_then(Value::as_bool) == Some(true)))),
                w("ID_Fstr_DWeg", Some(guid.clone())),
                w("ID_W_Kr_Gsp_Element", Some(element)),
            ]
            .concat();
            plan.put("Fstr_DWeg_W_Kr", &planpro_id(&format!("overlap-switch:{guid}:{k}")), body, state);
        }
    }

    // Routes, and what each shows.
    let mut written_routes: HashSet<String> = HashSet::new();
    for r in col("routes") {
        let id = str_of(r, "id").unwrap_or("");
        let path = r.get("path").map(|p| list(p, "trackIds")).unwrap_or(&[]);
        let (start, target) = (str_of(r, "startSignalId"), str_of(r, "targetSignalId"));
        let (Some(from), Some(to)) = (start_of(start), start_of(target)) else {
            plan.not_written("routes", id, "no_signals");
            continue;
        };
        let parts = run_parts(path, Some(from), Some((to.0, to.1)));
        if parts.is_empty() {
            plan.not_written("routes", id, "no_run");
            continue;
        }
        written_routes.insert(id.to_string());
        let guid = gid(r);
        let state = stages.of(r);
        let run = planpro_id(&format!("run:{guid}"));
        let body = [
            parts,
            // A route from a planning boundary at the boundary's speed, where its start states none.
            w(
                "Fstr_V_Hg",
                int_in(
                    speeds.get(id).copied().or_else(|| {
                        col("signals")
                            .iter()
                            .find(|s| str_of(s, "id") == start)
                            .and_then(|s| boundary_start_speed(core, s))
                    }),
                    0.0,
                    500.0,
                ),
            ),
            w("ID_Start", start.map(planpro_id)),
            w("ID_Ziel", target.map(planpro_id)),
        ]
        .concat();
        plan.put("Fstr_Fahrweg", &run, body, state);
        let kind = str_of(r, "kind").unwrap_or("train");
        let body = match kind {
            "shunting" => always(
                "Fstr_Rangier",
                [
                    w(
                        "Fstr_Rangier_Art",
                        Some(str_of(r, "shuntingType").map(String::from).unwrap_or_else(|| {
                            plan.defaulted("routes", id, "Fstr_Rangier_Art", "RR");
                            "RR".into()
                        })),
                    ),
                    section_refs(r, "clearSections", "ID_FMA_Anlage_Rangier_Frei"),
                ]
                .concat(),
            ),
            "middle" => always(
                "Fstr_Mittel",
                [
                    w("Fstr_Mittel_Art", Some(str_of(r, "middleType").unwrap_or("ZM").into())),
                    w("Fstr_Mittel_V_Aufwertung", Some(yes(false))),
                ]
                .concat(),
            ),
            _ => {
                let mut overlaps: Vec<&Value> = list(r, "overlaps")
                    .iter()
                    .filter(|x| str_of(x, "overlapId").is_some_and(|o| written_overlaps.contains(o)))
                    .collect();
                overlaps.sort_by_key(|x| x.get("preferred").and_then(Value::as_bool) != Some(true));
                if overlaps.len() > 1 {
                    plan.not_written("routes.overlaps", id, "only_one");
                }
                let ovl = overlaps.first().map_or(String::new(), |x| {
                    always(
                        "Fstr_Zug_DWeg",
                        [
                            w("DWeg_Vorzug", Some(yes(x.get("preferred").and_then(Value::as_bool) == Some(true)))),
                            w("ID_Fstr_DWeg", str_of(x, "overlapId").map(planpro_id)),
                        ]
                        .concat(),
                    )
                });
                let art = str_of(r, "trainType").map(String::from).unwrap_or_else(|| {
                    plan.defaulted("routes", id, "Fstr_Zug_Art", "ZZ");
                    "ZZ".into()
                });
                always(
                    "Fstr_Zug",
                    [
                        w(
                            "Automatische_Einstellung",
                            (r.get("autoSetting").and_then(Value::as_bool) == Some(true)).then(|| "ZL".into()),
                        ),
                        w(
                            "Fstr_Vsigabstand_Verkuerzt",
                            Some(yes(r.get("shortDistantSpacing").and_then(Value::as_bool) == Some(true))),
                        ),
                        w("Fstr_Zug_Art", Some(art)),
                        ovl,
                    ]
                    .concat(),
                )
            }
        };
        let rank = int_in(num_of(r, "rank"), 0.0, 30.0).unwrap_or_else(|| "0".into());
        let body = [
            always("Fstr_Zug_Rangier_Allg", w("Fstr_Reihenfolge", Some(rank))),
            w("ID_Fstr_Fahrweg", Some(run)),
            body,
        ]
        .concat();
        plan.put("Fstr_Zug_Rangier", &guid, body, state);
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
            let body = [w("ID_Fstr_Zug_Rangier", Some(guid.clone())), w("ID_Signal_Signalbegriff", Some(aid.clone()))].concat();
            plan.put("Fstr_Signalisierung", &planpro_id(&format!("shows:{guid}:{k}")), body, state);
        }
    }

    // Flank protection.
    let flank_ids = ids_of("flankProtection");
    for f in col("flankProtection") {
        let id = str_of(f, "id").unwrap_or("");
        let guid = gid(f);
        let state = stages.of(f);
        let req = f.get("requester").unwrap_or(&Value::Null);
        let p = f.get("protection").unwrap_or(&Value::Null);
        let Some(requester) = gref_in(req, "switchElementId", &written_elements) else {
            plan.not_written("flankProtection", id, "no_requester");
            continue;
        };
        let pass = f.get("passOn").unwrap_or(&Value::Null);
        let passed = [
            w("ID_Fla_Weitergabe_L", gref_in(pass, "left", &flank_ids)),
            w("ID_Fla_Weitergabe_R", gref_in(pass, "right", &flank_ids)),
        ]
        .concat();
        let protection = if let Some(sig) = gref_in(p, "signalId", &written_signals) {
            always(
                "Fla_Schutz_Signal",
                [
                    w("Fla_Signal_Zielsperrung", p.get("targetLock").and_then(Value::as_bool).map(yes)),
                    w("ID_Fla_Signal", Some(sig)),
                ]
                .concat(),
            )
        } else if let Some(el) = gref_in(p, "switchElementId", &written_elements) {
            always(
                "Fla_Schutz_W_Gsp",
                [
                    w("Fla_W_Lage", str_of(p, "position").and_then(m::position_key).map(String::from)),
                    w("ID_Fla_W_Gsp_Element", Some(el)),
                ]
                .concat(),
            )
        } else if !passed.is_empty() {
            always("Fla_Schutz_Weitergabe", passed)
        } else if p.get("waived").and_then(Value::as_bool) == Some(true) {
            w("Fla_Verzicht", Some(yes(true)))
        } else {
            plan.not_written("flankProtection", id, "no_protection");
            continue;
        };
        let body = [
            always(
                "Fla_Schutz_Anforderer",
                [
                    w("Fahrt_Ueber", str_of(req, "via").and_then(m::via_key).map(String::from)),
                    w("ID_Anforderer_Element", Some(requester)),
                ]
                .concat(),
            ),
            protection,
        ]
        .concat();
        plan.put("Fla_Schutz", &guid, body, state);
        for (k, s) in list(f, "clearSections")
            .iter()
            .filter_map(Value::as_str)
            .filter(|s| written_sections.contains(*s))
            .enumerate()
        {
            let body = [
                w("Fla_Raum_Freimeldung", Some(yes(true))),
                w("ID_Fla_Schutz", Some(guid.clone())),
                w("ID_FMA_Anlage", Some(planpro_id(s))),
            ]
            .concat();
            plan.put("Fla_Freimelde_Zuordnung", &planpro_id(&format!("flank-clear:{guid}:{k}")), body, state);
        }
    }
    // Twofold protection of the turnouts' elements, once flank protection is written.
    for (sid, eid) in &element_guids {
        let Some(z) = element_of_switch
            .get(sid.as_str())
            .and_then(|e| e.get("twofoldProtection"))
            .filter(|z| z.is_object())
        else {
            continue;
        };
        let element = element_of_switch[sid.as_str()];
        let eid_model = str_of(element, "id").unwrap_or("");
        let mut measure = |k: &str, field: &str| -> String {
            str_of(z, k).and_then(m::twofold_measure_key).map(String::from).unwrap_or_else(|| {
                plan.defaulted("switchElements", eid_model, field, "Verschluss");
                "Verschluss".into()
            })
        };
        let (left, right) = (measure("left", "Massnahme_L"), measure("right", "Massnahme_R"));
        let kind = str_of(z, "kind").and_then(m::twofold_kind_key).unwrap_or_else(|| {
            plan.defaulted("switchElements", eid_model, "Zwieschutz_Art", "echt");
            "echt"
        });
        let body = [
            always(
                "Fla_Zwieschutz_Element",
                [
                    w("Massnahme_L", Some(left)),
                    w("Massnahme_R", Some(right)),
                    w("Nachlaufverhinderung", Some(yes(z.get("preventsRunOn").and_then(Value::as_bool) == Some(true)))),
                ]
                .concat(),
            ),
            w("ID_W_Element", Some(eid.clone())),
            w("Zwieschutz_Art", Some(kind.into())),
        ]
        .concat();
        plan.put("Fla_Zwieschutz", &planpro_id(&format!("twofold:{eid}")), body, stages.of(element));
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
        let Some(signal) = gref_in(b, "signalId", &written_signals) else {
            plan.not_written("balises", id, "no_signal");
            continue;
        };
        let guid = gid(b);
        let state = stages.of(b);
        let z = planpro_id(&format!("assignment:{guid}"));
        let settings = b.get("settings").unwrap_or(&Value::Null);
        let frequency = str_of(settings, "frequency").and_then(m::pzb_frequency_key).unwrap_or_else(|| {
            plan.defaulted("balises", id, "PZB_Art", "1000_2000_Hz");
            "1000_2000_Hz"
        });
        let kind = if str_of(b, "kind") == Some("speedCheck") {
            let arrangement = str_of(settings, "arrangement").and_then(m::gue_arrangement_key).unwrap_or_else(|| {
                plan.defaulted("balises", id, "GUE_Anordnung", "1_EA");
                "1_EA"
            });
            let length = num_of(settings, "measuringLength")
                .filter(|l| (5.8..=75.0).contains(l))
                .map(|l| fixed(l, 2))
                .unwrap_or_else(|| {
                    plan.defaulted("balises", id, "GUE_Messstrecke", "21.00");
                    "21.00".into()
                });
            let speed = int_in(num_of(settings, "testSpeed"), 10.0, 150.0).unwrap_or_else(|| {
                plan.defaulted("balises", id, "Pruefgeschwindigkeit", "60");
                "60".into()
            });
            plan.defaulted("balises", id, "GUE_Bauart", "sonstige");
            plan.defaulted("balises", id, "GUE_Energieversorgung", "sonstige");
            always(
                "PZB_Element_GUE",
                [
                    w("GUE_Anordnung", Some(arrangement.into())),
                    w("GUE_Bauart", Some("sonstige".into())),
                    w("GUE_Energieversorgung", Some("sonstige".into())),
                    w("GUE_Messstrecke", Some(length)),
                    w("Pruefgeschwindigkeit", Some(speed)),
                ]
                .concat(),
            )
        } else {
            always(
                "PZB_Element_GM",
                w("PZB_INA", Some(yes(settings.get("ina").and_then(Value::as_bool) == Some(true)))),
            )
        };
        let body = [
            at.place(None),
            w("ID_PZB_Element_Zuordnung", Some(z.clone())),
            w("PZB_Art", Some(frequency.into())),
            kind,
        ]
        .concat();
        plan.put("PZB_Element", &guid, body, state);
        let routes: String = list(b, "routes")
            .iter()
            .filter_map(|r| {
                let route = gref_in(r, "routeId", &written_routes)?;
                let effect = str_of(r, "effect").and_then(m::pzb_route_effect_key).unwrap_or("sonstige");
                Some(always(
                    "PZB_Element_Zuordnung_Fstr",
                    [w("ID_Fstr_Zug_Rangier", Some(route)), w("Wirksamkeit_Fstr", Some(effect.into()))].concat(),
                ))
            })
            .collect();
        let effect = str_of(b, "effect").and_then(m::pzb_effect_key).unwrap_or("sonstige");
        let bp = always(
            "PZB_Element_Zuordnung_BP",
            [w("ID_PZB_Element_Bezugspunkt", Some(signal)), w("Wirksamkeit", Some(effect.into()))].concat(),
        );
        plan.put("PZB_Element_Zuordnung", &z, [bp, routes].concat(), state);
    }

    for k in ["stationBoundaries", "stoppingPlaces", "itineraries"] {
        for v in col(k) {
            plan.not_written(k, str_of(v, "id").unwrap_or(""), "kind_not_written");
        }
    }
    // The stopping places are the core's in infra-data-2.0.
    for v in c("stoppingPlaces") {
        plan.not_written("stoppingPlaces", str_of(v, "id").unwrap_or(""), "kind_not_written");
    }

    // ── the file ──
    let project_id = str_of(fields, "id").unwrap_or("");
    let title = str_of(fields, "title").or(str_of(project, "title")).unwrap_or("");
    let planning = Planning::of(options, title, project_id, &date);
    leading.sort_by_key(|a| std::cmp::Reverse(a.0));
    let location = planning.location.clone().or_else(|| leading.first().map(|l| l.1.clone())).unwrap_or_else(|| {
        plan.defaulted("planning", project_id, "Fuehrende_Oertlichkeit", "RAS");
        "RAS".into()
    });
    let rank: HashMap<&str, usize> = CONTAINER_ORDER.iter().enumerate().map(|(i, t)| (*t, i)).collect();
    plan.start.sort_by_key(|i| rank.get(i.tag).copied().unwrap_or(usize::MAX));
    plan.target.sort_by_key(|i| rank.get(i.tag).copied().unwrap_or(usize::MAX));
    let stamp = now.map(String::from).unwrap_or_else(|| format!("{date}T00:00:00Z"));
    let tool = [
        w("Erzeugung_Zeitstempel", Some(stamp.clone())),
        w("Werkzeug_Name", Some("Rail-Across-Suite".into())),
        w("Werkzeug_Version", Some(text(str_of(options, "version"), 30).unwrap_or_else(|| "dev".into()))),
    ]
    .concat();
    let mut text_out = String::new();
    let _ = writeln!(text_out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        text_out,
        "<nsPlanPro:PlanPro_Schnittstelle xmlns:nsPlanPro=\"{NS_PLANPRO}\" xmlns:nsSignalbegriffe_Ril_301=\"{NS_ASPECTS}\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">"
    );
    let _ = writeln!(text_out, "{}", id_el(&planpro_id(&format!("plan:{project_id}"))));
    let _ = writeln!(text_out, "<PlanPro_Schnittstelle_Allg>{tool}</PlanPro_Schnittstelle_Allg>");
    let _ = writeln!(text_out, "<LST_Planung><Fachdaten>");
    let mut groups = String::new();
    for (part, geo) in [("ESTW", false), ("Geo", true)] {
        let data_id = planpro_id(&format!("data:{project_id}:{part}"));
        let _ = writeln!(text_out, "<Ausgabe_Fachdaten>{}", id_el(&data_id));
        for (name, items) in [("LST_Zustand_Start", &plan.start), ("LST_Zustand_Ziel", &plan.target)] {
            let _ = writeln!(
                text_out,
                "<{name}>{}<Container>",
                id_el(&planpro_id(&format!("state:{project_id}:{part}:{name}")))
            );
            for i in items.iter().filter(|i| GEO_PART.contains(&i.tag) == geo) {
                let _ = writeln!(text_out, "{}", i.xml);
            }
            let _ = writeln!(text_out, "</Container></{name}>");
        }
        let _ = writeln!(text_out, "{}</Ausgabe_Fachdaten>", w("Untergewerk_Art", Some(part.into())));
        groups.push_str(&always("LST_Planung_Gruppe", planning.group(project_id, part, &data_id, &location)));
    }
    let _ = writeln!(text_out, "</Fachdaten>");
    let project_xml = [
        id_el(&planpro_id(&format!("planning-project:{project_id}"))),
        groups,
        always(
            "Planung_P_Allg",
            [
                w("Bezeichnung_Planung_Projekt", Some(planning.title.clone())),
                w("Datum_Abschluss_Projekt", Some(date.clone())),
                w("Projekt_Nummer", Some(planning.project_number.clone())),
                always("Projektleiter", planning.actor(&format!("{project_id}:lead"))),
            ]
            .concat(),
        ),
    ]
    .concat();
    let _ = writeln!(
        text_out,
        "<Objektmanagement><LST_Planung_Projekt>{project_xml}</LST_Planung_Projekt></Objektmanagement>"
    );
    let _ = writeln!(text_out, "</LST_Planung>");
    let _ = writeln!(text_out, "</nsPlanPro:PlanPro_Schnittstelle>");
    let zip = archive::zip_stored(
        &[("content.xml", text_out.as_bytes()), ("manifest.xml", archive::MANIFEST.as_bytes())],
        Some(&stamp),
    );
    Ok(json!({
        "text": text_out,
        "manifest": archive::MANIFEST,
        "archive": archive::base64(&zip),
        "report": { "notes": plan.notes },
        "counts": plan.counts,
    }))
}

/// The heights of a track: a point per point of its vertical tangent polygon,
/// a straight line between each two (Hoehenlinie Gerade); vertical curves
/// are not written.
fn heights_of(plan: &mut Plan, t: &Value, edge: &Edge, state: (bool, bool)) {
    let mut points: Vec<(f64, f64)> = list(t, "heights")
        .iter()
        .filter_map(|h| Some((num_of(h, "station")?, num_of(h, "z")?)))
        .collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points.retain(|(s, z)| (-1e-6..=edge.track_length() + 1e-6).contains(s) && (-100.0..=3000.0).contains(z));
    if points.is_empty() {
        return;
    }
    let track = str_of(t, "id").unwrap_or("");
    if list(t, "heights").iter().any(|h| num_of(h, "rv").is_some_and(|r| r != 0.0)) {
        plan.not_written("heights", track, "vertical_curves_not_written");
    }
    let system = match num_of(t, "heightEpsg").map(|e| e as i64).unwrap_or(7837) {
        7837 => Some("DHHN_2016"),
        5783 => Some("DHHN_92"),
        _ => None,
    };
    let id = |i: usize| planpro_id(&format!("height:{}:{i}", edge.guid));
    for (i, (s, z)) in points.iter().enumerate() {
        let allg = [
            w("Hoehenpunkt_Hoehe", Some(fixed(*z, 3))),
            w("HSystem", system.map(String::from)),
            w("Plan_Quelle", Some(SOURCE.into())),
        ]
        .concat();
        plan.put(
            "Hoehenpunkt",
            &id(i),
            [place(edge, *s, None, Lateral::Offset(0.0)), always("Hoehenpunkt_Allg", allg)].concat(),
            state,
        );
    }
    for i in 1..points.len() {
        let allg = [
            w("Hoehenlinie_Form", Some("Gerade".into())),
            w("Hoehenlinie_Laenge", Some(fixed((points[i].0 - points[i - 1].0) * edge.scale, 3))),
            w("Plan_Quelle", Some(SOURCE.into())),
        ]
        .concat();
        let body = [
            always("Hoehenlinie_Allg", allg),
            w("ID_Hoehenpunkt_A", Some(id(i - 1))),
            w("ID_Hoehenpunkt_B", Some(id(i))),
        ]
        .concat();
        plan.put("Hoehenlinie", &planpro_id(&format!("height-line:{}:{i}", edge.guid)), body, state);
    }
}

/// The cant of a track, where its elements state one: a point at each end of
/// an element, a line between — the same along it, or a ramp.
fn cant_of(plan: &mut Plan, t: &Value, edge: &Edge, state: (bool, bool)) {
    let els = list(t, "elements");
    if !els
        .iter()
        .any(|e| e.get("cant").is_some() || e.get("cantStart").is_some() || e.get("cantEnd").is_some())
    {
        return;
    }
    let track = str_of(t, "id").unwrap_or("");
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut forms: Vec<&'static str> = Vec::new();
    let mut s = 0.0;
    for el in els {
        let len = num_of(el, "length").unwrap_or(0.0);
        let (a, b) = match num_of(el, "cant") {
            Some(c) => (c, c),
            None => (num_of(el, "cantStart").unwrap_or(0.0), num_of(el, "cantEnd").unwrap_or(0.0)),
        };
        if points.last().is_none_or(|p| (p.0 - s).abs() > 1e-6 || (p.1 - a).abs() > 1e-6) {
            if points.last().is_some_and(|p| (p.0 - s).abs() <= 1e-6) {
                forms.push("sonstige");
            } else if !points.is_empty() {
                forms.push("gleichbleibend");
            }
            points.push((s, a));
        }
        let form = if (a - b).abs() < 1e-6 {
            "gleichbleibend"
        } else if str_of(el, "transitionType") == Some("bloss") {
            "Rampe_Bloss"
        } else {
            "Rampe_gerade"
        };
        s += len;
        points.push((s, b));
        forms.push(form);
    }
    if points.iter().any(|p| p.1.abs() > 200.0) {
        plan.not_written("cant", track, "cant_over_200_mm");
        return;
    }
    let id = |i: usize| planpro_id(&format!("cant:{}:{i}", edge.guid));
    for (i, (s, c)) in points.iter().enumerate() {
        let allg = [w("Plan_Quelle", Some(SOURCE.into())), w("Ueberhoehung_Hoehe", Some(fixed(c.abs() / 1000.0, 3)))].concat();
        plan.put(
            "Ueberhoehung",
            &id(i),
            [place(edge, *s, None, Lateral::Offset(0.0)), always("Ueberhoehung_Allg", allg)].concat(),
            state,
        );
    }
    for i in 1..points.len() {
        let allg = [
            w("Plan_Quelle", Some(SOURCE.into())),
            w("Ueberhoehungslinie_Form", Some(forms[i - 1].into())),
            w("Ueberhoehungslinie_Laenge", Some(fixed((points[i].0 - points[i - 1].0) * edge.scale, 3))),
        ]
        .concat();
        let body = [
            w("ID_Ueberhoehung_A", Some(id(i - 1))),
            w("ID_Ueberhoehung_B", Some(id(i))),
            always("Ueberhoehungslinie_Allg", allg),
        ]
        .concat();
        plan.put("Ueberhoehungslinie", &planpro_id(&format!("cant-line:{}:{i}", edge.guid)), body, state);
    }
}
