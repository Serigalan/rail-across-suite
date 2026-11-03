// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The catalogue a run applies and the limits it makes of it — the cases of
//! services/optimizer/tests/verify.py (sections 13 and 14) the Python port
//! was held to.

mod common;

use rail_across_optimizer::element::Element;
use rail_across_optimizer::expr::{Basic, Value, eval_expr};
use rail_across_optimizer::geometry::{CANT_DEFICIENCY_COEFF, Profile, heading_coeffs, permissible_speed};
use rail_across_optimizer::grenzen::{Grenzen, Stufe};
use rail_across_optimizer::load_katalog;
use rail_across_optimizer::pruefung::{Context, element_scope};

#[test]
fn physics_has_not_drifted_from_the_kernel() {
    let physics = common::constraints("physics.json");
    let coeff = &physics["ueberhoehungsfehlbetrag_koeffizient"];
    assert_eq!(coeff["wert"].as_f64(), Some(CANT_DEFICIENCY_COEFF));
    let derived = 1000.0 * physics["wirksame_spurweite"]["wert"].as_f64().unwrap() / (physics["erdbeschleunigung"]["wert"].as_f64().unwrap() * 3.6_f64.powi(2));
    assert_eq!((derived * 10.0).round(), (CANT_DEFICIENCY_COEFF * 10.0).round());
    assert!(((700.0 * (120.0 + 130.0) / CANT_DEFICIENCY_COEFF).sqrt() - permissible_speed(700.0, 120.0, 130.0)).abs() < 1e-12);

    // kruemmung_calc against the kernel: κ(s) is the derivative of the heading
    // polynomial, a1 + 2 a2 s + 3 a3 s² + 4 a4 s³.
    let (k1, k2, length, s) = (-0.01, 0.02, 120.0, 45.0);
    for (name, profile) in physics["uebergangsbogenprofile"].as_object().unwrap() {
        let Some(formula) = profile.get("kruemmung_calc").and_then(|f| f.as_str()) else {
            continue;
        };
        let from_file = eval_expr(
            &formula.replace("**", "^"),
            &[("kappa1", k1.into()), ("kappa2", k2.into()), ("L", length.into()), ("s", s.into())],
            &Basic,
        )
        .unwrap()
        .as_num()
        .unwrap();
        let a = heading_coeffs(Profile::parse(name), k1, k2, length);
        let from_kernel = a[0] + s * (2.0 * a[1] + s * (3.0 * a[2] + s * 4.0 * a[3]));
        assert!((from_file - from_kernel).abs() < 1e-12, "{name}");
    }
}

#[test]
fn the_catalogue_as_regelkatalog_js_applies_it() {
    let k = common::katalog();
    let kb02 = k.rule("LP.KB.02").unwrap();
    let severity = |v: f64, uf: f64| {
        let scope = k.scope(&[("element.design_speed", v.into()), ("physics.u_f", uf.into())]);
        k.evaluate_rule(kb02, &scope, None).unwrap().severity.unwrap()
    };
    assert_eq!(severity(160.0, 140.0), "warning");
    assert_eq!(severity(160.0, 151.0), "error");
    assert_eq!(severity(150.0, 140.0), "error");
}

