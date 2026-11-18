// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A track's plan by column (core 6.0): rows to columns and back, what lies within a millimetre closed and what
//! lies further apart refused, and what the door finds of columns that do not hold together.

use rail_across_doc::alignment::{breaches, to_columns, to_rows};
use serde_json::{Value, json};

/// A track of every kind of element, its nodes as a rough plan states them — rough_closed makes them exact.
fn rows() -> Vec<Value> {
    vec![
        json!({ "elementType": "straight", "startNode": [500000.0, 5700000.0], "endNode": [500100.0, 5700000.0], "bearing": 90, "length": 100, "designSpeed": 80 }),
        json!({ "elementType": "transition", "transitionType": "clothoid", "startNode": [500100.0, 5700000.0], "endNode": [500160.0, 5699998.8], "bearing": 90, "length": 60, "r1": null, "r2": 500, "cantStart": 0, "cantEnd": 60 }),
        json!({ "elementType": "arc", "startNode": [500160.0, 5699998.8], "endNode": [500259.0, 5699984.4], "bearing": 93.4377, "length": 100, "radius": 500, "cant": 60, "role": "follow" }),
        json!({ "elementType": "transition", "transitionType": "bloss", "startNode": [500259.0, 5699984.4], "endNode": [500318.0, 5699972.0], "bearing": 104.8969, "length": 60, "r1": 500, "r2": null, "cantStart": 60, "cantEnd": 0 }),
        json!({ "elementType": "straight", "startNode": [500318.0, 5699972.0], "endNode": [500367.0, 5699961.0], "bearing": 108.3346, "endBearing": 110.0, "length": 50, "switchId": "a1b2c3d4-0000-4000-8000-000000000001", "switchRoute": "branch", "switchBranch": true }),
        json!({ "elementType": "straight", "startNode": [500367.0, 5699961.0], "endNode": [500416.0, 5699944.0], "bearing": 110.0, "length": 50 }),
    ]
}

/// The rows a first pass makes, which a second pass keeps.
fn chained() -> Vec<Value> {
    let first = to_columns(&rough_closed()).unwrap().unwrap();
    to_rows(&first)
}

/// The rough rows with each node where the element's values lead, walked by hand from the first start point.
fn rough_closed() -> Vec<Value> {
    let mut rows = rows();
    let mut start = [500000.0, 5700000.0];
    let mut bearing: f64 = 90.0;
    for el in rows.iter_mut() {
        let length = el["length"].as_f64().unwrap();
        let stated = el.clone();
        // Walk by hand: fine steps of the curvature the element states.
        let k = |s: f64| -> f64 {
            let t = s / length;
            let el = &stated;
            match (el["elementType"].as_str().unwrap(), el["transitionType"].as_str()) {
                ("arc", _) => 1.0 / el["radius"].as_f64().unwrap(),
                ("transition", kind) => {
                    let k1 = el["r1"].as_f64().map_or(0.0, |r| 1.0 / r);
                    let k2 = el["r2"].as_f64().map_or(0.0, |r| 1.0 / r);
                    let f = if kind == Some("bloss") { 3.0 * t * t - 2.0 * t.powi(3) } else { t };
                    k1 + (k2 - k1) * f
                }
                _ => 0.0,
            }
        };
        el["bearing"] = json!(bearing);
        el["startNode"] = json!(start);
        let n = 20000;
        let h = length / n as f64;
        let mut th = bearing.to_radians();
        for j in 0..n {
            let mid = th + k((j as f64 + 0.5) * h) * h / 2.0;
            start[0] += h * mid.sin();
            start[1] += h * mid.cos();
            th += k((j as f64 + 0.5) * h) * h;
        }
        el["endNode"] = json!(start);
        bearing = el.get("endBearing").and_then(Value::as_f64).unwrap_or(th.to_degrees());
    }
    rows
}

#[test]
fn rows_go_by_column_and_come_back() {
    let columns = to_columns(&rough_closed()).unwrap().unwrap();
    assert_eq!(columns["elementType"], json!(["straight", "clothoid", "arc", "bloss", "straight", "straight"]));
    assert_eq!(columns["stationing"], json!([0, 100, 160, 260, 320, 370, 420]));
    assert_eq!(columns["beginRadius"], json!([null, null, 500, 500, null, null]));
    assert_eq!(columns["endRadius"], json!([null, 500, 500, null, null, null]));
    assert_eq!(columns["beginCant"], json!([0, 0, 60, 60, 0, 0]));
    assert_eq!(columns["endCant"], json!([0, 60, 60, 0, 0, 0]));
    assert_eq!(columns["designSpeed"], json!([80, null, null, null, null, null]));
    assert_eq!(
        columns["elementDetails"],
        json!([
            { "element": 2, "role": "follow" },
            { "element": 4, "switchId": "a1b2c3d4-0000-4000-8000-000000000001", "switchRoute": "branch", "switchBranch": true },
        ])
    );
    // The kink after the straight: the step from its bearing to the next.
    let gon = columns["bearingGon"].as_array().unwrap();
    assert_eq!(gon[5].as_f64().unwrap(), (110.0_f64 / 0.9 * 1e9).round() / 1e9);
    assert!(breaches(&columns).is_empty(), "{:?}", breaches(&columns));

    // Back: the rows as the record keeps them, and a second pass changes nothing.
    let back = to_rows(&columns);
    assert_eq!(back[1]["transitionType"], "clothoid");
    assert_eq!(back[1]["r1"], Value::Null);
    assert_eq!(back[2]["cant"], 60);
    assert!(back[2].get("cantStart").is_none());
    assert_eq!(back[3]["cantStart"], 60);
    assert_eq!(back[4]["endBearing"], 110.0);
    assert_eq!(back[4]["switchRoute"], "branch");
    assert!(back[5].get("cant").is_none(), "a straight without cant states none");
    assert_eq!(to_columns(&back).unwrap().unwrap(), columns);
    assert_eq!(to_rows(&to_columns(&chained()).unwrap().unwrap()), chained());
}

