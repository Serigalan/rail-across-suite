// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Reading the points of an E57 file (ASTM E2807) in pieces (`e57Reader.js`):
//! pages with checksums passed over, the XML section, and each scan's
//! CompressedVector of bit-packed or float bytestreams. Only the coordinates
//! (Cartesian, or spherical turned Cartesian), the invalid state, the
//! intensity and the colour are decoded; the pose is applied.

use serde_json::{Value, json};

use crate::e57_xml::{Node, child, parse_xml};
use crate::source::Source;
use crate::{Batch, Error, Progress, ReadOptions, Result};

const SIGNATURE: &[u8] = b"ASTM-E57";
const HEADER_BYTES: usize = 48;
const CHECKSUM_BYTES: u64 = 4;
const WINDOW_BYTES: u64 = 4 * 1024 * 1024;
const SAMPLE_BYTES: u64 = 256 * 1024;
const MAX_PACKET: u64 = 65536;
const BOX_SLACK: f64 = 1.0;

/// Whether `bytes` (the start of a file) are an E57 file's.
pub fn is_e57(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && &bytes[..8] == SIGNATURE
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}

/// The file's logical bytes — its content without the page checksums.
struct Paged<'a> {
    source: &'a dyn Source,
    page: u64,
    data: u64,
}

impl Paged<'_> {
    fn logical(&self, physical: u64) -> u64 {
        physical / self.page * self.data + physical % self.page
    }

    fn read(&self, at: u64, length: u64) -> Result<Vec<u8>> {
        if length == 0 {
            return Ok(vec![]);
        }
        let d = self.data;
        let (first, last) = (at / d, (at + length - 1) / d);
        let start = first * self.page + at % d;
        let end = last * self.page + (at + length - 1) % d + 1;
        if end > self.source.size() {
            return Err(Error::msg("E57 file is incomplete"));
        }
        let raw = self.source.read(start, (end - start) as usize)?;
        if first == last {
            return Ok(raw);
        }
        let mut out = vec![0u8; length as usize];
        let (mut from, mut to) = (0usize, 0usize);
        for page in first..=last {
            let take = (if page == first { d - at % d } else { d }).min(length - to as u64) as usize;
            out[to..to + take].copy_from_slice(&raw[from..from + take]);
            to += take;
            from += take + CHECKSUM_BYTES as usize;
        }
        Ok(out)
    }
}

// ── the XML: scans, their poses and fields ──────────────────────────────────

fn value(node: Option<&Node>, fallback: f64) -> f64 {
    let Some(n) = node else { return fallback };
    let s = n.text.trim();
    if s.is_empty() {
        0.0
    } else {
        rail_across_js::js_number(Some(&Value::String(s.to_string())))
    }
}

fn big_attr(s: Option<&str>, fallback: i128) -> Result<i128> {
    let Some(s) = s.filter(|s| !s.is_empty()) else { return Ok(fallback) };
    if let Ok(v) = s.trim().parse::<i128>() {
        return Ok(v);
    }
    let n = rail_across_js::js_round(rail_across_js::js_number(Some(&Value::String(s.to_string()))));
    if !n.is_finite() {
        return Err(Error::msg(format!(
            "The number {} cannot be converted to a BigInt because it is not an integer",
            rail_across_js::js_string(Some(&rail_across_js::number(n)))
        )));
    }
    Ok(n as i128)
}

/// How a prototype field is coded in its bytestream.
#[derive(Clone, Debug)]
pub enum Codec {
    Float {
        bytes: usize,
        min: Option<f64>,
        max: Option<f64>,
    },
    Int {
        bits: u32,
        min: f64,
        scale: f64,
        offset: f64,
        low: f64,
        high: f64,
    },
    String,
    Other,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub codec: Codec,
}

