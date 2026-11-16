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

/// The UUID a short id of a fixture goes by in a document (decision 0029, phase 2).
fn u(id: &str) -> String {
    rail_across_doc::ids::uuid_of(id)
}

fn find<'a>(list: &'a Value, id: &str) -> &'a Value {
    let id = rail_across_doc::ids::uuid_of(id);
    list.as_array().unwrap().iter().find(|o| o["id"] == id.as_str()).unwrap_or_else(|| panic!("no {id}"))
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
        json!([u("hp-2")])
    );
    assert!(doc["domains"]["signalling"]["collections"].get("stoppingPlaces").is_none());
    // and the record holds them as the document does (decision 0028)
    let record = document_to_record(&doc).unwrap();
    assert!(find(&record["stoppingPlaces"], "hp-2").get("platformId").is_none());
    assert_eq!(find(&record["platforms"], "p1")["stoppingPlaceIds"], json!([u("hp-2")]));
    assert!(record["signalling"]["collections"].get("stoppingPlaces").is_none());
    assert_eq!(to_document(&record, false, true).unwrap(), doc);
}

#[test]
fn infra_data_1_0_reads_in_as_of_today() {
    let doc = check_document(&case("infra-data 1.0: a stopping place at a platform, a boundary marked by it")).unwrap();
    assert_eq!(doc["version"], "2.0");
    assert_eq!(domains(&doc), ["core", "civil", "signalling"]);
    assert_eq!(doc["domains"]["core"]["version"], rail_across_doc::core_format::domain_version("core").unwrap());
    assert_eq!(doc["domains"]["signalling"]["version"], rail_across_doc::core_format::domain_version("signalling").unwrap());
    let civil = &doc["domains"]["civil"];
    assert_eq!(civil["fields"]["planHeader"], json!({ "subtitle": "Lageplan" }));
    assert_eq!(
        doc["domains"]["core"]["collections"]["trackPaths"].as_array().unwrap().len(),
        1,
        "the runs the core's"
    );
    assert!(civil["collections"].get("trackPaths").is_none());
    assert_eq!(find(&civil["collections"]["platforms"], "p1")["stoppingPlaceIds"], json!([u("hp-2")]));
    let marks = &doc["domains"]["signalling"]["collections"]["stationBoundaries"][0]["marks"];
    assert_eq!(marks, &json!([{ "signalId": u("sig-a") }, { "stoppingPlaceId": u("hp-2") }]));
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
    assert_eq!(place["operationalPointId"], platform["operationalPointId"]);
    assert_eq!(platform["stoppingPlaceIds"], json!([made]));
}

#[test]
fn the_electrification_its_own_layer() {
    let doc = check_document(&case("the sample with its electrification")).unwrap();
    let record = document_to_record(&doc).unwrap();
    assert_eq!(record["electrification"]["collections"]["contactWires"][0]["voltage"], "15000V");
    assert_eq!(record["electrification"]["collections"]["suspensionPoints"].as_array().unwrap().len(), 2);
    assert_eq!(to_document(&record, false, true).unwrap(), doc);
    let mut empty = record.clone();
    empty["electrification"] =
        json!({ "fields": {}, "collections": { "contactWires": [], "masts": [], "suspensionPoints": [], "sectionInsulators": [] } });
    assert_eq!(domains(&to_document(&empty, false, true).unwrap()), ["core", "civil"]);
}

#[test]
fn an_electrified_range_of_before_is_a_contact_wire_carried_at_its_ends() {
    let doc = check_document(&case("stages: the sample with its electrification")).unwrap();
    let cols = &doc["domains"]["electrification"]["collections"];
    assert_eq!(doc["domains"]["electrification"]["version"], "3.0");
    let wires = cols["contactWires"].as_array().unwrap();
    assert_eq!(wires.len(), 1);
    assert_eq!(wires[0]["voltage"], "15000V");
    let points = cols["suspensionPoints"].as_array().unwrap();
    let mut stations: Vec<f64> = points.iter().map(|p| p["at"]["station"].as_f64().unwrap()).collect();
    stations.sort_by(f64::total_cmp);
    assert_eq!(stations, [0.0, 250.0]);
    assert!(points.iter().all(|p| p["contactWireId"] == wires[0]["id"]));
    assert_eq!(cols["masts"].as_array().unwrap().len(), 2);
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
            .filter(|h| h["id"] != u("hp-2").as_str())
            .all(|h| h.get("passengerStop").is_none())
    );
}

