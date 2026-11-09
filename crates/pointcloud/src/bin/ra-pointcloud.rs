// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! `ra-pointcloud`: one request `{ op, input }` as JSON on stdin, the answer
//! `{ ok }` or `{ error }` on stdout — the crate's functions for scripts and
//! for the JavaScript tests that hold the two to each other.

use std::io::{Read, Write};

fn main() {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).expect("stdin");
    let answer = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(req) => match rail_across_pointcloud::ops::run(req["op"].as_str().unwrap_or(""), &req["input"]) {
            Ok(v) => serde_json::json!({ "ok": v }),
            Err(e) => serde_json::json!({ "error": e.message() }),
        },
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };
    std::io::stdout().write_all(answer.to_string().as_bytes()).expect("stdout");
}
