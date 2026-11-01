// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope_sim/allowances/
// (Allowance.kt, AbstractAllowanceWithRanges.kt, LinearAllowance.kt,
// MarecoAllowance.kt) and envelope_utils/DoubleBinarySearch.java.

//! Allowances: time added to the fastest run, over ranges of the path —
//! spread by scaling the speeds (linear) or by capping the speed and coasting
//! where it saves the most energy (MARECO).

mod mareco;

use crate::SimError;
use crate::envelope::{Attrs, ConstrainedEnvelopePartBuilder, Constraint, ConstraintType, Envelope, EnvelopeBuilder, EnvelopePart, EnvelopeProfile, speed_cap};
use crate::physics::EnvelopeSimContext;
use crate::physics::overlays::{accelerate, decelerate};
use crate::units::{are_speeds_equal, are_times_equal, conditional_negate};

/// How much time an allowance adds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AllowanceValue {
    /// A fixed time, in seconds.
    FixedTime(f64),
    /// An added percentage of the running time.
    Percentage(f64),
    /// Added minutes per 100 km.
    TimePerDistance(f64),
}

impl AllowanceValue {
    /// Returns the allowance, given the total time and distance of the trip.
    pub fn allowance_time(&self, base_time: f64, distance: f64) -> f64 {
        match *self {
            AllowanceValue::FixedTime(time) => time,
            AllowanceValue::Percentage(percentage) => {
                debug_assert!(percentage >= 0.0);
                base_time * (percentage / 100.0)
            }
            AllowanceValue::TimePerDistance(time_per_distance) => {
                // number of portions of 100km in the train journey
                let n = distance / 100_000.0;
                time_per_distance * n * 60.0
            }
        }
    }

    /// Returns the share of the total allowance a given section gets.
    pub fn section_ratio(&self, section_time: f64, total_time: f64, section_distance: f64, total_distance: f64) -> f64 {
        match self {
            AllowanceValue::FixedTime(_) | AllowanceValue::Percentage(_) => section_time / total_time,
            AllowanceValue::TimePerDistance(_) => section_distance / total_distance,
        }
    }
}

/// A part of the path, between begin and end, with an allowance value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AllowanceRange {
    pub begin_pos: f64,
    pub end_pos: f64,
    pub value: AllowanceValue,
}

/// How an allowance spreads its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    /// All speeds scaled by the same ratio.
    Linear,
    /// A speed cap and coasting, saving the most energy (MARECO).
    Mareco,
}

/// An allowance over the region [begin_pos, end_pos] of an envelope, split
/// into ranges of their own values.
#[derive(Clone, Debug)]
pub struct Allowance {
    pub distribution: Distribution,
    pub begin_pos: f64,
    pub end_pos: f64,
    /// The speed under which the train would use too much capacity.
    pub capacity_speed_limit: f64,
    pub ranges: Vec<AllowanceRange>,
}

impl Allowance {
    pub fn new(distribution: Distribution, begin_pos: f64, end_pos: f64, capacity_speed_limit: f64, ranges: Vec<AllowanceRange>) -> Result<Self, SimError> {
        if ranges.iter().any(|r| r.begin_pos < begin_pos || r.end_pos > end_pos) {
            return Err(SimError::AllowanceRangeOutOfBounds);
        }
        debug_assert!(
            distribution != Distribution::Mareco || capacity_speed_limit >= 1.0,
            "capacity speed limit can't be lower than 1m/s for mareco allowances"
        );
        Ok(Allowance {
            distribution,
            begin_pos,
            end_pos,
            capacity_speed_limit,
            ranges,
        })
    }

    pub fn linear(begin_pos: f64, end_pos: f64, capacity_speed_limit: f64, ranges: Vec<AllowanceRange>) -> Result<Self, SimError> {
        Allowance::new(Distribution::Linear, begin_pos, end_pos, capacity_speed_limit, ranges)
    }

    pub fn mareco(begin_pos: f64, end_pos: f64, capacity_speed_limit: f64, ranges: Vec<AllowanceRange>) -> Result<Self, SimError> {
        Allowance::new(Distribution::Mareco, begin_pos, end_pos, capacity_speed_limit, ranges)
    }

    /// Get the total distance the allowance covers.
    pub fn distance(&self) -> f64 {
        self.end_pos - self.begin_pos
    }

