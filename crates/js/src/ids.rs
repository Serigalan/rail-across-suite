// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! New ids (`crypto.randomUUID()`): from a queue a caller fills — the replay
//! hands the very ids the JavaScript drew — else from a generator the core's
//! face seeds with the browser's own randomness when it loads the module.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

thread_local! {
    static QUEUE: RefCell<VecDeque<String>> = const { RefCell::new(VecDeque::new()) };
    static STATE: Cell<[u64; 2]> = const { Cell::new([0x853c_49e6_748f_ea9b, 0xda3e_39cb_94b9_5bdb]) };
}

/// Seeds the generator (xorshift128+), as the face does once with `crypto.getRandomValues`.
pub fn seed_ids(a: u64, b: u64) {
    STATE.with(|s| s.set([a | 1, b | 1]));
}

/// The ids the next calls take, in order, before the generator.
pub fn queue_ids(ids: impl IntoIterator<Item = String>) {
    QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        q.clear();
        q.extend(ids);
    });
}

fn next_u64() -> u64 {
    STATE.with(|s| {
        let [mut x, y] = s.get();
        x ^= x << 23;
        let next = x ^ y ^ (x >> 17) ^ (y >> 26);
        s.set([y, next]);
        next.wrapping_add(y)
    })
}

/// A new id: the next queued one, else a version 4 UUID.
pub fn generate_id() -> String {
    if let Some(id) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
        return id;
    }
    let (a, b) = (next_u64(), next_u64());
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&a.to_be_bytes());
    bytes[8..].copy_from_slice(&b.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}
