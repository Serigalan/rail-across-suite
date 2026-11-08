// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The planes a track can lie in and the conversions between them
//! (`coordinateUtils.js`, `ntv2Grid.js`): which datum and zone an EPSG code
//! stands for, the systems they belong to, and every point moved by the proj4
//! port (`proj.rs`) — on a datum's grids where they are loaded, on its
//! 7-parameter shift otherwise, and again without the grid for a point past
//! its edge.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

use crate::proj::{Def, Ellps, Proj, Shift, Subgrid, put_grid, transform};

/// A datum a German plane is stated in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Datum {
    Dhdn,
    Pd83,
    Rd83,
    S4283,
    Dbref,
}

impl Datum {
    pub fn name(self) -> &'static str {
        match self {
            Datum::Dhdn => "DHDN",
            Datum::Pd83 => "PD83",
            Datum::Rd83 => "RD83",
            Datum::S4283 => "S4283",
            Datum::Dbref => "DBREF",
        }
    }

    pub fn from_name(name: &str) -> Option<Datum> {
        [Datum::Dhdn, Datum::Pd83, Datum::Rd83, Datum::S4283, Datum::Dbref]
            .into_iter()
            .find(|d| d.name() == name)
    }

    fn label(self) -> &'static str {
        match self {
            Datum::Dhdn => "DHDN",
            Datum::Pd83 => "PD/83",
            Datum::Rd83 => "RD/83",
            Datum::S4283 => "42/83",
            Datum::Dbref => "DB_REF",
        }
    }

    fn ellps(self) -> Ellps {
        if self == Datum::S4283 { Ellps::Krass } else { Ellps::Bessel }
    }

    fn helmert(self) -> [f64; 7] {
        match self {
            Datum::Dhdn => [598.1, 73.7, 418.2, 0.202, 0.045, -2.455, 6.7],
            Datum::Pd83 => [599.4, 72.4, 419.2, -0.062, -0.022, -2.723, 6.46],
            Datum::Rd83 => [612.4, 77.0, 440.2, -0.054, 0.057, -2.797, 2.55],
            Datum::S4283 => [24.9, -126.4, -93.2, -0.063, -0.247, -0.041, 1.01],
            Datum::Dbref => [584.9636, 107.7175, 413.8067, 1.1155, 0.2824, -3.1384, 7.9922],
        }
    }
}

/// The datum and zone an EPSG code stands for (`planeOf`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub datum: Datum,
    pub zone: Option<i32>,
    pub soldner: bool,
}

pub fn plane_of(code: i64) -> Option<Plane> {
    let p = |datum, zone| {
        Some(Plane {
            datum,
            zone: Some(zone),
            soldner: false,
        })
    };
    match code {
        5681..=5685 => p(Datum::Dbref, (code - 5680) as i32),
        5676..=5680 => p(Datum::Dhdn, if code == 5680 { 1 } else { (code - 5674) as i32 }),
        3396..=3397 => p(Datum::Pd83, (code - 3393) as i32),
        3398..=3399 => p(Datum::Rd83, (code - 3394) as i32),
        2397..=2399 => p(Datum::S4283, (code - 2394) as i32),
        3068 => Some(Plane {
            datum: Datum::Dhdn,
            zone: None,
            soldner: true,
        }),
        _ => None,
    }
}

// ── Systems ─────────────────────────────────────────────────────────────────

/// A coordinate system: a datum with one projection, cut into strips.
pub struct System {
    pub key: &'static str,
    pub name: &'static str,
    pub zones: Vec<(Option<i32>, i64)>,
    centre: fn(Option<i32>) -> f64,
}

fn gk_centre(z: Option<i32>) -> f64 {
    f64::from(z.unwrap_or(0) * 3)
}
fn utm_centre(z: Option<i32>) -> f64 {
    f64::from(z.unwrap_or(0) * 6 - 183)
}
fn soldner_centre(_: Option<i32>) -> f64 {
    13.6272036666667
}