#[test]
fn a_2_0_document_of_before_reads_with_the_passenger_stops_and_the_runs_in_the_core() {
    // The core 2.0 and civil 1.0 as they were first: no passenger stops, the runs civil's.
    let mut doc = check_document(&case("the sample with its signalling")).unwrap();
    let run = json!({ "id": u("r"), "name": "R", "path": { "trackIds": [u("t1")] } });
    let signalling = &mut doc["domains"]["signalling"];
    signalling["version"] = json!("2.0");
    signalling["schema"] = json!("sha256:b46b740ba722a6eae945038d8a6149cd0c540258df478a77b4a9b8988ae7d515");
    let core = &mut doc["domains"]["core"];
    core["version"] = json!("2.0");
    core["schema"] = json!("sha256:53f47ced7884515ad1ed5ef2f19e4eb2b895a45b76797751110e99e2f4d08877");
    core["collections"].as_object_mut().unwrap().remove("trackPaths");
    for h in core["collections"]["stoppingPlaces"].as_array_mut().unwrap() {
        h.as_object_mut().unwrap().remove("passengerStop");
    }
    let civil = &mut doc["domains"]["civil"];
    civil["version"] = json!("1.0");
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
        json!({ "id": "proj", "tracks": [], "switches": [], "platforms": platforms, "stoppingPlaces": places })
    }
    fn place(id: &str, track: &str, a: f64, b: f64) -> Value {
        json!({ "id": id, "at": { "anchor": "station", "trackId": track, "startStation": a, "endStation": b } })
    }
    fn places(r: &Value) -> &Vec<Value> {
        r["stoppingPlaces"].as_array().unwrap()
    }
    /// The stopping places the platform `id` names.
    fn named(r: &Value, id: &str) -> Value {
        r["platforms"].as_array().unwrap().iter().find(|p| p["id"] == id).unwrap().get("stoppingPlaceIds").cloned().unwrap_or(json!([]))
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
        assert_eq!(named(&r, "p1"), json!(["hp"]));
        assert_eq!(ps[0]["passengerStop"], true);
        assert_eq!(ps[0]["at"], place("hp", "t1", 90.0, 200.0)["at"], "its stretch as the Signalling-Tool set it");
        assert!(ps[1].get("passengerStop").is_none());
    }

    #[test]
    fn one_is_made_over_the_platform_where_none_is_at_it() {
        let p = json!({ "id": "p1", "trackId": "t1", "startStation": 240.0, "endStation": 100.0, "lifespan": { "from": "open" } });
        let r = with_platform_stops(&record(json!([]), json!([p])), "p1", Some("Bstg. 1"));
        let h = &places(&r)[0];
        assert_eq!(h["name"], "Bstg. 1");
        assert_eq!(
            h["at"],
            json!({ "anchor": "station", "trackId": "t1", "startStation": 100.0, "endStation": 240.0 })
        );
        assert_eq!((named(&r, "p1"), h["passengerStop"].clone()), (json!([h["id"]]), json!(true)));
        assert_eq!(h["lifespan"], json!({ "from": "open" }), "the platform's");
        // and again, nothing more
        assert_eq!(with_platform_stops(&r, "p1", Some("Bstg. 1")), r);
    }

    #[test]
    fn a_record_without_stopping_places_gets_the_one_over_the_platform() {
        let r = with_platform_stops(&json!({ "id": "proj", "tracks": [], "switches": [], "platforms": [P1()] }), "p1", None);
        assert_eq!(places(&r).len(), 1);
        assert!(r.get("signalling").is_none(), "the core's, not the signalling's");
    }

    #[test]
    fn a_platform_moved_away_or_deleted_leaves_its_stopping_places_for_every_train() {
        let mut h = place("hp", "t1", 90.0, 200.0);
        h["passengerStop"] = json!(true);
        let moved = json!({ "id": "p1", "trackId": "t1", "startStation": 500.0, "endStation": 640.0, "stoppingPlaceIds": ["hp"] });
        let r = with_platform_stops(&record(json!([h.clone()]), json!([moved])), "p1", None);
        assert_eq!(places(&r)[0], place("hp", "t1", 90.0, 200.0));
        assert_eq!(places(&r).len(), 2, "one made where it is now");
        assert_eq!(named(&r, "p1"), json!([places(&r)[1]["id"]]), "and it names only that one");
        let gone = with_platform_stops(&record(json!([h]), json!([])), "p1", None);
        assert_eq!(places(&gone), &vec![place("hp", "t1", 90.0, 200.0)]);
    }

    #[test]
    fn a_stopping_place_another_platform_names_stays_its() {
        let mut h = place("hp", "t1", 90.0, 200.0);
        h["passengerStop"] = json!(true);
        let p0 = json!({ "id": "p0", "trackId": "t1", "startStation": 80.0, "endStation": 200.0, "side": "right", "stoppingPlaceIds": ["hp"] });
        let r = with_platform_stops(&record(json!([h.clone()]), json!([p0, P1()])), "p1", None);
        assert_eq!(places(&r), &vec![h]);
        assert_eq!((named(&r, "p0"), named(&r, "p1")), (json!(["hp"]), json!([])));
    }
}

