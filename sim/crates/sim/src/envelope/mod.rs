// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Envelopes — speed over space, as OSRD's `envelope` package: parts,
//! envelopes, cursors over them, and the builders that overlay simulated
//! parts on a base envelope.

pub mod builders;
pub mod cursor;
#[allow(clippy::module_inception)]
pub mod envelope;
pub mod part;
pub mod physics;

pub use builders::{
    ConstrainedEnvelopePartBuilder, Constraint, ConstraintType, EnvelopePartBuilder, MrspEnvelopeBuilder, OverlayEnvelopeBuilder, constant_speed_part,
    limit_attrs, speed_cap,
};
pub use cursor::{EnvelopeCursor, NextStepResult};
pub use envelope::{Envelope, EnvelopeBuilder, TimedPoint, min_envelopes};
pub use part::{Attrs, EnvelopePart, EnvelopeProfile, LimitKind};
pub use physics::EnvelopePoint;
