// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The project held in the module's memory (decision 0005; decision
//! 0005): the page opens it once and from then on writes only what a
//! step changed (`session.write`), and every operation that takes the
//! project's tracks, switches or any other member of it is handed them from
//! here — the page names them (`{"$held": "tracks"}`) instead of carrying them
//! across as JSON, which cost more than most edits themselves (114 ms a drag
//! step on the PEK Bestand). The member is lent to the operation for the call,
//! not copied (`lend`, `give_back`).

use std::cell::RefCell;

use rail_across_commands::edit;
use rail_across_doc::persistence;
use rail_across_js::truthy;
use serde_json::{Value, json};

use crate::args::Args;
use crate::ops::Failure;

thread_local! {
    static HELD: RefCell<Value> = const { RefCell::new(Value::Null) };
    /// The items of the last answers on the held project, by `{"$a": key}`:
    /// what the page writes back of them it names instead of sending them
    /// again (`remember`, `written`).
    static ANSWERS: RefCell<Vec<(u64, Vec<Value>)>> = const { RefCell::new(Vec::new()) };
    static ANSWER: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many answers are kept to be named back.
const ANSWERS_KEPT: usize = 4;

/// The tracks of an answer kept to be named back; the answer's number, for the
/// page to name an item of it by (`{"$a": [answer, i]}`).
fn remember(tracks: &[Value]) -> u64 {
    let n = ANSWER.with(|a| {
        a.set(a.get() + 1);
        a.get()
    });
    ANSWERS.with(|answers| {
        let mut answers = answers.borrow_mut();
        answers.push((n, tracks.to_vec()));
        if answers.len() > ANSWERS_KEPT {
            answers.remove(0);
        }
    });
    n
}

/// Item `i` of answer `n`, where it is still kept.
fn answer_item(n: u64, i: usize) -> Option<Value> {
    ANSWERS.with(|answers| answers.borrow().iter().find(|(m, _)| *m == n).and_then(|(_, items)| items.get(i).cloned()))
}

fn not_open() -> Failure {
    Failure::new("session_not_open", None)
}

/// The members of a project written from a patch: `{"$items": [...]}` builds
/// the array anew from items given whole and `{"$k": i}`, item `i` of the
/// array held before (moved, not copied); any other value replaces the member.
fn written(old: Option<Value>, value: Value) -> Value {
    let Value::Object(mut o) = value else { return value };
    let Some(Value::Array(items)) = o.remove("$items") else {
        return Value::Object(o);
    };
    let mut before: Vec<Option<Value>> = match old {
        Some(Value::Array(a)) => a.into_iter().map(Some).collect(),
        _ => Vec::new(),
    };
    Value::Array(
        items
            .into_iter()
            .map(|item| {
                if let Some(i) = item
                    .get("$k")
                    .and_then(Value::as_u64)
                    .filter(|_| item.as_object().is_some_and(|o| o.len() == 1))
                {
                    return before.get_mut(i as usize).and_then(Option::take).unwrap_or(Value::Null);
                }
                if let Some([n, i]) = item.get("$a").and_then(Value::as_array).map(Vec::as_slice) {
                    return answer_item(n.as_u64().unwrap_or(0), i.as_u64().unwrap_or(0) as usize).unwrap_or(Value::Null);
                }
                item
            })
            .collect(),
    )
}

/// Operations that take the input as their own: the project opened and written.
pub fn run_owned(op: &str, input: &mut Value) -> Option<Result<Value, Failure>> {
    match op {
        // Hold a project: the record the page has open, `[project]`.
        "session.open" => {
            let project = input.as_array_mut().and_then(|a| a.first_mut()).map(Value::take).unwrap_or(Value::Null);
            HELD.with(|h| *h.borrow_mut() = project);
            Some(Ok(json!(true)))
        }
        // What a step changed: `[{ member: value | {"$items": […]}, … }, [members removed]]`.
        "session.write" => Some(HELD.with(|h| {
            let mut held = h.borrow_mut();
            let Value::Object(project) = &mut *held else { return Err(not_open()) };
            let mut args = std::mem::take(input);
            // An answer named that is no longer kept: nothing is written, the page sends it whole.
            let gone = args[0].as_object().into_iter().flat_map(|p| p.values()).any(|v| {
                v.get("$items").and_then(Value::as_array).into_iter().flatten().any(|item| {
                    item.get("$a").and_then(Value::as_array).map(Vec::as_slice).is_some_and(|na| match na {
                        [n, i] => answer_item(n.as_u64().unwrap_or(0), i.as_u64().unwrap_or(0) as usize).is_none(),
                        _ => true,
                    })
                })
            });
            if gone {
                return Err(Failure::new("session_answer_gone", None));
            }
            if let Some(Value::Object(patch)) = args.get_mut(0).map(Value::take) {
                for (k, v) in patch {
                    let old = project.remove(&k);
                    project.insert(k, written(old, v));
                }
            }
            for k in args.get(1).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
                project.remove(k);
            }
            Ok(json!(true))
        })),
        _ => None,
    }
}

