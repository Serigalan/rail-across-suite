// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/src/.../envelope_sim_infra/MRSP.kt and
// core/envelope-sim/.../envelope_sim/pipelines/MaxSpeedEnvelope.kt,
// MaxEffortEnvelope.kt — without ETCS braking curves.

//! From speed limits to the fastest a train can run: the most restrictive
//! speed profile (MRSP), the braking curves before every lower limit and
//! stop (max speed envelope), the acceleration curves and the speed it can
//! hold on grades (max effort envelope).

use crate::SimError;
use crate::envelope::{
    ConstrainedEnvelopePartBuilder, Constraint, ConstraintType, Envelope, EnvelopeCursor, EnvelopePart, EnvelopeProfile, LimitKind, MrspEnvelopeBuilder,
    OverlayEnvelopeBuilder, constant_speed_part, limit_attrs,
};
use crate::physics::EnvelopeSimContext;
use crate::physics::overlays::{accelerate as accelerate_overlay, decelerate, maintain};

/// A speed limit over a range of the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedLimit {
    pub begin: f64,
    pub end: f64,
    /// in m/s
    pub speed: f64,
    /// The index of the limit's source, as the caller numbers them.
    pub source: Option<u32>,
}

/// Computes the MRSP — the maximum speed allowed at any given point — of a
/// train on a path of the given length.
///
/// With `add_rolling_stock_length`, a limit holds until the train's tail has
/// left it.
pub fn compute_mrsp(path_length: f64, rs_max_speed: f64, rs_length: f64, add_rolling_stock_length: bool, speed_limits: &[SpeedLimit]) -> Envelope {
    let mut builder = MrspEnvelopeBuilder::new();
    let offset = if add_rolling_stock_length { rs_length } else { 0.0 };
    for limit in speed_limits {
        // Compute where this limit is active from and to
        let start = limit.begin.max(0.0);
        let end = path_length.min(offset + limit.end);
        if limit.speed != 0.0 && start < end {
            builder.add_part(constant_speed_part(limit_attrs(LimitKind::SpeedLimit, limit.source), start, end, limit.speed));
        }
    }
    // Add a limit corresponding to the hardware's maximum operational speed
    builder.add_part(constant_speed_part(limit_attrs(LimitKind::TrainLimit, None), 0.0, path_length, rs_max_speed));
    builder.build()
}

fn increase(_prev_pos: f64, prev_speed: f64, _next_pos: f64, next_speed: f64) -> bool {
    // Works for both accelerations (forwards) and decelerations (backwards)
    prev_speed < next_speed
}

/// Generate braking curves overlay everywhere the mrsp decreases (increases
/// backwards) with a discontinuity, using constant deceleration.
fn add_slowdown_braking_curves(context: &EnvelopeSimContext, envelope: &Envelope) -> Result<Envelope, SimError> {
    let mut builder = OverlayEnvelopeBuilder::backward(envelope);
    let mut cursor = EnvelopeCursor::backward(envelope);
    let mut last_position = envelope.end_pos();
    while cursor.find_part_transition(increase) {
        if cursor.position() > last_position {
            // The next braking curve already covers this point, this braking curve is hidden
            cursor.next_part();
            continue;
        }
        let mut overlay_builder = ConstrainedEnvelopePartBuilder::with_profile(
            EnvelopeProfile::Braking,
            vec![
                Constraint::speed(0.0, ConstraintType::Floor),
                Constraint::envelope(envelope, ConstraintType::Ceiling),
            ],
        );
        let start_speed = cursor.speed();
        let start_position = cursor.position();
        decelerate(context, start_position, start_speed, &mut overlay_builder, -1.0)?;
        builder.add_part(overlay_builder.sink.build());
        cursor.next_part();
        last_position = overlay_builder.last_pos();
    }
    Ok(builder.build())
}

/// Generate braking curves overlay at every stop position.
fn add_stop_braking_curves(context: &EnvelopeSimContext, stops: &[f64], curve_with_decelerations: Envelope) -> Result<Envelope, SimError> {
    let mut envelope = curve_with_decelerations;
    for &stop in stops.iter().filter(|s| **s != 0.0) {
        let part = {
            let mut overlay_builder = ConstrainedEnvelopePartBuilder::with_profile(
                EnvelopeProfile::Braking,
                vec![
                    Constraint::speed(0.0, ConstraintType::Floor),
                    Constraint::envelope(&envelope, ConstraintType::Ceiling),
                ],
            );
            decelerate(context, stop, 0.0, &mut overlay_builder, -1.0)?;
            overlay_builder.sink.build()
        };
        let mut builder = OverlayEnvelopeBuilder::backward(&envelope);
        builder.add_part(part);
        envelope = builder.build();
    }
    Ok(envelope)
}

/// Generate a max speed envelope given a mrsp: the MRSP with braking curves
/// before every decrease and every stop.
pub fn max_speed_envelope_from(context: &EnvelopeSimContext, stops: &[f64], mrsp: &Envelope) -> Result<Envelope, SimError> {
    let max_speed_envelope = add_slowdown_braking_curves(context, mrsp)?;
    add_stop_braking_curves(context, stops, max_speed_envelope)
}

