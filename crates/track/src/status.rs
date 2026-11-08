// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! What a plan says about each object: existing, new, removal (`planStatus.js`),
//! and finding tracks and switches by what is typed (`search.js`).

use std::collections::HashMap;

use rail_across_js::{js_string, truthy};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::switch_model::{DEFAULT_SWITCH_KIND, port_track_id, ports_of};
use crate::util::text;

pub const STATUSES: [&str; 3] = ["existing", "new", "removal"];

fn valid(s: Option<&str>) -> Option<&str> {
    s.filter(|s| STATUSES.contains(s))
}

/// A track's status (`trackStatus`).
pub fn track_status(track: Option<&Value>) -> &str {
    track.and_then(|t| valid(text(t, "status"))).unwrap_or("existing")
}

/// A switch's own status, or the one its tracks give it (`switchStatus`).
pub fn switch_status<'a>(sw: &'a Value, track_by_id: &HashMap<String, &'a Value>) -> &'a str {
    if let Some(own) = valid(text(sw, "status")) {
        return own;
    }
    let turnout = text(sw, "kind").unwrap_or(DEFAULT_SWITCH_KIND) == "turnout";
    let found: Vec<&str> = ports_of(sw)
        .iter()
        .filter(|(p, _)| !turnout || *p == "B1")
        .map(|(p, _)| track_status(port_track_id(sw, p).and_then(|id| track_by_id.get(id).copied())))
        .collect();
    if found.contains(&"new") {
        "new"
    } else if found.contains(&"removal") {
        "removal"
    } else {
        "existing"
    }
}

fn norm(v: &str) -> String {
    v.to_lowercase().nfkd().filter(|c| !is_combining_mark(*c)).collect()
}

fn matches_all(haystack: &[Option<&Value>], query: &str) -> bool {
    let hay = norm(&haystack.iter().filter(|v| truthy(**v)).map(|v| js_string(*v)).collect::<Vec<_>>().join(" "));
    norm(query).split_whitespace().all(|w| hay.contains(w))
}

/// Whether `track` answers `query` (`trackMatches`).
pub fn track_matches(track: &Value, query: &str) -> bool {
    let f = |k: &str| track.get(k);
    matches_all(
        &[f("name"), f("lineNumber"), f("lineName"), f("stationName"), f("uicStation"), f("trackNumber")],
        query,
    )
}

/// Whether `sw` answers `query` (`switchMatches`).
pub fn switch_matches(sw: &Value, query: &str) -> bool {
    let id = sw.get("switchId").and_then(Value::as_str).map(|s| Value::String(s.chars().take(8).collect()));
    matches_all(&[sw.get("name"), sw.get("label"), sw.get("number"), id.as_ref()], query)
}
