// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Imported tracks fitted to hold together, their coordinates fixed (`fit`): a curve group between
//! its straights with its radii and transitions, straights in a row made one within 5 mm, the curve at a
//! track's end from its first point and bearing, a switch's route left as it is.

use rail_across_optimizer::fit::{fit_elements, fit_payload};
use rail_across_optimizer::geometry::{Profile, arc_forward, dir_of, transition_end};
use serde_json::{Value, json};

/// An element of the app from its start, bearing and values, its end where they lead.
fn el(kind: &str, start: (f64, f64), bearing: f64, length: f64, r: (Option<f64>, Option<f64>)) -> (Value, (f64, f64), f64) {
    let (end, end_bearing) = match kind {
        "arc" => arc_forward(start.0, start.1, bearing, r.0.unwrap(), length / r.0.unwrap().abs()),
        "transition" => {
            let (e, n, b) = transition_end(start.0, start.1, bearing, length, r.0, r.1, Profile::Clothoid);
            ((e, n), b)
        }
        _ => {
            let d = dir_of(bearing);
            ((start.0 + length * d.0, start.1 + length * d.1), bearing)
        }
    };
    let mut v = json!({ "elementType": kind, "startNode": [start.0, start.1], "endNode": [end.0, end.1], "bearing": bearing, "length": length });
    match kind {
        "arc" => v["radius"] = json!(r.0),
        "transition" => {
            v["r1"] = json!(r.0);
            v["r2"] = json!(r.1);
            v["transitionType"] = json!("clothoid");
        }
        _ => {}
    }
    (v, end, end_bearing)
}

/// A piece of a chain: its kind, length and radii at its two ends.
type Piece<'a> = (&'a str, f64, (Option<f64>, Option<f64>));

/// A chain from (0, 0) on bearing 90°: each piece from where the one before ends.
fn chain(pieces: &[Piece]) -> Vec<Value> {
    let (mut at, mut bearing) = ((0.0, 0.0), 90.0);
    pieces
        .iter()
        .map(|&(kind, length, r)| {
            let (v, end, b) = el(kind, at, bearing, length, r);
            (at, bearing) = (end, b);
            v
        })
        .collect()
}

fn pt(v: &Value, key: &str) -> (f64, f64) {
    (v[key][0].as_f64().unwrap(), v[key][1].as_f64().unwrap())
}

/// What the door asks of a chain: each element's values lead to the next start, a curve on the next bearing.
fn holds(els: &[Value]) {
    for (i, e) in els.iter().enumerate() {
        let r = match e["elementType"].as_str().unwrap() {
            "arc" => (e["radius"].as_f64(), e["radius"].as_f64()),
            "transition" => (e["r1"].as_f64(), e["r2"].as_f64()),
            _ => (None, None),
        };
        let (_, end, b) = el(
            e["elementType"].as_str().unwrap(),
            pt(e, "startNode"),
            e["bearing"].as_f64().unwrap(),
            e["length"].as_f64().unwrap(),
            r,
        );
        let stated = pt(e, "endNode");
        assert!((end.0 - stated.0).hypot(end.1 - stated.1) < 1e-6, "element {i} ends off its end node: {e}");
        if let Some(next) = els.get(i + 1) {
            let s = pt(next, "startNode");
            assert!((end.0 - s.0).hypot(end.1 - s.1) < 1e-6, "element {i} ends off where the next begins");
            if e["elementType"] != "straight" {
                let nb = next["bearing"].as_f64().unwrap();
                assert!(((b - nb + 540.0).rem_euclid(360.0) - 180.0).abs() < 1e-7, "a kink after element {i}");
            }
        }
    }
}

const CURVE: [Piece; 5] = [
    ("straight", 100.0, (None, None)),
    ("transition", 60.0, (None, Some(500.0))),
    ("arc", 120.0, (Some(500.0), Some(500.0))),
    ("transition", 60.0, (Some(500.0), None)),
    ("straight", 150.0, (None, None)),
];

#[test]
fn a_curve_group_fits_between_its_straights_with_its_radius_and_transitions() {
    let mut els = chain(&CURVE);
    // The file states the arc 2 cm long and the last straight from where the arc should have ended.
    els[2]["length"] = json!(120.02);
    let (start, end) = (pt(&els[0], "startNode"), pt(&els[4], "endNode"));
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    holds(&fitted);
    assert_eq!(fitted.len(), 5);
    assert_eq!(fitted[2]["radius"], 500.0);
    assert_eq!(fitted[1]["length"], 60.0);
    assert_eq!(fitted[3]["length"], 60.0);
    assert!(
        (fitted[2]["length"].as_f64().unwrap() - 120.0).abs() < 1e-6,
        "the arc as long as the corner asks"
    );
    assert_eq!(pt(&fitted[0], "startNode"), start);
    assert_eq!(pt(&fitted[4], "endNode"), end);
}

