// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The export read back: a project written as PlanPro and read again by the
//! crate's reader comes out as it went in — the plan of the reader's tests
//! (every object the reader takes) through the reader, the writer and the
//! reader once more; and a project in UTM, written in DB_REF.

use rail_across_geom::element::Utm;
use rail_across_geom::factory::{arc_element, straight_element};
use rail_across_js::Object;
use rail_across_planpro::planpro::planpro_record;
use rail_across_planpro::write::{planpro_id, write_planpro};
use serde_json::{Value, json};
use std::collections::HashSet;

/// The plan of the reader's tests, as its vectors hold it.
fn test_plan() -> String {
    let file = format!("{}/tests/vectors/planpro.json", env!("CARGO_MANIFEST_DIR"));
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    cases
        .iter()
        .filter(|c| c["op"] == "planpro.planproRecord" && c.get("output").is_some())
        .map(|c| c["input"][0].as_str().unwrap().to_string())
        .find(|t| t.contains("Neubau ESTW Testdorf") && t.contains("sig-x"))
        .expect("the test plan")
}

fn same(a: &Value, b: &Value, path: &str, tol: f64) -> Result<(), String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if (x - y).abs() <= tol { Ok(()) } else { Err(format!("{path}: {x} ≠ {y}")) }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} items ≠ {}", x.len(), y.len()));
            }
            x.iter().zip(y).enumerate().try_for_each(|(i, (p, q))| same(p, q, &format!("{path}/{i}"), tol))
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                match (x.get(k), y.get(k)) {
                    (Some(p), Some(q)) => same(p, q, &format!("{path}/{k}"), tol)?,
                    (p, q) => return Err(format!("{path}/{k}: {p:?} ≠ {q:?}")),
                }
            }
            Ok(())
        }
        _ if a == b => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {b}")),
    }
}

/// Every id of the record's objects, as the export writes it.
fn as_written(record: &Value) -> Value {
    let mut ids = HashSet::new();
    let mut take = |v: &Value, k: &str| {
        if let Some(s) = v.get(k).and_then(Value::as_str) {
            ids.insert(s.to_string());
        }
    };
    for t in record["tracks"].as_array().unwrap() {
        take(t, "id");
    }
    for s in record["switches"].as_array().unwrap() {
        take(s, "switchId");
    }
    for m in record["endMarks"].as_array().unwrap() {
        take(m, "id");
    }
    for (_, list) in record["signalling"]["collections"].as_object().unwrap() {
        for o in list.as_array().unwrap() {
            take(o, "id");
        }
    }
    fn walk(v: &Value, ids: &HashSet<String>) -> Value {
        match v {
            Value::String(s) if ids.contains(s) => json!(planpro_id(s)),
            Value::Array(a) => Value::Array(a.iter().map(|x| walk(x, ids)).collect()),
            Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), walk(x, ids))).collect()),
            _ => v.clone(),
        }
    }
    walk(record, &ids)
}

/// Every list of objects with ids in id order.
fn by_id(v: &Value) -> Value {
    match v {
        Value::Array(a) => {
            let mut a: Vec<Value> = a.iter().map(by_id).collect();
            if a.iter().all(|x| x.get("id").and_then(Value::as_str).is_some()) {
                a.sort_by(|x, y| x["id"].as_str().cmp(&y["id"].as_str()));
            }
            Value::Array(a)
        }
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), by_id(x))).collect()),
        _ => v.clone(),
    }
}

/// The record as it reads back once the values the report says were filled in are.
fn with_defaults(mut record: Value, written: &Value) -> Value {
    for n in written["report"]["notes"].as_array().unwrap().iter().filter(|n| n["code"] == "defaulted") {
        let id = json!(planpro_id(n["id"].as_str().unwrap()));
        if n["field"] == "Signal_Befestigungsart" {
            for s in record["signalling"]["collections"]["signals"].as_array_mut().unwrap() {
                if s["id"] == id {
                    s["mounting"] = json!("other");
                }
            }
        }
    }
    record
}

