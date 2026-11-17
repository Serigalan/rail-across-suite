// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Core 4.0, its words and names (decision 0006): what core 3.1, civil 2.1 and signalling 3.1 stated
//! as numbers is stated as words — an element's, a track's and a side's kind —, every reference is named for
//! what it names and ends in `Id`, an element has no plane of its own, and a switch is a `switch`, no longer a
//! `turnout`. `forward` makes the domains of a document so, `back` undoes what the record has not taken over
//! yet: the door's step from 3.1 takes the one, the record — which keeps the old words until it follows the
//! document (decision 0006) — both. Core 4.0 had a switch's `B1` its main track and `B2` its branch;
//! core 5.0 has them as the suite always did and as the OSRD export writes them, `B1` the branch
//! (`swap_switch_ports`).

use rail_across_js::Object;
use serde_json::{Value, json};

const ELEMENT_TYPES: &[(i64, &str)] = &[(0, "straight"), (1, "arc"), (2, "transition")];
const TRACK_TYPES: &[(i64, &str)] = &[(1, "line"), (2, "station")];
const SIDES: &[(i64, &str)] = &[(1, "regular"), (2, "opposite")];

/// The references renamed: (domain, collection, old name, new name).
const NAMES: &[(&str, &str, &str, &str)] = &[];

/// The references the record names as the document does since it follows it (decision 0006): renamed
/// on the way in only, for a record of before — a working copy may still hold one.
const NAMES_FOLLOWED: &[(&str, &str, &str, &str)] = &[
    ("core", "tracks", "operationalPoint", "operationalPointId"),
    ("civil", "platforms", "operationalPoint", "operationalPointId"),
    ("signalling", "trackSections", "boundedBy", "detectionPointIds"),
    ("signalling", "routes", "clearSections", "clearSectionIds"),
    ("signalling", "overlaps", "clearSections", "clearSectionIds"),
    ("signalling", "flankProtection", "clearSections", "clearSectionIds"),
];

#[derive(Clone, Copy, PartialEq)]
enum Way {
    Forward,
    Back,
}

/// A member renamed in place, its position kept.
fn rename(o: &mut Object, from: &str, to: &str) {
    if o.contains_key(from) {
        *o = std::mem::take(o)
            .into_iter()
            .map(|(k, v)| if k == from { (to.to_string(), v) } else { (k, v) })
            .collect();
    }
}

/// A member's value in the other vocabulary; one already in it stays, one without a counterpart goes.
fn translate(o: &mut Object, key: &str, table: &[(i64, &str)], way: Way) {
    let Some(v) = o.get(key) else { return };
    let already = match way {
        Way::Forward => v.as_str().is_some_and(|s| table.iter().any(|(_, w)| *w == s)),
        Way::Back => v.as_i64().is_some_and(|n| table.iter().any(|(m, _)| *m == n)),
    };
    if already {
        return;
    }
    let other = match way {
        Way::Forward => v.as_i64().and_then(|n| table.iter().find(|(m, _)| *m == n)).map(|(_, w)| json!(w)),
        Way::Back => v.as_str().and_then(|s| table.iter().find(|(_, w)| *w == s)).map(|(n, _)| json!(n)),
    };
    match other {
        Some(x) => {
            o.insert(key.into(), x);
        }
        None => {
            o.shift_remove(key);
        }
    }
}

fn items<'a>(domains: &'a mut Object, domain: &str, collection: &str) -> impl Iterator<Item = &'a mut Object> {
    domains
        .get_mut(domain)
        .and_then(|d| d.get_mut("collections"))
        .and_then(|c| c.get_mut(collection))
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object_mut)
}

fn convert(domains: &mut Object, way: Way) {
    let names = |from: &'static str, to: &'static str| if way == Way::Forward { (from, to) } else { (to, from) };

    for track in items(domains, "core", "tracks") {
        // The record states a track's kind and side as words since it follows the document (phase 6); numbers of
        // before are read on the way in, as a working copy may hold them.
        if way == Way::Forward {
            translate(track, "trackType", TRACK_TYPES, way);
            translate(track, "side", SIDES, way);
        }
        for el in track
            .get_mut("elements")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object_mut)
        {
            // The record states an element's kind as a word since it follows the document (phase 6); a number of
            // before is read on the way in, as a working copy may hold one.
            if way == Way::Forward {
                translate(el, "elementType", ELEMENT_TYPES, way);
                el.shift_remove("epsg");
            }
        }
        // The record names a height point's dependency `depId` since it follows the document (phase 6); `dep` of
        // before is read on the way in, as a working copy may hold it.
        if way == Way::Forward {
            for h in track
                .get_mut("heights")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
                .filter_map(Value::as_object_mut)
            {
                rename(h, "dep", "depId");
            }
        }
    }
    // The record names a switch `switch` since it follows the document (phase 6); `turnout` of before is read on
    // the way in, as a working copy may hold it. The ports stay: `B1` the branch, as core 5.0 has it.
    if way == Way::Forward {
        for sw in items(domains, "core", "switches") {
            if sw.get("kind").and_then(Value::as_str) == Some("turnout") {
                sw.insert("kind".into(), json!("switch"));
            }
        }
    }
    for (domain, collection, old, new) in NAMES {
        let (from, to) = names(old, new);
        for o in items(domains, domain, collection) {
            rename(o, from, to);
        }
    }
    if way == Way::Forward {
        for (domain, collection, old, new) in NAMES_FOLLOWED {
            for o in items(domains, domain, collection) {
                rename(o, old, new);
            }
        }
        for fp in items(domains, "signalling", "flankProtection") {
            if let Some(Value::Object(pass_on)) = fp.get_mut("passOn") {
                rename(pass_on, "left", "leftId");
                rename(pass_on, "right", "rightId");
            }
        }
    }
}

/// The domains of a document in the words and names of core 4.0.
pub fn forward(domains: &mut Object) {
    convert(domains, Way::Forward);
}

/// A switch's `B1` and `B2` the other way round: between core 4.0, which had `B1` its main track, and the
/// record and core 5.0, which have `B1` its branch.
pub fn swap_switch_ports(domains: &mut Object) {
    for sw in items(domains, "core", "switches") {
        if sw.get("kind").and_then(Value::as_str) != Some("switch") {
            continue;
        }
        if let Some(Value::Object(ports)) = sw.get_mut("ports") {
            let (b1, b2) = (ports.shift_remove("B1"), ports.shift_remove("B2"));
            if let Some(v) = b2 {
                ports.insert("B1".into(), v);
            }
            if let Some(v) = b1 {
                ports.insert("B2".into(), v);
            }
        }
    }
}

/// The domains of a document in the words and names of before, as the record keeps them.
pub fn back(domains: &mut Object) {
    convert(domains, Way::Back);
}