#[test]
fn a_gap_within_a_millimetre_is_closed_one_beyond_refused() {
    let mut rows = rough_closed();
    rows[3]["startNode"][0] = json!(rows[3]["startNode"][0].as_f64().unwrap() + 0.0005);
    let columns = to_columns(&rows).unwrap().unwrap();
    assert!(breaches(&columns).is_empty());
    rows[3]["startNode"][0] = json!(rows[3]["startNode"][0].as_f64().unwrap() + 0.005);
    let found = to_columns(&rows).unwrap_err();
    assert!(found.iter().any(|f| f.starts_with("element 2: ends 5.")), "{found:?}");
}

#[test]
fn a_length_that_does_not_reach_the_next_start_is_refused() {
    let mut rows = rough_closed();
    rows[2]["length"] = json!(100.01);
    let found = to_columns(&rows).unwrap_err();
    assert!(found[0].starts_with("element 2: ends"), "{found:?}");
}

#[test]
fn columns_that_do_not_hold_together_are_found() {
    let columns = to_columns(&rough_closed()).unwrap().unwrap();
    let mut gap = columns.clone();
    gap["coordinates"][3][1] = json!(gap["coordinates"][3][1].as_f64().unwrap() + 0.01);
    assert!(breaches(&gap).iter().any(|b| b.starts_with("element 2: ends 10.0 mm")), "{:?}", breaches(&gap));
    let mut kink = columns.clone();
    kink["bearingGon"][3] = json!(kink["bearingGon"][3].as_f64().unwrap() + 0.001);
    let found = breaches(&kink);
    assert!(found.contains(&"element 2: a kink after an arc".to_string()), "{found:?}");
    let mut short = columns.clone();
    short["endCant"].as_array_mut().unwrap().pop();
    assert_eq!(breaches(&short), vec!["endCant: holds 5 values, not 6".to_string()]);
    let mut arc = columns;
    arc["endRadius"][2] = json!(400);
    assert!(breaches(&arc).contains(&"element 2: an arc without its one radius".to_string()));
}

#[test]
fn the_plan_of_the_draft_holds_together() {
    // The example the format was drafted on: the OSRD export's values, to the millimetre and 1e-7 gon.
    let plan = json!({
        "stationing": [0, 23.138, 27.138, 36.614, 72.183, 78.184],
        "coordinates": [
            [4485284.234, 5726338.507], [4485301.067, 5726322.634], [4485303.913, 5726319.823],
            [4485310.655, 5726313.164], [4485336.744, 5726288.997], [4485341.298, 5726285.089]
        ],
        "bearingGon": [146.6587342, 149.6047975, 149.6047975, 149.6047975, 145.5289248, 144.764737],
        "elementType": ["arc", "straight", "straight", "arc", "arc"],
        "beginRadius": [500, null, null, -555.555556, -500],
        "endRadius": [500, null, null, -555.555556, -500],
        "beginCant": [0, 0, 0, 0, 0],
        "endCant": [0, 0, 0, 0, 0],
        "designSpeed": [60, 60, 60, null, null]
    });
    assert_eq!(breaches(&plan), Vec::<String>::new());
}

mod km_lines {
    use super::*;
    use rail_across_doc::core_format::document_to_record;
    use rail_across_doc::door::{check_document, to_document};
    use rail_across_geom::km_line::{km_at, wgs84_runs};

    const POINTS: &str = r#"[[500600.0, 5700000.0, 13000.0], [500700.0, 5700000.0, 12900.0]]"#;

    /// A record with a km line in its plane: one run by its elements, another by its points.
    fn record(rows: Vec<Value>) -> Value {
        let points: Value = serde_json::from_str(POINTS).unwrap();
        json!({
            "id": "p", "tracks": [], "switches": [], "platforms": [],
            "kmLines": [{
                "id": "km-6050", "lineNumber": 6050, "epsg": 25832, "source": "vermesn",
                "runs": [
                    { "kmStart": 12400.0, "counting": "up", "elements": rows },
                    { "kmStart": 13000.0, "counting": "down", "points": points },
                ],
            }],
        })
    }

