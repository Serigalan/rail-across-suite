// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Small things the server needs everywhere: ISO times as JavaScript writes
//! them, random tokens and ids, SHA-256, base64.

use rand::RngCore;
use sha2::{Digest, Sha256};

/// `new Date(ms).toISOString()`.
pub fn iso(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    // Civil from days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let (h, mi, s, milli) = (rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000);
    if (0..=9999).contains(&y) {
        format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z")
    } else {
        format!(
            "{}{:06}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z",
            if y < 0 { "-" } else { "+" },
            y.abs()
        )
    }
}

/// `Date.parse` of an ISO time as `iso` writes it (and of a plain date); None where it is none.
pub fn parse_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> { s.get(from..from + len)?.parse().ok() };
    let (y, m, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return None;
    }
    let (mut h, mut mi, mut sec, mut ms) = (0, 0, 0, 0);
    if s.len() > 10 {
        if b.get(10) != Some(&b'T') {
            return None;
        }
        h = num(11, 2)?;
        mi = num(14, 2)?;
        if b.get(16) == Some(&b':') {
            sec = num(17, 2)?;
            if b.get(19) == Some(&b'.') {
                let frac: String = s[20..].chars().take_while(char::is_ascii_digit).collect();
                ms = format!("{frac:0<3}")[..3].parse().ok()?;
            }
        }
    }
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400_000 + h * 3_600_000 + mi * 60_000 + sec * 1000 + ms)
}

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b
}

/// `crypto.randomUUID()`.
pub fn uuid() -> String {
    let mut b = random_bytes(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex(&b);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn sha256_hex(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64url without padding (`toString('base64url')`).
pub fn base64url(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        for k in 0..=chunk.len() {
            let c = B64[(n >> (18 - 6 * k) & 63) as usize];
            out.push(match c {
                b'+' => '-',
                b'/' => '_',
                c => c as char,
            });
        }
    }
    out
}

/// `Buffer.from(text, 'base64')`: standard or url alphabet, what is not of it passed over.
pub fn from_base64(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// A JavaScript number as JSON: integral values without a fraction.
pub fn n(x: f64) -> serde_json::Value {
    rail_across_js::number(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_as_javascript_writes_them() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_790_841_600_000), "2026-10-01T08:00:00.000Z");
        assert_eq!(parse_iso("2026-10-01T08:00:00.000Z"), Some(1_790_841_600_000));
        assert_eq!(parse_iso("2026-10-01T08:00:00Z"), Some(1_790_841_600_000));
        assert_eq!(iso(-1), "1969-12-31T23:59:59.999Z");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(from_base64("-_8"), vec![0xfb, 0xff]);
    }
}
