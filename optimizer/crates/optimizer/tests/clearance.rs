// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track spacing of a splice — the cases of the service's
//! verify_clearance.py.
//!
//! An existing inner track — a straight running east, a left-hand arc R 300
//! about (100, 300), a straight running north — and the two straights of a new
//! outer track 4.5 m outside it. The corner arc tangent to both is concentric
//! with the inner one at R 304.5 and keeps its 4.5 m; a larger one moves its
//! middle towards the inner arc. Without cant, 4.0 m are kept up to R ≈ 305.7.

mod common;

use rail_across_optimizer::clearance::{CantModel, Spacing, auto_cant};
use rail_across_optimizer::element::Element;
use rail_across_optimizer::geometry::{Profile, Pt, arc_forward, transition_end};
use rail_across_optimizer::grenzen::transition_lengths;
use rail_across_optimizer::rules::Rules;
use rail_across_optimizer::splice::splice_payload;
use serde_json::{Value, json};
use std::f64::consts::PI;
use std::time::Instant;

const E0: f64 = 500000.0;
const N0: f64 = 5600000.0;

fn profile() -> Value {
    let data = common::constraints("db-ril-800-0130.json");
    data["lichtraum"]["profile"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "hauptgleis")
        .unwrap()["umriss"]
        .clone()
}

fn half(profile: &Value) -> Vec<Pt> {
    profile
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
        .collect()
}

fn model() -> Value {
    json!({"coeff": 6.5, "defCoeff": 11.8, "defMin": 60, "max": 160, "step": 5})
}

/// Axis points of the inner track every metre, with its cant (negative: a left-hand curve).
fn inner(cant: f64) -> Value {
    let mut pts: Vec<Value> = (0..100).map(|x| json!([E0 + f64::from(x), N0, 0.0])).collect();
    let (cx, cy) = (E0 + 100.0, N0 + 300.0);
    let n = (300.0 * PI / 2.0) as usize;
    for i in 0..=n {
        let a = -PI / 2.0 + (PI / 2.0) * i as f64 / n as f64;
        pts.push(json!([cx + 300.0 * a.cos(), cy + 300.0 * a.sin(), -cant]));
    }
    pts.extend((1..400).map(|y| json!([E0 + 400.0, N0 + 300.0 + f64::from(y), 0.0])));
    Value::Array(pts)
}

fn dep(cant: f64) -> Value {
    json!({"start": [E0, N0 - 4.5], "end": [E0 + 100.0, N0 - 4.5], "bearing": 90.0, "radius": null, "cant": cant})
}

fn arr(cant: f64) -> Value {
    json!({"start": [E0 + 404.5, N0 + 700.0], "end": [E0 + 404.5, N0 + 300.0], "bearing": 180.0, "radius": null, "cant": cant})
}

/// The best solution of an answer, or the error.
fn best(rules: &Rules, request: Value) -> Value {
    let answer = splice_payload(rules, &request).unwrap();
    if answer.get("solutions").is_some() {
        answer["solutions"][0].clone()
    } else {
        answer
    }
}

