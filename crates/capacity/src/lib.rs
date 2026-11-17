// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The capacity of a stretch from a timetable's blocking times (the roadmap,
//! phase 4, K1; decision 0013): the minimum headway of two trains that follow
//! each other, the timetable compressed after UIC Code 406 — each train as
//! early after the ones before as their blocking times let it, in the order
//! they run —, the occupancy time and rate it gives, the trains that set it,
//! the element that binds each of them to the one before, the areas two
//! trains both hold, and how long each resource of a station is held.
//!
//! A train comes as `{ id, kind?, at, blocks: [{ begin, end, resources }] }`:
//! `at` its time at the stretch's start [s], each block its blocking time and
//! what it holds — `{ section }` a track section, or `{ track, lo, hi }` a
//! stretch of a track no section covers, each with its own `end` where it is
//! released before the block (a section left behind). Two blocks of two trains
//! conflict where they hold the same section, or the same track's stretches
//! overlapping by more than half a metre.

use serde_json::{Value, json};

/// How far two stretches of a track overlap at least to be held both [m].
const SHARE: f64 = 0.5;

#[derive(Clone, Debug)]
enum Resource {
    Section(String),
    Track(String, f64, f64),
}

#[derive(Clone, Debug)]
struct Held {
    resource: Resource,
    begin: f64,
    end: f64,
    /// The index of the block it is held by, in the train's `blocks`.
    block: usize,
}

/// A resource as a list names it: `section:<id>`, or `track:<id>:<lo>-<hi>` to the metre.
fn key_of(resource: &Resource) -> String {
    match resource {
        Resource::Section(s) => format!("section:{s}"),
        Resource::Track(tr, a, b) => format!("track:{tr}:{}-{}", a.round(), b.round()),
    }
}

#[derive(Clone, Debug)]
pub struct Train {
    pub id: String,
    pub kind: String,
    pub at: f64,
    held: Vec<Held>,
    begin: f64,
    end: f64,
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or_default().to_string()
}

impl Train {
    /// A train as it comes (see the crate's text); None where it holds nothing.
    pub fn from_json(v: &Value) -> Option<Train> {
        let mut held = Vec::new();
        for (block, b) in v.get("blocks").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let (begin, end) = (num(&b["begin"]), num(&b["end"]));
            if !(begin.is_finite() && end.is_finite()) {
                continue;
            }
            for r in b.get("resources").and_then(Value::as_array).into_iter().flatten() {
                let until = r.get("end").map(num).filter(|e| e.is_finite()).unwrap_or(end);
                let resource = match r.get("section").and_then(Value::as_str) {
                    Some(s) => Resource::Section(s.to_string()),
                    None => {
                        let (a, b) = (num(&r["lo"]), num(&r["hi"]));
                        Resource::Track(text(r.get("track")), a.min(b), a.max(b))
                    }
                };
                held.push(Held {
                    resource,
                    begin,
                    end: until,
                    block,
                });
            }
        }
        if held.is_empty() {
            return None;
        }
        let begin = held.iter().map(|h| h.begin).fold(f64::INFINITY, f64::min);
        let end = held.iter().map(|h| h.end).fold(f64::NEG_INFINITY, f64::max);
        Some(Train {
            id: text(v.get("id")),
            kind: text(v.get("kind")),
            at: num(&v["at"]),
            held,
            begin,
            end,
        })
    }

    /// The train `by` seconds later.
    fn shifted(&self, by: f64) -> Train {
        let mut t = self.clone();
        t.at += by;
        t.begin += by;
        t.end += by;
        for h in &mut t.held {
            h.begin += by;
            h.end += by;
        }
        t
    }
}

fn conflict(a: &Resource, b: &Resource) -> bool {
    match (a, b) {
        (Resource::Section(x), Resource::Section(y)) => x == y,
        (Resource::Track(x, a0, a1), Resource::Track(y, b0, b1)) => x == y && a1.min(*b1) - a0.max(*b0) > SHARE,
        _ => false,
    }
}

/// How much earlier `after` could run behind `before` without holding what it
/// holds while `before` still holds it [s] — negative where it holds it too
/// soon already —; None where the two hold nothing the same.
fn slack(before: &Train, after: &Train) -> Option<f64> {
    let mut least: Option<f64> = None;
    for a in &before.held {
        for b in &after.held {
            if conflict(&a.resource, &b.resource) {
                let s = b.begin - a.end;
                least = Some(least.map_or(s, |l: f64| l.min(s)));
            }
        }
    }
    least
}

/// What binds `after` to `before` (Belegungselement): the resource both hold
/// where `after` could come least earlier — the one that sets the minimum
/// headway —, held by `before` until `end`, by `after` from `begin`, each by a
/// block of its own; None where the two hold nothing the same.
fn binding<'a>(before: &'a Train, after: &'a Train) -> Option<(&'a Held, &'a Held)> {
    let mut least: Option<(f64, &Held, &Held)> = None;
    for a in &before.held {
        for b in &after.held {
            if conflict(&a.resource, &b.resource) {
                let s = b.begin - a.end;
                if least.is_none_or(|(l, _, _)| s < l) {
                    least = Some((s, a, b));
                }
            }
        }
    }
    least.map(|(_, a, b)| (a, b))
}

