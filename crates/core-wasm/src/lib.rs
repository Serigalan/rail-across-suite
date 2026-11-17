// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The core's crates as one WebAssembly module (decision 0005), with no
//! bindings generator — as the simulation and the optimizer: the caller
//! allocates a buffer in the module's memory, writes the request's JSON
//! (UTF-8) into it, calls `run`, and reads the answer's JSON from the pointer
//! and length it is given back. A request is `{"op", "input"}`; the answer is
//! `{"ok": …}` or `{"error", "detail"?, "data"?}` (see [`dispatch`]). The core's face,
//! `packages/core/src/native/`, does this for the JavaScript.

use std::cell::Cell;

use serde_json::{Value, json};

mod args;
mod hydrate;
mod ops;
mod ops_commands;
mod ops_geom;
mod ops_planpro;
mod ops_rules;
mod ops_session;
mod ops_signalling;
mod ops_survey;
mod ops_switch;
mod ops_track;
mod ops_views;

thread_local! {
    static RESULT_LEN: Cell<usize> = const { Cell::new(0) };
}

/// One request as JSON text, answered as JSON text: `{"ok": …}` or `{"error", "detail"?}`.
pub fn dispatch(request: &str) -> String {
    let answer = match serde_json::from_str::<Value>(request) {
        Ok(req) => {
            let op = req.get("op").and_then(Value::as_str).unwrap_or("");
            let input = req.get("input").cloned().unwrap_or(Value::Null);
            // The ids a call takes before the generator (the replay's, as the JavaScript drew them).
            if let Some(ids) = req.get("ids").and_then(Value::as_array) {
                rail_across_js::queue_ids(ids.iter().filter_map(Value::as_str).map(str::to_string));
            }
            match ops::run(op, input) {
                Ok(value) => json!({ "ok": value }),
                Err(ops::Failure { error, detail, data }) => json!({ "error": error, "detail": detail, "data": data }),
            }
        }
        Err(err) => json!({ "error": "invalid_request", "detail": err.to_string() }),
    };
    answer.to_string()
}

/// Allocates `len` bytes in the module's memory.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees `len` bytes at `ptr`, allocated by `alloc` or returned by `run`.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `run` and `result_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller hands back a buffer of this module of that capacity.
    unsafe { drop(Vec::from_raw_parts(ptr, 0, len.max(1))) };
}

/// Runs the request of `len` bytes at `ptr` (and frees them); returns the
/// answer's JSON, whose length `result_len` tells.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(ptr: *mut u8, len: usize) -> *mut u8 {
    // SAFETY: the caller wrote `len` bytes into a buffer from `alloc(len)`.
    let input = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let output = match std::str::from_utf8(&input) {
        Ok(request) => dispatch(request),
        Err(_) => r#"{"error":"invalid_request","detail":"the request is not UTF-8"}"#.to_string(),
    };
    let bytes = output.into_bytes();
    let len = bytes.len();
    RESULT_LEN.with(|l| l.set(len));
    let out = alloc(len);
    // SAFETY: `out` has room for `len` bytes, which do not overlap `bytes`.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, len) };
    out
}

/// The length of the last answer `run` returned.
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    RESULT_LEN.with(|l| l.get())
}
