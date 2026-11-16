//! operations-1.0 (the roadmap, phase 4, O1; decision 0029, phase 4): the
//! timetables as the suite keeps them into a document of the domains vehicles
//! and timetables and back, a document of before brought to them, the door's
//! verdicts, and the vectors the JavaScript wrote (schemas/vectors/operations.json)
//! giving the same hashes.

use std::collections::HashSet;

use rail_across_doc::ids::uuid_of as u;
use rail_across_doc::manifest::document_hash;
use rail_across_doc::operations::{check_operations, from_before, from_operations, to_operations};
use serde_json::{Value, json};

fn point(id: &str, place: &str, stop: bool) -> Value {
    json!({ "id": u(id), "operationalPointId": u(place), "trackId": null, "stop": stop, "dwell": 60, "departure": null, "itineraryId": null })
}

fn record() -> Value {
    json!({
        "timetables": [
            { "id": u("tt1"), "name": "Grundtakt", "data": {
                "dispatch": { "mode": "wait", "priority": "category", "categories": ["ICE", "RB"], "times": { "setup": 12, "approach": 1000, "release": 6 } },
                "trains": [
                    { "id": u("z2"), "number": "16003", "category": "RB", "name": "", "vehicle": "osrd:loco", "mode": null, "comfort": "STANDARD",
                      "departure": 30000, "margin": "5 %", "distribution": "mareco", "points": [point("a", "x", true), point("b", "y", true)] },
                    { "id": u("z1"), "number": "16001", "category": "RB", "name": "", "vehicle": "osrd:loco", "mode": null, "comfort": "STANDARD",
                      "departure": 28800, "margin": "5 %", "distribution": "mareco", "points": [point("c", "x", true), point("d", "y", true)] }
                ] } },
            { "id": u("tt2"), "name": "Güter", "data": { "trains": [
                { "id": u("z1"), "number": "", "category": "Gz", "name": "Kies", "vehicle": null, "mode": null, "comfort": "STANDARD",
                  "departure": 3600, "margin": "", "distribution": "linear", "points": [point("e", "y", false), point("f", "x", true)] }
            ] } }
        ]
    })
}

fn cols(doc: &Value) -> &Value {
    &doc["domains"]["timetables"]["collections"]
}

fn vehicles(doc: &Value) -> &Value {
    &doc["domains"]["vehicles"]["collections"]["vehicles"]
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
    let tt1 = c["timetables"].as_array().unwrap().iter().find(|t| t["id"] == u("tt1").as_str()).unwrap();
    assert_eq!(tt1["dispatching"]["priority"], "kind");
    assert_eq!(tt1["blockingTimes"], json!({ "setup": 12, "approach": 1000, "release": 6 }));
    // the second z1, of another timetable, drawn anew
    let ids: Vec<&str> = c["trains"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&u("z1").as_str()) && ids.contains(&u("z2").as_str()) && ids.len() == 3);
    for t in c["trains"].as_array().unwrap() {
        assert!(t.get("category").is_none() && t.get("vehicle").is_none());
        assert!(t["points"][0].get("trackId").is_none(), "no nulls in the document");
    }
    // the vehicle the trains run with, the catalogue's: named by its key
    assert_eq!(vehicles(&doc).as_array().unwrap().len(), 1);
    assert_eq!(vehicles(&doc)[0]["key"], "osrd:loco");
    assert!(vehicles(&doc)[0].get("rollingStock").is_none());
    assert_eq!(c["formations"][0]["tractionUnits"], json!([{ "vehicleId": vehicles(&doc)[0]["id"] }]));
}

