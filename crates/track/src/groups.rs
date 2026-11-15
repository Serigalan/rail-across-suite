// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Tracks grouped by line and station, and their names (`trackGroups.js`),
//! and the platform form (`platformForm.js`).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use rail_across_js::{get, js_number, js_round, js_string, js_trim, number, strict_eq, truthy};
use regex::Regex;
use serde_json::{Value, json};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::heights::track_length;
use crate::model::next_track_name;
use crate::platforms::{DEFAULT_PLATFORM_HEIGHT, PLATFORM_FRONT_OFFSET, PLATFORM_HEIGHTS, PLATFORM_WIDTH};
use crate::util::{obj, text};

const KM_NAME_DIGITS: usize = 5;

/// A line track's name from its kilometre (`kmTrackName`).
pub fn km_track_name(line: &str, km: f64, taken: &HashSet<String>) -> String {
    let name = |step: i64| format!("{line}.{}{:0>width$}", if step < 0 { "-" } else { "" }, step.abs(), width = KM_NAME_DIGITS);
    let mut step = js_round(km / 10.0) as i64;
    while taken.contains(&name(step)) {
        step += 1;
    }
    name(step)
}

fn tx(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(v) => js_trim(&js_string(Some(v))).to_string(),
    }
}

/// A track's kind: line or station (`trackKind`) — as it states it, else a
/// track naming an operational point is a station track (decision 0028).
pub fn track_kind(track: &Value) -> Option<&'static str> {
    match track.get("trackType").and_then(Value::as_str) {
        Some("line") => return Some("line"),
        Some("station") => return Some("station"),
        _ => {}
    }
    if !tx(track.get("operationalPointId")).is_empty() {
        return Some("station");
    }
    if !tx(track.get("lineNumber")).is_empty() {
        return Some("line");
    }
    None
}

/// `line_track` / `station_track`, else the fallback (`trackTypeName`).
pub fn track_type_name(track: &Value, fallback: &Value) -> Value {
    match track_kind(track) {
        Some(k) => json!(format!("{k}_track")),
        None => fallback.clone(),
    }
}

/// The key a track is grouped by (`trackGroupKey`): its line, or its operational point by id.
pub fn track_group_key(track: &Value) -> Option<String> {
    match track_kind(track) {
        Some("line") => {
            let n = tx(track.get("lineNumber"));
            (!n.is_empty()).then(|| format!("line:{n}"))
        }
        Some("station") => {
            let point = tx(track.get("operationalPointId"));
            (!point.is_empty()).then(|| format!("station:{point}"))
        }
        _ => None,
    }
}

