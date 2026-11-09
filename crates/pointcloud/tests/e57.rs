// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The E57 reader against what libE57Format wrote and numpy computed from it
//! (`packages/pointcloud/tests/fixtures/e57/`, `tools/e57fixtures.py`): counts,
//! sums, sampled points and colours, scan after scan with each pose.

use rail_across_pointcloud::import::{Header, read_cloud_header, read_cloud_points};
use rail_across_pointcloud::source::BytesSource;
use rail_across_pointcloud::{Error, Progress, ReadOptions};
use serde_json::Value;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/pointcloud/test/fixtures/e57/");

fn expected() -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}expected.json")).unwrap()).unwrap()
}

fn fixture(name: &str) -> BytesSource {
    BytesSource(std::fs::read(format!("{FIXTURES}{name}")).unwrap())
}

/// Every point: x, y, z, intensity, and the colour where there is one.
fn read_all(source: &BytesSource, header: &Header) -> Vec<[f64; 7]> {
    let mut pts = Vec::new();
    read_cloud_points(source, header, ReadOptions::default(), &mut |b| {
        for i in 0..b.count {
            // Stored as 8 bits times 257 by colorLimits: the 8 bits are the high byte.
            let c = |v: &Option<Vec<u16>>| v.as_ref().map(|v| f64::from(v[i] >> 8)).unwrap_or(-1.0);
            pts.push([b.x[i], b.y[i], b.z[i], f64::from(b.intensity[i]), c(&b.red), c(&b.green), c(&b.blue)]);
        }
        Ok(())
    })
    .unwrap();
    pts
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 5e-7
}

/// The points against the expectation: count, sums, every sampled point.
fn expect_matches(pts: &[[f64; 7]], exp: &Value, with_intensity: bool) {
    assert_eq!(pts.len() as u64, exp["count"].as_u64().unwrap());
    for k in 0..3 {
        let sum: f64 = pts.iter().map(|p| p[k]).sum();
        assert!((sum - f(&exp["sum"][k])).abs() / pts.len() as f64 <= 1e-6, "sum {k}");
    }
    if with_intensity {
        let sum: f64 = pts.iter().map(|p| p[3]).sum();
        assert_eq!(sum, f(&exp["intensitySum"]));
    }
    for s in exp["sample"].as_array().unwrap() {
        let p = pts[s[0].as_u64().unwrap() as usize];
        assert!(close(p[0], f(&s[1])) && close(p[1], f(&s[2])) && close(p[2], f(&s[3])), "point {}", s[0]);
        if with_intensity {
            assert_eq!(p[3], f(&s[4]));
        }
    }
}

fn e57(header: &Header) -> &rail_across_pointcloud::e57::E57Header {
    match header {
        Header::E57(h) => h,
        Header::Las(_) => panic!("not read as E57"),
    }
}

#[test]
fn scaled_integers_of_odd_widths_the_pose_turned_and_shifted_invalid_points_left_out() {
    let source = fixture("scaled_pose.e57");
    let header = read_cloud_header(&source).unwrap();
    let h = e57(&header);
    assert_eq!((h.version.as_str(), h.point_count, h.coordinate_metadata.as_str()), ("1.0", 30000, "EPSG:5678"));
    assert_eq!(h.scale, [0.0001; 3]);
    let exp = &expected()["scaled_pose.e57"];
    for k in 0..3 {
        assert!(h.min[k] <= f(&exp["min"][k]) && h.max[k] >= f(&exp["max"][k]));
        assert!(f(&exp["min"][k]) - h.min[k] < 10.0);
    }
    expect_matches(&read_all(&source, &header), exp, true);
}

#[test]
fn spherical_doubles_and_cartesian_singles_scan_after_scan_each_with_its_pose() {
    let source = fixture("spherical_two_scans.e57");
    let header = read_cloud_header(&source).unwrap();
    let pts = read_all(&source, &header);
    let exp = expected();
    let scans = exp["spherical_two_scans.e57"]["scans"].as_array().unwrap();
    let n = scans[0]["count"].as_u64().unwrap() as usize;
    expect_matches(&pts[..n], &scans[0], true);
    expect_matches(&pts[n..], &scans[1], false);
    // Without an intensity field the points come with none.
    assert!(pts[n..].iter().all(|p| p[3] == 0.0));
}

#[test]
fn colours_as_their_eight_bits() {
    let source = fixture("color_rgb.e57");
    let header = read_cloud_header(&source).unwrap();
    assert!(header.rgb());
    let exp = &expected()["color_rgb.e57"];
    let pts = read_all(&source, &header);
    // No intensity field in this one.
    expect_matches(&pts, exp, false);
    for k in 0..3 {
        let sum: f64 = pts.iter().map(|p| p[4 + k]).sum();
        assert_eq!(sum, f(&exp["rgbSum"][k]), "colour {k}");
    }
    for s in exp["rgbSample"].as_array().unwrap() {
        let p = pts[s[0].as_u64().unwrap() as usize];
        assert_eq!([p[4], p[5], p[6]], [f(&s[1]), f(&s[2]), f(&s[3])]);
    }
}

#[test]
fn progress_counts_records_and_ends_at_all_of_them_and_an_abort_stops_it() {
    let source = fixture("scaled_pose.e57");
    let header = read_cloud_header(&source).unwrap();
    let mut last = Progress::default();
    let mut on = |p: Progress| last = p;
    read_cloud_points(
        &source,
        &header,
        ReadOptions {
            on_progress: Some(&mut on),
            aborted: None,
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!((last.points, last.total_points), (30000.0, 30000.0));
    assert_eq!(last.bytes, last.total_bytes);
    let stop = || true;
    let r = read_cloud_points(
        &source,
        &header,
        ReadOptions {
            on_progress: None,
            aborted: Some(&stop),
        },
        &mut |_| Ok(()),
    );
    assert_eq!(r, Err(Error::Aborted));
}

#[test]
fn a_file_cut_short_or_none_at_all_is_refused() {
    let bytes = std::fs::read(format!("{FIXTURES}scaled_pose.e57")).unwrap();
    let cut = read_cloud_header(&BytesSource(bytes[..100000].to_vec())).unwrap_err();
    assert!(cut.message().contains("incomplete"), "{}", cut.message());
    let none = rail_across_pointcloud::e57::read_e57_header(&BytesSource(vec![0; 400])).unwrap_err();
    assert!(none.message().contains("not an E57"), "{}", none.message());
}
