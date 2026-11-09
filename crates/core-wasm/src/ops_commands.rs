// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The commands' operations, named and called as the JavaScript's.

use rail_across_commands::merge::{carry as mcarry, diff as mdiff, remap as mremap, three_way as m3};
use rail_across_commands::{
    connect as cn, crossover as co, crossover_edit as ce, edit, optimize as opt, reconnect as rc, roles, scurve as sc, splice as sp, switches as swc, table,
    tracks as tr,
};
use rail_across_js::number;
use rail_across_switch::mid_chain as mc;
use serde_json::{Value, json};

use crate::args::{Args, opt_num, opt_str};
use crate::ops::Failure;

/// Runs command operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    match op {
        // tracks
        "commands.trackMeta" => ok(tr::track_meta(a.v(0), a.v(1))),
        "commands.trackOf" => ok(tr::track_of(a.list(0), a.v(1), a.opt(2).unwrap_or(&json!({})), a.opt(3))),
        "commands.buildLineTrack" => ok(tr::build_line_track(a.v(0))),
        "commands.curveCant" => ok(number(tr::curve_cant(a.f(0), a.opt(1)))),
        "commands.buildCurvedLineTrack" => ok(tr::build_curved_line_track(a.v(0))),
        "commands.buildParallelLineTrack" => ok(tr::build_parallel_line_track(a.v(0))),
        "commands.buildParallelTrack" => ok(tr::build_parallel_track(a.v(0))),
        "commands.buildConnectStraight" => ok(Value::Array(tr::build_connect_straight(a.v(0)))),
        "commands.cantAtTrackEnd" => ok(opt_num(tr::cant_at_track_end(a.opt(0)))),
        "commands.buildConnectCurved" => ok(Value::Array(tr::build_connect_curved(a.v(0)))),
        "commands.translateTracks" => ok(tr::translate_tracks(a.v(0), &a.list(1), a.f(2), a.f(3))),
        // elementRoles
        "commands.intermediateSpan" => ok(roles::span_json(roles::intermediate_span(&a.list(0), a.u(1)))),
        "commands.elementRoles" => ok(Value::Array(roles::element_roles(&a.list(0)))),
        "commands.rolesFor" => ok(json!(roles::roles_for(a.v(0)))),
        "commands.withRole" => {
            let (elements, error) = roles::with_role(&a.list(0), a.u(1), a.s(2));
            ok(json!({ "elements": elements, "error": error }))
        }
        // tableView
        "commands.tableParts" => ok(table::table_parts(a.v(0), &a.list(1), &a.list(2))),
        "commands.tableRows" => ok(table::table_rows(&a.list(0), &a.list(1))),
        "commands.rowOf" => ok(json!(table::row_of(&a.list(0), a.v(1), a.u(2)))),
        "commands.tableRoles" => ok(Value::Array(table::table_roles(&a.list(0), &a.list(1)))),
        "commands.trackEdit" => ok(table::track_edit(a.v(0), a.s(1), a.v(2))),
        "commands.writeRows" => ok(Value::Array(table::write_rows(&a.list(0), &a.list(1), &a.list(2)))),
        "commands.inRunElementChange" => {
            let (track_id, change) = (a.v(0)["trackId"].clone(), a.v(3).clone());
            ok(table::in_run(a.v(0), &a.list(1), &a.list(2), &|t, s, i| {
                edit::plan_element_change(t, s, &track_id, i, &change)
            }))
        }
        // editGeometry
        "commands.planElementChange" => ok(edit::plan_element_change(&a.list(0), &a.list(1), a.v(2), a.u(3), a.v(4))),
        "commands.planIntermediateChange" => ok(edit::plan_intermediate_change(&a.list(0), a.v(1), a.u(2), a.of(3))),
        "commands.mergeElementEdits" => {
            let list = |k: &str| a.option(2, k).and_then(Value::as_array).cloned().unwrap_or_default();
            ok(Value::Array(edit::merge_element_edits(
                &a.list(0),
                &a.list(1),
                &list("changed"),
                &list("reshaped"),
            )))
        }
        // sCurve
        "commands.connectionPick" => ok(sc::connection_pick(a.v(0), a.u(1), a.f(2))),
        "commands.isUsableStem" => ok(json!(sc::is_usable_stem(a.v(0)))),
        "commands.runBeside" => {
            let els = a.list(0);
            ok(sc::run_beside(&els, a.u(1), a.s(2))
                .map(|l| {
                    let el = &els[a.u(1)];
                    json!({ "elementType": el.get("elementType"), "radius": rail_across_js::get(el, "radius").cloned().unwrap_or(Value::Null), "length": number(l) })
                })
                .unwrap_or(Value::Null))
        }
        "commands.stems" => {
            let (g1, g2) = sc::stems(&a.list(0));
            ok(json!({ "g1": g1.to_json(), "g2": g2.to_json() }))
        }
        "commands.REASON_MSG" => ok(sc::reason_msg()),
        "commands.solveConnection" => with_ask(a.v(0), |ask, shift| ok(sc::solve_connection(ask, shift))),
        "commands.settleConnection" => with_ask(a.v(0), |ask, shift| ok(sc::settle_connection(ask, shift).to_json())),
        "commands.connectionRemnants" => ok(Value::Array(
            sc::connection_remnants(&a.list(0), a.f(1), a.v(2), a.f(3))
                .iter()
                .map(|r| r.to_json())
                .collect(),
        )),
        "commands.computeShiftBounds" => with_ask(a.v(0), |ask, _| {
            let (min, max) = sc::compute_shift_bounds(ask);
            ok(json!({ "min": number(min), "max": number(max) }))
        }),
        "commands.waOffset" => ok(number(sc::wa_offset(&a.list(0)))),
        "commands.hasStraightMid" => ok(json!(sc::has_straight_mid(a.v(0)))),
        "commands.straightShifts" => with_ask(a.v(0), |ask, _| {
            ok(Value::Array(sc::straight_shifts(ask, bounds(a.v(1))).into_iter().map(number).collect()))
        }),
        "commands.shiftSnaps" => with_ask(a.v(0), |ask, _| {
            ok(Value::Array(sc::shift_snaps(ask, bounds(a.v(1))).into_iter().map(number).collect()))
        }),
        "commands.preferredShift" => with_ask(a.v(0), |ask, _| ok(number(sc::preferred_shift(ask, bounds(a.v(1)))))),
        "commands.buildSCurve" => ok(sc::build_s_curve(a.v(0))),
        // switches
        "commands.SLIDER_REACH" => ok(number(swc::SLIDER_REACH)),
        "commands.switchOnTrackPlacement" => {
            let o = a.v(0);
            ok(swc::placement_json(swc::switch_on_track_placement(
                &swc::OnTrack::from_json(o),
                o.get("toeStation").and_then(Value::as_f64),
            )))
        }
        "commands.switchOnTrackRouteEnd" => ok(swc::switch_on_track_route_end(&swc::OnTrack::from_json(a.v(0)), a.f(1))
            .map(|(u, b, k)| json!({ "utm": u.to_json(), "bearing": number(b), "k": number(k) }))
            .unwrap_or(Value::Null)),
        "commands.switchOnTrackRemnant" => ok(swc::switch_on_track_remnant(a.v(0), a.f(1), a.b(2), a.of(3))),
        "commands.switchOnTrackToeValid" => ok(json!(swc::switch_on_track_toe_valid(&swc::OnTrack::from_json(a.v(0)), a.f(1)))),
        "commands.switchOnTrackSlider" => ok(swc::switch_on_track_slider(&swc::OnTrack::from_json(a.v(0)), a.of(1))),
        "commands.buildSwitchOnTrack" => {
            // The placement as the dialog settled on it: laid again from its station.
            let o = a.v(0);
            let place = rail_across_switch::placement::place_switch_on_track(
                &o["track"],
                rail_across_js::js_number(o.get("toeStation")),
                rail_across_js::truthy(o.get("reversed")),
                rail_across_js::js_number(o.get("straightLen")),
            );
            ok(match place {
                Ok(p) => swc::build_switch_on_track(o, &p),
                Err(_) => Value::Null,
            })
        }
        "commands.buildSwitchAtTrackEnd" => ok(swc::build_switch_at_track_end(a.v(0))),
        "commands.buildCrossingOnTrack" => {
            let o = a.v(0);
            let place = rail_across_switch::placement::place_switch_on_track(
                &o["track"],
                rail_across_js::js_number(o.get("pointStation")),
                false,
                rail_across_js::js_number(o.get("endDist")),
            );
            ok(match place {
                Ok(p) => swc::build_crossing_on_track(o, &p),
                Err(_) => Value::Null,
            })
        }
        "commands.crossingOnTrackPlacement" => {
            let o = a.v(0);
            let ask = crossing_ask(o);
            ok(match swc::crossing_on_track_placement(&ask, o.get("pointStation").and_then(Value::as_f64)) {
                Err(e) => json!({ "error": e }),
                Ok(None) => json!({ "error": null }),
                Ok(Some((ahead, back, geom))) => json!({ "error": null, "ahead": ahead.to_json(), "back": back.to_json(), "geom": geom }),
            })
        }
        "commands.crossingOnTrackPort" => {
            let o = a.v(0);
            ok(
                swc::crossing_on_track_port(&crossing_ask(o), rail_across_js::js_number(o.get("pointStation")), a.s(1))
                    .map(|(u, b)| json!({ "utm": u.to_json(), "bearing": number(b) }))
                    .unwrap_or(Value::Null),
            )
        }
        "commands.crossingOnTrackSlider" => ok(swc::crossing_on_track_slider(&crossing_ask(a.v(0)), a.of(1))),
        // connectSwitch, connectCrossing, connectRemedies
        "commands.hostPointAt" => ok(cn::host_point_at(a.v(0), a.f(1), a.of(2).unwrap_or(0.0))
            .map(|u| u.to_json())
            .unwrap_or(Value::Null)),
        "commands.switchConnectSolutions" => ok(cn::switch_connect_solutions(a.v(0))),
        "commands.switchConnectChainSolutions" => ok(cn::switch_connect_chain_solutions(a.v(0), &mc::mids_from(&a.v(0)["mids"]))),
        "commands.switchConnectOutcome" => ok(cn::switch_connect_outcome(a.v(0))),
        "commands.refitOutcome" | "commands.crossingConnectOutcome" => {
            let o = a.v(0);
            let s = &o["solution"];
            let fit = if s["fit"].get("pieces").is_some() {
                let p = rail_across_switch::route::to_chain(&s["fit"]["pieces"]);
                rail_across_switch::connect::Fit {
                    route: p[0],
                    lead: None,
                    end_bearing: f64::NAN,
                    miss: 0.0,
                    pieces: Some(p),
                }
            } else {
                rail_across_switch::connect::Fit::from_json(&s["fit"])
            };
            ok(cn::refit_outcome(
                &o["target"],
                &rail_across_switch::connect::Anchor::from_json(&o["anchor"]),
                &fit,
                rail_across_geom::element::Utm::from_json(&s["at"]["port"]),
                o.get("speedLimit").and_then(Value::as_f64),
            ))
        }
        "commands.buildSwitchConnect" => ok(cn::build_switch_connect(a.v(0))),
        "commands.crossAngleOf" => ok(number(cn::cross_angle_of(a.v(0), a.s(1)))),
        "commands.crossingConnectSolutions" => ok(cn::crossing_connect_solutions(a.v(0))),
        "commands.buildCrossingConnect" => ok(cn::build_crossing_connect(a.v(0))),
        "commands.connectRemediesForSwitch" => {
            // The remedies as the turnout dialog asks them: each lead solved again near the base solution.
            let o = a.v(0);
            let anchor = rail_across_switch::connect::Anchor::from_json(&o["anchor"]);
            let base = o["baseSolution"].clone();
            let outcome_for = |c: &Value| -> Value {
                let lead = c.get("lead").and_then(Value::as_f64).unwrap_or(0.0);
                let s = if lead > 0.0 {
                    let ask = json!({ "track": o["track"], "clickStation": base["x"], "anchor": o["anchor"], "sw": o["sw"], "lead": number(lead), "only": o["only"], "reach": number(lead + 60.0) });
                    cn::switch_connect_solutions(&ask)["solutions"].get(0).cloned().unwrap_or(Value::Null)
                } else {
                    base.clone()
                };
                if s.is_null() {
                    return Value::Null;
                }
                cn::switch_connect_outcome(
                    &json!({ "track": o["track"], "target": o["target"], "anchor": o["anchor"], "solution": s, "speed": o["speed"], "speedLimit": c.get("speedLimit") }),
                )
            };
            ok(Value::Array(cn::connect_remedies(&anchor, &o["current"], &o["outcome"], &outcome_for)))
        }
        // splice
        "commands.splicePick" => ok(sp::splice_pick(a.v(0), a.u(1))),
        "commands.pickRuns" => {
            let (b, af) = sp::pick_runs(&a.list(0), a.u(1));
            ok(json!({ "runBefore": number(b), "runAfter": number(af) }))
        }
        "commands.secondPickRefusal" => ok(opt_str(sp::second_pick_refusal(a.v(0), a.v(1)))),
        "commands.picksAtSpeed" => ok(Value::Array(sp::picks_at_speed(&a.list(0), a.opt(1)))),
        "commands.spliceKeptCant" => ok(opt_num(sp::splice_kept_cant(&a.list(0), a.opt(1)))),
        "commands.spliceRequest" => ok(sp::splice_request(a.v(0), a.v(1), a.v(2), a.opt(3))),
        "commands.neighbourAxis" => ok(Value::Array(sp::neighbour_axis(a.v(0), a.f(1), a.v(2), a.v(3)))),
        "commands.neighbourAxisNear" => {
            let corners: Vec<rail_across_geom::element::Utm> = a.list(2).iter().map(rail_across_geom::element::Utm::from_json).collect();
            ok(Value::Array(sp::neighbour_axis_near(a.v(0), a.f(1), &corners)))
        }
        "commands.clearanceRequest" => ok(sp::clearance_request(a.v(0))),
        "commands.spliceFromAnswer" => ok(sp::splice_from_answer(a.v(0), &a.list(1), a.opt(2))),
        "commands.settleLengths" => ok(sp::settle_lengths(a.v(0), a.opt(1))),
        "commands.buildSplice" => ok(sp::build_splice(a.v(0))),
        // reconnect
        "commands.AXIS_SPACING" => ok(number(rc::AXIS_SPACING)),
        "commands.reconnectRange" => ok(rc::reconnect_range(a.v(0), a.u(1), a.u(2))),
        "commands.replacedSpan" => {
            let (first, last) = rc::replaced_span(a.v(0));
            ok(json!({ "first": first, "last": last }))
        }
        "commands.reconnectPick" => ok(rc::reconnect_pick(a.v(0), a.u(1), a.s(2), a.b(3))),
        "commands.elementPoints" => ok(Value::Array(
            rc::element_points(a.v(0), a.i(1), a.f(2))
                .into_iter()
                .map(|p| json!([number(p[0]), number(p[1])]))
                .collect(),
        )),
        "commands.axisPoints" => ok(rc::axis_points(a.v(0), a.u(1), a.u(2), a.of(3).unwrap_or(rc::AXIS_SPACING))),
        "commands.stretchDefaults" => ok(rc::stretch_defaults(a.v(0), a.v(1))),
        "commands.reconnectRequest" => ok(rc::reconnect_request(&a.list(0), a.v(1), a.v(2), a.opt(3))),
        "commands.reconnectFromAnswer" => ok(rc::reconnect_from_answer(a.v(0), &a.list(1), a.opt(2))),
        "commands.stationMap" => ok(number(rc::station_map(a.f(0), a.f(1), a.f(2), a.f(3)))),
        "commands.buildReconnect" => {
            let o = a.v(0);
            ok(rc::build_reconnect(&o["track"], &o["range"], &o["solution"], o.get("speed"))
                .map(|r| json!({ "track": r["track"] }))
                .unwrap_or(Value::Null))
        }
        // crossover, crossoverEdit
        "commands.crossoverRequest" => {
            let o = a.v(0);
            ok(co::crossover_request(
                o["picks"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                rail_across_js::js_number(o.get("speed")),
                o.get("form").filter(|f| !f.is_null()),
            ))
        }
        "commands.adoptCrossover" => {
            let o = a.v(0);
            ok(co::adopt_crossover(
                o["picks"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                rail_across_js::js_number(o.get("speed")),
                &o["solution"],
                o.get("form").filter(|f| !f.is_null()),
                rail_across_js::truthy(o.get("lenient")),
                rail_across_js::truthy(o.get("exactFirst")),
            )
            .unwrap_or(Value::Null))
        }
        "commands.switchConnectionOf" => {
            let o = a.v(0);
            ok(ce::switch_connection_of(
                &o["trackId"],
                o["tracks"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                o["switches"].as_array().map(Vec::as_slice).unwrap_or(&[]),
            ))
        }
        "commands.takeApart" => {
            let o = a.v(1);
            ok(ce::take_apart(
                a.v(0),
                o["tracks"].as_array().map(Vec::as_slice).unwrap_or(&[]),
                o["switches"].as_array().map(Vec::as_slice).unwrap_or(&[]),
            ))
        }
        "commands.editStart" => ok(ce::edit_start(a.v(0), a.v(1))),
        "commands.editCommit" => ok(ce::edit_commit(a.v(0), a.v(1), a.v(2))),
        // optimizeApply
        "commands.optimizeRequest" => ok(opt::optimize_request(a.v(0), a.v(1))),
        "commands.reshapedHeights" => ok(opt::reshaped_heights(a.v(0), &a.list(1)).unwrap_or(Value::Null)),
        "commands.optimizedTrack" => ok(opt::optimized_track(a.v(0), a.v(1))),
        // merge
        "merge.primary" => ok(mdiff::primary(a.v(0))),
        "merge.diffProject" => ok(mdiff::diff_project(a.v(0), a.v(1))),
        "merge.diffSize" => ok(json!(mdiff::diff_size(a.v(0)))),
        "merge.diffEntries" => ok(Value::Array(mdiff::diff_entries(a.v(0)))),
        "merge.mergeProject" => ok(m3::merge_project(a.v(0))),
        "merge.validationConflicts" => ok(m3::validation_conflicts(a.v(0), a.v(1))),
        "merge.resolve" => ok(m3::resolve(a.v(0), a.opt(1).unwrap_or(&json!({})))),
        "merge.remapKind" => ok(opt_str(mremap::remap_kind(a.s(0), &a.list(1)))),
        "merge.carryAfterWrite" => ok(mcarry::carry_after_write(a.v(0), a.v(1))),
        "merge.carryDomainOnto" => {
            let (domain, carried) = mcarry::carry_domain_onto(a.v(0), &a.list(1), &a.list(2), &a.list(3));
            ok(json!({ "domain": domain, "carried": carried.iter().map(mcarry::Moved::to_json).collect::<Vec<_>>() }))
        }
        // The references to tracks gone repointed, the tracks before handed as a list.
        "merge.carryReferences" => {
            let logs: Vec<Vec<Value>> = a.list(1).iter().map(|l| l.as_array().cloned().unwrap_or_default()).collect();
            let logs: Vec<&[Value]> = logs.iter().map(Vec::as_slice).collect();
            let old = a.list(2);
            let old_track = |id: &str| old.iter().find(|t| rail_across_js::js_string(t.get("id")) == id).cloned();
            let (project, carried) = mremap::carry_references(a.v(0), &logs, &old_track);
            ok(json!({ "project": project, "carried": carried.iter().map(mremap::Carried::to_json).collect::<Vec<_>>() }))
        }
        _ => None,
    }
}

fn crossing_ask(o: &Value) -> swc::CrossingAsk<'_> {
    swc::CrossingAsk {
        track: &o["track"],
        end_dist: rail_across_js::js_number(o.get("endDist")),
        form: &o["form"],
        cross_angle: rail_across_js::js_number(o.get("crossAngle")),
    }
}

fn bounds(v: &Value) -> (f64, f64) {
    (
        v.get("min").and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get("max").and_then(Value::as_f64).unwrap_or(f64::NAN),
    )
}

/// Runs `then` with the ask an options object `{ picks, speed, shift, mids, seed, form, lenient }` states.
fn with_ask<T>(o: &Value, then: impl FnOnce(&sc::Ask, f64) -> T) -> T {
    let picks: Vec<Value> = o.get("picks").and_then(Value::as_array).cloned().unwrap_or_default();
    let seed = o.get("seed").and_then(|s| {
        Some((
            s.get("ds")?.as_f64()?,
            s.get("values")?.as_array()?.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect(),
        ))
    });
    let ask = sc::Ask {
        picks: &picks,
        speed: rail_across_js::js_number(o.get("speed")),
        mids: o.get("mids").filter(|m| !m.is_null()).map(mc::mids_from),
        seed,
        form: o.get("form").filter(|f| !f.is_null()),
        lenient: rail_across_js::truthy(o.get("lenient")),
    };
    then(&ask, o.get("shift").and_then(Value::as_f64).unwrap_or(0.0))
}
