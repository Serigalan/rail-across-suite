// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The operations of the cross section, the topology diagram and the
//! project's validation, named and called as the JavaScript's.

use rail_across_js::{js_number, number};
use rail_across_project::validate;
use rail_across_track::cross_section as cs;
use rail_across_track::diagram::{labels, topology_graph as tg, topology_lanes};
use serde_json::{Value, json};

use crate::args::{Args, opt_num};
use crate::ops::Failure;

fn utm_bearing(p: Option<(rail_across_geom::element::Utm, f64)>) -> Value {
    p.map(|(u, b)| json!({ "utm": u.to_json(), "bearing": number(b) })).unwrap_or(Value::Null)
}

/// Runs operation `op`, where it is one of these.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    let opt_f = |i: usize, key: &str, d: f64| a.option(i, key).map(|v| js_number(Some(v))).unwrap_or(d);
    match op {
        // crossSection
        "crossSection.RAILS" => ok(cs::rails_json()),
        "crossSection.SLEEPERS" => ok(cs::sleepers_json()),
        "crossSection.LICHTRAUM_SOURCE" => ok(json!(cs::lichtraum_source())),
        "crossSection.GAUGE_PROFILES" => ok(cs::gauge_profiles()),
        "crossSection.PLANUM_EDGE" => ok(number(cs::planum_edge())),
        "crossSection.gaugeProfile" => ok(cs::gauge_profile(a.os(0))),
        "crossSection.gaugeProfileLabelKey" => ok(json!(cs::gauge_profile_label_key(&rail_across_js::js_string(a.opt(0))))),
        "crossSection.gaugeProfileRing" => ok(Value::Array(cs::gauge_profile_ring(&a.list(0)))),
        "crossSection.gaugeProfileAreas" => ok(Value::Array(cs::gauge_profile_areas(&a.list(0)))),
        "crossSection.elementStartStation" => ok(number(cs::element_start_station(a.v(0), a.u(1)))),
        "crossSection.superstructureAt" => ok(cs::superstructure_at(a.v(0), a.f(1))),
        "crossSection.cantAngle" => ok(number(cs::cant_angle(a.of(0).unwrap_or(0.0)))),
        "crossSection.cantPivot" => ok(cs::pts(&[cs::cant_pivot(a.of(0).unwrap_or(0.0))])[0].clone()),
        "crossSection.rotatePoint" => {
            let p = cs::read_pts(&json!([a.v(0)]))[0];
            ok(cs::pts(&[cs::rotate_point(p, a.f(1))])[0].clone())
        }
        "crossSection.fitSection" => ok(cs::fit_section(
            &cs::read_pts(a.v(0)),
            opt_f(1, "w", f64::NAN),
            opt_f(1, "h", f64::NAN),
            a.of(2).unwrap_or(0.0),
        )),
        "crossSection.sectionAtStation" => ok(cs::section_at_station(a.v(0), a.f(1))),
        "crossSection.crossSection" => ok(cs::cross_section(a.v(0))),
        "crossSection.placeSection" => ok(cs::pts(&cs::place_section(
            &cs::read_pts(a.v(0)),
            opt_f(1, "offset", 0.0),
            opt_f(1, "dz", 0.0),
            rail_across_js::truthy(a.option(1, "mirrored")),
        ))),
        "crossSection.sectionNeighbours" => ok(Value::Array(cs::section_neighbours(a.v(0), a.f(1), &a.list(2), a.f(3)))),
        "crossSection.terrainHeightAt" => ok(opt_num(cs::terrain_height_at(&a.list(0), a.f(1)))),
        "crossSection.sectionLevels" => ok(cs::section_levels(&a.list(0), &a.list(1))),
        "crossSection.sectionOrigin" => ok(utm_bearing(cs::section_origin(a.v(0), a.f(1)))),
        "crossSection.sectionLinePoints" => ok(Value::Array(cs::section_line_points(a.v(0), a.f(1), a.f(2), a.f(3), a.of(4).unwrap_or(1.0)))),
        "crossSection.platformSection" => ok(cs::pts(&cs::platform_section(
            a.v(0),
            a.option(1, "rail").and_then(Value::as_str),
            a.option(1, "sleeper").and_then(Value::as_str),
        ))),
        // topology
        "topology.labelBox" => {
            let (w, h) = labels::label_box(&rail_across_js::js_string(a.opt(0)));
            ok(json!({ "w": number(w), "h": number(h) }))
        }
        "topology.segmentHitsBox" => {
            let s = a.list(0).iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect::<Vec<_>>();
            let seg = [0, 1, 2, 3].map(|i| s.get(i).copied().unwrap_or(f64::NAN));
            ok(json!(labels::segment_hits_box(seg, labels::Box2::from_json(a.v(1)))))
        }
        "topology.placeLabels" => {
            let segs: Vec<[f64; 4]> = a
                .list(1)
                .iter()
                .map(|s| [0, 1, 2, 3].map(|i| s.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN)))
                .collect();
            let boxes: Vec<labels::Box2> = a.list(2).iter().map(labels::Box2::from_json).collect();
            ok(Value::Object(labels::place_labels(&a.list(0), &segs, &boxes)))
        }
        "topology.buildTopologyGraph" => {
            let (nodes, edges) = tg::build_topology_graph(&a.list(0), &a.list(1), &a.list(2));
            ok(json!({ "nodes": nodes, "edges": edges }))
        }
        "topology.topologyClusters" => {
            let g = a.v(0);
            let nodes = g.get("nodes").and_then(Value::as_object).cloned().unwrap_or_default();
            let edges = g.get("edges").and_then(Value::as_array).cloned().unwrap_or_default();
            ok(tg::topology_clusters(&nodes, &edges))
        }
        "topology.selectionHighlight" => ok(tg::selection_highlight(a.opt(0), &a.list(1))),
        "topology.switchTrackIds" => ok(Value::Array(tg::switch_track_ids(a.v(0)))),
        "topology.layoutClusterLanes" => ok(topology_lanes::layout_cluster_lanes(a.v(0))),
        // validateProject
        "project.validateProject" => ok(validate::validate_project(a.v(0))),
        "project.hydrateProjects" => ok(Value::Array(a.list(0).iter().map(crate::hydrate::hydrate_project).collect())),
        "project.newFindings" => ok(Value::Array(validate::new_findings(&a.list(0), &a.list(1)))),
        "project.overlappingTracks" => ok(Value::Array(validate::overlapping_tracks(&a.list(0), &a.list(1)))),
        _ => None,
    }
}
