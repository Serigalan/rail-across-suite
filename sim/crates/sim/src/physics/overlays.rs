// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope_sim/overlays/
// (EnvelopeAcceleration, EnvelopeDeceleration, EnvelopeMaintain, EnvelopeCoasting).

//! Simulated envelope parts: the train accelerates, brakes, keeps its speed
//! or coasts from a point until a constraint of the consumer stops it.

use super::EnvelopeSimContext;
use super::integrator::{Action, step};
use crate::SimError;
use crate::envelope::ConstrainedEnvelopePartBuilder;

/// The most steps one simulated part may take. OSRD relies on the
/// constraints alone to end a part; this keeps a mistake from hanging.
const MAX_STEPS: usize = 10_000_000;

/// Accelerate, storing the resulting steps into consumer.
pub fn accelerate(
    context: &EnvelopeSimContext,
    start_position: f64,
    start_speed: f64,
    consumer: &mut ConstrainedEnvelopePartBuilder,
    direction: f64,
) -> Result<(), SimError> {
    consumer.check_constraints();
    if !consumer.init_envelope_part(start_position, start_speed, direction) {
        return Ok(());
    }
    let mut position = start_position;
    let mut speed = start_speed;
    for _ in 0..MAX_STEPS {
        let step = step(context, position, speed, Action::Accelerate, direction);
        position += step.position_delta;
        speed = step.end_speed;
        if step.position_delta == 0.0 && step.start_speed != step.end_speed {
            // This case causes failed assertions when added, it has to fail
            // earlier than in the generic positionDelta == 0 case
            return Err(SimError::impossible(position));
        }
        if !consumer.add_step_with_time(position, speed, step.time_delta) {
            return Ok(());
        }
        if step.position_delta == 0.0 {
            return Err(SimError::impossible(position));
        }
    }
    Err(SimError::impossible(position))
}

/// Generate a deceleration curve overlay.
pub fn decelerate(
    context: &EnvelopeSimContext,
    start_position: f64,
    start_speed: f64,
    consumer: &mut ConstrainedEnvelopePartBuilder,
    direction: f64,
) -> Result<(), SimError> {
    simulate(context, start_position, start_speed, consumer, direction, |_, _| Action::Brake)
}

/// Generate a coasting curve overlay.
pub fn coast(
    context: &EnvelopeSimContext,
    start_position: f64,
    start_speed: f64,
    consumer: &mut ConstrainedEnvelopePartBuilder,
    direction: f64,
) -> Result<(), SimError> {
    simulate(context, start_position, start_speed, consumer, direction, |_, _| Action::Coast)
}

/// Maintain the speed, storing the resulting steps into consumer.
pub fn maintain(
    context: &EnvelopeSimContext,
    start_position: f64,
    start_speed: f64,
    consumer: &mut ConstrainedEnvelopePartBuilder,
    direction: f64,
) -> Result<(), SimError> {
    simulate(context, start_position, start_speed, consumer, direction, |speed, start_speed| {
        if speed < start_speed { Action::Accelerate } else { Action::Maintain }
    })
}

fn simulate(
    context: &EnvelopeSimContext,
    start_position: f64,
    start_speed: f64,
    consumer: &mut ConstrainedEnvelopePartBuilder,
    direction: f64,
    action: impl Fn(f64, f64) -> Action,
) -> Result<(), SimError> {
    consumer.check_constraints();
    if !consumer.init_envelope_part(start_position, start_speed, direction) {
        return Ok(());
    }
    let mut position = start_position;
    let mut speed = start_speed;
    for _ in 0..MAX_STEPS {
        let step = step(context, position, speed, action(speed, start_speed), direction);
        position += step.position_delta;
        speed = step.end_speed;
        if !consumer.add_step_with_time(position, speed, step.time_delta) {
            return Ok(());
        }
    }
    Err(SimError::impossible(position))
}
