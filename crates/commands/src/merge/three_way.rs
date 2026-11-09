// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The three-way merge of two states of a project (`merge.js`).

use rail_across_js::{js_string, truthy};
use rail_across_project::validate::{new_findings, validate_project};
use rail_across_track::switch_model::{port_track, ports_of};
use serde_json::{Map, Value, json};

use super::collections::{WHOLE, collection_names, fields_of, index_collection, key_of, object_from_fields, object_label, project_fields, same_value};
use super::diff::primary;
use super::remap::{carry_references, remap_kind};

fn opt(v: Option<&Value>) -> Option<Value> {
    v.cloned()
}

/// One conflict.
fn conflict(
    collection: &str,
    object_id: &str,
    field: Option<&str>,
    label: Value,
    base: Option<Value>,
    mine: Option<Value>,
    theirs: Option<Value>,
    kind: &str,
    hint: Option<&str>,
) -> Value {
    let mut o = Map::new();
    o.insert("id".into(), json!(format!("{collection}:{object_id}:{}", field.unwrap_or(WHOLE))));
    o.insert("kind".into(), json!(kind));
    o.insert("collection".into(), json!(collection));
    o.insert("objectId".into(), json!(object_id));
    o.insert("field".into(), field.map(|f| json!(f)).unwrap_or(Value::Null));
    o.insert("label".into(), label);
    for (k, v) in [("base", base), ("mine", mine), ("theirs", theirs)] {
        if let Some(v) = v {
            o.insert(k.into(), v);
        }
    }
    if let Some(h) = hint {
        o.insert("hint".into(), json!(h));
    }
    Value::Object(o)
}

/// Merge two maps of field values against a base map (`mergeFieldMaps`): the result and the fields taken.
fn merge_field_maps(
    b: &Map<String, Value>,
    m: &Map<String, Value>,
    t: &Map<String, Value>,
    on_conflict: &mut dyn FnMut(&str, Option<Value>, Option<Value>, Option<Value>),
) -> (Map<String, Value>, Vec<String>) {
    let mut out = Map::new();
    let mut taken = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    for k in m.keys().chain(t.keys()).chain(b.keys()) {
        if !fields.contains(k) {
            fields.push(k.clone());
        }
    }
    for field in fields {
        let (vb, vm, vt) = (b.get(&field), m.get(&field), t.get(&field));
        let mine_changed = !same_value(vb, vm);
        let theirs_changed = !same_value(vb, vt);
        let mut v = vm;
        if !mine_changed && theirs_changed {
            v = vt;
            taken.push(field.clone());
        } else if mine_changed && theirs_changed && !same_value(vm, vt) {
            on_conflict(&field, opt(vb), opt(vm), opt(vt));
        }
        if let Some(v) = v {
            out.insert(field, v.clone());
        }
    }
    (out, taken)
}

