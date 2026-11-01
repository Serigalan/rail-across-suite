// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope_sim/allowances/MarecoAllowance.kt
// and mareco_impl/ (AcceleratingSlopeCoast, BrakingPhaseCoast,
// CoastingGenerator, CoastingOpportunity), envelope_utils/DistanceAverage.java.

//! MARECO: the allowance that saves the most energy. The speed is capped
//! at v1, and the train coasts down to vf before it brakes and before
//! accelerating slopes. The formulas are those of the MARECO paper
//! (https://osrd.fr/pdf/MARECO.pdf).

use crate::SimError;
use crate::envelope::{
    Attrs, ConstrainedEnvelopePartBuilder, Constraint, ConstraintType, Envelope, EnvelopeCursor, EnvelopePart, EnvelopeProfile, OverlayEnvelopeBuilder,
    speed_cap,
};
use crate::physics::integrator::{Action, step};
use crate::physics::overlays::coast;
use crate::physics::{EnvelopeSimContext, RollingStock};
use crate::pipelines::{add_acceleration_and_constant_speed_parts, max_effort_plateau};
use crate::units::{are_positions_equal, are_speeds_equal};

/// Given a ceiling speed v1, compute vf, the speed at which the train
/// should end coasting and start braking.
fn compute_vf(capacity_speed_limit: f64, v1: f64, rolling_stock: &RollingStock) -> f64 {
    let wle = v1 * v1 * rolling_stock.rolling_resistance_deriv(v1);
    let vf = wle * v1 / (wle + rolling_stock.rolling_resistance(v1) * v1);
    // Prevents coasting from starting below capacity speed limit
    vf.max(capacity_speed_limit)
}

/// The initial high bound for the binary search, which ensures that the
/// speed vf will be higher than the max speed of the envelope.
pub(super) fn initial_high_bound(capacity_speed_limit: f64, section: &Envelope, rolling_stock: &RollingStock) -> f64 {
    let section_max_speed = section.max_speed();
    let mut max_speed = section_max_speed;
    let mut vf = compute_vf(capacity_speed_limit, max_speed, rolling_stock);
    while vf < section_max_speed {
        max_speed *= 2.0;
        vf = compute_vf(capacity_speed_limit, max_speed, rolling_stock);
    }
    max_speed
}

/// The core of MARECO: a speed cap at v1, and coasting opportunities before
/// braking and before accelerating slopes.
pub(super) fn compute_core(capacity_speed_limit: f64, base: &Envelope, context: &EnvelopeSimContext, v1: f64) -> Result<Envelope, SimError> {
    let vf = compute_vf(capacity_speed_limit, v1, &context.rolling_stock);

    // 1) cap the core base envelope at v1 and check if v1 is physically reachable
    let capped = speed_cap(
        base,
        &Attrs {
            mareco_speed_limit: true,
            ..Attrs::default()
        },
        v1,
    );
    let initial_position = capped.begin_pos();
    let initial_speed = capped.begin_speed();
    let capped = add_acceleration_and_constant_speed_parts(context, &capped, initial_position, initial_speed)?;

    // 2) find accelerating slopes on constant speed limit regions
    let mut opportunities: Vec<CoastingOpportunity> = AcceleratingSlopeCoast::find_all(&capped, context, vf)
        .into_iter()
        .map(CoastingOpportunity::Slope)
        .collect();
    // 3) find coasting opportunities related to braking
    opportunities.extend(braking_phase_coasts(&capped, v1, vf).into_iter().map(CoastingOpportunity::Braking));

    // 4) evaluate coasting opportunities in reverse order, thus skipping overlapping ones
    opportunities.sort_by(|a, b| a.end_position().total_cmp(&b.end_position()));
    opportunities.reverse();
    let mut builder = OverlayEnvelopeBuilder::backward(&capped);
    let mut last_coast_begin = f64::INFINITY;
    for opportunity in &opportunities {
        if last_coast_begin < opportunity.end_position() {
            continue;
        }
        let Some(overlay) = opportunity.compute(&capped, context, v1, vf)? else {
            continue;
        };
        last_coast_begin = overlay.begin_pos();
        builder.add_part(overlay);
    }
    let res = builder.build();
    debug_assert!(res.continuous, "Discontinuity in MARECO core phase");
    Ok(res)
}

