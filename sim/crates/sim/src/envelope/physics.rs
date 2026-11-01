// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope/EnvelopePhysics.kt.

//! The kinematics of one envelope step: constant acceleration over time,
//! drawn over space — a parabola.

use crate::units::{are_accelerations_equal, clamp};

/// A point of speed over space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnvelopePoint {
    pub position: f64,
    pub speed: f64,
}

impl EnvelopePoint {
    pub fn new(position: f64, speed: f64) -> Self {
        EnvelopePoint { position, speed }
    }
}

/// Compute the constant acceleration between two space / speed points.
pub fn step_acceleration(last_pos: f64, next_pos: f64, last_speed: f64, next_speed: f64) -> f64 {
    let position_delta = next_pos - last_pos;
    if position_delta == 0.0 {
        debug_assert!(last_speed == next_speed);
        return 0.0;
    }
    (next_speed * next_speed - last_speed * last_speed) / 2.0 / position_delta
}

/// Given a constant acceleration, a last known speed and a position offset, compute the new speed.
pub fn interpolate_speed_with_acceleration(acceleration: f64, last_speed: f64, position_delta: f64) -> f64 {
    (last_speed * last_speed + 2.0 * acceleration * position_delta).max(0.0).sqrt()
}

/// Compute the speed at offset positionDelta inside a given step.
pub fn interpolate_step_speed(last_pos: f64, next_pos: f64, last_speed: f64, next_speed: f64, position_delta: f64) -> f64 {
    let acceleration = step_acceleration(last_pos, next_pos, last_speed, next_speed);
    interpolate_speed_with_acceleration(acceleration, last_speed, position_delta)
}

/// Compute the time required to go from lastPos to lastPos + positionDelta inside the given range.
pub fn interpolate_step_time_at(last_pos: f64, next_pos: f64, last_speed: f64, next_speed: f64, position_delta: f64) -> f64 {
    let acceleration = step_acceleration(last_pos, next_pos, last_speed, next_speed);
    if are_accelerations_equal(acceleration, 0.0) {
        let average_speed = (last_speed + next_speed) / 2.0;
        return (position_delta / average_speed).abs();
    }
    let interpolated_speed = interpolate_speed_with_acceleration(acceleration, last_speed, position_delta);
    ((interpolated_speed - last_speed) / acceleration).abs()
}

/// Compute the time required to go from lastPos to nextPos.
pub fn interpolate_step_time(last_pos: f64, next_pos: f64, last_speed: f64, next_speed: f64) -> f64 {
    let position_delta = next_pos - last_pos;
    let acceleration = step_acceleration(last_pos, next_pos, last_speed, next_speed);
    if acceleration == 0.0 {
        return (position_delta / last_speed).abs();
    }
    ((next_speed - last_speed) / acceleration).abs()
}

/// a and b are two 1D segments, and val is clamped to their intersections.
fn clamp_1d(val: f64, a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    let min_a = a1.min(a2);
    let min_b = b1.min(b2);
    let max_a = a1.max(a2);
    let max_b = b1.max(b2);
    clamp(val, min_a.max(min_b), max_a.min(max_b))
}

/// Computes the intersection of two envelope steps. The acceleration is
/// assumed to be constant over **time** inside a step. Interpolation thus
/// needs to be done on a parabola, as envelopes are over **space**. The
/// intersection is guaranteed to be within the bounds of both segments.
#[allow(clippy::too_many_arguments)]
pub fn intersect_steps(a1_pos: f64, a1_speed: f64, a2_pos: f64, a2_speed: f64, b1_pos: f64, b1_speed: f64, b2_pos: f64, b2_speed: f64) -> EnvelopePoint {
    let mut point = EnvelopePoint::default();
    let clamp_point = |point: &mut EnvelopePoint| {
        point.position = clamp_1d(point.position, a1_pos, a2_pos, b1_pos, b2_pos);
        point.speed = clamp_1d(point.speed, a1_speed, a2_speed, b1_speed, b2_speed);
    };
    // find acceleration for the parabolas formula
    let acc_a = step_acceleration(a1_pos, a2_pos, a1_speed, a2_speed);
    let acc_b = step_acceleration(b1_pos, b2_pos, b1_speed, b2_speed);

    // A is at constant speed
    if acc_a == 0.0 {
        point.position = intersect_step_with_speed_acc(b1_speed, b1_pos, acc_b, a1_speed);
        point.speed = a1_speed;
        clamp_point(&mut point);
        return point;
    }
    // B is at constant speed
    if acc_b == 0.0 {
        point.position = intersect_step_with_speed_acc(a1_speed, a1_pos, acc_a, b1_speed);
        point.speed = b1_speed;
        clamp_point(&mut point);
        return point;
    }

    // find intersection between parabolas
    let a1_speed_square = a1_speed * a1_speed;
    let b1_speed_square = b1_speed * b1_speed;
    point.position = (b1_speed_square / 2.0 - a1_speed_square / 2.0 - acc_b * b1_pos + acc_a * a1_pos) / (acc_a - acc_b);
    // inject the position in the formula of the second parabola. this
    // function is typically used to intersect the left parabola into the
    // right one, cutting the left one at just the right point, then
    // interpolating again to cut the right one: doing it this way guarantees
    // we get the same result.
    point.speed = interpolate_speed_with_acceleration(acc_b, b1_speed, point.position - b1_pos);
    clamp_point(&mut point);
    point
}

/// Returns the position at which a step intersects a speed.
pub fn intersect_step_with_speed_acc(a1_speed: f64, a1_pos: f64, acc_a: f64, b_speed: f64) -> f64 {
    if a1_speed == b_speed {
        return a1_pos;
    }
    let res = (b_speed * b_speed - a1_speed * a1_speed + 2.0 * acc_a * a1_pos) / 2.0 / acc_a;
    debug_assert!(!res.is_infinite());
    res
}

/// Returns the position at which a step intersects a speed.
pub fn intersect_step_with_speed(a1_pos: f64, a1_speed: f64, a2_pos: f64, a2_speed: f64, b_speed: f64) -> f64 {
    let acc_a = step_acceleration(a1_pos, a2_pos, a1_speed, a2_speed);
    let mut interpolated_position = intersect_step_with_speed_acc(a1_speed, a1_pos, acc_a, b_speed);
    let min_pos = a1_pos.min(a2_pos);
    let max_pos = a1_pos.max(a2_pos);
    debug_assert!(min_pos - 0.0001 <= interpolated_position && interpolated_position <= max_pos + 0.0001);
    if interpolated_position > max_pos {
        interpolated_position = max_pos;
    }
    if interpolated_position < min_pos {
        interpolated_position = min_pos;
    }
    interpolated_position
}