fn refers_to(collection: &str, obj: &Value, track_id: &str) -> bool {
    match collection {
        "switches" => ports_of(obj)
            .iter()
            .any(|(p, _)| port_track(obj, p).map(|v| js_string(Some(v))) == Some(track_id.to_string())),
        "routes" => obj
            .get("trackIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|v| js_string(Some(v)) == track_id),
        _ => obj.get("trackId").map(|v| js_string(Some(v))) == Some(track_id.to_string()),
    }
}

fn old_track_of(states: Vec<Value>) -> impl Fn(&str) -> Option<Value> {
    let sources: Vec<Map<String, Value>> = states
        .iter()
        .map(|p| {
            p.get("tracks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|t| (js_string(t.get("id")), t.clone()))
                .collect()
        })
        .collect();
    move |id: &str| sources.iter().find_map(|s| s.get(id).cloned())
}

fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn carry(record: &Value, ctx: &Value) -> (Value, Vec<super::remap::Carried>) {
    let old = old_track_of(vec![ctx["base"].clone(), ctx["theirs"].clone(), ctx["mine"].clone()]);
    carry_references(record, &[list(ctx, "remapsMine"), list(ctx, "remapsTheirs")], &old)
}

/// Three-way merge of two states against their common ancestor (`mergeProject`).
pub fn merge_project(o: &Value) -> Value {
    let (b, m, t) = (primary(&o["base"]), primary(&o["mine"]), primary(&o["theirs"]));
    let remaps_mine: Vec<Value> = o.get("remapsMine").and_then(Value::as_array).cloned().unwrap_or_default();
    let remaps_theirs: Vec<Value> = o.get("remapsTheirs").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut conflicts: Vec<Value> = Vec::new();
    let mut applied: Vec<Value> = Vec::new();
    let (project_out, taken) = merge_field_maps(&project_fields(&b), &project_fields(&m), &project_fields(&t), &mut |f, vb, vm, vt| {
        conflicts.push(conflict("project", "project", Some(f), json!(f), vb, vm, vt, "field", None));
    });
    if !taken.is_empty() {
        applied.push(json!({ "collection": "project", "id": "project", "label": null, "kind": "changed", "fields": taken }));
    }
    let mut merged = project_out;
    let mut all_tracks: Map<String, Value> = Map::new();
    for tr in list(&b, "tracks").iter().chain(list(&t, "tracks")).chain(list(&m, "tracks")) {
        all_tracks.insert(js_string(tr.get("id")), tr.clone());
    }
    let mut sides = [b.clone(), m.clone(), t.clone()];
    let mut carried_before: Vec<(super::remap::Carried, &'static str)> = Vec::new();
    let order: Vec<&str> = std::iter::once("tracks").chain(collection_names().filter(|n| *n != "tracks")).collect();
    for name in order {
        let [bc, mc, tc] = &sides;
        let (bi, mi, ti) = (index_collection(bc, name), index_collection(mc, name), index_collection(tc, name));
        if !truthy(bc.get(name)) && !truthy(mc.get(name)) && !truthy(tc.get(name)) {
            continue;
        }
        let mut out: Map<String, Value> = Map::new();
        let mut ids: Vec<String> = mi.keys().cloned().collect();
        ids.extend(ti.keys().filter(|id| !mi.contains_key(*id)).cloned());
        ids.extend(bi.keys().filter(|id| !mi.contains_key(*id) && !ti.contains_key(*id)).cloned());
        for id in ids {
            let (bo, mo, to) = (bi.get(&id), mi.get(&id), ti.get(&id));
            let label = object_label(name, bo.or(mo).or(to), Some(&all_tracks));
            match (mo, to) {
                (Some(mv), Some(tv)) => {
                    if bo.is_none() && !same_value(Some(mv), Some(tv)) {
                        conflicts.push(conflict(name, &id, None, label, None, Some(mv.clone()), Some(tv.clone()), "both_added", None));
                        out.insert(id, mv.clone());
                        continue;
                    }
                    let empty = json!({});
                    let (fields, taken) = merge_field_maps(
                        &fields_of(name, bo.unwrap_or(&empty)),
                        &fields_of(name, mv),
                        &fields_of(name, tv),
                        &mut |f, vb, vm, vt| {
                            conflicts.push(conflict(name, &id, Some(f), label.clone(), vb, vm, vt, "field", None));
                        },
                    );
                    if !taken.is_empty() {
                        applied.push(json!({ "collection": name, "id": id, "label": label, "kind": "changed", "fields": taken }));
                    }
                    out.insert(id, object_from_fields(name, &fields));
                }
                (Some(mv), None) => {
                    let Some(bv) = bo else {
                        out.insert(id, mv.clone());
                        continue;
                    };
                    if same_value(Some(bv), Some(mv)) {
                        applied.push(json!({ "collection": name, "id": id, "label": label, "kind": "removed" }));
                        continue;
                    }
                    conflicts.push(conflict(
                        name,
                        &id,
                        None,
                        label,
                        Some(bv.clone()),
                        Some(mv.clone()),
                        None,
                        "changed_deleted",
                        remap_kind(&id, &remaps_theirs),
                    ));
                    out.insert(id, mv.clone());
                }
                (None, Some(tv)) => {
                    let Some(bv) = bo else {
                        applied.push(json!({ "collection": name, "id": id, "label": label, "kind": "added" }));
                        out.insert(id, tv.clone());
                        continue;
                    };
                    if same_value(Some(bv), Some(tv)) {
                        continue;
                    }
                    conflicts.push(conflict(
                        name,
                        &id,
                        None,
                        label,
                        Some(bv.clone()),
                        None,
                        Some(tv.clone()),
                        "deleted_changed",
                        remap_kind(&id, &remaps_mine),
                    ));
                }
                (None, None) => {}
            }
        }
        merged.insert(name.into(), Value::Array(out.values().cloned().collect()));
        if name == "tracks" {
            let merged_tracks = merged["tracks"].clone();
            let old = old_track_of(vec![b.clone(), t.clone(), m.clone()]);
            let onto = |p: &Value, side: &'static str, carried_before: &mut Vec<(super::remap::Carried, &'static str)>| -> Value {
                let mut with = p.clone();
                with["tracks"] = merged_tracks.clone();
                let (project, carried) = carry_references(&with, &[&remaps_mine, &remaps_theirs], &old);
                carried_before.extend(carried.into_iter().map(|c| (c, side)));
                let mut out = project;
                match p.get("tracks") {
                    Some(tr) => {
                        out["tracks"] = tr.clone();
                    }
                    None => {
                        out.as_object_mut().expect("a record").remove("tracks");
                    }
                }
                out
            };
            sides = [
                onto(&b, "base", &mut carried_before),
                onto(&m, "mine", &mut carried_before),
                onto(&t, "theirs", &mut carried_before),
            ];
        }
    }
    let merged = Value::Object(merged);
    let context = json!({
        "base": b, "mine": m, "theirs": t, "remapsMine": remaps_mine, "remapsTheirs": remaps_theirs,
        "known": validate_project(&m),
    });
    let (project, carried) = carry(&merged, &context);
    let mut reported: Vec<String> = Vec::new();
    for c in carried_before.iter().filter(|(_, s)| *s != "base").map(|(c, _)| c).chain(carried.iter()) {
        let key = format!("{}:{}", c.collection, js_string(Some(&c.id)));
        if reported.contains(&key) {
            continue;
        }
        let before = list(&m, &c.collection).iter().find(|o| key_of(&c.collection, o) == js_string(Some(&c.id)));
        if let Some(bf) = before
            && c.to.split(',').all(|id| refers_to(&c.collection, bf, id))
        {
            continue;
        }
        reported.push(key);
        applied.push(json!({ "collection": c.collection, "id": c.id, "label": null, "kind": "carried", "from": c.from, "to": c.to }));
    }
    let checked = validation_conflicts(&project, &context);
    let mut all = conflicts;
    all.extend(checked["conflicts"].as_array().cloned().unwrap_or_default());
    json!({ "merged": project, "conflicts": all, "warnings": checked["warnings"], "applied": applied, "context": context })
}

/// The errors a merged record has that mine did not, as conflicts (`validationConflicts`).
pub fn validation_conflicts(record: &Value, ctx: &Value) -> Value {
    let v = validate_project(record);
    let known_errors: Vec<Value> = ctx["known"]["errors"].as_array().cloned().unwrap_or_default();
    let known_warnings: Vec<Value> = ctx["known"]["warnings"].as_array().cloned().unwrap_or_default();
    let fresh = new_findings(v["errors"].as_array().map(Vec::as_slice).unwrap_or(&[]), &known_errors);
    let mut by_object: Vec<(String, Value)> = Vec::new();
    for f in fresh {
        let collection = js_string(f.get("collection"));
        let fid = js_string(f.get("id"));
        let id = format!("object:{collection}:{fid}");
        if !by_object.iter().any(|(k, _)| *k == id) {
            let find = |p: &Value| list(p, &collection).iter().find(|o| key_of(&collection, o) == fid).cloned();
            let obj = find(&ctx["mine"]).or_else(|| find(&ctx["theirs"])).or_else(|| find(record));
            let mut tracks: Map<String, Value> = Map::new();
            for p in [&ctx["base"], &ctx["theirs"], &ctx["mine"], record] {
                for t in list(p, "tracks") {
                    tracks.insert(js_string(t.get("id")), t.clone());
                }
            }
            let label = match &obj {
                Some(o) => object_label(&collection, Some(o), Some(&tracks)),
                None => f.get("label").cloned().unwrap_or(Value::Null),
            };
            let mut entry = Map::new();
            entry.insert("id".into(), json!(id));
            entry.insert("kind".into(), json!("validation"));
            entry.insert("collection".into(), json!(collection));
            entry.insert("objectId".into(), json!(fid));
            entry.insert("field".into(), Value::Null);
            entry.insert("label".into(), label);
            for (k, p) in [("base", &ctx["base"]), ("mine", &ctx["mine"]), ("theirs", &ctx["theirs"])] {
                if let Some(o) = find(p) {
                    entry.insert(k.into(), o);
                }
            }
            entry.insert("findings".into(), json!([]));
            by_object.push((id.clone(), Value::Object(entry)));
        }
        if let Some((_, e)) = by_object.iter_mut().find(|(k, _)| *k == id) {
            e["findings"].as_array_mut().expect("findings").push(f.clone());
        }
    }
    json!({
        "conflicts": by_object.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
        "warnings": new_findings(v["warnings"].as_array().map(Vec::as_slice).unwrap_or(&[]), &known_warnings),
    })
}

fn apply_choice(record: &mut Value, collection: &str, object_id: &str, field: Option<&str>, value: Option<&Value>) {
    if collection == "project" {
        let f = field.unwrap_or("");
        let o = record.as_object_mut().expect("a record");
        match value {
            None => {
                o.remove(f);
            }
            Some(v) => {
                o.insert(f.into(), v.clone());
            }
        }
        return;
    }
    if !record.get(collection).is_some_and(Value::is_array) {
        record[collection] = json!([]);
    }
    let list = record[collection].as_array_mut().expect("a list");
    let at = list.iter().position(|o| key_of(collection, o) == object_id);
    match field {
        None => match (value, at) {
            (None, Some(i)) => {
                list.remove(i);
            }
            (None, None) => {}
            (Some(v), Some(i)) => list[i] = v.clone(),
            (Some(v), None) => list.push(v.clone()),
        },
        Some(f) => {
            let Some(i) = at else { return };
            let mut fields = fields_of(collection, &list[i]);
            match value {
                None => {
                    fields.remove(f);
                }
                Some(v) => {
                    fields.insert(f.into(), v.clone());
                }
            }
            list[i] = object_from_fields(collection, &fields);
        }
    }
}

/// The record with every conflict decided (`resolve`).
pub fn resolve(merge_result: &Value, choices: &Value) -> Value {
    let ctx = &merge_result["context"];
    let mut record = merge_result["merged"].clone();
    let choice = |id: &str| choices.get(id).and_then(Value::as_str).unwrap_or("mine");
    for c in merge_result["conflicts"].as_array().into_iter().flatten() {
        if c["kind"].as_str() == Some("validation") || choice(&js_string(c.get("id"))) != "theirs" {
            continue;
        }
        let field = c.get("field").and_then(Value::as_str);
        apply_choice(
            &mut record,
            &js_string(c.get("collection")),
            &js_string(c.get("objectId")),
            field,
            c.get("theirs"),
        );
    }
    for (id, side) in choices.as_object().into_iter().flatten() {
        let Some(rest) = id.strip_prefix("object:") else { continue };
        let Some((collection, object_id)) = rest.split_once(':') else { continue };
        if collection.is_empty() || object_id.is_empty() {
            continue;
        }
        let from = if side.as_str() == Some("theirs") { &ctx["theirs"] } else { &ctx["mine"] };
        let value = list(from, collection).iter().find(|o| key_of(collection, o) == object_id).cloned();
        apply_choice(&mut record, collection, object_id, None, value.as_ref());
    }
    carry(&record, ctx).0
}
