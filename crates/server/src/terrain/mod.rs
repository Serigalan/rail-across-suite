// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Ground heights from the DGM1 of the Länder (`terrain/`, decision 0005):
//! the kilometre tiles fetched once, kept, read bilinearly for points and
//! drawn into terrain-RGB relief tiles for the map.

pub mod dem;
pub mod store;
pub mod tile;

use serde_json::{Value, json};

use store::TileStore;

const TH: &str = "https://geoportal.geoportal-th.de/hoehendaten/DGM";
const SN: &str = "https://geocloud.landesvermessung.sachsen.de/public.php/dav/files/JCcXyifaNdLDnxZ";
const NW: &str = "https://www.wcs.nrw.de/geobasis/wcs_nw_dgm";

/// How a source's files are packed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fmt {
    XyzZip,
    TifZip,
    Tif,
}

/// One Land's DGM1 (`sources.js`).
pub struct Source {
    pub id: &'static str,
    pub name: &'static str,
    pub crs: i64,
    pub bbox: [f64; 4],
    pub size: f64,
    pub fmt: Fmt,
    pub vintages: Vec<(Option<&'static str>, fn(i64, i64) -> String)>,
    pub offset: [f64; 2],
}

impl Source {
    pub fn covers(&self, lng: f64, lat: f64) -> bool {
        self.bbox[0] <= lng && lng <= self.bbox[2] && self.bbox[1] <= lat && lat <= self.bbox[3]
    }
    /// Lower-left corner of the tile holding (e, n), in whole kilometres.
    pub fn tile_of(&self, e: f64, n: f64) -> (i64, i64) {
        let k = |v: f64, o: f64| ((o + self.size * ((v - o) / self.size).floor()) / 1000.0).floor() as i64;
        (k(e, self.offset[0]), k(n, self.offset[1]))
    }
}

pub fn sources() -> &'static [Source] {
    static S: std::sync::LazyLock<Vec<Source>> = std::sync::LazyLock::new(|| {
        vec![
            Source {
                id: "dgm1-th",
                name: "Thüringen",
                crs: 25832,
                bbox: [9.85, 50.17, 12.68, 51.68],
                size: 1000.0,
                fmt: Fmt::XyzZip,
                vintages: vec![
                    (Some("2020-2025"), |e, n| format!("{TH}/dgm_2020-2025/dgm1_32_{e}_{n}_1_th_2020-2025.zip")),
                    (Some("2014-2019"), |e, n| format!("{TH}/dgm_2014-2019/dgm1_{e}_{n}_1_th_2014-2019.zip")),
                ],
                offset: [0.0, 0.0],
            },
            Source {
                id: "dgm1-sn",
                name: "Sachsen",
                crs: 25833,
                bbox: [11.85, 50.15, 15.05, 51.70],
                size: 2000.0,
                fmt: Fmt::TifZip,
                vintages: vec![(None, |e, n| format!("{SN}/dgm1_33{e}_{n}_2_sn_tiff.zip"))],
                offset: [0.0, 0.0],
            },
            Source {
                id: "dgm1-be",
                name: "Berlin",
                crs: 25833,
                bbox: [13.07, 52.33, 13.78, 52.68],
                size: 2000.0,
                fmt: Fmt::XyzZip,
                vintages: vec![(None, |e, n| format!("https://gdi.berlin.de/data/dgm1/atom/DGM1_{e}_{n}.zip"))],
                offset: [0.0, 0.0],
            },
            Source {
                id: "dgm1-bb",
                name: "Brandenburg",
                crs: 25833,
                bbox: [11.25, 51.35, 14.78, 53.57],
                size: 1000.0,
                fmt: Fmt::TifZip,
                vintages: vec![(None, |e, n| format!("https://data.geobasis-bb.de/geobasis/daten/dgm/tif/dgm_33{e}-{n}.zip"))],
                offset: [0.0, 0.0],
            },
            Source {
                id: "dgm1-by",
                name: "Bayern",
                crs: 25832,
                bbox: [8.95, 47.26, 13.85, 50.57],
                size: 1000.0,
                fmt: Fmt::Tif,
                vintages: vec![(None, |e, n| format!("https://download1.bayernwolke.de/a/dgm/dgm1/{e}_{n}.tif"))],
                offset: [0.0, 0.0],
            },
            Source {
                id: "dgm1-bw",
                name: "Baden-Württemberg",
                crs: 25832,
                bbox: [7.50, 47.53, 10.50, 49.80],
                size: 2000.0,
                fmt: Fmt::XyzZip,
                vintages: vec![(None, |e, n| format!("https://opengeodata.lgl-bw.de/data/dgm/dgm1_32_{e}_{n}_2_bw.zip"))],
                offset: [1000.0, 0.0],
            },
            Source {
                id: "dgm1-nw",
                name: "Nordrhein-Westfalen",
                crs: 25832,
                bbox: [5.85, 50.32, 9.47, 52.54],
                size: 1000.0,
                fmt: Fmt::Tif,
                vintages: vec![(None, |e, n| {
                    format!(
                        "{NW}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID=nw_dgm&FORMAT=image/tiff&SUBSET=x({},{})&SUBSET=y({},{})",
                        e * 1000,
                        e * 1000 + 1000,
                        n * 1000,
                        n * 1000 + 1000
                    )
                })],
                offset: [0.0, 0.0],
            },
        ]
    });
    &S
}

/// A WGS84 point in the plane of a source.
pub fn to_plane(crs: i64, lng: f64, lat: f64) -> (f64, f64) {
    rail_across_geom::coords::wgs84_to_plane(lng, lat, crs).unwrap_or((f64::NAN, f64::NAN))
}

/// Heights for WGS84 points (`sample`): { heights, sources }.
pub fn sample(points: &[Value], store: &TileStore) -> Value {
    let mut heights = Vec::new();
    let mut labels = Vec::new();
    for p in points {
        let (mut z, mut label) = (Value::Null, Value::Null);
        let (lng, lat) = match p.as_array() {
            Some(a) => (rail_across_js::js_number(a.first()), rail_across_js::js_number(a.get(1))),
            None => (f64::NAN, f64::NAN),
        };
        if lng.is_finite() && lat.is_finite() {
            for s in sources() {
                if !s.covers(lng, lat) {
                    continue;
                }
                let (e, n) = to_plane(s.crs, lng, lat);
                let (ek, nk) = s.tile_of(e, n);
                let (tile, _) = store.get(s, ek, nk);
                if let Some(h) = tile.as_ref().and_then(|t| t.height(e, n)) {
                    z = crate::util::n(rail_across_js::js_round(h * 100.0) / 100.0);
                    label = json!(tile.unwrap().label.clone());
                    break;
                }
            }
        }
        heights.push(z);
        labels.push(label);
    }
    json!({ "heights": heights, "sources": labels })
}
