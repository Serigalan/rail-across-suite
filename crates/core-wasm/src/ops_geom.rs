// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The geometry module's operations, named as the JavaScript names them.

use rail_across_geom::element::{self, Utm};
use rail_across_geom::{cant, chain, coords, factory, parallel, plane_transform, reconstruct, transition};
use rail_across_js::{Object, get, number, truthy};
use serde_json::{Value, json};

use crate::ops::Failure;

fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}
fn of(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}
fn code(v: &Value, k: &str) -> i64 {
    v.get(k)
        .and_then(|c| c.as_f64().or_else(|| c.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(f64::NAN) as i64
}
fn utm(v: &Value, k: &str) -> Utm {
    Utm::from_json(v.get(k).unwrap_or(&Value::Null))
}
fn pair(a: f64, b: f64) -> Value {
    json!([number(a), number(b)])
}
fn bloss(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("bloss")
}
fn elements(v: &Value, k: &str) -> Vec<Value> {
    v.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
}
fn extra(v: &Value) -> Object {
    v.get("extra").and_then(Value::as_object).cloned().unwrap_or_default()
}
fn unsupported(e: coords::Unsupported) -> Failure {
    Failure::new("unsupported_crs", Some(e.0))
}
fn gaps(list: Vec<(usize, f64)>, key: &str) -> Value {
    Value::Array(list.into_iter().map(|(i, g)| json!({ "index": i, key: number(g) })).collect())
}

/// Runs geometry operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let r = match op {
        "geometry.wgs84ToUTM" => {
            let ll = input["lngLat"].as_array().cloned().unwrap_or_default();
            let c = code(input, "crs");
            coords::wgs84_to_plane(
                ll.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
                ll.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
                c,
            )
            .map(|(e, n)| json!({ "easting": number(e), "northing": number(n), "zone": c }))
            .map_err(unsupported)
        }
        "geometry.utmToWgs84" => coords::plane_to_wgs84(f(input, "easting"), f(input, "northing"), code(input, "crs"))
            .map(|(a, b)| pair(a, b))
            .map_err(unsupported),
        "geometry.transformPlanePoint" => {
            coords::transform_plane_point(f(input, "easting"), f(input, "northing"), code(input, "fromCrs"), code(input, "toCrs"))
                .map(|(a, b)| pair(a, b))
                .map_err(unsupported)
        }
        "geometry.transformGridBearing" => coords::transform_grid_bearing(
            f(input, "easting"),
            f(input, "northing"),
            f(input, "bearing"),
            code(input, "fromCrs"),
            code(input, "toCrs"),
        )
        .map(number)
        .map_err(unsupported),
        "geometry.planeCoordsToWgs84" => {
            let pts: Vec<[f64; 2]> = elements(input, "points")
                .iter()
                .map(|p| [p[0].as_f64().unwrap_or(f64::NAN), p[1].as_f64().unwrap_or(f64::NAN)])
                .collect();
            coords::plane_coords_to_wgs84(&pts, code(input, "crs"))
                .map(|v| Value::Array(v.into_iter().map(|[a, b]| pair(a, b)).collect()))
                .map_err(unsupported)
        }
        "geometry.crsName" => Ok(coords::crs_name(code(input, "crs")).map(Value::String).unwrap_or(Value::Null)),
        "geometry.crsSystem" => Ok(coords::crs_system(code(input, "crs")).map(|s| json!(s)).unwrap_or(Value::Null)),
        "geometry.crsZone" => Ok(coords::crs_zone(code(input, "crs")).map(|z| json!(z)).unwrap_or(Value::Null)),
        "geometry.crsCentre" => Ok(coords::crs_centre(code(input, "crs")).map(number).unwrap_or(Value::Null)),
        "geometry.gkZone" => Ok(coords::gk_zone(code(input, "crs")).map(|z| json!(z)).unwrap_or(Value::Null)),
        "geometry.crsDatum" => Ok(coords::crs_datum(code(input, "crs")).map(|d| json!(d.name())).unwrap_or(Value::Null)),
        "geometry.systemCodeAt" => Ok(coords::system_code_at(input["key"].as_str().unwrap_or(""), f(input, "lng"))
            .map(|c| json!(c))
            .unwrap_or(Value::Null)),
        "geometry.epsgForLngLat" => Ok(json!(coords::epsg_for_lng_lat(
            input[0].as_f64().unwrap_or(f64::NAN),
            input[1].as_f64().unwrap_or(f64::NAN)
        ))),
        "geometry.loadGrid" => {
            coords::load_grid_json(input);
            Ok(json!(true))
        }
        "geometry.nadgridsList" => Ok(coords::Datum::from_name(input["datum"].as_str().unwrap_or("DHDN"))
            .and_then(coords::nadgrids_list)
            .map(Value::String)
            .unwrap_or(Value::Null)),

        "geometry.sampleTransitionUtm" => {
            let s = utm(input, "start");
            let length = f(input, "length");
            let pts = transition::sample_transition(
                s.e,
                s.n,
                f(input, "bearing"),
                length,
                of(input, "r1"),
                of(input, "r2"),
                bloss(input),
                input.get("steps").and_then(Value::as_u64).unwrap_or(16) as usize,
                input.get("subdiv").and_then(Value::as_u64).unwrap_or(1) as usize,
                of(input, "end").unwrap_or(length),
            );
            Ok(Value::Array(pts.into_iter().map(|[a, b]| pair(a, b)).collect()))
        }
        "geometry.computeClothoidUtm" => {
            let s = utm(input, "start");
            transition::compute_clothoid(s.e, s.n, s.zone, f(input, "bearing"), f(input, "length"), of(input, "r1"), of(input, "r2"), of(input, "maxDeviation").unwrap_or(transition::DEFAULT_SAGITTA), bloss(input))
                .map(|c| json!({ "coords": c.coords.iter().map(|[a, b]| pair(*a, *b)).collect::<Vec<_>>(), "endUtm": Utm { e: c.end[0], n: c.end[1], zone: s.zone }.to_json(), "endBearing": number(c.end_bearing) }))
                .map_err(unsupported)
        }
        "geometry.transitionPointAtUtm" => {
            let s = utm(input, "start");
            let length = f(input, "length");
            let [e, n] = transition::transition_point_at(
                s.e,
                s.n,
                f(input, "bearing"),
                length,
                of(input, "r1"),
                of(input, "r2"),
                bloss(input),
                of(input, "s").unwrap_or(length),
            );
            Ok(Utm { e, n, zone: s.zone }.to_json())
        }
        "geometry.transitionBearingAtUtm" => {
            let length = f(input, "length");
            Ok(number(transition::transition_bearing_at(
                f(input, "bearing"),
                length,
                of(input, "r1"),
                of(input, "r2"),
                bloss(input),
                of(input, "s").unwrap_or(length),
            )))
        }
        "geometry.transitionShift" => {
            let (p, t, phi) = transition::transition_shift(f(input, "L"), f(input, "R"), bloss(input));
            Ok(json!({ "p": number(p), "t": number(t), "phi": number(phi) }))
        }
        "geometry.clothoidRadiusAt" => Ok(
            transition::clothoid_radius_at(of(input, "r1"), of(input, "r2"), f(input, "length"), f(input, "s"))
                .map(number)
                .unwrap_or(Value::Null),
        ),
        "geometry.projectOnTransitionUtm" => {
            let s = utm(input, "start");
            let p = utm(input, "point");
            let (along, perp) = transition::project_on_transition(
                s.e,
                s.n,
                p.e,
                p.n,
                f(input, "bearing"),
                f(input, "length"),
                of(input, "r1"),
                of(input, "r2"),
                bloss(input),
            );
            Ok(json!({ "along": number(along), "perp": number(perp) }))
        }

        "geometry.computeStraightValuesUtm" => Ok(element::straight_values_json(utm(input, "start"), utm(input, "end"))),
        "geometry.computeCurvedValuesUtm" => Ok(element::curved_values_json(utm(input, "start"), utm(input, "end"), f(input, "radius"))),
        "geometry.arcPlanePoints" => Ok(element::arc_plane_points(
            utm(input, "start"),
            utm(input, "end"),
            f(input, "radius"),
            of(input, "maxDeviation").unwrap_or(0.2),
            input.get("minSegments").and_then(Value::as_u64).unwrap_or(2) as usize,
        )
        .map(|v| Value::Array(v.into_iter().map(|[a, b]| pair(a, b)).collect()))
        .unwrap_or(Value::Null)),
        "geometry.arcCoordsFromRadiusUtm" => element::arc_coords_from_radius(
            utm(input, "start"),
            utm(input, "end"),
            f(input, "radius"),
            of(input, "maxDeviation").unwrap_or(0.2),
        )
        .map(|v| v.map(Value::Array).unwrap_or(Value::Null))
        .map_err(unsupported),
        "geometry.displayCoords" => element::display_coords(&input["el"], code(input, "epsg"), of(input, "spacing").unwrap_or(element::DISPLAY_SPACING))
            .map(Value::Array)
            .map_err(unsupported),
        "geometry.endPointStraightUtm" => Ok(element::end_point_straight(utm(input, "start"), f(input, "bearing"), f(input, "length")).to_json()),
        "geometry.endPointCurvedUtm" => {
            Ok(element::end_point_curved(utm(input, "start"), f(input, "bearing"), f(input, "length"), f(input, "radius")).to_json())
        }
        "geometry.projectOnBearingUtm" => {
            let (a, p) = element::project_on_bearing(utm(input, "start"), utm(input, "point"), f(input, "bearing"));
            Ok(json!({ "along": number(a), "perp": number(p) }))
        }
        "geometry.projectOnArcUtm" => {
            let (a, p) = element::project_on_arc(utm(input, "start"), utm(input, "point"), f(input, "bearing"), of(input, "radius"));
            Ok(json!({ "along": number(a), "perp": number(p) }))
        }
        "geometry.signedRadiusFrom3PointsUtm" => Ok(element::signed_radius_from_3_points(utm(input, "p1"), utm(input, "p2"), utm(input, "p3"))
            .map(number)
            .unwrap_or(Value::Null)),
        "geometry.bearingAfterUtm" => Ok(number(element::bearing_after(f(input, "bearing"), f(input, "length"), of(input, "radius")))),
        "geometry.derivedEndBearing" => Ok(element::derived_end_bearing(input).map(number).unwrap_or(Value::Null)),
        "geometry.resolveEndBearing" => Ok(number(element::resolve_end_bearing(&input["el"], code(input, "epsg")))),
        "geometry.reverseElement" => Ok(element::reverse_element(input)),

        "geometry.straightElement" => factory::straight_element(utm(input, "start"), utm(input, "end"), &extra(input)).map_err(unsupported),
        "geometry.straightFrom" => factory::straight_from(utm(input, "start"), f(input, "bearing"), f(input, "length"), &extra(input)).map_err(unsupported),
        "geometry.arcElement" => factory::arc_element(utm(input, "start"), utm(input, "end"), f(input, "radius"), &extra(input)).map_err(unsupported),
        "geometry.arcFrom" => {
            factory::arc_from(utm(input, "start"), f(input, "bearing"), f(input, "length"), f(input, "radius"), &extra(input)).map_err(unsupported)
        }
        "geometry.transitionElement" => {
            let end = get(input, "endUtm").map(Utm::from_json);
            let tt = input.get("transitionType").cloned().unwrap_or(json!("clothoid"));
            factory::transition_element(
                utm(input, "start"),
                f(input, "bearing"),
                f(input, "length"),
                of(input, "r1"),
                of(input, "r2"),
                &tt,
                end,
                &extra(input),
            )
            .map(|(el, end, eb)| json!({ "element": el, "endUtm": end.to_json(), "endBearing": number(eb) }))
            .map_err(unsupported)
        }

        "geometry.reconstructElements" => Ok(Value::Array(reconstruct::reconstruct_elements(
            &elements(input, "elements"),
            code(input, "epsg"),
        ))),
        "geometry.offsetTrackElements" => parallel::offset_track_elements(&elements(input, "elements"), f(input, "dist"), code(input, "epsg"))
            .map(|v| v.map(Value::Array).unwrap_or(Value::Null))
            .map_err(unsupported),
        "geometry.transformElement" => {
            Ok(plane_transform::transform_element(&input["el"], code(input, "fromCrs"), code(input, "toCrs")).unwrap_or(Value::Null))
        }
        "geometry.transformTrackToPlane" => Ok(plane_transform::transform_track_to_plane(&input["track"], f(input, "toCrs"))),

        "geometry.nodeGaps" => Ok(gaps(
            chain::node_gaps(&elements(input, "elements"), of(input, "tol").unwrap_or(chain::JOINT_TOL)),
            "gap",
        )),
        "geometry.tangentBreaks" => Ok(gaps(
            chain::tangent_breaks(
                &elements(input, "elements"),
                code(input, "epsg"),
                of(input, "tol").unwrap_or(chain::BEARING_TOL),
            ),
            "delta",
        )),
        "geometry.untrueLengths" => Ok(Value::Array(
            chain::untrue_lengths(&elements(input, "elements"), of(input, "tol").unwrap_or(chain::JOINT_TOL))
                .into_iter()
                .map(|(i, l, e)| json!({ "index": i, "length": number(l), "expected": number(e) }))
                .collect(),
        )),
        "geometry.epsgMismatches" => Ok(Value::Array(
            chain::epsg_mismatches(input)
                .into_iter()
                .map(|(i, e)| json!({ "index": i, "epsg": e }))
                .collect(),
        )),
        "geometry.refitToNodes" => Ok(chain::refit_to_nodes(
            &input["el"],
            code(input, "epsg"),
            of(input, "lengthChange").unwrap_or(0.0),
        )),
        "geometry.snapNodeGaps" => {
            let (els, snapped) = chain::snap_node_gaps(&elements(input, "elements"), code(input, "epsg"), f(input, "tol"));
            Ok(json!({ "elements": els, "snapped": gaps(snapped, "gap") }))
        }
        "geometry.snapTrackJoints" => {
            let (tracks, snapped) = chain::snap_track_joints(&elements(input, "tracks"), f(input, "tol"));
            Ok(
                json!({ "tracks": tracks, "snapped": snapped.into_iter().map(|(id, e, g)| json!({ "trackId": id, "endpoint": e, "gap": number(g) })).collect::<Vec<_>>() }),
            )
        }

        "geometry.moveTrackEnd" => {
            let n = input["node"].as_array().cloned().unwrap_or_default();
            Ok(chain::move_track_end(
                &input["track"],
                input["endpoint"].as_str().unwrap_or("BEGIN"),
                [
                    n.first().and_then(Value::as_f64).unwrap_or(f64::NAN),
                    n.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
                ],
            ))
        }
        "geometry.storedTrackCants" => Ok(Value::Array(cant::stored_track_cants(&elements(input, "elements")))),
        "geometry.loadedTrackCants" => Ok(Value::Array(cant::loaded_track_cants(&elements(input, "elements")))),
        "geometry.cantEnds" | "geometry.transitionCantEnds" => {
            let els = elements(input, "elements");
            let i = input["i"].as_u64().unwrap_or(0) as usize;
            let (s, e) = if op == "geometry.cantEnds" {
                cant::cant_ends(&els, i)
            } else {
                cant::transition_cant_ends(&els, i)
            };
            Ok(json!({ "start": number(s), "end": number(e) }))
        }
        "geometry.withCantRamp" => Ok(cant::with_cant_ramp(&input["el"], f(input, "start"), f(input, "end"))),
        "geometry.hasCantEnds" => Ok(json!(cant::has_cant_ends(input))),
        "geometry.withCantAtEnd" => Ok(Value::Array(cant::with_cant_at_end(
            &elements(input, "elements"),
            input["endpoint"].as_str().unwrap_or("BEGIN"),
            f(input, "v"),
        ))),
        "geometry.withCantsOver" => {
            let ends: Vec<(f64, f64)> = elements(input, "ends")
                .iter()
                .map(|p| (p[0].as_f64().unwrap_or(0.0), p[1].as_f64().unwrap_or(0.0)))
                .collect();
            Ok(Value::Array(cant::with_cants_over(
                &elements(input, "elements"),
                input["from"].as_u64().unwrap_or(0) as usize,
                input["to"].as_u64().unwrap_or(0) as usize,
                &ends,
            )))
        }
        "geometry.withCantEdit" => Ok(Value::Array(cant::with_cant_edit(
            &elements(input, "elements"),
            input["i"].as_u64().unwrap_or(0) as usize,
            input["key"].as_str().unwrap_or("cant"),
            f(input, "value"),
        ))),
        _ => return None,
    };
    let _ = truthy;
    Some(r)
}