/// What the report says was not written — not the values it filled in.
fn not_written(written: &Value) -> Value {
    Value::Array(
        written["report"]["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["code"] == "not_written")
            .cloned()
            .collect(),
    )
}

#[test]
fn the_test_plan_comes_back_as_it_was_read() {
    let first = planpro_record(&test_plan(), Some("p1"), None).unwrap()["record"].clone();
    let written = write_planpro(&first, &json!({ "now": "2026-11-09T12:00:00Z", "version": "test" })).unwrap();
    let text = written["text"].as_str().unwrap();
    assert!(text.contains("<Werkzeug_Name><Wert>Rail-Across-Suite</Wert></Werkzeug_Name>"));
    assert_eq!(not_written(&written), json!([]), "{}", written["report"]);
    let again = planpro_record(text, Some("p1"), None).unwrap();
    // The export reads the project as an infra-data document, whose collections stand by id.
    same(&by_id(&again["record"]), &by_id(&with_defaults(as_written(&first), &written)), "", 1e-3).unwrap();
    // Nothing the writer writes is left unread but the danger point and the placeholder of the outside element control.
    assert_eq!(
        again["report"]["skipped"],
        json!({ "Aussenelementansteuerung": 1, "Sonstiger_Punkt": 1 }),
        "{}",
        again["report"]
    );
}

#[test]
fn ids_become_guids_by_name_and_guids_stay() {
    let g = planpro_id("sig-a");
    assert_eq!(g, planpro_id("sig-a"));
    assert_ne!(g, planpro_id("sig-b"));
    assert_eq!(g.len(), 36);
    assert_eq!(&g[14..15], "8");
    assert_eq!(planpro_id("B3C4D5E6-0000-4000-8000-000000000001"), "b3c4d5e6-0000-4000-8000-000000000001");
}

/// A turnout in UTM: a line running east into a tip, the straight route on
/// east, the branch curving off to the north, a buffer stop at the line's start.
fn utm_project() -> Value {
    let p = |e: f64, n: f64| Utm { e, n, zone: 25832 };
    let none = Object::new();
    let line = straight_element(p(690_000.0, 5_710_000.0), p(690_200.0, 5_710_000.0), &none).unwrap();
    let on = straight_element(p(690_200.0, 5_710_000.0), p(690_300.0, 5_710_000.0), &none).unwrap();
    // A left-hand arc of R 300 over about 30 m.
    let a = 30.0_f64 / 300.0;
    let end = p(690_200.0 + 300.0 * a.sin(), 5_710_000.0 + 300.0 * (1.0 - a.cos()));
    let branch = arc_element(p(690_200.0, 5_710_000.0), end, -300.0, &none).unwrap();
    let track = |id: &str, el: Value| json!({ "id": id, "epsg": 25832, "name": id, "elements": [el] });
    json!({
        "id": "utm",
        "title": "UTM",
        "tracks": [track("line", line), track("on", on), track("branch", branch)],
        "switches": [{
            "switchId": "w1", "kind": "turnout", "name": "W1", "label": "300 – 1:9",
            "ports": {
                "A": { "trackId": "line", "endpoint": "END" },
                "B1": { "trackId": "branch", "endpoint": "BEGIN" },
                "B2": { "trackId": "on", "endpoint": "BEGIN" },
            },
        }],
        "endMarks": [{ "id": "stop", "kind": "buffer_stop", "trackId": "line", "endpoint": "BEGIN" }],
        "signalling": { "collections": { "signals": [{
            "id": "sig-a", "name": "A", "kind": "real",
            "at": { "anchor": "station", "trackId": "line", "station": 100, "side": "right", "offset": 3.1, "direction": "up" },
            "logicalSignals": [{ "system": "ks", "type": "main", "function": "entry" }],
        }, {
            "id": "orphan", "kind": "real", "at": { "anchor": "plane", "e": 1, "n": 2 },
        }] } },
    })
}

#[test]
fn a_project_in_utm_is_written_in_db_ref() {
    let project = utm_project();
    let written = write_planpro(&project, &json!({})).unwrap();
    let text = written["text"].as_str().unwrap();
    // 690 km east in UTM 32 is about 11.7° east: the fourth strip.
    assert!(text.contains("<GEO_Koordinatensystem><Wert>ER0</Wert></GEO_Koordinatensystem>"));
    assert_eq!(
        not_written(&written),
        json!([{ "code": "not_written", "collection": "signals", "id": "orphan", "why": "no_track" }])
    );
    let back = planpro_record(text, Some("utm"), None).unwrap()["record"].clone();
    let tracks = back["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 3);
    assert!(tracks.iter().all(|t| t["epsg"] == json!(5684)));
    // The scale of the strip: lengths a little different, never much.
    let length = |t: &Value| t["elements"].as_array().unwrap().iter().map(|e| e["length"].as_f64().unwrap()).sum::<f64>();
    let line = tracks.iter().find(|t| t["id"] == json!(planpro_id("line"))).unwrap();
    assert!((length(line) - 200.0).abs() < 0.2, "{}", length(line));
    // The turnout: its branch the arc, its straight route the line going on.
    let sw = &back["switches"][0];
    assert_eq!(sw["ports"]["A"]["trackId"], json!(planpro_id("line")));
    assert_eq!(sw["ports"]["B1"]["trackId"], json!(planpro_id("branch")));
    assert_eq!(sw["ports"]["B2"]["trackId"], json!(planpro_id("on")));
    assert_eq!(sw["name"], json!("W1"));
    // The form as PlanPro names it: the kind of turnout, radius and ratio.
    assert_eq!(sw["label"], json!("EW 300-1:9"));
    let marks = back["endMarks"].as_array().unwrap();
    let stop = marks.iter().find(|m| m["id"] == json!(planpro_id("stop"))).unwrap();
    assert_eq!(stop["kind"], json!("buffer_stop"));
    assert_eq!(marks.len(), 3);
    // The signal at its station, scaled as the track.
    let sig = &back["signalling"]["collections"]["signals"][0];
    let station = sig["at"]["station"].as_f64().unwrap();
    assert!((station - 100.0 * length(line) / 200.0).abs() < 0.002, "{station}");
    assert_eq!(sig["at"]["side"], json!("right"));
    assert_eq!(sig["at"]["direction"], json!("up"));
    assert_eq!(sig["logicalSignals"][0]["system"], json!("ks"));
    assert_eq!(sig["logicalSignals"][0]["function"], json!("entry"));
}