/// Detects if an envelope part is a plateau.
pub fn max_effort_plateau(part: &EnvelopePart) -> bool {
    part.min_speed() == part.max_speed()
}

/// Generate acceleration curves overlay everywhere the max speed envelope
/// increases with a discontinuity, and compute the constant speed parts to
/// check whether the train can physically maintain its speed.
pub fn add_acceleration_and_constant_speed_parts(
    context: &EnvelopeSimContext,
    max_speed_profile: &Envelope,
    initial_position: f64,
    initial_speed: f64,
) -> Result<Envelope, SimError> {
    let mut builder = OverlayEnvelopeBuilder::forward(max_speed_profile);
    let mut cursor = EnvelopeCursor::forward(max_speed_profile);
    let max_speed = max_speed_profile.interpolate_speed_right_dir(initial_position, 1.0);
    if initial_speed < max_speed {
        accelerate(context, max_speed_profile, initial_speed, initial_position, &mut builder, &mut cursor)?;
    }
    while !cursor.has_reached_end() {
        if cursor.check_part(max_effort_plateau) {
            let start_speed = cursor.step_begin_speed();
            let start_position = cursor.position();
            let part = cursor.part();
            let mut overlay_builder = ConstrainedEnvelopePartBuilder::with_profile(
                EnvelopeProfile::ConstantSpeed,
                vec![
                    Constraint::speed(start_speed, ConstraintType::Equal),
                    Constraint::position(part.begin_pos(), part.end_pos()),
                ],
            );
            maintain(context, start_position, start_speed, &mut overlay_builder, 1.0)?;
            // check if the speed can be maintained from the first position
            // before adding the part, otherwise it would only be a single point
            if overlay_builder.sink.step_count() > 1 {
                builder.add_part(overlay_builder.sink.build());
                cursor.find_position(overlay_builder.last_pos());
            }
            // if the cursor didn't reach the end of the constant speed part,
            // that means the train was slowed down by a steep ramp
            if !cursor.has_reached_end() && cursor.position() < cursor.part().end_pos() {
                let mut overlay_builder = ConstrainedEnvelopePartBuilder::with_profile(
                    EnvelopeProfile::CatchingUp,
                    vec![
                        Constraint::speed(0.0, ConstraintType::Floor),
                        Constraint::envelope(max_speed_profile, ConstraintType::Ceiling),
                    ],
                );
                let start_position = cursor.position();
                let start_speed = max_speed_profile.interpolate_speed_left_dir(start_position, 1.0);
                accelerate_overlay(context, start_position, start_speed, &mut overlay_builder, 1.0)?;
                cursor.find_position(overlay_builder.last_pos());
                if overlay_builder.last_intersection == 0 {
                    // Train stopped while trying to catch up
                    return Err(SimError::impossible(overlay_builder.last_pos()));
                }
                // check that the train was actually slowed down by the ramp
                if overlay_builder.sink.step_count() > 0 {
                    builder.add_part(overlay_builder.sink.build());
                } else if !cursor.has_reached_end() {
                    // otherwise skip this position as the train isn't really
                    // being slowed down, and step 1m further
                    let max_position = cursor.part().end_pos();
                    if cursor.position() < max_position {
                        cursor.find_position(max_position.min(cursor.position() + 1.0));
                    }
                }
            }
        } else if cursor.check_part_transition(&increase) {
            let start_speed = max_speed_profile.interpolate_speed_left_dir(cursor.position(), 1.0);
            let position = cursor.position();
            accelerate(context, max_speed_profile, start_speed, position, &mut builder, &mut cursor)?;
        } else {
            cursor.next_part();
        }
    }
    Ok(builder.build())
}

/// Accelerates starting at the given speed and position — when starting up
/// and at part transitions.
fn accelerate(
    context: &EnvelopeSimContext,
    max_speed_profile: &Envelope,
    initial_speed: f64,
    start_position: f64,
    builder: &mut OverlayEnvelopeBuilder,
    cursor: &mut EnvelopeCursor,
) -> Result<(), SimError> {
    let mut overlay_builder = ConstrainedEnvelopePartBuilder::with_profile(
        EnvelopeProfile::Accelerating,
        vec![
            Constraint::speed(0.0, ConstraintType::Floor),
            Constraint::envelope(max_speed_profile, ConstraintType::Ceiling),
        ],
    );
    accelerate_overlay(context, start_position, initial_speed, &mut overlay_builder, 1.0)?;
    cursor.find_position(overlay_builder.last_pos());
    if overlay_builder.last_intersection == 0 {
        // The train stopped before reaching the end
        return Err(SimError::impossible(overlay_builder.last_pos()));
    }
    builder.add_part(overlay_builder.sink.build());
    Ok(())
}

/// Generate a max effort envelope given a max speed envelope: the train
/// goes as fast as it can.
pub fn max_effort_envelope_from(context: &EnvelopeSimContext, initial_speed: f64, max_speed_profile: &Envelope) -> Result<Envelope, SimError> {
    let max_effort_envelope = add_acceleration_and_constant_speed_parts(context, max_speed_profile, 0.0, initial_speed)?;
    debug_assert!(max_effort_envelope.continuous, "Discontinuity in max effort envelope");
    debug_assert!(max_effort_envelope.begin_pos() == 0.0);
    Ok(max_effort_envelope)
}