/// Runs session operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    match op {
        "session.close" => {
            HELD.with(|h| *h.borrow_mut() = Value::Null);
            Some(Ok(json!(true)))
        }
        // Whether a project is held.
        "session.isOpen" => Some(Ok(json!(HELD.with(|h| h.borrow().is_object())))),
        // The project held, as it is kept: its infra-data-1.0 document — what a
        // working copy and a file hold — `{ carrySchemas?, check? }`.
        "session.document" => Some(HELD.with(|h| {
            let held = h.borrow();
            if !held.is_object() {
                return Err(not_open());
            }
            let flag = |k: &str, d: bool| a.option(0, k).map(|v| truthy(Some(v))).unwrap_or(d);
            Ok(persistence::project_document(&held, flag("carrySchemas", false), flag("check", true))?)
        })),
        // The project held as its record is kept: derived data stripped, numbers rounded.
        "session.kept" => Some(HELD.with(|h| {
            let held = h.borrow();
            if !held.is_object() {
                return Err(not_open());
            }
            Ok(persistence::dehydrate_project(&held))
        })),
        // planElementChange on the held project: only the tracks it touched come back.
        "session.planElementChange" => Some(HELD.with(|h| {
            let held = h.borrow();
            if !held.is_object() {
                return Err(not_open());
            }
            let empty = Vec::new();
            let tracks = held.get("tracks").and_then(Value::as_array).unwrap_or(&empty);
            let switches = held.get("switches").and_then(Value::as_array).unwrap_or(&empty);
            let mut plan = edit::plan_element_change_touched(tracks, switches, a.v(0), a.u(1), a.v(2));
            let n = remember(plan["tracks"].as_array().map(Vec::as_slice).unwrap_or(&[]));
            plan["answer"] = json!(n);
            Ok(plan)
        })),
        _ => None,
    }
}

/// Where in the input a part of the held project was lent, and which.
pub struct Lent(Vec<(Vec<usize>, Vec<String>, Part)>);

/// A part of the held project: the path to it — empty for the project itself,
/// `["tracks"]` for a member, `["tracks", 3]` for an item of it,
/// `["tracks", 3, "elements"]` for a member of that.
#[derive(Clone, PartialEq)]
struct Part(Vec<Seg>);

#[derive(Clone, PartialEq)]
enum Seg {
    Key(String),
    At(usize),
}