mod stage_days {
    use rail_across_doc::core_format::document_to_record;
    use rail_across_doc::door::{check_document, to_document};
    use rail_across_doc::stages::{day_after, day_before};
    use serde_json::{Value, json};

    use super::u;

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
                track("b", json!({ "lifespan": { "to": "2028-06-01" } })),
                track("c", json!({ "lifespan": { "from": "2028-06-02" } })),
                track("d", json!({ "lifespan": { "from": "open" } })),
                track("e", json!({ "lifespan": { "from": "2027-03-01", "to": "2028-06-01" } })),
                track("f", json!({ "lifespan": { "to": "open" } })),
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
    fn a_lifespan_is_kept_as_the_record_states_it() {
        let doc = to_document(&record(), false, true).unwrap();
        assert!(doc["domains"]["core"]["fields"].get("stages").is_none(), "no list of stages");
        let back = document_to_record(&check_document(&doc).unwrap()).unwrap();
        for t in record()["tracks"].as_array().unwrap() {
            let id = u(t["id"].as_str().unwrap());
            let of = |list: &Value| list.as_array().unwrap().iter().find(|x| x["id"] == id.as_str()).unwrap().get("lifespan").cloned();
            assert_eq!(of(&doc["domains"]["core"]["collections"]["tracks"]), t.get("lifespan").cloned(), "{} in the document", t["id"]);
            assert_eq!(of(&back["tracks"]), t.get("lifespan").cloned(), "{} back", t["id"]);
        }
    }

    /// The document of `record()` as infra-data 2.0 had it last: a list of stages, the lifespans naming them.
    fn of_2_0() -> Value {
        let mut doc = to_document(&record(), false, true).unwrap();
        let core = &mut doc["domains"]["core"];
        core["version"] = json!("2.0");
        core["schema"] = json!("sha256:98b48fc30ca564b929df9b839b49d918c4f6c5b2f7d29393cce8e268818a32e3");
        core["fields"]["stages"] =
            json!([{ "id": "s0" }, { "id": "s1", "date": "2027-03-01" }, { "id": "s2", "name": "Umbau", "date": "2028-06-02" }, { "id": "s9" }]);
        let life = [
            ("b", json!({ "to": "s2" })),
            ("c", json!({ "from": "s2" })),
            ("d", json!({ "from": "s9" })),
            ("e", json!({ "from": "s1", "to": "s2" })),
            ("f", json!({ "to": "s9" })),
        ];
        for t in core["collections"]["tracks"].as_array_mut().unwrap() {
            if let Some((_, l)) = life.iter().find(|(id, _)| t["id"] == u(id).as_str()) {
                t["lifespan"] = l.clone();
            }
        }
        doc
    }

    #[test]
    fn a_document_of_2_0_states_its_stages_as_days() {
        let now = check_document(&of_2_0()).unwrap_or_else(|e| panic!("refused ({e})"));
        assert_eq!(now, to_document(&record(), false, true).unwrap(), "as the record of today makes it");
        assert_eq!(now["domains"]["core"]["version"], rail_across_doc::core_format::domain_version("core").unwrap());
        assert_eq!(check_document(&now).unwrap(), now, "and stays as it is");
    }

    #[test]
    fn a_stage_the_list_lacks_is_an_open_day() {
        let mut doc = of_2_0();
        doc["domains"]["core"]["collections"]["tracks"][2]["lifespan"] = json!({ "from": "of-another-project" });
        doc["domains"]["core"]["collections"]["tracks"][0]["lifespan"] = json!({ "from": "s0" });
        let now = check_document(&doc).unwrap_or_else(|e| panic!("refused ({e})"));
        let tracks = &now["domains"]["core"]["collections"]["tracks"];
        assert_eq!(tracks[2]["lifespan"], json!({ "from": "open" }));
        assert!(tracks[0].get("lifespan").is_none(), "from the existing state: no statement");
    }

