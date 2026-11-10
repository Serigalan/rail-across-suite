// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The container file of a plan (`.planpro`), as PlanPro tools exchange it: a
//! ZIP of the plan (`content.xml`) and its `manifest.xml`. The entries are
//! stored, not compressed — nothing to depend on, in the WebAssembly module as
//! on the server — and handed over in Base64, the way JSON carries bytes.

/// The manifest of a container holding the plan alone, no media.
pub const MANIFEST: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n\
<manifest>\n\n  <contentList>\n    <content name=\"content.xml\" type=\"text/xml\"/>\n  </contentList>\n\n  <mediaList/>\n\n</manifest>\n";

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!0u32, |c, b| CRC_TABLE[((c ^ u32::from(*b)) & 0xff) as usize] ^ (c >> 8))
}

/// The MS-DOS time and date of an ISO time stamp (`2026-11-10T12:00:00Z`), 1980-01-01 without one.
fn dos_time(stamp: Option<&str>) -> (u16, u16) {
    let n = |s: Option<&str>, a: usize, b: usize| s.and_then(|s| s.get(a..b)).and_then(|x| x.parse::<u16>().ok());
    match (n(stamp, 0, 4), n(stamp, 5, 7), n(stamp, 8, 10)) {
        (Some(y), Some(mo), Some(d)) if y >= 1980 => {
            let (h, mi, s) = (n(stamp, 11, 13).unwrap_or(0), n(stamp, 14, 16).unwrap_or(0), n(stamp, 17, 19).unwrap_or(0));
            ((h << 11) | (mi << 5) | (s / 2), ((y - 1980) << 9) | (mo << 5) | d)
        }
        _ => (0, (1 << 5) | 1),
    }
}

/// A ZIP of the files, each stored as it is, stamped with `stamp`.
pub fn zip_stored(files: &[(&str, &[u8])], stamp: Option<&str>) -> Vec<u8> {
    let (time, date) = dos_time(stamp);
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let (crc, size, name_len) = (crc32(data), data.len() as u32, name.len() as u16);
        // Local header: version 2.0, flag bit 11 (UTF-8 names), method 0 (stored).
        let head = |sig: u32, central: bool| {
            let mut h = Vec::new();
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes());
            }
            for v in [20u16, 0x0800, 0, time, date] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            for v in [crc, size, size] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            h.extend_from_slice(&name_len.to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes());
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend(head(0x0201_4b50, true));
        // Comment length, disk, internal and external attributes, offset of the local header.
        for v in [0u16, 0, 0] {
            central.extend_from_slice(&v.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    let count = files.len() as u16;
    for v in [0u16, 0, count, count] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// The bytes in Base64 (RFC 4648, padded).
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= chunk.len() { ABC[((n >> shift) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_and_base64_as_the_standards_have_them() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn a_zip_lists_its_entries_at_the_end() {
        let zip = zip_stored(&[("content.xml", b"<a/>"), ("manifest.xml", MANIFEST.as_bytes())], Some("2026-11-10T12:30:00Z"));
        assert_eq!(&zip[..4], b"PK\x03\x04");
        let end = &zip[zip.len() - 22..];
        assert_eq!(&end[..4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([end[10], end[11]]), 2);
        assert_eq!(dos_time(Some("2026-11-10T12:30:00Z")), ((12 << 11) | (30 << 5), (46 << 9) | (11 << 5) | 10));
    }
}
