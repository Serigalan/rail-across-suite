// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The drawn switch: route coordinates, the body ring, the LCS line, the
//! label, and the symbol rebuilt from its tracks (`symbol.js`).

use rail_across_geom::coords::plane_to_wgs84;
use rail_across_geom::element::{Utm, project_on_arc, reverse_element};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::project_on_transition;
use rail_across_js::{get, js_number, js_string, number, truthy};
use rail_across_track::switch_model::{element_belongs_to_switch, is_link_switch, port_end, port_track};
use rail_across_track::topology::link_symbol;
use serde_json::{Map, Value, json};

use crate::catalogue::{switch_branch_length, switch_form_chain, switch_mark, switch_straight_length, switch_type_by_label};
use crate::crossing::{crossing_end_distance, crossing_wedge_ring};
use crate::route::{
    FormSection, Polyline, Route, chain_geometry, chain_json, clothoid_route_points, form_chain, neg_r, pts_json, switch_branch_chain, switch_chain_bearing_at,
    switch_chain_bent_form, switch_chain_point_utm, switch_chain_segments_utm, switch_chain_to, switch_element_route, switch_route_bearing_at,
    switch_route_segments, switch_route_varies, switch_stem_route, to_route, utm_end_route,
};

const SWITCH_LABEL_OFFSET: f64 = 2.0;

fn wgs(e: f64, n: f64, zone: i64) -> [f64; 2] {
    let (lng, lat) = plane_to_wgs84(e, n, zone).unwrap_or((f64::NAN, f64::NAN));
    [lng, lat]
}

fn route_offset_coords(origin: Utm, bearing: f64, r: &Route, offset: f64, epsg: i64) -> Polyline {
    let n = switch_route_segments(r.length, r.r1, r.r2);
    let pts = switch_route_varies(r).then(|| clothoid_route_points(origin, bearing, r));
    (0..=n)
        .map(|i| {
            let s = r.length * i as f64 / n as f64;
            let p = match &pts {
                Some(pts) => Utm {
                    e: pts[i][0],
                    n: pts[i][1],
                    zone: origin.zone,
                },
                None => utm_end_route(origin, bearing, s, r.r1),
            };
            let b = switch_route_bearing_at(bearing, r, s) * std::f64::consts::PI / 180.0;
            wgs(p.e - offset * b.cos(), p.n + offset * b.sin(), epsg)
        })
        .collect()
}

fn chain_offset_coords(origin: Utm, bearing: f64, chain: &[Route], offset: f64, epsg: i64) -> Polyline {
    let mut out = Vec::new();
    for (i, seg) in switch_chain_segments_utm(origin, bearing, chain).into_iter().enumerate() {
        let c = route_offset_coords(seg.start, seg.bearing, &seg.route, offset, epsg);
        out.extend(if i == 0 { &c[..] } else { &c[1..] });
    }
    out
}

fn route_points_plane(origin: Utm, bearing: f64, r: &Route) -> Vec<[f64; 2]> {
    if switch_route_varies(r) {
        return clothoid_route_points(origin, bearing, r);
    }
    let n = switch_route_segments(r.length, r.r1, r.r1);
    (0..=n)
        .map(|i| {
            let p = utm_end_route(origin, bearing, r.length * i as f64 / n as f64, r.r1);
            [p.e, p.n]
        })
        .collect()
}

/// Points of a switch route or chain in the plane (`switchRoutePointsUtm`).
pub fn switch_route_points_utm(origin: Utm, bearing: f64, chain: &[Route], single: bool) -> Vec<[f64; 2]> {
    if single {
        return route_points_plane(origin, bearing, &chain[0]);
    }
    let mut out = Vec::new();
    for (i, seg) in switch_chain_segments_utm(origin, bearing, chain).into_iter().enumerate() {
        let p = route_points_plane(seg.start, seg.bearing, &seg.route);
        out.extend(if i == 0 { &p[..] } else { &p[1..] });
    }
    out
}

fn project_on_piece(start: Utm, bearing: f64, piece: &Route, p: Utm) -> (f64, f64) {
    if switch_route_varies(piece) {
        project_on_transition(start.e, start.n, p.e, p.n, bearing, piece.length, piece.r1, piece.r2, false)
    } else {
        project_on_arc(start, p, bearing, piece.r1)
    }
}

