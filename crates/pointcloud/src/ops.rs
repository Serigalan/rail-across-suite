// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The crate's functions by name, JSON in and out — what the replay holds to
//! the JavaScript and the command line (`ra-pointcloud`) answers. Files go in
//! as base64; clouds are imported into memory at level 1 first.

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::clearance::{check_clearance, ring_of, scan_clearance, stretches_of};
use crate::crs::{PlaneMapper, Transform};
use crate::e57_xml::parse_xml;
use crate::import::{Header, MemoryWriter, Writer, import_levels, read_cloud_header, read_cloud_points, read_first_points};
use crate::las::parse_las_header;
use crate::rail_detect::{detect_track, edge_candidates, top_surface};
use crate::rail_trace::{Guide, Point, axis_points_csv, cant_mm, detect_in_clouds, foresee, gaps_of, so_height, trace_track};
use crate::registration as reg;
use crate::section::{Cloud, cloud_section_points};
use crate::slice::{slice_frame, tiles_in_slice};
use crate::source::{BytesSource, TileBytes};
use crate::tiles::{Grid, decode_grid_segment, decode_segment};
use crate::{Batch, Error, ReadOptions, Result};

fn n(x: f64) -> Value {
    rail_across_js::number(x)
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn arr<const N: usize>(v: &Value) -> [f64; N] {
    std::array::from_fn(|i| v.get(i).map(f).unwrap_or(f64::NAN))
}

/// The JavaScript tests' noise source.
fn noise(seed: f64) -> impl FnMut(f64) -> f64 {
    let mut s = seed;
    move |amp| {
        s = (s * 16807.0) % 2147483647.0;
        ((s / 2147483647.0) * 2.0 - 1.0) * amp
    }
}

struct Made {
    x: f64,
    y: f64,
    z: f64,
    intensity: f64,
    rgb: Option<[f64; 3]>,
}

/// A plain LAS 1.2 file of `points` (`makeLas` of the package's tests).
fn make_las(points: &[Made], offset: [f64; 3], scale: [f64; 3], format: u8) -> Vec<u8> {
    const HEADER: usize = 227;
    let len = if format == 3 { 34 } else { 28 };
    let mut b = vec![0u8; HEADER + points.len() * len];
    b[0..4].copy_from_slice(b"LASF");
    b[24] = 1;
    b[25] = 2;
    b[94..96].copy_from_slice(&(HEADER as u16).to_le_bytes());
    b[96..100].copy_from_slice(&(HEADER as u32).to_le_bytes());
    b[104] = format;
    b[105..107].copy_from_slice(&(len as u16).to_le_bytes());
    b[107..111].copy_from_slice(&(points.len() as u32).to_le_bytes());
    for i in 0..3 {
        b[131 + 8 * i..139 + 8 * i].copy_from_slice(&scale[i].to_le_bytes());
        b[155 + 8 * i..163 + 8 * i].copy_from_slice(&offset[i].to_le_bytes());
        let pick = |p: &Made| [p.x, p.y, p.z][i];
        let (hi, lo) = if points.is_empty() {
            (0.0, 0.0)
        } else {
            points
                .iter()
                .fold((f64::NEG_INFINITY, f64::INFINITY), |(hi, lo), p| (hi.max(pick(p)), lo.min(pick(p))))
        };
        b[179 + 16 * i..187 + 16 * i].copy_from_slice(&hi.to_le_bytes());
        b[187 + 16 * i..195 + 16 * i].copy_from_slice(&lo.to_le_bytes());
    }
    let int32 = |v: f64| rail_across_js::js_round(v) as i64 as i32;
    for (k, p) in points.iter().enumerate() {
        let at = HEADER + k * len;
        for (j, (v, (o, sc))) in [p.x, p.y, p.z].iter().zip(offset.iter().zip(scale)).enumerate() {
            b[at + 4 * j..at + 4 * j + 4].copy_from_slice(&int32((v - o) / sc).to_le_bytes());
        }
        b[at + 12..at + 14].copy_from_slice(&(p.intensity as i64 as u16).to_le_bytes());
        if format == 3 {
            let c = p.rgb.unwrap_or([0.0; 3]);
            for j in 0..3 {
                b[at + 28 + 2 * j..at + 30 + 2 * j].copy_from_slice(&(c[j] as i64 as u16).to_le_bytes());
            }
        }
    }
    b
}

fn scatter(n: usize, seed: f64, rgb: bool) -> Vec<Made> {
    let mut rnd = noise(seed);
    let (base, span) = ([500000.0, 5700000.0, 100.0], [30.0, 20.0, 5.0]);
    (0..n)
        .map(|_| {
            let x = base[0] + span[0] / 2.0 + rnd(span[0] / 2.0);
            let y = base[1] + span[1] / 2.0 + rnd(span[1] / 2.0);
            let z = base[2] + span[2] / 2.0 + rnd(span[2] / 2.0);
            let intensity = rnd(4000.0).abs().floor();
            let rgb = rgb.then(|| {
                let r = rnd(65535.0).abs().floor();
                let g = rnd(65535.0).abs().floor();
                [r, g, rnd(255.0).abs().floor()]
            });
            Made { x, y, z, intensity, rgb }
        })
        .collect()
}

/// The synthetic track scene of the recorder: rail heads with their inner
/// flanks, sleepers and ballast, a mast in the clearance outline at 7 m.
fn track_scene() -> Vec<Made> {
    let mut rnd = noise(11.0);
    let (start, length, z) = ([500100.0, 5700100.0], 12.0, 100.0);
    let b = 60.0 * std::f64::consts::PI / 180.0;
    let (along, right) = ([b.sin(), b.cos()], [b.cos(), -b.sin()]);
    let mut pts = Vec::new();
    let mut at = |s: f64, q: f64, h: f64, intensity: f64| {
        let x = start[0] + along[0] * s + right[0] * q + rnd(0.001);
        let y = start[1] + along[1] * s + right[1] * q + rnd(0.001);
        let zz = h + rnd(0.001);
        pts.push(Made {
            x,
            y,
            z: zz,
            intensity,
            rgb: None,
        });
    };
    let sleeper = z - 0.17;
    let mut s = 0.0;
    while s <= length {
        for side in [-1.0, 1.0] {
            let inner = side * 0.7175;
            let centre = inner + side * 0.0335;
            let mut q = centre - 0.0335;
            while q <= centre + 0.0335 + 1e-9 {
                at(s, q, z, 3000.0);
                q += 0.01;
            }
            let mut dz = 0.005;
            while dz <= 0.05 {
                at(s, inner, z - dz, 2500.0);
                dz += 0.004;
            }
        }
        s += 0.02;
    }
    let mut s = 0.0;
    while s <= length {
        let mut q = -2.5;
        while q <= 2.5 {
            let d = f64::abs(q);
            at(s, q, if d < 1.3 { sleeper } else { sleeper - 0.05 - (d - 1.3) * 0.3 }, 1200.0);
            q += 0.08;
        }
        s += 0.08;
    }
    let mut h = 0.0;
    while h < 5.0 {
        at(7.0, 2.0, z + h, 4000.0);
        h += 0.05;
    }
    pts
}

/// A file made from its description (`made` of the recorder).
fn made(spec: &Value) -> Vec<u8> {
    let three = |k: &str, d: [f64; 3]| spec.get(k).filter(|v| v.is_array()).map(arr::<3>).unwrap_or(d);
    let (offset, scale) = (three("offset", [0.0; 3]), three("scale", [0.001; 3]));
    let format = spec.get("format").and_then(Value::as_u64).unwrap_or(1) as u8;
    if spec["make"] == "track" {
        return make_las(&track_scene(), offset, scale, format);
    }
    let mut pts = scatter(
        spec["n"].as_u64().unwrap_or(0) as usize,
        f(&spec["seed"]),
        rail_across_js::truthy(spec.get("rgb")),
    );
    if rail_across_js::truthy(spec.get("dark")) {
        for p in &mut pts {
            if let Some(c) = p.rgb.as_mut() {
                *c = [f64::from(c[0] as u32 & 0xFF), f64::from(c[1] as u32 & 0xFF), 0.0];
            }
        }
    }
    make_las(&pts, offset, scale, format)
}

/// A file: base64, made from a description, or `{ path }` relative to the working directory.
fn file(v: &Value) -> Result<BytesSource> {
    if v.get("make").is_some() {
        return Ok(BytesSource(made(v)));
    }
    if let Some(path) = v.get("path").and_then(Value::as_str) {
        return Ok(BytesSource(std::fs::read(path)?));
    }
    Ok(BytesSource(crate::base64::decode(v.as_str().unwrap_or(""))?))
}

fn batch_json(b: &Batch) -> Value {
    let mut v = json!({
        "count": b.count,
        "x": b.x.iter().map(|x| n(*x)).collect::<Vec<_>>(),
        "y": b.y.iter().map(|x| n(*x)).collect::<Vec<_>>(),
        "z": b.z.iter().map(|x| n(*x)).collect::<Vec<_>>(),
        "intensity": b.intensity,
    });
    if let (Some(r), Some(g), Some(bl)) = (&b.red, &b.green, &b.blue) {
        v["red"] = json!(r);
        v["green"] = json!(g);
        v["blue"] = json!(bl);
    }
    v
}

/// A decoded segment as a few sums — what two zlibs agree on, order included.
fn segment_summary(seg: &crate::tiles::Decoded) -> Value {
    let (mut x, mut y, mut z, mut i, mut o) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for k in 0..seg.x.len() {
        let (a, b, c) = (f64::from(seg.x[k]), f64::from(seg.y[k]), f64::from(seg.z[k]));
        x += a;
        y += b;
        z += c;
        i += f64::from(seg.i[k]);
        o += (k % 1000) as f64 * (a + 3.0 * b + 7.0 * c);
    }
    let sum = |v: &[u8]| v.iter().fold(0.0, |s, x| s + f64::from(*x));
    json!({
        "x": n(x), "y": n(y), "z": n(z), "i": n(i), "o": n(o),
        "intensity": seg.intensity.as_ref().map(|v| n(v.iter().fold(0.0, |s, x| s + f64::from(*x)))),
        "rgb": seg.rgb.as_ref().map(|c| json!([n(sum(&c[0])), n(sum(&c[1])), n(sum(&c[2]))])),
    })
}

/// Index bodies with each segment summed instead of its bytes' place: [count, z0, summary].
fn summarised_bodies(bodies: Vec<(usize, Value)>, tiles: &[Vec<u8>]) -> Result<Value> {
    let mut out = Map::new();
    for ((level, mut body), bytes) in bodies.into_iter().zip(tiles) {
        let grid = body.get("grid").and_then(Grid::from_json);
        let shift = body["intensityShift"].as_u64().unwrap_or(0) as u32;
        let rgb = body["rgb"].as_bool().unwrap_or(false);
        let mut tiles_out = Vec::new();
        for t in body["tiles"].as_array().cloned().unwrap_or_default() {
            let mut segs = Vec::new();
            for s in t[2].as_array().cloned().unwrap_or_default() {
                let (o, l, c) = (f(&s[0]) as usize, f(&s[1]) as usize, f(&s[2]) as usize);
                let seg = match grid {
                    Some(_) => decode_grid_segment(&bytes[o..o + l], c, shift, rgb)?,
                    None => decode_segment(&bytes[o..o + l], c, rgb)?,
                };
                segs.push(json!([c, s[3], segment_summary(&seg)]));
            }
            tiles_out.push(json!([t[0], t[1], segs]));
        }
        body["tiles"] = Value::Array(tiles_out);
        body.as_object_mut().unwrap().remove("bytes");
        out.insert(level.to_string(), body);
    }
    Ok(Value::Object(out))
}

/// A batch as a few sums and every 97th point.
fn batch_summary(b: &Batch) -> Value {
    let (mut x, mut y, mut z, mut i) = (0.0, 0.0, 0.0, 0.0);
    let mut rgb = [0.0; 3];
    let mut sample = Vec::new();
    for k in 0..b.count {
        x += b.x[k];
        y += b.y[k];
        z += b.z[k];
        i += f64::from(b.intensity[k]);
        let c = [&b.red, &b.green, &b.blue].map(|c| c.as_ref().map(|c| c[k]));
        for (s, v) in rgb.iter_mut().zip(c) {
            *s += f64::from(v.unwrap_or(0));
        }
        if k % 97 == 0 {
            let mut p = vec![json!(k), n(b.x[k]), n(b.y[k]), n(b.z[k]), json!(b.intensity[k])];
            if b.red.is_some() {
                p.extend(c.iter().map(|v| json!(v.unwrap())));
            }
            sample.push(Value::Array(p));
        }
    }
    json!({ "count": b.count, "x": n(x), "y": n(y), "z": n(z), "i": n(i), "rgb": b.red.as_ref().map(|_| json!(rgb.map(n))), "sample": sample })
}

fn import(source: &BytesSource, header: &Header, opts: &Value) -> Result<(Vec<(usize, Value)>, Vec<Vec<u8>>)> {
    let levels: Vec<usize> = opts
        .get("levels")
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(Value::as_u64).map(|l| l as usize).collect())
        .unwrap_or_else(|| vec![1]);
    let idle = opts.get("idlePoints").and_then(Value::as_u64);
    let mapper = match (opts.get("from").and_then(Value::as_i64), opts.get("to").and_then(Value::as_i64)) {
        (Some(a), Some(b)) => Some(PlaneMapper::new(a, b)?),
        _ => None,
    };
    let map = |e: f64, n: f64| mapper.as_ref().unwrap().map(e, n);
    let mut writers: Vec<MemoryWriter> = levels.iter().map(|_| MemoryWriter::default()).collect();
    let bodies = {
        let refs: Vec<&mut dyn Writer> = writers.iter_mut().map(|w| w as &mut dyn Writer).collect();
        import_levels(
            source,
            header,
            mapper.as_ref().map(|_| &map as &dyn Fn(f64, f64) -> (f64, f64)),
            refs,
            &levels,
            idle,
            None,
            None,
        )?
    };
    Ok((bodies, writers.into_iter().map(|w| w.0).collect()))
}

