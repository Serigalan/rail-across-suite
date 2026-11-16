// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The cross section of a track at a station: running plane, superstructure
//! and the clearance contours of DB Ril 800.0130 (`crossSection/`).

use rail_across_geom::kind::is_transition;
use std::sync::LazyLock;

use rail_across_geom::cant::{cant_ends, transition_cant_ends};
use rail_across_geom::coords::{plane_to_wgs84, transform_plane_point};
use rail_across_js::{get, js_number, number, truthy};
use serde_json::{Value, json};

use crate::platforms::{DEFAULT_PLATFORM_HEIGHT, edge_offsets, element_stations, point_on_element};

const CATALOGUE_FILE: &str = include_str!("../../../packages/core/constraints/db-ril-800-0130.json");

/// The catalogue of DB Ril 800.0130 (`CROSS_SECTION_CATALOGUE`).
pub static CROSS_SECTION_CATALOGUE: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(CATALOGUE_FILE).expect("db-ril-800-0130.json"));

pub const RUNNING_CIRCLE_DISTANCE: f64 = 1500.0;
pub const TRACK_GAUGE: f64 = 1435.0;
const HALF_RUNNING: f64 = RUNNING_CIRCLE_DISTANCE / 2.0;
const HALF_GAUGE: f64 = TRACK_GAUGE / 2.0;
pub const DEFAULT_RAIL: &str = "54E4";
pub const DEFAULT_SLEEPER: &str = "B70";
pub const DEFAULT_GAUGE_PROFILE: &str = "hauptgleis";
pub const ASSUMED_RAIL_OVER_TERRAIN: f64 = 0.20;
const MAX_CROSSING_ANGLE: f64 = 45.0;
const HEAD_SHARE: f64 = 0.28;
const FOOT_SHARE: f64 = 0.16;

/// A rail profile [mm] (`RAILS`).
pub struct Rail {
    pub key: &'static str,
    pub label: &'static str,
    pub head: f64,
    pub head_bottom: f64,
    pub web: f64,
    pub foot: f64,
    pub height: f64,
    pub height_worn: f64,
    pub head_height: f64,
    pub head_height_worn: f64,
    pub flank_height: f64,
    pub flank_height_worn: f64,
}

pub const RAILS: [Rail; 3] = [
    Rail {
        key: "49E5",
        label: "49 E5 (S 49)",
        head: 67.0,
        head_bottom: 70.0,
        web: 14.0,
        foot: 125.0,
        height: 149.0,
        height_worn: 139.1,
        head_height: 51.5,
        head_height_worn: 41.6,
        flank_height: 39.8,
        flank_height_worn: 29.8,
    },
    Rail {
        key: "54E4",
        label: "54 E4 (S 54)",
        head: 67.0,
        head_bottom: 70.0,
        web: 16.0,
        foot: 125.0,
        height: 154.0,
        height_worn: 143.2,
        head_height: 55.0,
        head_height_worn: 44.2,
        flank_height: 43.3,
        flank_height_worn: 32.5,
    },
    Rail {
        key: "60E2",
        label: "60 E2 (UIC 60)",
        head: 72.0,
        head_bottom: 74.3,
        web: 16.5,
        foot: 150.0,
        height: 172.0,
        height_worn: 162.6,
        head_height: 51.0,
        head_height_worn: 41.6,
        flank_height: 37.5,
        flank_height_worn: 28.1,
    },
];

/// A sleeper type [mm] (`SLEEPERS`).
pub struct Sleeper {
    pub key: &'static str,
    pub label: &'static str,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub usage: &'static str,
}

pub const SLEEPERS: [Sleeper; 3] = [
    Sleeper {
        key: "B70",
        label: "B70",
        length: 2600.0,
        width: 300.0,
        height: 210.0,
        usage: "normal",
    },
    Sleeper {
        key: "B90",
        label: "B90",
        length: 2600.0,
        width: 320.0,
        height: 200.0,
        usage: "switch_zone",
    },
    Sleeper {
        key: "SWITCH",
        label: "Weichenschwelle",
        length: 2600.0,
        width: 300.0,
        height: 220.0,
        usage: "switch",
    },
];

pub fn rail(key: Option<&str>) -> Option<&'static Rail> {
    RAILS.iter().find(|r| Some(r.key) == key)
}

