// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The shared vectors (`schemas/vectors/`) that hold the JavaScript core to
//! its results hold this one too: canonical JSON and its hash, the schema
//! check, whole documents through the door.

use rail_across_doc::canonical::content_hash;
use rail_across_doc::door::{check_document, schema_of};
use rail_across_doc::manifest::{document_hash, manifest_of};
use rail_across_doc::schema::SchemaSet;
use rail_across_js::canonical_json;
use serde_json::Value;

fn vectors(name: &str) -> Vec<Value> {
    let path = format!("{}/../../schemas/vectors/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("vectors")).expect("JSON")
}

#[test]
fn canonical_json_and_its_hash() {
    for case in vectors("canonical") {
        let text = case["json"].as_str().unwrap();
        let value: Value = serde_json::from_str(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(canonical_json(&value).unwrap(), case["canonical"].as_str().unwrap(), "{text}");
        assert_eq!(content_hash(&value), case["hash"].as_str().unwrap(), "{text}");
    }
}

#[test]
fn the_schema_check() {
    let mut set = SchemaSet::new();
    for name in ["infra-data", "core", "layout", "signalling"] {
        set.add(schema_of(name).unwrap().clone()).unwrap();
    }
    for case in vectors("schema") {
        let reference = case["ref"].as_str().unwrap();
        let errors = set.check(reference, &case["value"], 20).unwrap();
        // As the vectors state them: each path once, sorted (tools/vectors.mjs).
        let mut paths: Vec<&str> = errors.iter().map(|e| e.path.as_str()).collect();
        paths.sort_by(|a, b| rail_across_js::cmp_utf16(a, b));
        paths.dedup();
        let want: Vec<&str> = case["paths"].as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
        assert_eq!(errors.is_empty(), case["valid"].as_bool().unwrap(), "{reference} {}: {errors:?}", case["value"]);
        assert_eq!(paths, want, "{reference} {}", case["value"]);
    }
}

#[test]
fn whole_documents_through_the_door() {
    for case in vectors("documents") {
        let name = case["name"].as_str().unwrap();
        match check_document(&case["document"]) {
            Ok(doc) => {
                assert!(case["valid"].as_bool().unwrap(), "{name}: taken, but the vectors refuse it");
                assert_eq!(manifest_of(&doc), case["manifest"], "{name}");
                assert_eq!(document_hash(&doc), case["hash"].as_str().unwrap(), "{name}");
            }
            Err(err) => {
                assert!(!case["valid"].as_bool().unwrap(), "{name}: refused ({err}), but the vectors take it");
                assert_eq!(err.code, case["code"].as_str().unwrap(), "{name}: {err}");
            }
        }
    }
}
