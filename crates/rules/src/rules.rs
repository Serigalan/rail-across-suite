// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The catalogue a request is held to, with its limits at both levels — built
//! once per catalogue and shared by every computation that asks it.

use crate::grenzen::{Grenzen, GrenzenError, Stufe};
use crate::katalog::Katalog;

pub struct Rules<'k> {
    pub katalog: &'k Katalog,
    /// The limits at the Regelwert and at the Ermessensgrenze.
    pub reg: Grenzen<'k>,
    pub discretion: Grenzen<'k>,
}

impl<'k> Rules<'k> {
    pub fn new(katalog: &'k Katalog) -> Result<Rules<'k>, GrenzenError> {
        Ok(Rules {
            katalog,
            reg: Grenzen::new(katalog, Stufe::Reg)?,
            discretion: Grenzen::new(katalog, Stufe::Discretion)?,
        })
    }

    /// The limits at one level.
    pub fn at(&self, stufe: Stufe) -> &Grenzen<'k> {
        match stufe {
            Stufe::Reg => &self.reg,
            Stufe::Discretion => &self.discretion,
        }
    }
}
