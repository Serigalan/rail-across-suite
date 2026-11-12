// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The construction stages of a project by their days (decision 0027 on
//! 0003): the existing state first, the final state last, and between them a
//! stage for every day something is put into or taken out of service. The
//! record states a track's, switch's, platform's or end mark's days —
//! `inServiceFrom` the first day it is in service, `inServiceTo` the last —
//! beside its planning status; the document a lifespan over the stages, each
//! stage between the first and the last beginning on its `date`.
//!
//! | Record | Lifespan |
//! |---|---|
//! | `inServiceFrom: d` | `from`: the stage beginning on d |
//! | `inServiceTo: d` | `to`: the stage beginning the day after d |
//! | `status: "new"` without `inServiceFrom` | `from`: the final stage |
//! | `status: "removal"` without `inServiceTo` | `to`: the final stage |
//!
//! Back, the status is "new" where the lifespan has a `from`, the final stage
//! or a dated one, unless it ends at the final stage ("removal"); "removal"
//! where it has a `to` alone.

use std::collections::BTreeMap;

use rail_across_js::{Object, js_string};
use serde_json::{Value, json};

use crate::canonical::derived_id;
use crate::error::{PayloadError, Result};

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

/// The collections of the record whose objects carry days.
pub const DATED: &[&str] = &["tracks", "switches", "platforms", "endMarks"];

/// The stages of a project: the existing and the final state, and a stage for each day between them.
pub struct Stages {
    pub existing: String,
    pub last: String,
    /// day → (id, name)
    days: BTreeMap<String, (String, Option<Value>)>,
}

fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

impl Stages {
    /// The stages a record makes: those it keeps (`stages`, [{ id?, date, name? }]) and one for every day its
    /// objects are put into service or taken out of it — the day after the last a thing is in service.
    pub fn of_record(record: &Value) -> Result<Stages> {
        let pid = js_string(record.get("id"));
        let defaults = crate::core_format::default_stages(&pid);
        let mut days = BTreeMap::new();
        let mut bad = Vec::new();
        for (i, s) in record.get("stages").and_then(Value::as_array).into_iter().flatten().enumerate() {
            match text(s.get("date")).filter(|d| is_day(d)) {
                Some(day) => {
                    let id = text(s.get("id")).unwrap_or_else(|| derived_id(&format!("{pid}\u{0}stage\u{0}{day}")));
                    days.insert(day, (id, s.get("name").cloned()));
                }
                None => bad.push(format!("/stages/{i}/date: is no day")),
            }
        }
        let mut add = |day: String| {
            days.entry(day.clone())
                .or_insert_with(|| (derived_id(&format!("{pid}\u{0}stage\u{0}{day}")), None));
        };
        for name in DATED {
            for (i, o) in record.get(*name).and_then(Value::as_array).into_iter().flatten().enumerate() {
                for (key, shift) in [("inServiceFrom", false), ("inServiceTo", true)] {
                    let Some(v) = o.get(key).filter(|v| !v.is_null()) else { continue };
                    let day = v.as_str().filter(|d| is_day(d)).map(|d| if shift { day_after(d) } else { Some(d.to_string()) });
                    match day.flatten() {
                        Some(day) => add(day),
                        None => bad.push(format!("/{name}/{i}/{key}: is no day")),
                    }
                }
            }
        }
        if !bad.is_empty() {
            return Err(PayloadError::with("invalid_payload", bad));
        }
        Ok(Stages {
            existing: js_string(defaults[0].get("id")),
            last: js_string(defaults[1].get("id")),
            days,
        })
    }

    /// The stages a document holds; `stages_unsupported` where one between the first and the last has no day.
    pub fn of_document(stages: &[Value]) -> Result<Stages> {
        if stages.len() < 2 {
            return Err(PayloadError::new("stages_unsupported"));
        }
        let mut days = BTreeMap::new();
        for s in &stages[1..stages.len() - 1] {
            let Some(day) = text(s.get("date")).filter(|d| is_day(d)) else {
                return Err(PayloadError::new("stages_unsupported"));
            };
            days.insert(day, (js_string(s.get("id")), s.get("name").cloned()));
        }
        Ok(Stages {
            existing: js_string(stages[0].get("id")),
            last: js_string(stages[stages.len() - 1].get("id")),
            days,
        })
    }

    /// The stage list of the document: the existing state, the days in their order, the final state.
    pub fn to_document(&self) -> Value {
        let mut out = vec![json!({ "id": self.existing })];
        for (day, (id, name)) in &self.days {
            let mut o = Object::new();
            o.insert("id".into(), json!(id));
            if let Some(n) = name.as_ref().filter(|n| !n.is_null()) {
                o.insert("name".into(), n.clone());
            }
            o.insert("date".into(), json!(day));
            out.push(Value::Object(o));
        }
        out.push(json!({ "id": self.last }));
        Value::Array(out)
    }

    /// The stages the record keeps: those between the first and the last, [{ id, date, name? }]; none where there are none.
    pub fn to_record(&self) -> Option<Value> {
        if self.days.is_empty() {
            return None;
        }
        Some(Value::Array(
            self.days
                .iter()
                .map(|(day, (id, name))| {
                    let mut o = Object::new();
                    o.insert("id".into(), json!(id));
                    o.insert("date".into(), json!(day));
                    if let Some(n) = name.as_ref().filter(|n| !n.is_null()) {
                        o.insert("name".into(), n.clone());
                    }
                    Value::Object(o)
                })
                .collect(),
        ))
    }

    fn of_day(&self, day: &str) -> Option<&String> {
        self.days.get(day).map(|(id, _)| id)
    }

    fn day_of(&self, id: &str) -> Option<&String> {
        self.days.iter().find(|(_, (i, _))| i == id).map(|(d, _)| d)
    }