fn merge(mut base: Value, extra: Value) -> Value {
    for (k, v) in extra.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

#[test]
fn the_spacing_the_cant_adds() {
    let sp = Spacing::new(&half(&profile()));
    assert_eq!((sp.widening(0.0, 0.0, 1), sp.widening(0.0, 0.0, -1)), (0.0, 0.0));
    // A positive cant raises the left rail: the track leans right.
    let w = sp.widening(150.0, 0.0, 1);
    assert!(0.25 < w && w < 0.55, "{w}");
    assert_eq!(sp.widening(150.0, 0.0, -1), 0.0);
    let alike = sp.widening(150.0, 150.0, 1);
    assert!((0.0..0.06).contains(&alike), "{alike}");
    assert!((sp.widening(0.0, -150.0, 1) - w).abs() < 0.01);
    let model = CantModel::from_json(&model()).unwrap();
    assert_eq!(auto_cant(80.0, 300.0, &model), 140.0);
    assert_eq!(auto_cant(40.0, 2000.0, &model), 0.0);
}

#[test]
fn two_straights_kept_off_the_inner_track() {
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let ask = |extra: Value, clearance: Value| {
        let cl = merge(json!({"ref": inner(0.0), "dMin": 4.0, "profile": profile(), "cantModel": model()}), clearance);
        best(
            &rules,
            merge(json!({"dep": dep(0.0), "arr": arr(0.0), "radius": 304.5, "clearance": cl}), extra),
        )
    };

    let c = ask(json!({}), json!({}))["info"]["clearance"].clone();
    assert!(c["kept"] == true && (c["distance"].as_f64().unwrap() - 4.5).abs() < 0.01, "{c}");
    assert_eq!(ask(json!({"radius": 320}), json!({}))["info"]["clearance"]["kept"], false);

    let res = ask(json!({}), json!({"maximize": true, "speed": 0}));
    let c = &res["info"]["clearance"];
    assert_eq!(c["radius"], 305.0, "{c}");
    assert!(c["kept"] == true && c["margin"].as_f64().unwrap() >= 0.0);
    assert!((res["info"]["signedR"].as_f64().unwrap().abs() - 305.0).abs() < 1e-9);

    // Without transitions the new arc keeps the straights' cant (Entscheidung 315).
    let c = ask(json!({}), json!({"maximize": true, "speed": 80}))["info"]["clearance"].clone();
    assert!(c["radius"] == 305.0 && c["cant"] == 0.0, "{c}");

    // The straights canted 135 mm: the arc keeps that and leans towards the inner track.
    let c = ask(json!({"dep": dep(135.0), "arr": arr(135.0)}), json!({"maximize": true, "speed": 80}))["info"]["clearance"].clone();
    assert!(c["radius"].as_f64().unwrap() < 305.0 && c["cant"] == 135.0, "{c}");
    assert!(c["required"].as_f64().unwrap() > 4.2);

    // Both canted alike: they lean together, and the plain 4.0 m holds again.
    let c = ask(
        json!({"dep": dep(140.0), "arr": arr(140.0)}),
        json!({"maximize": true, "speed": 80, "ref": inner(140.0)}),
    )["info"]["clearance"]
        .clone();
    assert_eq!(c["radius"], 305.0, "{c}");

    // The cant ramps over transitions.
    let res = ask(json!({"lDep": 40, "lArr": 40}), json!({"maximize": true, "speed": 80}));
    let c = &res["info"]["clearance"];
    assert!(res.get("error").is_none() && c["kept"] == true && c["margin"].as_f64().unwrap() >= 0.0, "{res}");
    let m = CantModel::from_json(&model()).unwrap();
    assert!(c["cant"].as_f64().unwrap() == auto_cant(80.0, c["radius"].as_f64().unwrap(), &m) && c["cant"].as_f64().unwrap() > 0.0);

    // The transitions at the Regellänge (AP S.6): those of the radius found.
    let t0 = Instant::now();
    let res = ask(
        json!({"lDep": 40, "lArr": 40, "speed": 80, "modeDep": "regular", "modeArr": "regular"}),
        json!({"maximize": true, "speed": 80}),
    );
    let took = t0.elapsed().as_secs_f64();
    let c = &res["info"]["clearance"];
    let (r, u) = (c["radius"].as_f64().unwrap(), c["cant"].as_f64().unwrap());
    assert!(c["kept"] == true && u == auto_cant(80.0, r, &m));
    let straight = Element {
        element_type: Some(0),
        cant: Some(0.0),
        speed: Some(80.0),
        ..Element::default()
    };
    let curve = Element {
        element_type: Some(1),
        radius: Some(r),
        cant: Some(u),
        speed: Some(80.0),
        ..Element::default()
    };
    let want = transition_lengths(Some(&straight), Some(&curve), None, "clothoid", 80.0, &rules.reg, &rules.discretion)
        .unwrap()
        .regular
        .unwrap();
    let lengths: Vec<f64> = res["lengths"].as_array().unwrap().iter().map(|l| l["length"].as_f64().unwrap()).collect();
    assert_eq!(lengths, [want, want]);
    let arc = res["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|el| el["role"] == "new" && el["elementType"] == 1)
        .unwrap();
    assert_eq!(arc["cant"].as_f64().unwrap().abs(), u);
    assert!(took < 5.0, "{took}");

    // A neighbour on the corner itself: every arc leaves it at its tangent point.
    let corner = json!([[E0, N0 - 4.5, 0], [E0 + 404.5, N0 - 4.5, 0], [E0 + 404.5, N0 + 700.0, 0]]);
    assert_eq!(
        ask(json!({}), json!({"maximize": true, "speed": 0, "ref": corner}))["error"],
        "splice_error_clearance"
    );
}

#[test]
fn an_arc_and_a_straight() {
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    // The new outer track comes in on a flat arc (R 1500, left, from (100, −4.5) heading east).
    let (end, b_end) = arc_forward(E0 + 100.0, N0 - 4.5, 90.0, -1500.0, 60.0 / 1500.0);
    let arc = json!({"start": [E0 + 100.0, N0 - 4.5], "end": [end.0, end.1], "bearing": b_end, "radius": -1500.0});
    let ask = |radius: f64, cl: Value| {
        let cl = merge(json!({"ref": inner(0.0), "dMin": 4.0, "profile": profile(), "cantModel": model()}), cl);
        best(&rules, json!({"dep": arc, "arr": arr(0.0), "radius": radius, "clearance": cl}))
    };
    let t0 = Instant::now();
    let res = ask(300.0, json!({"maximize": true, "speed": 0, "lMin": 12}));
    assert!(t0.elapsed().as_secs_f64() < 5.0);
    let c = &res["info"]["clearance"];
    let r = c["radius"].as_f64().unwrap();
    assert!(
        c["kept"] == true && (200.0..=320.0).contains(&r) && res["info"]["arcLength"].as_f64().unwrap() >= 12.0,
        "{res}"
    );
    let over = ask(r + 3.0, json!({}));
    assert!(over.get("error").is_some() || over["info"]["clearance"]["kept"] == false);
    assert_eq!(ask(r, json!({}))["info"]["clearance"]["kept"], true);
}

#[test]
fn two_arcs_are_checked_not_searched() {
    let katalog = common::katalog();
    let rules = Rules::new(&katalog).unwrap();
    let (a, b_a) = arc_forward(E0, N0, 20.0, 900.0, 250.0 / 900.0);
    let (x, y, b_s1) = transition_end(a.0, a.1, b_a, 70.0, Some(900.0), None, Profile::Clothoid);
    let s1 = (x, y);
    let rad = b_s1.to_radians();
    let s2 = (s1.0 + 120.0 * rad.sin(), s1.1 + 120.0 * rad.cos());
    let (x, y, b_b) = transition_end(s2.0, s2.1, b_s1, 70.0, None, Some(-700.0), Profile::Clothoid);
    let (c, _) = arc_forward(x, y, b_b, -700.0, 200.0 / 700.0);
    // A neighbour 6 m to the left of the straight between them.
    let left = (-rad.cos(), rad.sin());
    let side_ref: Vec<Value> = (-20..141)
        .map(|t| {
            json!([
                s1.0 + 6.0 * left.0 + f64::from(t) * rad.sin(),
                s1.1 + 6.0 * left.1 + f64::from(t) * rad.cos(),
                0
            ])
        })
        .collect();
    let res = best(
        &rules,
        json!({
            "dep": {"start": [E0, N0], "end": [a.0, a.1], "bearing": b_a, "radius": 900.0},
            "arr": {"start": [c.0, c.1], "end": [x, y], "bearing": (b_b + 180.0) % 360.0, "radius": 700.0},
            "lDep": 70.0, "lArr": 70.0,
            "clearance": {"ref": side_ref, "dMin": 4.0, "profile": profile(), "cantModel": model(), "maximize": true, "speed": 0},
        }),
    );
    let c = &res["info"]["clearance"];
    assert!(res.get("error").is_none() && c.get("radius").is_none() && c["kept"] == true, "{res}");
    assert!((c["distance"].as_f64().unwrap() - 6.0).abs() < 0.05);
}