fn project_on_chain_utm(origin: Utm, bearing: f64, chain: &[Route], p: Utm) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for seg in switch_chain_segments_utm(origin, bearing, chain) {
        let (along, perp) = project_on_piece(seg.start, seg.bearing, &seg.route, p);
        let miss = if along < 0.0 { -along } else { 0f64.max(along - seg.route.length) };
        if best.is_none_or(|(m, pp)| miss < m || (miss == m && perp.abs() < pp.abs())) {
            best = Some((miss, perp));
        }
    }
    best.map(|b| b.1)
}

fn ring_centroid(ring: &[[f64; 2]]) -> [f64; 2] {
    let [ox, oy] = ring[0];
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..ring.len() - 1 {
        let (x1, y1) = (ring[i][0] - ox, ring[i][1] - oy);
        let (x2, y2) = (ring[i + 1][0] - ox, ring[i + 1][1] - oy);
        let f = x1 * y2 - x2 * y1;
        a += f;
        cx += (x1 + x2) * f;
        cy += (y1 + y2) * f;
    }
    if a.abs() < 1e-9 {
        let n = ring.len() as f64;
        return [ring.iter().map(|p| p[0]).sum::<f64>() / n, ring.iter().map(|p| p[1]).sum::<f64>() / n];
    }
    [ox + cx / (3.0 * a), oy + cy / (3.0 * a)]
}

fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

fn away_offset(perp: Option<f64>, d: f64) -> f64 {
    let c = -perp.unwrap_or(f64::NAN);
    let s = js_sign(if c != 0.0 && !c.is_nan() { c } else { 1.0 });
    c - s * d
}

/// What a turnout's lettering needs (`switchLabelGeometry`): (labelCoords, bodyCentre).
pub fn switch_label_geometry(toe: Utm, bearing: f64, through: &[Route], branch: &[Route], epsg: i64) -> (Polyline, [f64; 2]) {
    let ring = switch_fill_ring(
        &switch_route_points_utm(toe, bearing, through, false),
        &switch_route_points_utm(toe, bearing, branch, false),
    );
    let centre = ring_centroid(&ring);
    let centre_utm = Utm {
        e: centre[0],
        n: centre[1],
        zone: epsg,
    };
    let off = away_offset(project_on_chain_utm(toe, bearing, through, centre_utm), SWITCH_LABEL_OFFSET);
    (chain_offset_coords(toe, bearing, through, off, epsg), wgs(centre[0], centre[1], epsg))
}

/// The body of a switch as a closed ring (`switchFillRing`).
pub fn switch_fill_ring(through: &[[f64; 2]], branch: &[[f64; 2]]) -> Vec<[f64; 2]> {
    through.iter().chain(branch.iter().rev()).copied().collect()
}

/// LCS marker line in the plane (`lcsLineUtm`).
pub fn lcs_line_utm(a: [f64; 2], b1: [f64; 2], b2: [f64; 2], d_lcs: f64) -> Option<[[f64; 2]; 2]> {
    let (dxb, dyb) = (b2[0] - b1[0], b2[1] - b1[1]);
    let len_b = dxb.hypot(dyb);
    if !(len_b > 0.0) || !(d_lcs >= 0.0) {
        return None;
    }
    let (dir_e, dir_n) = (dxb / len_b, dyb / len_b);
    let (perp_e, perp_n) = (-dir_n, dir_e);
    let (m_e, m_n) = ((b1[0] + b2[0]) / 2.0, (b1[1] + b2[1]) / 2.0);
    let away = perp_e * (m_e - a[0]) + perp_n * (m_n - a[1]);
    let (s_e, s_n) = if away >= 0.0 { (perp_e, perp_n) } else { (-perp_e, -perp_n) };
    let (ce, cn) = (m_e + d_lcs * s_e, m_n + d_lcs * s_n);
    let half = 4.6 / 2.0;
    Some([[ce - half * dir_e, cn - half * dir_n], [ce + half * dir_e, cn + half * dir_n]])
}

/// The same as WGS84 (`lcsLine`).
/// A mark of `null` is read as JavaScript reads it in arithmetic: 0.
pub fn lcs_line(a: [f64; 2], b1: [f64; 2], b2: [f64; 2], d_lcs: Option<f64>, zone: i64) -> Option<Polyline> {
    lcs_line_utm(a, b1, b2, d_lcs.unwrap_or(0.0)).map(|l| l.iter().map(|p| wgs(p[0], p[1], zone)).collect())
}

