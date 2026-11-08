// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track module's operations, named and called as the JavaScript's.

use std::collections::{HashMap, HashSet};

use rail_across_js::{js_string, number};
use rail_across_track::{
    bands, end_marks as em, groups, heights as h, ids, merge, model, platforms as pf, routes, split, status, switch_model as sm, topology as tp,
};
use serde_json::{Value, json};

use crate::args::{Args, opt_list, opt_num, opt_str};
use crate::ops::Failure;

fn pairs(v: Vec<(Value, String)>, key: &str) -> Value {
    Value::Array(v.into_iter().map(|(t, e)| json!({ key: t, "endpoint": e })).collect())
}

/// Runs track operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    match op {
        // switchModel
        "track.switchRoutePorts" => ok(Value::Object(
            sm::switch_route_ports(a.os(0)).into_iter().map(|(r, p)| (r.to_string(), json!(p))).collect(),
        )),
        "track.switchRoutes" => ok(json!(sm::switch_routes(a.os(0)))),
        "track.turnoutLinePort" => ok(json!(sm::turnout_line_port(a.v(0)))),
        "track.turnoutDivergingPort" => ok(json!(sm::turnout_diverging_port(a.v(0)))),
        "track.turnoutLineRoute" => ok(json!(sm::turnout_line_route(a.v(0)))),
        "track.switchKindLabelKey" => ok(json!(sm::switch_kind_label_key(a.os(0)))),
        "track.switchRouteLabelKey" => ok(opt_str(sm::switch_route_label_key(a.os(0), a.s(1)))),
        "track.isLinkSwitch" => ok(json!(sm::is_link_switch(a.v(0)))),
        "track.switchElementMark" => ok(sm::switch_element_mark(a.v(0), a.s(1))),
        "track.elementBelongsToSwitch" => ok(json!(sm::element_belongs_to_switch(a.v(0), a.v(1)))),
        "track.newSwitchFields" => ok(sm::new_switch_fields(a.os(0).unwrap_or(sm::DEFAULT_SWITCH_KIND))),
        "track.isModelledSwitch" => ok(json!(sm::is_modelled_switch(a.v(0)))),
        "track.switchPorts" => ok(sm::ports_json(&sm::switch_ports(a.os(0)))),
        "track.portsOf" => ok(sm::ports_json(&sm::ports_of(a.v(0)))),
        "track.portTrack" => ok(sm::port_track(a.v(0), a.s(1)).cloned().unwrap_or(Value::Null)),
        "track.portEnd" => ok(opt_str(sm::port_end(a.v(0), a.s(1)))),
        "track.withPort" => ok(sm::with_port(a.v(0), a.s(1), a.v(2))),
        "track.elementOnSwitchRoute" => ok(json!(sm::element_on_switch_route(a.v(0), a.v(1), a.s(2)))),
        "track.unmarkSwitchElement" => ok(sm::unmark_switch_element(a.v(0))),
        // trackEndMarks
        "track.endKey" => ok(json!(em::end_key(a.opt(0), a.opt(1)))),
        "track.newBufferStop" => ok(em::new_buffer_stop(a.v(0), a.v(1), a.opt(2), a.of(3))),
        "track.newBoundary" => ok(em::new_boundary(a.v(0), a.v(1), a.opt(2))),
        "track.endMarkName" => ok(json!(em::end_mark_name(a.v(0), a.v(1)))),
        "track.bufferStopFits" => ok(json!(em::buffer_stop_fits(a.f(0), a.f(1)))),
        "track.bufferStopStations" => ok(em::buffer_stop_stations_json(a.f(0), a.v(1))),
        "track.endMarkStation" => ok(number(em::end_mark_station(a.f(0), a.v(1)))),
        "track.remapEndMarks" => ok(em::remap_end_marks(a.v(0), &a.list(1), &a.list(2))),
        "track.flipEndMarks" => ok(em::flip_end_marks(a.v(0), a.v(1))),
        "track.pruneEndMarks" => ok(em::prune_end_marks(a.v(0), &a.list(1), &a.list(2))),
        // identifierUtils, heightDatums
        "track.buildTypeFields" => ok(ids::build_type_fields(a.v(0))),
        "track.switchDesignation" => ok(json!(ids::switch_designation(a.v(0), a.os(1)))),
        "track.switchNumberOf" => ok(opt_num(ids::switch_number_of(a.v(0)))),
        "track.switchNumbersInUse" => ok(json!(ids::switch_numbers_in_use(&a.list(0)))),
        "track.nextSwitchNumber" => ok(number(ids::next_switch_number(
            &a.list(0),
            &a.list(1).iter().filter_map(Value::as_f64).collect::<Vec<_>>(),
        ))),
        "track.heightDatumLabel" => ok(json!(ids::height_datum_label(a.opt(0)))),
        // planStatus, search
        "track.trackStatus" => ok(json!(status::track_status(a.opt(0)))),
        "track.switchStatus" => {
            let tracks = a.list(1);
            let by_id: HashMap<String, &Value> = tracks.iter().map(|t| (js_string(t.get("id")), t)).collect();
            ok(json!(status::switch_status(a.v(0), &by_id)))
        }
        "track.trackMatches" => ok(json!(status::track_matches(a.v(0), a.s(1)))),
        "track.switchMatches" => ok(json!(status::switch_matches(a.v(0), a.s(1)))),
        // trackModel
        "track.portTracks" => ok(Value::Array(model::port_tracks(a.v(0)))),
        "track.referencesTrack" => ok(json!(model::references_track(a.v(0), a.v(1)))),
        "track.remapSwitches" => ok(Value::Array(model::remap_switches(&a.list(0), &a.list(1)))),
        "track.flipSwitchEndpoints" => ok(Value::Array(model::flip_switch_endpoints(&a.list(0), a.v(1)))),
        "track.rebuildCoords" => ok(Value::Array(model::rebuild_coords(&a.list(0)))),
        "track.nextTrackName" => ok(json!(model::next_track_name(a.s(0), &a.strings(1)))),
        "track.makeTrack" => ok(model::make_track(a.v(0), a.list(1), a.opt(2), a.opt(3))),
        "track.reverseTrack" => ok(model::reverse_track(a.v(0))),
        "track.joinTracks" => ok(model::join_tracks(a.v(0), a.v(1))),
        "track.trackEndAnchor" => ok(model::track_end_anchor(a.v(0))),
        "track.inheritedSpeed" => ok(opt_num(model::inherited_speed(a.v(0), &a.list(1)))),
        "track.trackLabel" => ok(json!(model::track_label(a.opt(0)))),
        // heightUtils
        "track.trackLength" => ok(number(h::track_length(a.v(0)))),
        "track.heightStations" => ok(json!(h::height_stations(a.f(0)).into_iter().map(number).collect::<Vec<_>>())),
        "track.pointAtStationUtm" => ok(h::point_at_station_utm(a.v(0), a.f(1), a.i(2)).map(|p| p.to_json()).unwrap_or(Value::Null)),
        "track.elementAtStation" => ok(h::element_at_station_json(&a.list(0), a.f(1))),
        "track.trackSamplePoints" => ok(h::track_sample_points(a.v(0), &a.list(1).iter().filter_map(Value::as_f64).collect::<Vec<_>>())),
        "track.heightAt" => ok(opt_num(h::height_at(&a.list(0), a.f(1)))),
        "track.pointGrades" => ok(h::point_grades(&a.list(0), a.i(1))),
        "track.roundHeight" => ok(number(h::round_height(a.f(0)))),
        "track.solveHeightPoint" => ok(h::solve_height_point(
            &a.list(0),
            a.u(1),
            a.v(2),
            a.option(3, "length").and_then(Value::as_f64).unwrap_or(f64::INFINITY),
            a.option(3, "minGap").and_then(Value::as_f64).unwrap_or(0.1),
        )),
        "track.insertHeightPoint" => ok(h::insert_height_point(&a.list(0), a.f(1), a.of(2).unwrap_or(0.1))),
        "track.splitHeights" => {
            let (x, y) = h::split_heights(a.opt(0).map(|_| a.0[0].as_array().map(Vec::as_slice).unwrap_or(&[])), a.f(1));
            ok(json!([opt_list(x), opt_list(y)]))
        }
        "track.truncateHeights" => ok(opt_list(h::truncate_heights(a.opt(0).and_then(Value::as_array).map(Vec::as_slice), a.f(1)))),
        "track.sliceHeights" => ok(opt_list(h::slice_heights(
            a.opt(0).and_then(Value::as_array).map(Vec::as_slice),
            a.f(1),
            a.f(2),
        ))),
        "track.reverseHeights" => ok(h::reverse_heights(a.opt(0), a.f(1)).unwrap_or(Value::Null)),
        "track.trackProfile" => ok(h::track_profile(a.v(0))),
        "track.adjacentTracks" => {
            let tracks = a.list(0);
            ok(Value::Array(
                h::adjacent_tracks(&tracks, &a.list(1), a.v(2), a.s(3))
                    .into_iter()
                    .map(|(t, e)| json!({ "track": t, "endpoint": e }))
                    .collect(),
            ))
        }
        "track.neighbourStub" => ok(h::neighbour_stub(&a.v(0)["track"], a.v(0)["endpoint"].as_str().unwrap_or(""))),
        "track.endOfIndex" => ok(opt_str(h::end_of_index(a.v(0), a.u(1)))),
        "track.jointGroup" => {
            let r = a.v(2);
            ok(Value::Array(
                h::joint_group(
                    &a.list(0),
                    &a.list(1),
                    r.get("trackId").unwrap_or(&Value::Null),
                    r["index"].as_u64().unwrap_or(0) as usize,
                )
                .into_iter()
                .map(|(t, i)| json!({ "trackId": t, "index": i }))
                .collect(),
            ))
        }
        "track.jointHeightUpdates" => ok(h::updates_json(h::joint_height_updates(&a.list(0), &a.list(1), &a.list(2)))),
        "track.gradientAt" => ok(opt_num(h::gradient_at(&a.list(0), a.f(1)))),
        "track.verticalCurves" => ok(h::vertical_curves(&a.list(0), a.opt(1).and_then(Value::as_u64).unwrap_or(24) as usize)),
        "track.verticalCurveOverlaps" => ok(h::vertical_curve_overlaps(&a.list(0))),
        "track.tangentLength" => ok(opt_num(h::tangent_length(&a.list(0), a.u(1)))),
        // platformUtils
        "track.elementStations" => {
            let els = rail_across_track::util::elements(a.v(0)).to_vec();
            ok(Value::Array(
                pf::element_stations(a.v(0))
                    .into_iter()
                    .map(|(i, s, e)| json!({ "el": els[i], "index": i, "start": number(s), "end": number(e) }))
                    .collect(),
            ))
        }
        "track.pointOnElement" => ok(pf::point_json(pf::point_on_element(a.v(0), a.i(1), a.f(2)))),
        "track.pointAtStation" => ok(pf::point_json(pf::point_at_station(a.v(0), a.f(1)))),
        "track.stationOnTrack" => ok(pf::station_on_track(a.v(0), a.utm(1))
            .map(|(s, d, b)| json!({ "station": number(s), "dist": number(d), "bearing": number(b) }))
            .unwrap_or(Value::Null)),
        "track.offsetEdgeCoords" => ok(opt_list(pf::offset_edge_coords(a.v(0), a.f(1), a.f(2), a.f(3)))),
        "track.edgeOffsets" => {
            let (f, b) = pf::edge_offsets(a.v(0));
            ok(json!({ "front": number(f), "back": number(b) }))
        }
        "track.platformEdgeElevation" => ok(opt_num(pf::platform_edge_elevation(a.v(0), a.v(1), a.f(2)))),
        "track.platformRing" => ok(opt_list(pf::platform_ring(a.v(0), a.v(1)))),
        "track.rebuildPlatformSymbol" => ok(pf::rebuild_platform_symbol(a.v(0), &a.list(1))),
        "track.stationFromClick" => ok(opt_num(pf::station_from_click(a.v(0), a.u(1), a.utm(2)))),
        "track.platformLength" => ok(number(pf::platform_length(a.v(0)))),
        "track.carryPlatforms" => ok(pf::carry_platforms(a.v(0), &a.list(1))),
        "track.flipPlatforms" => ok(pf::flip_platforms(a.v(0), a.v(1), a.f(2))),
        // topology, trackLinkUtils, kmLines
        "track.switchNodes" => ok(tp::switch_nodes(&a.list(0), &a.list(1))),
        "track.classifyTrackEnds" => ok(tp::classify_track_ends(&a.list(0), &a.list(1), &a.list(2))),
        "track.freeEnds" => ok(Value::Array(tp::free_ends(&a.list(0)))),
        "track.tracksOnFrom" => ok(pairs(tp::tracks_on_from(a.v(0), a.s(1), &a.list(2), &a.list(3)), "trackId")),
        "track.trackEndAt" => ok(tp::track_end_at(a.opt(0), a.s(1)).map(|e| e.to_json()).unwrap_or(Value::Null)),
        "track.findTrackJoints" => ok(tp::find_track_joints_json(&a.list(0), &a.list(1), a.v(2))),
        "track.existingLinks" => ok(tp::existing_links(&a.list(0), &a.list(1))),
        "track.linksForJoints" => ok(Value::Array(tp::links_for_joints(&a.list(0), &a.list(1), &a.strings(2)))),
        "track.linkAllJoints" => ok(tp::link_all_joints(&a.list(0), &a.list(1), &a.strings(2))),
        "track.jointFit" => {
            let (gap, off) = tp::joint_fit(&tp::End::from_json(a.v(0)), &tp::End::from_json(a.v(1)));
            ok(json!({ "gap": number(gap), "bearingOff": number(off) }))
        }
        "track.linkSymbol" => ok(tp::link_symbol(a.v(0), &a.list(1))),
        "track.claimedEnds" => {
            let mut v: Vec<String> = tp::claimed_ends(&a.list(0)).into_iter().collect();
            v.sort();
            ok(json!(v))
        }
        "track.kmOnRuns" => {
            let runs: Vec<Vec<[f64; 3]>> = a
                .list(0)
                .iter()
                .map(|r| {
                    r.as_array()
                        .into_iter()
                        .flatten()
                        .map(|p| {
                            [
                                p[0].as_f64().unwrap_or(f64::NAN),
                                p[1].as_f64().unwrap_or(f64::NAN),
                                p[2].as_f64().unwrap_or(f64::NAN),
                            ]
                        })
                        .collect()
                })
                .collect();
            ok(
                tp::km_on_runs(&runs, [a.v(1)[0].as_f64().unwrap_or(f64::NAN), a.v(1)[1].as_f64().unwrap_or(f64::NAN)])
                    .map(|(k, o)| json!({ "km": number(k), "offset": number(o) }))
                    .unwrap_or(Value::Null),
            )
        }
        "track.planeRuns" => ok(json!(
            tp::plane_runs(a.v(0), a.i(1))
                .into_iter()
                .map(|r| r.into_iter().map(|p| json!([number(p[0]), number(p[1]), number(p[2])])).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        )),
        "track.kmForTrackPoint" => ok(tp::km_for_track_point(
            &a.list(0),
            a.v(1),
            [a.v(2)[0].as_f64().unwrap_or(f64::NAN), a.v(2)[1].as_f64().unwrap_or(f64::NAN)],
        )),
        // mergeElements
        "track.mergeChain" => ok(merge::merge_chain(
            &a.list(0),
            a.i(1),
            a.option(2, "equal").is_some_and(|v| rail_across_js::truthy(Some(v))),
        )),
        "track.mergeableRun" => ok(json!(merge::mergeable_run(
            &a.list(0),
            a.i(1),
            a.option(2, "equal").is_some_and(|v| rail_across_js::truthy(Some(v)))
        ))),
        // trackSplitUtils
        "track.carveSwitchRoute" => ok(split::carve_switch_route(a.v(0), a.s(1), a.v(2), a.v(3), a.of(4)).unwrap_or(Value::Null)),
        "track.splitTrackAtJoint" => {
            let mut names = a.strings(3);
            ok(split::split_track_at_joint(a.v(0), a.u(1), a.f(2), &mut names))
        }
        "track.splitElementAt" => {
            let mut names = a.strings(4);
            ok(split::split_element_at(a.v(0), a.u(1), a.v(2), a.f(3), &mut names).unwrap_or(Value::Null))
        }
        // routes
        "track.resolveRoute" => ok(routes::resolve_route(a.v(0), &a.list(1), &a.list(2))),
        "track.partStation" => ok(number(routes::part_station(a.v(0), a.f(1)))),
        "track.routeAt" => ok(routes::route_at(a.v(0), a.f(1))),
        "track.routeStationOf" => ok(opt_num(routes::route_station_of(a.v(0), a.v(1), a.f(2)))),
        "track.routePointAt" => ok(routes::route_point_at(a.v(0), a.f(1))),
        "track.routePath" => {
            let exits: Vec<String> = a.strings(3);
            let exits: Vec<&str> = exits.iter().map(String::as_str).collect();
            ok(opt_list(routes::route_path(&a.list(0), &a.list(1), a.v(2), &exits, a.v(4))))
        }
        "track.extendRoute" => ok(routes::extend_route(&a.list(0), a.v(1), &a.list(2), &a.list(3))),
        "track.closeGaps" => ok(routes::close_gaps(&a.list(0), &a.list(1), &a.list(2))),
        "track.piecesAlong" => ok(Value::Array(routes::pieces_along(a.v(0), &a.list(1)))),
        "track.repairRoutes" => ok(routes::repair_routes(a.v(0), a.v(1))),
        "track.nextRouteName" => ok(json!(routes::next_route_name(&a.list(0), a.os(1).unwrap_or("Route")))),
        // trackGroups, platformForm
        "track.kmTrackName" => ok(json!(groups::km_track_name(
            &js_string(a.opt(0)),
            a.f(1),
            &a.strings(2).into_iter().collect::<HashSet<_>>()
        ))),
        "track.trackKind" => ok(opt_str(groups::track_kind(a.v(0)))),
        "track.trackTypeName" => ok(groups::track_type_name(a.v(0), a.v(1))),
        "track.trackGroupKey" => ok(opt_str(groups::track_group_key(a.v(0)))),
        "track.groupTracks" => ok(groups::group_tracks(&a.list(0))),
        "track.groupTitle" => ok(json!(groups::group_title(a.v(0)))),
        "track.trackListLabel" => ok(json!(groups::track_list_label(a.v(0)))),
        "track.assignmentFields" => ok(groups::assignment_fields(a.v(0))),
        "track.assignTracks" => ok(Value::Array(groups::assign_tracks(
            &a.list(0),
            &a.list(1),
            a.v(2),
            a.option(3, "names").unwrap_or(&Value::Null),
            a.option(3, "trackNumbers").unwrap_or(&Value::Null),
        ))),
        "track.stationTrackStem" => ok(json!(groups::station_track_stem(&a.opt(0).map(|v| js_string(Some(v))).unwrap_or_default()))),
        "track.plannedNames" => ok(groups::planned_names(
            &a.list(0),
            &a.list(1),
            a.v(2),
            a.option(3, "kmById").unwrap_or(&Value::Null),
            a.option(3, "numberById").unwrap_or(&Value::Null),
        )),
        "track.platformForm" => ok(groups::platform_form(a.v(0))),
        "track.pickedStation" => ok(json!(groups::picked_station(a.f(0)))),
        "track.platformDraft" => ok(groups::platform_draft(a.v(0))),
        "track.platformFormValid" => ok(json!(groups::platform_form_valid(a.v(0), a.opt(1)))),
        // alignmentBands, bufferStopGeometry
        "track.elementSpans" => {
            let els = a.list(0);
            ok(Value::Array(
                bands::element_spans(&els)
                    .into_iter()
                    .map(|(i, f, t)| json!({ "el": els[i], "elIdx": i, "from": number(f), "to": number(t) }))
                    .collect(),
            ))
        }
        "track.curvatureBand" => ok(Value::Array(bands::curvature_band(&a.list(0)))),
        "track.cantBand" => ok(Value::Array(bands::cant_band(&a.list(0)))),
        "track.speedBand" => ok(Value::Array(bands::speed_band(&a.list(0)))),
        "track.bandValueAt" => ok(opt_num(bands::band_value_at(&a.list(0), a.f(1)))),
        "track.bandRuns" => ok(Value::Array(bands::band_runs(&a.list(0), a.of(1).unwrap_or(1e-9)))),
        "track.bandRange" => ok(bands::band_range(&a.list(0)).map(|(x, y)| json!([number(x), number(y)])).unwrap_or(Value::Null)),
        "track.bandFindings" => ok(bands::band_findings(a.v(0))),
        "track.withBandValue" => ok(Value::Array(bands::with_band_value(
            &a.list(0),
            &a.list(1).iter().filter_map(Value::as_u64).map(|x| x as usize).collect::<Vec<_>>(),
            a.s(2),
            a.of(3),
        ))),
        "track.withCantEnds" => ok(Value::Array(bands::with_cant_ends(
            &a.list(0),
            &a.list(1).iter().filter_map(Value::as_u64).map(|x| x as usize).collect::<Vec<_>>(),
            a.v(2),
        ))),
        "track.bufferStopFeatures" => ok(Value::Array(bands::buffer_stop_features(&a.list(0), &a.list(1)))),
        "track.boundaryFeatures" => ok(Value::Array(bands::boundary_features(&a.list(0), &a.list(1)))),
        _ => None,
    }
}
