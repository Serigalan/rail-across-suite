// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The search held to what the Python solver it replaces did: the cases of
//! its acceptance (services/optimizer/tests/verify_crossover.py, removed with
//! it) on branch ends built here, and two requests as the app makes them with
//! the answers Python gave for them (tests/data, recorded 2026-11-03).

use rail_across_crossover::branch::{BranchEnd, Sample};
use rail_across_crossover::problem::{Field, K_SCALE, Kind, Limits, Problem, Template};
use rail_across_crossover::solve::{Solution, order};
use rail_across_crossover::{Request, solve, solve_json};
use serde_json::Value;
use std::f64::consts::FRAC_PI_2;

const R: f64 = 500.0;
const N: f64 = 12.0;
const SPEED: f64 = 60.0;

fn limits() -> Limits {
    Limits {
        l_min: Some(6.0),
        rw_reg: Some(490.0),
        rw_discretion: Some(400.0),
        max_cant_def: Some(100.0),
        cant_def_coeff: Some(11.8),
        cant_tol: Some(1.0),
        max_length: None,
    }
}

fn left_arc(x: f64, y: f64, psi: f64, k: f64, length: f64) -> (f64, f64, f64) {
    (
        x + ((psi + k * length).sin() - psi.sin()) / k,
        y - ((psi + k * length).cos() - psi.cos()) / k,
        psi + k * length,
    )
}

/// Samples of both branch ends of a 500 – 1:12 (one arc): turnout 1 on x = 0
/// running +y, turnout 2 on a line through (−gap, 0) heading π/2 + angle,
/// opening back towards −y; both branches turn towards the other track.
fn form(gap: f64, angle_deg: f64, reach: f64) -> Value {
    let a = angle_deg.to_radians();
    let w = (1.0 / N).atan();
    let step = 0.5;
    let n = (2.0 * reach / step).round() as usize;
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for i in 0..=n {
        let s = -reach + i as f64 * step;
        let (x, y, psi) = left_arc(0.0, s, FRAC_PI_2, 1.0 / R, R * w);
        first.push(serde_json::json!({ "s": s, "x": x, "y": y, "psi": psi, "kappa": 1.0 / R, "tail": 0.0, "cantLds": 0.0, "ok": true }));
        let (px, py) = (-gap - s * a.sin(), s * a.cos());
        let (x2, y2, psi2) = left_arc(px, py, -FRAC_PI_2 + a, 1.0 / R, R * w);
        second.push(
            serde_json::json!({ "s": s, "x": x2, "y": y2, "psi": psi2 + std::f64::consts::PI, "kappa": -1.0 / R, "tail": 0.0, "cantLds": 0.0, "ok": true }),
        );
    }
    serde_json::json!({ "label": "500 – 1:12", "first": first, "second": second })
}

fn request(forms: Vec<Value>, extra: Value) -> Request {
    let mut req = serde_json::json!({
        "speed": SPEED,
        "limits": { "lMin": 6.0, "rwReg": 490.0, "rwDiscretion": 400.0, "maxCantDef": 100.0, "cantDefCoeff": 11.8, "cantTol": 1.0 },
        "forms": forms,
    });
    for (k, v) in extra.as_object().unwrap() {
        req[k] = v.clone();
    }
    serde_json::from_value(req).unwrap()
}

fn samples(v: &Value) -> Vec<Sample> {
    serde_json::from_value(v.clone()).unwrap()
}

/// The variables of an answer, as the search had them.
fn variables(sol: &Solution) -> (Template, Vec<f64>) {
    let kinds: Vec<Kind> = sol
        .template
        .iter()
        .map(|t| serde_json::from_value(Value::String(t.to_string())).unwrap())
        .collect();
    let tpl = Template::new(&kinds);
    let mut z = vec![sol.s1, sol.s2];
    for &(i, field) in &tpl.slots {
        z.push(match field {
            Field::Length => sol.pieces[i].length,
            Field::K => sol.pieces[i].kappa.unwrap() * K_SCALE,
        });
    }
    (tpl, z)
}

/// Every answer closes on the second branch and keeps every limit, and they come best first.
fn check(label: &str, form: &Value, limits: &Limits, speed: f64, sols: &[Solution]) {
    assert!(!sols.is_empty(), "{label}: solutions found");
    let (first, second) = (
        BranchEnd::new(&samples(&form["first"])).unwrap(),
        BranchEnd::new(&samples(&form["second"])).unwrap(),
    );
    let problem = Problem::new(&first, &second, limits, speed);
    for sol in sols {
        let (tpl, z) = variables(sol);
        let st = problem.state(&tpl, &z);
        let r = problem.residual(&st);
        assert!(r[0].hypot(r[1]) < 1e-4, "{label} {:?}: closes on the second branch", sol.template);
        let (g, _) = problem.inequalities(&tpl, &st);
        assert!(g.iter().all(|&v| v >= -1e-7), "{label} {:?}: keeps every limit", sol.template);
        assert!(
            sol.pieces.iter().all(|p| p.length >= limits.l_min.unwrap_or(0.0) - 1e-6),
            "{label}: each element at least l_min"
        );
    }
    let mut sorted = sols.to_vec();
    order(&mut sorted);
    let same = sorted.iter().zip(sols).all(|(a, b)| a.template == b.template && a.s1 == b.s1);
    assert!(same, "{label}: best first");
}

