// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An infra-data-1.0 document written as a PlanPro plan, for trying the export
//! on a project at hand: `cargo run -p rail-across-planpro --example write_plan
//! -- <document.json> <plan.planpro>` writes the container file and its
//! `content.xml` beside it; the report goes to stderr. A PlanPro plan
//! (`.ppxml`, `.xml`) is read first, as the crate's tests read one back.

use rail_across_planpro::planpro::planpro_record;
use rail_across_planpro::write::write_planpro;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        eprintln!("usage: write_plan <document.json> <plan.planpro>");
        std::process::exit(2);
    };
    let given = std::fs::read_to_string(input).expect("readable");
    let doc: Value = if given.trim_start().starts_with('<') {
        planpro_record(&given, Some("plan"), None).expect("a PlanPro plan")["record"].clone()
    } else {
        serde_json::from_str(&given).expect("JSON")
    };
    let out = write_planpro(&doc, &json!({ "now": "2026-11-10T12:00:00Z", "version": "dev" })).expect("written");
    std::fs::write(output, decode(out["archive"].as_str().unwrap())).expect("writable");
    std::fs::write(std::path::Path::new(output).with_extension("xml"), out["text"].as_str().unwrap()).expect("writable");
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&json!({ "counts": out["counts"], "notes": out["report"]["notes"] })).unwrap()
    );
}

/// Base64 back into bytes.
fn decode(s: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
    };
    let clean: Vec<u8> = s.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, c)| n | (u32::from(value(*c)) << (18 - 6 * i)));
        out.extend_from_slice(&n.to_be_bytes()[1..chunk.len()]);
    }
    out
}
