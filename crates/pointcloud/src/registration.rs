// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Re-referencing a point cloud (`registration.js`, decisions 211–214): the
//! spatial Helmert transformation T(p) = c + t + s·R·(p − c) found by least
//! squares from pairs picked in both clouds — 3D pairs with three equations,
//! section pairs with two (across and height).

use std::collections::HashMap;

use rail_across_js::js_hypot;
use serde_json::{Map, Value, json};

const PARAMETERS: [&str; 7] = ["tE", "tN", "tH", "kappa", "omega", "phi", "scale"];
const SUSPECT_W: f64 = 3.29;
const RANK_TOL: f64 = 1e-9;
const MAX_ITER: usize = 30;

/// A matrix that changes nothing.
pub const IDENTITY: [f64; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

fn rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

fn mul3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            out[3 * i + j] = a[3 * i] * b[j] + a[3 * i + 1] * b[3 + j] + a[3 * i + 2] * b[6 + j];
        }
    }
    out
}

fn rotation(kappa: f64, omega: f64, phi: f64) -> [f64; 9] {
    let (ck, sk, co, so, cp, sp) = (kappa.cos(), kappa.sin(), omega.cos(), omega.sin(), phi.cos(), phi.sin());
    let rz = [ck, -sk, 0.0, sk, ck, 0.0, 0.0, 0.0, 1.0];
    let ry = [cp, 0.0, sp, 0.0, 1.0, 0.0, -sp, 0.0, cp];
    let rx = [1.0, 0.0, 0.0, 0.0, co, -so, 0.0, so, co];
    mul3(&rz, &mul3(&ry, &rx))
}

pub type Params = HashMap<&'static str, f64>;

fn p(params: &Params, k: &str) -> f64 {
    params.get(k).copied().unwrap_or(0.0)
}

/// The 4 × 4 matrix of T for `params` about centre `c` (`transformMatrix`).
pub fn transform_matrix(params: &Params, c: &[f64; 3]) -> [f64; 16] {
    let s = 1.0 + p(params, "scale");
    let r = rotation(p(params, "kappa"), p(params, "omega"), p(params, "phi")).map(|v| v * s);
    let t3 = [p(params, "tE"), p(params, "tN"), p(params, "tH")];
    let t = [0, 1, 2].map(|i| c[i] + t3[i] - (r[3 * i] * c[0] + r[3 * i + 1] * c[1] + r[3 * i + 2] * c[2]));
    [r[0], r[1], r[2], t[0], r[3], r[4], r[5], t[1], r[6], r[7], r[8], t[2], 0.0, 0.0, 0.0, 1.0]
}

/// A point through a 4 × 4 matrix (`applyMatrix`).
pub fn apply_matrix(m: &[f64; 16], [e, n, h]: [f64; 3]) -> [f64; 3] {
    [
        m[0] * e + m[1] * n + m[2] * h + m[3],
        m[4] * e + m[5] * n + m[6] * h + m[7],
        m[8] * e + m[9] * n + m[10] * h + m[11],
    ]
}

/// The inverse of a similarity matrix (`invertMatrix`).
pub fn invert_matrix(m: &[f64; 16]) -> [f64; 16] {
    let s2 = m[0] * m[0] + m[4] * m[4] + m[8] * m[8];
    let r = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]].map(|v| v / s2);
    let t = [0, 1, 2].map(|i| -(r[3 * i] * m[3] + r[3 * i + 1] * m[7] + r[3 * i + 2] * m[11]));
    [r[0], r[1], r[2], t[0], r[3], r[4], r[5], t[1], r[6], r[7], r[8], t[2], 0.0, 0.0, 0.0, 1.0]
}

/// First `a`, then `b` (`chainMatrices`).
pub fn chain_matrices(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let mut out = [0.0; 16];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                out[4 * i + j] += b[4 * i + k] * a[4 * k + j];
            }
        }
    }
    out
}

