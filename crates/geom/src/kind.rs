// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An element's kind as the record states it since it follows the document (decision 0006): a word,
//! `straight`, `arc` or `transition`, as core 5.0 has it. An element without one is a straight.

use serde_json::Value;

pub const STRAIGHT: &str = "straight";
pub const ARC: &str = "arc";
pub const TRANSITION: &str = "transition";

/// The kind an element states, where it states one of the three.
pub fn kind_of(el: &Value) -> Option<&'static str> {
    match el.get("elementType").and_then(Value::as_str)? {
        "straight" => Some(STRAIGHT),
        "arc" => Some(ARC),
        "transition" => Some(TRANSITION),
        _ => None,
    }
}

pub fn is_straight(el: &Value) -> bool {
    kind_of(el) == Some(STRAIGHT)
}

pub fn is_arc(el: &Value) -> bool {
    kind_of(el) == Some(ARC)
}

pub fn is_transition(el: &Value) -> bool {
    kind_of(el) == Some(TRANSITION)
}