/// The tracks by id, as `trackById` holds them.
pub type ById<'a> = &'a Map<String, Value>;

fn track_at<'a>(by_id: ById<'a>, sw: &Value, port: &str) -> Option<&'a Value> {
    port_track(sw, port).and_then(|t| by_id.get(&js_string(Some(t))))
}

fn route_elements(sw: &Value, by_id: ById, port: &str, route: &str, length: Option<f64>, first: bool) -> Vec<Value> {
    let track = track_at(by_id, sw, port);
    let endpoint = port_end(sw, port);
    let els = track.and_then(|t| get(t, "elements")).and_then(Value::as_array).cloned().unwrap_or_default();
    if els.is_empty() || (endpoint != Some("BEGIN") && endpoint != Some("END")) {
        return vec![];
    }
    let at_end = endpoint == Some("END");
    let ordered: Vec<Value> = if at_end { els.into_iter().rev().collect() } else { els };
    let mut out: Vec<Value> = Vec::new();
    let mut total = 0.0;
    for el in ordered {
        let stop = match length {
            None => !out.is_empty(),
            Some(l) => total >= l - NODE_TOL,
        };
        if stop {
            break;
        }
        let sr = el.get("switchRoute");
        let mine = truthy(el.get("switchBranch")) && (!truthy(sr) || sr.and_then(Value::as_str) == Some(route)) && element_belongs_to_switch(&el, sw);
        if !mine && !(first && out.is_empty()) {
            break;
        }
        let l = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
        if !(l > 0.0) {
            break;
        }
        out.push(if at_end { reverse_element(&el) } else { el });
        total += l;
    }
    out
}

fn node(el: &Value, k: &str) -> Option<[f64; 2]> {
    let a = el.get(k)?.as_array()?;
    Some([a.first()?.as_f64().unwrap_or(f64::NAN), a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN)])
}

