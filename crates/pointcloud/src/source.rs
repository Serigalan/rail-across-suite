// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Where a file's bytes come from: a file on disk or bytes in memory, read
//! in pieces (`{ size, read(offset, length) }` of the JavaScript).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use crate::Result;

/// A file read in pieces. A read past the end gives what there is.
pub trait Source: Send + Sync {
    fn size(&self) -> u64;
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>>;
}

/// A file on disk.
pub struct FileSource {
    file: Mutex<File>,
    size: u64,
}

impl FileSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        Ok(FileSource { file: Mutex::new(file), size })
    }
}

impl Source for FileSource {
    fn size(&self) -> u64 {
        self.size
    }
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(offset))?;
        let mut buf = Vec::with_capacity(length);
        f.by_ref().take(length as u64).read_to_end(&mut buf)?;
        Ok(buf)
    }
}

/// Bytes in memory.
pub struct BytesSource(pub Vec<u8>);

impl Source for BytesSource {
    fn size(&self) -> u64 {
        self.0.len() as u64
    }
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let from = (offset as usize).min(self.0.len());
        let to = from.saturating_add(length).min(self.0.len());
        Ok(self.0[from..to].to_vec())
    }
}

/// A source as `Read + Seek`, for the LAZ decompressor.
pub struct SourceReader<'a> {
    pub source: &'a dyn Source,
    pub pos: u64,
}

impl Read for SourceReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.source.read(self.pos, buf.len()).map_err(|e| std::io::Error::other(e.message()))?;
        buf[..bytes.len()].copy_from_slice(&bytes);
        self.pos += bytes.len() as u64;
        Ok(bytes.len())
    }
}

impl Seek for SourceReader<'_> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let size = self.source.size() as i64;
        let pos = match to {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::End(d) => size + d,
            SeekFrom::Current(d) => self.pos as i64 + d,
        };
        if pos < 0 {
            return Err(std::io::Error::other("seek before the start"));
        }
        self.pos = pos as u64;
        Ok(self.pos)
    }
}

/// Where a cloud's tile file lies, read in several ranges at once (`readMany`).
pub trait TileSource: Send + Sync {
    fn read_many(&self, ranges: &[(u64, usize)]) -> Result<Vec<Vec<u8>>>;
}

/// A tile file on disk.
pub struct TileFile(pub FileSource);

impl TileSource for TileFile {
    fn read_many(&self, ranges: &[(u64, usize)]) -> Result<Vec<Vec<u8>>> {
        ranges.iter().map(|&(o, l)| self.0.read(o, l)).collect()
    }
}

/// A tile file in memory.
pub struct TileBytes(pub Vec<u8>);

impl TileSource for TileBytes {
    fn read_many(&self, ranges: &[(u64, usize)]) -> Result<Vec<Vec<u8>>> {
        Ok(ranges
            .iter()
            .map(|&(o, l)| {
                let from = (o as usize).min(self.0.len());
                self.0[from..(from + l).min(self.0.len())].to_vec()
            })
            .collect())
    }
}