/// The marker `{"$held": part}` — `""` for the whole project, a member's name,
/// or the path to a part — or None.
fn marker(v: &Value) -> Option<Part> {
    let Value::Object(o) = v else { return None };
    if o.len() != 1 {
        return None;
    }
    match o.get("$held")? {
        Value::String(s) if s.is_empty() => Some(Part(Vec::new())),
        Value::String(s) => Some(Part(vec![Seg::Key(s.clone())])),
        Value::Array(path) => path
            .iter()
            .map(|s| match s {
                Value::String(k) => Some(Seg::Key(k.clone())),
                Value::Number(n) => n.as_u64().map(|i| Seg::At(i as usize)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(Part),
        _ => None,
    }
}

/// The slot of `part` in the held project (None for the whole of it).
fn held_slot<'v>(held: &'v mut Value, part: &Part) -> Option<&'v mut Value> {
    if part.0.is_empty() {
        return None;
    }
    let mut at = held;
    for seg in &part.0 {
        at = match seg {
            Seg::Key(k) => at.as_object_mut()?.get_mut(k)?,
            Seg::At(i) => at.as_array_mut()?.get_mut(*i)?,
        };
    }
    Some(at)
}

/// Whether two parts overlap — one holds the other: lending one leaves the other incomplete.
fn overlap(a: &Part, b: &Part) -> bool {
    let n = a.0.len().min(b.0.len());
    a.0[..n] == b.0[..n]
}

/// Every marker in the input — an argument, or a member or item of one, two
/// levels down — replaced by what the held project has there: moved out of
/// it the first time, copied where a part of it is named again or lent
/// already (the whole project, a member and an item of it in one call).
pub fn lend(input: &mut Value) -> Lent {
    let mut lent: Vec<(Vec<usize>, Vec<String>, Part)> = Vec::new();
    let mut copies: Vec<(Vec<usize>, Vec<String>, Part)> = Vec::new();
    HELD.with(|h| {
        let mut held = h.borrow_mut();
        if !held.is_object() {
            return;
        }
        // Copies first where a part is wanted whole by another marker: what is moved out is gone from the rest.
        let mut wanted: Vec<Part> = Vec::new();
        visit(input, &mut Vec::new(), &mut Vec::new(), 0, &mut |slot, _, _| match marker(slot) {
            Some(p) => {
                wanted.push(p);
                true
            }
            None => false,
        });
        let whole = wanted.iter().any(|w| w.0.is_empty());
        visit(input, &mut Vec::new(), &mut Vec::new(), 0, &mut |slot, idx, keys| {
            let Some(part) = marker(slot) else { return false };
            let shared = whole || wanted.iter().filter(|w| overlap(w, &part)).count() > 1;
            if part.0.is_empty() {
                *slot = held.clone();
            } else if shared {
                copies.push((idx.to_vec(), keys.to_vec(), part));
            } else if let Some(v) = held_slot(&mut held, &part) {
                *slot = v.take();
                lent.push((idx.to_vec(), keys.to_vec(), part));
            } else {
                *slot = Value::Null;
            }
            true
        });
        for (idx, keys, part) in &copies {
            let value = held_slot(&mut held, part).map(|v| v.clone()).unwrap_or(Value::Null);
            if let Some(slot) = slot_at(input, idx, keys) {
                *slot = value;
            }
        }
    });
    Lent(lent)
}

/// What `visit` calls on each slot: whether it took it.
type Visitor<'f> = dyn FnMut(&mut Value, &[usize], &[String]) -> bool + 'f;

/// Walk arguments and their members and items to depth 2; `f` returns
/// whether it took the slot (and so the walk does not go into it).
fn visit(v: &mut Value, idx: &mut Vec<usize>, keys: &mut Vec<String>, depth: usize, f: &mut Visitor<'_>) {
    if depth > 0 && f(v, idx, keys) {
        return;
    }
    if depth > 2 {
        return;
    }
    match v {
        Value::Array(items) => {
            for (i, item) in items.iter_mut().enumerate() {
                idx.push(i);
                keys.push(String::new());
                visit(item, idx, keys, depth + 1, f);
                keys.pop();
                idx.pop();
            }
        }
        Value::Object(o) => {
            for (k, item) in o.iter_mut() {
                idx.push(usize::MAX);
                keys.push(k.clone());
                visit(item, idx, keys, depth + 1, f);
                keys.pop();
                idx.pop();
            }
        }
        _ => {}
    }
}

fn slot_at<'v>(v: &'v mut Value, idx: &[usize], keys: &[String]) -> Option<&'v mut Value> {
    let mut at = v;
    for (i, k) in idx.iter().zip(keys) {
        at = if *i == usize::MAX {
            at.as_object_mut()?.get_mut(k)?
        } else {
            at.as_array_mut()?.get_mut(*i)?
        };
    }
    Some(at)
}

/// What `lend` moved out of the held project, put back.
pub fn give_back(input: &mut Value, lent: Lent) {
    if lent.0.is_empty() {
        return;
    }
    HELD.with(|h| {
        let mut held = h.borrow_mut();
        for (idx, keys, part) in lent.0 {
            if let (Some(slot), Some(home)) = (slot_at(input, &idx, &keys), held_slot(&mut held, &part)) {
                *home = slot.take();
            }
        }
    });
}
