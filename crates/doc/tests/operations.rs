//! operations-1.0 (the roadmap, phase 4, O1): the timetables as the suite keeps
//! them into a document and back, the door's verdicts, and the vectors the
//! JavaScript wrote (schemas/vectors/operations.json) giving the same hashes.

use rail_across_doc::manifest::document_hash;
use rail_across_doc::operations::{check_operations, from_operations, to_operations};
use serde_json::{Value, json};

fn point(id: &str, place: &str, stop: bool) -> Value {
    json!({ "id": id, "place": place, "station": place.to_uppercase(), "track": null, "stop": stop, "dwell": 60, "departure": null, "itinerary": null })
}

fn record() -> Value {
    json!({
        "timetables": [
            { "id": "tt1", "name": "Grundtakt", "data": {
                "dispatch": { "mode": "wait", "priority": "category", "categories": ["ICE", "RB"], "times": { "setup": 12, "approach": 1000, "release": 6 } },
                "trains": [
                    { "id": "z2", "number": "16003", "category": "RB", "name": "", "vehicle": "osrd:loco", "mode": null, "comfort": "STANDARD",
                      "departure": 30000, "margin": "5 %", "distribution": "mareco", "points": [point("a", "x", true), point("b", "y", true)] },
                    { "id": "z1", "number": "16001", "category": "RB", "name": "", "vehicle": "osrd:loco", "mode": null, "comfort": "STANDARD",
                      "departure": 28800, "margin": "5 %", "distribution": "mareco", "points": [point("c", "x", true), point("d", "y", true)] }
                ] } },
            { "id": "tt2", "name": "Güter", "data": { "trains": [
                { "id": "z1", "number": "", "category": "Gz", "name": "Kies", "vehicle": null, "mode": null, "comfort": "STANDARD",
                  "departure": 3600, "margin": "", "distribution": "linear", "points": [point("e", "y", false), point("f", "x", true)] }
            ] } }
        ]
    })
}

fn cols(doc: &Value) -> &Value {
    &doc["domains"]["operations"]["collections"]
}

#[test]
fn the_timetables_become_a_document_of_train_kinds_and_formations() {
    let doc = to_operations(&record(), true).unwrap();
    let c = cols(&doc);
    assert_eq!(c["timetables"].as_array().unwrap().len(), 2);
    assert_eq!(c["trains"].as_array().unwrap().len(), 3);
    let kinds: Vec<&str> = c["trainKinds"].as_array().unwrap().iter().map(|k| k["name"].as_str().unwrap()).collect();
    assert_eq!(kinds.len(), 3, "{kinds:?}");
    assert_eq!(
        c["formations"].as_array().unwrap().len(),
        1,
        "one formation for the vehicle both trains run with"
    );
    let tt1 = c["timetables"].as_array().unwrap().iter().find(|t| t["id"] == "tt1").unwrap();
    assert_eq!(tt1["dispatching"]["priority"], "kind");
    assert_eq!(tt1["blockingTimes"], json!({ "setup": 12, "approach": 1000, "release": 6 }));
    // the second z1, of another timetable, drawn anew
    let ids: Vec<&str> = c["trains"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"z1") && ids.contains(&"z2") && ids.len() == 3);
    for t in c["trains"].as_array().unwrap() {
        assert!(t.get("category").is_none() && t.get("vehicle").is_none());
        assert!(t["points"][0].get("track").is_none(), "no nulls in the document");
    }
}

#[test]
fn the_document_comes_back_as_the_timetables_were() {
    let rec = record();
    let back = from_operations(&to_operations(&rec, true).unwrap()).unwrap();
    let tt1 = &back["timetables"][0];
    assert_eq!(tt1["id"], "tt1");
    assert_eq!(tt1["data"]["dispatch"], rec["timetables"][0]["data"]["dispatch"]);
    // the trains by departure
    let mut want = rec["timetables"][0]["data"]["trains"].as_array().unwrap().clone();
    want.reverse();
    assert_eq!(tt1["data"]["trains"], json!(want));
    assert_eq!(back["timetables"][1]["data"]["trains"][0]["category"], "Gz");
    assert!(
        back["formations"].as_array().unwrap().is_empty(),
        "the one-vehicle formation is the train's vehicle again"
    );
    // and once more the same document
    let doc = to_operations(&rec, true).unwrap();
    assert_eq!(to_operations(&from_operations(&doc).unwrap(), true).unwrap(), doc.clone());
    assert_eq!(
        document_hash(&to_operations(&from_operations(&doc).unwrap(), true).unwrap()),
        document_hash(&doc)
    );
}

#[test]
fn a_vehicle_carried_pins_its_formation() {
    let mut rec = record();
    let stock = json!({ "name": "Lok", "mass": 84000 });
    rec["vehicles"] = json!([{ "key": "osrd:loco", "vehicle": stock }]);
    let doc = to_operations(&rec, true).unwrap();
    let hash = cols(&doc)["vehicles"][0]["hash"].as_str().unwrap().to_string();
    assert_eq!(doc["blobs"][&hash], stock);
    assert_eq!(cols(&doc)["formations"][0]["tractionUnits"][0]["hash"], hash.as_str());
    let back = from_operations(&doc).unwrap();
    assert_eq!(back["vehicles"][0], json!({ "key": "osrd:loco", "hash": hash, "vehicle": stock }));
    assert_eq!(back["formations"].as_array().unwrap().len(), 1, "a pinned formation is kept");
    assert_eq!(back["timetables"][0]["data"]["trains"][0]["vehicle"], "osrd:loco");
}

#[test]
fn what_the_door_refuses() {
    let doc = to_operations(&record(), true).unwrap();
    let mut other = doc.clone();
    other["version"] = json!("2.0");
    assert_eq!(check_operations(&other).unwrap_err().code, "unsupported_version");
    let mut dangling = doc.clone();
    dangling["domains"]["operations"]["collections"]["trains"][0]["timetableId"] = json!("nowhere");
    let err = check_operations(&dangling).unwrap_err();
    assert_eq!(err.code, "invalid_payload");
    assert!(err.details[0].contains("names no timetables nowhere"), "{err:?}");
    let mut shape = doc.clone();
    shape["domains"]["operations"]["collections"]["trains"][0]["departure"] = json!("8:00");
    assert_eq!(check_operations(&shape).unwrap_err().code, "invalid_payload");
    let mut twice = doc.clone();
    let first = twice["domains"]["operations"]["collections"]["timetables"][0].clone();
    twice["domains"]["operations"]["collections"]["timetables"].as_array_mut().unwrap().push(first);
    assert!(check_operations(&twice).unwrap_err().details.iter().any(|d| d.contains("twice")));
}

#[test]
fn the_vectors_of_the_javascript() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/vectors/operations.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        match check_operations(&case["document"]) {
            Ok(doc) => {
                assert!(case["valid"].as_bool().unwrap(), "{name}: taken, but the vectors refuse it");
                assert_eq!(document_hash(&doc), case["hash"].as_str().unwrap(), "{name}");
            }
            Err(err) => {
                assert!(!case["valid"].as_bool().unwrap(), "{name}: refused ({err}), but the vectors take it");
                assert_eq!(err.code, case["code"].as_str().unwrap(), "{name}");
            }
        }
    }
}