fn frame(bearing: f64) -> ([f64; 2], [f64; 2]) {
    let b = rad(bearing);
    ([b.sin(), b.cos()], [b.cos(), -b.sin()])
}

/// The bearing of the line the points lie on (`mainBearing`).
pub fn main_bearing(points: &[[f64; 3]]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let n = points.len() as f64;
    let me = points.iter().fold(0.0, |a, p| a + p[0]) / n;
    let mn = points.iter().fold(0.0, |a, p| a + p[1]) / n;
    let (mut see, mut snn, mut sen) = (0.0, 0.0, 0.0);
    for p in points {
        see += (p[0] - me).powi(2);
        snn += (p[1] - mn).powi(2);
        sen += (p[0] - me) * (p[1] - mn);
    }
    let theta = 0.5 * (2.0 * sen).atan2(see - snn);
    ((90.0 - theta * 180.0 / std::f64::consts::PI) % 180.0 + 180.0) % 180.0
}

/// One pair of points picked in both clouds.
#[derive(Clone, Debug)]
pub struct Pair {
    pub id: Value,
    pub section: bool,
    pub reference: [f64; 3],
    pub src: [f64; 3],
    pub bearing: Option<f64>,
}

fn helmert2d(pairs: &[Pair]) -> f64 {
    let n = pairs.len() as f64;
    let cs = [0, 1].map(|i| pairs.iter().fold(0.0, |a, p| a + p.src[i]) / n);
    let cr = [0, 1].map(|i| pairs.iter().fold(0.0, |a, p| a + p.reference[i]) / n);
    let (mut a, mut b) = (0.0, 0.0);
    for p in pairs {
        let (x, y) = (p.src[0] - cs[0], p.src[1] - cs[1]);
        let (u, v) = (p.reference[0] - cr[0], p.reference[1] - cr[1]);
        a += x * u + y * v;
        b += x * v - y * u;
    }
    b.atan2(a)
}

/// Eigenvalues and vectors of a symmetric matrix (`symmetricEigen`, Jacobi).
pub fn symmetric_eigen(a: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    let mut v: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    for _ in 0..100 {
        let mut off = 0.0;
        for i in 0..n {
            for j in i + 1..n {
                off += m[i][j].powi(2);
            }
        }
        if off < 1e-30 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if m[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
                let sign = if theta == 0.0 || theta.is_nan() { 1.0 } else { theta.signum() };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (mkp, mkq) = (m[k][p], m[k][q]);
                    m[k][p] = c * mkp - s * mkq;
                    m[k][q] = s * mkp + c * mkq;
                }
                for k in 0..n {
                    let (mpk, mqk) = (m[p][k], m[q][k]);
                    m[p][k] = c * mpk - s * mqk;
                    m[q][k] = s * mpk + c * mqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k][p], v[k][q]);
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| m[i][i]).collect(), v)
}

fn inverse_from_eigen(values: &[f64], vectors: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = values.len();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let mut s = 0.0;
                    for k in 0..n {
                        s += vectors[i][k] * vectors[j][k] / values[k];
                    }
                    s
                })
                .collect()
        })
        .collect()
}

struct Row {
    pair: usize,
    part: u8,
    dir: [f64; 2],
}

fn equations(pairs: &[Pair]) -> Vec<Row> {
    let mut rows = Vec::new();
    for (k, p) in pairs.iter().enumerate() {
        if p.section {
            rows.push(Row {
                pair: k,
                part: b'a',
                dir: frame(p.bearing.unwrap_or(f64::NAN)).1,
            });
            rows.push(Row {
                pair: k,
                part: b'h',
                dir: [0.0; 2],
            });
        } else {
            for part in [b'e', b'n', b'h'] {
                rows.push(Row { pair: k, part, dir: [0.0; 2] });
            }
        }
    }
    rows
}