fn transform_of(v: Option<&Value>) -> Option<Transform> {
    let t = v.filter(|t| t.is_object())?;
    Some(Transform {
        matrix: arr::<16>(&t["matrix"]),
        crs: t["crs"].as_i64()?,
    })
}

/// A cloud from a file imported at level 1 into memory: `{ file, crs, transform, idlePoints }`.
fn cloud(spec: &Value) -> Result<Cloud> {
    let source = file(&spec["file"])?;
    let header = read_cloud_header(&source)?;
    let (bodies, tiles) = import(&source, &header, &json!({ "levels": [1], "idlePoints": spec.get("idlePoints") }))?;
    let id = spec.get("id").and_then(Value::as_str).unwrap_or("cloud");
    Ok(Cloud::from_index(
        id,
        Some(1),
        &bodies[0].1,
        spec.get("crs").and_then(Value::as_i64),
        transform_of(spec.get("transform")),
        Arc::new(TileBytes(tiles.into_iter().next().unwrap())),
    ))
}

fn clouds(v: &Value) -> Result<Vec<Cloud>> {
    v.as_array().map(|a| a.iter().map(cloud).collect()).unwrap_or_else(|| Ok(vec![]))
}

fn points_of(v: &Value) -> (Vec<f32>, Vec<f64>) {
    let y = v["y"].as_array().map(|a| a.iter().map(|x| f(x) as f32).collect()).unwrap_or_default();
    let z = v["z"].as_array().map(|a| a.iter().map(f).collect()).unwrap_or_default();
    (y, z)
}

