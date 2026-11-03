// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The track optimizer of the Layout-Tool, in Rust, for the browser
//! (WebAssembly; decision 0019 — it replaced the Python service): splicing
//! two tracks and its spacing to a neighbour ([`splice`], [`clearance`]),
//! reconnecting existing elements ([`reconnect`]), optimizing a track
//! ([`optimize`]), the alignment and gradient fit from axis points
//! ([`alignment`], [`gradient`]).
//!
//! Under them: the rule catalogue read and applied ([`expr`], [`katalog`]),
//! judged on an element chain as the app's element table judges it
//! ([`pruefung`]), turned into the limits a run is held to ([`grenzen`]), and
//! the geometry kernel ([`geometry`]). The catalogue is not compiled in — the
//! app hands over the file it bundles. The shared vectors in
//! packages/core/constraints/tests/ hold the expression language, the check
//! and the shortest transition lengths to the app's own (tests/vectors.rs).

pub mod alignment;
pub mod clearance;
pub mod element;
pub mod expr;
pub mod geometry;
pub mod gradient;
pub mod grenzen;
pub mod katalog;
pub mod numerics;
pub mod optimize;
pub mod poly;
pub mod pruefung;
pub mod reconnect;
pub mod rules;
pub mod splice;
pub mod track;

use std::collections::HashMap;

/// The catalogue `data` (one file of packages/core/constraints) with the
/// kernel's physics: the one number the catalogue asks of the physics
/// document is u0_factor, for the comparison radius between two rows of its
/// table.
pub fn load_katalog(data: &serde_json::Value) -> Result<katalog::Katalog, expr::Error> {
    katalog::Katalog::new(data, HashMap::from([("u0_factor".to_string(), geometry::CANT_DEFICIENCY_COEFF)]))
}

use serde_json::{Value, json};
use std::cell::RefCell;

thread_local! {
    /// The catalogue last handed over, with its limits — kept between
    /// requests, so that a dialog asking many times builds them once. Leaked
    /// on purpose: a catalogue is replaced a handful of times in a page's life.
    static RULES: RefCell<Option<&'static rules::Rules<'static>>> = const { RefCell::new(None) };
}

fn load_rules(catalogue: &Value) -> Result<&'static rules::Rules<'static>, String> {
    let katalog: &'static katalog::Katalog = Box::leak(Box::new(load_katalog(catalogue).map_err(|e| e.to_string())?));
    let rules = rules::Rules::new(katalog).map_err(|e| e.to_string())?;
    Ok(Box::leak(Box::new(rules)))
}

/// One request of the app, its JSON in, the answer's out:
/// `{"op", "catalogue"?, "request"}` — `catalogue` the rule catalogue the app
/// bundles (packages/core/constraints/db-ril-800-0110.json), sent with the
/// first request and whenever it changes, and kept until then. The ops:
///
/// * `splice` — "Zwei Gleise zusammenfügen", with or without a spacing ([`splice::splice_payload`])
/// * `reconnect` — "Bestehende Elemente neu verbinden" ([`reconnect::reconnect_payload`])
/// * `optimize` — a track or one curve of it at a higher speed ([`optimize::optimize_payload`])
/// * `align` — an alignment and its gradient from measured axis points ([`alignment::align_payload`])
/// * `limits` — what a run is held to at both levels, for the regelwerk popup
///
/// A request that is not one answers `{"error": "invalid_request", "detail"}`.
pub fn run_json(text: &str) -> String {
    let answer = (|| -> Result<Value, String> {
        let message: Value = serde_json::from_str(text).map_err(|e| format!("invalid request: {e}"))?;
        if let Some(catalogue) = message.get("catalogue").filter(|c| !c.is_null()) {
            let rules = load_rules(catalogue)?;
            RULES.with(|r| *r.borrow_mut() = Some(rules));
        }
        let rules = RULES.with(|r| *r.borrow()).ok_or("invalid request: no catalogue handed over yet")?;
        let request = message.get("request").unwrap_or(&Value::Null);
        match message.get("op").and_then(Value::as_str) {
            Some("splice") => splice::splice_payload(rules, request),
            Some("reconnect") => reconnect::reconnect_payload(rules, request),
            Some("optimize") => optimize::optimize_payload(rules, request),
            Some("align") => alignment::align_payload(request),
            Some("limits") => Ok(json!({
                "id": rules.katalog.id, "name": rules.katalog.title, "version": rules.katalog.version,
                "grenzwerte": {"reg": rules.reg.describe(), "discretion": rules.discretion.describe()},
            })),
            other => Err(format!("invalid request: unknown op {other:?}")),
        }
    })();
    match answer {
        Ok(value) => value.to_string(),
        Err(detail) => json!({"error": "invalid_request", "detail": detail}).to_string(),
    }
}
