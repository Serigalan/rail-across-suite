//! Reference case A (docs/reference-cases/a-two-track-line.md), K1: the minimum
//! headways of two trains on a two-track line and an hour's timetable compressed
//! after UIC Code 406, as worked out by hand.

use rail_across_capacity::{Train, compress, conflict_areas, headways, least_headway, resource_occupancy};
use serde_json::{Value, json};

/// A train's blocks on track L, each from its signal to the next with the overlap: [from, reach, begin, end].
fn train(id: &str, kind: &str, at: f64, blocks: &[(f64, f64, f64, f64)]) -> Value {
    let blocks: Vec<Value> = blocks
        .iter()
        .map(|(lo, hi, b, e)| json!({ "begin": b + at, "end": e + at, "resources": [{ "track": "L", "lo": lo, "hi": hi }] }))
        .collect();
    json!({ "id": id, "kind": kind, "at": at, "blocks": blocks })
}

const A: [(f64, f64, f64, f64); 4] = [
    (0.0, 1500.0, -12.0, 88.5),
    (1500.0, 3700.0, 5.0, 187.5),
    (3500.0, 5700.0, 63.0, 288.5),
    (5500.0, 6000.0, 163.0, 306.0),
];
const B: [(f64, f64, f64, f64); 4] = [
    (0.0, 1500.0, -12.0, 171.0),
    (1500.0, 3700.0, 20.0, 370.0),
    (3500.0, 5700.0, 138.0, 571.0),
    (5500.0, 6000.0, 338.0, 606.0),
];

fn t(v: &Value) -> Train {
    Train::from_json(v).unwrap()
}

#[test]
fn the_minimum_headways_as_worked_out_by_hand() {
    let (a, b) = (t(&train("a", "A", 0.0, &A)), t(&train("b", "B", 0.0, &B)));
    assert_eq!(least_headway(&a, &a), Some(283.5));
    assert_eq!(least_headway(&a, &b), Some(268.5));
    assert_eq!(least_headway(&b, &a), Some(566.0));
    assert_eq!(least_headway(&b, &b), Some(551.0));
    // later at the start, the same headway
    assert_eq!(least_headway(&a, &t(&train("a2", "A", 1000.0, &A))), Some(283.5));
}

#[test]
fn the_hour_compressed_as_worked_out_by_hand() {
    let trains = json!([
        train("A1", "A", 0.0, &A),
        train("B1", "B", 300.0, &B),
        train("A2", "A", 1200.0, &A),
        train("A3", "A", 1800.0, &A)
    ]);
    let r = compress(&json!({ "trains": trains, "from": 0, "to": 3600 }));
    let at: Vec<f64> = r["trains"].as_array().unwrap().iter().map(|x| x["compressedAt"].as_f64().unwrap()).collect();
    assert_eq!(at, vec![0.0, 268.5, 834.5, 1118.0]);
    let by: Vec<Value> = r["trains"].as_array().unwrap().iter().map(|x| x["boundBy"].clone()).collect();
    assert_eq!(by, vec![Value::Null, json!("A1"), json!("B1"), json!("A2")]);
    assert_eq!(r["occupancy"], 1436.0);
    assert!((r["rate"].as_f64().unwrap() - 1436.0 / 3600.0).abs() < 1e-12);
    assert_eq!(r["critical"], json!(["A1", "B1", "A2", "A3"]));
    // B1 bound to A1 where its block 1 (1500–3700) meets A1's block 2 (3500–5700 with the overlap), at 288.5
    assert_eq!(r["trains"][0]["binding"], Value::Null);
    assert_eq!(
        r["trains"][1]["binding"],
        json!({ "resource": "track:L:3500-5700", "beforeBlock": 2, "block": 1, "released": 288.5, "taken": 288.5 })
    );
    // the additional times on top
    let with = compress(&json!({ "trains": trains, "from": 0, "to": 3600, "additional": { "perTrain": 60, "fixed": 120 } }));
    assert_eq!(with["additional"], 360.0);
    assert!((with["rateWithAdditional"].as_f64().unwrap() - 1796.0 / 3600.0).abs() < 1e-12);
    // a train outside the period is not counted
    let late = compress(&json!({ "trains": trains, "from": 0, "to": 1500 }));
    assert_eq!(late["trains"].as_array().unwrap().len(), 3);
}

#[test]
fn every_headway_in_both_orders_and_what_each_track_holds() {
    let trains = json!([train("A1", "A", 0.0, &A), train("B1", "B", 300.0, &B)]);
    let h = headways(&json!({ "trains": trains }));
    assert_eq!(
        h,
        json!([
            { "before": "A1", "after": "B1", "kinds": ["A", "B"], "least": 268.5 },
            { "before": "B1", "after": "A1", "kinds": ["B", "A"], "least": 566.0 },
        ])
    );
    let held = resource_occupancy(&json!({ "trains": trains, "from": 0, "to": 3600 }));
    // block 2's track, held by A1 from 63 to 288.5 and by B1 from 438 to 871
    let b2 = held.as_array().unwrap().iter().find(|r| r["resource"] == "track:L:3500-5700").unwrap();
    assert_eq!(b2["held"], 225.5 + 433.0);
    assert_eq!(b2["trains"], json!(["A1", "B1"]));
}

#[test]
fn trains_that_hold_nothing_the_same_follow_at_once() {
    let other = json!({ "id": "x", "at": 10.0, "blocks": [{ "begin": 0, "end": 50, "resources": [{ "section": "elsewhere" }] }] });
    let r = compress(&json!({ "trains": [train("A1", "A", 0.0, &A), other], "from": 0, "to": 3600 }));
    assert_eq!(r["trains"][1]["compressedAt"], 0.0);
    assert_eq!(r["trains"][1]["headway"], Value::Null);
    assert_eq!(r["critical"], json!(["A1"]));
}

#[test]
fn the_areas_two_trains_both_hold() {
    let other = json!({ "id": "x", "at": 10.0, "blocks": [{ "begin": 0, "end": 50, "resources": [{ "section": "elsewhere" }] }] });
    let trains = json!([train("A1", "A", 0.0, &A), train("B1", "B", 300.0, &B), other]);
    let areas = conflict_areas(&json!({ "trains": trains, "from": 0, "to": 3600 }));
    // the blocks' stretches that overlap, one area; the first block's apart (they only touch); the section only x holds none
    assert_eq!(
        areas,
        json!([
            // A1 from 5 to 306, B1 from 320 to 906
            { "resource": "track:L:1500-6000", "trains": ["A1", "B1"], "held": 887.0, "rate": 887.0 / 3600.0 },
            { "resource": "track:L:0-1500", "trains": ["A1", "B1"], "held": 271.5, "rate": 271.5 / 3600.0 },
        ])
    );
}