pub fn systems() -> &'static [System] {
    static SYSTEMS: OnceLock<Vec<System>> = OnceLock::new();
    SYSTEMS.get_or_init(|| {
        let range = |from: i32, to: i32, code: fn(i32) -> i64| (from..=to).map(|z| (Some(z), code(z))).collect::<Vec<_>>();
        vec![
            System {
                key: "ETRS89_UTM",
                name: "ETRS89 / UTM",
                zones: range(28, 38, |z| 25800 + i64::from(z)),
                centre: utm_centre,
            },
            System {
                key: "DBREF_GK",
                name: "DB_REF / Gauß-Krüger",
                zones: range(1, 5, |z| 5680 + i64::from(z)),
                centre: gk_centre,
            },
            System {
                key: "DHDN_GK",
                name: "DHDN / Gauß-Krüger",
                zones: range(1, 5, |z| if z == 1 { 5680 } else { 5674 + i64::from(z) }),
                centre: gk_centre,
            },
            System {
                key: "PD83_GK",
                name: "PD/83 / Gauß-Krüger",
                zones: range(3, 4, |z| 3393 + i64::from(z)),
                centre: gk_centre,
            },
            System {
                key: "RD83_GK",
                name: "RD/83 / Gauß-Krüger",
                zones: range(4, 5, |z| 3394 + i64::from(z)),
                centre: gk_centre,
            },
            System {
                key: "S4283_GK",
                name: "42/83 / Gauß-Krüger",
                zones: range(3, 5, |z| 2394 + i64::from(z)),
                centre: gk_centre,
            },
            System {
                key: "DHDN_SOLDNER",
                name: "DHDN / Soldner Berlin",
                zones: vec![(None, 3068)],
                centre: soldner_centre,
            },
            System {
                key: "WGS84_UTM_N",
                name: "WGS 84 / UTM Nord",
                zones: range(1, 60, |z| 32600 + i64::from(z)),
                centre: utm_centre,
            },
            System {
                key: "WGS84_UTM_S",
                name: "WGS 84 / UTM Süd",
                zones: range(1, 60, |z| 32700 + i64::from(z)),
                centre: utm_centre,
            },
        ]
    })
}

fn system_of_code(code: i64) -> Option<(&'static System, Option<i32>)> {
    systems().iter().find_map(|s| s.zones.iter().find(|(_, c)| *c == code).map(|(z, _)| (s, *z)))
}

/// The system a plane belongs to (`crsSystem`).
pub fn crs_system(code: i64) -> Option<&'static str> {
    system_of_code(code).map(|(s, _)| s.key)
}

/// The meridian strip of a plane within its system (`crsZone`).
pub fn crs_zone(code: i64) -> Option<i32> {
    system_of_code(code).and_then(|(_, z)| z)
}

/// The central meridian of a plane's strip [°] (`crsCentre`).
pub fn crs_centre(code: i64) -> Option<f64> {
    system_of_code(code).map(|(s, z)| (s.centre)(z))
}

/// Whether `key` names a system.
pub fn is_crs_system(key: &str) -> bool {
    systems().iter().any(|s| s.key == key)
}

/// What a system is called.
pub fn crs_system_name(key: &str) -> &'static str {
    systems().iter().find(|s| s.key == key).map(|s| s.name).unwrap_or("")
}

/// The strips of a system: (zone, code, centre).
pub fn system_zones(key: &str) -> Vec<(Option<i32>, i64, f64)> {
    systems()
        .iter()
        .find(|s| s.key == key)
        .map(|s| s.zones.iter().map(|(z, c)| (*z, *c, (s.centre)(*z))).collect())
        .unwrap_or_default()
}

/// The plane of system `key` whose strip lies nearest to `lng` (`systemCodeAt`).
pub fn system_code_at(key: &str, lng: f64) -> Option<i64> {
    let zones = system_zones(key);
    let first = zones.first()?;
    let mut best = *first;
    for z in &zones {
        if (z.2 - lng).abs() < (best.2 - lng).abs() {
            best = *z;
        }
    }
    Some(best.1)
}

/// Gauss-Krüger zone of a German plane (`gkZone`).
pub fn gk_zone(code: i64) -> Option<i32> {
    plane_of(code).and_then(|p| p.zone)
}

/// Which datum a plane is stated in (`crsDatum`).
pub fn crs_datum(code: i64) -> Option<Datum> {
    plane_of(code).map(|p| p.datum)
}

/// What a supported EPSG code is called (`crsName`).
pub fn crs_name(code: i64) -> Option<String> {
    if let Some(p) = plane_of(code) {
        return Some(if p.soldner {
            "DHDN / Soldner Berlin".into()
        } else {
            format!("{} / GK Zone {}", p.datum.label(), p.zone.unwrap_or(0))
        });
    }
    match code {
        25828..=25838 => Some(format!("ETRS89 / UTM Zone {}N", code - 25800)),
        32601..=32660 => Some(format!("WGS 84 / UTM Zone {}N", code - 32600)),
        32701..=32760 => Some(format!("WGS 84 / UTM Zone {}S", code - 32700)),
        _ => None,
    }
}

