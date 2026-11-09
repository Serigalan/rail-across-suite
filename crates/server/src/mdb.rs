// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An Access database (DB ASCII interface, Satzarten 11–41) turned into the
//! import payload (`mdb.js`): the tables lifted out with `mdb-export` from
//! mdbtools, one child process per table; their meaning is the Layout-Tool's.

use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

const MAGIC: &[u8] = b"\x00\x01\x00\x00Standard Jet DB";
const MAGIC_ACE: &[u8] = b"\x00\x01\x00\x00Standard ACE DB";

#[derive(Debug, Clone)]
pub struct MdbError {
    pub code: String,
    pub message: String,
}

impl MdbError {
    fn new(code: &str, message: &str) -> Self {
        MdbError {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Rows of a CSV as mdb-export writes it (`parseCsv`).
pub fn parse_csv(text: &str) -> Vec<Vec<(String, String)>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if quoted {
            if c == '"' && chars.get(i + 1) == Some(&'"') {
                field.push('"');
                i += 1;
            } else if c == '"' {
                quoted = false;
            } else {
                field.push(c);
            }
        } else if c == '"' {
            quoted = true;
        } else if c == ',' {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.get(i + 1) == Some(&'\n') {
                i += 1;
            }
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else {
            field.push(c);
        }
        i += 1;
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    let rows: Vec<Vec<String>> = rows.into_iter().filter(|r| r.len() > 1 || !r[0].is_empty()).collect();
    let Some((head, body)) = rows.split_first() else { return vec![] };
    body.iter()
        .map(|r| {
            head.iter()
                .enumerate()
                .map(|(k, h)| (h.clone(), r.get(k).cloned().unwrap_or_default()))
                .collect()
        })
        .collect()
}

fn num(v: Option<&String>) -> Option<f64> {
    let s = v?;
    if rail_across_js::js_trim(s).is_empty() {
        return None;
    }
    let x = rail_across_js::js_number(Some(&Value::String(s.clone())));
    x.is_finite().then_some(x)
}

fn int(v: Option<&String>, fallback: Option<f64>) -> Option<f64> {
    num(v).map(f64::trunc).or(fallback)
}

fn txt(v: Option<&String>) -> String {
    rail_across_js::js_trim(v.map(String::as_str).unwrap_or("")).to_string()
}

fn mp(m: Option<&String>, e: Option<&String>) -> Option<f64> {
    num(m).map(|m| m * 10f64.powf(int(e, Some(0.0)).unwrap()))
}

fn o(v: Option<f64>) -> Value {
    v.map(crate::util::n).unwrap_or(Value::Null)
}

type Row = HashMap<String, String>;

fn tool() -> String {
    std::env::var("RA_MDB_EXPORT").unwrap_or_else(|_| "mdb-export".into())
}

fn timeout() -> Duration {
    Duration::from_secs_f64(std::env::var("RA_MDB_TIMEOUT").ok().and_then(|v| v.parse().ok()).unwrap_or(180.0))
}

/// One table as rows, without the records marked deleted; a table the file lacks yields nothing.
fn rows(path: &std::path::Path, table: &str) -> Result<Vec<Row>, MdbError> {
    let mut cmd = Command::new(tool());
    cmd.arg(path).arg(table).stdout(Stdio::piped()).stderr(Stdio::null());
    // mdb-export bundled with its libraries (the desktop app): RA_MDB_LIBS is where they lie.
    if let Ok(libs) = std::env::var("RA_MDB_LIBS") {
        let var = if cfg!(target_os = "macos") { "DYLD_LIBRARY_PATH" } else { "LD_LIBRARY_PATH" };
        let joined = match std::env::var(var) {
            Ok(old) if !old.is_empty() => format!("{libs}:{old}"),
            _ => libs,
        };
        cmd.env(var, joined);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(MdbError::new("internal", "mdb-export not installed")),
        Err(_) => return Ok(vec![]),
    };
    let mut out = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = out.read_to_end(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| MdbError::new("internal", &e.to_string()))? {
            break st;
        }
        if start.elapsed() > timeout() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(MdbError::new("timeout", "timeout"));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let bytes = reader.join().unwrap_or_default();
    if !status.success() {
        return Ok(vec![]);
    }
    Ok(parse_csv(&String::from_utf8_lossy(&bytes))
        .into_iter()
        .map(|r| r.into_iter().collect::<Row>())
        .filter(|r| int(r.get("bDel"), Some(0.0)) == Some(0.0))
        .collect())
}

/// The payload the Layout-Tool's `parseMdbPayload` reads (`convertMdb`).
pub fn convert_mdb(bytes: &[u8]) -> Result<Value, MdbError> {
    let head = &bytes[..bytes.len().min(MAGIC.len())];
    if head != MAGIC && head != MAGIC_ACE {
        return Err(MdbError::new("not_a_database", "not_a_database"));
    }
    let dir = std::env::temp_dir().join(format!("ra-mdb-{}", crate::util::uuid()));
    std::fs::create_dir_all(&dir).map_err(|e| MdbError::new("internal", &e.to_string()))?;
    let path = dir.join("upload.mdb");
    let result = (|| {
        std::fs::write(&path, bytes).map_err(|e| MdbError::new("internal", &e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        let t = |table: &str| rows(&path, table);
        let version = t("TABSYS_GNDED")?.iter().find_map(|r| int(r.get("TABSYS_VERSION"), None));
        let points: Vec<Value> = t("X_ASC12_PL")?
            .iter()
            .map(|r| {
                (
                    txt(r.get("PAD")),
                    txt(r.get("LSYS")),
                    num(r.get("Y")),
                    num(r.get("X")),
                    mp(r.get("MP"), r.get("MPEXP")),
                )
            })
            .filter(|p| !p.0.is_empty() && p.2.is_some() && p.3.is_some())
            .map(|p| json!({ "pad": p.0, "sys": p.1, "y": o(p.2), "x": o(p.3), "mp": o(p.4) }))
            .collect();
        let elements: Vec<Value> = t("X_ASC21_EL")?
            .iter()
            .map(|r| {
                json!({
                    "pad1": txt(r.get("PAD1")), "pad2": txt(r.get("PAD2")), "sys": txt(r.get("ELSYS")), "typ": o(int(r.get("ELTYP"), Some(-1.0))),
                    "p1": o(num(r.get("ELPAR1"))), "p2": o(num(r.get("ELPAR2"))), "p3": o(num(r.get("ELPAR3"))), "ariwi": o(num(r.get("ELARIWI"))),
                    "text": txt(r.get("ELTEXT")), "err": o(int(r.get("ErrStatus"), Some(0.0))),
                })
            })
            .collect();
        let cants: Vec<Value> = t("X_ASC23_EU")?
            .iter()
            .map(|r| {
                json!({
                    "pad1": txt(r.get("PAD1")), "pad2": txt(r.get("PAD2")), "typ": o(int(r.get("EUTYP"), Some(-1.0))),
                    "p1": o(num(r.get("EUPAR1"))), "p2": o(num(r.get("EUPAR2"))), "p3": o(num(r.get("EUPAR3"))), "err": o(int(r.get("ErrStatus"), Some(0.0))),
                })
            })
            .collect();
        let mut chains: Vec<(String, Vec<(f64, String)>)> = Vec::new();
        for r in t("X_ASC33_GS_L")? {
            let key = txt(r.get("ParseLfdNr"));
            let item = (int(r.get("LfdNr"), Some(0.0)).unwrap(), txt(r.get("L")));
            match chains.iter_mut().find(|(k, _)| *k == key) {
                Some((_, l)) => l.push(item),
                None => chains.push((key, vec![item])),
            }
        }
        let tracks: Vec<Value> = t("X_ASC33_GS")?
            .iter()
            .map(|r| {
                let key = txt(r.get("ParseLfdNr"));
                let mut chain = chains.iter().find(|(k, _)| *k == key).map(|(_, l)| l.clone()).unwrap_or_default();
                chain.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let chain: Vec<String> = chain.into_iter().map(|(_, p)| p).filter(|p| !p.is_empty()).collect();
                json!({
                    "id": key, "a": txt(r.get("AKNOTEN")), "e": txt(r.get("EKNOTEN")), "strecke": txt(r.get("STRECKE")), "gleis": txt(r.get("GLEISNR")),
                    "richtung": o(int(r.get("STRRIKZ"), Some(0.0))), "chain": chain, "err": o(int(r.get("ErrStatus"), Some(0.0))),
                })
            })
            .collect();
        let stations: Vec<Value> = t("X_ASC11_PP")?
            .iter()
            .map(|r| json!({ "pad": txt(r.get("PAD")), "station": o(num(r.get("STATION"))), "strecke": txt(r.get("PSTRECKE")), "richtung": o(int(r.get("PSTRRIKZ"), Some(0.0))) }))
            .filter(|s| s["pad"] != "")
            .collect();
        let heights: Vec<Value> = t("X_ASC13_PH")?
            .iter()
            .map(|r| (txt(r.get("PAD")), txt(r.get("HSYS")), num(r.get("H"))))
            .filter(|h| !h.0.is_empty() && h.2.is_some())
            .map(|h| json!({ "pad": h.0, "sys": h.1, "h": o(h.2) }))
            .collect();
        let gradients: Vec<Value> = t("X_ASC22_EH")?
            .iter()
            .map(|r| {
                json!({
                    "pad1": txt(r.get("PAD1")), "pad2": txt(r.get("PAD2")), "sys": txt(r.get("EHSYS")), "typ": o(int(r.get("EHTYP"), Some(-1.0))),
                    "p1": o(num(r.get("EHPAR1"))), "p2": o(num(r.get("EHPAR2"))), "p3": o(num(r.get("EHPAR3"))), "err": o(int(r.get("ErrStatus"), Some(0.0))),
                })
            })
            .collect();
        let km: Vec<Value> = t("X_ASC24_EK")?
            .iter()
            .map(|r| {
                json!({
                    "pad1": txt(r.get("PAD1")), "pad2": txt(r.get("PAD2")), "sys": txt(r.get("EKSYS")), "typ": o(int(r.get("EKTYP"), Some(-1.0))),
                    "p1": o(num(r.get("EKPAR1"))), "p2": o(num(r.get("EKPAR2"))), "p3": o(num(r.get("EKPAR3"))), "ariwi": o(num(r.get("EKARIWI"))),
                    "akm": o(num(r.get("EKAKM"))), "ekm": o(num(r.get("EKEKM"))),
                })
            })
            .collect();
        let nodes: Vec<Value> = t("X_ASC31_KN")?
            .iter()
            .map(|r| {
                json!({
                    "knoten": r.get("KNOTEN").cloned().unwrap_or_default(), "typ": o(int(r.get("KNTYP"), Some(-1.0))), "pad": txt(r.get("PAD")), "form": txt(r.get("KNBE")),
                    "nb": (["KN0", "KN1", "KN2", "KN3"].iter().map(|k| txt(r.get(*k))).collect::<Vec<_>>()), "err": o(int(r.get("ErrStatus"), Some(0.0))),
                })
            })
            .collect();
        if elements.is_empty() && nodes.is_empty() {
            return Err(MdbError::new("no_records", "no_records"));
        }
        let all: [(&str, Vec<Value>); 9] = [
            ("points", points),
            ("elements", elements),
            ("cants", cants),
            ("tracks", tracks),
            ("nodes", nodes),
            ("stations", stations),
            ("heights", heights),
            ("gradients", gradients),
            ("km", km),
        ];
        let mut out = Map::new();
        out.insert("version".into(), o(version));
        let mut counts = Map::new();
        for (k, v) in all {
            counts.insert(k.into(), json!(v.len()));
            out.insert(k.into(), Value::Array(v));
        }
        out.insert("counts".into(), Value::Object(counts));
        Ok(Value::Object(out))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
