// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch module's operations, named and called as the JavaScript's.

use rail_across_js::number;
use rail_across_switch::catalogue as cat;
use rail_across_switch::regelwerk;
use rail_across_switch::route as rt;
use serde_json::{Value, json};

use crate::args::{Args, opt_num};
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
        "switch.bauform" => ok(json!(cat::bauform(s_opt(0), s_opt(1)))),
        "switch.switchVariants" => ok(Value::Array(cat::switch_variants(a.opt(0)))),
        "switch.switchVariantFor" => ok(cat::switch_variant_for(a.opt(0), a.opt(1)).unwrap_or(Value::Null)),
        // weichenRegelwerk
        "switch.WEICHEN_REGELWERK" => ok(regelwerk::weichen_regelwerk()),
        "switch.weichenGruppen" => ok(Value::Array(regelwerk::weichen_gruppen())),
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
        "switch.switchChainBauform" => ok(json!(rt::switch_chain_bauform(&rt::to_chain(a.v(0)), &rt::to_chain(a.v(1)), a.b(2)))),
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
        _ => None,
    }
}