fn fv(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// A crossing kind's routes as its tracks state them (`crossingRoutesFromTracks`).
pub struct CrossingRoutes {
    pub form: &'static Value,
    pub epsg: Value,
    pub centre: Utm,
    pub main_bearing: f64,
    pub cross_bearing: f64,
    pub main: Vec<Route>,
    pub cross: Vec<Route>,
    pub main_back_bearing: f64,
    pub cross_back_bearing: f64,
    pub main_back: Vec<Route>,
    pub cross_back: Vec<Route>,
}

impl CrossingRoutes {
    pub fn to_json(&self) -> Value {
        json!({
            "type": self.form, "epsg": self.epsg, "centre": self.centre.to_json(),
            "mainBearing": number(self.main_bearing), "crossBearing": number(self.cross_bearing),
            "main": chain_json(&self.main), "cross": chain_json(&self.cross),
            "mainBackBearing": number(self.main_back_bearing), "crossBackBearing": number(self.cross_back_bearing),
            "mainBack": chain_json(&self.main_back), "crossBack": chain_json(&self.cross_back),
        })
    }
}

pub fn crossing_routes_from_tracks(sw: &Value, by_id: ById) -> Option<CrossingRoutes> {
    let form = switch_type_by_label(sw.get("label").and_then(Value::as_str));
    let half = form.map(crossing_end_distance).unwrap_or(f64::NAN);
    if !(half > 0.0) {
        return None;
    }
    let main_els = route_elements(sw, by_id, "C", "main", Some(half), false);
    let cross_els = route_elements(sw, by_id, "D", "cross", Some(half), false);
    let (mf, cf) = (main_els.first()?, cross_els.first()?);
    let (ms, cs) = (node(mf, "startNode")?, node(cf, "startNode")?);
    let epsg = track_at(by_id, sw, "C").and_then(|t| get(t, "epsg")).cloned()?;
    if (ms[0] - cs[0]).hypot(ms[1] - cs[1]) > NODE_TOL {
        return None;
    }
    let zone = js_number(Some(&epsg)) as i64;
    let centre = Utm { e: ms[0], n: ms[1], zone };
    let main = switch_chain_to(&main_els.iter().map(switch_element_route).collect::<Vec<_>>(), half);
    let cross = switch_chain_to(&cross_els.iter().map(switch_element_route).collect::<Vec<_>>(), half);
    let behind = |port: &str, route: &str, ahead: &[Route], ahead_bearing: f64| -> (f64, Vec<Route>) {
        let els = route_elements(sw, by_id, port, route, Some(half), false);
        if let Some(first) = els.first()
            && let Some(s) = node(first, "startNode")
            && (s[0] - centre.e).hypot(s[1] - centre.n) <= NODE_TOL
        {
            return (
                fv(first, "bearing"),
                switch_chain_to(&els.iter().map(switch_element_route).collect::<Vec<_>>(), half),
            );
        }
        (
            (ahead_bearing + 180.0) % 360.0,
            ahead
                .iter()
                .map(|p| Route {
                    length: p.length,
                    r1: neg_r(p.r1),
                    r2: neg_r(p.r2),
                })
                .collect(),
        )
    };
    let (mbb, mb) = behind("A", "main", &main, fv(mf, "bearing"));
    let (cbb, cb) = behind("B", "cross", &cross, fv(cf, "bearing"));
    Some(CrossingRoutes {
        form: form?,
        epsg,
        centre,
        main_bearing: fv(mf, "bearing"),
        cross_bearing: fv(cf, "bearing"),
        main,
        cross,
        main_back_bearing: mbb,
        cross_back_bearing: cbb,
        main_back: mb,
        cross_back: cb,
    })
}

/// The body of a crossing kind in the plane (`crossingBodyUtm`).
pub fn crossing_body_utm(r: &CrossingRoutes) -> [Vec<[f64; 2]>; 2] {
    let along = |b: f64, chain: &[Route]| switch_route_points_utm(r.centre, b, chain, false);
    [
        crossing_wedge_ring(&along(r.main_back_bearing, &r.main_back), &along(r.cross_back_bearing, &r.cross_back)),
        crossing_wedge_ring(&along(r.main_bearing, &r.main), &along(r.cross_bearing, &r.cross)),
    ]
}

/// The routes of a turnout as its tracks state them (`switchRoutesFromTracks`).
pub struct TurnoutRoutes {
    pub form: Option<&'static Value>,
    pub epsg: Value,
    pub node: Utm,
    pub bearing: f64,
    pub main_len: f64,
    pub stem: Vec<Route>,
    pub branch: Vec<Route>,
    pub branch_end: Value,
    pub branch_start_wgs: Option<Value>,
    pub branch_end_wgs: Option<Value>,
}

impl TurnoutRoutes {
    pub fn to_json(&self) -> Value {
        json!({
            "type": self.form, "epsg": self.epsg, "node": self.node.to_json(), "bearing": number(self.bearing), "mainLen": number(self.main_len),
            "stem": chain_json(&self.stem), "branch": chain_json(&self.branch), "branchEnd": self.branch_end,
            "branchStartWgs": self.branch_start_wgs, "branchEndWgs": self.branch_end_wgs,
        })
    }
}

pub fn switch_routes_from_tracks(sw: &Value, by_id: ById) -> Option<TurnoutRoutes> {
    let branch_track = track_at(by_id, sw, "B1");
    let form = switch_type_by_label(sw.get("label").and_then(Value::as_str));
    let branch_els = route_elements(sw, by_id, "B1", "branch", form.map(|f| switch_branch_length(Some(f))), true);
    let first = branch_els.first()?;
    if !(fv(first, "length") > 0.0) || node(first, "startNode").is_none() || node(first, "endNode").is_none() {
        return None;
    }
    let last = branch_els.last()?;
    node(last, "endNode")?;
    let epsg = branch_track.and_then(|t| t.get("epsg")).cloned().unwrap_or(Value::Null);
    let abs_r = get(first, "radius").and_then(Value::as_f64).unwrap_or(0.0).abs();
    let main_len = match form {
        Some(f) => switch_straight_length(f),
        None if abs_r > 0.0 => 2.0 * abs_r * (fv(first, "length") / (2.0 * abs_r)).tan(),
        None => return None,
    };
    let s = node(first, "startNode")?;
    let zone = js_number(Some(&epsg)) as i64;
    let nd = Utm { e: s[0], n: s[1], zone };
    let stem_els = route_elements(sw, by_id, "B2", "main", Some(main_len), false);
    let on_node = stem_els
        .first()
        .and_then(|e| node(e, "startNode"))
        .is_some_and(|p| (p[0] - nd.e).hypot(p[1] - nd.n) <= NODE_TOL);
    let coords = |el: &Value| {
        el.get("geometry")
            .and_then(|g| get(g, "coordinates"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (fc, lc) = (coords(first), coords(last));
    Some(TurnoutRoutes {
        form,
        epsg,
        node: nd,
        bearing: fv(first, "bearing"),
        main_len,
        stem: if on_node {
            switch_chain_to(&stem_els.iter().map(switch_element_route).collect::<Vec<_>>(), main_len)
        } else {
            vec![switch_stem_route(sw, main_len)]
        },
        branch: branch_els.iter().map(switch_element_route).collect(),
        branch_end: last.get("endNode").cloned().unwrap_or(Value::Null),
        branch_start_wgs: (fc.len() >= 2).then(|| fc[0].clone()),
        branch_end_wgs: (lc.len() >= 2).then(|| lc[lc.len() - 1].clone()),
    })
}

fn pair(v: &Value) -> [f64; 2] {
    [
        v.get(0).and_then(Value::as_f64).unwrap_or(f64::NAN),
        v.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
    ]
}

/// The display symbol rebuilt from the tracks (`rebuildSwitchSymbol`).
pub fn rebuild_switch_symbol(sw: &Value, by_id: ById) -> Value {
    let mut rest = sw.as_object().cloned().unwrap_or_default();
    for k in ["fillCoords", "lcsCoords", "labelCoords", "bodyCentre", "bauform"] {
        rest.remove(k);
    }
    if is_link_switch(sw) {
        let tracks: Vec<Value> = by_id.values().cloned().collect();
        return link_symbol(&Value::Object(rest), &tracks);
    }
    let kind = sw.get("kind").and_then(Value::as_str);
    if truthy(sw.get("kind")) && kind != Some("switch") {
        let Some(r) = crossing_routes_from_tracks(sw, by_id) else {
            return Value::Object(rest);
        };
        let zone = r.centre.zone;
        let fill: Vec<Value> = crossing_body_utm(&r)
            .iter()
            .map(|ring| pts_json(&ring.iter().map(|p| wgs(p[0], p[1], zone)).collect::<Vec<_>>()))
            .collect();
        let off = away_offset(project_on_chain_utm(r.centre, r.main_bearing, &r.main, r.centre), SWITCH_LABEL_OFFSET);
        rest.insert("fillCoords".into(), Value::Array(fill));
        rest.insert(
            "labelCoords".into(),
            pts_json(&chain_offset_coords(r.centre, r.main_bearing, &r.main, off, zone)),
        );
        rest.insert("bodyCentre".into(), pts_json(&[wgs(r.centre.e, r.centre.n, zone)])[0].clone());
        return Value::Object(rest);
    }
    let Some(r) = switch_routes_from_tracks(sw, by_id) else {
        return Value::Object(rest);
    };
    let (Some(bsw), Some(bew)) = (&r.branch_start_wgs, &r.branch_end_wgs) else {
        return Value::Object(rest);
    };
    let zone = r.node.zone;
    let main_end = switch_chain_point_utm(r.node, r.bearing, &r.stem, None).unwrap_or(r.node);
    let main_end_wgs = wgs(main_end.e, main_end.n, zone);
    let main_coords = chain_geometry(r.node, pair(bsw), r.bearing, &r.stem, main_end_wgs).0;
    let branch_coords = chain_geometry(r.node, pair(bsw), r.bearing, &r.branch, pair(bew)).0;
    let lcs = r
        .form
        .and_then(|f| lcs_line([r.node.e, r.node.n], pair(&r.branch_end), [main_end.e, main_end.n], switch_mark(Some(f)), zone));
    let (label, centre) = switch_label_geometry(r.node, r.bearing, &r.stem, &r.branch, zone);
    rest.insert("fillCoords".into(), pts_json(&switch_fill_ring(&main_coords, &branch_coords)));
    rest.insert("labelCoords".into(), pts_json(&label));
    rest.insert("bodyCentre".into(), json!([number(centre[0]), number(centre[1])]));
    let symmetric = r.form.is_some_and(|f| f.get("symmetric") == Some(&json!(true)));
    rest.insert("bauform".into(), json!(switch_chain_bent_form(&r.stem, &r.branch, symmetric)));
    if let Some(l) = lcs {
        rest.insert("lcsCoords".into(), pts_json(&l));
    }
    Value::Object(rest)
}

/// The stem `mainR` states (`computeSwitchGeometryUtm`'s fourth form).
pub enum MainR {
    Radius(Option<f64>),
    Clothoid(Option<f64>, Option<f64>),
    Chain(Vec<Route>),
}

impl MainR {
    pub fn from_json(v: Option<&Value>) -> MainR {
        match v {
            Some(Value::Array(_)) => MainR::Chain(crate::route::to_chain(v.expect("array"))),
            Some(Value::Object(o)) => MainR::Clothoid(o.get("r1").and_then(Value::as_f64), o.get("r2").and_then(Value::as_f64)),
            Some(Value::Null) | None => MainR::Radius(None),
            Some(x) => MainR::Radius(x.as_f64()),
        }
    }
}

fn is_straight_piece(p: &Route) -> bool {
    p.r1.is_none() && p.r2.is_none()
}

#[allow(clippy::too_many_arguments)]
fn swapped_plain_geometry(
    start: Utm,
    start_wgs: [f64; 2],
    bearing: f64,
    form: &Value,
    trailing: bool,
    stem: &[Route],
    branch_chain: &[Route],
    arc_len: f64,
) -> Value {
    if stem.iter().all(is_straight_piece) || !branch_chain.iter().all(is_straight_piece) {
        return Value::Null;
    }
    let toe = if trailing {
        switch_chain_point_utm(start, bearing, stem, Some(arc_len)).unwrap_or(start)
    } else {
        start
    };
    let toe_bearing = if trailing {
        (switch_chain_bearing_at(bearing, stem, Some(arc_len)) + 180.0) % 360.0
    } else {
        bearing
    };
    let target = if trailing {
        start
    } else {
        switch_chain_point_utm(start, bearing, stem, Some(arc_len)).unwrap_or(start)
    };
    let toe_utm = Utm { zone: start.zone, ..toe };
    let miss = |g: &Value| {
        let c = Utm::from_json(&g["curvedUtm"]);
        (c.e - target.e).hypot(c.n - target.n)
    };
    let [left, right] = ["left", "right"].map(|side| {
        compute_switch_geometry_utm(
            toe_utm,
            toe_bearing,
            form,
            side,
            false,
            if trailing { None } else { Some(start_wgs) },
            &MainR::Radius(None),
        )
    });
    if miss(&left) <= miss(&right) { left } else { right }
}

/// Geometry of a turnout laid from a plane point (`computeSwitchGeometryUtm`).
pub fn compute_switch_geometry_utm(start: Utm, bearing: f64, form: &Value, side: &str, trailing: bool, start_wgs: Option<[f64; 2]>, main_r: &MainR) -> Value {
    let arc_len = switch_branch_length(Some(form));
    let straight_len = switch_straight_length(form);
    let symmetric = form.get("symmetric") == Some(&json!(true));
    let base: Vec<FormSection> = form_chain(&Value::Array(switch_form_chain(Some(form), side)));
    let fchain: Vec<FormSection> = if symmetric {
        base.iter()
            .map(|s| FormSection {
                signed_r: s.signed_r.map(|r| r / 2.0),
                ..*s
            })
            .collect()
    } else {
        base.clone()
    };
    let form_signed_r = base.first().and_then(|s| s.signed_r);
    let unbent = if symmetric {
        form_signed_r.map(|r| if trailing { r } else { -r })
    } else {
        None
    };
    let stem: Vec<Route> = match main_r {
        MainR::Chain(c) => switch_chain_to(c, straight_len),
        MainR::Clothoid(r1, r2) => vec![to_route(&json!({ "length": straight_len, "r1": r1, "r2": r2 }))],
        MainR::Radius(r) => vec![to_route(&json!({ "length": straight_len, "radius": r.or(unbent) }))],
    };
    let main_signed_r = stem.first().and_then(|s| s.r1);
    let zone = start.zone;
    let s_wgs = start_wgs.unwrap_or_else(|| wgs(start.e, start.n, zone));
    let straight_utm = switch_chain_point_utm(start, bearing, &stem, None).unwrap_or(start);
    let main_end_bearing = switch_chain_bearing_at(bearing, &stem, None);
    let arc_origin = if trailing { straight_utm } else { start };
    let curve_bearing = if trailing { (main_end_bearing + 180.0) % 360.0 } else { bearing };
    let stem_chain: Vec<Route> = if trailing {
        stem.iter()
            .rev()
            .map(|p| Route {
                length: p.length,
                r1: neg_r(p.r2),
                r2: neg_r(p.r1),
            })
            .collect()
    } else {
        stem.clone()
    };
    let stem_at_toe = stem_chain.first().and_then(|s| s.r1);
    let branch_chain = switch_branch_chain(&fchain, &stem_chain);
    let signed_r = branch_chain.first().and_then(|b| b.r1);
    let curved = switch_chain_point_utm(arc_origin, curve_bearing, &branch_chain, None).unwrap_or(arc_origin);
    let up = |u: Utm| [u.e, u.n];
    let port_a = if trailing { up(straight_utm) } else { up(start) };
    let port_b1 = up(curved);
    let port_b2 = if trailing { up(start) } else { up(straight_utm) };
    let straight_end = wgs(straight_utm.e, straight_utm.n, zone);
    let arc_origin_wgs = if trailing { straight_end } else { s_wgs };
    let curved_end = wgs(curved.e, curved.n, zone);
    let (straight_coords, main_segs) = chain_geometry(start, s_wgs, bearing, &stem, straight_end);
    let (arc_coords, branch_segs) = chain_geometry(arc_origin, arc_origin_wgs, curve_bearing, &branch_chain, curved_end);
    let lcs = lcs_line(port_a, port_b1, port_b2, switch_mark(Some(form)), zone);
    let through: Vec<[f64; 2]> = if trailing {
        straight_coords.iter().rev().copied().collect()
    } else {
        straight_coords.clone()
    };
    let fill = switch_fill_ring(&through, &arc_coords);
    let (label, centre) = switch_label_geometry(arc_origin, curve_bearing, &stem_chain, &branch_chain, zone);
    let p = |x: [f64; 2]| json!([number(x[0]), number(x[1])]);
    let opt = |x: Option<f64>| x.map(number).unwrap_or(Value::Null);
    let segs = |s: Vec<(crate::route::Segment, Polyline)>| -> Value {
        Value::Array(
            s.into_iter()
                .map(|(g, c)| {
                    let mut o = g.to_json();
                    o["coords"] = pts_json(&c);
                    o
                })
                .collect(),
        )
    };
    let stem_segments = if trailing {
        Value::Array(
            switch_chain_segments_utm(arc_origin, curve_bearing, &stem_chain)
                .into_iter()
                .map(|s| s.to_json())
                .collect(),
        )
    } else {
        segs(main_segs)
    };
    json!({
        "straightCoords": pts_json(&straight_coords), "arcCoords": pts_json(&arc_coords), "fillCoords": pts_json(&fill),
        "labelCoords": pts_json(&label), "bodyCentre": p(centre),
        "bauform": switch_chain_bent_form(&stem_chain, &branch_chain, symmetric),
        "swapped": if symmetric { Value::Null } else { swapped_plain_geometry(start, s_wgs, bearing, form, trailing, &stem, &branch_chain, arc_len) },
        "portA": p(port_a), "portB1": p(port_b1), "portB2": p(port_b2),
        "portA_wgs": p(if trailing { straight_end } else { s_wgs }), "portB1_wgs": p(curved_end), "portB2_wgs": p(if trailing { s_wgs } else { straight_end }),
        "lcsCoords": lcs.map(|l| pts_json(&l)).unwrap_or(Value::Null),
        "straightEnd": p(straight_end), "arcOrigin": p(arc_origin_wgs), "curvedEnd": p(curved_end),
        "startUtm": start.to_json(), "straightUtm": straight_utm.to_json(), "arcOriginUtm": arc_origin.to_json(), "curvedUtm": curved.to_json(),
        "signedR": opt(signed_r), "mainSignedR": opt(main_signed_r), "stemAtToe": opt(stem_at_toe),
        "stemChain": chain_json(&stem_chain), "branchChain": chain_json(&branch_chain),
        "stemSegments": stem_segments, "branchSegments": segs(branch_segs),
        "curveBearing": number(curve_bearing), "mainEndBearing": number(main_end_bearing),
        "branchEndBearing": number(switch_chain_bearing_at(curve_bearing, &branch_chain, None)),
        "arcLen": number(arc_len), "straightLen": number(straight_len),
    })
}