#[test]
fn the_document_comes_back_as_the_timetables_were() {
    let rec = record();
    let back = from_operations(&to_operations(&rec, true).unwrap()).unwrap();
    let of = |id: &str| {
        back["timetables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == u(id).as_str())
            .unwrap()
            .clone()
    };
    let tt1 = of("tt1");
    assert_eq!(tt1["data"]["dispatch"], rec["timetables"][0]["data"]["dispatch"]);
    // the trains by departure
    let mut want = rec["timetables"][0]["data"]["trains"].as_array().unwrap().clone();
    want.reverse();
    assert_eq!(tt1["data"]["trains"], json!(want));
    assert_eq!(of("tt2")["data"]["trains"][0]["category"], "Gz");
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
fn a_vehicle_carried_keeps_its_rolling_stock_in_the_object() {
    let mut rec = record();
    let stock =
        json!({ "railjson_version": "3.3", "name": "Lok", "mass": 84000, "effort_curves": { "modes": { "15000V": { "is_electric": true, "curves": [{}] } } } });
    rec["vehicles"] = json!([{ "key": "osrd:loco", "vehicle": stock }]);
    let doc = to_operations(&rec, true).unwrap();
    assert_eq!(vehicles(&doc)[0]["rollingStock"], stock);
    assert_eq!(doc["blobs"], json!({}));
    let back = from_operations(&doc).unwrap();
    assert_eq!(back["vehicles"][0]["vehicle"], stock);
    assert_eq!(back["vehicles"][0]["key"], "osrd:loco");
    let tt1 = back["timetables"].as_array().unwrap().iter().find(|t| t["id"] == u("tt1").as_str()).unwrap();
    assert_eq!(tt1["data"]["trains"][0]["vehicle"], "osrd:loco");
}

#[test]
fn a_document_of_before_comes_in_the_two_domains() {
    // operations 1.0 as it was: one domain, a point naming its place in one member, the vehicle a blob
    let stock = json!({ "railjson_version": "3.3", "name": "Lok", "effort_curves": { "modes": { "thermal": { "is_electric": false, "curves": [{}] } } } });
    let hash = rail_across_doc::canonical::content_hash(&stock);
    let before_point =
        |id: &str, place: &str| json!({ "id": id, "place": place, "station": place, "track": "t1", "stop": true, "dwell": 30, "itinerary": "it" });
    let schema: Value = serde_json::from_str(include_str!("../../../schemas/operations/1.0.schema.json")).unwrap();
    let doc = json!({
        "format": "operations", "version": "1.0", "blobs": { hash.clone(): stock },
        "domains": { "operations": { "version": "1.0", "schema": rail_across_doc::canonical::content_hash(&schema), "fields": {}, "collections": {
            "timetables": [{ "id": "tt", "name": "T" }],
            "trains": [{ "id": "z", "timetableId": "tt", "departure": 0, "formationId": "f", "points": [before_point("p1", "op-a"), before_point("p2", "end-b")] }],
            "trainKinds": [], "formations": [{ "id": "f", "tractionUnits": [{ "vehicle": "osrd:loco", "hash": hash }] }],
            "vehicles": [{ "id": "v", "key": "osrd:loco", "hash": hash, "rollingStock": hash }],
            "connections": [], "turnarounds": [], "delayScenarios": [], "studies": [],
        } } },
    });
    let now = check_operations(&doc).unwrap();
    assert_eq!(now["domains"]["vehicles"]["collections"]["vehicles"][0]["rollingStock"], stock);
    let point = &now["domains"]["timetables"]["collections"]["trains"][0]["points"][1];
    assert_eq!(point["operationalPointId"], u("end-b").as_str(), "the door alone knows no end marks");
    let marks: HashSet<String> = ["end-b".to_string()].into();
    let with = from_before(&doc, &marks);
    let point = &with["domains"]["timetables"]["collections"]["trains"][0]["points"][1];
    assert_eq!(
        point,
        &json!({ "id": u("p2"), "endMarkId": u("end-b"), "trackId": u("t1"), "stop": true, "dwell": 30, "itineraryId": u("it") })
    );
    assert!(check_operations(&with).is_ok());
}

#[test]
fn what_the_door_refuses() {
    let doc = to_operations(&record(), true).unwrap();
    let mut other = doc.clone();
    other["version"] = json!("2.0");
    assert_eq!(check_operations(&other).unwrap_err().code, "unsupported_version");
    let mut dangling = doc.clone();
    dangling["domains"]["timetables"]["collections"]["trains"][0]["timetableId"] = json!(u("nowhere"));
    let err = check_operations(&dangling).unwrap_err();
    assert_eq!(err.code, "invalid_payload");
    assert!(err.details[0].contains("which timetables.timetables lacks"), "{err:?}");
    let mut shape = doc.clone();
    shape["domains"]["timetables"]["collections"]["trains"][0]["departure"] = json!("8:00");
    assert_eq!(check_operations(&shape).unwrap_err().code, "invalid_payload");
    let mut twice = doc.clone();
    let first = twice["domains"]["timetables"]["collections"]["timetables"][0].clone();
    twice["domains"]["timetables"]["collections"]["timetables"].as_array_mut().unwrap().push(first);
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