enum CoastingOpportunity {
    Slope(AcceleratingSlopeCoast),
    /// A braking phase, coasting before the position it ends at.
    Braking(f64),
}

impl CoastingOpportunity {
    /// The location at which coasting shall end.
    fn end_position(&self) -> f64 {
        match self {
            CoastingOpportunity::Slope(slope) => slope.end_position,
            CoastingOpportunity::Braking(end_position) => *end_position,
        }
    }

    fn compute(&self, base: &Envelope, context: &EnvelopeSimContext, v1: f64, vf: f64) -> Result<Option<EnvelopePart>, SimError> {
        match self {
            CoastingOpportunity::Slope(slope) => slope.compute(base, context, v1, vf),
            // coast backwards from the last point of braking phases above vf,
            // never below vf, until an intersection with the base is found
            CoastingOpportunity::Braking(end_position) => coast_from_end(base, context, *end_position, vf),
        }
    }
}

/// Finds all coasting opportunities caused by braking phases.
fn braking_phase_coasts(envelope: &Envelope, v1: f64, vf: f64) -> Vec<f64> {
    let mut res = Vec::new();
    for part in envelope.iter() {
        if !part.has_profile(EnvelopeProfile::Braking) {
            continue;
        }
        let target_speed = part.end_speed();
        // a braking phase towards a speed above v1 has no impact here
        if target_speed > v1 {
            continue;
        }
        // deceleration phases that are entirely above vf
        if target_speed > vf {
            res.push(part.end_pos());
            continue;
        }
        // deceleration phases that cross vf
        if part.max_speed() > vf {
            res.push(part.interpolate_position(vf));
        }
    }
    res
}

/// Average values over distances, adding segments iteratively.
#[derive(Default)]
struct DistanceAverage {
    total_weighted_value: f64,
    total_length: f64,
}

impl DistanceAverage {
    fn add_segment(&mut self, length: f64, value: f64) {
        self.total_weighted_value += value * length;
        self.total_length += length;
    }

    fn average(&self) -> f64 {
        self.total_weighted_value / self.total_length
    }
}

/// An accelerating slope on a plateau.
struct AcceleratingSlopeCoast {
    /// Position where the train starts accelerating again until it reaches the max speed.
    acceleration_start_position: f64,
    /// The start and end speed limit of the accelerating slope.
    speed_limit: f64,
    /// Position where the coasting envelope should merge back into the base envelope.
    end_position: f64,
    /// Average acceleration of the train during the slope.
    slope_average_acceleration: f64,
    /// An estimate of the average acceleration of the train from the start
    /// of coasting to the beginning of the accelerating slope.
    previous_acceleration_estimate: f64,
    mean_acceleration_builder: DistanceAverage,
}

impl AcceleratingSlopeCoast {
    fn new(acceleration_start_position: f64, speed_limit: f64) -> Self {
        AcceleratingSlopeCoast {
            acceleration_start_position,
            speed_limit,
            end_position: f64::NAN,
            slope_average_acceleration: f64::NAN,
            previous_acceleration_estimate: f64::NAN,
            mean_acceleration_builder: DistanceAverage::default(),
        }
    }

    /// Finish building the slope, once its end position is known.
    fn build(mut self, end_pos: f64, context: &EnvelopeSimContext) -> Self {
        self.slope_average_acceleration = self.mean_acceleration_builder.average();
        self.previous_acceleration_estimate = self.estimate_previous_acceleration(context);
        self.end_position = end_pos;
        debug_assert!(!self.slope_average_acceleration.is_nan());
        debug_assert!(!end_pos.is_nan());
        debug_assert!(!self.previous_acceleration_estimate.is_nan());
        self
    }

