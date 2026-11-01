// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Running time simulation of a train, ported from the simulation core of
//! OSRD, the Open Source Railway Designer of the OpenRail Association
//! (<https://github.com/OpenRailAssociation/osrd>, `core/`, at
//! fda69ee1f8027175096371d258c563fc98f9295e).
//!
//! What is ported is OSRD's envelope simulation: the most restrictive speed
//! profile, braking and acceleration curves, the speed held on grades, the
//! linear and MARECO allowances, and the standalone simulation of one train
//! that combines them with its schedule. Every module names the OSRD file
//! it comes from. Not ported: ETCS braking curves, electrification and
//! power restrictions, the signalling simulation, pathfinding and conflict
//! detection.
//!
//! The entry point is [`simulate_json`] (or [`simulation::run_simulation`]):
//! a path given by its length, grades and speed limits, a rolling stock, a
//! schedule and margins in; the runs of the train out.

pub mod allowances;
pub mod envelope;
pub mod physics;
pub mod pipelines;
pub mod simulation;
pub mod units;

use std::fmt;

pub use simulation::{SimulationRequest, SimulationResult, run_simulation};

/// Why a simulation failed.
#[derive(Clone, Debug, PartialEq)]
pub enum SimError {
    /// The request does not describe a train on a path.
    InvalidInput(String),
    /// The train cannot run the path — it stops on a grade, say — at a position, in m.
    ImpossibleSimulation {
        position: f64,
    },
    AllowanceRangeOutOfBounds,
    AllowanceOutOfBounds,
    /// The allowance asks for more time than the train can take.
    AllowanceTooMuchTime {
        range_index: Option<usize>,
    },
    /// The allowance asks for less time than the train needs.
    AllowanceNotEnoughTime {
        range_index: Option<usize>,
    },
    AllowanceDiscontinuity,
}

impl SimError {
    pub(crate) fn impossible(position: f64) -> Self {
        SimError::ImpossibleSimulation { position }
    }

    pub(crate) fn range_index(&self) -> Option<usize> {
        match self {
            SimError::AllowanceTooMuchTime { range_index } | SimError::AllowanceNotEnoughTime { range_index } => *range_index,
            _ => None,
        }
    }

    pub(crate) fn with_range_index(self, index: usize) -> Self {
        match self {
            SimError::AllowanceTooMuchTime { .. } => SimError::AllowanceTooMuchTime { range_index: Some(index) },
            SimError::AllowanceNotEnoughTime { .. } => SimError::AllowanceNotEnoughTime { range_index: Some(index) },
            other => other,
        }
    }

    /// A short code for the error, stable for callers.
    pub fn code(&self) -> &'static str {
        match self {
            SimError::InvalidInput(_) => "invalid_input",
            SimError::ImpossibleSimulation { .. } => "impossible_simulation",
            SimError::AllowanceRangeOutOfBounds => "allowance_range_out_of_bounds",
            SimError::AllowanceOutOfBounds => "allowance_out_of_bounds",
            SimError::AllowanceTooMuchTime { .. } => "allowance_too_much_time",
            SimError::AllowanceNotEnoughTime { .. } => "allowance_not_enough_time",
            SimError::AllowanceDiscontinuity => "allowance_discontinuity",
        }
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::InvalidInput(what) => write!(f, "invalid input: {what}"),
            SimError::ImpossibleSimulation { position } => write!(f, "the train cannot run the path, it stops at {position:.0} m"),
            SimError::AllowanceRangeOutOfBounds => write!(f, "an allowance range lies outside the allowance"),
            SimError::AllowanceOutOfBounds => write!(f, "the allowance lies outside the run"),
            SimError::AllowanceTooMuchTime { .. } => write!(f, "the margins ask for more time than the train can take"),
            SimError::AllowanceNotEnoughTime { .. } => write!(f, "the margins ask for less time than the train needs"),
            SimError::AllowanceDiscontinuity => write!(f, "the margins could not be met"),
        }
    }
}

impl std::error::Error for SimError {}

/// Simulates the train of a JSON [`SimulationRequest`] and returns the
/// JSON of a [`SimulationResult`], or of `{ "error": { "code", "message" } }`.
pub fn simulate_json(request: &str) -> String {
    let result = serde_json::from_str::<SimulationRequest>(request)
        .map_err(|e| SimError::InvalidInput(e.to_string()))
        .and_then(|request| run_simulation(&request));
    match result {
        Ok(result) => serde_json::to_string(&result).expect("a result serializes"),
        Err(e) => serde_json::json!({ "error": { "code": e.code(), "message": e.to_string() } }).to_string(),
    }
}