fn misclosures(pairs: &[Pair], rows: &[Row], centre: &[f64; 3], params: &Params) -> Vec<f64> {
    let m = transform_matrix(params, centre);
    let moved: Vec<[f64; 3]> = pairs.iter().map(|p| apply_matrix(&m, p.src)).collect();
    rows.iter()
        .map(|r| {
            let (q, pr) = (moved[r.pair], pairs[r.pair].reference);
            let d = [q[0] - pr[0], q[1] - pr[1], q[2] - pr[2]];
            match r.part {
                b'a' => d[0] * r.dir[0] + d[1] * r.dir[1],
                b'e' => d[0],
                b'n' => d[1],
                _ => d[2],
            }
        })
        .collect()
}

fn step_of(name: &str) -> f64 {
    if name.starts_with('t') { 1e-4 } else { 1e-8 }
}

fn jacobian(pairs: &[Pair], rows: &[Row], centre: &[f64; 3], params: &Params, names: &[&'static str]) -> Vec<Vec<f64>> {
    let cols: Vec<Vec<f64>> = names
        .iter()
        .map(|&name| {
            let h = step_of(name);
            let mut up = params.clone();
            up.insert(name, p(params, name) + h);
            let mut down = params.clone();
            down.insert(name, p(params, name) - h);
            let (u, d) = (misclosures(pairs, rows, centre, &up), misclosures(pairs, rows, centre, &down));
            u.iter().zip(d).map(|(u, d)| (u - d) / (2.0 * h)).collect()
        })
        .collect();
    (0..rows.len()).map(|k| cols.iter().map(|c| c[k]).collect()).collect()
}

fn undetermined(vector: &[f64], names: &[&'static str], along: [f64; 2]) -> &'static str {
    let w = |n: &str| names.iter().position(|x| *x == n).map(|i| vector[i]).unwrap_or(0.0);
    let horizontal = js_hypot(&[w("tE"), w("tN")]);
    let groups = [
        ("shift", horizontal),
        ("height", w("tH").abs()),
        ("rotation", w("kappa").abs()),
        ("tilt", js_hypot(&[w("omega"), w("phi")])),
        ("scale", w("scale").abs()),
    ];
    let mut top = groups[0];
    for g in &groups[1..] {
        if g.1 > top.1 {
            top = *g;
        }
    }
    if top.0 != "shift" {
        return top.0;
    }
    let share = (w("tE") * along[0] + w("tN") * along[1]).abs() / if horizontal == 0.0 || horizontal.is_nan() { 1.0 } else { horizontal };
    if share > 0.7 { "along" } else { "across" }
}

fn js_max(v: impl Iterator<Item = f64>) -> f64 {
    v.fold(f64::NEG_INFINITY, |a, x| if x.is_nan() || a.is_nan() { f64::NAN } else { a.max(x) })
}

/// The adjustment (`solveRegistration`). `pairs` as the JavaScript takes them;
/// `options`: `tilts`, `scale`, `start`.
pub fn solve_registration(all: &[Value], options: &Value) -> Value {
    let tilts = rail_across_js::truthy(options.get("tilts"));
    let with_scale = rail_across_js::truthy(options.get("scale"));
    let three = |v: Option<&Value>| -> [f64; 3] {
        let a = v.and_then(Value::as_array);
        [0, 1, 2].map(|i| a.and_then(|a| a.get(i)).and_then(Value::as_f64).unwrap_or(f64::NAN))
    };
    let pairs: Vec<Pair> = all
        .iter()
        .filter(|p| p.get("on") != Some(&json!(false)))
        .map(|p| Pair {
            id: p.get("id").cloned().unwrap_or(Value::Null),
            section: p.get("kind").and_then(Value::as_str) == Some("section"),
            reference: three(p.get("ref")),
            src: three(p.get("src")),
            bearing: p.get("bearing").and_then(Value::as_f64),
        })
        .collect();
    let names: Vec<&'static str> = PARAMETERS
        .iter()
        .copied()
        .filter(|&n| match n {
            "omega" | "phi" => tilts,
            "scale" => with_scale,
            _ => true,
        })
        .collect();
    let n_eq: usize = pairs.iter().map(|p| if p.section { 2 } else { 3 }).sum();
    let base = |mut o: Map<String, Value>| {
        o.insert("equations".into(), json!(n_eq));
        o.insert("unknowns".into(), json!(names.len()));
        Value::Object(o)
    };
    let fail = |reason: &str| {
        let mut o = Map::new();
        o.insert("ok".into(), json!(false));
        o.insert("reason".into(), json!(reason));
        base(o)
    };
    if pairs.is_empty() {
        return fail("no_pairs");
    }
    if n_eq < names.len() {
        return fail("too_few");
    }
    let np = pairs.len() as f64;
    let centre = [0, 1, 2].map(|i| pairs.iter().fold(0.0, |a, p| a + p.src[i]) / np);
    let ref_centre = [0, 1, 2].map(|i| pairs.iter().fold(0.0, |a, p| a + p.reference[i]) / np);
    let refs: Vec<[f64; 3]> = pairs.iter().map(|p| p.reference).collect();
    let line_bearing = main_bearing(&refs);
    let (along, _) = frame(line_bearing);
    let radius = (pairs.iter().fold(0.0, |a, p| {
        a + (p.src[0] - centre[0]).powi(2) + (p.src[1] - centre[1]).powi(2) + (p.src[2] - centre[2]).powi(2)
    }) / np)
        .sqrt()
        .max(1.0);
    let unit = |name: &str| if name.starts_with('t') { 1.0 } else { 1.0 / radius };
    let rows = equations(&pairs);
    let kappa0 = if pairs.len() >= 2 && radius > 1.0 { helmert2d(&pairs) } else { 0.0 };
    let mut params: Params = HashMap::from([
        ("tE", ref_centre[0] - centre[0]),
        ("tN", ref_centre[1] - centre[1]),
        ("tH", ref_centre[2] - centre[2]),
        ("kappa", kappa0),
        ("omega", 0.0),
        ("phi", 0.0),
        ("scale", 0.0),
    ]);
    if !pairs.iter().any(|p| !p.section) {
        let m: [f64; 16] = options
            .get("start")
            .and_then(Value::as_array)
            .filter(|a| a.len() >= 16)
            .map(|a| std::array::from_fn(|i| a[i].as_f64().unwrap_or(f64::NAN)))
            .unwrap_or(IDENTITY);
        let moved = apply_matrix(&m, centre);
        params.insert("tE", moved[0] - centre[0]);
        params.insert("tN", moved[1] - centre[1]);
        params.insert("tH", moved[2] - centre[2]);
        params.insert("kappa", m[4].atan2(m[0]));
    }
    let u = names.len();
    for _ in 0..MAX_ITER {
        let j = jacobian(&pairs, &rows, &centre, &params, &names);
        let v = misclosures(&pairs, &rows, &centre, &params);
        let js: Vec<Vec<f64>> = j.iter().map(|r| r.iter().enumerate().map(|(i, x)| x * unit(names[i])).collect()).collect();
        let nm: Vec<Vec<f64>> = (0..u).map(|i| (0..u).map(|k| js.iter().fold(0.0, |a, r| a + r[i] * r[k])).collect()).collect();
        let g: Vec<f64> = (0..u).map(|i| js.iter().enumerate().fold(0.0, |a, (k, r)| a + r[i] * v[k])).collect();
        let (values, vectors) = symmetric_eigen(&nm);
        let lmax = js_max(values.iter().copied());
        let lmin = values.iter().copied().fold(f64::INFINITY, f64::min);
        let kmin = values.iter().position(|x| *x == lmin).unwrap_or(0);
        if !(values[kmin] > lmax * RANK_TOL) {
            let vec: Vec<f64> = vectors.iter().map(|r| r[kmin]).collect();
            return fail(undetermined(&vec, &names, along));
        }
        let ninv = inverse_from_eigen(&values, &vectors);
        let dxs: Vec<f64> = ninv.iter().map(|r| -r.iter().zip(&g).fold(0.0, |a, (x, gj)| a + x * gj)).collect();
        for (i, name) in names.iter().enumerate() {
            let d = dxs[i] * unit(name);
            params.insert(name, p(&params, name) + d);
        }
        if js_max(dxs.iter().map(|d| d.abs())) < 1e-10 {
            break;
        }
    }
    let v = misclosures(&pairs, &rows, &centre, &params);
    let j = jacobian(&pairs, &rows, &centre, &params, &names);
    let nm: Vec<Vec<f64>> = (0..u).map(|i| (0..u).map(|k| j.iter().fold(0.0, |a, r| a + r[i] * r[k])).collect()).collect();
    let (values, vectors) = symmetric_eigen(&nm);
    let qxx = inverse_from_eigen(&values, &vectors);
    let redundancy = n_eq as i64 - u as i64;
    let vv = v.iter().fold(0.0, |a, x| a + x * x);
    let sigma0 = (redundancy > 0).then(|| (vv / redundancy as f64).sqrt());
    let n = rail_across_js::number;
    let mut sigmas = Map::new();
    for (i, name) in names.iter().enumerate() {
        sigmas.insert(name.to_string(), sigma0.map(|s| n(s * qxx[i][i].max(0.0).sqrt())).unwrap_or(Value::Null));
    }
    let w: Vec<Option<f64>> = j
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let mut q = 0.0;
            for i in 0..u {
                for jj in 0..u {
                    q += r[i] * qxx[i][jj] * r[jj];
                }
            }
            let qvv = 1.0 - q;
            match sigma0 {
                Some(s) if s != 0.0 && !s.is_nan() && qvv > 1e-9 => Some(v[k] / (s * qvv.sqrt())),
                _ => None,
            }
        })
        .collect();
    let matrix = transform_matrix(&params, &centre);
    let mut sizes = Vec::new();
    let residuals: Vec<Value> = pairs
        .iter()
        .enumerate()
        .map(|(k, pr)| {
            let q = apply_matrix(&matrix, pr.src);
            let d = [q[0] - pr.reference[0], q[1] - pr.reference[1], q[2] - pr.reference[2]];
            let (fa, fc) = frame(pr.bearing.unwrap_or(line_bearing));
            let ws: Vec<Option<f64>> = rows.iter().zip(&w).filter(|(r, _)| r.pair == k).map(|(_, w)| *w).collect();
            let w_max = if ws.iter().any(Option::is_none) && ws.iter().all(Option::is_none) {
                None
            } else {
                Some(js_max(ws.iter().flatten().map(|x| x.abs())))
            };
            let across = d[0] * fc[0] + d[1] * fc[1];
            let along_v = (!pr.section).then(|| d[0] * fa[0] + d[1] * fa[1]);
            sizes.push(js_hypot(&[across, along_v.unwrap_or(0.0), d[2]]));
            json!({
                "id": pr.id,
                "across": n(across),
                "along": along_v.map(n),
                "height": n(d[2]),
                "w": w_max.map(n),
                "suspect": w_max.is_some_and(|w| w > SUSPECT_W),
            })
        })
        .collect();
    let mut all_params = Map::new();
    for name in PARAMETERS {
        all_params.insert(name.into(), n(if names.contains(&name) { p(&params, name) } else { 0.0 }));
    }
    json!({
        "ok": true,
        "params": all_params,
        "estimated": names,
        "sigmas": sigmas,
        "centre": centre.map(n),
        "matrix": matrix.map(n),
        "sigma0": sigma0.map(n),
        "rms": n((vv / n_eq as f64).sqrt()),
        "max": n(js_max(sizes.into_iter())),
        "equations": n_eq,
        "unknowns": u,
        "redundancy": redundancy,
        "residuals": residuals,
    })
}