impl Field {
    fn numeric(&self) -> bool {
        matches!(self.codec, Codec::Float { .. } | Codec::Int { .. })
    }
    fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        let mut v = match &self.codec {
            Codec::Float { bytes, min, max } => json!({ "kind": "float", "bytes": bytes, "min": min.map(n), "max": max.map(n) }),
            Codec::Int {
                bits,
                min,
                scale,
                offset,
                low,
                high,
            } => {
                json!({ "kind": "int", "bits": bits, "min": n(*min), "scale": n(*scale), "offset": n(*offset), "low": n(*low), "high": n(*high) })
            }
            Codec::String => json!({ "kind": "string" }),
            Codec::Other => json!({ "kind": "other" }),
        };
        v["name"] = json!(self.name);
        v
    }
}

fn field_codec(node: &Node) -> Result<Codec> {
    let t = node.attr("type");
    let num = |k: &str| node.attr(k).map(|s| rail_across_js::js_number(Some(&Value::String(s.to_string()))));
    match t {
        Some("Float") => Ok(Codec::Float {
            bytes: if node.attr("precision") == Some("single") { 4 } else { 8 },
            min: num("minimum"),
            max: num("maximum"),
        }),
        Some("Integer") | Some("ScaledInteger") => {
            let lo = big_attr(node.attr("minimum"), i128::from(i64::MIN))?;
            let hi = big_attr(node.attr("maximum"), i128::from(i64::MAX))?;
            let range = hi - lo;
            let scaled = t == Some("ScaledInteger");
            let scale = if scaled { num("scale").unwrap_or(1.0) } else { 1.0 };
            let offset = if scaled { num("offset").unwrap_or(0.0) } else { 0.0 };
            let bits = if range > 0 { 128 - range.leading_zeros() } else { 0 };
            Ok(Codec::Int {
                bits,
                min: lo as f64,
                scale,
                offset,
                low: lo as f64 * scale + offset,
                high: hi as f64 * scale + offset,
            })
        }
        Some("String") => Ok(Codec::String),
        _ => Ok(Codec::Other),
    }
}

fn terminals(node: &Node, path: &str, out: &mut Vec<Field>) -> Result<()> {
    for c in &node.children {
        if c.attr("type") == Some("Structure") {
            terminals(c, &format!("{path}{}/", c.name), out)?;
        } else {
            out.push(Field {
                name: format!("{path}{}", c.name),
                codec: field_codec(c)?,
            });
        }
    }
    Ok(())
}

fn quaternion_matrix(w: f64, x: f64, y: f64, z: f64) -> [f64; 9] {
    let n = rail_across_js::js_hypot(&[w, x, y, z]);
    let n = if n == 0.0 || n.is_nan() { 1.0 } else { n };
    let (w, x, y, z) = (w / n, x / n, y / n, z / n);
    [
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y - w * z),
        2.0 * (x * z + w * y),
        2.0 * (x * y + w * z),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z - w * x),
        2.0 * (x * z - w * y),
        2.0 * (y * z + w * x),
        1.0 - 2.0 * (x * x + y * y),
    ]
}

/// One scan of the file: its fields, where its points lie, its pose.
#[derive(Clone, Debug)]
pub struct Scan {
    pub name: String,
    pub record_count: u64,
    pub file_offset: u64,
    pub fields: Vec<Field>,
    pub coords: Option<&'static str>,
    pub xyz: [Option<usize>; 3],
    pub state: Option<usize>,
    pub intensity: Option<usize>,
    pub intensity_range: Option<[f64; 2]>,
    pub color: Option<[usize; 3]>,
    pub color_range: Option<[[f64; 2]; 3]>,
    pub rotation: [f64; 9],
    pub translation: [f64; 3],
    pub bounds: Option<[f64; 6]>,
    pub range_maximum: Option<f64>,
}

