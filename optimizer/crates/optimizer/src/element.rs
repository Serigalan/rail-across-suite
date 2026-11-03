// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! An element of a track in the app's form, as far as the rules read it.
//!
//! Every field is optional, as in the record: a straight has no radius, a
//! transition states its curvature at both ends (`r1`, `r2`, null for none),
//! the cant is one value (`cant`) or one at each end (`cantStart`, `cantEnd`,
//! decision 369 of the Layout-Tool). Fields this crate does not read are
//! ignored.

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// 0 straight, 1 arc, 2 transition.
    pub element_type: Option<i64>,
    pub transition_type: Option<String>,
    pub radius: Option<f64>,
    pub r1: Option<f64>,
    pub r2: Option<f64>,
    pub length: Option<f64>,
    pub speed: Option<f64>,
    pub cant: Option<f64>,
    pub cant_start: Option<f64>,
    pub cant_end: Option<f64>,
    /// The turnout an element belongs to; any JSON value but null.
    pub switch_id: Option<serde_json::Value>,
}

impl Element {
    pub fn is_transition(&self) -> bool {
        self.element_type == Some(2)
    }

    /// The form of a transition — 'bloss' or 'clothoid' — and None for any other element.
    pub fn form(&self) -> Option<&'static str> {
        if !self.is_transition() {
            return None;
        }
        Some(if self.transition_type.as_deref() == Some("bloss") {
            "bloss"
        } else {
            "clothoid"
        })
    }

    /// The type the catalogue names it by.
    pub fn type_id(&self) -> &'static str {
        match self.element_type {
            Some(1) => "circular_arc",
            Some(2) => "transition_curve",
            _ => "straight",
        }
    }

    pub fn in_switch(&self) -> bool {
        self.switch_id.as_ref().is_some_and(|id| !id.is_null())
    }
}
