// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The header of a LAS or LAZ file (ASPRS LAS 1.0–1.4) and its points, read
//! in pieces (`lasHeader.js`, `lasReader.js`). LAZ is decoded chunk by chunk
//! with the `laz` crate (laz-rs), as laz-perf did for the JavaScript; a batch
//! is one chunk, as there.

use serde_json::{Value, json};

use crate::source::{Source, SourceReader};
use crate::{Batch, Error, Progress, ReadOptions, Result};

/// How many bytes of the file are enough to read the header and its VLRs.
pub const HEADER_PROBE_BYTES: usize = 64 * 1024;
const LASZIP_USER_ID: &str = "laszip encoded";
const LASZIP_RECORD_ID: u16 = 22204;
/// chunk_size of a LAZ file whose chunks differ in length.
pub const VARIABLE_CHUNK_SIZE: u32 = 0xFFFF_FFFF;
const LAS_BATCH: u64 = 50000;
const MAX_TABLE_BYTES: u64 = 64 * 1024 * 1024;

fn rgb_offset(format: u8) -> Option<usize> {
    match format {
        2 => Some(20),
        3 | 5 => Some(28),
        7 | 8 | 10 => Some(30),
        _ => None,
    }
}

/// The LASzip record of a LAZ file: how it is compressed.
#[derive(Clone, Debug)]
pub struct Laszip {
    pub compressor: u16,
    pub coder: u16,
    pub version: String,
    pub chunk_size: u32,
    /// The record as stored, for the decompressor.
    pub record: Vec<u8>,
}

/// What the import needs of a LAS/LAZ header.
#[derive(Clone, Debug)]
pub struct LasHeader {
    pub version: String,
    pub compressed: bool,
    pub point_format: u8,
    pub point_length: usize,
    pub point_count: u64,
    pub point_data_offset: u64,
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub laszip: Option<Laszip>,
    pub rgb: bool,
    pub rgb_offset: Option<usize>,
}

impl LasHeader {
    pub fn to_json(&self) -> Value {
        let n = |v: &[f64; 3]| json!(v.iter().map(|x| rail_across_js::number(*x)).collect::<Vec<_>>());
        json!({
            "version": self.version,
            "compressed": self.compressed,
            "pointFormat": self.point_format,
            "pointLength": self.point_length,
            "pointCount": self.point_count,
            "pointDataOffset": self.point_data_offset,
            "scale": n(&self.scale),
            "offset": n(&self.offset),
            "min": n(&self.min),
            "max": n(&self.max),
            "laszip": self.laszip.as_ref().map(|l| json!({ "compressor": l.compressor, "coder": l.coder, "version": l.version, "chunkSize": l.chunk_size })),
            "rgb": self.rgb,
            "rgbOffset": self.rgb_offset,
        })
    }
}

struct View<'a>(&'a [u8]);

impl View<'_> {
    fn bytes<const N: usize>(&self, at: usize) -> Result<[u8; N]> {
        self.0
            .get(at..at + N)
            .map(|b| b.try_into().unwrap())
            .ok_or_else(|| Error::msg("Offset is outside the bounds of the DataView"))
    }
    fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(at)?))
    }
    fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(at)?))
    }
    fn i32(&self, at: usize) -> Result<i32> {
        Ok(i32::from_le_bytes(self.bytes(at)?))
    }
    fn u64(&self, at: usize) -> Result<u64> {
        Ok(u64::from_le_bytes(self.bytes(at)?))
    }
    fn i64(&self, at: usize) -> Result<i64> {
        Ok(i64::from_le_bytes(self.bytes(at)?))
    }
    fn f64(&self, at: usize) -> Result<f64> {
        Ok(f64::from_le_bytes(self.bytes(at)?))
    }
}

fn ascii(bytes: &[u8], from: usize, length: usize) -> String {
    bytes.iter().skip(from).take(length).take_while(|b| **b != 0).map(|b| *b as char).collect()
}