    /// The lifespan of a record's object, from its status and days; null where it is existing and stays.
    pub fn lifespan(&self, o: &Value) -> Value {
        let status = o.get("status").and_then(Value::as_str);
        let from = text(o.get("inServiceFrom"))
            .and_then(|d| self.of_day(&d).cloned())
            .or_else(|| (status == Some("new")).then(|| self.last.clone()));
        let to = text(o.get("inServiceTo"))
            .and_then(|d| day_after(&d))
            .and_then(|d| self.of_day(&d).cloned())
            .or_else(|| (status == Some("removal")).then(|| self.last.clone()));
        let mut l = Object::new();
        if let Some(f) = from {
            l.insert("from".into(), json!(f));
        }
        if let Some(t) = to {
            l.insert("to".into(), json!(t));
        }
        if l.is_empty() { Value::Null } else { Value::Object(l) }
    }

    /// What the record states of a lifespan: (status, inServiceFrom, inServiceTo); `stages_unsupported` where it
    /// names a stage the list lacks, or begins at the existing state.
    pub fn record_fields(&self, lifespan: Option<&Value>) -> Result<(Value, Value, Value)> {
        let Some(l) = lifespan.filter(|l| rail_across_js::truthy(Some(l))) else {
            return Ok((Value::Null, Value::Null, Value::Null));
        };
        let day = |v: Option<&Value>| -> Result<Option<Option<String>>> {
            match v.and_then(Value::as_str) {
                None => Ok(None),
                Some(id) if id == self.last => Ok(Some(None)),
                Some(id) => self
                    .day_of(id)
                    .cloned()
                    .map(|d| Some(Some(d)))
                    .ok_or_else(|| PayloadError::new("stages_unsupported")),
            }
        };
        let (from, to) = (day(l.get("from"))?, day(l.get("to"))?);
        let status = match (&from, &to) {
            (_, Some(None)) => "removal",
            (Some(_), _) => "new",
            (None, Some(_)) => "removal",
            (None, None) => return Ok((Value::Null, Value::Null, Value::Null)),
        };
        let from_day = from.flatten().map(Value::String).unwrap_or(Value::Null);
        let to_day = to.flatten().and_then(|d| day_before(&d)).map(Value::String).unwrap_or(Value::Null);
        Ok((json!(status), from_day, to_day))
    }
}

// ── A record at a stage ─────────────────────────────────────────────────────

/// The stages of a record as a tool offers them: `[{ key, id, date? }]` — `existing`, each day, `final` — in their order.
pub fn stages_of(record: &Value) -> Value {
    let Ok(s) = Stages::of_record(record) else { return json!([]) };
    let mut out = vec![json!({ "key": "existing", "id": s.existing })];
    for (day, (id, name)) in &s.days {
        let mut o = Object::new();
        o.insert("key".into(), json!(day));
        o.insert("id".into(), json!(id));
        o.insert("date".into(), json!(day));
        if let Some(n) = name.as_ref().filter(|n| !n.is_null()) {
            o.insert("name".into(), n.clone());
        }
        out.push(Value::Object(o));
    }
    out.push(json!({ "key": "final", "id": s.last }));
    Value::Array(out)
}

impl Stages {
    /// The place of a stage in the list: 0 the existing state, the last the final.
    fn index_of_id(&self, id: &str) -> Option<usize> {
        if id == self.existing {
            return Some(0);
        }
        if id == self.last {
            return Some(self.days.len() + 1);
        }
        self.days.values().position(|(i, _)| i == id).map(|i| i + 1)
    }

    fn index_of_key(&self, key: &str) -> Option<usize> {
        match key {
            "existing" => Some(0),
            "final" => Some(self.days.len() + 1),
            day => self.days.keys().position(|d| d == day).map(|i| i + 1),
        }
    }

    /// Whether an object with that lifespan exists at stage `k`.
    fn lives_at(&self, lifespan: &Value, k: usize) -> bool {
        let from = lifespan.get("from").and_then(Value::as_str).and_then(|id| self.index_of_id(id)).unwrap_or(0);
        let to = lifespan
            .get("to")
            .and_then(Value::as_str)
            .and_then(|id| self.index_of_id(id))
            .unwrap_or(usize::MAX);
        from <= k && k < to
    }
}

/// The record as it stands at the stage `key` (`existing`, a day, `final`): the tracks, switches, platforms and
/// end marks in service then, a port of a switch whose track is not empty; the layers' objects by their lifespans.
/// The record as it is where it has no such stage.
pub fn record_at_stage(record: &Value, key: &str) -> Value {
    let Ok(s) = Stages::of_record(record) else { return record.clone() };
    let Some(k) = s.index_of_key(key) else { return record.clone() };
    let mut out = record.as_object().cloned().unwrap_or_default();
    let mut kept_tracks = std::collections::HashSet::new();
    for name in DATED {
        let Some(Value::Array(list)) = record.get(*name) else { continue };
        let kept: Vec<Value> = list.iter().filter(|o| s.lives_at(&s.lifespan(o), k)).cloned().collect();
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
    for layer in ["signalling", "electrification"] {
        let Some(Value::Object(d)) = record.get(layer) else { continue };
        let mut d = d.clone();
        if let Some(Value::Object(cols)) = d.get("collections").cloned() {
            let cols: Object = cols
                .into_iter()
                .map(|(name, v)| match v {
                    Value::Array(items) => {
                        let kept = items.into_iter().filter(|o| o.get("lifespan").is_none_or(|l| s.lives_at(l, k))).collect();
                        (name, Value::Array(kept))
                    }
                    other => (name, other),
                })
                .collect();
            d.insert("collections".into(), Value::Object(cols));
        }
        out.insert(layer.into(), Value::Object(d));
    }
    Value::Object(out)
}
