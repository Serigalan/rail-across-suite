// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch module's operations, named and called as the JavaScript's.

use rail_across_js::number;
use rail_across_switch::cants;
use rail_across_switch::catalogue as cat;
use rail_across_switch::connect as cn;
use rail_across_switch::connection as conn;
use rail_across_switch::crossing as cr;
use rail_across_switch::delete as del;
use rail_across_switch::detail;
use rail_across_switch::detail_scene;
use rail_across_switch::gradient as gr;
use rail_across_switch::mid_chain as mc;
use rail_across_switch::placement as pl;
use rail_across_switch::route as rt;
use rail_across_switch::rulebook;
use rail_across_switch::sleepers as sl;
use rail_across_switch::slider;
use rail_across_switch::symbol as sy;
use serde_json::{Value, json};

use crate::args::{Args, opt_num, opt_str};
use crate::ops::Failure;

fn forms(v: &[&Value]) -> Value {
    Value::Array(v.iter().map(|f| (*f).clone()).collect())
}

fn wgs(v: &Value) -> [f64; 2] {
    [
        v.get(0).and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ]
}

/// Runs switch operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    let s_opt = |i: usize| a.opt(i).and_then(Value::as_f64);
    match op {
        // catalogue
        "switch.SWITCH_TYPES" => ok(forms(&cat::SWITCH_TYPES)),
        "switch.SWITCH_TYPES_SPECIAL" => ok(forms(&cat::SWITCH_TYPES_SPECIAL)),
        "switch.SWITCH_PICK_TYPES" => ok(forms(&cat::SWITCH_PICK_TYPES)),
        "switch.ALL_SWITCH_TYPES" => ok(forms(&cat::ALL_SWITCH_TYPES)),
        "switch.CROSSING_TYPES" => ok(forms(&cat::CROSSING_TYPES)),
        "switch.DEFAULT_SWITCH_TYPE_IDX" => ok(json!(cat::default_switch_type_idx())),
        "switch.SWITCH_CONNECTION_STAGES" => ok(Value::Array(cat::SWITCH_CONNECTION_STAGES.iter().map(|s| forms(s)).collect())),
        "switch.switchTypeByLabel" => ok(cat::switch_type_by_label(a.os(0)).cloned().unwrap_or(Value::Null)),
        "switch.switchBranchSpeed" => ok(opt_num(cat::switch_branch_speed(a.v(0)))),
        "switch.switchArcLength" => ok(number(cat::switch_arc_length(a.f(0), a.f(1)))),
        "switch.switchStraightLength" => ok(number(cat::switch_straight_length(a.v(0)))),
        "switch.switchBranchSections" => ok(Value::Array(cat::switch_branch_sections(a.opt(0)).into_iter().map(|s| s.to_json()).collect())),
        "switch.hasCompoundBranch" => ok(json!(cat::has_compound_branch(a.opt(0)))),
        "switch.switchBranchPointAt" => {
            let (x, y) = cat::switch_branch_point_at(a.opt(0), a.f(1));
            ok(json!({ "x": number(x), "y": number(y) }))
        }
        "switch.switchTangents" => {
            let (t1, t2) = cat::switch_tangents(a.opt(0));
            ok(json!({ "t1": number(t1), "t2": number(t2) }))
        }
        "switch.switchBranchLength" => ok(number(cat::switch_branch_length(a.opt(0)))),
        "switch.switchMarkDistance" => ok(number(cat::switch_mark_distance(a.v(0)))),
        "switch.switchMark" => ok(opt_num(cat::switch_mark(a.opt(0)))),
        "switch.switchFormChain" => ok(Value::Array(cat::switch_form_chain(a.opt(0), a.s(1)))),
        "switch.asRadius" => ok(opt_num(cat::as_radius(s_opt(0)))),
        "switch.branchRadius" => ok(opt_num(cat::branch_radius(a.f(0), s_opt(1)))),
        "switch.bentForm" => ok(json!(cat::bent_form(s_opt(0), s_opt(1)))),
        "switch.switchVariants" => ok(Value::Array(cat::switch_variants(a.opt(0)))),
        "switch.switchVariantFor" => ok(cat::switch_variant_for(a.opt(0), a.opt(1)).unwrap_or(Value::Null)),
        // weichenRegelwerk
        "switch.WEICHEN_REGELWERK" => ok(rulebook::switch_rulebook()),
        "switch.switchGroups" => ok(Value::Array(rulebook::switch_groups())),
        // route
        "switch.toRoute" => ok(rt::to_route(a.v(0)).to_json()),
        "switch.toChain" => ok(rt::chain_json(&rt::to_chain(a.v(0)))),
        "switch.switchRouteRadiusAt" => ok(opt_num(rt::switch_route_radius_at(&rt::to_route(a.v(0)), a.f(1)))),
        "switch.switchRoutePointUtm" => {
            let r = rt::to_route(a.v(2));
            ok(rt::switch_route_point_utm(a.utm(0), a.f(1), &r, s_opt(3).unwrap_or(r.length)).to_json())
        }
        "switch.switchRouteBearingAt" => {
            let r = rt::to_route(a.v(1));
            ok(number(rt::switch_route_bearing_at(a.f(0), &r, s_opt(2).unwrap_or(r.length))))
        }
        "switch.switchRouteSlice" => ok(rt::switch_route_slice(&rt::to_route(a.v(0)), a.f(1), a.f(2)).to_json()),
        "switch.switchBranchRoute" => ok(rt::switch_branch_route(s_opt(0), &rt::to_route(a.v(1)), a.f(2)).to_json()),
        "switch.switchStemRoute" => ok(rt::switch_stem_route(a.v(0), a.f(1)).to_json()),
        "switch.switchElementRoute" => ok(rt::switch_element_route(a.v(0)).to_json()),
        "switch.switchChainTo" => ok(rt::chain_json(&rt::switch_chain_to(&rt::to_chain(a.v(0)), a.f(1)))),
        "switch.switchChainSegmentsUtm" => ok(Value::Array(
            rt::switch_chain_segments_utm(a.utm(0), a.f(1), &rt::to_chain(a.v(2)))
                .into_iter()
                .map(|s| s.to_json())
                .collect(),
        )),
        "switch.switchChainPointUtm" => ok(rt::switch_chain_point_utm(a.utm(0), a.f(1), &rt::to_chain(a.v(2)), s_opt(3))
            .map(|u| u.to_json())
            .unwrap_or(Value::Null)),
        "switch.switchChainBearingAt" => ok(number(rt::switch_chain_bearing_at(a.f(0), &rt::to_chain(a.v(1)), s_opt(2)))),
        "switch.switchChainSlice" => ok(rt::chain_json(&rt::switch_chain_slice(&rt::to_chain(a.v(0)), a.f(1), a.f(2)))),
        "switch.switchBranchChain" => ok(rt::chain_json(&rt::switch_branch_chain(&rt::form_chain(a.v(0)), &rt::to_chain(a.v(1))))),
        "switch.switchChainJoints" => ok(Value::Array(rt::switch_chain_joints(&rt::to_chain(a.v(0))).into_iter().map(number).collect())),
        "switch.switchAbreast" => {
            let joints: Vec<f64> = a.list(1).iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect();
            ok(number(rt::switch_abreast(a.f(0), &joints, a.f(2), a.f(3))))
        }
        "switch.switchChainBentForm" => ok(json!(rt::switch_chain_bent_form(&rt::to_chain(a.v(0)), &rt::to_chain(a.v(1)), a.b(2)))),
        "switch.switchRouteSegments" => {
            let r1 = s_opt(1);
            let r2 = if a.0.len() > 2 { s_opt(2) } else { r1 };
            ok(json!(rt::switch_route_segments(a.f(0), r1, r2)))
        }
        "switch.clothoidRoutePoints" => ok(rt::pts_json(&rt::clothoid_route_points(a.utm(0), a.f(1), &rt::to_route(a.v(2))))),
        "switch.utmEndRoute" => ok(rt::utm_end_route(a.utm(0), a.f(1), a.f(2), s_opt(3)).to_json()),
        "switch.routeCoords" => ok(rt::pts_json(&rt::route_coords(
            a.utm(0),
            wgs(a.v(1)),
            a.f(2),
            &rt::to_route(a.v(3)),
            wgs(a.v(4)),
        ))),
        "switch.chainGeometry" => {
            let (coords, segs) = rt::chain_geometry(a.utm(0), wgs(a.v(1)), a.f(2), &rt::to_chain(a.v(3)), wgs(a.v(4)));
            let segments: Vec<Value> = segs
                .into_iter()
                .map(|(s, c)| {
                    let mut o = s.to_json();
                    o["coords"] = rt::pts_json(&c);
                    o
                })
                .collect();
            ok(json!({ "coords": rt::pts_json(&coords), "segments": segments }))
        }
        // crossing
        "switch.crossingAngle" => ok(number(cr::crossing_angle(a.v(0)))),
        "switch.crossingLegRadius" => ok(opt_num(cr::crossing_leg_radius(a.opt(0)))),
        "switch.crossingLegSignedRadius" => ok(opt_num(cr::crossing_leg_signed_radius(a.opt(0), a.f(1)))),
        "switch.crossingEndDistance" => ok(number(cr::crossing_end_distance(a.v(0)))),
        "switch.crossingWedgeRing" => ok(rt::pts_json(&cr::crossing_wedge_ring(&pts(a.v(0)), &pts(a.v(1))))),
        "switch.crossingLegFitsTrack" | "switch.piecesOnRadius" => ok(json!(cr::pieces_on_radius(&rt::to_chain(&Value::Array(a.list(0))), s_opt(1)))),
        "switch.computeCrossingGeometryUtm" => ok(cr::compute_crossing_geometry_utm(
            a.utm(0),
            a.f(1),
            a.v(2),
            a.f(3),
            a.opt(4).map(wgs),
            a.opt(5).map(wgs),
            a.opt(6).map(|v| rail_across_js::js_number(Some(v)) as i64),
        )),
        "switch.computeCrossingGeometryFromPortA" => ok(cr::compute_crossing_geometry_from_port_a(a.utm(0), wgs(a.v(1)), a.f(2), a.v(3), a.f(4))),
        "switch.crossingElements" => ok(cr::crossing_elements(a.v(0), a.v(1))),
        // symbol
        "switch.switchRoutePointsUtm" => {
            // A single route is read as it is given: its r1 and r2, not a `radius`.
            let route = a.v(2);
            let chain = if route.is_array() {
                rt::to_chain(route)
            } else {
                vec![rt::Route {
                    length: a.v(2)["length"].as_f64().unwrap_or(f64::NAN),
                    r1: route.get("r1").and_then(Value::as_f64),
                    r2: route.get("r2").and_then(Value::as_f64),
                }]
            };
            ok(rt::pts_json(&sy::switch_route_points_utm(a.utm(0), a.f(1), &chain, !route.is_array())))
        }
        "switch.switchLabelGeometry" => {
            let (label, centre) = sy::switch_label_geometry(a.utm(0), a.f(1), &rt::to_chain(a.v(2)), &rt::to_chain(a.v(3)), a.i(4));
            ok(json!({ "labelCoords": rt::pts_json(&label), "bodyCentre": [number(centre[0]), number(centre[1])] }))
        }
        "switch.switchFillRing" => ok(rt::pts_json(&sy::switch_fill_ring(&pts(a.v(0)), &pts(a.v(1))))),
        "switch.lcsLineUtm" => ok(sy::lcs_line_utm(
            wgs(a.v(0)),
            wgs(a.v(1)),
            wgs(a.v(2)),
            a.opt(3)
                .map(|v| rail_across_js::js_number(Some(v)))
                .unwrap_or(if a.0.len() > 3 { 0.0 } else { f64::NAN }),
        )
        .map(|l| rt::pts_json(&l))
        .unwrap_or(Value::Null)),
        "switch.lcsLine" => ok(sy::lcs_line(
            wgs(a.v(0)),
            wgs(a.v(1)),
            wgs(a.v(2)),
            a.opt(3).map(|v| rail_across_js::js_number(Some(v))),
            a.i(4),
        )
        .map(|l| rt::pts_json(&l))
        .unwrap_or(Value::Null)),
        "switch.crossingRoutesFromTracks" => ok(sy::crossing_routes_from_tracks(a.v(0), &by_id(a.v(1)))
            .map(|r| r.to_json())
            .unwrap_or(Value::Null)),
        "switch.crossingBodyUtm" => {
            let r = sy::crossing_routes_from_tracks(a.v(0), &by_id(a.v(1)));
            ok(r.map(|r| Value::Array(sy::crossing_body_utm(&r).iter().map(|x| rt::pts_json(x)).collect()))
                .unwrap_or(Value::Null))
        }
        "switch.switchRoutesFromTracks" => ok(sy::switch_routes_from_tracks(a.v(0), &by_id(a.v(1)))
            .map(|r| r.to_json())
            .unwrap_or(Value::Null)),
        "switch.rebuildSwitchSymbol" => ok(sy::rebuild_switch_symbol(a.v(0), &by_id(a.v(1)))),
        "switch.computeSwitchGeometryUtm" => ok(sy::compute_switch_geometry_utm(
            a.utm(0),
            a.f(1),
            a.v(2),
            a.s(3),
            a.b(4),
            a.opt(5).map(wgs),
            &sy::MainR::from_json(a.opt(6)),
        )),
        // sleepers
        "switch.switchLds" => with_forms(a.option(1, "forms"), |f| ok(opt_num(sl::switch_lds(a.v(0), f)))),
        "switch.mainRouteLength" => ok(number(sl::main_route_length(a.v(0), a.v(1)))),
        "switch.turnoutSleepers" => with_forms(a.option(2, "forms"), |f| {
            let o = sl::SleeperOptions {
                form_of: f,
                rail: a.option(2, "rail").cloned(),
                lds_unknown: a.option(2, "ldsUnknown").and_then(Value::as_f64),
            };
            ok(sl::turnout_sleepers(&a.list(0), a.v(1), &o).map(|fr| fr.to_json()).unwrap_or(Value::Null))
        }),
        "switch.sleeperGrid" => {
            let count = a.opt(2).and_then(Value::as_f64).map(|c| c as usize);
            ok(Value::Array(sl::sleeper_grid(a.f(0), a.f(1), count).into_iter().map(number).collect()))
        }
        "switch.sleepersWeToLds" => ok(Value::Array(sl::sleepers_we_to_lds(a.f(0)).into_iter().map(number).collect())),
        "switch.tableLds" => ok(number(sl::table_lds(a.opt(0), a.f(1)))),
        "switch.sleeperNear" | "switch.sleeperThrough" | "switch.sleeperOffsets" => with_forms(a.option(4, "forms"), |f| {
            let o = sl::SleeperOptions {
                form_of: f,
                rail: None,
                lds_unknown: a.option(4, "ldsUnknown").and_then(Value::as_f64),
            };
            let Some(frame) = sl::turnout_sleepers(&a.list(0), a.v(1), &o) else {
                return ok(Value::Null);
            };
            let (side, station) = (a.s(2), a.f(3));
            ok(match op {
                "switch.sleeperNear" => sl::sleeper_near(&frame, side, station, 0.01).map(|s| s.to_json()).unwrap_or(Value::Null),
                "switch.sleeperThrough" => sl::sleeper_through(&frame, side, station)
                    .map(|(p, m, b)| {
                        let mut o = p.to_json();
                        o["main"] = number(m);
                        o["branch"] = number(b);
                        o
                    })
                    .unwrap_or(Value::Null),
                _ => {
                    let at = |s: &str| sl::sleeper_through(&frame, s, station);
                    at(side)
                        .and_then(|(_, m, b)| sl::sleeper_offsets(&frame.main, &frame.branch, m, b))
                        .map(|(x, y)| json!({ "a": number(x), "y": number(y) }))
                        .unwrap_or(Value::Null)
                }
            })
        }),
        // switchGradient
        "switch.roundMm" => ok(number(gr::round_mm(a.f(0)))),
        "switch.STATION_TOL" => ok(number(gr::STATION_TOL)),
        "switch.Z_TOL" => ok(number(gr::Z_TOL)),
        "switch.ldsFromToe" => with_forms(a.option(2, "forms"), |f| ok(opt_num(gr::lds_from_toe(&a.list(0), a.v(1), &opts(f))))),
        // `slim`: without the sleepers, the samples and the grid — what the heights read of a coupling.
        "switch.switchCouplings" => with_forms(a.option(2, "forms"), |f| {
            let slim = rail_across_js::truthy(a.option(2, "slim"));
            ok(Value::Array(
                gr::switch_couplings(&a.list(0), &a.list(1), &opts(f))
                    .iter()
                    .map(|c| {
                        let mut j = c.to_json();
                        if slim && let Some(o) = j.as_object_mut() {
                            for k in ["sleepers", "samples", "grid"] {
                                o.shift_remove(k);
                            }
                        }
                        j
                    })
                    .collect(),
            ))
        }),
        "switch.switchCoupling" => with_forms(a.option(2, "forms"), |f| {
            ok(gr::switch_coupling(&a.list(0), a.v(1), &opts(f)).map(|c| c.to_json()).unwrap_or(Value::Null))
        }),
        "switch.couplingOf" => with_forms(a.option(3, "forms"), |f| {
            ok(gr::coupling_of(&a.list(0), &a.list(1), a.v(2), &opts(f))
                .map(|c| c.to_json())
                .unwrap_or(Value::Null))
        }),
        // On a coupling as switchCouplings answers it: (coupling, mainHeights).
        "switch.ldsLift" if a.v(0).get("ldsSleeper").is_some() => ok(opt_num(gr::lds_lift_json(a.v(0), &a.list(1)))),
        "switch.ldsLift" => with_forms(a.option(4, "forms"), |f| {
            let c = gr::coupling_of(&a.list(0), &a.list(1), a.v(2), &opts(f));
            ok(c.map(|c| number(gr::lds_lift(&c, &a.list(3)))).unwrap_or(Value::Null))
        }),
        "switch.planeFrom" => ok(opt_num(gr::plane_from(a.v(0), a.f(1), s_opt(2), a.v(3), a.f(4)))),
        "switch.branchPlaneHeight" => with_forms(a.option(4, "forms"), |f| {
            let c = gr::coupling_of(&a.list(0), &a.list(1), a.v(2), &opts(f));
            ok(c.and_then(|c| gr::branch_plane_height(&c, a.f(3))).map(number).unwrap_or(Value::Null))
        }),
        "switch.trackHeightAt" => with_forms(a.option(4, "forms"), |f| {
            ok(opt_num(gr::track_height_at(&a.list(0), &a.list(1), a.v(2), a.f(3), &opts(f))))
        }),
        "switch.heightContext" => with_forms(a.option(3, "forms"), |f| ok(gr::height_context(&a.list(0), &a.list(1), a.v(2), &opts(f)))),
        "switch.switchBodySpans" => with_forms(a.option(3, "forms"), |f| {
            ok(Value::Array(gr::switch_body_spans(&a.list(0), &a.list(1), a.v(2), &opts(f))))
        }),
        // switchCants
        // Only the tracks the coupling changed, of the tracks after the write: `[before, after, switches,
        // changed?]` — `changed` the ids of the tracks the write changed, where the caller knows them.
        "switch.coupleSwitchCantsChanged" => {
            let after = a.list(1);
            if after.is_empty() || a.list(2).is_empty() {
                return ok(json!([]));
            }
            let hint: Option<std::collections::HashSet<String>> = a.opt(3).map(|_| a.strings(3).into_iter().collect());
            let coupled = cants::couple_switch_cants_from(&a.list(0), &after, &a.list(2), hint.as_ref());
            ok(Value::Array(
                coupled.into_iter().zip(after.iter()).filter(|(c, t)| c != *t).map(|(c, _)| c).collect(),
            ))
        }
        "switch.coupleSwitchCants" => {
            if a.opt(1).is_none_or(|v| v.as_array().is_none_or(Vec::is_empty)) {
                return ok(a.v(1).clone());
            }
            ok(Value::Array(cants::couple_switch_cants(&a.list(0), &a.list(1), &a.list(2))))
        }
        // stationSlider
        "switch.nearestValid" => ok(opt_num(slider::nearest_valid(a.f(0), &intervals(a.v(1))))),
        "switch.settleStation" => {
            let snaps: Vec<f64> = a
                .option(1, "snaps")
                .and_then(Value::as_array)
                .map(|v| v.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let tol = a.option(1, "tol").and_then(Value::as_f64).unwrap_or(1.0);
            ok(opt_num(slider::settle_station(
                a.f(0),
                &intervals(a.option(1, "intervals").unwrap_or(&Value::Null)),
                &snaps,
                tol,
            )))
        }
        "switch.uniqueStations" => {
            let list: Vec<f64> = a.list(0).iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect();
            ok(Value::Array(slider::unique_stations(&list).into_iter().map(number).collect()))
        }
        // midChain
        "switch.DEFAULT_MIDS" => ok(Value::Array(mc::DEFAULT_MIDS.iter().map(|m| m.to_json()).collect())),
        "switch.MID_OPEN" => ok(json!(mc::MID_OPEN)),
        "switch.isDefaultMids" => ok(json!(mc::is_default_mids(&mc::mids_from(a.v(0))))),
        "switch.midUnknowns" => ok(Value::Array(
            mc::mid_unknowns(&mc::mids_from(a.v(0)))
                .into_iter()
                .map(|u| json!({ "i": u.i, "field": if u.length { "length" } else { "k" } }))
                .collect(),
        )),
        "switch.midSignSets" => ok(json!(
            mc::mid_sign_sets(&mc::mids_from(a.v(0)), a.of(1).unwrap_or(1.0))
                .into_iter()
                .map(|s| s.into_iter().map(number).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        )),
        "switch.endCurvature" => ok(number(mc::end_curvature(&rt::to_chain(a.v(0))))),
        // switchDelete
        "switch.switchParts" => ok(del::parts_json(&del::switch_parts(a.v(0), &del::by_id(&a.list(1))))),
        // On the ports as switchParts gives them by port — `{ A: { occupied }, … }` — or on the tracks.
        "switch.keptRoutes" if a.v(1).is_object() => {
            const PORTS: [&str; 6] = ["A", "B1", "B2", "B", "C", "D"];
            let parts: Vec<del::Port> = PORTS
                .iter()
                .filter_map(|p| a.v(1).get(*p).map(|x| (p, x)))
                .map(|(p, x)| del::Port {
                    port: p,
                    route: None,
                    track_id: None,
                    endpoint: None,
                    track: None,
                    mine: Vec::new(),
                    beyond: Vec::new(),
                    occupied: rail_across_js::truthy(x.get("occupied")),
                })
                .collect();
            ok(json!(del::kept_routes(a.v(0), &parts)))
        }
        "switch.keptRoutes" => ok(json!(del::kept_routes(a.v(0), &del::switch_parts(a.v(0), &del::by_id(&a.list(1)))))),
        "switch.planSwitchDeletion" => ok(del::plan_switch_deletion(a.v(0), &a.list(1))),
        // switchPlacement
        "switch.turnoutRunsOn" => ok(json!(pl::turnout_runs_on(&a.list(0), a.u(1), a.s(2), a.i(3)))),
        "switch.clickStation" => ok(number(pl::click_station(a.v(0), a.u(1), a.utm(2)))),
        "switch.placeSwitchOnTrack" => ok(match pl::place_switch_on_track(a.v(0), a.f(1), a.b(2), a.f(3)) {
            Ok(p) => p.to_json(),
            Err(e) => json!({ "error": e }),
        }),
        "switch.placementCantAt" => ok(match pl::place_switch_on_track(a.v(0), a.f(1), a.b(2), a.f(3)) {
            Ok(p) => number(
                p.cant_at(
                    a.f(4),
                    a.opt(5)
                        .map(|o| (o["s0"].as_f64().unwrap_or(f64::NAN), o["length"].as_f64().unwrap_or(f64::NAN))),
                ),
            ),
            Err(_) => Value::Null,
        }),
        "switch.switchEndAnchorRefusal" => ok(opt_str(pl::switch_end_anchor_refusal(a.v(0), a.u(1)))),
        // connectToTrack
        "switch.connectionAnchor" => {
            let tracks = a.option(2, "tracks").and_then(Value::as_array).map(Vec::as_slice);
            let list = |k: &str| a.option(2, k).and_then(Value::as_array).cloned().unwrap_or_default();
            let count = a.option(2, "count").and_then(Value::as_u64).unwrap_or(1) as usize;
            ok(
                match cn::connection_anchor(a.v(0), a.s(1), tracks, &list("switches"), &list("endMarks"), count) {
                    Ok(an) => json!({ "anchor": an.to_json() }),
                    Err(e) => json!({ "error": e }),
                },
            )
        }
        "switch.fitConnectingElement" => ok(
            cn::fit_connecting_element(&cn::Anchor::from_json(a.v(0)), a.utm(1), a.f(2), a.of(3).unwrap_or(0.0))
                .map(|f| f.to_json())
                .unwrap_or(Value::Null),
        ),
        "switch.END_CHAIN_OPEN" => ok(json!(cn::END_CHAIN_OPEN)),
        "switch.endChainDefault" => ok(Value::Array(
            cn::end_chain_default(&cn::Anchor::from_json(a.v(0))).into_iter().map(|m| m.to_json()).collect(),
        )),
        "switch.endChainOpen" => ok(json!(cn::end_chain_open(&mc::mids_from(a.v(0))))),
        "switch.refitTrackEnd" => ok(cn::refit_track_end(
            a.v(0),
            &cn::Anchor::from_json(a.v(1)),
            &cn::Fit::from_json(a.v(2)),
            a.utm(3),
            a.option(4, "speedLimit").and_then(Value::as_f64),
        )),
        "switch.refitCount" => ok(json!(cn::refit_count(&cn::Fit::from_json(a.v(0))))),
        "switch.joinRouteIntoTrack" => {
            let (track, shift) = cn::join_route_into_track(a.v(0), &cn::Anchor::from_json(a.v(1)), a.v(2));
            ok(json!({ "track": track, "shift": number(shift) }))
        }
        // switchConnectionUtils
        "switch.formPair" => {
            let (x, y) = conn::form_pair(a.v(0));
            ok(json!([x, y]))
        }
        "switch.CONNECTION_SPEEDS" => ok(Value::Array(conn::connection_speeds().into_iter().map(number).collect())),
        "switch.orientStemToward" => {
            // The stem as it came, its own fields kept, turned where it runs away from the other pick.
            let turned = conn::orient_stem_toward(&conn::Stem::from_json(a.v(0)), &conn::Stem::from_json(a.v(1)));
            let mut out = a.v(0).clone();
            let dir = a.v(0).get("dir").and_then(Value::as_f64).unwrap_or(1.0);
            if turned.dir != dir {
                out["dir"] = number(turned.dir);
            }
            ok(out)
        }
        "switch.checkMidRun" => ok(conn::check_mid_run(&rt::to_chain(a.v(0)), &rt::to_chain(a.v(1)), &rt::to_chain(a.v(2)), a.f(3)).to_json()),
        "switch.connectionBranchEnds" => ok(conn::connection_branch_ends(
            &conn::Stem::from_json(a.v(0)),
            &conn::Stem::from_json(a.v(1)),
            a.f(2),
            a.option(3, "step").and_then(Value::as_f64).unwrap_or(0.5),
            a.option(3, "reach").and_then(Value::as_f64).unwrap_or(300.0),
            a.option(3, "form"),
        )),
        "switch.computeSwitchConnections" => {
            let mids = a.opt(3).map(mc::mids_from);
            ok(Value::Array(conn::compute_switch_connections(
                &conn::Stem::from_json(a.v(0)),
                &conn::Stem::from_json(a.v(1)),
                a.of(2).unwrap_or(0.0),
                mids.as_deref(),
                seed_of(a.opt(4)),
            )))
        }
        "switch.solveSwitchConnection" => {
            let mids = a.opt(4).map(mc::mids_from);
            ok(conn::solve_switch_connection(
                &conn::Stem::from_json(a.v(0)),
                &conn::Stem::from_json(a.v(1)),
                a.f(2),
                a.of(3).unwrap_or(0.0),
                mids.as_deref(),
                seed_of(a.opt(5)),
                a.opt(6),
            ))
        }
        "switch.buildConnectionElements" => ok(conn::build_connection_elements(a.v(0), a.f(1))),
        // detail
        "switch.RAIL_OFFSET" => ok(number(detail::RAIL_OFFSET)),
        "switch.SLEEPER_OVERHANG" => ok(number(detail::SLEEPER_OVERHANG)),
        "switch.switchDetail" => with_forms(a.option(2, "forms"), |f| {
            ok(detail::switch_detail(&a.list(0), a.v(1), f, a.option(2, "rail").cloned()).unwrap_or(Value::Null))
        }),
        "switch.formDetail" => ok(detail::form_detail(
            a.opt(0),
            a.option(1, "rail").cloned(),
            a.option(1, "side").and_then(Value::as_str).unwrap_or("left"),
        )
        .unwrap_or(Value::Null)),
        "switch.connectionPartners" => ok(Value::Array(
            detail_scene::connection_partners(&a.list(0), &a.list(1), a.v(2)).into_iter().cloned().collect(),
        )),
        "switch.switchDetailScene" => {
            ok(detail_scene::switch_detail_scene(&a.list(0), &a.list(1), a.v(2), a.option(3, "margin").and_then(Value::as_f64)).unwrap_or(Value::Null))
        }
        _ => None,
    }
}

fn seed_of(v: Option<&Value>) -> Option<(f64, Vec<f64>)> {
    let v = v?;
    Some((
        v.get("ds")?.as_f64()?,
        v.get("values")?.as_array()?.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect(),
    ))
}

fn opts(f: sl::FormOf) -> sl::SleeperOptions {
    sl::SleeperOptions {
        form_of: f,
        rail: None,
        lds_unknown: None,
    }
}

/// Runs `then` with the forms looked up in `forms` (label → form) where given, else in the catalogue.
fn with_forms<T>(forms: Option<&Value>, then: impl FnOnce(sl::FormOf) -> T) -> T {
    match forms.and_then(Value::as_object) {
        Some(table) => {
            let table = table.clone();
            let f = move |label: Option<&str>| label.and_then(|l| table.get(l)).filter(|v| !v.is_null()).cloned();
            then(&f)
        }
        None => then(&sl::catalogue_form),
    }
}

fn intervals(v: &Value) -> Vec<(f64, f64)> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| (p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)))
                .collect()
        })
        .unwrap_or_default()
}

fn pts(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().map(|x| x.iter().map(wgs).collect()).unwrap_or_default()
}

/// `trackById` as an object; a list is keyed by its tracks' ids.
fn by_id(v: &Value) -> serde_json::Map<String, Value> {
    match v {
        Value::Object(o) => o.clone(),
        Value::Array(a) => a.iter().map(|t| (rail_across_js::js_string(t.get("id")), t.clone())).collect(),
        _ => serde_json::Map::new(),
    }
}