pub fn sleeper(key: Option<&str>) -> Option<&'static Sleeper> {
    SLEEPERS.iter().find(|s| Some(s.key) == key)
}

/// `RAILS` as the JavaScript states it.
pub fn rails_json() -> Value {
    Value::Object(
        RAILS
            .iter()
            .map(|r| {
                (r.key.to_string(), json!({ "label": r.label, "head": number(r.head), "headBottom": number(r.head_bottom), "web": number(r.web), "foot": number(r.foot),
                    "height": number(r.height), "heightWorn": number(r.height_worn), "headHeight": number(r.head_height), "headHeightWorn": number(r.head_height_worn),
                    "flankHeight": number(r.flank_height), "flankHeightWorn": number(r.flank_height_worn) }))
            })
            .collect(),
    )
}

/// `SLEEPERS` as the JavaScript states it.
pub fn sleepers_json() -> Value {
    Value::Object(
        SLEEPERS
            .iter()
            .map(|s| {
                (
                    s.key.to_string(),
                    json!({ "label": s.label, "length": number(s.length), "width": number(s.width), "height": number(s.height), "use": s.usage }),
                )
            })
            .collect(),
    )
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    hi.min(lo.max(v))
}

// ── gauge profiles ──────────────────────────────────────────────────────────

/// "DB Ril 800.0130A01 — Lichtraum" → "DB Ril 800.0130A01" (`CLEARANCE_SOURCE`).
pub fn clearance_source() -> String {
    let k = &*CROSS_SECTION_CATALOGUE;
    let r = &k["lichtraum"]["source"]["ref"];
    let a01 = k["sources"].as_array().and_then(|s| s.iter().find(|s| &s["id"] == r));
    a01.and_then(|s| s["title"].as_str())
        .map(|t| t.split(" — ").next().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// The profiles by id (`GAUGE_PROFILES`).
pub fn gauge_profiles() -> Value {
    let profiles = CROSS_SECTION_CATALOGUE["lichtraum"]["profile"].as_array().cloned().unwrap_or_default();
    Value::Object(
        profiles
            .iter()
            .map(|p| {
                let key = rail_across_js::js_string(p.get("id"));
                let or = |k: &str| get(p, k).cloned().unwrap_or(json!([]));
                (
                    key,
                    json!({ "id": p.get("id"), "points": p.get("umriss"), "einragungen": or("einragungen"), "areaKinds": or("einragungen_arten") }),
                )
            })
            .collect(),
    )
}

/// The chosen profile, or the default (`gaugeProfile`).
pub fn gauge_profile(key: Option<&str>) -> Value {
    let all = gauge_profiles();
    key.and_then(|k| all.get(k))
        .or_else(|| all.get(DEFAULT_GAUGE_PROFILE))
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn gauge_profile_label_key(id: &str) -> String {
    format!("constraints_querschnitt_profil_{id}")
}

fn on_axis(p: &Value) -> bool {
    p.get(0).and_then(Value::as_f64) == Some(0.0)
}

fn mirror(p: &Value) -> Value {
    let y = if on_axis(p) { json!(0) } else { number(-f64_at(p, 0)) };
    json!([y, p.get(1).cloned().unwrap_or(Value::Null)])
}

fn f64_at(p: &Value, i: usize) -> f64 {
    p.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn closed(mut points: Vec<Value>) -> Vec<Value> {
    let (first, last) = (&points[0], &points[points.len() - 1]);
    if !(f64_at(first, 0) == f64_at(last, 0) && f64_at(first, 1) == f64_at(last, 1)) {
        points.push(first.clone());
    }
    points
}

/// The closed outline of a profile (`gaugeProfileRing`).
pub fn gauge_profile_ring(points: &[Value]) -> Vec<Value> {
    if points.is_empty() {
        return vec![];
    }
    let back: Vec<Value> = points.iter().rev().map(mirror).collect();
    let skip = usize::from(on_axis(&points[points.len() - 1]));
    closed(points.iter().cloned().chain(back.into_iter().skip(skip)).collect())
}

/// The areas of a profile, each as stated and mirrored (`gaugeProfileAreas`).
pub fn gauge_profile_areas(areas: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for area in areas {
        let Some(a) = area.as_array().filter(|a| !a.is_empty()) else { continue };
        out.push(Value::Array(closed(a.clone())));
        if !a.iter().all(on_axis) {
            out.push(Value::Array(closed(a.iter().map(mirror).collect())));
        }
    }
    out
}

/// The planum edge beyond the outermost track [mm] (`FORMATION_EDGE`).
pub fn formation_edge() -> f64 {
    let rows = CROSS_SECTION_CATALOGUE["streckenquerschnitte"]["rows"].as_array().cloned().unwrap_or_default();
    1000.0 * rows.iter().map(|r| js_number(r.get("planumskante"))).fold(f64::NEG_INFINITY, f64::max)
}

// ── the section ─────────────────────────────────────────────────────────────

/// The station a track's element begins at (`elementStartStation`).
pub fn element_start_station(track: &Value, el_idx: usize) -> f64 {
    track
        .get("elements")
        .and_then(Value::as_array)
        .map(|els| els.iter().take(el_idx).map(|el| get(el, "length").and_then(Value::as_f64).unwrap_or(0.0)).sum())
        .unwrap_or(0.0)
}

fn range_type_at(ranges: Option<&Value>, station: f64) -> Option<Value> {
    let ranges = ranges?.as_array()?;
    for r in ranges.iter().rev() {
        let from = get(r, "from").map(|v| js_number(Some(v))).unwrap_or(0.0);
        let to = get(r, "to").map(|v| js_number(Some(v))).unwrap_or(f64::INFINITY);
        if station >= from && station <= to {
            return Some(r.get("type").cloned().unwrap_or(Value::Null));
        }
    }
    None
}

/// The superstructure at a station (`superstructureAt`).
pub fn superstructure_at(track: &Value, station: f64) -> Value {
    let pick = |k: &str, d: &str| range_type_at(track.get(k), station).filter(|v| !v.is_null()).unwrap_or(json!(d));
    json!({ "rail": pick("rails", DEFAULT_RAIL), "sleeper": pick("sleepers", DEFAULT_SLEEPER) })
}

/// The angle the cant turns the section through [rad] (`cantAngle`).
pub fn cant_angle(cant: f64) -> f64 {
    -clamp(cant / RUNNING_CIRCLE_DISTANCE, -1.0, 1.0).asin()
}

/// The running circle the cant turns the section about (`cantPivot`).
pub fn cant_pivot(cant: f64) -> [f64; 2] {
    [if cant < 0.0 { -HALF_RUNNING } else { HALF_RUNNING }, 0.0]
}

pub fn rotate_point([y, z]: [f64; 2], angle: f64) -> [f64; 2] {
    let (c, s) = (angle.cos(), angle.sin());
    [y * c - z * s, y * s + z * c]
}

/// Points turned by `angle` about `[py, pz]` (`rotatePointsAbout`).
pub fn rotate_points_about(points: &[[f64; 2]], angle: f64, [py, pz]: [f64; 2]) -> Vec<[f64; 2]> {
    points
        .iter()
        .map(|&[y, z]| {
            let [ry, rz] = rotate_point([y - py, z - pz], angle);
            [ry + py, rz + pz]
        })
        .collect()
}

pub fn pts(points: &[[f64; 2]]) -> Value {
    Value::Array(points.iter().map(|p| json!([number(p[0]), number(p[1])])).collect())
}

pub fn read_pts(v: &Value) -> Vec<[f64; 2]> {
    v.as_array()
        .map(|a| a.iter().map(|p| [f64_at(p, 0), f64_at(p, 1)]).collect())
        .unwrap_or_default()
}

/// The mapping that fits a section into a drawing area (`fitSection`).
pub fn fit_section(points: &[[f64; 2]], w: f64, h: f64, margin: f64) -> Value {
    let min = |i: usize| points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min);
    let max = |i: usize| points.iter().map(|p| p[i]).fold(f64::NEG_INFINITY, f64::max);
    let (y_min, y_max, z_min, z_max) = (min(0), max(0), min(1), max(1));
    let k = ((w - 2.0 * margin) / (y_max - y_min).max(1.0)).min((h - 2.0 * margin) / (z_max - z_min).max(1.0));
    json!({
        "k": number(k), "cx": number(w / 2.0 - ((y_min + y_max) / 2.0) * k), "cy": number(h / 2.0 + ((z_min + z_max) / 2.0) * k),
        "bounds": { "yMin": number(y_min), "yMax": number(y_max), "zMin": number(z_min), "zMax": number(z_max) },
    })
}

/// Cant and radius at a station of a track (`sectionAtStation`).
pub fn section_at_station(track: &Value, station: f64) -> Value {
    let els = track.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    let mut start = 0.0;
    for (i, el) in els.iter().enumerate() {
        let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        if i != els.len() - 1 && station >= start + len - 1e-9 {
            start += len;
            continue;
        }
        let s = clamp(station - start, 0.0, len);
        let t = if len > 0.0 { s / len } else { 0.0 };
        if is_transition(el) {
            let (c1, c2) = transition_cant_ends(els, i);
            let k = |key: &str| get(el, key).map(|r| 1.0 / js_number(Some(r))).unwrap_or(0.0);
            let (k1, k2) = (k("r1"), k("r2"));
            let kk = k1 + t * (k2 - k1);
            return json!({ "elIdx": i, "station": number(start + s), "cant": number(c1 + t * (c2 - c1)), "radius": if kk.abs() > 1e-9 { number(1.0 / kk) } else { Value::Null } });
        }
        let (c1, c2) = cant_ends(els, i);
        return json!({ "elIdx": i, "station": number(start + s), "cant": number(c1 + t * (c2 - c1)), "radius": get(el, "radius").cloned().unwrap_or(Value::Null) });
    }
    Value::Null
}

fn rail_outline(r: &Rail, face_y: f64, side: f64) -> Vec<[f64; 2]> {
    let axis = face_y + side * r.head / 2.0;
    let head_z = -r.height * HEAD_SHARE;
    let foot_z = -r.height * (1.0 - FOOT_SHARE);
    let at = |hw: f64, z: f64| ([axis - hw, z], [axis + hw, z]);
    let (head_l, head_r) = at(r.head / 2.0, 0.0);
    let (hb_l, hb_r) = at(r.head / 2.0, head_z);
    let (web_l, web_r) = at(r.web / 2.0, head_z);
    let (wb_l, wb_r) = at(r.web / 2.0, foot_z);
    let (foot_l, foot_r) = at(r.foot / 2.0, foot_z);
    let (bot_l, bot_r) = at(r.foot / 2.0, -r.height);
    vec![head_l, head_r, hb_r, web_r, wb_r, foot_r, bot_r, bot_l, foot_l, wb_l, web_l, hb_l, head_l]
}

fn sleeper_outline(s: &Sleeper, rail_height: f64) -> Vec<[f64; 2]> {
    let half = s.length / 2.0;
    let (top, bottom) = (-rail_height, -rail_height - s.height);
    vec![[-half, top], [half, top], [half, bottom], [-half, bottom], [-half, top]]
}

/// The drawable cross section for one state of an element (`crossSection`).
pub fn cross_section(o: &Value) -> Value {
    let cant = match o.get("cant") {
        None => 0.0,
        Some(v) => v.as_f64().unwrap_or(0.0),
    };
    let angle = cant_angle(cant);
    let pivot = cant_pivot(cant);
    let turn = |p: &[[f64; 2]]| pts(&rotate_points_about(p, angle, pivot));
    let rp = rail(o.get("rail").and_then(Value::as_str));
    let sp = sleeper(o.get("sleeper").and_then(Value::as_str));
    let areas: Vec<Value> = o
        .get("gaugeAreas")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|a| turn(&read_pts(a)))
        .collect();
    json!({
        "angle": number(angle), "pivot": [number(pivot[0]), number(pivot[1])],
        "runningCircles": turn(&[[-HALF_RUNNING, 0.0], [HALF_RUNNING, 0.0]]),
        "railFaces": turn(&[[-HALF_GAUGE, 0.0], [HALF_GAUGE, 0.0]]),
        "gauge": turn(&read_pts(o.get("gaugeRing").unwrap_or(&json!([])))),
        "areas": areas,
        "rails": rp.map(|r| json!([turn(&rail_outline(r, -HALF_GAUGE, -1.0)), turn(&rail_outline(r, HALF_GAUGE, 1.0))])).unwrap_or(json!([])),
        "sleeper": match (sp, rp) { (Some(s), Some(r)) => turn(&sleeper_outline(s, r.height)), _ => json!([]) },
    })
}

/// The parts of a section moved into another track's frame (`placeSection`).
pub fn place_section(points: &[[f64; 2]], offset: f64, dz: f64, mirrored: bool) -> Vec<[f64; 2]> {
    points.iter().map(|&[y, z]| [(if mirrored { -y } else { y }) + offset, z + dz]).collect()
}

/// The other tracks the section line crosses within `reach` (`sectionNeighbours`).
pub fn section_neighbours(track: &Value, station: f64, tracks: &[Value], reach: f64) -> Vec<Value> {
    let epsg = js_number(track.get("epsg"));
    let rows = element_stations(track);
    if !truthy(track.get("epsg")) || rows.is_empty() {
        return vec![];
    }
    let els = track["elements"].as_array().cloned().unwrap_or_default();
    let total = rows[rows.len() - 1].2;
    let s0 = clamp(station, 0.0, total);
    let row0 = rows.iter().find(|r| s0 <= r.2 + 1e-9).unwrap_or(&rows[rows.len() - 1]);
    let Some((o, bearing)) = point_on_element(&els[row0.0], epsg as i64, s0 - row0.1) else {
        return vec![];
    };
    let rad = bearing * std::f64::consts::PI / 180.0;
    let along = [rad.sin(), rad.cos()];
    let right = [rad.cos(), -rad.sin()];
    let min_cos = (MAX_CROSSING_ANGLE * std::f64::consts::PI / 180.0).cos();
    let dot = |v: [f64; 2], w: [f64; 2]| v[0] * w[0] + v[1] * w[1];
    let mut found: Vec<(f64, Value)> = Vec::new();
    for other in tracks {
        if !truthy(other.get("epsg")) || other.get("id") == track.get("id") {
            continue;
        }
        let oe = js_number(other.get("epsg")) as i64;
        let to_here = |p: (rail_across_geom::element::Utm, f64)| {
            let (e, n) = transform_plane_point(p.0.e, p.0.n, oe, epsg as i64).unwrap_or((f64::NAN, f64::NAN));
            [e - o.e, n - o.n]
        };
        let oels = other["elements"].as_array().cloned().unwrap_or_default();
        for (idx, rstart, _) in element_stations(other) {
            let el = &oels[idx];
            let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
            if !(len > 0.0) {
                continue;
            }
            let poe = |s: f64| {
                point_on_element(el, oe, s).unwrap_or((
                    rail_across_geom::element::Utm {
                        e: f64::NAN,
                        n: f64::NAN,
                        zone: oe,
                    },
                    f64::NAN,
                ))
            };
            let start = to_here(poe(0.0));
            if start[0].hypot(start[1]) > len + reach + 1.0 {
                continue;
            }
            let at = |s: f64| to_here(poe(s));
            let n = 1usize.max((len / 2.0).ceil() as usize);
            let (mut prev_s, mut prev_d) = (0.0, dot(start, along));
            for k in 1..=n {
                let s = len * k as f64 / n as f64;
                let d = dot(at(s), along);
                if (prev_d <= 0.0 && d > 0.0) || (prev_d > 0.0 && d <= 0.0) {
                    let (mut lo, mut hi, mut d_lo) = (prev_s, s, prev_d);
                    let mut i = 0;
                    while i < 40 && hi - lo > 1e-4 {
                        let mid = (lo + hi) / 2.0;
                        let d_mid = dot(at(mid), along);
                        if (d_lo <= 0.0) == (d_mid <= 0.0) {
                            lo = mid;
                            d_lo = d_mid;
                        } else {
                            hi = mid;
                        }
                        i += 1;
                    }
                    let s_hit = (lo + hi) / 2.0;
                    let y = dot(at(s_hit), right);
                    let dir = poe(s_hit).1 * std::f64::consts::PI / 180.0;
                    let cos = dir.sin() * along[0] + dir.cos() * along[1];
                    if y.abs() <= reach && cos.abs() >= min_cos {
                        found.push((
                            y * 1000.0,
                            json!({ "track": other, "station": number(rstart + s_hit), "offset": number(y * 1000.0), "mirrored": cos < 0.0 }),
                        ));
                    }
                }
                prev_s = s;
                prev_d = d;
            }
        }
    }
    found.sort_by(|a, b| (a.0 - b.0).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
    found.into_iter().map(|(_, v)| v).collect()
}

/// The terrain height at `y` from the points read along the line (`terrainHeightAt`).
pub fn terrain_height_at(points: &[Value], y: f64) -> Option<f64> {
    for i in 1..points.len() {
        let (a, b) = (&points[i - 1], &points[i]);
        let (ay, by) = (js_number(a.get("y")), js_number(b.get("y")));
        if y < ay || y > by {
            continue;
        }
        let (az, bz) = (get(a, "z").map(|v| js_number(Some(v)))?, get(b, "z").map(|v| js_number(Some(v)))?);
        return Some(if by == ay { az } else { az + (bz - az) * (y - ay) / (by - ay) });
    }
    None
}

/// The level every drawn track stands at, and the reference (`sectionLevels`).
pub fn section_levels(tracks: &[Value], terrain: &[Value]) -> Value {
    let levels: Vec<(Option<f64>, bool)> = tracks
        .iter()
        .map(|t| match get(t, "z") {
            Some(z) => (Some(js_number(Some(z))), false),
            None => (
                terrain_height_at(terrain, js_number(t.get("offset"))).map(|g| g + ASSUMED_RAIL_OVER_TERRAIN),
                true,
            ),
        })
        .collect();
    let mut by_distance: Vec<usize> = (1..tracks.len()).collect();
    by_distance.sort_by(|&a, &b| {
        (js_number(tracks[a].get("offset")).abs() - js_number(tracks[b].get("offset")).abs())
            .partial_cmp(&0.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let first = levels.first();
    let z_ref = first
        .and_then(|l| l.0.filter(|_| !l.1))
        .or_else(|| by_distance.iter().map(|&i| levels[i]).find(|l| l.0.is_some() && !l.1).and_then(|l| l.0))
        .or_else(|| first.and_then(|l| l.0))
        .or_else(|| by_distance.iter().map(|&i| levels[i]).find(|l| l.0.is_some()).and_then(|l| l.0));
    json!({
        "levels": levels.iter().map(|(z, assumed)| json!({ "z": z.map(number), "assumed": assumed })).collect::<Vec<_>>(),
        "zRef": z_ref.map(number),
    })
}

/// Where the section is taken (`sectionOrigin`): the point and the bearing there.
pub fn section_origin(track: &Value, station: f64) -> Option<(rail_across_geom::element::Utm, f64)> {
    let rows = element_stations(track);
    if !truthy(track.get("epsg")) || rows.is_empty() {
        return None;
    }
    let epsg = js_number(track.get("epsg")) as i64;
    let total = rows[rows.len() - 1].2;
    let s0 = clamp(station, 0.0, total);
    let row = rows.iter().find(|r| s0 <= r.2 + 1e-9).unwrap_or(&rows[rows.len() - 1]);
    point_on_element(&track["elements"][row.0], epsg, s0 - row.1)
}

/// Where the terrain is read along the section line (`sectionLinePoints`).
pub fn section_line_points(track: &Value, station: f64, from: f64, to: f64, step: f64) -> Vec<Value> {
    let Some((utm, bearing)) = section_origin(track, station) else {
        return vec![];
    };
    if !(to > from) || !(step > 0.0) {
        return vec![];
    }
    let epsg = js_number(track.get("epsg")) as i64;
    let rad = bearing * std::f64::consts::PI / 180.0;
    let n = ((to - from) / step).ceil() as usize;
    (0..=n)
        .map(|i| {
            let y = from + (to - from) * i as f64 / n as f64;
            let (lng, lat) = plane_to_wgs84(utm.e + y * rad.cos(), utm.n - y * rad.sin(), epsg).unwrap_or((f64::NAN, f64::NAN));
            json!({ "y": number(y * 1000.0), "lngLat": [number(lng), number(lat)] })
        })
        .collect()
}

/// The outline of a platform in the section's frame (`platformSection`).
pub fn platform_section(platform: &Value, rail_key: Option<&str>, sleeper_key: Option<&str>) -> Vec<[f64; 2]> {
    let (front, back) = edge_offsets(platform);
    let height = js_number(platform.get("height"));
    let top = if height.is_finite() { height } else { DEFAULT_PLATFORM_HEIGHT };
    let r = rail(rail_key).or_else(|| rail(Some(DEFAULT_RAIL))).expect("default rail");
    let s = sleeper(sleeper_key).or_else(|| sleeper(Some(DEFAULT_SLEEPER))).expect("default sleeper");
    let bottom = -(r.height + s.height);
    let (fy, by) = (front * 1000.0, back * 1000.0);
    vec![[fy, top], [by, top], [by, bottom], [fy, bottom], [fy, top]]
}