    /// Get the total added time of the allowance region.
    pub fn added_time(&self, base: &Envelope) -> f64 {
        self.ranges
            .iter()
            .map(|range| {
                range
                    .value
                    .allowance_time(base.time_between(range.begin_pos, range.end_pos), range.end_pos - range.begin_pos)
            })
            .sum()
    }

    /// Get the total target time of the allowance region.
    pub fn target_time(&self, base: &Envelope) -> f64 {
        base.total_time() + self.added_time(base)
    }

    fn compute_core(&self, base: &Envelope, context: &EnvelopeSimContext, speed_cap: f64) -> Result<Envelope, SimError> {
        match self.distribution {
            Distribution::Linear => Ok(scale_envelope(base, speed_cap / base.max_speed())),
            Distribution::Mareco => mareco::compute_core(self.capacity_speed_limit, base, context, speed_cap),
        }
    }

    fn compute_initial_low_bound(&self) -> f64 {
        self.capacity_speed_limit
    }

    fn compute_initial_high_bound(&self, section: &Envelope, context: &EnvelopeSimContext) -> f64 {
        match self.distribution {
            Distribution::Linear => section.max_speed(),
            Distribution::Mareco => mareco::initial_high_bound(self.capacity_speed_limit, section, &context.rolling_stock),
        }
    }

