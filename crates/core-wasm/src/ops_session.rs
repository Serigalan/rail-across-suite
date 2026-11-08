// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The project held in the module's memory (decision 0020, step 5): opened
//! once, edited in place, and a drag sends only what it changes — the
//! measurement showed that carrying the whole project across JSON on every
//! call costs more than the edit itself.

use std::cell::RefCell;

use rail_across_commands::edit;
use rail_across_js::js_string;
use serde_json::{Value, json};

use crate::args::Args;
use crate::ops::Failure;

thread_local! {
    static HELD: RefCell<Option<Value>> = const { RefCell::new(None) };
}

fn list(p: &Value, k: &str) -> Vec<Value> {
    p.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
}

/// Runs session operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let not_open = || Some(Err(Failure::new("session_not_open", None)));
    match op {
        // Hold a project: { tracks, switches, … } as the record has it.
        "session.open" => {
            HELD.with(|h| *h.borrow_mut() = Some(a.v(0).clone()));
            Some(Ok(json!(true)))
        }
        "session.close" => {
            HELD.with(|h| *h.borrow_mut() = None);
            Some(Ok(json!(true)))
        }
        // Tracks written: replaced by id, added where new; ids removed.
        "session.write" => HELD.with(|h| {
            let mut held = h.borrow_mut();
            let Some(p) = held.as_mut() else { return not_open() };
            let mut tracks = list(p, "tracks");
            for t in a.list(0) {
                match tracks.iter_mut().find(|x| x.get("id") == t.get("id")) {
                    Some(slot) => *slot = t,
                    None => tracks.push(t),
                }
            }
            let removed: Vec<String> = a.list(1).iter().map(|v| js_string(Some(v))).collect();
            tracks.retain(|t| !removed.contains(&js_string(t.get("id"))));
            p["tracks"] = Value::Array(tracks);
            if let Some(sw) = a.opt(2) {
                p["switches"] = sw.clone();
            }
            Some(Ok(json!(true)))
        }),
        // planElementChange on the held project: only the tracks it touched come back.
        "session.planElementChange" => HELD.with(|h| {
            let held = h.borrow();
            let Some(p) = held.as_ref() else { return not_open() };
            let empty = Vec::new();
            let tracks = p.get("tracks").and_then(Value::as_array).unwrap_or(&empty);
            let switches = p.get("switches").and_then(Value::as_array).unwrap_or(&empty);
            Some(Ok(edit::plan_element_change_touched(tracks, switches, a.v(0), a.u(1), a.v(2))))
        }),
        _ => None,
    }
}