/// `Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })`: case and
/// accents left out, runs of digits by their value.
pub fn collate(a: &str, b: &str) -> Ordering {
    let fold = |s: &str| -> String { s.nfkd().filter(|c| !is_combining_mark(*c)).collect::<String>().to_lowercase() };
    let (a, b) = (fold(a), fold(b));
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut m = String::new();
                while let Some(c) = x.peek().copied().filter(char::is_ascii_digit) {
                    m.push(c);
                    x.next();
                }
                let mut n = String::new();
                while let Some(d) = y.peek().copied().filter(char::is_ascii_digit) {
                    n.push(d);
                    y.next();
                }
                let (m, n) = (m.trim_start_matches('0').to_string(), n.trim_start_matches('0').to_string());
                let o = m.len().cmp(&n.len()).then(m.cmp(&n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.cmp(&d);
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

fn name_or_id(t: &Value) -> String {
    match t.get("name").filter(|v| truthy(Some(v))) {
        Some(n) => js_string(Some(n)),
        None => js_string(t.get("id")),
    }
}

/// The tracks in groups by line and station, the rest last (`groupTracks`): a
/// station's group names its point (`operationalPointId`) and shows the point's
/// name and number (`stationName`, `uicStation`) from `points`, the register.
pub fn group_tracks(tracks: &[Value], points: &[Value]) -> Value {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (String, Value, Vec<Value>)> = HashMap::new();
    let mut none = Vec::new();
    for t in tracks {
        let Some(key) = track_group_key(t) else {
            none.push(t.clone());
            continue;
        };
        let kind = track_kind(t).unwrap_or("");
        let g = groups.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            (
                kind.to_string(),
                json!({ "key": key, "kind": kind, "lineNumber": null, "lineName": null, "stationName": null, "uicStation": null }),
                Vec::new(),
            )
        });
        if g.0 == "station" && g.1.get("operationalPointId").is_none() {
            let id = tx(t.get("operationalPointId"));
            let point = points.iter().find(|p| tx(p.get("id")) == id);
            let field = |k: &str| {
                let v = tx(point.and_then(|p| p.get(k)));
                if v.is_empty() { Value::Null } else { json!(v) }
            };
            g.1["operationalPointId"] = json!(id);
            g.1["stationName"] = field("name");
            g.1["uicStation"] = field("uic");
        }
        let fill = |g: &mut Value, k: &str| {
            if g[k].is_null() {
                let v = tx(t.get(k));
                g[k] = if v.is_empty() { Value::Null } else { json!(v) };
            }
        };
        if g.0 == "line" {
            fill(&mut g.1, "lineNumber");
            fill(&mut g.1, "lineName");
        }
        g.2.push(t.clone());
    }
    let mut sorted: Vec<(String, Value, Vec<Value>)> = order.into_iter().map(|k| groups.remove(&k).expect("group")).collect();
    sorted.sort_by(|a, b| {
        if a.0 != b.0 {
            return if a.0 == "line" { Ordering::Less } else { Ordering::Greater };
        }
        let s = |g: &Value, k: &str| g[k].as_str().unwrap_or("").to_string();
        if a.0 == "line" {
            collate(&s(&a.1, "lineNumber"), &s(&b.1, "lineNumber"))
        } else {
            let pick = |g: &Value| {
                if g["stationName"].is_null() {
                    s(g, "uicStation")
                } else {
                    s(g, "stationName")
                }
            };
            collate(&pick(&a.1), &pick(&b.1))
        }
    });
    if !none.is_empty() {
        sorted.push((
            "".into(),
            json!({ "key": "none", "kind": null, "lineNumber": null, "lineName": null, "stationName": null, "uicStation": null }),
            none,
        ));
    }
    Value::Array(
        sorted
            .into_iter()
            .map(|(_, mut g, mut ts)| {
                ts.sort_by(|a, b| collate(&name_or_id(a), &name_or_id(b)));
                g["tracks"] = Value::Array(ts);
                g
            })
            .collect(),
    )
}

/// A group's title (`groupTitle`).
pub fn group_title(group: &Value) -> String {
    match text(group, "kind") {
        Some("line") => ["lineNumber", "lineName"]
            .iter()
            .map(|k| tx(group.get(*k)))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        Some("station") => {
            let s = tx(group.get("stationName"));
            if s.is_empty() { tx(group.get("uicStation")) } else { s }
        }
        _ => String::new(),
    }
}

/// How a list names a track (`trackListLabel`).
pub fn track_list_label(track: &Value) -> String {
    match track.get("name").filter(|v| truthy(Some(v))) {
        Some(n) => js_string(Some(n)),
        None => js_string(track.get("id")).chars().take(8).collect(),
    }
}

/// The fields a track takes when assigned to a group (`assignmentFields`): a
/// station's its point, or the station as typed (`stationName`, `uicStation`)
/// where it is a new one, which the record makes a point of (decision 0028).
pub fn assignment_fields(target: &Value) -> Value {
    let s = |k: &str| {
        let v = tx(target.get(k));
        if v.is_empty() { Value::Null } else { json!(v) }
    };
    match text(target, "kind") {
        Some("line") => json!({ "trackType": "line", "lineNumber": number(js_number(Some(&json!(tx(target.get("lineNumber")))))), "lineName": s("lineName") }),
        Some("station") if !tx(target.get("operationalPointId")).is_empty() => {
            json!({ "trackType": "station", "operationalPointId": s("operationalPointId") })
        }
        Some("station") => json!({ "trackType": "station", "stationName": s("stationName"), "uicStation": s("uicStation") }),
        _ => json!({ "lineNumber": null, "lineName": null, "operationalPointId": null }),
    }
}

/// Tracks assigned to a group, renamed and renumbered where asked (`assignTracks`).
pub fn assign_tracks(tracks: &[Value], ids: &[Value], target: &Value, names: &Value, track_numbers: &Value) -> Vec<Value> {
    let fields = assignment_fields(target);
    let none = text(target, "kind").is_none() || !matches!(text(target, "kind"), Some("line" | "station"));
    let digits = Regex::new(r"^\d+$").expect("pattern");
    tracks
        .iter()
        .map(|t| {
            if !ids.iter().any(|id| strict_eq(Some(id), t.get("id"))) {
                return t.clone();
            }
            let mut o = obj(t);
            if none {
                o.shift_remove("trackType");
            }
            for (k, v) in obj(&fields) {
                o.insert(k, v);
            }
            let id = js_string(t.get("id"));
            if let Some(n) = names.get(&id) {
                o.insert("name".into(), n.clone());
            }
            if let Some(n) = track_numbers.as_object().and_then(|m| m.get(&id)) {
                let number_ = tx(Some(n));
                o.insert(
                    "trackNumber".into(),
                    if digits.is_match(&number_) {
                        number(js_number(Some(&json!(number_))))
                    } else if number_.is_empty() {
                        Value::Null
                    } else {
                        json!(number_)
                    },
                );
            }
            Value::Object(o)
        })
        .collect()
}

/// A station's name as the stem of its track names (`stationTrackStem`).
pub fn station_track_stem(name: &str) -> String {
    let brackets = Regex::new(r"\([^)]*\)|\[[^\]]*\]").expect("pattern");
    let s = brackets
        .replace_all(name, " ")
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss");
    let s: String = s.nfd().filter(|c| !('\u{300}'..='\u{36f}').contains(c)).collect();
    let dashes = Regex::new(r"[^a-z0-9]+").expect("pattern");
    let s = dashes.replace_all(&s, "-");
    let s = s.trim_matches('-');
    if s.is_empty() { "track".into() } else { s.to_string() }
}

/// The names tracks assigned to a group get (`plannedNames`): { names, fallback, clashes }.
pub fn planned_names(tracks: &[Value], ids: &[Value], target: &Value, km_by_id: &Value, number_by_id: &Value) -> Value {
    let wanted: HashSet<String> = ids.iter().map(|i| js_string(Some(i))).collect();
    let mut taken: HashSet<String> = tracks
        .iter()
        .filter(|t| !wanted.contains(&js_string(t.get("id"))))
        .filter_map(|t| t.get("name").filter(|v| truthy(Some(v))).map(|n| js_string(Some(n))))
        .collect();
    let mut names: Vec<(String, String)> = Vec::new();
    let mut fallback: Vec<String> = Vec::new();
    let mut clashes: Vec<String> = Vec::new();
    let add_unique = |v: &mut Vec<String>, s: String| {
        if !v.contains(&s) {
            v.push(s);
        }
    };
    let set = |names: &mut Vec<(String, String)>, id: String, name: String| match names.iter_mut().find(|(i, _)| *i == id) {
        Some(slot) => slot.1 = name,
        None => names.push((id, name)),
    };
    match text(target, "kind") {
        Some("line") => {
            let line = tx(target.get("lineNumber"));
            let km = |id: &str| km_by_id.get(id).and_then(Value::as_f64);
            let mut order: Vec<String> = ids.iter().map(|i| js_string(Some(i))).collect();
            order.sort_by(|a, b| {
                km(a)
                    .unwrap_or(f64::INFINITY)
                    .partial_cmp(&km(b).unwrap_or(f64::INFINITY))
                    .unwrap_or(Ordering::Equal)
            });
            for id in order {
                let name = match km(&id) {
                    Some(k) => km_track_name(&line, k, &taken),
                    None => {
                        add_unique(&mut fallback, id.clone());
                        next_track_name(&line, &taken.iter().cloned().collect::<Vec<_>>())
                    }
                };
                set(&mut names, id, name.clone());
                taken.insert(name);
            }
        }
        Some("station") => {
            let st = tx(target.get("stationName"));
            let source = if st.is_empty() { tx(target.get("uicStation")) } else { st };
            let prefix = station_track_stem(&source);
            let num_of = |id: &str| tx(number_by_id.get(id));
            let mut seen: Vec<(String, String)> = Vec::new();
            for id in ids.iter().map(|i| js_string(Some(i))).filter(|id| !num_of(id).is_empty()) {
                let name = format!("{prefix}.{}", num_of(&id));
                if taken.contains(&name) {
                    add_unique(&mut clashes, id.clone());
                }
                if let Some((_, other)) = seen.iter().find(|(n, _)| *n == name) {
                    let other = other.clone();
                    add_unique(&mut clashes, id.clone());
                    add_unique(&mut clashes, other);
                }
                match seen.iter_mut().find(|(n, _)| *n == name) {
                    Some(slot) => slot.1 = id.clone(),
                    None => seen.push((name.clone(), id.clone())),
                }
                set(&mut names, id, name);
            }
            for (n, _) in &seen {
                taken.insert(n.clone());
            }
            for id in ids.iter().map(|i| js_string(Some(i))).filter(|id| num_of(id).is_empty()) {
                let name = next_track_name(&prefix, &taken.iter().cloned().collect::<Vec<_>>());
                add_unique(&mut fallback, id.clone());
                set(&mut names, id, name.clone());
                taken.insert(name);
            }
        }
        _ => {}
    }
    let mut m = rail_across_js::Object::new();
    for (id, n) in names {
        m.insert(id, json!(n));
    }
    json!({ "names": m, "fallback": fallback, "clashes": clashes })
}

// ── The platform form (platformForm.js) ─────────────────────────────────────

fn station_text(v: f64) -> String {
    rail_across_js::es_number(js_round(v * 1000.0) / 1000.0)
}

/// The form of a platform (`platformForm`), its station the name and code of
/// `point`, the operational point it names.
pub fn platform_form(p: &Value, point: Option<&Value>) -> Value {
    let h = js_number(p.get("height"));
    let height = if h.is_finite() { h } else { DEFAULT_PLATFORM_HEIGHT };
    let off = js_number(p.get("frontOffset"));
    json!({
        "trackId": p.get("trackId"),
        "start": station_text(js_number(p.get("startStation"))), "end": station_text(js_number(p.get("endStation"))),
        "side": get(p, "side").cloned().unwrap_or(json!("right")),
        "height": number(height), "freeHeight": !PLATFORM_HEIGHTS.contains(&height),
        "stationName": point.and_then(|o| get(o, "name")).cloned().unwrap_or(json!("")),
        "code": point.and_then(|o| get(o, "rl100")).cloned().unwrap_or(json!("")),
        "frontOffset": number(if off.is_finite() { off } else { PLATFORM_FRONT_OFFSET }),
    })
}

/// A station as the form shows it (`pickedStation`).
pub fn picked_station(v: f64) -> String {
    station_text(v)
}

/// The platform a form describes (`platformDraft`), its station as typed — the
/// record makes it the point of that name and code (decision 0028).
pub fn platform_draft(f: &Value) -> Value {
    let (s1, s2, front) = (js_number(f.get("start")), js_number(f.get("end")), js_number(f.get("frontOffset")));
    json!({
        "trackId": f.get("trackId"), "startStation": number(s1.min(s2)), "endStation": number(s1.max(s2)),
        "side": f.get("side"), "frontOffset": number(front), "backOffset": number(front + PLATFORM_WIDTH),
        "height": f.get("height"), "stationName": f.get("stationName"), "code": f.get("code"),
    })
}

/// Can the form be taken (`platformFormValid`)?
pub fn platform_form_valid(f: &Value, track: Option<&Value>) -> bool {
    let Some(track) = track else { return false };
    let total = track_length(track);
    let s = [js_number(f.get("start")), js_number(f.get("end"))];
    s.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= total + 1e-6)
        && (s[1] - s[0]).abs() >= 1.0
        && js_number(f.get("frontOffset")) > 0.0
        && f.get("height") != Some(&json!(""))
        && js_number(f.get("height")) >= 0.0
}