#[test]
fn straights_in_a_row_become_one_within_five_millimetres() {
    let mut els = chain(&[("straight", 100.0, (None, None)), ("straight", 100.0, (None, None))]);
    els[0]["endNode"][1] = json!(0.003);
    els[1]["startNode"][1] = json!(0.003);
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert_eq!(fitted.len(), 1, "{notes:?}");
    let end = pt(&fitted[0], "endNode");
    assert!((end.0 - 200.0).abs() < 1e-9 && end.1.abs() < 1e-9, "{end:?}");

    els[0]["endNode"][1] = json!(0.008);
    els[1]["startNode"][1] = json!(0.008);
    let (fitted, _) = fit_elements(&els).unwrap();
    assert_eq!(fitted.len(), 2, "two straights with a kink between them");
    assert!(fitted[0].get("endBearing").is_some());
    holds(&fitted);
}

#[test]
fn a_curve_at_a_tracks_start_keeps_its_first_point_and_bearing() {
    let mut els = chain(&[("arc", 80.0, (Some(500.0), Some(500.0))), ("straight", 200.0, (None, None))]);
    // The straight's far end 5 cm aside: the arc's sweep runs it into the straight through that point.
    let far = pt(&els[1], "endNode");
    els[1]["endNode"] = json!([far.0 + 0.04, far.1 - 0.03]);
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    holds(&fitted);
    assert_eq!(pt(&fitted[0], "startNode"), (0.0, 0.0));
    assert_eq!(fitted[0]["bearing"], 90.0);
    assert_eq!(fitted[0]["radius"], 500.0);
    assert_eq!(pt(&fitted[1], "endNode"), (far.0 + 0.04, far.1 - 0.03));
}

#[test]
fn a_switchs_route_stays_as_it_lies() {
    let mut els = chain(&[
        ("arc", 21.0249, (Some(190.0), Some(190.0))),
        ("straight", 6.3726, (None, None)),
        ("straight", 50.0, (None, None)),
        ("arc", 60.0, (Some(-400.0), Some(-400.0))),
        ("straight", 80.0, (None, None)),
    ]);
    for e in &mut els[..2] {
        e["switchId"] = json!("w1");
        e["switchBranch"] = json!(true);
    }
    els[3]["length"] = json!(60.01);
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(fitted[0], els[0]);
    assert_eq!(fitted[1]["startNode"], els[1]["startNode"]);
    assert_eq!(fitted[1]["length"], els[1]["length"]);
    holds(&fitted[1..]);
}

#[test]
fn a_compound_curve_that_would_move_more_than_a_millimetre_stays() {
    let mut els = chain(&[
        ("straight", 100.0, (None, None)),
        ("arc", 60.0, (Some(500.0), Some(500.0))),
        ("arc", 60.0, (Some(800.0), Some(800.0))),
        ("straight", 100.0, (None, None)),
    ]);
    // The last straight turned about its end: the group would have to move centimetres.
    els[3]["startNode"][1] = json!(els[3]["startNode"][1].as_f64().unwrap() + 0.05);
    let before = els[1].clone();
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.iter().any(|n| n.contains("Korbbogen")), "{notes:?}");
    assert_eq!(fitted[1]["length"], before["length"]);
}

#[test]
fn the_payload_answers_per_track_with_how_far_it_moved() {
    let mut els = chain(&CURVE);
    els[2]["length"] = json!(120.02);
    let answer = fit_payload(&json!({ "tracks": [{ "id": "t1", "elements": els }] })).unwrap();
    let t = &answer["tracks"][0];
    assert_eq!(t["id"], "t1");
    assert_eq!(t["elements"].as_array().unwrap().len(), 5);
    assert!(t["moved"].as_f64().unwrap() < 0.05, "{}", t["moved"]);
}

#[test]
fn a_transition_in_pieces_is_walked_piece_by_piece() {
    let mut els = chain(&[
        ("straight", 100.0, (None, None)),
        ("transition", 30.0, (None, Some(1000.0))),
        ("transition", 30.0, (Some(1000.0), Some(500.0))),
        ("arc", 80.0, (Some(500.0), Some(500.0))),
        ("transition", 60.0, (Some(500.0), None)),
        ("straight", 100.0, (None, None)),
    ]);
    els[3]["length"] = json!(80.015);
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    holds(&fitted);
    assert_eq!(fitted[1]["length"], 30.0);
    assert_eq!(fitted[2]["length"], 30.0);
}

#[test]
fn a_sliver_a_cut_left_goes() {
    let mut els = chain(&[
        ("arc", 40.0, (Some(-1400.0), Some(-1400.0))),
        ("straight", 0.00003, (None, None)),
        ("straight", 50.0, (None, None)),
    ]);
    els[1]["bearing"] = json!(els[1]["bearing"].as_f64().unwrap() + 0.0005);
    let (fitted, notes) = fit_elements(&els).unwrap();
    assert!(notes.iter().any(|n| n.contains("unter 1 mm")), "{notes:?}");
    assert_eq!(fitted.len(), 2);
    holds(&fitted);
}
