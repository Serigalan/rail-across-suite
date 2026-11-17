// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Lifespans by day, and the states of a project they make (decision 0007):
//! an object states the days it is in service — `lifespan: { from, to }`, the
//! first and the last, the word "open" where the day is not known yet — and
//! nothing keeps a list of stages. A project passes through states: the
//! existing one, one for every day something is put into or taken out of
//! service, the final one. They are worked out here, as is the record as it
//! stands at one.
//!
//! | Lifespan | Stands |
//! |---|---|
//! | none | always |
//! | `from: d` | from day d on, and in the final state |
//! | `to: d` | in the existing state and until day d |
//! | `from: "open"` | in the final state only |
//! | `to: "open"` | in the existing state and on every day, not in the final state |

use std::collections::{BTreeSet, HashSet};

use rail_across_js::{Object, js_string};
use serde_json::{Value, json};

/// The word a lifespan states where the day is not known yet.
pub const OPEN: &str = "open";

/// Whether `s` is a day as ISO 8601 writes it, and one there is.
pub fn is_day(s: &str) -> bool {
    parse(s).is_some()
}

fn parse(s: &str) -> Option<(i64, i64, i64)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    (1..=12).contains(&m).then_some(())?;
    (1..=days_in(y, m)).contains(&d).then_some((y, m, d))
}

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in(y: i64, m: i64) -> i64 {
    match m {
        2 if leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn format((y, m, d): (i64, i64, i64)) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day after `s`.
pub fn day_after(s: &str) -> Option<String> {
    let (y, m, d) = parse(s)?;
    Some(format(if d < days_in(y, m) {
        (y, m, d + 1)
    } else if m < 12 {
        (y, m + 1, 1)
    } else {
        (y + 1, 1, 1)
    }))
}

/// The day before `s`.
pub fn day_before(s: &str) -> Option<String> {
    let (y, m, d) = parse(s)?;
    Some(format(if d > 1 {
        (y, m, d - 1)
    } else if m > 1 {
        (y, m - 1, days_in(y, m - 1))
    } else {
        (y - 1, 12, 31)
    }))
}

/// The collections of the record whose objects may carry a lifespan; the layers' collections do beside them.
pub const DATED: &[&str] = &["tracks", "switches", "platforms", "endMarks", "bridges", "retainingWalls", "noiseBarriers"];
/// The layers the record carries as the document has them, every collection's objects with their lifespans.
pub const LAYERS: &[&str] = &["signalling", "electrification"];

/// One end of a lifespan.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum End<'a> {
    None,
    Open,
    Day(&'a str),
}

fn end<'a>(lifespan: Option<&'a Value>, key: &str) -> End<'a> {
    match lifespan.and_then(|l| l.get(key)).and_then(Value::as_str) {
        None => End::None,
        Some(OPEN) => End::Open,
        Some(day) => End::Day(day),
    }
}

/// What a lifespan states that cannot be: a day there is none of, a first day after the last. Nothing where it is
/// sound — the door's rule beside the schema's shape. An open beginning may have an end, a day or an open one:
/// built for a while, the days not known yet.
pub fn lifespan_breach(lifespan: &Value) -> Option<String> {
    let (from, to) = (end(Some(lifespan), "from"), end(Some(lifespan), "to"));
    for (key, e) in [("from", from), ("to", to)] {
        if let End::Day(d) = e
            && !is_day(d)
        {
            return Some(format!("/{key}: {d} is no day"));
        }
    }
    match (from, to) {
        (End::Day(a), End::Day(b)) if a > b => Some(format!(": begins on {a}, after it ends on {b}")),
        _ => None,
    }
}

/// The planning status a lifespan states: "new" where it begins, "removal" where it only ends, "existing" else.
pub fn status_of(lifespan: Option<&Value>) -> &'static str {
    match (end(lifespan, "from"), end(lifespan, "to")) {
        (End::None, End::None) => "existing",
        (End::None, _) => "removal",
        _ => "new",
    }
}

/// A state of a project: the existing one, a day, the final one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State<'a> {
    Existing,
    Day(&'a str),
    Final,
}