impl Scan {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        let arr = |v: &[f64]| json!(v.iter().map(|x| n(*x)).collect::<Vec<_>>());
        json!({
            "name": self.name,
            "recordCount": self.record_count,
            "fileOffset": self.file_offset,
            "fields": self.fields.iter().map(Field::to_json).collect::<Vec<_>>(),
            "coords": self.coords,
            "xyz": self.xyz.iter().map(|k| k.map(|k| json!(k)).unwrap_or(json!(-1))).collect::<Vec<_>>(),
            "state": self.state,
            "intensity": self.intensity,
            "intensityRange": self.intensity_range.map(|r| arr(&r)),
            "color": self.color,
            "colorRange": self.color_range.map(|r| json!(r.iter().map(|c| arr(c)).collect::<Vec<_>>())),
            "rotation": arr(&self.rotation),
            "translation": arr(&self.translation),
            "bounds": self.bounds.map(|b| arr(&b)),
            "rangeMaximum": self.range_maximum.map(n),
        })
    }
}

const COLORS: [&str; 3] = ["Red", "Green", "Blue"];

fn intensity_range(scan: &Node, field: &Field) -> [f64; 2] {
    if let Some(limits) = scan.child("intensityLimits") {
        let lo = value(limits.child("intensityMinimum"), f64::NAN);
        let hi = value(limits.child("intensityMaximum"), f64::NAN);
        if lo.is_finite() && hi.is_finite() && hi > lo {
            return [lo, hi];
        }
    }
    match field.codec {
        Codec::Int { low, high, .. } if high > low => [low, high],
        Codec::Float {
            min: Some(lo), max: Some(hi), ..
        } if lo.is_finite() && hi.is_finite() && hi > lo && hi - lo < 1e30 => [lo, hi],
        _ => [0.0, 1.0],
    }
}

fn color_range(scan: &Node, field: &Field, channel: &str) -> [f64; 2] {
    if let Some(limits) = scan.child("colorLimits") {
        let lo = value(limits.child(&format!("color{channel}Minimum")), f64::NAN);
        let hi = value(limits.child(&format!("color{channel}Maximum")), f64::NAN);
        if lo.is_finite() && hi.is_finite() && hi > lo {
            return [lo, hi];
        }
    }
    match field.codec {
        Codec::Int { low, high, .. } if high > low => [low, high],
        _ => [0.0, 255.0],
    }
}

