// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Shift values: how far an axis lies from a reference axis, across and in
//! height, at round stations of the reference (`shiftValues.js`).

use std::collections::{HashMap, HashSet};

use rail_across_geom::element::element_points;
use rail_across_js::{get, js_number, js_round, js_to_fixed, number};
use rail_across_track::heights::gradient_at;
use rail_across_track::ids::DEFAULT_HEIGHT_EPSG;
use serde_json::{Value, json};

use crate::reference::{axis_range, reference_points};

const REACH: f64 = 2.0;
const LINE_STEP: f64 = 0.25;
const CELL: f64 = REACH;

/// The compared axis as a line (`comparedLine`'s answer).
pub struct Line {
    pub e: Vec<f64>,
    pub n: Vec<f64>,
    pub s: Vec<f64>,
    pub heights: Option<Vec<Value>>,
}

impl Line {
    pub fn from_json(v: &Value) -> Line {
        let col = |k: &str| {
            v.get(k)
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect())
                .unwrap_or_default()
        };
        Line {
            e: col("e"),
            n: col("n"),
            s: col("s"),
            heights: v.get("heights").and_then(Value::as_array).cloned(),
        }
    }
    pub fn to_json(&self) -> Value {
        let col = |a: &[f64]| Value::Array(a.iter().map(|x| number(*x)).collect());
        json!({ "e": col(&self.e), "n": col(&self.n), "s": col(&self.s), "heights": self.heights.clone().map(Value::Array).unwrap_or(Value::Null) })
    }
}

/// The compared axis as the shift values read it (`comparedLine`).
pub fn compared_line(elements: &[Value], epsg: i64, heights: Option<&Value>, station0: f64) -> Line {
    let (mut e, mut n, mut s) = (Vec::new(), Vec::new(), Vec::new());
    let mut start = station0;
    for el in elements {
        let len = get(el, "length").and_then(Value::as_f64).unwrap_or(0.0);
        if !(len > 0.0) {
            continue;
        }
        let pts = element_points(el, epsg, LINE_STEP).unwrap_or_default();
        let m = pts.len().saturating_sub(1);
        for (k, p) in pts.iter().enumerate().skip(if e.is_empty() { 0 } else { 1 }) {
            e.push(p[0]);
            n.push(p[1]);
            s.push(start + len * k as f64 / m as f64);
        }
        start += len;
    }
    let heights = heights.and_then(Value::as_array).filter(|h| h.len() >= 2).cloned();
    Line { e, n, s, heights }
}

fn segment_index(line: &Line) -> HashMap<(i64, i64), Vec<usize>> {
    let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for k in 0..line.e.len().saturating_sub(1) {
        let c = |v: f64| (v / CELL).floor() as i64;
        let (x0, x1) = (c(line.e[k].min(line.e[k + 1])), c(line.e[k].max(line.e[k + 1])));
        let (y0, y1) = (c(line.n[k].min(line.n[k + 1])), c(line.n[k].max(line.n[k + 1])));
        for x in x0..=x1 {
            for y in y0..=y1 {
                cells.entry((x, y)).or_default().push(k);
            }
        }
    }
    cells
}

/// Whether the reference axis and the compared gradient share a height system (`heightsComparable`).
pub fn heights_comparable(axis: &Value, height_epsg: Option<&Value>) -> bool {
    let has_z = rail_across_js::truthy(axis.get("points").and_then(|p| p.get("z")));
    let a = js_number(Some(get(axis, "heightEpsg").unwrap_or(&json!(DEFAULT_HEIGHT_EPSG))));
    let b = js_number(Some(height_epsg.filter(|v| !v.is_null()).unwrap_or(&json!(DEFAULT_HEIGHT_EPSG))));
    has_z && a == b
}

