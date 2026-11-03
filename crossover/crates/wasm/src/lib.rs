// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The search of a switch connection as a WebAssembly module, with no
//! bindings generator — as the simulation module (`sim/crates/wasm`): the
//! caller allocates a buffer in the module's memory, writes the request's
//! JSON (UTF-8) into it, calls `solve`, and reads the answer's JSON from the
//! pointer and length it is given back.
//!
//! ```js
//! const { instance } = await WebAssembly.instantiate(bytes)
//! const { memory, alloc, dealloc, solve, result_len } = instance.exports
//! const input = new TextEncoder().encode(JSON.stringify(request))
//! const ptr = alloc(input.length)
//! new Uint8Array(memory.buffer, ptr, input.length).set(input)
//! const out = solve(ptr, input.length)               // frees the input
//! const json = new TextDecoder().decode(new Uint8Array(memory.buffer, out, result_len()))
//! dealloc(out, result_len())
//! ```

use std::cell::Cell;

thread_local! {
    static RESULT_LEN: Cell<usize> = const { Cell::new(0) };
}

/// Allocates `len` bytes in the module's memory.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees `len` bytes at `ptr`, allocated by `alloc` or returned by `solve`.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `solve` and `result_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller hands back a buffer of this module of that capacity.
    unsafe { drop(Vec::from_raw_parts(ptr, 0, len.max(1))) };
}

/// Solves the request of `len` bytes at `ptr` (and frees them); returns the
/// answer's JSON, whose length `result_len` tells.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn solve(ptr: *mut u8, len: usize) -> *mut u8 {
    // SAFETY: the caller wrote `len` bytes into a buffer from `alloc(len)`.
    let input = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let output = match std::str::from_utf8(&input) {
        Ok(request) => rail_across_crossover::solve_json(request),
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

/// The length of the last answer `solve` returned.
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    RESULT_LEN.with(|l| l.get())
}