fn parse_scan(node: &Node, i: usize) -> Result<Scan> {
    let points = node.child("points");
    let name = node
        .child("name")
        .map(|n| n.text.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("scan {}", i + 1));
    let Some(points) = points.filter(|p| p.attr("type") == Some("CompressedVector")) else {
        return Err(Error::msg(format!("E57 scan \"{name}\" has no points")));
    };
    let Some(proto) = points.child("prototype") else {
        return Err(Error::msg(format!("E57 scan \"{name}\" has no prototype")));
    };
    if points.child("codecs").is_some_and(|c| !c.children.is_empty()) {
        return Err(Error::msg("E57 codecs other than bit packing are not supported"));
    }
    let mut fields = Vec::new();
    terminals(proto, "", &mut fields)?;
    let at = |n: &str| fields.iter().position(|f| f.name == n);
    let cart = ["cartesianX", "cartesianY", "cartesianZ"].map(at);
    let sph = ["sphericalRange", "sphericalAzimuth", "sphericalElevation"].map(at);
    let coords = if cart.iter().all(Option::is_some) {
        Some("cartesian")
    } else if sph.iter().all(Option::is_some) {
        Some("spherical")
    } else {
        None
    };
    let record_count = points
        .attr("recordCount")
        .map(|s| rail_across_js::js_number(Some(&Value::String(s.to_string()))))
        .unwrap_or(0.0);
    let record_count = if record_count.is_finite() && record_count > 0.0 {
        record_count as u64
    } else {
        0
    };
    if coords.is_none() && record_count > 0 {
        return Err(Error::msg(format!("E57 scan \"{name}\" has no coordinates")));
    }
    let xyz = if coords == Some("spherical") { sph } else { cart };
    if xyz.iter().flatten().any(|&k| !fields[k].numeric()) {
        return Err(Error::msg(format!("E57 scan \"{name}\": coordinates of an unsupported type")));
    }
    let state = at(if coords == Some("spherical") {
        "sphericalInvalidState"
    } else {
        "cartesianInvalidState"
    });
    let intensity = at("intensity").filter(|&k| fields[k].numeric());
    let color = COLORS.map(|c| at(&format!("color{c}")));
    let color = if color.iter().all(|k| k.is_some_and(|k| fields[k].numeric())) {
        Some(color.map(Option::unwrap))
    } else {
        None
    };

    let bounds = node
        .child("cartesianBounds")
        .map(|cb| ["xMinimum", "yMinimum", "zMinimum", "xMaximum", "yMaximum", "zMaximum"].map(|k| value(cb.child(k), f64::NAN)));
    let range = value(child(node.child("sphericalBounds"), "rangeMaximum"), f64::NAN);
    let pose = node.child("pose");
    let (r, t) = (child(pose, "rotation"), child(pose, "translation"));
    let file_offset = points
        .attr("fileOffset")
        .map(|s| rail_across_js::js_number(Some(&Value::String(s.to_string()))))
        .unwrap_or(0.0);
    Ok(Scan {
        name,
        record_count,
        file_offset: if file_offset.is_finite() && file_offset > 0.0 {
            file_offset as u64
        } else {
            0
        },
        coords,
        xyz,
        state,
        intensity,
        intensity_range: intensity.map(|k| intensity_range(node, &fields[k])),
        color_range: color.map(|c| [0, 1, 2].map(|i| color_range(node, &fields[c[i]], COLORS[i]))),
        color,
        rotation: match r {
            Some(r) => quaternion_matrix(
                value(r.child("w"), 1.0),
                value(r.child("x"), 0.0),
                value(r.child("y"), 0.0),
                value(r.child("z"), 0.0),
            ),
            None => [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        },
        translation: match t {
            Some(t) => ["x", "y", "z"].map(|k| value(t.child(k), 0.0)),
            None => [0.0; 3],
        },
        bounds: bounds.filter(|b| b.iter().all(|v| v.is_finite())),
        range_maximum: (range.is_finite() && range > 0.0).then_some(range),
        fields,
    })
}

// ── the bytestreams ──────────────────────────────────────────────────────────

enum Stream {
    Constant(f64),
    Float {
        bytes: usize,
        rest: Vec<u8>,
        queue: Vec<f64>,
    },
    Bits {
        bits: u32,
        min: f64,
        scale: f64,
        offset: f64,
        rest: Vec<u8>,
        bit: usize,
        queue: Vec<f64>,
    },
}

impl Stream {
    fn new(field: &Field) -> Stream {
        match field.codec {
            Codec::Float { bytes, .. } => Stream::Float {
                bytes,
                rest: vec![],
                queue: vec![],
            },
            Codec::Int { bits: 0, low, .. } => Stream::Constant(low),
            Codec::Int { bits, min, scale, offset, .. } => Stream::Bits {
                bits,
                min,
                scale,
                offset,
                rest: vec![],
                bit: 0,
                queue: vec![],
            },
            _ => Stream::Constant(0.0),
        }
    }

    fn queue(&self) -> Option<&Vec<f64>> {
        match self {
            Stream::Constant(_) => None,
            Stream::Float { queue, .. } | Stream::Bits { queue, .. } => Some(queue),
        }
    }

    fn get(&self, i: usize) -> f64 {
        match self {
            Stream::Constant(c) => *c,
            Stream::Float { queue, .. } | Stream::Bits { queue, .. } => queue[i],
        }
    }

    fn drop_front(&mut self, n: usize) {
        if let Stream::Float { queue, .. } | Stream::Bits { queue, .. } = self {
            queue.drain(..n);
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        match self {
            Stream::Constant(_) => {}
            Stream::Float { bytes, rest, queue } => {
                let mut buf = std::mem::take(rest);
                buf.extend_from_slice(chunk);
                let n = buf.len() / *bytes;
                for i in 0..n {
                    let at = i * *bytes;
                    queue.push(if *bytes == 8 {
                        f64::from_le_bytes(buf[at..at + 8].try_into().unwrap())
                    } else {
                        f64::from(f32::from_le_bytes(buf[at..at + 4].try_into().unwrap()))
                    });
                }
                *rest = buf[n * *bytes..].to_vec();
            }
            Stream::Bits {
                bits,
                min,
                scale,
                offset,
                rest,
                bit,
                queue,
            } => {
                let mut buf = std::mem::take(rest);
                buf.extend_from_slice(chunk);
                let bits = *bits as usize;
                let total = buf.len() * 8;
                let n = if total >= *bit { (total - *bit) / bits } else { 0 };
                let byte = |k: usize| u32::from(*buf.get(k).unwrap_or(&0));
                let mut p = *bit;
                for _ in 0..n {
                    let b = p >> 3;
                    let v = if bits <= 24 {
                        let word = byte(b) | byte(b + 1) << 8 | byte(b + 2) << 16 | byte(b + 3) << 24;
                        f64::from((word >> (p & 7)) & ((1u32 << bits) - 1))
                    } else if bits <= 32 {
                        let s = p & 7;
                        let word = (byte(b) | byte(b + 1) << 8 | byte(b + 2) << 16 | byte(b + 3) << 24) >> s;
                        (f64::from(word) + f64::from(byte(b + 4)) * 2f64.powi(32 - s as i32)) % 2f64.powi(bits as i32)
                    } else {
                        let mut v = 0.0;
                        let mut got = 0;
                        let mut q = p;
                        while got < bits {
                            let s = q & 7;
                            let take = (8 - s).min(bits - got);
                            v += f64::from((byte(q >> 3) >> s) & ((1u32 << take) - 1)) * 2f64.powi(got as i32);
                            got += take;
                            q += take;
                        }
                        v
                    };
                    queue.push((v + *min) * *scale + *offset);
                    p += bits;
                }
                let used = *bit + n * bits;
                *rest = buf[(used >> 3).min(buf.len())..].to_vec();
                *bit = used & 7;
            }
        }
    }
}

fn to16(v: f64, lo: f64, hi: f64) -> u16 {
    let s = rail_across_js::js_round((v - lo) * 65535.0 / (hi - lo));
    if s < 0.0 {
        0
    } else if s > 65535.0 {
        65535
    } else if s.is_nan() {
        0
    } else {
        s as u16
    }
}

fn assemble(scan: &Scan, streams: &mut [(usize, Stream)], n: usize, rgb: bool) -> Batch {
    let find = |k: Option<usize>| k.and_then(|k| streams.iter().position(|(f, _)| *f == k));
    let [ca, cb, cc] = scan.xyz.map(find);
    let (st, it) = (find(scan.state), find(scan.intensity));
    let cols = if rgb { scan.color.map(|c| c.map(|k| find(Some(k)))) } else { None };
    let r = scan.rotation;
    let t = scan.translation;
    let [ilo, ihi] = scan.intensity_range.unwrap_or([0.0, 1.0]);
    let iscale = 65535.0 / (ihi - ilo);
    let spherical = scan.coords == Some("spherical");
    let get = |k: Option<usize>, i: usize| k.map(|k| streams[k].1.get(i)).unwrap_or(f64::NAN);
    let mut batch = Batch::new(n, rgb);
    let mut m = 0;
    for i in 0..n {
        if st.is_some() && get(st, i) != 0.0 {
            continue;
        }
        let (mut x, mut y, mut z) = (get(ca, i), get(cb, i), get(cc, i));
        if spherical {
            let (rr, az, el) = (x, y, z);
            let h = rr * el.cos();
            x = h * az.cos();
            y = h * az.sin();
            z = rr * el.sin();
        }
        batch.x[m] = r[0] * x + r[1] * y + r[2] * z + t[0];
        batch.y[m] = r[3] * x + r[4] * y + r[5] * z + t[1];
        batch.z[m] = r[6] * x + r[7] * y + r[8] * z + t[2];
        if it.is_some() {
            let v = rail_across_js::js_round((get(it, i) - ilo) * iscale);
            batch.intensity[m] = if v < 0.0 || v.is_nan() {
                0
            } else if v > 65535.0 {
                65535
            } else {
                v as u16
            };
        }
        if let (Some(cols), Some(cr)) = (cols, scan.color_range) {
            batch.red.as_mut().unwrap()[m] = to16(get(cols[0], i), cr[0][0], cr[0][1]);
            batch.green.as_mut().unwrap()[m] = to16(get(cols[1], i), cr[1][0], cr[1][1]);
            batch.blue.as_mut().unwrap()[m] = to16(get(cols[2], i), cr[2][0], cr[2][1]);
        }
        m += 1;
    }
    batch.truncate(m);
    batch.records = n;
    for (_, s) in streams.iter_mut() {
        s.drop_front(n);
    }
    batch
}

fn open_section(paged: &Paged, scan: &Scan) -> Result<(u64, u64)> {
    let section_at = paged.logical(scan.file_offset);
    let head = paged.read(section_at, 32)?;
    if head.len() < 32 || head[0] != 1 {
        return Err(Error::msg(format!("E57 scan \"{}\": no CompressedVector section", scan.name)));
    }
    let end = section_at + u64_at(&head, 8);
    let data_at = paged.logical(u64_at(&head, 16));
    if !(data_at >= section_at + 32 && data_at <= end) {
        return Err(Error::msg(format!("E57 scan \"{}\": bad section header", scan.name)));
    }
    Ok((data_at, end))
}

/// One scan's points, batch by batch (`scanBatches`).
fn scan_batches(
    paged: &Paged,
    scan: &Scan,
    window: u64,
    limit: Option<u64>,
    rgb: bool,
    o: &mut ReadOptions,
    on_bytes: &mut dyn FnMut(u64),
    each: &mut dyn FnMut(Batch) -> Result<()>,
) -> Result<()> {
    if scan.record_count == 0 {
        return Ok(());
    }
    let (data_at, end) = open_section(paged, scan)?;
    let mut needed: Vec<usize> = scan.xyz.iter().flatten().copied().collect();
    needed.extend(scan.state);
    needed.extend(scan.intensity);
    if rgb {
        needed.extend(scan.color.into_iter().flatten());
    }
    let mut streams: Vec<(usize, Stream)> = Vec::new();
    for k in needed {
        if !streams.iter().any(|(f, _)| *f == k) {
            streams.push((k, Stream::new(&scan.fields[k])));
        }
    }
    let mut remaining = scan.record_count;
    let mut at = data_at;
    let stop = end.min(limit.map(|l| data_at + l).unwrap_or(u64::MAX));
    while remaining > 0 && at < stop {
        o.check()?;
        let buf = paged.read(at, window.max(MAX_PACKET).min(end - at))?;
        let mut p = 0usize;
        while p + 4 <= buf.len() {
            let kind = buf[p];
            let length = u16_at(&buf, p + 2) + 1;
            if p + length > buf.len() {
                break;
            }
            if kind == 1 {
                let count = u16_at(&buf, p + 4);
                let mut q = p + 6 + 2 * count;
                for s in 0..count {
                    let len = u16_at(&buf, p + 6 + 2 * s);
                    if let Some((_, stream)) = streams.iter_mut().find(|(f, _)| *f == s) {
                        stream.push(&buf[q.min(buf.len())..(q + len).min(buf.len())]);
                    }
                    q += len;
                }
                if q > p + length {
                    return Err(Error::msg(format!("E57 scan \"{}\": bad data packet", scan.name)));
                }
            } else if kind != 0 && kind != 2 {
                return Err(Error::msg(format!("E57 scan \"{}\": unknown packet type {kind}", scan.name)));
            }
            p += length;
        }
        if p == 0 {
            return Err(Error::msg(format!("E57 scan \"{}\": truncated packet", scan.name)));
        }
        at += p as u64;
        on_bytes(at - data_at);
        let mut n = remaining;
        for (_, s) in &streams {
            if let Some(q) = s.queue() {
                n = n.min(q.len() as u64);
            }
        }
        if n > 0 {
            remaining -= n;
            let batch = assemble(scan, &mut streams, n as usize, rgb);
            each(batch)?;
        }
    }
    if remaining > 0 && limit.is_none() {
        return Err(Error::msg(format!("E57 scan \"{}\" ends {remaining} points early", scan.name)));
    }
    Ok(())
}

fn transform_box(scan: &Scan, b: [f64; 6]) -> [f64; 6] {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let (r, t) = (scan.rotation, scan.translation);
    for x in [b[0], b[3]] {
        for y in [b[1], b[4]] {
            for z in [b[2], b[5]] {
                let g = [
                    r[0] * x + r[1] * y + r[2] * z + t[0],
                    r[3] * x + r[4] * y + r[5] * z + t[1],
                    r[6] * x + r[7] * y + r[8] * z + t[2],
                ];
                for k in 0..3 {
                    min[k] = min[k].min(g[k]);
                    max[k] = max[k].max(g[k]);
                }
            }
        }
    }
    [min[0], min[1], min[2], max[0], max[1], max[2]]
}

fn holds(b: &[f64; 6], s: &[f64; 6]) -> bool {
    b[0] - BOX_SLACK <= s[0]
        && b[1] - BOX_SLACK <= s[1]
        && b[2] - BOX_SLACK <= s[2]
        && b[3] + BOX_SLACK >= s[3]
        && b[4] + BOX_SLACK >= s[4]
        && b[5] + BOX_SLACK >= s[5]
}

fn sample_box(paged: &Paged, scan: &Scan) -> Result<Option<[f64; 6]>> {
    let mut bx = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut o = ReadOptions::default();
    scan_batches(paged, scan, SAMPLE_BYTES, Some(SAMPLE_BYTES), false, &mut o, &mut |_| {}, &mut |b| {
        for i in 0..b.count {
            let p = [b.x[i], b.y[i], b.z[i]];
            for k in 0..3 {
                bx[k] = bx[k].min(p[k]);
                bx[k + 3] = bx[k + 3].max(p[k]);
            }
        }
        Ok(())
    })?;
    Ok((bx[0] <= bx[3]).then_some(bx))
}

fn scan_extent(paged: &Paged, scan: &Scan) -> Result<Option<[f64; 6]>> {
    let sample = sample_box(paged, scan)?;
    let mut candidates: Vec<[f64; 6]> = match scan.bounds {
        Some(b) => vec![transform_box(scan, b), b],
        None => vec![],
    };
    if let Some(r) = scan.range_maximum {
        let t = scan.translation;
        candidates.push([t[0] - r, t[1] - r, t[2] - r, t[0] + r, t[1] + r, t[2] + r]);
    }
    let Some(sample) = sample else { return Ok(candidates.first().copied()) };
    Ok(Some(candidates.into_iter().find(|b| holds(b, &sample)).unwrap_or(sample)))
}

/// The header of an E57 file (`readE57Header`).
#[derive(Clone, Debug)]
pub struct E57Header {
    pub version: String,
    pub point_count: u64,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub scale: [f64; 3],
    pub page_size: u64,
    pub coordinate_metadata: String,
    pub rgb: bool,
    pub scans: Vec<Scan>,
}

impl E57Header {
    pub fn to_json(&self) -> Value {
        let n = rail_across_js::number;
        let arr = |v: &[f64; 3]| json!(v.iter().map(|x| n(*x)).collect::<Vec<_>>());
        json!({
            "format": "e57",
            "version": self.version,
            "compressed": false,
            "pointFormat": null,
            "pointCount": self.point_count,
            "min": arr(&self.min),
            "max": arr(&self.max),
            "scale": arr(&self.scale),
            "offset": [0, 0, 0],
            "pageSize": self.page_size,
            "coordinateMetadata": self.coordinate_metadata,
            "rgb": self.rgb,
            "scans": self.scans.iter().map(Scan::to_json).collect::<Vec<_>>(),
        })
    }
}

pub fn read_e57_header(source: &dyn Source) -> Result<E57Header> {
    if source.size() < HEADER_BYTES as u64 {
        return Err(Error::msg("not an E57 file"));
    }
    let head = source.read(0, HEADER_BYTES)?;
    if !is_e57(&head) {
        return Err(Error::msg("not an E57 file"));
    }
    let major = u32::from_le_bytes(head[8..12].try_into().unwrap());
    let minor = u32::from_le_bytes(head[12..16].try_into().unwrap());
    if major != 1 {
        return Err(Error::msg(format!("E57 version {major}.{minor} is not supported")));
    }
    let (physical_length, xml_at, xml_length, page_size) = (u64_at(&head, 16), u64_at(&head, 24), u64_at(&head, 32), u64_at(&head, 40));
    if physical_length > source.size() {
        return Err(Error::msg("E57 file is incomplete"));
    }
    if page_size <= CHECKSUM_BYTES {
        return Err(Error::msg("E57 page size is invalid"));
    }
    let paged = Paged {
        source,
        page: page_size,
        data: page_size - CHECKSUM_BYTES,
    };
    let xml = String::from_utf8_lossy(&paged.read(paged.logical(xml_at), xml_length)?).into_owned();
    let root = parse_xml(&xml)?;
    let scans = root
        .child("data3D")
        .map(|d| d.children.iter().enumerate().map(|(i, s)| parse_scan(s, i)).collect::<Result<Vec<_>>>())
        .transpose()?
        .unwrap_or_default();
    if !scans.iter().any(|s| s.record_count > 0) {
        return Err(Error::msg("E57 file holds no points"));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for scan in scans.iter().filter(|s| s.record_count > 0) {
        let Some(b) = scan_extent(&paged, scan)? else { continue };
        for k in 0..3 {
            min[k] = min[k].min(b[k]);
            max[k] = max[k].max(b[k + 3]);
        }
    }
    let step = scans
        .iter()
        .flat_map(|s| s.xyz.iter().flatten().map(|&k| &s.fields[k]))
        .filter_map(|f| match f.codec {
            Codec::Int { scale, .. } if scale > 0.0 => Some(scale),
            _ => None,
        })
        .fold(None, |a: Option<f64>, s| Some(a.map_or(s, |a| a.min(s))))
        .unwrap_or(0.001);
    Ok(E57Header {
        version: format!("{major}.{minor}"),
        point_count: scans.iter().map(|s| s.record_count).sum(),
        min,
        max,
        scale: [step; 3],
        page_size,
        coordinate_metadata: root.child("coordinateMetadata").map(|c| c.text.trim().to_string()).unwrap_or_default(),
        rgb: scans.iter().any(|s| s.record_count > 0) && scans.iter().all(|s| s.record_count == 0 || s.color.is_some()),
        scans,
    })
}

/// The points of an E57 file, batch by batch, scan after scan (`readE57Points`).
pub fn read_e57_points(source: &dyn Source, header: &E57Header, mut o: ReadOptions, each: &mut dyn FnMut(Batch) -> Result<()>) -> Result<()> {
    let paged = Paged {
        source,
        page: header.page_size,
        data: header.page_size - CHECKSUM_BYTES,
    };
    let total_points = header.point_count as f64;
    let mut total_bytes = 0;
    for scan in header.scans.iter().filter(|s| s.record_count > 0) {
        let (data_at, end) = open_section(&paged, scan)?;
        total_bytes += end - data_at;
    }
    let (mut done, mut points) = (0u64, 0u64);
    for scan in &header.scans {
        let read = std::cell::Cell::new(0u64);
        let mut inner = ReadOptions {
            on_progress: None,
            aborted: o.aborted,
        };
        scan_batches(&paged, scan, WINDOW_BYTES, None, header.rgb, &mut inner, &mut |n| read.set(n), &mut |batch| {
            points += batch.records as u64;
            o.progress(Progress {
                bytes: total_bytes.min(done + read.get()) as f64,
                total_bytes: total_bytes as f64,
                points: points as f64,
                total_points,
            });
            each(batch)
        })?;
        done += read.get();
    }
    Ok(())
}
