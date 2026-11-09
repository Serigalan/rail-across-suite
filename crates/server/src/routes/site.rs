// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The built suite served from one directory beside the API (`static.js`):
//! the start page at the root, each module's app under its path — so page
//! and API share one origin, as behind Caddy in production.

use std::path::{Component, Path, PathBuf};

use serde_json::json;

use crate::http::{Payload, Req, Resp, percent_decode};

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("wasm") => "application/wasm",
        Some("tif" | "tiff") => "image/tiff",
        Some("woff2") => "font/woff2",
        Some("pbf") => "application/x-protobuf",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The file a request path names inside `root`, or none.
fn file_of(root: &Path, url_path: &str) -> Option<(PathBuf, u64)> {
    if url_path.starts_with("/api/") || url_path == "/api" {
        return None;
    }
    let decoded = percent_decode(url_path.split('?').next().unwrap_or(""));
    let mut full = root.to_path_buf();
    for c in Path::new(&decoded).components() {
        match c {
            Component::Normal(p) => full.push(p),
            Component::ParentDir if !full.pop() || !full.starts_with(root) => return None,
            _ => {}
        }
    }
    if !full.starts_with(root) {
        return None;
    }
    let meta = std::fs::metadata(&full).ok()?;
    if meta.is_dir() {
        let index = full.join("index.html");
        let m = std::fs::metadata(&index).ok().filter(|m| m.is_file())?;
        return Some((index, m.len()));
    }
    let len = meta.len();
    meta.is_file().then_some((full, len))
}

/// One GET or HEAD for a file of the site.
pub fn serve(root: &Path, req: &Req) -> Resp {
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let Some((file, size)) = file_of(&root, &req.path) else {
        return Resp::json(404, json!({ "error": "not_found" }));
    };
    let name = file.to_string_lossy();
    let asset = file.components().any(|c| c.as_os_str() == "assets");
    let cache = if name.ends_with(".html") {
        "no-cache"
    } else if asset {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    };
    let mut start = 0u64;
    let mut end = size.saturating_sub(1);
    let mut status = 200;
    let mut range_header = None;
    if let Some((a, b)) = req.header("range").and_then(|r| r.strip_prefix("bytes=")).and_then(|r| r.split_once('-')) {
        let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
        if digits(a) && digits(b) && (!a.is_empty() || !b.is_empty()) {
            if !a.is_empty() {
                start = a.parse().unwrap_or(u64::MAX);
                if !b.is_empty() {
                    end = end.min(b.parse().unwrap_or(u64::MAX));
                }
            } else {
                start = size.saturating_sub(b.parse().unwrap_or(0));
            }
            if start > end || start >= size {
                return Resp::json(416, json!({ "error": "range_not_satisfiable" })).header("content-range", format!("bytes */{size}"));
            }
            status = 206;
            range_header = Some(format!("bytes {start}-{end}/{size}"));
        }
    }
    let len = if size == 0 { 0 } else { end - start + 1 };
    let mut r = Resp {
        status,
        headers: vec![],
        payload: Payload::File(file.clone(), start, len),
    }
    .header("content-type", content_type(&file))
    .header("accept-ranges", "bytes")
    .header("cache-control", cache)
    .header("x-content-type-options", "nosniff");
    if let Some(h) = range_header {
        r = r.header("content-range", h);
    }
    r
}