/// The minimum headway of `after` behind `before` (Mindestzugfolgezeit) [s]:
/// how soon after `before` at the stretch's start it may be there, its
/// blocking times touching but not overlapping; None where they hold nothing the same.
pub fn least_headway(before: &Train, after: &Train) -> Option<f64> {
    slack(before, after).map(|s| after.at - before.at - s)
}

/// The recommended bounds of the occupancy rate after UIC Code 406 (2013), by
/// the kind of line and the period: the peak hour and the day.
pub fn bounds() -> Value {
    json!([
        { "line": "suburban", "peak": 0.85, "day": 0.70 },
        { "line": "highSpeed", "peak": 0.75, "day": 0.60 },
        { "line": "mixed", "peak": 0.75, "day": 0.60 },
    ])
}

/// A stretch compressed after UIC Code 406. `input`: `{ trains, from, to,
/// additional?: { perTrain, fixed } }` — the trains whose time at the stretch's
/// start lies in the period [from, to), compressed in the order they run there,
/// each as early behind those before as their blocking times let it, never
/// before the one before it. What comes back: the trains as compressed (`shift`
/// how much earlier [s]), the minimum headway of each behind the one before,
/// the occupancy time (from the first blocking time begun to the last ended),
/// the rate it gives of the period, with the additional times (`perTrain` for
/// each train, `fixed` once) and without, and the trains that set it — the
/// chain from the first to the one ending last, each bound to the one before it.
/// A train bound to one before says what binds it (`binding`): the resource,
/// the block of each of the two it is held by, the moment the one releases it
/// and the other takes it, as compressed.
pub fn compress(input: &Value) -> Value {
    let (from, to) = (num(&input["from"]), num(&input["to"]));
    let period = to - from;
    let mut trains: Vec<Train> = input
        .get("trains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Train::from_json)
        .filter(|t| t.at >= from && t.at < to)
        .collect();
    trains.sort_by(|a, b| a.at.total_cmp(&b.at).then_with(|| a.id.cmp(&b.id)));
    let mut placed: Vec<Train> = Vec::new();
    let mut bound_by: Vec<Option<usize>> = Vec::new();
    let mut out = Vec::new();
    for t in &trains {
        let mut by: Option<usize> = None;
        // At the earliest with the one before; later where one placed still holds what it holds.
        let mut at = placed.last().map_or(t.at, |p| p.at);
        for (k, p) in placed.iter().enumerate() {
            if let Some(h) = least_headway(p, t) {
                let earliest = p.at + h;
                if earliest > at + 1e-9 || (by.is_none() && (earliest - at).abs() <= 1e-9) {
                    at = earliest;
                    by = Some(k);
                }
            }
        }
        let c = t.shifted(at - t.at);
        let headway = placed.last().and_then(|p| least_headway(p, t).map(|h| json!(h)));
        let binds = by
            .and_then(|k| binding(&placed[k], &c))
            .map(|(a, b)| json!({ "resource": key_of(&a.resource), "beforeBlock": a.block, "block": b.block, "released": a.end, "taken": b.begin }));
        out.push(json!({
            "id": t.id, "kind": t.kind, "at": t.at, "compressedAt": at, "shift": t.at - at,
            "begin": c.begin, "end": c.end, "headway": headway, "boundBy": by.map(|k| json!(trains[k].id)), "binding": binds,
        }));
        placed.push(c);
        bound_by.push(by);
    }
    let begin = placed.iter().map(|t| t.begin).fold(f64::INFINITY, f64::min);
    let end = placed.iter().map(|t| t.end).fold(f64::NEG_INFINITY, f64::max);
    let occupancy = if placed.is_empty() { 0.0 } else { end - begin };
    let per_train = input["additional"].get("perTrain").map(num).filter(|x| x.is_finite()).unwrap_or(0.0);
    let fixed = input["additional"].get("fixed").map(num).filter(|x| x.is_finite()).unwrap_or(0.0);
    let additional = if placed.is_empty() { 0.0 } else { per_train * placed.len() as f64 + fixed };
    // The chain that sets it: back from the train that ends last, each to the one it is bound to.
    let mut critical = Vec::new();
    let mut at = placed.iter().enumerate().max_by(|a, b| a.1.end.total_cmp(&b.1.end)).map(|(i, _)| i);
    while let Some(i) = at {
        critical.push(json!(trains[i].id));
        at = bound_by[i];
    }
    critical.reverse();
    json!({
        "from": from, "to": to, "period": period, "trains": out, "occupancy": occupancy,
        "begin": if placed.is_empty() { Value::Null } else { json!(begin) },
        "end": if placed.is_empty() { Value::Null } else { json!(end) },
        "rate": if period > 0.0 { json!(occupancy / period) } else { Value::Null },
        "additional": additional,
        "rateWithAdditional": if period > 0.0 { json!((occupancy + additional) / period) } else { Value::Null },
        "critical": critical,
    })
}

/// The minimum headways of every two trains of `input.trains` that hold something
/// the same, in both orders: `[{ before, after, kinds: [before's, after's], least }]`.
pub fn headways(input: &Value) -> Value {
    let trains: Vec<Train> = input
        .get("trains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Train::from_json)
        .collect();
    let mut out = Vec::new();
    for a in &trains {
        for b in &trains {
            if a.id == b.id {
                continue;
            }
            if let Some(h) = least_headway(a, b) {
                out.push(json!({ "before": a.id, "after": b.id, "kinds": [a.kind, b.kind], "least": h }));
            }
        }
    }
    Value::Array(out)
}

/// A resource by its key, the times it is held, the trains that hold it.
type Keyed = (String, Vec<(f64, f64)>, Vec<String>);

/// How long each resource is held in the period [from, to) by the trains of
/// `input.trains` as they run: `[{ resource, held, rate, trains }]` — the union
/// of the times held [s], its share of the period, the trains that hold it —,
/// the most held first: a station's tracks and routes.
pub fn resource_occupancy(input: &Value) -> Value {
    let (from, to) = (num(&input["from"]), num(&input["to"]));
    let period = to - from;
    let trains: Vec<Train> = input
        .get("trains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Train::from_json)
        .collect();
    let mut keyed: Vec<Keyed> = Vec::new();
    for t in &trains {
        for h in &t.held {
            let key = key_of(&h.resource);
            let (a, b) = (h.begin.max(from), h.end.min(to));
            if b <= a {
                continue;
            }
            match keyed.iter_mut().find(|k| k.0 == key) {
                Some(k) => {
                    k.1.push((a, b));
                    if !k.2.contains(&t.id) {
                        k.2.push(t.id.clone());
                    }
                }
                None => keyed.push((key, vec![(a, b)], vec![t.id.clone()])),
            }
        }
    }
    let mut out: Vec<(f64, Value)> = keyed
        .into_iter()
        .map(|(key, mut spans, ids)| {
            spans.sort_by(|x, y| x.0.total_cmp(&y.0));
            let (mut held, mut end) = (0.0, f64::NEG_INFINITY);
            for (a, b) in spans {
                if a > end {
                    held += b - a;
                    end = b;
                } else if b > end {
                    held += b - end;
                    end = b;
                }
            }
            let rate = if period > 0.0 { json!(held / period) } else { Value::Null };
            (held, json!({ "resource": key, "held": held, "rate": rate, "trains": ids }))
        })
        .collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    Value::Array(out.into_iter().map(|(_, v)| v).collect())
}

/// A conflict area as it is gathered: what is held, the trains that hold it, the times they hold it.
type Area = (Resource, Vec<String>, Vec<(f64, f64)>);

/// The conflict areas of the trains of `input.trains` in the period [from, to):
/// what two or more of them hold — a track section, or a stretch of a track
/// where the stretches they hold overlap by more than half a metre, as one —,
/// `[{ resource, trains, held, rate }]`, the trains that hold it, the union of
/// the times held [s] and its share of the period; those held by most trains
/// first, then the most held.
pub fn conflict_areas(input: &Value) -> Value {
    let (from, to) = (num(&input["from"]), num(&input["to"]));
    let period = to - from;
    let trains: Vec<Train> = input
        .get("trains")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Train::from_json)
        .collect();
    let mut areas: Vec<Area> = Vec::new();
    for t in &trains {
        for h in &t.held {
            let (a, b) = (h.begin.max(from), h.end.min(to));
            if b <= a {
                continue;
            }
            let at = areas.iter().position(|(r, _, _)| match (r, &h.resource) {
                (Resource::Section(x), Resource::Section(y)) => x == y,
                (Resource::Track(x, a0, a1), Resource::Track(y, b0, b1)) => x == y && a1.min(*b1) - a0.max(*b0) > SHARE,
                _ => false,
            });
            match at {
                Some(i) => {
                    let area = &mut areas[i];
                    if let (Resource::Track(_, a0, a1), Resource::Track(_, b0, b1)) = (&mut area.0, &h.resource) {
                        *a0 = a0.min(*b0);
                        *a1 = a1.max(*b1);
                    }
                    if !area.1.contains(&t.id) {
                        area.1.push(t.id.clone());
                    }
                    area.2.push((a, b));
                }
                None => areas.push((h.resource.clone(), vec![t.id.clone()], vec![(a, b)])),
            }
        }
    }
    let mut out: Vec<(usize, f64, Value)> = areas
        .into_iter()
        .filter(|(_, ids, _)| ids.len() >= 2)
        .map(|(resource, ids, mut spans)| {
            spans.sort_by(|x, y| x.0.total_cmp(&y.0));
            let (mut held, mut end) = (0.0, f64::NEG_INFINITY);
            for (a, b) in spans {
                if a > end {
                    held += b - a;
                    end = b;
                } else if b > end {
                    held += b - end;
                    end = b;
                }
            }
            let rate = if period > 0.0 { json!(held / period) } else { Value::Null };
            (
                ids.len(),
                held,
                json!({ "resource": key_of(&resource), "trains": ids, "held": held, "rate": rate }),
            )
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)));
    Value::Array(out.into_iter().map(|(_, _, v)| v).collect())
}
