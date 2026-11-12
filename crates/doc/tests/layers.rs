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
    assert_eq!(
        find(&doc["domains"]["civil"]["collections"]["platforms"], "p1")["stoppingPlaceIds"],
        json!(["hp-2"])
    );
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
    assert_eq!(
        doc["domains"]["core"]["collections"]["trackPaths"].as_array().unwrap().len(),
        1,
        "the runs the core's"
    );
    assert!(civil["collections"].get("trackPaths").is_none());
    assert_eq!(find(&civil["collections"]["platforms"], "p1")["stoppingPlaceIds"], json!(["hp-2"]));
    let marks = &doc["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"];
    assert_eq!(marks, &json!([{ "signalId": "sig-a" }, { "stoppingPlaceId": "hp-2" }]));
}

#[test]
fn a_boundary_marked_by_a_platform_no_stopping_place_is_at_gets_one() {
    let doc = check_document(&case("infra-data 1.0: a boundary marked by a platform no stopping place is at")).unwrap();
    let marks = doc["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"]
        .as_array()
        .unwrap()
        .clone();
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
    record["signalling"]["collections"]["stationBoundaries"][0]["marks"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "platformId": "p1" }));
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

#[test]
fn a_boundary_keeps_its_headway() {
    let doc = check_document(&case("a boundary's headway")).unwrap();
    let record = document_to_record(&doc).unwrap();
    assert_eq!(record["endMarks"][0]["headway"], 120);
    assert_eq!(to_document(&record, false, true).unwrap(), doc);
}

#[test]
fn a_stopping_place_a_platform_names_is_a_passenger_stop() {
    let doc = check_document(&case("the sample with its signalling")).unwrap();
    let places = &doc["domains"]["core"]["collections"]["stoppingPlaces"];
    assert_eq!(find(places, "hp-2")["passengerStop"], true);
    assert!(
        places
            .as_array()
            .unwrap()
            .iter()
            .filter(|h| h["id"] != "hp-2")
            .all(|h| h.get("passengerStop").is_none())
    );
}

#[test]
fn a_2_0_document_of_before_reads_with_the_passenger_stops_and_the_runs_in_the_core() {
    // The core 2.0 and civil 1.0 as they were first: no passenger stops, the runs civil's.
    let mut doc = check_document(&case("the sample with its signalling")).unwrap();
    let run = json!({ "id": "r", "name": "R", "path": { "trackIds": ["t1"] } });
    let core = &mut doc["domains"]["core"];
    core["schema"] = json!("sha256:53f47ced7884515ad1ed5ef2f19e4eb2b895a45b76797751110e99e2f4d08877");
    core["collections"].as_object_mut().unwrap().remove("trackPaths");
    for h in core["collections"]["stoppingPlaces"].as_array_mut().unwrap() {
        h.as_object_mut().unwrap().remove("passengerStop");
    }
    let civil = &mut doc["domains"]["civil"];
    civil["schema"] = json!("sha256:0b73763db4b8ca57f56472f4b8aeef8a8b02369f4f8a9e3f2b4072480c48b1fb");
    civil["collections"]["trackPaths"] = json!([run.clone()]);
    let checked = check_document(&doc).unwrap_or_else(|e| panic!("refused ({e})"));
    let hashes = rail_across_doc::door::schema_hashes();
    assert_eq!(checked["domains"]["core"]["schema"], hashes["core"].as_str());
    assert_eq!(checked["domains"]["civil"]["schema"], hashes["civil"].as_str());
    assert_eq!(
        find(&checked["domains"]["core"]["collections"]["stoppingPlaces"], "hp-2")["passengerStop"],
        true
    );
    assert_eq!(checked["domains"]["core"]["collections"]["trackPaths"], json!([run]));
    assert!(checked["domains"]["civil"]["collections"].get("trackPaths").is_none());
    // A core of between (passenger stops, no runs) takes the runs as well.
    let mut between = doc.clone();
    between["domains"]["core"]["schema"] = json!("sha256:9a5f24df5585bebdd3386021af91aa98325de4c80c5d13fca7092d2eb79ae84f");
    assert_eq!(
        check_document(&between).unwrap()["domains"]["core"]["collections"]["trackPaths"],
        checked["domains"]["core"]["collections"]["trackPaths"]
    );
}

