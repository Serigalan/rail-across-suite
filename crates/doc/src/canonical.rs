// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Canonical JSON and content hashes (`canonical.js`, `sha256.js`): the one
//! byte form of a value, so the browser, the server and the Python core hash
//! the same data alike (data model S-R3).

use rail_across_js::canonical_json;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The SHA-256 of a text (UTF-8), as hex.
pub fn sha256_of_text(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

/// The SHA-256 of bytes, as hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The content hash of `value`: "sha256:" and the SHA-256 of its canonical JSON.
/// A value with a number that is not finite has none, as it has no JSON.
pub fn content_hash(value: &Value) -> String {
    let text = canonical_json(value).unwrap_or_default();
    format!("sha256:{}", sha256_of_text(&text))
}

/// An id of UUID form drawn from `text` — the same text, the same id, on every device.
pub fn derived_id(text: &str) -> String {
    let h = sha256_of_text(text);
    let nibble = u8::from_str_radix(&h[16..17], 16).unwrap_or(0);
    let variant = format!("{:x}", (nibble & 0x3) | 0x8);
    format!("{}-{}-8{}-{}{}-{}", &h[0..8], &h[8..12], &h[13..16], variant, &h[17..20], &h[20..32])
}
