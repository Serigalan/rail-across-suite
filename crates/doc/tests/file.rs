//! The file rail-across-1.0 (decision 0029, phase 5): a project's infrastructure and operations in one file
//! and out again, each part through its door; a domain the reader does not know kept as it came.

use rail_across_doc::door::check_document;
use rail_across_doc::file::{from_file, is_file, to_file};
use rail_across_doc::operations::to_operations;
use serde_json::{Value, json};

fn case(name: &str) -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/vectors/documents.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    cases.into_iter().find(|c| c["name"] == name).unwrap()["document"].clone()
}

fn operations() -> Value {
    let point = |id: &str, place: &str| json!({ "id": id, "operationalPointId": place, "stop": true, "dwell": 60 });
    let record = json!({ "timetables": [{ "id": "tt", "name": "Takt", "data": { "trains": [
        { "id": "z", "number": "1", "category": "RB", "vehicle": "osrd:loco", "departure": 3600, "points": [point("a", "x"), point("b", "y")] }
    ] } }] });
    to_operations(&record, true).unwrap()
}

#[test]
fn a_project_goes_into_one_file_and_comes_out_as_it_was() {
    let infra = check_document(&case("the sample")).unwrap();
    let ops = operations();
    let file = to_file(Some(&infra), Some(&ops), &json!({ "savedAt": "2026-11-14T08:00:00Z" })).unwrap();
    assert!(is_file(&file));
    assert_eq!(file["format"], "rail-across");
    assert_eq!(file["projectId"], infra["domains"]["core"]["fields"]["id"]);
    assert_eq!(file["generator"]["name"], "Rail-Across-Suite");
    assert_eq!(file["infraData"], json!({ "format": "infra-data", "version": "1.0", "domains": infra["domains"] }));
    assert!(file.get("schemas").is_none(), "a file carries no schemas");
    let parts = from_file(&file).unwrap();
    assert_eq!(parts["infra"], infra);
    assert_eq!(parts["operations"], ops);
    assert_eq!(parts["savedAt"], "2026-11-14T08:00:00Z");
}

#[test]
fn a_file_may_hold_one_part() {
    let file = to_file(None, Some(&operations()), &json!({ "projectId": "p" })).unwrap();
    assert!(file.get("infraData").is_none());
    let parts = from_file(&file).unwrap();
    assert!(parts.get("infra").is_none());
    assert_eq!(parts["operations"], operations());
}

#[test]
fn a_domain_the_reader_does_not_know_is_kept() {
    let infra = check_document(&case("the sample")).unwrap();
    let mut file = to_file(Some(&infra), None, &json!({})).unwrap();
    let noise = json!({ "version": "1.0", "schema": format!("sha256:{}", "a".repeat(64)), "dependsOn": ["core"], "fields": {}, "collections": { "barriers": [{ "id": "n1" }] } });
    file["infraData"]["domains"]["noise"] = noise.clone();
    let parts = from_file(&file).unwrap();
    assert_eq!(parts["infra"]["domains"]["noise"], noise);
    let record = rail_across_doc::core_format::document_to_record(&parts["infra"]).unwrap();
    assert_eq!(record["keptDomains"]["noise"], noise);
    let back = rail_across_doc::door::to_document(&record, false, true).unwrap();
    assert_eq!(back["domains"]["noise"], noise, "and written back as it came");
}

#[test]
fn a_file_of_another_version_or_shape_is_refused() {
    let file = to_file(None, Some(&operations()), &json!({})).unwrap();
    let mut other = file.clone();
    other["version"] = json!("2.0");
    assert_eq!(from_file(&other).unwrap_err().code, "unsupported_version");
    let mut extra = file.clone();
    extra["schemas"] = json!({});
    assert_eq!(from_file(&extra).unwrap_err().code, "invalid_payload");
}