/// The shift values of a line against a reference axis (`shiftValues`).
pub fn shift_values(axis: &Value, line: &Line, every: f64, with_heights: bool) -> Vec<Value> {
    if axis.is_null() || line.e.len() < 2 {
        return vec![];
    }
    let r = reference_points(axis);
    let (from, to) = axis_range(axis);
    let (mut e0, mut e1, mut n0, mut n1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for k in 0..line.e.len() {
        e0 = e0.min(line.e[k]);
        e1 = e1.max(line.e[k]);
        n0 = n0.min(line.n[k]);
        n1 = n1.max(line.n[k]);
    }
    e0 -= REACH;
    e1 += REACH;
    n0 -= REACH;
    n1 += REACH;
    let cells = segment_index(line);
    let mut rows = Vec::new();
    let mut st = (from / every - 1e-9).ceil() * every;
    while st <= to + 1e-9 {
        let here = st;
        st += every;
        let i = js_round((here - r.s0) / r.step);
        if i < 0.0 || i >= r.count as f64 {
            continue;
        }
        let i = i as usize;
        let (pe, pn) = (r.e[i], r.n[i]);
        if pe < e0 || pe > e1 || pn < n0 || pn > n1 {
            continue;
        }
        let (a, b) = (i.saturating_sub(1), (r.count - 1).min(i + 1));
        let (mut te, mut tn) = (r.e[b] - r.e[a], r.n[b] - r.n[a]);
        let tl = te.hypot(tn);
        if !(tl > 0.0) {
            continue;
        }
        te /= tl;
        tn /= tl;
        let (re, rn) = (tn, -te);
        let (cx, cy) = ((pe / CELL).floor() as i64, (pn / CELL).floor() as i64);
        let mut best: Option<(f64, usize, f64)> = None;
        let mut seen = HashSet::new();
        for x in cx - 1..=cx + 1 {
            for y in cy - 1..=cy + 1 {
                for &k in cells.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[]) {
                    if !seen.insert(k) {
                        continue;
                    }
                    let (ae, an) = (line.e[k], line.n[k]);
                    let (de, dn) = (line.e[k + 1] - ae, line.n[k + 1] - an);
                    let den = re * dn - rn * de;
                    if den.abs() < 1e-12 {
                        continue;
                    }
                    let (qe, qn) = (ae - pe, an - pn);
                    let u = (qe * dn - qn * de) / den;
                    let lam = (qe * rn - qn * re) / den;
                    if lam < -1e-9 || lam > 1.0 + 1e-9 || u.abs() > REACH {
                        continue;
                    }
                    if best.is_none_or(|b| u.abs() < b.0.abs()) {
                        best = Some((u, k, lam));
                    }
                }
            }
        }
        let Some((u, k, lam)) = best else { continue };
        let along = line.s[k] + (line.s[k + 1] - line.s[k]) * lam;
        let z_ref = r.z.as_ref().map(|z| z[i]).filter(|z| z.is_finite());
        let mut dz = Value::Null;
        if let (true, Some(h), Some(zr)) = (with_heights, &line.heights, z_ref)
            && let Some(zc) = gradient_at(h, along)
        {
            dz = number(zc - zr);
        }
        rows.push(json!({
            "station": number(js_round(here * 1000.0) / 1000.0), "dq": number(u), "dz": dz, "zRef": z_ref.map(number),
            "at": [number(pe), number(pn)], "hit": [number(pe + u * re), number(pn + u * rn)], "along": number(along),
        }));
    }
    rows
}

fn fv(r: &Value, k: &str) -> Option<f64> {
    r.get(k).and_then(Value::as_f64)
}

/// The reference axis' gradient over the compared line's stationing (`referenceProfile`).
pub fn reference_profile(rows: &[Value], heights: Option<&Value>, every: f64) -> Vec<Vec<Value>> {
    let heights = heights.and_then(Value::as_array).filter(|h| h.len() >= 2);
    let mut pts: Vec<(f64, f64, Option<f64>)> = rows
        .iter()
        .filter_map(|r| {
            let z = fv(r, "zRef")?;
            let along = fv(r, "along").unwrap_or(f64::NAN);
            let zc = heights.and_then(|h| gradient_at(h, along));
            Some((along, z, zc.map(|zc| zc - z)))
        })
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut runs: Vec<Vec<(f64, f64, Option<f64>)>> = Vec::new();
    for p in pts {
        match runs.last_mut() {
            Some(run) if !(p.0 - run[run.len() - 1].0 > 2.5 * every) => run.push(p),
            _ => runs.push(vec![p]),
        }
    }
    runs.into_iter()
        .map(|run| {
            run.into_iter()
                .map(|(s, z, dz)| json!({ "s": number(s), "z": number(z), "dz": dz.map(number) }))
                .collect()
        })
        .collect()
}