    fn rows() -> Vec<Value> {
        rough_closed()
            .into_iter()
            .map(|mut r| {
                for k in ["cant", "cantStart", "cantEnd", "designSpeed", "role", "switchId", "switchRoute", "switchBranch"] {
                    r.as_object_mut().unwrap().shift_remove(k);
                }
                r
            })
            .collect()
    }

    #[test]
    fn a_km_line_goes_by_column_and_comes_back_in_its_plane() {
        let doc = to_document(&record(rows()), false, true).unwrap_or_else(|e| panic!("refused ({e})"));
        let line = &doc["domains"]["core"]["collections"]["kmLines"][0];
        assert_eq!(line["epsg"], 25832);
        let plan = &line["runs"][0]["horizontalAlignment"];
        assert_eq!(plan["stationing"], json!([0, 100, 160, 260, 320, 370, 420]));
        assert!(plan.get("beginCant").is_none() && plan.get("designSpeed").is_none(), "{plan}");
        assert!(line["runs"][1]["points"].is_string(), "the run of points a blob");
        let back = document_to_record(&check_document(&doc).unwrap()).unwrap();
        let km = &back["kmLines"][0];
        assert_eq!(km["epsg"], 25832);
        assert!(
            km.get("crs").is_none() && km.get("design").is_none(),
            "the record holds the line as the document does"
        );
        let elements = km["runs"][0]["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 6);
        assert!(elements.iter().all(|e| e.get("cant").is_none()), "a km line has no cant");
        assert_eq!(km["runs"][1]["points"].as_array().unwrap().len(), 2);
        assert_eq!(to_document(&back, false, true).unwrap(), doc);
    }

    #[test]
    fn a_record_of_before_is_read_by_its_design_or_its_points() {
        let doc = to_document(&record(rows()), false, true).unwrap();
        // The record of the last build: the points of WGS 84 to draw, the line itself beside them.
        let mut before = record(rows());
        let line = before["kmLines"][0].take();
        before["kmLines"][0] = json!({
            "id": line["id"], "lineNumber": line["lineNumber"], "source": line["source"], "crs": "EPSG:4326",
            "runs": [[[9.0, 51.4, 12400.0], [9.001, 51.4, 12470.0]]],
            "design": { "epsg": line["epsg"], "runs": line["runs"] },
        });
        assert_eq!(to_document(&before, false, true).unwrap(), doc);
        // A record of before that, its points of WGS 84 alone: a run of points in the UTM plane of the first.
        before["kmLines"][0].as_object_mut().unwrap().shift_remove("design");
        let points = to_document(&before, false, true).unwrap();
        let line = &points["domains"]["core"]["collections"]["kmLines"][0];
        assert_eq!(line["epsg"], 25832);
        assert!(line["runs"][0]["points"].is_string());
    }

    #[test]
    fn a_kilometre_on_elements_is_the_station_along_them() {
        let line = record(rows())["kmLines"][0].clone();
        // 30 m along the arc (it begins at 160 m): the kilometre 12400 + 190, the point on the axis.
        let arc = &line["runs"][0]["elements"][2];
        let start = (arc["startNode"][0].as_f64().unwrap(), arc["startNode"][1].as_f64().unwrap());
        let at = arc_end(start, arc["bearing"].as_f64().unwrap(), 30.0, arc["radius"].as_f64().unwrap());
        let (km, offset) = km_at(&line, [at.0, at.1], 25832).unwrap();
        assert!((km - 12590.0).abs() < 1e-6, "{km}");
        assert!(offset < 1e-6);
        // On the run of points, between them; counting down.
        let (km, _) = km_at(&line, [500650.0, 5700001.0], 25832).unwrap();
        assert!((km - 12950.0).abs() < 1e-9, "{km}");
        // Drawn in WGS 84 from its elements.
        assert!(wgs84_runs(&line, 5.0)[0].len() > 80);
    }

    /// Where an arc of signed radius `r` leads from `start` on `bearing` after `length`.
    fn arc_end(start: (f64, f64), bearing: f64, length: f64, r: f64) -> (f64, f64) {
        let s = rail_across_geom::element::Utm {
            e: start.0,
            n: start.1,
            zone: 0,
        };
        let e = rail_across_geom::element::end_point_curved(s, bearing, length, r);
        (e.e, e.n)
    }

    #[test]
    fn a_km_lines_elements_that_do_not_hold_together_are_refused() {
        let mut design = rows();
        design[3]["startNode"][0] = json!(design[3]["startNode"][0].as_f64().unwrap() + 0.005);
        let e = to_document(&record(design), false, true).unwrap_err();
        assert!(e.details.iter().any(|d| d.contains("6050, run 0, element 2")), "{:?}", e.details);
    }
}
