// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The whole simulation, through its JSON: a regional train over 20 km with
//! grades, speed limits, two stops, a standard margin and an arrival to keep.

use rail_across_sim::simulate_json;
use serde_json::{Value, json};

fn request() -> Value {
    json!({
        "path": {
            "length": 20000.0,
            "gradients": [
                { "begin": 2000.0, "end": 6000.0, "value": 12.5 },
                { "begin": 9000.0, "end": 12000.0, "value": -8.0 }
            ],
            "speed_limits": [
                { "begin": 0.0, "end": 20000.0, "speed": 160.0 / 3.6, "source": "track" },
                { "begin": 7000.0, "end": 8000.0, "speed": 80.0 / 3.6, "source": "route" }
            ]
        },
        "rolling_stock": {
            "mass": 120000.0, "length": 70.0, "max_speed": 160.0 / 3.6, "inertia_coefficient": 1.08,
            "rolling_resistance": { "A": 1200.0, "B": 25.0, "C": 6.0 },
            "const_gamma": 0.6,
            "effort_curve": [
                { "speed": 0.0, "max_effort": 150000.0 }, { "speed": 15.0, "max_effort": 150000.0 },
                { "speed": 30.0, "max_effort": 80000.0 }, { "speed": 45.0, "max_effort": 50000.0 }
            ]
        },
        "schedule": [
            { "path_offset": 10000.0, "stop_duration": 30.0 },
            { "path_offset": 20000.0, "stop_duration": 0.0 }
        ],
        "margins": { "boundaries": [], "values": ["5%"] },
        "distribution": "mareco",
        "time_step": 2.0
    })
}

fn simulate(request: &Value) -> Value {
    serde_json::from_str(&simulate_json(&request.to_string())).unwrap()
}

#[test]
fn a_train_runs_its_path() {
    let result = simulate(&request());
    assert!(result.get("error").is_none(), "{result}");
    let base = &result["base"];
    let positions = base["positions"].as_array().unwrap();
    let times: Vec<f64> = base["times"].as_array().unwrap().iter().map(|t| t.as_f64().unwrap()).collect();
    let speeds: Vec<f64> = base["speeds"].as_array().unwrap().iter().map(|s| s.as_f64().unwrap()).collect();
    assert_eq!(positions.first().unwrap().as_f64(), Some(0.0));
    assert_eq!(positions.last().unwrap().as_f64(), Some(20000.0));
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "time runs forward");
    assert_eq!(*speeds.last().unwrap(), 0.0);
    // the stop: arrival, 30 s, departure
    let stop = &base["scheduled_points"][0];
    let (arrival, departure) = (stop["arrival"].as_f64().unwrap(), stop["departure"].as_f64().unwrap());
    assert!((departure - arrival - 30.0).abs() < 1e-6);
    // never faster than 80 km/h where the route says so, once the train is in
    for (p, s) in positions.iter().zip(&speeds) {
        let p = p.as_f64().unwrap();
        if (7000.0..=8000.0).contains(&p) {
            assert!(*s <= 80.0 / 3.6 + 1e-6, "{s} m/s at {p} m");
        }
    }
    // the MRSP shown: 160, 80, 160 — the train's own max speed is the same as the track's here
    let mrsp = result["mrsp"].as_array().unwrap();
    assert!(
        mrsp.iter().any(|r| r["source"] == "route" && r["begin"] == 7000.0 && r["end"] == 8000.0),
        "{mrsp:?}"
    );
}

#[test]
fn the_margin_adds_five_percent() {
    let result = simulate(&request());
    let base = result["base"]["total_time"].as_f64().unwrap() - 30.0;
    let provisional = result["provisional"]["total_time"].as_f64().unwrap() - 30.0;
    assert!((provisional - base * 1.05).abs() <= 2.0, "base {base} s, provisional {provisional} s");
    // MARECO saves energy
    assert!(result["provisional"]["energy_consumption"].as_f64() < result["base"]["energy_consumption"].as_f64());
}

#[test]
fn an_arrival_is_kept() {
    let mut req = request();
    let base = simulate(&req)["provisional"]["scheduled_points"][0]["arrival"].as_f64().unwrap();
    let wanted = (base + 60.0).round();
    req["schedule"][0]["arrival"] = json!(wanted);
    let result = simulate(&req);
    let arrival = result["final"]["scheduled_points"][0]["arrival"].as_f64().unwrap();
    assert!((arrival - wanted).abs() <= 2.0, "arrives at {arrival} s, scheduled {wanted} s");
}

#[test]
fn linear_margins_by_distance() {
    let mut req = request();
    req["distribution"] = json!("linear");
    req["margins"] = json!({ "boundaries": [10000.0], "values": ["3min/100km", "none"] });
    let result = simulate(&req);
    assert!(result.get("error").is_none(), "{result}");
    let base = result["base"]["total_time"].as_f64().unwrap();
    let provisional = result["provisional"]["total_time"].as_f64().unwrap();
    // 3 min per 100 km over the first 10 km: 18 s
    assert!((provisional - base - 18.0).abs() <= 2.0, "base {base} s, provisional {provisional} s");
}

#[test]
fn errors_come_back_as_json() {
    let mut req = request();
    req["rolling_stock"]["effort_curve"] = json!([{ "speed": 0.0, "max_effort": 1000.0 }]);
    let result = simulate(&req);
    assert_eq!(result["error"]["code"], "impossible_simulation", "{result}");
    let result: Value = serde_json::from_str(&simulate_json("{")).unwrap();
    assert_eq!(result["error"]["code"], "invalid_input");
}