/// How a CRS is written out in full (`crsLabel`).
pub fn crs_label(code: Option<i64>) -> String {
    match code {
        None | Some(0) => String::new(),
        Some(c) => match crs_name(c) {
            Some(name) => format!("EPSG {c} – {name}"),
            None => format!("EPSG {c}"),
        },
    }
}

/// The UTM plane suggested for a new track at `lng`, `lat` (`epsgForLngLat`).
pub fn epsg_for_lng_lat(lng: f64, lat: f64) -> i64 {
    let zone = ((lng + 180.0) / 6.0).floor() as i64 + 1;
    if lat >= 0.0 && (28..=38).contains(&zone) {
        return 25800 + zone;
    }
    (if lat < 0.0 { 32700 } else { 32600 }) + zone
}

// ── The grids (ntv2Grid.js) ─────────────────────────────────────────────────

/// The grids the core knows: key, datum, whether it is a base grid.
pub const GRIDS: &[(&str, Datum, bool)] = &[
    ("BETA2007", Datum::Dhdn, true),
    ("NTV2TH", Datum::Pd83, true),
    ("HETA2010", Datum::Dhdn, false),
    ("SETA2016", Datum::Dhdn, false),
];

fn loaded() -> &'static RwLock<HashSet<String>> {
    static LOADED: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
    LOADED.get_or_init(|| RwLock::new(HashSet::new()))
}

/// A grid the page (or the server) has read, registered under its key: from
/// now on its datum converts on it (`proj4.nadgrid` and `rebuildLists`).
pub fn load_grid(key: &str, subgrids: Vec<Subgrid>) {
    put_grid(key, subgrids);
    loaded().write().expect("grids").insert(key.to_string());
    CONVERTERS.with(|c| c.borrow_mut().clear());
}

/// The `+nadgrids=` list of a datum, or None while it has no grid (`nadgridsList`).
pub fn nadgrids_list(datum: Datum) -> Option<String> {
    let done = loaded().read().expect("grids");
    let regional = GRIDS.iter().filter(|(k, d, base)| !base && *d == datum && done.contains(*k));
    let base = GRIDS.iter().filter(|(k, d, base)| *base && *d == datum && done.contains(*k));
    let keys: Vec<&str> = regional.chain(base).map(|(k, _, _)| *k).collect();
    if keys.is_empty() {
        return None;
    }
    Some(
        keys.iter()
            .enumerate()
            .map(|(i, k)| if i < keys.len() - 1 { format!("@{k}") } else { (*k).to_string() })
            .collect::<Vec<_>>()
            .join(","),
    )
}

// ── Definitions and conversions ─────────────────────────────────────────────

fn plane_def(plane: Plane, shift: Shift) -> Def {
    if plane.soldner {
        return Def::Cass {
            lat0: 52.4186482777778,
            lon0: 13.6272036666667,
            x0: 40000.0,
            y0: 10000.0,
            ellps: Ellps::Bessel,
            shift,
        };
    }
    let zone = plane.zone.unwrap_or(0);
    Def::Tmerc {
        lon0: f64::from(zone * 3),
        x0: f64::from(zone) * 1_000_000.0 + 500_000.0,
        ellps: plane.datum.ellps(),
        shift,
    }
}

fn shift_for(datum: Datum) -> Shift {
    match nadgrids_list(datum) {
        Some(list) => Shift::Grids(list),
        None => Shift::Helmert(datum.helmert()),
    }
}

/// Why a code has no plane.
#[derive(Debug, Clone, PartialEq)]
pub struct Unsupported(pub String);

/// The definition of a supported EPSG code (`projStringFor`).
pub fn def_for(code: i64) -> Result<Def, Unsupported> {
    if let Some(p) = plane_of(code) {
        return Ok(plane_def(p, shift_for(p.datum)));
    }
    match code {
        25828..=25838 => Ok(Def::Utm {
            zone: (code - 25800) as i32,
            south: false,
        }),
        32601..=32660 => Ok(Def::Utm {
            zone: (code - 32600) as i32,
            south: false,
        }),
        32701..=32760 => Ok(Def::Utm {
            zone: (code - 32700) as i32,
            south: true,
        }),
        _ => Err(Unsupported(format!("Unsupported CRS: {code}"))),
    }
}

