// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A reference axis: a survey's alignment sampled every centimetre, kept as
//! millimetre deltas (`referenceAxis.js`).

use rail_across_geom::element::element_points;
use rail_across_js::{generate_id, get, js_number, js_round, number, truthy};
use rail_across_track::heights::gradient_at;
use serde_json::{Value, json};

const REFERENCE_STEP: f64 = 0.01;
pub const REFERENCE_MAX_LENGTH: f64 = 2000.0;

fn mm(v: f64) -> f64 {
    js_round(v * 1000.0)
}

/// The stretches of at most `max` a range is cut into (`referenceStretches`).
pub fn reference_stretches(from: f64, to: f64, max: f64) -> Vec<(f64, f64)> {
    let n = 1usize.max(((to - from) / max - 1e-9).ceil() as usize);
    let mut out = Vec::new();
    let mut at = from;
    for k in 1..n {
        let round = js_round((from + (to - from) * k as f64 / n as f64) / 10.0) * 10.0;
        let cut = round.min(at + max).max(to - (n - k) as f64 * max);
        out.push((at, cut));
        at = cut;
    }
    out.push((at, to));
    out
}

fn deltas(values: &[f64]) -> Vec<Value> {
    let mut prev = 0.0;
    values
        .iter()
        .map(|v| {
            let d = v - prev;
            prev = *v;
            number(d)
        })
        .collect()
}

/// The elements sampled every `step` between two stations (`sampleAlong`): (offset, e, n).
pub fn sample_along(elements: &[Value], epsg: i64, from: f64, to: f64, step: f64) -> (f64, Vec<f64>, Vec<f64>) {
    let k0 = (from / step - 1e-9).ceil() as i64;
    let k1 = (to / step + 1e-9).floor() as i64;
    let (mut e, mut n) = (Vec::new(), Vec::new());
    let mut start = 0.0;
    let mut k = k0;
    for el in elements {
        let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        let end = start + len;
        if end >= k as f64 * step - 1e-9 && start <= k1 as f64 * step + 1e-9 && len > 0.0 {
            let pts = element_points(el, epsg, step).unwrap_or_default();
            let m = pts.len().saturating_sub(1);
            while k <= k1 && k as f64 * step <= end + 1e-9 {
                let x = (m as f64).min(0f64.max((k as f64 * step - start) / len * m as f64));
                let j = (m.saturating_sub(1)).min(x.floor() as usize);
                let t = x - j as f64;
                e.push(pts[j][0] + (pts[j + 1][0] - pts[j][0]) * t);
                n.push(pts[j][1] + (pts[j + 1][1] - pts[j][1]) * t);
                k += 1;
            }
        }
        start = end;
        if k > k1 {
            break;
        }
    }
    (k0 as f64 * step, e, n)
}

/// A reference axis from an alignment and its gradient (`buildReferenceAxis`).
pub fn build_reference_axis(o: &Value) -> Result<Value, String> {
    let f = |k: &str| get(o, k).and_then(Value::as_f64);
    let (from, to) = (f("from").unwrap_or(f64::NAN), f("to").unwrap_or(f64::NAN));
    if !(to > from) || to - from > REFERENCE_MAX_LENGTH + 1e-6 {
        return Err("reference axis: stretch".into());
    }
    let start_station = f("startStation").unwrap_or(0.0);
    let step = f("step").unwrap_or(REFERENCE_STEP);
    let epsg = js_number(o.get("epsg"));
    let elements = o.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();
    let (offset, e, n) = sample_along(&elements, epsg as i64, from - start_station, to - start_station, step);
    if e.len() < 2 {
        return Err("reference axis: no points".into());
    }
    let s0 = start_station + offset;
    let em: Vec<f64> = e.iter().map(|v| mm(*v)).collect();
    let nm: Vec<f64> = n.iter().map(|v| mm(*v)).collect();
    let (e0, n0) = (em[0], nm[0]);
    let mut z = Value::Null;
    if let Some(g) = o.get("gradient").and_then(Value::as_array).filter(|g| g.len() >= 2) {
        let st = |p: &Value| p.get("station").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let (first, last) = (st(&g[0]), st(&g[g.len() - 1]));
        let mut hs = Vec::new();
        let mut i0: Option<usize> = None;
        for i in 0..e.len() {
            let s = s0 + i as f64 * step;
            if s < first - 1e-6 || s > last + 1e-6 {
                if i0.is_some() {
                    break;
                }
                continue;
            }
            i0.get_or_insert(i);
            hs.push(mm(gradient_at(g, s).unwrap_or(f64::NAN)));
        }
        if hs.len() >= 2 {
            let base = hs[0];
            z = json!({ "i0": i0, "z0": number(base), "dz": deltas(&hs.iter().map(|h| h - base).collect::<Vec<_>>()) });
        }
    }
    let height_epsg = if !z.is_null() && truthy(o.get("heightEpsg")) {
        number(js_number(o.get("heightEpsg")))
    } else {
        Value::Null
    };
    Ok(json!({
        "id": get(o, "id").cloned().unwrap_or_else(|| json!(generate_id())),
        "name": o.get("name"), "epsg": number(epsg), "heightEpsg": height_epsg,
        "source": { "tra": o.get("tra"), "gra": if z.is_null() { Value::Null } else { get(o, "gra").cloned().unwrap_or(Value::Null) }, "importedAt": o.get("importedAt") },
        "points": {
            "s0": number(js_round(s0 * 1000.0) / 1000.0), "step": number(step), "count": e.len(),
            "e0": number(e0 / 1000.0), "n0": number(n0 / 1000.0),
            "de": deltas(&em.iter().map(|v| v - e0).collect::<Vec<_>>()), "dn": deltas(&nm.iter().map(|v| v - n0).collect::<Vec<_>>()),
            "z": z,
        },
    }))
}