    #[test]
    fn domains_of_versions_that_do_not_belong_together_are_refused() {
        let mut doc = of_2_0();
        let civil = rail_across_doc::door::schema_hashes()["civil"].clone();
        doc["domains"]["civil"] = json!({ "version": "2.0", "schema": civil, "dependsOn": ["core"], "fields": {}, "collections": {} });
        assert_eq!(check_document(&doc).unwrap_err().code, "unsupported_version");
        let mut doc = to_document(&record(), false, true).unwrap();
        doc["domains"]["core"]["version"] = json!("6.0");
        let e = check_document(&doc).unwrap_err();
        assert_eq!(e.code, "unsupported_version");
        assert!(e.details[0].contains("newer than this program reads"), "{:?}", e.details);
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
        for l in [json!({ "to": "01.06.2028" }), json!({ "to": "2028-02-30" }), json!({ "from": "2028-06-02", "to": "2028-06-01" }), json!({})] {
            let mut r = record();
            r["tracks"][1]["lifespan"] = l.clone();
            assert_eq!(to_document(&r, false, true).unwrap_err().code, "invalid_payload", "{l}");
        }
        let mut r = record();
        r["tracks"][1]["lifespan"] = json!({ "from": "2028-06-01", "to": "2028-06-01" });
        assert!(to_document(&r, false, true).is_ok(), "in service for one day");
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
                { "id": "a" }, { "id": "b", "lifespan": { "to": "2028-06-01" } },
                { "id": "c", "lifespan": { "from": "2028-06-02" } }, { "id": "d", "lifespan": { "from": "open" } },
            ],
            "bridges": [{ "id": "old", "lifespan": { "to": "open" } }, { "id": "stays" }],
            "signalling": { "fields": {}, "collections": { "signals": [{ "id": "s", "lifespan": { "from": "2030-01-01" } }] } },
            "switches": [{ "switchId": "v", "kind": "stage_link", "ports": {
                "A": { "trackId": "a", "endpoint": "END" }, "B1": { "trackId": "b", "endpoint": "BEGIN" }, "B2": { "trackId": "c", "endpoint": "BEGIN" } } }],
        });
        let keys: Vec<String> = stages_of(&r)
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(keys, ["existing", "2028-06-02", "2030-01-01", "final"], "every day something changes, a layer's too");
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
        assert_eq!(at("2031-01-01").0, ["a", "c"], "any later day: what is in service then, not the final state");
        let rest = |k: &str| {
            let v = record_at_stage(&r, k);
            let ids = |list: &serde_json::Value| list.as_array().unwrap().iter().map(|o| o["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
            (ids(&v["bridges"]), ids(&v["signalling"]["collections"]["signals"]))
        };
        assert_eq!(rest("existing"), (vec!["old".to_string(), "stays".to_string()], vec![]));
        assert_eq!(rest("2030-01-01"), (vec!["old".to_string(), "stays".to_string()], vec!["s".to_string()]), "removed some day: still there");
        assert_eq!(rest("final"), (vec!["stays".to_string()], vec!["s".to_string()]));
        assert_eq!(record_at_stage(&r, "neither"), r, "no such stage: the record as it is");
    }
}

mod structures {
    use rail_across_doc::core_format::document_to_record;
    use rail_across_doc::door::{check_document, to_document};
    use serde_json::json;

    use super::u;