/// The same plane without its grid, where it is on one (`offGridProjString`).
fn off_grid_def(code: Option<i64>) -> Option<Def> {
    let p = plane_of(code?)?;
    nadgrids_list(p.datum)?;
    Some(plane_def(p, Shift::Helmert(p.datum.helmert())))
}

thread_local! {
    static CONVERTERS: RefCell<HashMap<String, Proj>> = RefCell::new(HashMap::new());
}

fn with_proj<T>(def: &Def, f: impl FnOnce(&Proj) -> T) -> T {
    let key = format!("{def:?}");
    CONVERTERS.with(|c| {
        let mut c = c.borrow_mut();
        let proj = c.entry(key).or_insert_with(|| Proj::new(def));
        f(proj)
    })
}

fn convert(from: &Def, to: &Def, x: f64, y: f64) -> (f64, f64) {
    let source = with_proj(from, Clone::clone);
    with_proj(to, |dest| transform(&source, dest, x, y))
}

/// proj4 with the one way the grid fails: a point outside comes back NaN and
/// is converted again without the grid (`project`).
fn project(from: &Def, to: &Def, x: f64, y: f64, from_code: Option<i64>, to_code: Option<i64>) -> (f64, f64) {
    let out = convert(from, to, x, y);
    if out.0.is_finite() && out.1.is_finite() {
        return out;
    }
    let from2 = off_grid_def(from_code).unwrap_or_else(|| from.clone());
    let to2 = off_grid_def(to_code).unwrap_or_else(|| to.clone());
    if from2 == *from && to2 == *to { out } else { convert(&from2, &to2, x, y) }
}

/// A WGS 84 [lng, lat] in the plane `code` (`wgs84ToUTM`): (easting, northing).
pub fn wgs84_to_plane(lng: f64, lat: f64, code: i64) -> Result<(f64, f64), Unsupported> {
    let to = def_for(code)?;
    Ok(project(&Def::Wgs84, &to, lng, lat, None, Some(code)))
}

/// A plane point as WGS 84 (lng, lat) (`utmToWgs84`).
pub fn plane_to_wgs84(easting: f64, northing: f64, code: i64) -> Result<(f64, f64), Unsupported> {
    let from = def_for(code)?;
    Ok(project(&from, &Def::Wgs84, easting, northing, Some(code), None))
}

/// A polyline out of one plane into WGS 84 (`planeCoordsToWgs84`).
pub fn plane_coords_to_wgs84(points: &[[f64; 2]], code: i64) -> Result<Vec<[f64; 2]>, Unsupported> {
    let from = def_for(code)?;
    Ok(points
        .iter()
        .map(|[e, n]| {
            let (lng, lat) = project(&from, &Def::Wgs84, *e, *n, Some(code), None);
            [lng, lat]
        })
        .collect())
}

/// A point moved from one plane to another (`transformPlanePoint`).
pub fn transform_plane_point(easting: f64, northing: f64, from_code: i64, to_code: i64) -> Result<(f64, f64), Unsupported> {
    if from_code == to_code {
        return Ok((easting, northing));
    }
    let from = def_for(from_code)?;
    let to = def_for(to_code)?;
    Ok(project(&from, &to, easting, northing, Some(from_code), Some(to_code)))
}

/// A grid bearing at a point carried from one plane's grid to another's (`transformGridBearing`).
pub fn transform_grid_bearing(easting: f64, northing: f64, bearing: f64, from_code: i64, to_code: i64) -> Result<f64, Unsupported> {
    if from_code == to_code {
        return Ok(bearing);
    }
    let from = def_for(from_code)?;
    let to = def_for(to_code)?;
    let rad = bearing * std::f64::consts::PI / 180.0;
    let (e1, n1) = project(&from, &to, easting, northing, Some(from_code), Some(to_code));
    let (e2, n2) = project(
        &from,
        &to,
        easting + 10.0 * rad.sin(),
        northing + 10.0 * rad.cos(),
        Some(from_code),
        Some(to_code),
    );
    Ok(((e2 - e1).atan2(n2 - n1) * 180.0 / std::f64::consts::PI + 360.0) % 360.0)
}
