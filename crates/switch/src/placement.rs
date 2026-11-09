// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Where a turnout may lie on a track (`switchPlacement.js`).

use rail_across_geom::cant::cant_ends;
use rail_across_geom::element::{Utm, node_utm, project_on_arc, resolve_end_bearing, reverse_element};
use rail_across_geom::tolerances::NODE_TOL;
use rail_across_geom::transition::{is_bloss, project_on_transition};
use rail_across_js::{get, has, js_number, number, truthy};
use rail_across_track::heights::{element_at_station, point_at_station_utm};
use serde_json::{Value, json};

use crate::catalogue::as_radius;
use crate::route::{Route, chain_json, switch_element_route, switch_route_bearing_at, switch_route_slice, to_chain};

const KINK_TOL: f64 = 1e-6;
const DEG2RAD: f64 = std::f64::consts::PI / 180.0;

fn angle_between(a: f64, b: f64) -> f64 {
    (((a - b + 540.0) % 360.0) - 180.0).abs() * DEG2RAD
}

fn f(el: &Value, k: &str) -> Option<f64> {
    get(el, k).and_then(Value::as_f64)
}

fn refusal(el: &Value) -> Option<&'static str> {
    if truthy(el.get("switchBranch")) {
        return Some("switch_on_track_over_switch");
    }
    let t = el.get("elementType").and_then(Value::as_f64);
    if t == Some(2.0) && (el.get("transitionType").and_then(Value::as_str) == Some("bloss") || !has(el, "r1")) {
        return Some("switch_on_track_bloss");
    }
    if t != Some(2.0)
        && f(el, "radius").is_none()
        && let Some(eb) = f(el, "endBearing")
        && angle_between(eb, f(el, "bearing").unwrap_or(f64::NAN)) > KINK_TOL
    {
        return Some("switch_on_track_kink");
    }
    None
}

fn els(t: &Value) -> &[Value] {
    t.get("elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// May a turnout on element `i` run on into its neighbour (`turnoutRunsOn`)?
pub fn turnout_runs_on(elements: &[Value], i: usize, side: &str, epsg: i64) -> bool {
    let j = if side == "before" { i.checked_sub(1) } else { Some(i + 1) };
    let Some(j) = j else { return false };
    let (Some(ej), Some(ei)) = (elements.get(j), elements.get(i)) else {
        return false;
    };
    let (a, b) = if side == "before" { (ej, ei) } else { (ei, ej) };
    if !(f(ej, "length").unwrap_or(f64::NAN) > 0.0) || refusal(ej).is_some() {
        return false;
    }
    angle_between(f(b, "bearing").unwrap_or(f64::NAN), resolve_end_bearing(a, epsg)) <= KINK_TOL
}

fn epsg_of(track: &Value) -> i64 {
    js_number(track.get("epsg")) as i64
}

fn first_coord(el: &Value) -> Option<&Value> {
    el.get("geometry").and_then(|g| g.get("coordinates")).and_then(|c| c.get(0))
}

/// Station of a point clicked beside element `el_idx` (`clickStation`).
pub fn click_station(track: &Value, el_idx: usize, click: Utm) -> f64 {
    let el = &els(track)[el_idx];
    let epsg = epsg_of(track);
    let start = node_utm(el.get("startNode"), first_coord(el), epsg).unwrap_or(Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    });
    let bearing = f(el, "bearing").unwrap_or(f64::NAN);
    let len = el.get("length").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let (along, _) = if el.get("elementType").and_then(Value::as_f64) == Some(2.0) {
        project_on_transition(
            start.e,
            start.n,
            click.e,
            click.n,
            bearing,
            len,
            f(el, "r1"),
            f(el, "r2"),
            is_bloss(el.get("transitionType").and_then(Value::as_str)),
        )
    } else {
        project_on_arc(start, click, bearing, as_radius(f(el, "radius")))
    };
    let before: f64 = els(track).iter().take(el_idx).map(|e| f(e, "length").unwrap_or(0.0)).sum();
    before + len.min(0f64.max(along))
}

/// One element under a turnout.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub el_idx: usize,
    pub s0: f64,
    pub length: f64,
    pub from: f64,
    pub to: f64,
    pub cant_start: f64,
    pub cant_end: f64,
}