    /// Estimates the average acceleration during the part where the train decelerates.
    fn estimate_previous_acceleration(&self, context: &EnvelopeSimContext) -> f64 {
        // the natural acceleration an offset before the start of the slope:
        // the train's length, so that its head isn't on the slope
        let acceleration_start = self.find_exact_start_accelerating_slope(context);
        let offset = context.rolling_stock.length;
        let estimate_position = (acceleration_start - offset).max(0.0);
        natural_acceleration(context, estimate_position, self.speed_limit).min(0.0)
    }

    /// The exact position where the natural acceleration sign goes from negative to positive.
    fn find_exact_start_accelerating_slope(&self, context: &EnvelopeSimContext) -> f64 {
        // start from the first position with a positive acceleration, and go
        // back until it's negative
        let position_step = 10.0;
        let mut position = self.acceleration_start_position;
        while position > 0.0 && natural_acceleration(context, position, self.speed_limit) > 0.0 {
            position -= position_step;
        }
        if position <= 0.0 {
            return 0.0; // The path is on a negative slope from its start
        }
        let acceleration_prev_step = natural_acceleration(context, position, self.speed_limit);
        let acceleration_next_step = natural_acceleration(context, position + position_step, self.speed_limit);
        debug_assert!(acceleration_prev_step <= 0.0);
        debug_assert!(acceleration_next_step >= 0.0);
        interpolate_acceleration_sign_change(acceleration_next_step, acceleration_prev_step, position + position_step, position)
    }

    /// The optimized speed v the train should have when entering the accelerating slope.
    fn compute_v(&self, rolling_stock: &RollingStock, v1: f64, vf: f64) -> f64 {
        let wle = rolling_stock.rolling_resistance(v1) * v1 * vf / (v1 - vf);
        let accel_ratio = self.previous_acceleration_estimate / self.slope_average_acceleration;
        debug_assert!(accel_ratio <= 0.0);
        1.0 / (1.0 / self.speed_limit + rolling_stock.rolling_resistance(self.speed_limit) / (wle * (1.0 - accel_ratio)))
    }

    fn compute(&self, base: &Envelope, context: &EnvelopeSimContext, v1: f64, vf: f64) -> Result<Option<EnvelopePart>, SimError> {
        // coast backwards from the end of accelerating slope, limiting the
        // minimum speed to v, then forward from the point reached
        let v = self.compute_v(&context.rolling_stock, v1, vf);
        let min_coasting_speed = v.max(vf); // We don't want to coast below vf nor v
        coast_from_end(base, context, self.end_position, min_coasting_speed)
    }

    /// Finds all the opportunities for coasting on accelerating slopes.
    fn find_all(envelope: &Envelope, context: &EnvelopeSimContext, vf: f64) -> Vec<AcceleratingSlopeCoast> {
        let mut res = Vec::new();
        let mut cursor = EnvelopeCursor::forward(envelope);
        // scan until maintain speed envelope parts
        while cursor.find_part(max_effort_plateau) {
            let speed = cursor.step_begin_speed();
            // no coasting will be triggered if the speed is below vf
            if speed <= vf {
                cursor.next_part();
                continue;
            }
            let mut previous_position = f64::NAN;
            let mut previous_acceleration = f64::NAN;
            let mut current: Option<AcceleratingSlopeCoast> = None;
            let envelope_part = cursor.part();
            let position_step = context.time_step * speed;
            while !cursor.has_reached_end() && cursor.position() <= envelope_part.end_pos() {
                let position = cursor.position();
                let acceleration = natural_acceleration(context, position, speed);
                if acceleration > 0.0 {
                    // Accelerating slope
                    current
                        .get_or_insert_with(|| AcceleratingSlopeCoast::new(position, speed))
                        .mean_acceleration_builder
                        .add_segment(position_step, acceleration);
                } else if let Some(slope) = current.take() {
                    // end the accelerating slope
                    let end_pos = interpolate_acceleration_sign_change(acceleration, previous_acceleration, position, previous_position);
                    res.push(slope.build(end_pos, context));
                }
                previous_acceleration = acceleration;
                previous_position = position;
                cursor.find_position(position + position_step);
            }
            // if the end of the plateau is an accelerating slope
            if let Some(slope) = current {
                res.push(slope.build(previous_position, context));
            }
        }
        res
    }
}

