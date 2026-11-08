// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The survey module's operations, named and called as the JavaScript's.

use rail_across_js::{js_number, number};
use rail_across_survey::{axis, fit, reference as rf, shift};
use serde_json::{Value, json};

use crate::args::{Args, opt_list, opt_num, opt_str};
use crate::ops::Failure;

fn line(v: &Value) -> shift::Line {
    shift::Line::from_json(v)
}

/// Runs survey operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    let opt_f = |i: usize, key: &str, d: f64| a.option(i, key).map(|v| js_number(Some(v))).unwrap_or(d);
    match op {
        // axisSurvey
        "survey.surveyFromTrace" => ok(axis::survey_from_trace(a.v(0), a.v(1))),
        "survey.surveyPoints" => ok(Value::Array(axis::survey_points(a.v(0)))),
        "survey.withoutPoints" => ok(axis::without_points(
            a.v(0),
            &a.list(1).iter().map(|v| v.as_f64().unwrap_or(-1.0) as usize).collect::<Vec<_>>(),
        )),
        "survey.surveyStats" => ok(axis::survey_stats(a.v(0))),
        "survey.surveyDefect" => ok(opt_str(axis::survey_defect(a.v(0)))),
        // referenceAxis
        "survey.referenceStretches" => ok(Value::Array(
            rf::reference_stretches(a.f(0), a.f(1), a.of(2).unwrap_or(rf::REFERENCE_MAX_LENGTH))
                .into_iter()
                .map(|(f, t)| json!({ "from": number(f), "to": number(t) }))
                .collect(),
        )),
        "survey.buildReferenceAxis" => Some(rf::build_reference_axis(a.v(0)).map_err(|m| Failure {
            error: "thrown".into(),
            detail: Some(m.clone()),
            data: Some(json!({ "message": m })),
        })),
        "survey.axisRange" => {
            let (from, to) = rf::axis_range(a.v(0));
            ok(json!({ "from": number(from), "to": number(to) }))
        }
        "survey.axisOutline" => ok(Value::Array(rf::axis_outline(a.v(0), a.of(1).unwrap_or(1.0)))),
        "survey.referenceAxisDefect" => ok(opt_str(rf::reference_axis_defect(a.v(0)))),
        // shiftValues
        "survey.comparedLine" => {
            let station0 = opt_f(2, "station0", 0.0);
            ok(shift::compared_line(&a.list(0), a.i(1), a.option(2, "heights"), station0).to_json())
        }
        "survey.heightsComparable" => ok(json!(shift::heights_comparable(a.v(0), a.opt(1)))),
        "survey.shiftValues" => {
            let with_heights = a.v(2).get("withHeights").is_none_or(|w| w.is_null() || rail_across_js::truthy(Some(w)));
            ok(Value::Array(shift::shift_values(a.v(0), &line(a.v(1)), opt_f(2, "every", 5.0), with_heights)))
        }
        "survey.referenceProfile" => ok(json!(shift::reference_profile(&a.list(0), a.opt(1), a.f(2)))),
        "survey.deviationAt" => ok(opt_num(shift::deviation_at(&a.list(0), a.f(1)))),
        "survey.shiftSummary" => ok(shift::shift_summary(
            &a.list(0),
            opt_f(1, "limitQ", f64::INFINITY),
            opt_f(1, "limitZ", f64::INFINITY),
        )),
        "survey.nearestAxis" => ok(shift::nearest_axis(&a.list(0), &line(a.v(1)), js_number(a.opt(2)))
            .cloned()
            .unwrap_or(Value::Null)),
        "survey.shiftValuesCsv" => {
            let name = |k: &str| a.option(1, k).and_then(Value::as_str).unwrap_or("").to_string();
            ok(json!(shift::shift_values_csv(&a.list(0), &name("axisName"), &name("lineName"))))
        }
        // alignmentFit
        "survey.surveyAxis" => ok(Value::Array(fit::survey_axis(a.v(0)))),
        "survey.parseAxisPointFile" => ok(fit::parse_axis_point_file(&rail_across_js::js_string(a.opt(0)), a.os(1).unwrap_or(""))),
        "survey.alignRequest" => ok(fit::align_request(&a.list(0), a.option(1, "straights"), a.option(1, "settings"))),
        "survey.fittedCurvature" => ok(Value::Array(fit::fitted_curvature(a.v(0)))),
        "survey.elementReport" => ok(Value::Array(fit::element_report(a.v(0), a.f(1)))),
        "survey.gradientReport" => ok(Value::Array(fit::gradient_report(a.v(0)))),
        "survey.trackFromFit" => ok(fit::track_from_fit(a.v(0), a.option(1, "name"), a.option(1, "epsg"))),
        "survey.straightRanges" => ok(Value::Array(fit::straight_ranges(&a.list(0)))),
        "survey.moveStraightEnd" => ok(Value::Array(fit::move_straight_end(&a.list(0), a.u(1), a.s(2), a.f(3)))),
        "survey.insertStraight" => ok(opt_list(fit::insert_straight(&a.list(0), a.f(1), a.f(2)))),
        "survey.deleteStraight" => ok(Value::Array(fit::delete_straight(&a.list(0), a.u(1)))),
        _ => None,
    }
}