/// A reference axis decoded (`referencePoints`).
pub struct RefPoints {
    pub s0: f64,
    pub step: f64,
    pub count: usize,
    pub e: Vec<f64>,
    pub n: Vec<f64>,
    pub z: Option<Vec<f64>>,
}

pub fn reference_points(axis: &Value) -> RefPoints {
    let p = &axis["points"];
    let a = |k: &str| p.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let (de, dn) = (a("de"), a("dn"));
    let count = de.len();
    let f = |k: &str| p.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let (mut e, mut n) = (Vec::with_capacity(count), Vec::with_capacity(count));
    let (mut ce, mut cn) = (0.0, 0.0);
    for (i, d) in de.iter().enumerate() {
        ce += d.as_f64().unwrap_or(f64::NAN);
        cn += dn.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
        e.push(f("e0") + ce / 1000.0);
        n.push(f("n0") + cn / 1000.0);
    }
    let z = p.get("z").filter(|z| truthy(Some(z))).map(|zz| {
        let mut out = vec![f64::NAN; count];
        let i0 = zz.get("i0").and_then(Value::as_u64).unwrap_or(0) as usize;
        let z0 = zz.get("z0").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let mut cz = 0.0;
        for (k, d) in zz.get("dz").and_then(Value::as_array).into_iter().flatten().enumerate() {
            cz += d.as_f64().unwrap_or(f64::NAN);
            if let Some(slot) = out.get_mut(i0 + k) {
                *slot = (z0 + cz) / 1000.0;
            }
        }
        out
    });
    RefPoints {
        s0: f("s0"),
        step: f("step"),
        count,
        e,
        n,
        z,
    }
}

/// The stations an axis covers (`axisRange`).
pub fn axis_range(axis: &Value) -> (f64, f64) {
    let p = &axis["points"];
    let s0 = p.get("s0").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let step = p.get("step").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let len = p.get("de").and_then(Value::as_array).map(Vec::len).unwrap_or(0) as f64;
    (s0, s0 + (len - 1.0) * step)
}

/// The axis as a polyline every `every` metres (`axisOutline`).
pub fn axis_outline(axis: &Value, every: f64) -> Vec<Value> {
    let r = reference_points(axis);
    let stride = 1usize.max(js_round(every / r.step) as usize);
    let mut out = Vec::new();
    let mut i = 0;
    while i < r.count {
        out.push(json!([number(r.e[i]), number(r.n[i])]));
        i += stride;
    }
    if r.count > 0 && (r.count - 1) % stride != 0 {
        out.push(json!([number(r.e[r.count - 1]), number(r.n[r.count - 1])]));
    }
    out
}

/// What is wrong with a reference axis as kept, or None (`referenceAxisDefect`).
pub fn reference_axis_defect(axis: &Value) -> Option<&'static str> {
    if !truthy(axis.get("id")) {
        return Some("id");
    }
    if !js_number(axis.get("epsg")).is_finite() {
        return Some("epsg");
    }
    let Some(p) = axis.get("points").filter(|p| truthy(Some(p))) else {
        return Some("points");
    };
    let (Some(de), Some(dn)) = (p.get("de").and_then(Value::as_array), p.get("dn").and_then(Value::as_array)) else {
        return Some("points");
    };
    if de.len() != dn.len() || de.len() < 2 {
        return Some("points");
    }
    let f = |k: &str| p.get(k).and_then(Value::as_f64);
    if !f("step").is_some_and(|s| s > 0.0)
        || !f("s0").is_some_and(f64::is_finite)
        || !f("e0").is_some_and(f64::is_finite)
        || !f("n0").is_some_and(f64::is_finite)
    {
        return Some("points");
    }
    if (de.len() as f64 - 1.0) * f("step").unwrap_or(f64::NAN) > REFERENCE_MAX_LENGTH + 1.0 {
        return Some("length");
    }
    if let Some(z) = p.get("z").filter(|z| truthy(Some(z))) {
        let dz = z.get("dz").and_then(Value::as_array);
        let i0 = z.get("i0").and_then(Value::as_f64);
        if dz.is_none() || !i0.is_some_and(|i| i >= 0.0) || i0.unwrap_or(0.0) as usize + dz.map(Vec::len).unwrap_or(0) > de.len() {
            return Some("z");
        }
    }
    None
}
