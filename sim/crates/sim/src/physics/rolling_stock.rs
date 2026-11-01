// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/osrd-mp/.../envelope_sim/PhysicsRollingStock.kt and
// core/src/.../train/RollingStock.kt, for one tractive effort curve over the
// whole path (no electrification modes, no ETCS braking).

use serde::{Deserialize, Serialize};

/// The effort the train can apply at a given speed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TractiveEffortPoint {
    /// in m/s
    pub speed: f64,
    /// in N
    pub max_effort: f64,
}

/// The physical model of a train.
#[derive(Clone, Debug)]
pub struct RollingStock {
    /// The mass of the train, in kilograms.
    pub mass: f64,
    /// The inertia of the train, in kilograms (mass times its inertia coefficient).
    pub inertia: f64,
    /// The length of the train, in meters.
    pub length: f64,
    /// The maximum speed the train can reach, in m/s.
    pub max_speed: f64,
    /// Davis coefficients of the resistance to movement: N, N/(m/s), N/(m/s)².
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// The constant deceleration of the train, in m/s² (negative).
    pub deceleration: f64,
    /// Its tractive effort curve, by increasing speed.
    pub tractive_effort_curve: Vec<TractiveEffortPoint>,
}

/// Rotating masses after ERA's braking curves simulation tool v5.1, in %.
const M_ROTATING_MIN: f64 = 2.0;
const M_ROTATING_MAX: f64 = 15.0;

impl RollingStock {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mass: f64,
        inertia_coefficient: f64,
        length: f64,
        max_speed: f64,
        a: f64,
        b: f64,
        c: f64,
        const_gamma: f64,
        tractive_effort_curve: Vec<TractiveEffortPoint>,
    ) -> Self {
        RollingStock {
            mass,
            inertia: mass * inertia_coefficient,
            length,
            max_speed,
            a,
            b,
            c,
            deceleration: -const_gamma,
            tractive_effort_curve,
        }
    }

    /// The resistance to movement at a given speed, in newtons: the Davis
    /// equation, empirical, modelling the drag and friction forces.
    pub fn rolling_resistance(&self, speed: f64) -> f64 {
        let speed = speed.abs();
        self.a + self.b * speed + self.c * speed * speed
    }

    /// The first derivative of the resistance to movement at a given speed, in kg/s.
    pub fn rolling_resistance_deriv(&self, speed: f64) -> f64 {
        let speed = speed.abs();
        self.b + 2.0 * self.c * speed
    }
}

/// Get the effort the train can apply at a given speed, in newtons.
pub fn max_effort(speed: f64, curve: &[TractiveEffortPoint]) -> f64 {
    let mut index: usize = 0;
    let mut left: isize = 0;
    let mut right: isize = curve.len() as isize - 1;
    while left <= right {
        let mid = ((left + right) as usize) >> 1;
        if (curve[mid].speed - speed.abs()).abs() < 0.000001 {
            index = mid;
            break;
        } else if curve[mid].speed < speed.abs() {
            left = mid as isize + 1;
            index = left as usize;
        } else {
            right = mid as isize - 1;
        }
    }
    if index == 0 {
        return curve[0].max_effort;
    }
    if index == curve.len() {
        return curve[index - 1].max_effort;
    }
    let previous_point = curve[index - 1];
    let next_point = curve[index];
    let coeff = (previous_point.max_effort - next_point.max_effort) / (previous_point.speed - next_point.speed);
    previous_point.max_effort + coeff * (speed.abs() - previous_point.speed)
}

/// The gradient acceleration of the rolling stock taking its rotating mass
/// into account, in m/s². Grade is in m/km.
pub fn gradient_acceleration(grade: f64) -> f64 {
    let m_rotating = if grade >= 0.0 { M_ROTATING_MAX } else { M_ROTATING_MIN };
    -super::integrator::GRAVITY_ACCELERATION * grade / (1000.0 + 10.0 * m_rotating)
}
