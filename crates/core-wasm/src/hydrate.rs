// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Loading a record (`hydrateProjects`, decision 0005): the display
//! geometry derived from the plane data alone — every element's geometry and
//! render coordinates, the end bearing an arc or a transition leaves to its
//! values, each track's coordinates, the switch symbols and the platform
//! polygons. Nothing is converted or repaired: what reaches this point has
//! been through the door or was written by the app.

use rail_across_geom::cant::loaded_track_cants;
use rail_across_geom::element::derived_end_bearing;
use rail_across_geom::reconstruct::reconstruct_elements;
use rail_across_js::{js_number, js_string, number};
use rail_across_switch::symbol::rebuild_switch_symbol;
use rail_across_track::model::rebuild_coords;
use rail_across_track::platforms::rebuild_platform_symbol;
use serde_json::{Map, Value};

fn hydrated_track(t: &Value) -> Value {
    let Some(o) = t.as_object() else { return t.clone() };
    let mut out = o.clone();
    let elements: Vec<Value> = t.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    let elements: Vec<Value> = loaded_track_cants(&elements)
        .into_iter()
        .map(|el| {
            if el.get("endBearing").is_some_and(|b| !b.is_null()) {
                return el;
            }
            match (derived_end_bearing(&el), el) {
                (Some(b), Value::Object(mut e)) => {
                    e.insert("endBearing".into(), number(b));
                    Value::Object(e)
                }
                (_, el) => el,
            }
        })
        .collect();
    let epsg = js_number(t.get("epsg"));
    let elements = reconstruct_elements(&elements, if epsg.is_finite() { epsg as i64 } else { 0 });
    out.insert("coordinates".into(), Value::Array(rebuild_coords(&elements)));
    out.insert("elements".into(), Value::Array(elements));
    Value::Object(out)
}

/// One record with its display geometry.
pub fn hydrate_project(p: &Value) -> Value {
    let Some(o) = p.as_object() else { return p.clone() };
    let mut out = o.clone();
    let tracks: Vec<Value> = p
        .get("tracks")
        .and_then(Value::as_array)
        .map(|ts| ts.iter().map(hydrated_track).collect())
        .unwrap_or_default();
    if p.get("tracks").is_some() {
        out.insert("tracks".into(), Value::Array(tracks.clone()));
    }
    let has = |k: &str| p.get(k).is_some_and(|v| !v.is_null());
    if has("switches") || has("platforms") {
        if let Some(Value::Array(sws)) = p.get("switches") {
            let by_id: Map<String, Value> = tracks.iter().map(|t| (js_string(t.get("id")), t.clone())).collect();
            out.insert(
                "switches".into(),
                Value::Array(sws.iter().map(|sw| rebuild_switch_symbol(sw, &by_id)).collect()),
            );
        }
        if let Some(Value::Array(pfs)) = p.get("platforms") {
            out.insert(
                "platforms".into(),
                Value::Array(pfs.iter().map(|pf| rebuild_platform_symbol(pf, &tracks)).collect()),
            );
        }
    }
    Value::Object(out)
}