/// The lift at station `s` read off the runs (`deviationAt`).
pub fn deviation_at(runs: &[Value], s: f64) -> Option<f64> {
    for run in runs {
        let Some(run) = run.as_array().filter(|r| !r.is_empty()) else { continue };
        let ps = |p: &Value| fv(p, "s").unwrap_or(f64::NAN);
        if s < ps(&run[0]) || s > ps(&run[run.len() - 1]) {
            continue;
        }
        let k = run.iter().position(|p| ps(p) >= s)?;
        let (b, a) = (&run[k], &run[k.saturating_sub(1)]);
        let (adz, bdz) = (fv(a, "dz")?, fv(b, "dz")?);
        return Some(if ps(b) > ps(a) {
            adz + (bdz - adz) * (s - ps(a)) / (ps(b) - ps(a))
        } else {
            bdz
        });
    }
    None
}

/// What a table of shift values comes to (`shiftSummary`).
pub fn shift_summary(rows: &[Value], limit_q: f64, limit_z: f64) -> Value {
    let pick = |key: &str, better: fn(f64, f64) -> bool| {
        let mut acc: Option<&Value> = None;
        for r in rows {
            if let Some(v) = fv(r, key)
                && better(v, acc.and_then(|a| fv(a, key)).unwrap_or(0.0))
            {
                acc = Some(r);
            }
        }
        acc.cloned().unwrap_or(Value::Null)
    };
    let dq = |r: &Value| js_number(r.get("dq"));
    json!({
        "right": pick("dq", |v, w| v > w), "left": pick("dq", |v, w| v < w),
        "up": pick("dz", |v, w| v > w), "down": pick("dz", |v, w| v < w),
        "beyondQ": rows.iter().filter(|r| dq(r).abs() > limit_q + 1e-9).count(),
        "beyondZ": rows.iter().filter(|r| fv(r, "dz").is_some_and(|d| d.abs() > limit_z + 1e-9)).count(),
        "heights": rows.iter().any(|r| fv(r, "dz").is_some()),
    })
}

/// The reference axis a line runs along the furthest (`nearestAxis`).
pub fn nearest_axis<'a>(axes: &'a [Value], line: &Line, epsg: f64) -> Option<&'a Value> {
    let (mut best, mut hits) = (None, 0);
    for axis in axes {
        if js_number(axis.get("epsg")) != epsg {
            continue;
        }
        let n = shift_values(axis, line, 10.0, false).len();
        if n > hits {
            best = Some(axis);
            hits = n;
        }
    }
    best
}

/// The rows as a CSV file's text (`shiftValuesCsv`).
pub fn shift_values_csv(rows: &[Value], axis_name: &str, line_name: &str) -> String {
    let mm = |v: Option<f64>| v.map(|v| rail_across_js::es_number(js_round(v * 1000.0))).unwrap_or_default();
    let fx = |v: Option<&Value>| js_to_fixed(js_number(v), 3);
    let mut out = vec![
        format!("# Verschiebewerte {line_name} gegen Bestandsachse {axis_name}; quer rechts +, Höhe oben + [mm]"),
        "Station;Quer;Hoehe;Rechtswert_Bestand;Hochwert_Bestand;Rechtswert_neu;Hochwert_neu".to_string(),
    ];
    for r in rows {
        out.push(
            [
                fx(r.get("station")),
                mm(fv(r, "dq")),
                mm(fv(r, "dz")),
                fx(r["at"].get(0)),
                fx(r["at"].get(1)),
                fx(r["hit"].get(0)),
                fx(r["hit"].get(1)),
            ]
            .join(";"),
        );
    }
    out.join("\n") + "\n"
}
