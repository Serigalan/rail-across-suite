//! infra-data-2.0's layers (decision 0025): a document is the core and what is
//! planned on it, every layer may be missing; an infra-data 1.0 document reads
//! in as 2.0 — the platforms and layout into civil, the stopping places into the
//! core, a boundary marked by a platform marked by the stopping place at it.

use rail_across_doc::core_format::document_to_record;
use rail_across_doc::door::{check_document, to_document};
use serde_json::{Value, json};

fn case(name: &str) -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/vectors/documents.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    cases.into_iter().find(|c| c["name"] == name).unwrap_or_else(|| panic!("no case {name}"))["document"].clone()
}

fn domains(doc: &Value) -> Vec<String> {
    doc["domains"].as_object().unwrap().keys().cloned().collect()
}

fn find<'a>(list: &'a Value, id: &str) -> &'a Value {
    list.as_array().unwrap().iter().find(|o| o["id"] == id).unwrap_or_else(|| panic!("no {id}"))
}

#[test]
fn a_document_without_the_layers_it_does_not_hold() {
    for (left_out, kept) in [
        (vec!["civil"], vec!["core", "signalling"]),
        (vec!["signalling"], vec!["core", "civil"]),
        (vec!["civil", "signalling"], vec!["core"]),
    ] {
        let mut doc = case("the sample with its signalling");
        for d in &left_out {
            doc["domains"].as_object_mut().unwrap().remove(*d);
        }
        let checked = check_document(&doc).unwrap_or_else(|e| panic!("without {left_out:?}: refused ({e})"));
        let back = to_document(&document_to_record(&checked).unwrap(), false, true).unwrap();
        assert_eq!(domains(&back), kept, "without {left_out:?}");
        assert_eq!(back["domains"]["core"], checked["domains"]["core"], "without {left_out:?}: the core as it was");
    }
}

#[test]
fn a_document_without_its_core_is_refused() {
    let mut doc = case("the sample");
    doc["domains"].as_object_mut().unwrap().remove("core");
    assert_eq!(check_document(&doc).unwrap_err().code, "invalid_payload");
}

#[test]
fn the_stopping_places_in_the_core_and_the_platform_naming_them() {
    let doc = check_document(&case("the sample with its signalling")).unwrap();
    let core = &doc["domains"]["core"]["collections"];
    assert!(core.get("platforms").is_none());
    assert!(find(&core["stoppingPlaces"], "hp-2").get("platformId").is_none());
    assert_eq!(find(&doc["domains"]["civil"]["collections"]["platforms"], "p1")["stoppingPlaceIds"], json!(["hp-2"]));
    assert!(doc["domains"]["signalling"]["collections"].get("stoppingPlaces").is_none());
    // and the record names the platform at its stopping place again, in its signalling
    let record = document_to_record(&doc).unwrap();
    assert_eq!(find(&record["signalling"]["collections"]["stoppingPlaces"], "hp-2")["platformId"], "p1");
    assert_eq!(find(&record["platforms"], "p1").get("stoppingPlaceIds"), None);
    assert_eq!(to_document(&record, false, true).unwrap(), doc);
}

#[test]
fn infra_data_1_0_reads_in_as_2_0() {
    let doc = check_document(&case("infra-data 1.0: a stopping place at a platform, a boundary marked by it")).unwrap();
    assert_eq!(doc["version"], "2.0");
    assert_eq!(domains(&doc), ["core", "civil", "signalling"]);
    assert_eq!(doc["domains"]["core"]["version"], "2.0");
    assert_eq!(doc["domains"]["signalling"]["version"], "2.0");
    let civil = &doc["domains"]["civil"];
    assert_eq!(civil["fields"]["planHeader"], json!({ "subtitle": "Lageplan" }));
    assert_eq!(civil["collections"]["trackPaths"].as_array().unwrap().len(), 1);
    assert_eq!(find(&civil["collections"]["platforms"], "p1")["stoppingPlaceIds"], json!(["hp-2"]));
    let marks = &doc["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"];
    assert_eq!(marks, &json!([{ "signalId": "sig-a" }, { "stoppingPlaceId": "hp-2" }]));
}

#[test]
fn a_boundary_marked_by_a_platform_no_stopping_place_is_at_gets_one() {
    let doc = check_document(&case("infra-data 1.0: a boundary marked by a platform no stopping place is at")).unwrap();
    let marks = doc["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"].as_array().unwrap().clone();
    let made = marks[1]["stoppingPlaceId"].as_str().unwrap().to_string();
    let place = find(&doc["domains"]["core"]["collections"]["stoppingPlaces"], &made);
    let platform = find(&doc["domains"]["civil"]["collections"]["platforms"], "p1");
    assert_eq!(place["at"], platform["at"]);
    assert_eq!(place["operationalPointId"], platform["operationalPoint"]);
    assert_eq!(platform["stoppingPlaceIds"], json!([made]));
}

#[test]
fn a_record_with_a_boundary_marked_by_a_platform_as_it_was_kept() {
    // a working copy of a browser keeps the record as infra-data 1.0 had it
    let doc = check_document(&case("the sample with its signalling")).unwrap();
    let mut record = document_to_record(&doc).unwrap();
    record["signalling"]["collections"]["stationBoundaries"][0]["marks"].as_array_mut().unwrap().push(json!({ "platformId": "p1" }));
    let written = to_document(&record, false, true).unwrap();
    let marks = &written["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"];
    assert_eq!(marks[1], json!({ "stoppingPlaceId": "hp-2" }));
}

#[test]
fn the_electrification_its_own_layer() {
    let doc = check_document(&case("the sample with its electrification")).unwrap();
    let record = document_to_record(&doc).unwrap();
    assert_eq!(record["electrification"]["collections"]["electrifications"][0]["system"], "15000V");
    assert_eq!(to_document(&record, false, true).unwrap(), doc);
    let mut empty = record.clone();
    empty["electrification"] = json!({ "fields": {}, "collections": { "electrifications": [] } });
    assert_eq!(domains(&to_document(&empty, false, true).unwrap()), ["core", "civil"]);
}