mod platform_stops {
    use rail_across_doc::layers::with_platform_stops;
    use serde_json::{Value, json};

    fn record(places: Value, platforms: Value) -> Value {
        json!({ "id": "proj", "tracks": [], "switches": [], "platforms": platforms,
            "signalling": { "fields": {}, "collections": { "stoppingPlaces": places } } })
    }
    fn place(id: &str, track: &str, a: f64, b: f64) -> Value {
        json!({ "id": id, "at": { "anchor": "station", "trackId": track, "startStation": a, "endStation": b } })
    }
    fn places(r: &Value) -> &Vec<Value> {
        r["signalling"]["collections"]["stoppingPlaces"].as_array().unwrap()
    }
    const P1: fn() -> Value = || json!({ "id": "p1", "trackId": "t1", "startStation": 100.0, "endStation": 240.0, "side": "left" });

    #[test]
    fn a_stopping_place_at_the_platform_is_detailed_as_a_passenger_stop() {
        let r = with_platform_stops(
            &record(json!([place("hp", "t1", 90.0, 200.0), place("other", "t2", 90.0, 200.0)]), json!([P1()])),
            "p1",
            Some("Bstg. 1"),
        );
        let ps = places(&r);
        assert_eq!(ps.len(), 2, "none made: one was there");
        assert_eq!(ps[0]["platformId"], "p1");
        assert_eq!(ps[0]["passengerStop"], true);
        assert_eq!(ps[0]["at"], place("hp", "t1", 90.0, 200.0)["at"], "its stretch as the Signalling-Tool set it");
        assert!(ps[1].get("passengerStop").is_none());
    }

    #[test]
    fn one_is_made_over_the_platform_where_none_is_at_it() {
        let p = json!({ "id": "p1", "trackId": "t1", "startStation": 240.0, "endStation": 100.0, "status": "new" });
        let r = with_platform_stops(&record(json!([]), json!([p])), "p1", Some("Bstg. 1"));
        let h = &places(&r)[0];
        assert_eq!(h["name"], "Bstg. 1");
        assert_eq!(
            h["at"],
            json!({ "anchor": "station", "trackId": "t1", "startStation": 100.0, "endStation": 240.0 })
        );
        assert_eq!((h["platformId"].clone(), h["passengerStop"].clone()), (json!("p1"), json!(true)));
        assert!(h["lifespan"]["from"].is_string(), "of the platform's stage");
        // and again, nothing more
        assert_eq!(with_platform_stops(&r, "p1", Some("Bstg. 1")), r);
    }

    #[test]
    fn a_record_without_signalling_gets_the_stopping_place_in_it() {
        let r = with_platform_stops(&json!({ "id": "proj", "tracks": [], "switches": [], "platforms": [P1()] }), "p1", None);
        assert_eq!(places(&r).len(), 1);
        assert_eq!(r["signalling"]["fields"], json!({}));
    }

    #[test]
    fn a_platform_moved_away_or_deleted_leaves_its_stopping_places_for_every_train() {
        let mut h = place("hp", "t1", 90.0, 200.0);
        h["platformId"] = json!("p1");
        h["passengerStop"] = json!(true);
        let moved = json!({ "id": "p1", "trackId": "t1", "startStation": 500.0, "endStation": 640.0 });
        let r = with_platform_stops(&record(json!([h.clone()]), json!([moved])), "p1", None);
        assert_eq!(places(&r)[0], place("hp", "t1", 90.0, 200.0));
        assert_eq!(places(&r).len(), 2, "one made where it is now");
        let gone = with_platform_stops(&record(json!([h]), json!([])), "p1", None);
        assert_eq!(places(&gone), &vec![place("hp", "t1", 90.0, 200.0)]);
    }

    #[test]
    fn a_stopping_place_another_platform_names_stays_its() {
        let mut h = place("hp", "t1", 90.0, 200.0);
        h["platformId"] = json!("p0");
        h["passengerStop"] = json!(true);
        let p0 = json!({ "id": "p0", "trackId": "t1", "startStation": 80.0, "endStation": 200.0, "side": "right" });
        let r = with_platform_stops(&record(json!([h.clone()]), json!([p0, P1()])), "p1", None);
        assert_eq!(places(&r), &vec![h]);
    }
}