/// Where a turnout lies on a track (`placeSwitchOnTrack`).
#[derive(Clone, Debug)]
pub struct Placement {
    pub toe: Utm,
    pub bearing: f64,
    pub joint: Option<usize>,
    pub el_idx: Option<usize>,
    pub s: Option<f64>,
    pub cuts_clothoid: bool,
    pub pieces: Vec<Route>,
    pub spans: Vec<Span>,
    pub end: Utm,
}

impl Placement {
    /// The cant `s` from the toe, on the span holding the piece `on` where given (`cantAt`).
    pub fn cant_at(&self, s: f64, on: Option<(f64, f64)>) -> f64 {
        let at = on.map(|(s0, l)| s0 + l / 2.0).unwrap_or(s);
        let sp = self
            .spans
            .iter()
            .find(|sp| at <= sp.s0 + sp.length)
            .unwrap_or(&self.spans[self.spans.len() - 1]);
        if sp.length > 0.0 {
            sp.cant_start + (sp.cant_end - sp.cant_start) * (s - sp.s0) / sp.length
        } else {
            sp.cant_start
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "toeUtm": self.toe.to_json(), "bearing": number(self.bearing), "joint": self.joint, "elIdx": self.el_idx, "s": self.s.map(number),
            "cutsClothoid": self.cuts_clothoid, "pieces": chain_json(&self.pieces),
            "spans": self.spans.iter().map(|sp| json!({ "elIdx": sp.el_idx, "s0": number(sp.s0), "length": number(sp.length), "from": number(sp.from), "to": number(sp.to),
                "cantStart": number(sp.cant_start), "cantEnd": number(sp.cant_end) })).collect::<Vec<_>>(),
            "endUtm": self.end.to_json(),
        })
    }

    /// A placement as `to_json` wrote it — what the page hands back to lay what its dialog settled on.
    pub fn from_json(v: &Value) -> Option<Placement> {
        let o = v.as_object()?;
        let f = |x: &Value| x.as_f64().unwrap_or(f64::NAN);
        let idx = |x: Option<&Value>| x.and_then(Value::as_u64).map(|i| i as usize);
        Some(Placement {
            toe: Utm::from_json(o.get("toeUtm")?),
            bearing: f(o.get("bearing")?),
            joint: idx(o.get("joint")),
            el_idx: idx(o.get("elIdx")),
            s: o.get("s").and_then(Value::as_f64),
            cuts_clothoid: truthy(o.get("cutsClothoid")),
            pieces: to_chain(o.get("pieces")?),
            spans: o
                .get("spans")?
                .as_array()?
                .iter()
                .map(|sp| Span {
                    el_idx: sp.get("elIdx").and_then(Value::as_u64).unwrap_or(0) as usize,
                    s0: f(&sp["s0"]),
                    length: f(&sp["length"]),
                    from: f(&sp["from"]),
                    to: f(&sp["to"]),
                    cant_start: f(&sp["cantStart"]),
                    cant_end: f(&sp["cantEnd"]),
                })
                .collect(),
            end: Utm::from_json(o.get("endUtm")?),
        })
    }
}

