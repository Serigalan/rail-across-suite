// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The catalogue a request is held to, with its limits at both levels — built
//! once per catalogue and shared by every computation that asks it.

use crate::catalogue::Catalogue;
use crate::limits::{Level, Limits, LimitsError};

pub struct Rules<'k> {
    pub catalogue: &'k Catalogue,
    /// The limits at the Regelwert and at the Ermessensgrenze.
    pub reg: Limits<'k>,
    pub discretion: Limits<'k>,
}

impl<'k> Rules<'k> {
    pub fn new(catalogue: &'k Catalogue) -> Result<Rules<'k>, LimitsError> {
        Ok(Rules {
            catalogue,
            reg: Limits::new(catalogue, Level::Reg)?,
            discretion: Limits::new(catalogue, Level::Discretion)?,
        })
    }

    /// The limits at one level.
    pub fn at(&self, level: Level) -> &Limits<'k> {
        match level {
            Level::Reg => &self.reg,
            Level::Discretion => &self.discretion,
        }
    }
}