mod stage_days {
    use rail_across_doc::core_format::document_to_record;
    use rail_across_doc::door::{check_document, to_document};
    use rail_across_doc::stages::{day_after, day_before};
    use serde_json::{Value, json};

    fn track(id: &str, extra: Value) -> Value {
        let el = json!({ "elementType": 0, "startNode": [500000, 5700000], "endNode": [500100, 5700000], "bearing": 90, "length": 100, "speed": 80 });
        let mut t = json!({ "id": id, "name": id, "epsg": 25832, "elements": [el] });
        for (k, v) in extra.as_object().unwrap() {
            t[k] = v.clone();
        }
        t
    }

    fn record() -> Value {
        json!({
            "id": "p", "switches": [], "platforms": [],
            "tracks": [
                track("a", json!({})),
                track("b", json!({ "status": "removal", "inServiceTo": "2028-06-01" })),
                track("c", json!({ "status": "new", "inServiceFrom": "2028-06-02" })),
                track("d", json!({ "status": "new" })),
                track("e", json!({ "status": "new", "inServiceFrom": "2027-03-01", "inServiceTo": "2028-06-01" })),
            ],
        })
    }

    #[test]
    fn days_go_round_the_year() {
        assert_eq!(day_after("2028-06-01").as_deref(), Some("2028-06-02"));
        assert_eq!(day_after("2028-02-28").as_deref(), Some("2028-02-29"));
        assert_eq!(day_after("2027-12-31").as_deref(), Some("2028-01-01"));
        assert_eq!(day_before("2028-03-01").as_deref(), Some("2028-02-29"));
        assert_eq!(day_after("2028-02-30"), None);
    }

    #[test]
    fn a_stage_for_each_day_and_the_lifespans_over_them() {
        let doc = to_document(&record(), false, true).unwrap();
        let stages = doc["domains"]["core"]["fields"]["stages"].as_array().unwrap().clone();
        let days: Vec<Value> = stages.iter().map(|s| s.get("date").cloned().unwrap_or(Value::Null)).collect();
        assert_eq!(days, vec![Value::Null, json!("2027-03-01"), json!("2028-06-02"), Value::Null]);
        let tracks = doc["domains"]["core"]["collections"]["tracks"].as_array().unwrap().clone();
        let life = |id: &str| tracks.iter().find(|t| t["id"] == id).unwrap().get("lifespan").cloned().unwrap_or(Value::Null);
        let (s1, s2, last) = (stages[1]["id"].clone(), stages[2]["id"].clone(), stages[3]["id"].clone());
        assert_eq!(life("a"), Value::Null);
        assert_eq!(life("b"), json!({ "to": s2 }), "until 1 June: no longer from the stage of 2 June");
        assert_eq!(life("c"), json!({ "from": s2 }));
        assert_eq!(life("d"), json!({ "from": last }));
        assert_eq!(life("e"), json!({ "from": s1, "to": s2 }));
        // and back as it was
        let back = document_to_record(&check_document(&doc).unwrap()).unwrap();
        for t in record()["tracks"].as_array().unwrap() {
            let b = back["tracks"].as_array().unwrap().iter().find(|x| x["id"] == t["id"]).unwrap();
            for k in ["status", "inServiceFrom", "inServiceTo"] {
                assert_eq!(b.get(k), t.get(k), "{} {k}", t["id"]);
            }
        }
        assert_eq!(
            back["stages"].as_array().unwrap().iter().map(|s| s["date"].clone()).collect::<Vec<_>>(),
            vec![json!("2027-03-01"), json!("2028-06-02")]
        );
    }

    #[test]
    fn a_stage_link_joins_three_tracks() {
        let mut r = record();
        r["switches"] = json!([{ "switchId": "v", "kind": "stage_link", "formVersion": 1, "ports": {
            "A": { "trackId": "a", "endpoint": "END" }, "B1": { "trackId": "b", "endpoint": "BEGIN" }, "B2": { "trackId": "c", "endpoint": "BEGIN" } } }]);
        let doc = to_document(&r, false, true).unwrap_or_else(|e| panic!("refused ({e})"));
        assert_eq!(doc["domains"]["core"]["collections"]["switches"][0]["kind"], "stage_link");
    }

