// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! `ra-sim [request.json]` — simulates the request (from the file, or from
//! standard input) and writes the result's JSON to standard output. Exits
//! with 1 when the simulation fails, the error's JSON still written.

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") || args.len() > 1 {
        eprintln!("usage: ra-sim [request.json]   (reads standard input without a file)");
        return ExitCode::from(2);
    }
    let request = match args.first() {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).map(|_| s)
        }
    };
    let request = match request {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ra-sim: {e}");
            return ExitCode::from(2);
        }
    };
    let result = rail_across_sim::simulate_json(&request);
    println!("{result}");
    if result.starts_with(r#"{"error""#) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