/// The header of a LAS/LAZ file from its first bytes (`parseLasHeader`).
pub fn parse_las_header(bytes: &[u8]) -> Result<LasHeader> {
    if bytes.len() < 227 || ascii(bytes, 0, 4) != "LASF" {
        return Err(Error::msg("not a LAS file"));
    }
    let v = View(bytes);
    let (major, minor) = (bytes[24], bytes[25]);
    let header_size = v.u16(94)? as usize;
    let point_data_offset = u64::from(v.u32(96)?);
    let vlr_count = v.u32(100)?;
    let format_byte = bytes[104];
    let compressed = format_byte & 0xC0 != 0;
    let point_format = format_byte & 0x3F;
    let point_length = v.u16(105)? as usize;
    let mut point_count = u64::from(v.u32(107)?);
    if minor >= 4 && header_size >= 375 {
        let big = v.u64(247)?;
        if big > 0 {
            point_count = big;
        }
    }
    let three = |a: usize, b: usize, c: usize| -> Result<[f64; 3]> { Ok([v.f64(a)?, v.f64(b)?, v.f64(c)?]) };
    let scale = three(131, 139, 147)?;
    let offset = three(155, 163, 171)?;
    let max = three(179, 195, 211)?;
    let min = three(187, 203, 219)?;

    let mut laszip = None;
    let mut at = header_size;
    let mut i = 0;
    while i < vlr_count && at + 54 <= bytes.len() {
        let user_id = ascii(bytes, at + 2, 16);
        let record_id = v.u16(at + 18)?;
        let length = v.u16(at + 20)? as usize;
        let body = at + 54;
        if user_id == LASZIP_USER_ID && record_id == LASZIP_RECORD_ID && body + 34 <= bytes.len() {
            laszip = Some(Laszip {
                compressor: v.u16(body)?,
                coder: v.u16(body + 2)?,
                version: format!("{}.{}r{}", bytes[body + 4], bytes[body + 5], v.u16(body + 6)?),
                chunk_size: v.u32(body + 12)?,
                record: bytes[body..(body + length).min(bytes.len())].to_vec(),
            });
        }
        at = body + length;
        i += 1;
    }
    if compressed && laszip.is_none() {
        return Err(Error::msg("LAZ file without a LASzip record"));
    }
    if let Some(l) = laszip.as_ref().filter(|_| compressed) {
        if l.compressor != 2 && l.compressor != 3 {
            return Err(Error::msg(format!("LAZ compressor {} is not supported", l.compressor)));
        }
    }
    let rgb_offset = rgb_offset(point_format);
    Ok(LasHeader {
        version: format!("{major}.{minor}"),
        compressed,
        point_format,
        point_length,
        point_count,
        point_data_offset,
        scale,
        offset,
        min,
        max,
        laszip,
        rgb: rgb_offset.is_some_and(|o| point_length >= o + 6),
        rgb_offset,
    })
}

/// The header of the file behind a source (`readLasHeader`).
pub fn read_las_header(source: &dyn Source) -> Result<LasHeader> {
    parse_las_header(&source.read(0, (source.size() as usize).min(HEADER_PROBE_BYTES))?)
}

/// One chunk of a LAZ file: where it lies and how many points it holds.
#[derive(Clone, Copy, Debug)]
pub struct Chunk {
    pub offset: u64,
    pub bytes: u64,
    pub points: u64,
}

/// Where each chunk of a LAZ file lies (`lazChunks`).
pub fn laz_chunks(source: &dyn Source, header: &LasHeader) -> Result<Vec<Chunk>> {
    let head = source.read(header.point_data_offset, 8)?;
    let mut table_at = View(&head).i64(0)?;
    if table_at == -1 {
        let tail = source.read(source.size() - 8, 8)?;
        table_at = View(&tail).i64(0)?;
    }
    if !(table_at > header.point_data_offset as i64) || table_at as u64 >= source.size() {
        return Err(Error::msg("LAZ chunk table not found"));
    }
    let table_at = table_at as u64;
    let table = source.read(table_at, (source.size() - table_at).min(MAX_TABLE_BYTES) as usize)?;
    let chunk_size = header.laszip.as_ref().map(|l| l.chunk_size).unwrap_or(0);
    let variable = chunk_size == VARIABLE_CHUNK_SIZE;
    let entries = laz::laszip::ChunkTable::read(&mut std::io::Cursor::new(&table), variable).map_err(|e| Error::msg(e.to_string()))?;
    let mut offset = header.point_data_offset + 8;
    let mut remaining = header.point_count;
    let mut out = Vec::new();
    for e in entries.as_ref() {
        let points = if variable { e.point_count } else { u64::from(chunk_size).min(remaining) };
        remaining = remaining.saturating_sub(points);
        out.push(Chunk {
            offset,
            bytes: e.byte_count,
            points,
        });
        offset += e.byte_count;
    }
    Ok(out)
}