    #[test]
    fn bridges_walls_and_barriers_are_civil_and_come_back() {
        let el = json!({ "elementType": 0, "startNode": [500000, 5700000], "endNode": [500100, 5700000], "bearing": 90, "length": 100, "speed": 80 });
        let at = |a: f64, b: f64, side: &str| json!({ "anchor": "station", "trackId": u("t"), "startStation": a, "endStation": b, "side": side });
        let record = json!({
            "id": u("p"), "switches": [], "platforms": [],
            "tracks": [{ "id": u("t"), "name": "T", "epsg": 25832, "elements": [el] }],
            "bridges": [{ "id": u("br"), "name": "EÜ Bach", "at": at(20.0, 40.0, "left"), "kind": "arch", "spans": 1, "crosses": "Bach" }],
            "retainingWalls": [{ "id": u("sm"), "at": at(0.0, 60.0, "right"), "kind": "cantilever", "offset": 4.5, "height": 3 }],
            "noiseBarriers": [{ "id": u("lsw"), "at": at(10.0, 90.0, "left"), "kind": "absorbing", "offset": 3.8, "height": 2 }],
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

mod references {
    use super::{case, u};
    use rail_across_doc::door::{check_document, without_dangling};
    use serde_json::json;

    #[test]
    fn a_reference_to_nothing_is_refused_and_named() {
        let mut doc = check_document(&case("the sample with its signalling")).unwrap();
        doc["domains"]["signalling"]["collections"]["switchElements"][0]["switchId"] = json!(u("w9"));
        let e = check_document(&doc).unwrap_err();
        assert_eq!(e.code, "invalid_payload");
        assert_eq!(
            e.details,
            [format!("/domains/signalling/collections/switchElements/0/switchId: names {}, which core.switches lacks", u("w9"))]
        );
    }

    #[test]
    fn what_names_nothing_is_cut_with_what_needs_it() {
        let mut doc = check_document(&case("the sample with its signalling")).unwrap();
        let sig = &mut doc["domains"]["signalling"]["collections"];
        sig["switchElements"][0]["switchId"] = json!(u("w9"));
        sig["signals"][0]["operationalPointId"] = json!("op-nowhere");
        let (cut, what) = without_dangling(&doc);
        let paths: Vec<&str> = what.iter().filter_map(|c| c["path"].as_str()).collect();
        // the switch element requires its switch: it goes, and the flank protection it requests with it;
        // the signal's station is its own to lose
        assert!(paths.contains(&"/collections/switchElements/0"), "{paths:?}");
        assert!(paths.contains(&"/collections/signals/0/operationalPointId"), "{paths:?}");
        assert!(cut["domains"]["signalling"]["collections"]["switchElements"].as_array().unwrap().is_empty());
        assert!(cut["domains"]["signalling"]["collections"]["flankProtection"].as_array().unwrap().is_empty());
        assert!(check_document(&cut).is_ok(), "nothing dangles after it");
        assert_eq!(without_dangling(&cut).1, Vec::<serde_json::Value>::new(), "and a second time nothing is cut");
    }
}

#[test]
fn a_switch_of_core_4_0_has_its_branch_at_b1_again() {
    // A switch of core 5.0: B1 its branch, B2 its main track, as the record and the OSRD export have them.
    let mut now = check_document(&case("the core alone")).unwrap();
    let tracks: Vec<Value> = now["domains"]["core"]["collections"]["tracks"].as_array().unwrap().iter().map(|t| t["id"].clone()).collect();
    let end = |i: usize, endpoint: &str| json!({ "trackId": tracks[i], "endpoint": endpoint });
    now["domains"]["core"]["collections"]["switches"] = json!([{
        "id": u("w1"), "kind": "switch", "formVersion": 1, "name": "W1",
        "ports": { "A": end(0, "END"), "B1": end(2, "BEGIN"), "B2": end(1, "BEGIN") },
    }]);
    let now = check_document(&now).unwrap();
    assert_eq!(now["domains"]["core"]["version"], "5.0");
    // The same switch as core 4.0 stated it, B1 its main track: read, it is the one of 5.0.
    let mut before = now.clone();
    let core = &mut before["domains"]["core"];
    core["version"] = json!("4.0");
    core["schema"] = json!("sha256:2dd6dee14dc22be99741f2b095a314185273f3662f809433d7639fe4c59f6abf");
    core["collections"]["switches"][0]["ports"] = json!({ "A": end(0, "END"), "B1": end(1, "BEGIN"), "B2": end(2, "BEGIN") });
    assert_eq!(check_document(&before).unwrap(), now);
    // The record keeps the ports as they are, and names the kind as the document does.
    let record = document_to_record(&now).unwrap();
    assert_eq!(record["switches"][0]["kind"], "switch");
    assert_eq!(record["switches"][0]["ports"]["B1"]["trackId"], tracks[2]);
    assert_eq!(to_document(&record, false, true).unwrap()["domains"]["core"], now["domains"]["core"]);
}

#[test]
fn a_turnout_of_a_record_of_before_is_a_switch_its_ports_kept() {
    let doc = check_document(&case("the core alone")).unwrap();
    let mut record = document_to_record(&doc).unwrap();
    let ids: Vec<Value> = record["tracks"].as_array().unwrap().iter().map(|t| t["id"].clone()).collect();
    let end = |i: usize, endpoint: &str| json!({ "trackId": ids[i], "endpoint": endpoint });
    record["switches"] = json!([{
        "switchId": "w1", "kind": "turnout", "formVersion": 1, "ports": { "A": end(0, "END"), "B1": end(2, "BEGIN"), "B2": end(1, "BEGIN") },
    }]);
    let sw = &to_document(&record, false, true).unwrap()["domains"]["core"]["collections"]["switches"][0];
    assert_eq!(sw["kind"], "switch");
    assert_eq!(sw["ports"]["B1"]["trackId"], ids[2]);
}
