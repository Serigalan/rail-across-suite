// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A Bloss transition carries the curved ramp (geschwungene Rampe) of its own
//! form: its cant follows 3t² − 2t³ between its ends, as its curvature does;
//! a clothoid's cant runs straight.

use rail_across_track::bands::cant_band;
use rail_across_track::cross_section::section_at_station;
use serde_json::{Value, json};

fn chain(form: &str) -> Vec<Value> {
    vec![
        json!({ "elementType": "straight", "length": 100.0, "cant": 0 }),
        json!({ "elementType": "transition", "transitionType": form, "length": 100.0, "r1": null, "r2": 1000.0 }),
        json!({ "elementType": "arc", "length": 100.0, "radius": 1000.0, "cant": 80 }),
    ]
}

fn value_at(band: &[Value], s: f64) -> f64 {
    for w in band.windows(2) {
        let (a, b) = (w[0]["s"].as_f64().unwrap(), w[1]["s"].as_f64().unwrap());
        if s >= a && s <= b && b > a {
            let (va, vb) = (w[0]["v"].as_f64().unwrap(), w[1]["v"].as_f64().unwrap());
            return va + (vb - va) * (s - a) / (b - a);
        }
    }
    panic!("no value at {s}");
}

#[test]
fn a_bloss_transition_has_the_curved_ramp_in_the_cant_band() {
    let band = cant_band(&chain("bloss"));
    // a quarter along: 3/16 − 2/64 = 0.15625 of the 80 mm
    assert!((value_at(&band, 125.0) - 12.5).abs() < 1e-9);
    assert!((value_at(&band, 150.0) - 40.0).abs() < 1e-9);
    assert!((value_at(&band, 175.0) - 67.5).abs() < 1e-9);
    // it starts and ends flat: the first and the last step are the shallowest
    let ramp: Vec<f64> = band
        .iter()
        .filter(|p| (100.0..=200.0).contains(&p["s"].as_f64().unwrap()))
        .map(|p| p["v"].as_f64().unwrap())
        .collect();
    let steps: Vec<f64> = ramp.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.0).collect();
    let mid = steps[steps.len() / 2];
    assert!(steps[0] < mid / 5.0 && steps[steps.len() - 1] < mid / 5.0);
}

#[test]
fn a_clothoid_keeps_the_straight_ramp() {
    let band = cant_band(&chain("clothoid"));
    assert!((value_at(&band, 125.0) - 20.0).abs() < 1e-9);
    assert_eq!(band.iter().filter(|p| (100.0..=200.0).contains(&p["s"].as_f64().unwrap())).count(), 4);
}

#[test]
fn the_cant_and_the_radius_at_a_station_follow_the_bloss_form() {
    let track = json!({ "elements": chain("bloss") });
    let at = section_at_station(&track, 125.0);
    assert!((at["cant"].as_f64().unwrap() - 12.5).abs() < 1e-9);
    // the curvature as far along its form: 1000 m / 0.15625
    assert!((at["radius"].as_f64().unwrap() - 6400.0).abs() < 1e-6);
    let clothoid = section_at_station(&json!({ "elements": chain("clothoid") }), 125.0);
    assert!((clothoid["cant"].as_f64().unwrap() - 20.0).abs() < 1e-9);
    assert!((clothoid["radius"].as_f64().unwrap() - 4000.0).abs() < 1e-6);
}