pub fn place_switch_on_track(track: &Value, station: f64, reversed: bool, length: f64) -> Result<Placement, &'static str> {
    let es = els(track);
    let epsg = epsg_of(track);
    let (hit_idx, hit_s) = element_at_station(es, station).ok_or("switch_on_track_outside")?;
    let hit = &es[hit_idx];
    let hit_len = f(hit, "length").unwrap_or(f64::NAN);
    let dir = if reversed { -1.0 } else { 1.0 };
    let joint = if hit_s <= NODE_TOL {
        Some(hit_idx)
    } else if hit_s >= hit_len - NODE_TOL {
        Some(hit_idx + 1)
    } else {
        None
    };
    let utm_or = |r: Result<Utm, _>| {
        r.unwrap_or(Utm {
            e: f64::NAN,
            n: f64::NAN,
            zone: epsg,
        })
    };
    let (mut idx, mut local, toe, tangent): (i64, f64, Utm, f64);
    if let Some(j) = joint {
        idx = if reversed { j as i64 - 1 } else { j as i64 };
        let el = (idx >= 0).then(|| es.get(idx as usize)).flatten().ok_or("switch_on_track_no_room")?;
        let coords = el.get("geometry").and_then(|g| g.get("coordinates")).and_then(Value::as_array);
        if reversed {
            toe = utm_or(node_utm(el.get("endNode"), coords.and_then(|c| c.last()), epsg));
            tangent = resolve_end_bearing(el, epsg);
            local = f(el, "length").unwrap_or(f64::NAN);
        } else {
            toe = utm_or(node_utm(el.get("startNode"), coords.and_then(|c| c.first()), epsg));
            tangent = f(el, "bearing").unwrap_or(f64::NAN);
            local = 0.0;
        }
    } else {
        if let Some(why) = refusal(hit) {
            return Err(why);
        }
        idx = hit_idx as i64;
        local = hit_s;
        toe = point_at_station_utm(hit, local, epsg).unwrap_or(Utm {
            e: f64::NAN,
            n: f64::NAN,
            zone: epsg,
        });
        tangent = switch_route_bearing_at(f(hit, "bearing").unwrap_or(f64::NAN), &switch_element_route(hit), local);
    }
    let bearing = if reversed { (tangent + 180.0) % 360.0 } else { tangent };
    let mut pieces = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut covered = 0.0;
    let mut last_end_bearing: Option<f64> = None;
    while covered < length - NODE_TOL {
        let el = (idx >= 0).then(|| es.get(idx as usize)).flatten().ok_or("switch_on_track_no_room")?;
        let el_len = f(el, "length").unwrap_or(f64::NAN);
        if el_len > 0.0 {
            if let Some(why) = refusal(el) {
                return Err(why);
            }
            let oriented = if reversed { reverse_element(el) } else { el.clone() };
            if let Some(lb) = last_end_bearing
                && angle_between(f(&oriented, "bearing").unwrap_or(f64::NAN), lb) > KINK_TOL
            {
                return Err("switch_on_track_kink");
            }
            let route = switch_element_route(&oriented);
            let from = if reversed { el_len - local } else { local };
            let take = (route.length - from).min(length - covered);
            let to = from + take;
            pieces.push(switch_route_slice(&route, from, to));
            let (rs, re) = cant_ends(es, idx as usize);
            let cant_own = |s: f64| rs + (re - rs) * s / el_len;
            let cant_run = |s: f64| dir * cant_own(if reversed { el_len - s } else { s });
            spans.push(Span {
                el_idx: idx as usize,
                s0: covered,
                length: take,
                from,
                to,
                cant_start: cant_run(from),
                cant_end: cant_run(to),
            });
            covered += take;
            last_end_bearing = Some(resolve_end_bearing(&oriented, epsg));
        }
        idx += if reversed { -1 } else { 1 };
        local = if reversed {
            (idx >= 0).then(|| es.get(idx as usize)).flatten().and_then(|e| f(e, "length")).unwrap_or(0.0)
        } else {
            0.0
        };
    }
    let last = *spans.last().ok_or("switch_on_track_no_room")?;
    let end_el = &es[last.el_idx];
    let end_len = f(end_el, "length").unwrap_or(f64::NAN);
    let end = point_at_station_utm(end_el, if reversed { end_len - last.to } else { last.to }, epsg).unwrap_or(Utm {
        e: f64::NAN,
        n: f64::NAN,
        zone: epsg,
    });
    Ok(Placement {
        toe,
        bearing,
        joint,
        el_idx: joint.is_none().then_some(hit_idx),
        s: joint.is_none().then_some(hit_s),
        cuts_clothoid: joint.is_none() && hit.get("elementType").and_then(Value::as_f64) == Some(2.0),
        pieces,
        spans,
        end,
    })
}

/// May a turnout be attached at the far end of element `el_idx` (`switchEndAnchorRefusal`)?
pub fn switch_end_anchor_refusal(track: &Value, el_idx: usize) -> Option<&'static str> {
    let es = els(track);
    if es.get(el_idx).is_none() {
        return Some("switch_anchor_no_element");
    }
    if el_idx != es.len() - 1 {
        return Some("switch_anchor_not_track_end");
    }
    None
}
