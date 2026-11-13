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

/// What an object's lifespan says of it (decision 0028): new where it states a first day, removal where it
/// states only a last one — nothing where it states neither, the object existing and staying.
fn stated(o: Option<&Value>) -> Option<&'static str> {
    let lifespan = o?.get("lifespan")?;
    let has = |k: &str| lifespan.get(k).is_some_and(|v| !v.is_null());
    if has("from") {
        Some("new")
    } else if has("to") {
        Some("removal")
    } else {
        None
    }
}

/// A track's status (`trackStatus`).
pub fn track_status(track: Option<&Value>) -> &'static str {
    stated(track).unwrap_or("existing")
}

/// A switch's own status, or the one its tracks give it (`switchStatus`).
pub fn switch_status(sw: &Value, track_by_id: &HashMap<String, &Value>) -> &'static str {
    if let Some(own) = stated(Some(sw)) {
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

/// Whether `track` answers `query` (`trackMatches`), its station's name and
/// number those of `point`, the operational point it names.
pub fn track_matches(track: &Value, query: &str, point: Option<&Value>) -> bool {
    let f = |k: &str| track.get(k);
    let p = |k: &str| point.and_then(|o| o.get(k));
    matches_all(&[f("name"), f("lineNumber"), f("lineName"), p("name"), p("uic"), f("trackNumber")], query)
}

/// Whether `sw` answers `query` (`switchMatches`).
pub fn switch_matches(sw: &Value, query: &str) -> bool {
    let id = sw.get("switchId").and_then(Value::as_str).map(|s| Value::String(s.chars().take(8).collect()));
    matches_all(&[sw.get("name"), sw.get("label"), sw.get("number"), id.as_ref()], query)
}
