// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The terrain and MDB pieces of the Rust server, held to what the Node
//! server's own tests ask of its JavaScript (`server/test/terrain.test.js`).
//! The GeoTIFFs here were written by geotiff.js, as that test writes them.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use rail_across_server::mdb::parse_csv;
use rail_across_server::terrain::dem::{Dgm5, render};
use rail_across_server::terrain::store::TileStore;
use rail_across_server::terrain::tile::{parse_tif, parse_xyz, read_tile};
use rail_across_server::terrain::{Fmt, sample, sources, to_plane};
use serde_json::json;

fn xyz(e0: f64, n0: f64, size: usize, h: impl Fn(f64, f64) -> f64) -> String {
    let mut lines = Vec::new();
    let mut n = n0 + size as f64 - 0.5;
    while n > n0 {
        let mut e = e0 + 0.5;
        while e < e0 + size as f64 {
            lines.push(format!("{e} {n} {:.3}", h(e, n)));
            e += 1.0;
        }
        n -= 1.0;
    }
    lines.join("\n")
}

fn zip(files: &[(&str, String)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, text) in files {
        w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(text.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

#[test]
fn an_xyz_list_is_placed_by_its_coordinates_and_read_bilinearly() {
    let t = parse_xyz(&xyz(1000.0, 2000.0, 4, |e, n| e + 10.0 * n), "x").unwrap();
    assert_eq!((t.rows, t.cols, t.x0, t.y0), (4, 4, 1000.5, 2003.5));
    assert!(close(t.height(1001.25, 2001.75).unwrap(), 1001.25 + 20017.5, 3));
    assert!(close(t.height(999.0, 2001.5).unwrap(), 1000.5 + 20015.0, 3));
    let gaps = parse_xyz(&xyz(0.0, 0.0, 3, |e, _| if e > 2.0 { -9999.0 } else { 5.0 }), "x").unwrap();
    assert_eq!(gaps.height(1.0, 1.0), Some(5.0));
    assert_eq!(gaps.height(2.2, 1.0), None);
}

#[test]
fn a_geotiff_is_placed_by_its_tie_point_corner_or_centre() {
    let area = parse_tif(include_bytes!("fixtures/tie-area.tif"), "a").unwrap();
    assert_eq!((area.rows, area.cols, area.x0, area.y0), (2, 3, 500.5, 1001.5));
    assert_eq!(area.height(501.5, 1000.5), Some(4.0));
    let point = parse_tif(include_bytes!("fixtures/tie-point.tif"), "p").unwrap();
    assert_eq!((point.x0, point.y0), (500.0, 1002.0));
}

#[test]
fn a_zip_of_several_lists_is_one_grid() {
    let z = zip(&[("a.xyz", xyz(0.0, 0.0, 2, |_, _| 1.0)), ("b.xyz", xyz(2.0, 0.0, 2, |_, _| 2.0))]);
    let t = read_tile(&z, Fmt::XyzZip, "z").unwrap();
    assert_eq!((t.rows, t.cols), (2, 4));
    assert_eq!(t.height(3.0, 1.0), Some(2.0));
}

fn tmp() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("ra-terrain-{}", rail_across_server::util::uuid()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_store_downloads_once_keeps_the_tile_on_disk_and_remembers_a_missing_one() {
    let th = sources().iter().find(|s| s.id == "dgm1-th").unwrap();
    let asked = Arc::new(Mutex::new(Vec::<String>::new()));
    let a2 = asked.clone();
    let fetch: rail_across_server::terrain::store::Fetch = Arc::new(move |url: &str| {
        a2.lock().unwrap().push(url.to_string());
        Ok((url.contains("2020-2025") && url.contains("_640_5700_")).then(|| zip(&[("t.xyz", xyz(640000.0, 5700000.0, 3, |_, _| 7.0))])))
    });
    let dir = tmp();
    let store = TileStore::new(dir.clone(), fetch.clone());
    let (a, _) = store.get(th, 640, 5700);
    assert_eq!(a.as_ref().unwrap().label, "dgm1-th 2020-2025");
    let (b, _) = store.get(th, 640, 5700);
    assert!(Arc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap()));
    assert_eq!(asked.lock().unwrap().len(), 1);
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["dgm1-th_640_5700.dgm1"]);
    let again = TileStore::new(dir.clone(), fetch).get(th, 640, 5700);
    assert_eq!(again.0.unwrap().height(640001.0, 5700001.0), Some(7.0));
    assert_eq!(asked.lock().unwrap().len(), 1);
    assert!(store.get(th, 1, 1).0.is_none());
    assert!(!store.get(th, 1, 1).1);
    assert_eq!(asked.lock().unwrap().len(), 3);
    let calls = Arc::new(Mutex::new(0));
    let c2 = calls.clone();
    let down = TileStore::new(
        tmp(),
        Arc::new(move |_: &str| {
            *c2.lock().unwrap() += 1;
            Err("portal down".into())
        }),
    );
    let got = down.get(th, 2, 2);
    assert!(got.0.is_none() && got.1);
    down.get(th, 2, 2);
    assert_eq!(*calls.lock().unwrap(), 2);
}

/// A store whose Thüringen tiles are a plane rising to the east.
fn plane_store() -> TileStore {
    TileStore::new(
        tmp(),
        Arc::new(|url: &str| {
            if !url.contains("2020-2025") {
                return Ok(None);
            }
            let tail = url.rsplit("dgm1_32_").next().unwrap();
            let mut it = tail.split('_');
            let ek: f64 = it.next().unwrap().parse().unwrap();
            let nk: f64 = it.next().unwrap().parse().unwrap();
            Ok(Some(zip(&[(
                "t.xyz",
                xyz(ek * 1000.0, nk * 1000.0, 1000, |e, _| 100.0 + (e - ek * 1000.0) / 100.0),
            )])))
        }),
    )
}

#[test]
fn heights_and_relief_tiles_come_from_the_plane() {
    let store = plane_store();
    let (e, _) = to_plane(25832, 11.97, 51.48);
    let got = sample(&[json!([11.97, 51.48]), json!([-3, 40]), json!(["x"])], &store);
    let h = got["heights"][0].as_f64().unwrap();
    assert!(close(h, 100.0 + (e - (e / 1000.0).floor() * 1000.0) / 100.0, 1));
    assert_eq!(got["sources"], json!(["dgm1-th 2020-2025", null, null]));
    assert_eq!(got["heights"][1], json!(null));

    let z = 16;
    let x = ((11.33 + 180.0) / 360.0 * 2f64.powi(z)).floor() as i64;
    let y = ((1.0 - (50.98f64.to_radians().tan()).asinh() / std::f64::consts::PI) / 2.0 * 2f64.powi(z)).floor() as i64;
    let (png, keep) = render(z as i64, x, y, &store, &Dgm5::default()).unwrap();
    let png = png.unwrap();
    assert!(keep);
    assert_eq!(&png[1..4], b"PNG");
    let at = png.windows(4).position(|w| w == b"IDAT").unwrap();
    let len = u32::from_be_bytes(png[at - 4..at].try_into().unwrap()) as usize;
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&png[at + 4..at + 4 + len]).read_to_end(&mut raw).unwrap();
    let hh = |r: usize, c: usize| {
        (f64::from(raw[r * 769 + 1 + 3 * c]) * 65536.0 + f64::from(raw[r * 769 + 2 + 3 * c]) * 256.0 + f64::from(raw[r * 769 + 3 + 3 * c])) * 0.01 - 10000.0
    };
    assert!(hh(0, 0) > 100.0 && hh(0, 0) < 110.0);
    assert!(hh(128, 255) > hh(128, 0));
    assert!(render(16, 0, 0, &store, &Dgm5::default()).unwrap().0.is_none());
}

#[test]
fn mdb_export_csv_with_quotes_doubled_quotes_and_line_breaks() {
    let rows = parse_csv("A,B,C\n1,\"x, \"\"y\"\"\",\n2,\"two\nlines\",3\n");
    let as_pairs = |r: &Vec<(String, String)>| r.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>();
    assert_eq!(as_pairs(&rows[0]), vec!["A=1", "B=x, \"y\"", "C="]);
    assert_eq!(as_pairs(&rows[1]), vec!["A=2", "B=two\nlines", "C=3"]);
    assert!(rail_across_server::mdb::convert_mdb(b"not a database at all").is_err_and(|e| e.code == "not_a_database"));
}