    #[test]
    fn a_day_that_is_none_is_refused() {
        let mut r = record();
        r["tracks"][1]["inServiceTo"] = json!("01.06.2028");
        assert_eq!(to_document(&r, false, true).unwrap_err().code, "invalid_payload");
    }
}

mod record_at_stage {
    use rail_across_doc::stages::{record_at_stage, stages_of};
    use serde_json::json;

    #[test]
    fn what_stands_at_each_stage() {
        let r = json!({
            "id": "p", "platforms": [],
            "tracks": [
                { "id": "a" }, { "id": "b", "status": "removal", "inServiceTo": "2028-06-01" },
                { "id": "c", "status": "new", "inServiceFrom": "2028-06-02" }, { "id": "d", "status": "new" },
            ],
            "switches": [{ "switchId": "v", "kind": "stage_link", "ports": {
                "A": { "trackId": "a", "endpoint": "END" }, "B1": { "trackId": "b", "endpoint": "BEGIN" }, "B2": { "trackId": "c", "endpoint": "BEGIN" } } }],
        });
        let keys: Vec<String> = stages_of(&r)
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(keys, ["existing", "2028-06-02", "final"]);
        let at = |k: &str| {
            let v = record_at_stage(&r, k);
            let ids: Vec<String> = v["tracks"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap().to_string()).collect();
            (ids, v["switches"][0]["ports"].clone())
        };
        let (ids, ports) = at("existing");
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(ports["B2"], json!(null), "the new track not yet: its port empty");
        let (ids, ports) = at("2028-06-02");
        assert_eq!(ids, ["a", "c"]);
        assert_eq!(ports["B1"], json!(null));
        assert_eq!(at("final").0, ["a", "c", "d"]);
        assert_eq!(at("1999-01-01").0, ["a", "b"], "a day before every stage: the existing state");
        assert_eq!(at("2028-06-01").0, ["a", "b"], "the last day of the old track");
        assert_eq!(at("2031-01-01").0, ["a", "c"], "any later day: the stage in force, not the final state");
        assert_eq!(record_at_stage(&r, "neither"), r, "no such stage: the record as it is");
    }
}

mod structures {
    use rail_across_doc::core_format::document_to_record;
    use rail_across_doc::door::{check_document, to_document};
    use serde_json::json;

    #[test]
    fn bridges_walls_and_barriers_are_civil_and_come_back() {
        let el = json!({ "elementType": 0, "startNode": [500000, 5700000], "endNode": [500100, 5700000], "bearing": 90, "length": 100, "speed": 80 });
        let at = |a: f64, b: f64, side: &str| json!({ "anchor": "station", "trackId": "t", "startStation": a, "endStation": b, "side": side });
        let record = json!({
            "id": "p", "switches": [], "platforms": [],
            "tracks": [{ "id": "t", "name": "T", "epsg": 25832, "elements": [el] }],
            "bridges": [{ "id": "br", "name": "EÜ Bach", "at": at(20.0, 40.0, "left"), "kind": "arch", "spans": 1, "crosses": "Bach" }],
            "retainingWalls": [{ "id": "sm", "at": at(0.0, 60.0, "right"), "kind": "cantilever", "offset": 4.5, "height": 3 }],
            "noiseBarriers": [{ "id": "lsw", "at": at(10.0, 90.0, "left"), "kind": "absorbing", "offset": 3.8, "height": 2 }],
        });
        let doc = to_document(&record, false, true).unwrap_or_else(|e| panic!("refused ({e})"));
        let civil = &doc["domains"]["civil"]["collections"];
        assert_eq!(civil["bridges"][0]["kind"], "arch");
        assert_eq!(civil["noiseBarriers"][0]["height"], 2);
        let back = document_to_record(&check_document(&doc).unwrap()).unwrap();
        for k in ["bridges", "retainingWalls", "noiseBarriers"] {
            assert_eq!(back[k], record[k], "{k}");
        }
    }
}
