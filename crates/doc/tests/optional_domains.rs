//! A document is the core and what is planned on it: layout and signalling may
//! be missing, and one read without them is written back without them.

use rail_across_doc::core_format::document_to_record;
use rail_across_doc::door::{check_document, to_document};
use serde_json::Value;

fn sample() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/vectors/documents.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&text).unwrap();
    cases.into_iter().find(|c| c["name"] == "the sample with its signalling").unwrap()["document"].clone()
}

fn domains(doc: &Value) -> Vec<String> {
    doc["domains"].as_object().unwrap().keys().cloned().collect()
}

#[test]
fn a_document_without_the_domains_it_does_not_hold() {
    for (left_out, kept) in [
        (vec!["layout"], vec!["core", "signalling"]),
        (vec!["signalling"], vec!["core", "layout"]),
        (vec!["layout", "signalling"], vec!["core"]),
    ] {
        let mut doc = sample();
        for d in &left_out {
            doc["domains"].as_object_mut().unwrap().remove(*d);
        }
        let checked = check_document(&doc).unwrap_or_else(|e| panic!("without {left_out:?}: refused ({e})"));
        let record = document_to_record(&checked).unwrap();
        let back = to_document(&record, false, true).unwrap();
        assert_eq!(domains(&back), kept, "without {left_out:?}");
        assert_eq!(back["domains"]["core"], checked["domains"]["core"], "without {left_out:?}: the core as it was");
    }
}

#[test]
fn a_document_without_its_core_is_refused() {
    let mut doc = sample();
    doc["domains"].as_object_mut().unwrap().remove("core");
    assert_eq!(check_document(&doc).unwrap_err().code, "invalid_payload");
}
