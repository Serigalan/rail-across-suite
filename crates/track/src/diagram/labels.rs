// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Labels of a diagram set where they cover nothing (`labelPlacement.js`).

use rail_across_js::{js_string, number};
use serde_json::{Map, Value, json};

#[derive(Clone, Copy, Debug)]
pub struct Box2 {
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

impl Box2 {
    pub fn from_json(v: &Value) -> Box2 {
        let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
        Box2 {
            x0: f("x0"),
            x1: f("x1"),
            y0: f("y0"),
            y1: f("y1"),
        }
    }
}

/// The size of a label [px] (`labelBox`): (w, h).
pub fn label_box(text: &str) -> (f64, f64) {
    (text.encode_utf16().count() as f64 * 5.2 + 4.0, 11.0)
}

/// Whether a segment touches a box (`segmentHitsBox`, Liang–Barsky).
pub fn segment_hits_box([x1, y1, x2, y2]: [f64; 4], b: Box2) -> bool {
    let (mut t0, mut t1) = (0.0, 1.0);
    let (dx, dy) = (x2 - x1, y2 - y1);
    for (p, q) in [(-dx, x1 - b.x0), (dx, b.x1 - x1), (-dy, y1 - b.y0), (dy, b.y1 - y1)] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            if r > t1 {
                return false;
            }
            if r > t0 {
                t0 = r;
            }
        } else {
            if r < t0 {
                return false;
            }
            if r < t1 {
                t1 = r;
            }
        }
    }
    true
}

fn boxes_meet(a: &Box2, b: &Box2) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Where each label goes (`placeLabels`): key → { x, y }, in the items' order.
pub fn place_labels(items: &[Value], segments: &[[f64; 4]], node_boxes: &[Box2]) -> Map<String, Value> {
    let mut placed: Vec<Box2> = Vec::new();
    let mut out = Map::new();
    for it in items {
        let (w, h) = label_box(&js_string(it.get("text")));
        let f = |k: &str| it.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
        let mut best: Option<(usize, Box2, f64, f64)> = None;
        for o in it.get("offsets").and_then(Value::as_array).into_iter().flatten() {
            let d = |i: usize| o.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
            let (cx, cy) = (f("x") + d(0), f("y") + d(1));
            let bx = Box2 {
                x0: cx - w / 2.0,
                x1: cx + w / 2.0,
                y0: cy - h / 2.0,
                y1: cy + h / 2.0,
            };
            let hits = segments.iter().filter(|s| segment_hits_box(**s, bx)).count() * 2
                + node_boxes.iter().filter(|nb| boxes_meet(nb, &bx)).count() * 2
                + placed.iter().filter(|pb| boxes_meet(pb, &bx)).count();
            if best.is_none_or(|b| hits < b.0) {
                best = Some((hits, bx, cx, cy));
            }
            if hits == 0 {
                break;
            }
        }
        let Some((_, bx, cx, cy)) = best else { continue };
        placed.push(bx);
        out.insert(js_string(it.get("key")), json!({ "x": number(cx), "y": number(cy) }));
    }
    out
}