#[test]
fn limits_at_both_levels() {
    let k = common::katalog();
    let reg = Grenzen::new(&k, Stufe::Reg).unwrap();
    let erm = Grenzen::new(&k, Stufe::Discretion).unwrap();

    let min_len: Vec<f64> = [40.0, 70.0, 75.0, 100.0, 105.0, 300.0]
        .iter()
        .map(|&v| reg.min_length("straight", v, false).unwrap())
        .collect();
    assert_eq!(min_len, [4.0, 7.0, 11.25, 15.0, 21.0, 60.0]);
    assert_eq!(reg.min_length("straight", 40.0, true).unwrap(), 7.2);
    assert_eq!(reg.min_length("straight", 45.0, true).unwrap(), 4.5);

    let rw_reg: Vec<f64> = [40.0, 45.0, 100.0, 200.0, 205.0].iter().map(|&v| reg.jump_need(v, false)).collect();
    assert_eq!(rw_reg, [220.0, 340.0, 1370.0, 10000.0, f64::INFINITY]);
    assert_eq!(erm.jump_need(40.0, false), 178.0);
    assert!((erm.jump_need(45.0, false) - CANT_DEFICIENCY_COEFF * 45.0 * 45.0 / 106.5).abs() < 1e-9);
    assert_eq!((erm.jump_need(100.0, false), erm.jump_need(200.0, false)), (1110.0, 7000.0));

    assert_eq!(reg.speeds, erm.speeds);
    assert_eq!((reg.speeds[0], reg.v_top), (40.0, 300.0));
    assert!(reg.speeds.windows(2).all(|w| w[1] - w[0] == 5.0));
    assert_eq!((reg.u_max(false), erm.u_max(false), reg.u_step, erm.u_step), (160.0, 160.0, 5.0, 5.0));
    assert!(reg.speeds.iter().all(|&v| reg.uf_max(v, false) == 130.0));
    assert_eq!((erm.uf_max(150.0, false), erm.uf_max(155.0, false)), (130.0, 150.0));

    assert!((reg.speed(2000.0, 0.0, false) - permissible_speed(2000.0, 0.0, 130.0)).abs() < 1e-9);
    assert!((erm.speed(2000.0, 0.0, false) - permissible_speed(2000.0, 0.0, 150.0)).abs() < 1e-9);
    assert_eq!((reg.speed(2000.0, 0.0, false).round(), erm.speed(2000.0, 0.0, false).round()), (148.0, 159.0));
    // Just above 150 km/h with 150 mm, its design speed is 150 — so 130 mm.
    assert_eq!(erm.speed(1740.0, 0.0, false), permissible_speed(1740.0, 0.0, 130.0));

    let ramp_reg = reg.ramp_bounds("clothoid", 100.0, 100.0, 0.0).unwrap().unwrap();
    let ramp_erm = erm.ramp_bounds("clothoid", 100.0, 100.0, 0.0).unwrap().unwrap();
    assert_eq!((ramp_reg.shortest, ramp_reg.by.as_deref(), ramp_erm.shortest), (100.0, Some("LP.UB.03"), 80.0));
    let duf = reg.ramp_bounds("clothoid", 100.0, 0.0, 130.0).unwrap().unwrap();
    assert_eq!((duf.shortest, duf.by.as_deref()), (4.0 * 100.0 * 130.0 / 1000.0, Some("LP.UB.05")));
    let slow = reg.ramp_bounds("clothoid", 40.0, 100.0, 0.0).unwrap().unwrap();
    assert_eq!((slow.shortest, slow.by.as_deref()), (60.0, Some("LP.UB.07")));
    assert_eq!(erm.ramp_bounds("clothoid", 40.0, 100.0, 0.0).unwrap().unwrap().shortest, 40.0);
    let flat = reg.ramp_bounds("clothoid", 150.0, 5.0, 0.0).unwrap().unwrap();
    assert_eq!((flat.longest, flat.shortest), (15.0, 30.0));

    assert_eq!(reg.jump_speed(900.0, 120.0, false), Some(80.0));
    assert_eq!(erm.jump_speed(900.0, 120.0, false), Some(90.0));
    assert_eq!(reg.jump_speed(5000.0, 120.0, false), Some(120.0));
    assert_eq!(reg.jump_speed(100.0, 120.0, false), None);
    assert_eq!(reg.jump_speed(100.0, 120.0, true), Some(120.0));

    assert!(Stufe::parse("approval").is_err());
    assert_eq!(reg.forms, [("clothoid", true), ("bloss", false)]);
    assert_eq!(erm.forms, [("clothoid", true), ("bloss", true)]);
}

#[test]
fn a_rule_a_run_cannot_keep_is_refused() {
    let mut data = common::constraints("db-ril-800-0110.json");
    data["rules"].as_array_mut().unwrap().push(serde_json::json!({
        "id": "LP.TEST.01", "title": "Mindestradius", "status": "confirmed",
        "applies_to": {"scope": "element", "element_types": ["circular_arc"]},
        "inputs": {"r": {"from": "element.radius", "unit": "m"}},
        "thresholds": {"min": {"expr": "300", "unit": "m"}},
        "evaluation": [{"if": "r >= min", "severity": "ok"}, {"else": "error"}]}));
    let k = load_katalog(&data).unwrap();
    let Err(e) = Grenzen::new(&k, Stufe::Reg) else { panic!("accepted") };
    assert!(e.0.contains("LP.TEST.01"), "{e}");

    // The same rule as a mere hint binds no run.
    let last = data["rules"].as_array().unwrap().len() - 1;
    data["rules"][last]["evaluation"][1] = serde_json::json!({"else": "hint"});
    let k = load_katalog(&data).unwrap();
    assert!(Grenzen::new(&k, Stufe::Reg).is_ok());
}

#[test]
fn a_ramp_on_an_arc_reads_its_cant_from_its_ends() {
    // A ramp on an arc stands in the record as cantStart/cantEnd only
    // (decision 369): the check reads its one cant from them as
    // geometry.withCantRamp does — the end that helps the curve least.
    let ramp = Element {
        element_type: Some(1),
        radius: Some(-950.0),
        length: Some(12.0),
        speed: Some(80.0),
        cant_start: Some(-21.875),
        cant_end: Some(-40.625),
        ..Element::default()
    };
    let with = element_scope(
        &[Element {
            cant: Some(-21.875),
            ..ramp.clone()
        }],
        0,
        Context::default(),
    );
    let without = element_scope(&[ramp], 0, Context::default());
    let get = |scope: &[(&str, Value)], name: &str| scope.iter().find(|(n, _)| *n == name).unwrap().1.clone();
    assert_eq!(get(&without, "element.cant"), Value::Num(21.875));
    assert_eq!(get(&with, "element.cant"), get(&without, "element.cant"));
    assert_eq!(get(&with, "physics.u_f"), get(&without, "physics.u_f"));
}
