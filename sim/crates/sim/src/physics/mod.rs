// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/osrd-mp/.../envelope_sim/ (PhysicsRollingStock.kt,
// EnvelopeSimContext.kt, TrainPhysicsIntegrator.kt, IntegrationStep.kt,
// Action.kt), core/kt-osrd-path/.../EnvelopeSimPath.kt,
// core/src/.../train/RollingStock.kt and the overlays of
// core/envelope-sim/.../envelope_sim/overlays/.

//! The train's physics: its rolling stock, the grades of its path, and the
//! numerical integration of its movement.

pub mod integrator;
pub mod overlays;
pub mod path;
pub mod rolling_stock;

pub use integrator::{Action, IntegrationStep, step};
pub use path::SimPath;
pub use rolling_stock::{RollingStock, TractiveEffortPoint};

/// Everything a simulation of one train needs about it and its path.
#[derive(Clone, Debug)]
pub struct EnvelopeSimContext {
    pub rolling_stock: RollingStock,
    pub path: SimPath,
    /// The integration time step, in seconds.
    pub time_step: f64,
}

impl EnvelopeSimContext {
    pub fn new(rolling_stock: RollingStock, path: SimPath, time_step: f64) -> Self {
        EnvelopeSimContext {
            rolling_stock,
            path,
            time_step,
        }
    }
}