#[test]
fn parallel_tracks_four_metres_apart_give_the_classical_straight() {
    let f = form(4.0, 0.0, 100.0);
    let sols = solve(&request(vec![f.clone()], serde_json::json!({}))).unwrap();
    check("parallel 4.0 m", &f, &limits(), SPEED, &sols);
    // The one arc with no curvature, at the click, of 6.57 m.
    let single = sols.iter().find(|s| s.template == ["arc"]).expect("the one arc");
    assert!(single.s1.abs() < 0.5);
    assert!((single.pieces[0].length - 6.57).abs() < 0.05);
    assert!(single.pieces[0].kappa.unwrap().abs() < 1e-6);
    // Nearest the click first.
    assert!(sols[0].s1.abs() < 0.5);
}

#[test]
fn converging_tracks_find_a_chain_where_one_arc_jumps_too_far() {
    let f = form(7.0, -1.5, 100.0);
    let sols = solve(&request(vec![f.clone()], serde_json::json!({}))).unwrap();
    check("converging 1.5°", &f, &limits(), SPEED, &sols);
    assert!(sols.iter().any(|s| s.pieces.len() > 1));
}

#[test]
fn branches_that_overlap_have_no_solution() {
    let f = form(2.5, 0.0, 40.0);
    let out: Value = serde_json::from_str(&solve_json(
        &serde_json::to_string(&serde_json::json!({
            "speed": SPEED, "limits": { "lMin": 6.0, "rwReg": 490.0, "rwDiscretion": 400.0, "maxCantDef": 100.0 }, "forms": [f],
        }))
        .unwrap(),
    ))
    .unwrap();
    assert_eq!(out["error"], "no_solution");
}

#[test]
fn elements_given_are_kept_and_a_given_radius_bends_either_way() {
    let f = form(7.0, -1.5, 100.0);
    let req = request(
        vec![f.clone()],
        serde_json::json!({
            "elements": [{ "type": "arc", "radius": 1500.0 }, { "type": "straight" }, { "type": "arc" }],
        }),
    );
    let sols = solve(&req).unwrap();
    check("given radius", &f, &limits(), SPEED, &sols);
    assert_eq!(sols.len(), 1, "one sequence, its best answer");
    let k = sols[0].pieces[0].kappa.unwrap();
    assert!((1.0 / k.abs() - 1500.0).abs() < 1e-6);

    let req = request(
        vec![f.clone()],
        serde_json::json!({ "elements": [{ "type": "straight", "length": 12.5 }, { "type": "arc" }] }),
    );
    let sols = solve(&req).unwrap();
    check("given length", &f, &limits(), SPEED, &sols);
    assert!((sols[0].pieces[0].length - 12.5).abs() < 1e-9);
}

#[test]
fn only_the_sequences_named_are_tried() {
    let f = form(7.0, -1.5, 100.0);
    let req = request(
        vec![f.clone()],
        serde_json::json!({ "templates": [["transition", "arc"], ["arc", "transition"]] }),
    );
    let sols = solve(&req).unwrap();
    check("named sequences", &f, &limits(), SPEED, &sols);
    assert!(sols.iter().all(|s| s.template.len() == 2 && s.template.contains(&"transition")));
}

#[test]
fn a_request_it_cannot_read_says_so() {
    let out: Value = serde_json::from_str(&solve_json("{\"speed\": 0, \"forms\": []}")).unwrap();
    assert_eq!(out["error"], "invalid_request");
    let out: Value = serde_json::from_str(&solve_json("not json")).unwrap();
    assert_eq!(out["error"], "invalid_request");
}

/// A request as the app made it and Python's answer to it: the search finds
/// what Python found — a solution of every sequence Python had one of, each
/// as near the click, unless eight better ones leave no room for it — and
/// every one of its own holds.
fn as_python(name: &str) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/");
    let req_json = std::fs::read_to_string(format!("{dir}{name}.request.json")).unwrap();
    let python: Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}{name}.python.json")).unwrap()).unwrap();
    let raw: Value = serde_json::from_str(&req_json).unwrap();
    let req: Request = serde_json::from_str(&req_json).unwrap();
    let sols = solve(&req).unwrap();
    let limits: Limits = serde_json::from_value(raw["limits"].clone()).unwrap();
    check(name, &raw["forms"][0], &limits, req.speed, &sols);
    for theirs in python["solutions"].as_array().unwrap() {
        let template: Vec<&str> = theirs["template"].as_array().unwrap().iter().map(|t| t.as_str().unwrap()).collect();
        let ours = sols.iter().find(|s| s.template == template);
        let Some(ours) = ours else {
            // Eight at most: one of Python's may have made way for a better one.
            assert_eq!(sols.len(), 8, "{name}: {template:?} found");
            continue;
        };
        // As near the click as Python's, in the whole metres the ranking counts.
        assert!(
            ours.s1.abs() <= theirs["s1"].as_f64().unwrap().abs() + 1.0,
            "{name} {template:?}: as near the click as Python's"
        );
    }
}

#[test]
fn finds_what_python_found_beside_a_curve_bending_away() {
    as_python("away800");
}

#[test]
fn finds_what_python_found_on_converging_tracks() {
    as_python("converging");
}
