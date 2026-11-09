// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The rules module's operations, named as the JavaScript names them.

use rail_across_js::{get, number};
use rail_across_rules::alignment_check::Context;
use rail_across_rules::element::Element;
use rail_across_rules::{bundled, cant, check, element_length, rule_fields, speed};
use serde_json::{Value, json};

use crate::ops::Failure;

fn elements(input: &Value, key: &str) -> Result<Vec<Element>, Failure> {
    serde_json::from_value(input.get(key).cloned().unwrap_or(json!([]))).map_err(|e| Failure::new("invalid_elements", Some(e.to_string())))
}

fn context(input: &Value) -> Result<(Option<Element>, Option<Element>), Failure> {
    let side = |i: usize| -> Result<Option<Element>, Failure> {
        match input.get("context").and_then(|c| c.get(i)).filter(|v| !v.is_null()) {
            None => Ok(None),
            Some(v) => serde_json::from_value(v.clone())
                .map(Some)
                .map_err(|e| Failure::new("invalid_elements", Some(e.to_string()))),
        }
    };
    Ok((side(0)?, side(1)?))
}

fn catalogue(e: rail_across_rules::expr::Error) -> Failure {
    Failure::new("catalogue_error", Some(e.to_string()))
}

fn opt(x: Option<f64>) -> Value {
    x.map(number).unwrap_or(Value::Null)
}

/// Runs rules operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let f = |k: &str| input.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let of = |k: &str| get(input, k).and_then(Value::as_f64);
    let r = (|| -> Result<Value, Failure> {
        Ok(match op {
            "rules.constants" => {
                let (min, max) = bundled::catalog_speed_range();
                json!({
                    "MAX_CANT": cant::max_cant(), "CANT_STEP": cant::cant_step(), "MAX_SWITCH_CANT": cant::max_switch_cant(),
                    "MAX_SWITCH_CANT_DEF": cant::max_switch_cant_def(), "MAX_SWITCH_CANT_EXCEPTION": cant::max_switch_cant_exception(),
                    "AUTO_CANT_MODEL": cant::auto_cant_model(), "catalogSpeedRange": { "min": number(min), "max": number(max) },
                })
            }
            "rules.checkTrack" => {
                let els = elements(input, "elements")?;
                let (before, after) = context(input)?;
                check::check_track(
                    &els,
                    Context {
                        before: before.as_ref(),
                        after: after.as_ref(),
                    },
                )
                .map_err(catalogue)?
            }
            "rules.elementVerdict" => {
                let els = elements(input, "elements")?;
                let (before, after) = context(input)?;
                let i = input["i"].as_u64().unwrap_or(0) as usize;
                Value::Array(
                    check::element_verdict(
                        &els,
                        i,
                        Context {
                            before: before.as_ref(),
                            after: after.as_ref(),
                        },
                    )
                    .map_err(catalogue)?,
                )
            }
            "rules.hasRuleError" => {
                let els = elements(input, "elements")?;
                let (before, after) = context(input)?;
                json!(
                    check::has_rule_error(
                        &els,
                        Context {
                            before: before.as_ref(),
                            after: after.as_ref()
                        }
                    )
                    .map_err(catalogue)?
                )
            }
            "rules.ruleMaxSpeed" => {
                let els = elements(input, "elements")?;
                let (before, after) = context(input)?;
                speed::rule_max_speed_json(
                    &els,
                    input["i"].as_u64().unwrap_or(0) as usize,
                    of("cap"),
                    Context {
                        before: before.as_ref(),
                        after: after.as_ref(),
                    },
                )
            }
            "rules.maxSpeeds" => {
                let raw = input["elements"].as_array().cloned().unwrap_or_default();
                Value::Array(speed::max_speeds(&raw, &elements(input, "elements")?, of("cap")))
            }
            "rules.governing" => {
                let raw = input["elements"].as_array().cloned().unwrap_or_default();
                match speed::governing(&raw, input["i"].as_u64().unwrap_or(0) as usize) {
                    Some((radius, c)) => json!({ "radius": number(radius), "cant": number(c) }),
                    None => Value::Null,
                }
            }
            "rules.minElementLength" => opt(element_length::min_element_length(f("v"))),
            "rules.comparisonRadiusLimits" => match element_length::comparison_radius_limits(f("v")) {
                Some((reg, discretion)) => json!({ "reg": number(reg), "discretion": number(discretion) }),
                None => Value::Null,
            },
            "rules.fieldFindings" => rule_fields::field_findings(input.as_array().map(Vec::as_slice).unwrap_or(&[])),
            "rules.cantDefLimit" => number(cant::cant_def_limit(of("speed"))),
            "rules.limitCantDef" => number(cant::limit_cant_def(&input["el"])),
            "rules.cantDefLevel" => cant::cant_def_level(&input["el"], f("cantDef")).map(|s| json!(s)).unwrap_or(Value::Null),
            "rules.computeAutoC" => number(cant::compute_auto_c(f("speed"), f("radius"))),
            "rules.standardCant" => number(cant::standard_cant(f("speed"), f("radius"), input["inSwitch"].as_bool().unwrap_or(false))),
            "rules.cantLimit" => number(cant::cant_limit(input)),
            "rules.clampCant" => number(cant::clamp_cant(f("value"), &input["el"])),
            "rules.worstCantOf" => number(cant::worst_cant_of(input)),
            "rules.cantExceedsLimit" => json!(cant::cant_exceeds_limit(input)),
            "rules.cantExceptionFields" => cant::cant_exception_fields(of("cant"), input.get("reason").and_then(Value::as_str)),
            "rules.cantFromInput" => opt(cant::cant_from_input(&input["draft"], f("min"), f("max"))),
            "rules.switchCantError" => cant::switch_cant_error(of("cant"), input.get("reason").and_then(Value::as_str))
                .map(|s| json!(s))
                .unwrap_or(Value::Null),
            "rules.computeSwitchCant" => number(cant::compute_switch_cant(f("speed"), f("radius"))),
            "rules.computeCantDef" => number(cant::compute_cant_def(f("speed"), f("radius"), f("cant"))),
            "rules.computeCantDefSigned" => number(cant::compute_cant_def_signed(f("speed"), f("radius"), of("cant"))),
            "rules.computeMaxSpeed" => opt(cant::compute_max_speed(f("radius"), of("cant"), f("limit"))),
            "rules.maxSpeedFor" => opt(cant::max_speed_for(&input["el"], f("radius"), of("cant"))),
            _ => return Err(Failure::new("not_rules", None)),
        })
    })();
    match r {
        Err(Failure { ref error, .. }) if error == "not_rules" => None,
        other => Some(other),
    }
}