fn matrix(v: &Value) -> [f64; 16] {
    arr::<16>(v)
}

fn guide_of(v: &Value) -> Guide {
    if v.get("kind").and_then(Value::as_str) == Some("track") {
        return Guide::track(&v["track"]);
    }
    let vertices: Vec<[f64; 2]> = v["vertices"].as_array().map(|a| a.iter().map(arr::<2>).collect()).unwrap_or_default();
    Guide::line(&vertices, v.get("epsg").cloned().unwrap_or(Value::Null))
}

/// Runs operation `op` on `input` (the arguments as the JavaScript takes them).
pub fn run(op: &str, input: &Value) -> Result<Value> {
    let a = |i: usize| input.get(i).unwrap_or(&Value::Null);
    let opt = |i: usize, k: &str| a(i).get(k).filter(|v| !v.is_null());
    let never = || false;
    match op {
        "pointcloud.parseLasHeader" => Ok(parse_las_header(&crate::base64::decode(a(0).as_str().unwrap_or(""))?)?.to_json()),
        "pointcloud.readCloudHeader" => Ok(read_cloud_header(&file(a(0))?)?.to_json()),
        "pointcloud.readPoints" => {
            let source = file(a(0))?;
            let header = read_cloud_header(&source)?;
            let mut progress = Vec::new();
            let mut on = |p: crate::Progress| {
                progress.push(json!({ "bytes": n(p.bytes), "totalBytes": n(p.total_bytes), "points": n(p.points), "totalPoints": n(p.total_points) }))
            };
            let mut batches = Vec::new();
            read_cloud_points(
                &source,
                &header,
                ReadOptions {
                    on_progress: Some(&mut on),
                    aborted: None,
                },
                &mut |b| {
                    batches.push(batch_json(&b));
                    Ok(())
                },
            )?;
            Ok(json!({ "batches": batches, "progress": progress }))
        }
        "pointcloud.readPointsSummary" => {
            let source = file(a(0))?;
            let header = read_cloud_header(&source)?;
            // At most this many batches where a second argument says so.
            let most = a(1).as_u64().map(|m| m as usize).unwrap_or(usize::MAX);
            let mut progress = Vec::new();
            let mut on = |p: crate::Progress| {
                progress.push(json!({ "bytes": n(p.bytes), "totalBytes": n(p.total_bytes), "points": n(p.points), "totalPoints": n(p.total_points) }))
            };
            let mut batches = Vec::new();
            let r = read_cloud_points(
                &source,
                &header,
                ReadOptions {
                    on_progress: Some(&mut on),
                    aborted: None,
                },
                &mut |b| {
                    batches.push(batch_summary(&b));
                    if batches.len() >= most { Err(Error::Aborted) } else { Ok(()) }
                },
            );
            if let Some(e) = r.as_ref().err().filter(|e| **e != Error::Aborted) {
                return Err(e.clone());
            }
            Ok(json!({ "batches": batches, "progress": progress }))
        }
        "pointcloud.readFirstPoints" => {
            let source = file(a(0))?;
            let header = read_cloud_header(&source)?;
            let pts = read_first_points(&source, &header, a(1).as_f64().filter(|x| *x > 0.0).unwrap_or(0.0) as usize)?;
            Ok(Value::Array(
                pts.iter()
                    .map(|p| json!({ "x": n(p[0]), "y": n(p[1]), "z": n(p[2]), "intensity": n(p[3]) }))
                    .collect(),
            ))
        }
        "pointcloud.importLevels" => {
            let source = file(a(0))?;
            let header = read_cloud_header(&source)?;
            let (bodies, tiles) = import(&source, &header, a(1))?;
            summarised_bodies(bodies, &tiles)
        }
        "pointcloud.parseE57Xml" => Ok(parse_xml(a(0).as_str().unwrap_or("")).map_err(Error::Msg)?.to_json()),
        "pointcloud.planeMapper" => {
            let m = PlaneMapper::new(a(0).as_i64().unwrap_or(0), a(1).as_i64().unwrap_or(0))?;
            Ok(Value::Array(
                a(2).as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|p| {
                        let (e, nn) = m.map(f(&p[0]), f(&p[1]));
                        json!([n(e), n(nn)])
                    })
                    .collect(),
            ))
        }
        "pointcloud.solveRegistration" => Ok(reg::solve_registration(a(0).as_array().map(Vec::as_slice).unwrap_or(&[]), a(1))),
        "pointcloud.invertMatrix" => Ok(json!(reg::invert_matrix(&matrix(a(0))).map(n))),
        "pointcloud.chainMatrices" => Ok(json!(reg::chain_matrices(&matrix(a(0)), &matrix(a(1))).map(n))),
        "pointcloud.applyMatrix" => Ok(json!(reg::apply_matrix(&matrix(a(0)), arr::<3>(a(1))).map(n))),
        "pointcloud.transformMatrix" => {
            let mut params = reg::Params::new();
            for k in ["tE", "tN", "tH", "kappa", "omega", "phi", "scale"] {
                if let Some(v) = a(0).get(k).and_then(Value::as_f64) {
                    params.insert(k, v);
                }
            }
            Ok(json!(reg::transform_matrix(&params, &arr::<3>(a(1))).map(n)))
        }
        "pointcloud.mainBearing" => Ok(n(reg::main_bearing(
            &a(0).as_array().cloned().unwrap_or_default().iter().map(arr::<3>).collect::<Vec<_>>(),
        ))),
        "pointcloud.symmetricEigen" => {
            let m: Vec<Vec<f64>> = a(0)
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|r| r.as_array().cloned().unwrap_or_default().iter().map(f).collect())
                .collect();
            let (values, vectors) = reg::symmetric_eigen(&m);
            Ok(
                json!({ "values": values.iter().map(|x| n(*x)).collect::<Vec<_>>(), "vectors": vectors.iter().map(|r| r.iter().map(|x| n(*x)).collect::<Vec<_>>()).collect::<Vec<_>>() }),
            )
        }
        "pointcloud.sliceFrame" => {
            let g = |k: &str| opt(0, k).map(f).unwrap_or(f64::NAN);
            Ok(slice_frame(g("easting"), g("northing"), g("bearing"), g("halfWidth"), g("thickness")).to_json())
        }
        "pointcloud.tilesInSlice" => {
            let fr = a(1);
            let frame = crate::slice::Frame {
                e: f(&fr["e"]),
                n: f(&fr["n"]),
                along: arr::<2>(&fr["along"]),
                right: arr::<2>(&fr["right"]),
                half_width: f(&fr["halfWidth"]),
                half: f(&fr["half"]),
            };
            let tiles = a(0)["tiles"].as_array().cloned().unwrap_or_default();
            let coords: Vec<(i64, i64)> = tiles.iter().map(|t| (t[0].as_i64().unwrap_or(0), t[1].as_i64().unwrap_or(0))).collect();
            Ok(Value::Array(
                tiles_in_slice(f(&a(0)["tileSize"]), &coords, &frame)
                    .into_iter()
                    .map(|k| tiles[k].clone())
                    .collect(),
            ))
        }
        "pointcloud.checkClearance" => {
            let (y, z) = points_of(a(0));
            let areas: Vec<_> = opt(1, "areas")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(ring_of)
                .collect();
            Ok(check_clearance(
                &y,
                &z,
                opt(1, "zTrack").map(f).unwrap_or(f64::NAN),
                opt(1, "cant").map(f).unwrap_or(0.0),
                &ring_of(&a(1)["ring"]),
                &areas,
            )
            .to_json())
        }
        "pointcloud.stretchesOf" => {
            let hits: Vec<(f64, usize, f64)> = a(0)
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|h| (f(&h["station"]), h["inside"].as_u64().unwrap_or(0) as usize, f(&h["depth"])))
                .collect();
            Ok(Value::Array(stretches_of(&hits, a(1).as_f64().unwrap_or(0.5))))
        }
        "pointcloud.topSurface" => {
            let (y, z) = points_of(a(0));
            Ok(top_surface(&y, &z, f(a(1)), f(a(2))).to_json())
        }
        "pointcloud.edgeCandidates" => {
            let (y, z) = points_of(a(0));
            let s = top_surface(&y, &z, f(a(1)), f(a(2)));
            Ok(Value::Array(
                edge_candidates(&s, f(a(1)), f(a(2)), f(a(3)), f(a(4)))
                    .iter()
                    .map(|e| json!({ "y": n(e.y), "z": n(e.z), "drop": n(e.drop) }))
                    .collect(),
            ))
        }
        "pointcloud.detectTrack" => {
            let (y, z) = points_of(a(0));
            Ok(detect_track(
                &y,
                &z,
                opt(1, "around").map(f).unwrap_or(0.0),
                opt(1, "window").map(f).unwrap_or(0.3),
                opt(1, "rail").and_then(Value::as_str),
            )
            .to_json())
        }
        "pointcloud.foresee" => {
            let pts: Vec<Point> = a(0).as_array().cloned().unwrap_or_default().iter().map(Point::from_json).collect();
            Ok(foresee(&pts, f(a(1)))
                .map(|(o, s)| json!({ "offset": n(o), "slope": n(s) }))
                .unwrap_or(Value::Null))
        }
        "pointcloud.gapsOf" => {
            let st: Vec<f64> = a(0).as_array().cloned().unwrap_or_default().iter().map(f).collect();
            Ok(Value::Array(
                gaps_of(&st, a(1).as_f64().unwrap_or(0.5))
                    .into_iter()
                    .map(|(a, b)| json!({ "from": n(a), "to": n(b) }))
                    .collect(),
            ))
        }
        "pointcloud.guide" => {
            let g = guide_of(a(0));
            let at: Vec<Value> = a(1)
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|s| {
                    g.at(f(s))
                        .map(|(e, nn, b)| json!({ "easting": n(e), "northing": n(nn), "bearing": n(b) }))
                        .unwrap_or(Value::Null)
                })
                .collect();
            Ok(json!({ "epsg": g.epsg(), "length": n(g.length()), "window": n(g.window()), "at": at }))
        }
        "pointcloud.soHeight" => Ok(n(so_height(&Point::from_json(a(0)), a(1).as_str().unwrap_or("lower")))),
        "pointcloud.cantMm" => Ok(n(cant_mm(&Point::from_json(a(0))))),
        "pointcloud.axisPointsCsv" => {
            let pts: Vec<Point> = a(0).as_array().cloned().unwrap_or_default().iter().map(Point::from_json).collect();
            Ok(json!(axis_points_csv(&pts, opt(1, "soReference").and_then(Value::as_str).unwrap_or("lower"))))
        }
        "pointcloud.cloudSectionPoints" => {
            let c = cloud(a(0))?;
            let o = a(1);
            let p = cloud_section_points(
                &c,
                (f(&o["origin"]["easting"]), f(&o["origin"]["northing"])),
                f(&o["bearing"]),
                o["crs"].as_i64().unwrap_or(0),
                f(&o["halfWidth"]),
                f(&o["thickness"]),
            )?;
            let mut v = p.to_json();
            v["tiles"] = json!(p.tiles);
            Ok(v)
        }
        "pointcloud.detectInClouds" => {
            let cs = clouds(a(0))?;
            let o = a(1);
            let d = detect_in_clouds(
                &cs,
                (f(&o["origin"]["easting"]), f(&o["origin"]["northing"])),
                f(&o["bearing"]),
                o["crs"].as_i64().unwrap_or(0),
                o.get("around").map(f).unwrap_or(0.0),
                o.get("window").and_then(Value::as_f64).unwrap_or(1.5),
                o.get("rail").and_then(Value::as_str),
            )?;
            Ok(d.to_json())
        }
        "pointcloud.scanClearance" => {
            let p = a(0);
            let cs = clouds(&p["clouds"])?;
            let areas: Vec<_> = p
                .get("areas")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(ring_of)
                .collect();
            let list = |k: &str| p.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
            let mut shares = Vec::new();
            let r = scan_clearance(
                &cs,
                &p["track"],
                &ring_of(&p["ring"]),
                &areas,
                p.get("from").and_then(Value::as_f64).unwrap_or(0.0),
                p.get("to").and_then(Value::as_f64),
                &list("tracks"),
                &list("switches"),
                p.get("step").and_then(Value::as_f64),
                &mut |s| shares.push(n(s)),
                &never,
            )?;
            Ok(json!({ "result": r, "progress": shares }))
        }
        "pointcloud.traceTrack" => {
            let p = a(0);
            let cs = clouds(&p["clouds"])?;
            let guide = guide_of(&p["guide"]);
            let rail = p.get("rail").and_then(Value::as_str).map(str::to_string);
            let mut detect = |origin: (f64, f64), bearing: f64, epsg: &Value, around: f64, window: f64| {
                detect_in_clouds(&cs, origin, bearing, epsg.as_i64().unwrap_or(0), around, window, rail.as_deref())
            };
            let mut shares = Vec::new();
            let r = trace_track(&guide, p.get("step").and_then(Value::as_f64), &mut detect, &mut |s| shares.push(n(s)), &never)?;
            Ok(json!({ "result": r, "progress": shares }))
        }
        _ => Err(Error::msg(format!("unknown operation {op}"))),
    }
}
