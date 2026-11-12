// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The operations of the signalling domain's content — the catalogue, the
//! check, overlaps and the track sections' extent — named as the JavaScript's.

use std::collections::HashSet;

use rail_across_js::{js_string, number};
use rail_across_signalling::{catalogue as cat, check, overlaps, sections};
use serde_json::{Map, Value, json};

use crate::args::{Args, opt_num};
use crate::ops::Failure;

fn found(v: Option<&Value>) -> Value {
    v.cloned().unwrap_or(Value::Null)
}

fn domains(input: &Value) -> (&Value, &Value) {
    let both = input.get(0).unwrap_or(&Value::Null);
    (both.get("core").unwrap_or(&Value::Null), both.get("signalling").unwrap_or(&Value::Null))
}

/// Runs operation `op`, where it is one of these.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    let s = |i: usize| a.opt(i).and_then(Value::as_str);
    match op {
        "signalling.CATALOGUE_REF" => ok(cat::catalogue_ref()),
        "signalling.aspectOf" => ok(found(cat::aspect_of(&js_string(a.opt(0))))),
        "signalling.titleIn" => ok(cat::title_in(a.opt(0), &js_string(a.opt(1)))),
        "signalling.signalKindOf" => ok(found(cat::signal_kind_of(s(0), s(1)))),
        "signalling.knownSystem" => ok(json!(cat::known_system(s(0)))),
        "signalling.screenOf" => ok(cat::screen_of(&a.list(0))),
        "signalling.protectionSystemOf" => ok(found(cat::protection_system_of(s(0)))),
        "signalling.baliseKindOf" => ok(found(cat::balise_kind_of(s(0), s(1)))),
        "signalling.settingFits" => ok(json!(cat::setting_fits(a.v(0), a.v(1)))),
        "signalling.settingsProblems" => {
            let pflicht = a.option(2, "pflicht").and_then(Value::as_array).cloned().unwrap_or_default();
            let erlaubt = a.option(2, "erlaubt").and_then(Value::as_array).map(Vec::as_slice);
            let problems = cat::settings_problems(a.opt(0), a.opt(1), &pflicht, erlaubt);
            ok(Value::Array(problems.into_iter().map(|(p, n)| json!({ "problem": p, "setting": n })).collect()))
        }
        "signalling.boardAspect" => ok(json!(cat::board_aspect(&js_string(a.opt(0))))),
        "signalling.symbolFits" => ok(json!(cat::symbol_fits(a.v(0), a.opt(1)))),
        "signalling.aspectFitsFrame" => ok(json!(cat::aspect_fits_frame(a.v(0), s(1)))),
        "signalling.aspectSpeed" => ok(opt_num(cat::aspect_speed(a.opt(0), a.opt(1)))),
        "signalling.checkSignalling" => {
            let (core, signalling) = domains(input);
            ok(check::check_signalling_on(
                core,
                signalling,
                input.get(0).and_then(|b| b.get("civil")).unwrap_or(&Value::Null),
            ))
        }
        "signalling.overlapLength" => ok(opt_num(overlaps::overlap_length(a.of(0), a.of(1).unwrap_or(0.0)))),
        "signalling.signallingSpeedOf" => ok(found(overlaps::signalling_speed_of(a.v(0), a.v(1)).as_ref())),
        "signalling.gradientAlong" => ok(opt_num(overlaps::gradient_along(&a.list(0), a.v(1), a.v(2), a.f(3)))),
        "signalling.branchSpeedOnRun" => ok(overlaps::branch_speed_json(overlaps::branch_speed_on_run(&a.list(0), &a.list(1)))),
        "signalling.netParts" => {
            let (core, signalling) = domains(input);
            let only: Option<HashSet<String>> = a.opt(1).and_then(Value::as_array).map(|l| l.iter().map(|v| js_string(Some(v))).collect());
            ok(Value::Array(
                sections::net_parts(core, signalling, only.as_ref())
                    .iter()
                    .map(sections::Part::to_json)
                    .collect(),
            ))
        }
        "signalling.trackSectionExtents" => {
            let (core, signalling) = domains(input);
            let mut out = Map::new();
            for (id, e) in sections::track_section_extents(core, signalling) {
                let parts: Vec<Value> = e
                    .parts
                    .iter()
                    .map(|(t, f, to)| json!({ "trackId": t, "from": number(*f), "to": number(*to) }))
                    .collect();
                out.insert(js_string(Some(&id)), json!({ "parts": parts, "matches": e.matches, "missing": e.missing }));
            }
            ok(Value::Object(out))
        }
        _ => None,
    }
}