/// The acceleration at the given position if the train coasts.
fn natural_acceleration(context: &EnvelopeSimContext, position: f64, speed: f64) -> f64 {
    step(context, position, speed, Action::Coast, 1.0).acceleration
}

/// Interpolate the exact position where the natural acceleration is 0.
fn interpolate_acceleration_sign_change(current_acceleration: f64, previous_acceleration: f64, current_position: f64, previous_position: f64) -> f64 {
    debug_assert!(!previous_acceleration.is_nan());
    if are_positions_equal(current_position, previous_position) {
        return current_position;
    }
    if are_speeds_equal(current_acceleration, previous_acceleration) {
        return current_position;
    }
    let factor = (previous_acceleration - current_acceleration) / (previous_position - current_position);
    let y0 = previous_acceleration - factor * previous_position;
    let res = -y0 / factor;
    debug_assert!(res >= previous_position && res <= current_position);
    res
}

/// Generate a coasting envelope part which starts at startPos.
fn coast_from_beginning(envelope: &Envelope, context: &EnvelopeSimContext, start_pos: f64, start_speed: f64) -> Result<Option<EnvelopePart>, SimError> {
    let mut constrained = ConstrainedEnvelopePartBuilder::with_profile(
        EnvelopeProfile::Coasting,
        vec![
            Constraint::speed(0.0, ConstraintType::Floor),
            Constraint::envelope(envelope, ConstraintType::Ceiling),
        ],
    );
    coast(context, start_pos, start_speed, &mut constrained, 1.0)?;
    if constrained.last_intersection == 0 {
        // We reached a stop while coasting: the coasting envelope did not
        // intersect the base envelope. Returning nothing keeps the binary
        // search going.
        return Ok(None);
    }
    if constrained.sink.is_empty() {
        return Ok(None);
    }
    Ok(Some(constrained.sink.build()))
}

/// Generate a coasting envelope part which ends at endPos and never goes below lowSpeedLimit.
fn coast_from_end(envelope: &Envelope, context: &EnvelopeSimContext, end_pos: f64, low_speed_limit: f64) -> Result<Option<EnvelopePart>, SimError> {
    debug_assert!(end_pos >= 0.0 && end_pos <= context.path.length);
    // coast backwards from the end position until the base curve is met
    let mut constrained = ConstrainedEnvelopePartBuilder::with_profile(
        EnvelopeProfile::Coasting,
        vec![
            Constraint::speed(0.0, ConstraintType::Floor),
            Constraint::envelope(envelope, ConstraintType::Ceiling),
        ],
    );
    let mut position = end_pos;
    let mut speed = envelope.interpolate_speed(position);
    debug_assert!(
        speed >= low_speed_limit || are_speeds_equal(speed, low_speed_limit),
        "start coasting below min speed"
    );
    let init_inter = constrained.init_envelope_part(position, speed, -1.0);
    debug_assert!(init_inter);
    let mut reached_low_limit = false;
    for _ in 0..10_000_000 {
        let step = step(context, position, speed, Action::Coast, -1.0);
        position += step.position_delta;
        speed = step.end_speed;
        if !are_speeds_equal(speed, low_speed_limit) && speed < low_speed_limit {
            speed = low_speed_limit;
            reached_low_limit = true;
        }
        if !constrained.add_step_with_time(position, speed, step.time_delta) {
            break;
        }
    }
    if constrained.sink.is_empty() {
        return Ok(None);
    }
    debug_assert!(constrained.last_pos() < end_pos);
    if !reached_low_limit {
        return Ok(Some(constrained.sink.build()));
    }
    // a coasting going forward is only needed if the low speed limit has
    // been reached, to add accelerations where the speed was clipped
    let Some(result_coast) = coast_from_beginning(envelope, context, constrained.last_pos(), constrained.last_speed())? else {
        return Ok(None);
    };
    if result_coast.end_pos() > end_pos + context.time_step * speed {
        // The coasting envelope didn't intersect with the base envelope,
        // which can happen if it should have intersected in the middle of a
        // simulation step
        return Ok(None);
    }
    Ok(Some(result_coast))
}
