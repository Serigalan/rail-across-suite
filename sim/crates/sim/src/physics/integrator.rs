// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/osrd-mp/.../envelope_sim/TrainPhysicsIntegrator.kt,
// IntegrationStep.kt and Action.kt, with constant braking only.

use super::EnvelopeSimContext;
use super::path::SimPath;
use super::rolling_stock::{RollingStock, max_effort};
use crate::units::{POSITION_EPSILON, SPEED_EPSILON, are_speeds_equal};

/// Gravity acceleration, in m/s².
pub const GRAVITY_ACCELERATION: f64 = 9.81;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Accelerate,
    Brake,
    Maintain,
    Coast,
}

/// One step of the integration, which always keeps positive speeds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrationStep {
    pub time_delta: f64,
    pub position_delta: f64,
    pub start_speed: f64,
    pub end_speed: f64,
    pub acceleration: f64,
    pub direction_sign: f64,
}

impl IntegrationStep {
    /// Create a new integration step which always keeps positive speeds,
    /// from a step which may not.
    pub fn from_naive_step(time_delta: f64, position_delta: f64, start_speed: f64, end_speed: f64, acceleration: f64, direction_sign: f64) -> Self {
        let (mut time_delta, mut position_delta, mut end_speed) = (time_delta, position_delta, end_speed);
        // if the end of the step dips below 0, cut the step in half
        if end_speed < 0.0 {
            debug_assert!(direction_sign * acceleration < 0.0);
            end_speed = 0.0;
            time_delta = -start_speed / (direction_sign * acceleration);
            position_delta = start_speed * time_delta + 0.5 * acceleration * time_delta * time_delta;
            position_delta = position_delta.abs().copysign(direction_sign);
        }
        debug_assert!(are_speeds_equal(end_speed, start_speed + direction_sign * acceleration * time_delta));
        IntegrationStep {
            time_delta,
            position_delta,
            start_speed,
            end_speed,
            acceleration,
            direction_sign,
        }
    }
}

/// Simulates train movement over one time step: a Runge-Kutta estimate of
/// the mean acceleration, then a Newton step with it.
pub fn step(context: &EnvelopeSimContext, initial_location: f64, initial_speed: f64, action: Action, direction_sign: f64) -> IntegrationStep {
    let integrator = Integrator {
        rolling_stock: &context.rolling_stock,
        path: &context.path,
        action,
        direction_sign,
    };
    let time_step = context.time_step;
    let half_step = time_step / 2.0;
    let step1 = integrator.step(half_step, initial_location, initial_speed);
    let step2 = integrator.step(half_step, initial_location + step1.position_delta, step1.end_speed);
    let step3 = integrator.step(time_step, initial_location + step2.position_delta, step2.end_speed);
    let step4 = integrator.step(time_step, initial_location + step3.position_delta, step3.end_speed);
    let mean_acceleration = (step1.acceleration + 2.0 * step2.acceleration + 2.0 * step3.acceleration + step4.acceleration) / 6.0;
    newton_step(time_step, initial_speed, mean_acceleration, direction_sign)
}

struct Integrator<'a> {
    rolling_stock: &'a RollingStock,
    path: &'a SimPath,
    action: Action,
    direction_sign: f64,
}

impl Integrator<'_> {
    fn step(&self, time_step: f64, position: f64, speed: f64) -> IntegrationStep {
        if self.action == Action::Brake {
            return newton_step(time_step, speed, self.rolling_stock.deceleration, self.direction_sign);
        }
        let mut traction_force = 0.0;
        let max_traction_force = max_effort(speed, &self.rolling_stock.tractive_effort_curve);
        let rolling_resistance = self.rolling_stock.rolling_resistance(speed);
        let average_grade = average_grade(self.rolling_stock, self.path, position);
        let weight_force = weight_force(self.rolling_stock, average_grade);
        if self.action == Action::Maintain {
            traction_force = rolling_resistance - weight_force;
            if traction_force <= max_traction_force {
                return newton_step(time_step, speed, 0.0, self.direction_sign);
            } else {
                traction_force = max_traction_force;
            }
        }
        if self.action == Action::Accelerate {
            traction_force = max_traction_force;
        }
        let acceleration = compute_acceleration(self.rolling_stock, rolling_resistance, weight_force, speed, traction_force, self.direction_sign);
        newton_step(time_step, speed, acceleration, self.direction_sign)
    }
}

/// The average grade under a train whose head is at a position, in m/km.
pub fn average_grade(rolling_stock: &RollingStock, path: &SimPath, head_position: f64) -> f64 {
    let tail_position = (head_position - rolling_stock.length).max(0.0).min(path.length);
    let head_position = head_position.max(0.0).min(path.length);
    path.average_grade(tail_position, head_position)
}

/// The lowest grade under a train whose head is at a position, in m/km.
pub fn min_grade(rolling_stock: &RollingStock, path: &SimPath, head_position: f64) -> f64 {
    let tail_position = (head_position - rolling_stock.length).max(0.0).min(path.length);
    let head_position = head_position.max(0.0).min(path.length);
    path.min_grade(tail_position, head_position)
}

/// The weight force of a rolling stock on a grade (m/km).
pub fn weight_force(rolling_stock: &RollingStock, grade: f64) -> f64 {
    // an angle from a m/km elevation difference
    let angle = (grade / 1000.0).atan();
    -rolling_stock.mass * GRAVITY_ACCELERATION * angle.sin()
}

/// The acceleration given a rolling stock, the forces on it, a speed and a direction.
pub fn compute_acceleration(
    rolling_stock: &RollingStock,
    rolling_resistance: f64,
    weight_force: f64,
    current_speed: f64,
    traction_force: f64,
    direction_sign: f64,
) -> f64 {
    debug_assert!(traction_force >= 0.0);
    if current_speed == 0.0 && direction_sign > 0.0 {
        // If we are stopped and the forces are not enough to compensate the
        // opposite force, the rolling resistance and braking force don't
        // apply and the speed stays at 0 — unless we integrate backwards,
        // then we need the speed to increase
        let total_other_force = traction_force + weight_force;
        if total_other_force.abs() < rolling_resistance {
            return 0.0;
        }
    }
    // as the opposite forces are reaction forces, they need to be adjusted
    // to be opposed to the other forces
    if current_speed >= 0.0 {
        (traction_force + weight_force - rolling_resistance) / rolling_stock.inertia
    } else {
        (traction_force + weight_force + rolling_resistance) / rolling_stock.inertia
    }
}

/// Integrate the Newton movement equations.
pub fn newton_step(time_step: f64, current_speed: f64, acceleration: f64, direction_sign: f64) -> IntegrationStep {
    let signed_time_step = time_step.abs().copysign(direction_sign);
    let mut new_speed = current_speed + acceleration * signed_time_step;
    if new_speed.abs() < SPEED_EPSILON {
        new_speed = 0.0;
    }
    // dx = currentSpeed * dt + 1/2 * acceleration * dt * dt
    let mut position_delta = current_speed * signed_time_step + 0.5 * acceleration * signed_time_step * signed_time_step;
    if position_delta.abs() < POSITION_EPSILON {
        position_delta = 0.0;
    }
    IntegrationStep::from_naive_step(time_step, position_delta, current_speed, new_speed, acceleration, direction_sign)
}