    /// Apply the allowance to a given envelope.
    pub fn apply(&self, base: &Envelope, context: &EnvelopeSimContext) -> Result<Envelope, SimError> {
        if base.begin_pos() > self.begin_pos || base.end_pos() < self.end_pos {
            return Err(SimError::AllowanceOutOfBounds);
        }
        debug_assert!(base.continuous);
        // get only the region on which the allowance applies
        let region = Envelope::new(base.slice(self.begin_pos, self.end_pos));
        let mut allowance = self.clone();
        // slice parts that are not modified and run the allowance algorithm on the allowance region
        loop {
            match allowance.compute_allowance_region(&region, context) {
                Ok(allowance_region) => {
                    let mut builder = EnvelopeBuilder::new();
                    builder.add_parts(base.slice(f64::NEG_INFINITY, self.begin_pos));
                    for envelope in &allowance_region {
                        builder.add_envelope(envelope);
                    }
                    builder.add_parts(base.slice(self.end_pos, f64::INFINITY));
                    let res = builder.build();
                    debug_assert!(res.continuous, "Discontinuity on the edges of the allowance region");
                    return Ok(res);
                }
                Err(e @ (SimError::AllowanceTooMuchTime { .. } | SimError::AllowanceNotEnoughTime { .. })) => {
                    // The ranges are too short and the constraints too
                    // important: merge ranges together to find a realistic
                    // solution that follows most of the constraints
                    let mut range_index = e.range_index().unwrap_or(0);
                    if range_index + 1 >= allowance.ranges.len() {
                        if range_index > 0 {
                            range_index -= 1;
                        } else {
                            return Err(e);
                        }
                    }
                    allowance.merge_ranges_at_index(base, range_index);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Merge together the ranges at `range_index` and `range_index + 1`. The
    /// time over the two ranges is kept, but the transition time will not be enforced.
    fn merge_ranges_at_index(&mut self, base: &Envelope, range_index: usize) {
        let prev = self.ranges[range_index];
        let next = self.ranges[range_index + 1];
        let prev_time = prev
            .value
            .allowance_time(base.time_between(prev.begin_pos, prev.end_pos), prev.end_pos - prev.begin_pos);
        let next_time = next
            .value
            .allowance_time(base.time_between(next.begin_pos, next.end_pos), next.end_pos - next.begin_pos);
        self.ranges[range_index] = AllowanceRange {
            begin_pos: prev.begin_pos,
            end_pos: next.end_pos,
            value: AllowanceValue::FixedTime(prev_time + next_time),
        };
        self.ranges.remove(range_index + 1);
    }

    /// Apply the allowance to the region it affects. The region is split in
    /// ranges, computed one by one from the lowest to the highest base time;
    /// once a range is computed, its beginning and end speeds are imposed to
    /// the ranges on its left and right, which keeps the result continuous.
    fn compute_allowance_region(&self, region: &Envelope, context: &EnvelopeSimContext) -> Result<Vec<Envelope>, SimError> {
        let n = self.ranges.len();
        // the imposed speeds between ranges
        let mut imposed = vec![f64::NAN; n + 1];
        imposed[0] = region.begin_speed();
        imposed[n] = region.end_speed();
        // the speed at the border of ranges that don't add any time has to match the base envelope
        for (i, range) in self.ranges.iter().enumerate() {
            let added_time = range
                .value
                .allowance_time(region.time_between(range.begin_pos, range.end_pos), range.end_pos - range.begin_pos);
            if are_times_equal(0.0, added_time) {
                imposed[i] = region.interpolate_speed(range.begin_pos);
                imposed[i + 1] = region.interpolate_speed(range.end_pos);
            }
        }
        // ranges are computed with increasing base time values
        let base_times: Vec<f64> = self.ranges.iter().map(|r| region.time_between(r.begin_pos, r.end_pos)).collect();
        let mut range_order: Vec<usize> = (0..n).collect();
        range_order.sort_by(|a, b| base_times[*a].total_cmp(&base_times[*b]));
        let mut res: Vec<Option<Envelope>> = vec![None; n];
        for range_index in range_order {
            let range = self.ranges[range_index];
            let envelope_range = Envelope::new(region.slice(range.begin_pos, range.end_pos));
            let range_ratio = envelope_range.total_time() / region.total_time();
            let tolerance = context.time_step * range_ratio;
            let allowance_range = self
                .compute_allowance_range(&envelope_range, context, range.value, imposed[range_index], imposed[range_index + 1], tolerance)
                .map_err(|e| e.with_range_index(range_index))?;
            imposed[range_index] = allowance_range.begin_speed();
            imposed[range_index + 1] = allowance_range.end_speed();
            res[range_index] = Some(allowance_range);
        }
        Ok(res.into_iter().map(|e| e.unwrap()).collect())
    }

    /// Apply the allowance to the given range, split into sections between
    /// stops, computed independently.
    fn compute_allowance_range(
        &self,
        envelope_range: &Envelope,
        context: &EnvelopeSimContext,
        value: AllowanceValue,
        imposed_range_begin_speed: f64,
        imposed_range_end_speed: f64,
        tolerance: f64,
    ) -> Result<Envelope, SimError> {
        let base_time = envelope_range.total_time();
        let base_distance = envelope_range.total_distance();
        let added_time = value.allowance_time(base_time, base_distance);
        // if no time is added, just return the base envelope without performing binary search
        if are_times_equal(0.0, added_time) {
            return Ok(envelope_range.clone());
        }
        debug_assert!(added_time > 0.0, "Adding negative time from allowance {value:?} ({added_time} seconds)");

        // compute the slowest running time, given the capacity speed limit,
        // to make sure the user asked for a margin that is actually possible
        let total_target_time = base_time + added_time;
        let mut slowest_running_time = f64::INFINITY;
        if self.capacity_speed_limit > 0.0 {
            let attrs = Attrs {
                capacity_speed_limit: true,
                ..Attrs::default()
            };
            slowest_running_time = speed_cap(envelope_range, &attrs, self.capacity_speed_limit).total_time();
        }
        if total_target_time > slowest_running_time {
            return Err(SimError::AllowanceTooMuchTime { range_index: None });
        }

        let range_begin_pos = envelope_range.begin_pos();
        let range_end_pos = envelope_range.end_pos();
        // the points between which the computation is divided: each division is a section
        let mut split_points = vec![range_begin_pos];
        split_points.extend(envelope_range.iter().filter(|p| p.end_speed() == 0.0).map(|p| p.end_pos()));
        if *split_points.last().unwrap() != range_end_pos {
            split_points.push(range_end_pos);
        }
        let mut builder = EnvelopeBuilder::new();
        for w in split_points.windows(2) {
            let (section_begin_pos, section_end_pos) = (w[0], w[1]);
            let section = Envelope::new(envelope_range.slice(section_begin_pos, section_end_pos));
            let section_time = section.total_time();
            let section_distance = section.total_distance();
            let section_ratio = value.section_ratio(section_time, base_time, section_distance, base_distance);
            let target_time = section_time + added_time * section_ratio;
            // the imposed begin and end speeds only apply to the first and
            // last section of the range respectively
            let imposed_begin_speed = if section_begin_pos == range_begin_pos {
                imposed_range_begin_speed
            } else {
                f64::NAN
            };
            let imposed_end_speed = if section_end_pos == range_end_pos {
                imposed_range_end_speed
            } else {
                f64::NAN
            };
            let distributed_tolerance = tolerance * section_ratio;
            let allowance_section =
                self.compute_allowance_section(&section, context, target_time, imposed_begin_speed, imposed_end_speed, distributed_tolerance)?;
            debug_assert!((allowance_section.total_time() - target_time).abs() <= context.time_step);
            builder.add_envelope(&allowance_section);
        }
        Ok(builder.build())
    }

    /// Iteratively apply the allowance on the given section, until the target time is reached.
    fn compute_allowance_section(
        &self,
        section: &Envelope,
        context: &EnvelopeSimContext,
        target_time: f64,
        imposed_begin_speed: f64,
        imposed_end_speed: f64,
        tolerance: f64,
    ) -> Result<Envelope, SimError> {
        let initial_low_bound = self.compute_initial_low_bound();
        let initial_high_bound = self.compute_initial_high_bound(section, context);
        if initial_low_bound > initial_high_bound {
            // This can happen when capacity speed limit > max speed. We know
            // in advance no solution can be found.
            return Err(SimError::AllowanceTooMuchTime { range_index: None });
        }
        let mut res: Option<Envelope> = None;
        let mut last_error: Option<SimError> = None;
        let mut search = DoubleBinarySearch::new(initial_low_bound, initial_high_bound, target_time, tolerance, true);
        let mut last_time = 0.0;
        let mut i = 1;
        while i < 30 && !search.complete() {
            let input = search.input();
            match self.compute_iteration(section, context, input, imposed_begin_speed, imposed_end_speed) {
                Ok(envelope) => {
                    last_time = envelope.total_time();
                    search.feedback(last_time);
                    res = Some(envelope);
                }
                // can't go slow enough to even build a valid envelope: go faster
                Err(e @ SimError::AllowanceTooMuchTime { .. }) => {
                    last_error = Some(e);
                    search.feedback(f64::INFINITY);
                }
                // can't go fast enough to even build a valid envelope: go slower
                Err(e @ SimError::AllowanceNotEnoughTime { .. }) => {
                    last_error = Some(e);
                    search.feedback(0.0);
                }
                Err(e) => return Err(e),
            }
            i += 1;
        }
        if !search.complete() {
            match res {
                // We couldn't match the distributed tolerance, but we're still
                // within one time step: the closest result is used
                Some(envelope) if (last_time - target_time).abs() <= context.time_step => return Ok(envelope),
                _ => {
                    if let Some(e) = last_error {
                        return Err(e);
                    }
                    return Err(if !search.has_raised_low_bound() {
                        SimError::AllowanceTooMuchTime { range_index: None }
                    } else if !search.has_lowered_high_bound() {
                        SimError::AllowanceNotEnoughTime { range_index: None }
                    } else {
                        SimError::AllowanceDiscontinuity
                    });
                }
            }
        }
        Ok(res.expect("a complete search has a result"))
    }

    /// Compute one iteration of the binary search, with specified speeds on the edges.
    fn compute_iteration(
        &self,
        base: &Envelope,
        context: &EnvelopeSimContext,
        input: f64,
        imposed_begin_speed: f64,
        imposed_end_speed: f64,
    ) -> Result<Envelope, SimError> {
        // The part of the envelope on which the margin is applied is split in
        // 3: left junction, then core phase, then right junction. The junction
        // parts keep the total envelope continuous when beginning or end
        // speeds are imposed.
        let core_envelope = self.compute_core(base, context, input)?;
        // 1) compute the potential junction parts (slowdown or speedup)
        let left_part = compute_left_junction(base, &core_envelope, context, imposed_begin_speed)?;
        let left_part_end_pos = left_part.as_ref().map_or(base.begin_pos(), |p| p.end_pos());
        let core_envelope_with_left = compute_envelope_with_left_junction(base, &core_envelope, left_part.as_ref());
        let right_part = compute_right_junction(base, &core_envelope_with_left, context, imposed_end_speed)?;
        let right_part_begin_pos = right_part.as_ref().map_or(base.end_pos(), |p| p.begin_pos());
        if right_part_begin_pos <= left_part_end_pos {
            // if the junction parts touch or intersect, there is no core phase
            return intersect_left_right_parts(left_part, right_part);
        }
        // 2) stick phases back together
        let mut builder = EnvelopeBuilder::new();
        let mut left_part_end_speed = f64::NAN;
        let mut right_part_begin_speed = f64::NAN;
        if let Some(left) = &left_part {
            left_part_end_speed = left.end_speed();
            debug_assert!(are_speeds_equal(core_envelope.interpolate_speed(left_part_end_pos), left_part_end_speed));
            builder.add_part(left.clone());
        }
        if let Some(right) = &right_part {
            right_part_begin_speed = right.begin_speed();
            debug_assert!(are_speeds_equal(core_envelope.interpolate_speed(right_part_begin_pos), right_part_begin_speed));
        }
        // the left part end speed and right part begin speed are forced, to
        // avoid epsilon differences that would cause errors later on
        builder.add_parts(core_envelope.slice_with_speeds(left_part_end_pos, left_part_end_speed, right_part_begin_pos, right_part_begin_speed));
        if let Some(right) = right_part {
            builder.add_part(right);
        }
        let result = builder.build();
        // 3) check for continuity of the section
        debug_assert!(result.continuous, "Discontinuity in allowance section");
        Ok(result)
    }
}

/// Compute the left junction of the section if a beginning speed is
/// imposed: a slow-down or a speed-up, depending on the imposed begin speed
/// and the target envelope.
fn compute_left_junction(
    section: &Envelope,
    target: &Envelope,
    context: &EnvelopeSimContext,
    imposed_begin_speed: f64,
) -> Result<Option<EnvelopePart>, SimError> {
    // if there is no imposed begin speed, no junction needs to be computed
    if imposed_begin_speed.is_nan() {
        return Ok(None);
    }
    let mut constraints = vec![Constraint::position(section.begin_pos(), section.end_pos())];
    let mut last_intersection = -1;
    let mut part = None;
    // if the imposed speed is above the target, compute slowdown, else, compute speedup
    if imposed_begin_speed > target.begin_speed() {
        constraints.push(Constraint::envelope(target, ConstraintType::Floor));
        let mut constrained = ConstrainedEnvelopePartBuilder::new(Default::default(), constraints);
        decelerate(context, section.begin_pos(), imposed_begin_speed, &mut constrained, 1.0)?;
        constrained.sink.set_profile(EnvelopeProfile::Braking);
        last_intersection = constrained.last_intersection;
        part = Some(constrained.sink);
    } else if imposed_begin_speed < section.begin_speed() {
        constraints.push(Constraint::envelope(target, ConstraintType::Ceiling));
        constraints.push(Constraint::envelope(section, ConstraintType::Ceiling));
        constraints.push(Constraint::speed(0.0, ConstraintType::Floor));
        let mut constrained = ConstrainedEnvelopePartBuilder::new(Default::default(), constraints);
        accelerate(context, section.begin_pos(), imposed_begin_speed, &mut constrained, 1.0)?;
        constrained.sink.set_profile(EnvelopeProfile::Accelerating);
        last_intersection = constrained.last_intersection;
        part = Some(constrained.sink);
    }
    if last_intersection == 0 {
        // The end of the part has been reached without crossing the target
        // envelope: the allowance is too restrictive
        return Err(SimError::AllowanceTooMuchTime { range_index: None });
    }
    Ok(part.filter(|p| !p.is_empty()).map(|p| p.build()))
}

/// Compute the right junction of the section if an end speed is imposed: a
/// speed-up or a slow-down, depending on the imposed end speed and the
/// target envelope.
fn compute_right_junction(
    section: &Envelope,
    target: &Envelope,
    context: &EnvelopeSimContext,
    imposed_end_speed: f64,
) -> Result<Option<EnvelopePart>, SimError> {
    if imposed_end_speed.is_nan() {
        return Ok(None);
    }
    let mut constraints = vec![Constraint::position(section.begin_pos(), section.end_pos())];
    let mut last_intersection = -1;
    let mut part = None;
    // if the imposed speed is above the target compute speed-up, else, compute slow-down
    if imposed_end_speed > target.end_speed() {
        constraints.push(Constraint::envelope(target, ConstraintType::Floor));
        let mut constrained = ConstrainedEnvelopePartBuilder::new(Default::default(), constraints);
        accelerate(context, section.end_pos(), imposed_end_speed, &mut constrained, -1.0)?;
        constrained.sink.set_profile(EnvelopeProfile::Accelerating);
        last_intersection = constrained.last_intersection;
        part = Some(constrained.sink);
    } else if imposed_end_speed < section.end_speed() {
        constraints.push(Constraint::speed(0.0, ConstraintType::Floor));
        constraints.push(Constraint::envelope(target, ConstraintType::Ceiling));
        let mut constrained = ConstrainedEnvelopePartBuilder::new(Default::default(), constraints);
        decelerate(context, section.end_pos(), imposed_end_speed, &mut constrained, -1.0)?;
        constrained.sink.set_profile(EnvelopeProfile::Braking);
        last_intersection = constrained.last_intersection;
        part = Some(constrained.sink);
    }
    if last_intersection == 0 {
        return Err(SimError::AllowanceTooMuchTime { range_index: None });
    }
    Ok(part.filter(|p| !p.is_empty()).map(|p| p.build()))
}

/// Turn the left junction into an envelope that spans from the beginning to
/// the end of the section, filling the gap with the core envelope.
fn compute_envelope_with_left_junction(section: &Envelope, core_envelope: &Envelope, left_junction: Option<&EnvelopePart>) -> Envelope {
    let Some(left) = left_junction else { return core_envelope.clone() };
    let mut builder = EnvelopeBuilder::new();
    builder.add_part(left.clone());
    if left.end_pos() < section.end_pos() {
        builder.add_parts(core_envelope.slice(left.end_pos(), f64::INFINITY));
    }
    builder.build()
}

/// If the left and right part intersect, build an envelope with the intersection.
fn intersect_left_right_parts(left_part: Option<EnvelopePart>, right_part: Option<EnvelopePart>) -> Result<Envelope, SimError> {
    let (Some(left), Some(right)) = (left_part, right_part) else {
        return Err(SimError::AllowanceTooMuchTime { range_index: None });
    };
    match left.slice_with_speeds(f64::NEG_INFINITY, f64::NAN, right.begin_pos(), right.begin_speed()) {
        Some(sliced) if sliced.end_pos() == right.begin_pos() => Ok(Envelope::new(vec![sliced, right])),
        // The curves don't intersect at all. This sometimes happens when one
        // part is very short compared to the time step: there is very little
        // margin to add time
        _ => Err(SimError::AllowanceTooMuchTime { range_index: None }),
    }
}

/// Scale an envelope: new speed = old speed * ratio.
pub fn scale_envelope(envelope: &Envelope, ratio: f64) -> Envelope {
    let mut builder = EnvelopeBuilder::new();
    for part in envelope.iter() {
        let scaled_speeds = part.speeds().iter().map(|s| s * ratio).collect();
        let attrs = Attrs {
            profile: part.profile(),
            ..Attrs::default()
        };
        builder.add_part(EnvelopePart::generate_times(attrs, part.positions().to_vec(), scaled_speeds));
    }
    builder.build()
}

/// A binary search over a monotonic function, to a target output.
pub struct DoubleBinarySearch {
    low_bound: f64,
    high_bound: f64,
    input: f64,
    target: f64,
    target_error_margin: f64,
    /// Positive if increasing, negative if decreasing.
    direction: f64,
    is_complete: bool,
    has_lowered_high_bound: bool,
    has_raised_low_bound: bool,
}

impl DoubleBinarySearch {
    pub fn new(low_bound: f64, high_bound: f64, target: f64, target_error_margin: f64, decreasing: bool) -> Self {
        DoubleBinarySearch {
            low_bound,
            high_bound,
            input: (low_bound + high_bound) / 2.0,
            target,
            target_error_margin,
            direction: if decreasing { -1.0 } else { 1.0 },
            is_complete: false,
            has_lowered_high_bound: false,
            has_raised_low_bound: false,
        }
    }

    pub fn complete(&self) -> bool {
        self.is_complete
    }

    pub fn input(&self) -> f64 {
        debug_assert!(!self.is_complete);
        self.input
    }

    pub fn result(&self) -> f64 {
        debug_assert!(self.is_complete);
        self.input
    }

    /// Feeds back the output of the tested function for the current input.
    pub fn feedback(&mut self, output: f64) {
        debug_assert!(!self.is_complete);
        let delta = output - self.target;
        if delta.abs() <= self.target_error_margin {
            self.is_complete = true;
            return;
        }
        if conditional_negate(delta, self.direction) < 0.0 {
            self.low_bound = self.input;
            self.has_raised_low_bound = true;
        } else {
            self.high_bound = self.input;
            self.has_lowered_high_bound = true;
        }
        self.input = (self.low_bound + self.high_bound) / 2.0;
    }

    pub fn has_lowered_high_bound(&self) -> bool {
        self.has_lowered_high_bound
    }

    pub fn has_raised_low_bound(&self) -> bool {
        self.has_raised_low_bound
    }
}