fn fill(batch: &mut Batch, i: usize, rec: &[u8], header: &LasHeader) -> Result<()> {
    let v = View(rec);
    let [sx, sy, sz] = header.scale;
    let [ox, oy, oz] = header.offset;
    batch.x[i] = f64::from(v.i32(0)?) * sx + ox;
    batch.y[i] = f64::from(v.i32(4)?) * sy + oy;
    batch.z[i] = f64::from(v.i32(8)?) * sz + oz;
    batch.intensity[i] = v.u16(12)?;
    if header.rgb {
        let at = header.rgb_offset.unwrap_or(0);
        batch.red.as_mut().unwrap()[i] = v.u16(at)?;
        batch.green.as_mut().unwrap()[i] = v.u16(at + 2)?;
        batch.blue.as_mut().unwrap()[i] = v.u16(at + 4)?;
    }
    Ok(())
}

/// The points of a LAS/LAZ file, batch by batch, handed to `each` (`readLasPoints`).
pub fn read_las_points(source: &dyn Source, header: &LasHeader, mut o: ReadOptions, each: &mut dyn FnMut(Batch) -> Result<()>) -> Result<()> {
    let total_points = header.point_count;
    let mut total_bytes = source.size().saturating_sub(header.point_data_offset);
    let mut points = 0u64;
    let len = header.point_length;
    if !header.compressed {
        let mut first = 0;
        while first < total_points {
            o.check()?;
            let n = LAS_BATCH.min(total_points - first);
            let bytes = source.read(header.point_data_offset + first * len as u64, n as usize * len)?;
            let mut batch = Batch::new(n as usize, header.rgb);
            for i in 0..n as usize {
                fill(&mut batch, i, bytes.get(i * len..).unwrap_or(&[]), header)?;
            }
            points += n;
            o.progress(Progress {
                bytes: ((first + n) * len as u64) as f64,
                total_bytes: total_bytes as f64,
                points: points as f64,
                total_points: total_points as f64,
            });
            each(batch)?;
            first += LAS_BATCH;
        }
        return Ok(());
    }

    let chunks = laz_chunks(source, header)?;
    if let Some(last) = chunks.last() {
        total_bytes = last.offset + last.bytes - header.point_data_offset;
    }
    let laszip = header.laszip.as_ref().unwrap();
    let vlr = laz::LazVlr::from_buffer(&laszip.record).map_err(|e| Error::msg(e.to_string()))?;
    // The decompressor asks for a few bytes at a time: a megabyte read ahead
    // keeps that from becoming millions of reads of the source (the page's file
    // in the browser's worker).
    let reader = std::io::BufReader::with_capacity(
        1 << 20,
        SourceReader {
            source,
            pos: header.point_data_offset,
        },
    );
    let mut decompressor = laz::LasZipDecompressor::new(reader, vlr).map_err(|e| Error::msg(e.to_string()))?;
    let mut buf = Vec::new();
    for chunk in &chunks {
        o.check()?;
        let n = chunk.points as usize;
        buf.resize(n * len, 0);
        decompressor.decompress_many(&mut buf).map_err(|e| Error::msg(e.to_string()))?;
        let mut batch = Batch::new(n, header.rgb);
        for i in 0..n {
            fill(&mut batch, i, &buf[i * len..(i + 1) * len], header)?;
        }
        points += chunk.points;
        o.progress(Progress {
            bytes: (chunk.offset + chunk.bytes - header.point_data_offset) as f64,
            total_bytes: total_bytes as f64,
            points: points as f64,
            total_points: total_points as f64,
        });
        each(batch)?;
    }
    Ok(())
}