impl<'a> State<'a> {
    /// The state `key` names: `existing`, a day (YYYY-MM-DD), `final`.
    pub fn of(key: &'a str) -> Option<State<'a>> {
        match key {
            "existing" => Some(State::Existing),
            "final" => Some(State::Final),
            day if is_day(day) => Some(State::Day(day)),
            _ => None,
        }
    }
}

/// Whether an object with that lifespan stands at a state. Days compare as text (YYYY-MM-DD).
pub fn stands(lifespan: Option<&Value>, at: State) -> bool {
    let (from, to) = (end(lifespan, "from"), end(lifespan, "to"));
    match at {
        State::Existing => from == End::None,
        State::Final => to == End::None,
        State::Day(day) => {
            let begun = match from {
                End::None => true,
                End::Open => false,
                End::Day(d) => d <= day,
            };
            let ended = match to {
                End::None | End::Open => false,
                End::Day(d) => d < day,
            };
            begun && !ended
        }
    }
}

/// Every lifespan a record holds: its own collections' and the layers'.
fn lifespans(record: &Value) -> impl Iterator<Item = &Value> {
    let own = DATED.iter().filter_map(|name| record.get(*name)?.as_array()).flatten();
    let layers = LAYERS
        .iter()
        .filter_map(|layer| record.get(*layer)?.get("collections")?.as_object())
        .flat_map(|cols| cols.values().filter_map(Value::as_array).flatten());
    own.chain(layers).filter_map(|o| o.get("lifespan"))
}

/// The states a record passes through, as a tool offers them: `[{ key, date? }]` — `existing`, every day
/// something is put into service or taken out of it (the day after the last it is in service), `final`.
pub fn stages_of(record: &Value) -> Value {
    let mut days = BTreeSet::new();
    for l in lifespans(record) {
        if let End::Day(d) = end(Some(l), "from")
            && is_day(d)
        {
            days.insert(d.to_string());
        }
        if let End::Day(d) = end(Some(l), "to")
            && let Some(after) = day_after(d)
        {
            days.insert(after);
        }
    }
    let mut out = vec![json!({ "key": "existing" })];
    out.extend(days.into_iter().map(|day| json!({ "key": day, "date": day })));
    out.push(json!({ "key": "final" }));
    Value::Array(out)
}

/// The record as it stands at the state `key` (`existing`, a day, `final`): the tracks, switches, platforms,
/// end marks and civil structures in service then, a port of a switch whose track is not empty; the layers'
/// objects by their lifespans. The record as it is where `key` is none of these.
pub fn record_at_stage(record: &Value, key: &str) -> Value {
    let Some(at) = State::of(key) else { return record.clone() };
    let standing = |items: &[Value]| -> Vec<Value> { items.iter().filter(|o| stands(o.get("lifespan"), at)).cloned().collect() };
    let mut out = record.as_object().cloned().unwrap_or_default();
    let mut kept_tracks = HashSet::new();
    for name in DATED {
        let Some(Value::Array(list)) = record.get(*name) else { continue };
        let kept = standing(list);
        if *name == "tracks" {
            kept_tracks.extend(kept.iter().map(|t| js_string(t.get("id"))));
        }
        out.insert((*name).into(), Value::Array(kept));
    }
    if let Some(Value::Array(switches)) = out.get("switches").cloned() {
        let switches: Vec<Value> = switches
            .into_iter()
            .map(|mut sw| {
                if let Some(Value::Object(ports)) = sw.get_mut("ports") {
                    for v in ports.values_mut() {
                        if v.get("trackId").is_some_and(|t| !kept_tracks.contains(&js_string(Some(t)))) {
                            *v = Value::Null;
                        }
                    }
                }
                sw
            })
            .collect();
        out.insert("switches".into(), Value::Array(switches));
    }
    for layer in LAYERS {
        let Some(Value::Object(d)) = record.get(*layer) else { continue };
        let mut d = d.clone();
        if let Some(Value::Object(cols)) = d.get("collections").cloned() {
            let cols: Object = cols
                .into_iter()
                .map(|(name, v)| match v {
                    Value::Array(items) => (name, Value::Array(standing(&items))),
                    other => (name, other),
                })
                .collect();
            d.insert("collections".into(), Value::Object(cols));
        }
        out.insert((*layer).into(), Value::Object(d));
    }
    Value::Object(out)
}
