// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope/part/EnvelopePart.kt and
// envelope_sim/EnvelopeProfile.java. OSRD keys an envelope part's attributes
// by their class; here they are the fields of `Attrs`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::physics::{interpolate_step_speed, interpolate_step_time_at, intersect_step_with_speed, intersect_steps};
use crate::units::{SearchableEnvelope, TIME_EPSILON, are_positions_equal, binary_search};

/// The shape of an envelope part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeProfile {
    Accelerating,
    ConstantSpeed,
    CatchingUp,
    Coasting,
    Braking,
}

/// What a part of the most restrictive speed profile comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    SpeedLimit,
    TrainLimit,
    SafetyApproachSpeed,
}

/// The metadata of an envelope part — one value per kind, as OSRD's
/// attribute map has one per class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attrs {
    pub profile: Option<EnvelopeProfile>,
    pub limit: Option<LimitKind>,
    /// The index of the speed limit's source, as the caller numbers them.
    pub source: Option<u32>,
    /// A part capped by an allowance's capacity speed limit.
    pub capacity_speed_limit: bool,
    /// A part capped by MARECO's speed.
    pub mareco_speed_limit: bool,
}

impl Attrs {
    pub fn profile(profile: EnvelopeProfile) -> Self {
        Attrs {
            profile: Some(profile),
            ..Attrs::default()
        }
    }

    /// Sets every attribute `other` has, as OSRD's `setAttrs`.
    pub fn merge(&mut self, other: &Attrs) {
        if other.profile.is_some() {
            self.profile = other.profile;
        }
        if other.limit.is_some() {
            self.limit = other.limit;
        }
        if other.source.is_some() {
            self.source = other.source;
        }
        self.capacity_speed_limit |= other.capacity_speed_limit;
        self.mareco_speed_limit |= other.mareco_speed_limit;
    }
}

/// Envelope parts are polylines over speed and space, defined as a sequence of points.
/// - single and zero point polylines are not valid
/// - the position of line points must be strictly increasing
/// - each segment of the line is thus a step over space, which is indexed from 0 to stepCount()
#[derive(Clone, Debug)]
pub struct EnvelopePart {
    attrs: Attrs,
    /// A list of N spacial offsets.
    positions: Vec<f64>,
    /// A list of N speeds, one per position.
    speeds: Vec<f64>,
    /// A list of N - 1 time deltas between positions.
    time_deltas: Vec<f64>,
    /// Required for inverse lookups on speeds.
    strictly_monotonic_speeds: bool,
    max_speed: f64,
    min_speed: f64,
    /// The time from the start of the part to each point, in microseconds.
    cumulative_us_times: Vec<i64>,
}

impl PartialEq for EnvelopePart {
    fn eq(&self, other: &Self) -> bool {
        self.attrs == other.attrs && self.positions == other.positions && self.speeds == other.speeds && self.time_deltas == other.time_deltas
    }
}

impl SearchableEnvelope for EnvelopePart {
    fn binary_search_positions(&self, position: f64) -> isize {
        binary_search(&self.positions, position)
    }

    fn position_points_count(&self) -> usize {
        self.positions.len()
    }
}

impl EnvelopePart {
    /// Creates an EnvelopePart.
    pub fn new(attrs: Attrs, positions: Vec<f64>, speeds: Vec<f64>, time_deltas: Vec<f64>) -> Self {
        let strictly_monotonic_speeds = check_strictly_monotonic(&speeds);
        let max_speed = speeds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_speed = speeds.iter().copied().fold(f64::INFINITY, f64::min);
        let mut cumulative_us_times = Vec::with_capacity(positions.len());
        cumulative_us_times.push(0i64);
        let mut total_time: i64 = 0;
        for dt in &time_deltas {
            total_time += (dt * 1_000_000.0) as i64;
            cumulative_us_times.push(total_time);
        }
        let part = EnvelopePart {
            attrs,
            positions,
            speeds,
            time_deltas,
            strictly_monotonic_speeds,
            max_speed,
            min_speed,
            cumulative_us_times,
        };
        part.run_sanity_checks();
        part
    }

    /// Creates an envelope part by generating step times from speeds and positions.
    pub fn generate_times(attrs: Attrs, positions: Vec<f64>, speeds: Vec<f64>) -> Self {
        let times = compute_times(&positions, &speeds);
        EnvelopePart::new(attrs, positions, speeds, times)
    }

    /// Runs every assertion on the envelope part values.
    fn run_sanity_checks(&self) {
        debug_assert!(self.attrs.profile.is_some(), "missing EnvelopeProfile attribute");
        debug_assert!(!self.positions.is_empty(), "attempted to create an empty EnvelopePart");
        debug_assert!(self.positions.len() == self.speeds.len(), "there must be the same number of point and speeds");
        debug_assert!(
            self.time_deltas.len() + 1 == self.positions.len(),
            "there must be as many timeDeltas as gaps between points"
        );
        debug_assert!(self.positions.iter().all(|v| !v.is_nan()), "NaNs in positions");
        debug_assert!(self.speeds.iter().all(|v| !v.is_nan()), "NaNs in speeds");
        debug_assert!(self.time_deltas.iter().all(|v| !v.is_nan()), "NaNs in timeDeltas");
        debug_assert!(self.speeds.iter().all(|v| *v >= 0.0), "negative speeds");
        debug_assert!(self.time_deltas.iter().all(|v| *v >= 0.0), "negative timeDeltas");
        debug_assert!(check_strictly_monotonic_increasing(&self.positions), "positions aren't strictly increasing");
    }

    pub fn attrs(&self) -> &Attrs {
        &self.attrs
    }

    pub fn profile(&self) -> Option<EnvelopeProfile> {
        self.attrs.profile
    }

    pub fn has_profile(&self, profile: EnvelopeProfile) -> bool {
        self.attrs.profile == Some(profile)
    }

    fn check_position(&self, step_index: usize, position: f64) -> bool {
        position >= self.begin_pos_at(step_index) && position <= self.end_pos_at(step_index)
    }

    /// The number of points in the envelope part.
    pub fn point_count(&self) -> usize {
        self.positions.len()
    }

    /// The number of steps in the envelope part.
    pub fn step_count(&self) -> usize {
        self.positions.len() - 1
    }

    pub fn max_speed(&self) -> f64 {
        self.max_speed
    }

    pub fn min_speed(&self) -> f64 {
        self.min_speed
    }

    pub fn begin_pos_at(&self, step_index: usize) -> f64 {
        self.positions[step_index]
    }

    pub fn begin_pos(&self) -> f64 {
        self.begin_pos_at(0)
    }

    pub fn end_pos_at(&self, step_index: usize) -> f64 {
        self.positions[step_index + 1]
    }

    pub fn end_pos(&self) -> f64 {
        self.end_pos_at(self.step_count() - 1)
    }

    pub fn total_distance(&self) -> f64 {
        (self.end_pos() - self.begin_pos()).abs()
    }

    pub fn begin_speed_at(&self, step_index: usize) -> f64 {
        self.speeds[step_index]
    }

    pub fn begin_speed(&self) -> f64 {
        self.begin_speed_at(0)
    }

    pub fn end_speed_at(&self, step_index: usize) -> f64 {
        self.speeds[step_index + 1]
    }

    pub fn end_speed(&self) -> f64 {
        self.end_speed_at(self.step_count() - 1)
    }

    pub fn point_pos(&self, point_index: usize) -> f64 {
        self.positions[point_index]
    }

    pub fn point_speed(&self, point_index: usize) -> f64 {
        self.speeds[point_index]
    }

    pub fn step_time(&self, step_index: usize) -> f64 {
        self.time_deltas[step_index]
    }

    pub fn positions(&self) -> &[f64] {
        &self.positions
    }

    pub fn speeds(&self) -> &[f64] {
        &self.speeds
    }

    pub fn time_deltas(&self) -> &[f64] {
        &self.time_deltas
    }

    /// Returns the total time of the envelope part, in microseconds.
    pub fn total_time_us(&self) -> i64 {
        *self.cumulative_us_times.last().unwrap()
    }

    /// Returns the time from the start of the part to the given point, in microseconds.
    pub fn total_time_us_at(&self, point_index: usize) -> i64 {
        self.cumulative_us_times[point_index]
    }

    /// Given a position return the interpolated speed.
    pub fn interpolate_speed(&self, position: f64) -> f64 {
        debug_assert!(position >= self.begin_pos() && position <= self.end_pos());
        let point_index = binary_search(&self.positions, position);
        // if the position matches one of the data points
        if point_index >= 0 {
            return self.speeds[point_index as usize];
        }
        // when the position isn't found, binarySearch returns -(insertion point) - 1
        let insertion_point = (-(point_index + 1)) as usize;
        // the index of the step is the index of the point which starts the range
        let step_index = insertion_point - 1;
        interpolate_step_speed(
            self.positions[step_index],
            self.positions[step_index + 1],
            self.speeds[step_index],
            self.speeds[step_index + 1],
            position - self.positions[step_index],
        )
    }

    /// Given a position inside a step, return the interpolated speed.
    pub fn interpolate_speed_in_step(&self, step_index: usize, position: f64) -> f64 {
        debug_assert!(self.check_position(step_index, position));
        if position == self.positions[step_index] {
            return self.speeds[step_index];
        }
        if position == self.positions[step_index + 1] {
            return self.speeds[step_index + 1];
        }
        interpolate_step_speed(
            self.positions[step_index],
            self.positions[step_index + 1],
            self.speeds[step_index],
            self.speeds[step_index + 1],
            position - self.positions[step_index],
        )
    }

    /// Given a position return the interpolated deltaTime.
    fn interpolate_time_delta(&self, step_index: usize, position: f64) -> f64 {
        debug_assert!(self.check_position(step_index, position));
        if position == self.positions[step_index] {
            return 0.0;
        }
        if position == self.positions[step_index + 1] {
            return self.time_deltas[step_index];
        }
        interpolate_step_time_at(
            self.positions[step_index],
            self.positions[step_index + 1],
            self.speeds[step_index],
            self.speeds[step_index + 1],
            position - self.positions[step_index],
        )
    }

    /// Returns the time required to get from the start of the envelope part
    /// to the given position, in microseconds.
    pub fn interpolate_total_time_us(&self, position: f64) -> i64 {
        debug_assert!(position >= self.begin_pos());
        debug_assert!(position <= self.end_pos());
        let point_index = binary_search(&self.positions, position);
        if point_index >= 0 {
            return self.total_time_us_at(point_index as usize);
        }
        let insertion_point = (-(point_index + 1)) as usize;
        let step_index = insertion_point - 1;
        let time_to_step_start = self.total_time_us_at(step_index);
        let interpolated_time = self.interpolate_time_delta(step_index, position);
        time_to_step_start + (interpolated_time * 1_000_000.0) as i64
    }

    /// Returns the time required to get from the start of the envelope part
    /// to the given position, in seconds.
    pub fn interpolate_total_time(&self, position: f64) -> f64 {
        self.interpolate_total_time_us(position) as f64 / 1_000_000.0
    }

    /// Given a speed return a position. The envelope part must be bijective
    /// in order for this method to work.
    pub fn interpolate_position(&self, speed: f64) -> f64 {
        debug_assert!(self.strictly_monotonic_speeds);
        debug_assert!(is_between(speed, self.min_speed, self.max_speed));
        for i in 0..self.positions.len() - 1 {
            let step_begin = self.positions[i];
            let step_end = self.positions[i + 1];
            let speed_begin = self.speeds[i];
            let speed_end = self.speeds[i + 1];
            if is_between(speed, speed_begin, speed_end) {
                return intersect_step_with_speed(step_begin, speed_begin, step_end, speed_end, speed);
            }
        }
        unreachable!("This should be unreachable.")
    }

    /// Makes a copy of this EnvelopePart from beginStepIndex (included) to endStepIndex (excluded).
    pub fn slice_index(&self, begin_step_index: usize, end_step_index: usize) -> Option<EnvelopePart> {
        let (positions, speeds, times) = self.slice_index_raw(begin_step_index, end_step_index)?;
        Some(EnvelopePart::new(self.attrs, positions, speeds, times))
    }

    #[allow(clippy::type_complexity)]
    fn slice_index_raw(&self, begin_step_index: usize, end_step_index: usize) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>)> {
        debug_assert!(end_step_index <= self.step_count());
        debug_assert!(begin_step_index <= self.step_count());
        debug_assert!(begin_step_index <= end_step_index);
        if end_step_index <= begin_step_index {
            return None;
        }
        Some((
            self.positions[begin_step_index..=end_step_index].to_vec(),
            self.speeds[begin_step_index..=end_step_index].to_vec(),
            self.time_deltas[begin_step_index..end_step_index].to_vec(),
        ))
    }

    pub fn slice_beginning(&self, end_index: usize, end_position: f64, end_speed: f64) -> Option<EnvelopePart> {
        self.slice_full(0, f64::NEG_INFINITY, f64::NAN, end_index, end_position, end_speed)
    }

    pub fn slice_end(&self, begin_index: usize, begin_position: f64, begin_speed: f64) -> Option<EnvelopePart> {
        self.slice_full(begin_index, begin_position, begin_speed, self.step_count() - 1, f64::INFINITY, f64::NAN)
    }

    /// Cuts an envelope part with imposed speeds on the edges.
    pub fn slice_with_speeds(&self, begin_position: f64, begin_speed: f64, end_position: f64, end_speed: f64) -> Option<EnvelopePart> {
        let (begin_index, new_begin_pos, end_index, new_end_pos) = self.slice_bounds(begin_position, end_position);
        self.slice_full(begin_index, new_begin_pos, begin_speed, end_index, new_end_pos, end_speed)
    }

    /// Cuts an envelope part, interpolating new points if required.
    pub fn slice(&self, begin_position: f64, end_position: f64) -> Option<EnvelopePart> {
        let (begin_index, new_begin_pos, end_index, new_end_pos) = self.slice_bounds(begin_position, end_position);
        self.slice_full(begin_index, new_begin_pos, f64::NAN, end_index, new_end_pos, f64::NAN)
    }

    fn slice_bounds(&self, begin_position: f64, end_position: f64) -> (usize, f64, usize, f64) {
        let mut new_begin_pos = begin_position;
        let mut new_end_pos = end_position;
        let mut begin_index = 0usize;
        if new_begin_pos <= self.begin_pos() {
            new_begin_pos = f64::NEG_INFINITY;
        }
        if new_begin_pos != f64::NEG_INFINITY {
            begin_index = self.find_right(new_begin_pos) as usize;
        }
        let mut end_index = self.step_count() - 1;
        if new_end_pos >= self.end_pos() {
            new_end_pos = f64::INFINITY;
        }
        if new_end_pos != f64::INFINITY {
            end_index = self.find_left(new_end_pos) as usize;
        }
        (begin_index, new_begin_pos, end_index, new_end_pos)
    }

    /// Cuts an envelope part, interpolating new points if required.
    ///
    /// The begin position must belong to the step at begin_step_index, the
    /// end position to the step at end_step_index; NaN speeds are
    /// interpolated, others forced; infinite positions keep the part's ends.
    pub fn slice_full(
        &self,
        begin_step_index: usize,
        begin_position: f64,
        begin_speed: f64,
        end_step_index: usize,
        end_position: f64,
        end_speed: f64,
    ) -> Option<EnvelopePart> {
        let mut new_begin_step_index = begin_step_index as isize;
        let mut new_begin_pos = begin_position;
        let mut new_begin_speed = begin_speed;
        let mut new_end_step_index = end_step_index as isize;
        let mut new_end_pos = end_position;
        let mut new_end_speed = end_speed;
        debug_assert!(end_step_index < self.step_count());
        debug_assert!(begin_step_index < self.step_count());

        // remove empty ranges from the slice and avoid needless interpolations
        if new_end_pos == self.begin_pos_at(end_step_index) {
            new_end_pos = f64::INFINITY;
            new_end_step_index -= 1;
        } else if new_end_pos == self.end_pos_at(end_step_index) {
            new_end_pos = f64::INFINITY;
        }
        if new_begin_pos == self.end_pos_at(begin_step_index) {
            new_begin_pos = f64::NEG_INFINITY;
            new_begin_step_index += 1;
        } else if new_begin_pos == self.begin_pos_at(begin_step_index) {
            new_begin_pos = f64::NEG_INFINITY;
        }

        // if the slice spans all the envelope part, don't make a copy
        if new_begin_step_index == 0
            && new_end_step_index == self.step_count() as isize - 1
            && new_begin_pos == f64::NEG_INFINITY
            && new_end_pos == f64::INFINITY
            && new_begin_speed.is_nan()
            && new_end_speed.is_nan()
        {
            return Some(self.clone());
        }

        if new_end_step_index < new_begin_step_index || new_end_step_index < 0 {
            return None;
        }
        let new_begin_step_index = new_begin_step_index as usize;
        let new_end_step_index = new_end_step_index as usize;
        // copy affected steps
        let (mut positions, mut speeds, mut times) = self.slice_index_raw(new_begin_step_index, new_end_step_index + 1)?;

        // interpolate if necessary
        if new_end_pos != f64::INFINITY {
            if new_end_speed.is_nan() {
                new_end_speed = self.interpolate_speed_in_step(new_end_step_index, new_end_pos);
            }
            let interpolated_time_delta = self.interpolate_time_delta(new_end_step_index, new_end_pos);
            let last_point = positions.len() - 1;
            positions[last_point] = new_end_pos;
            let last_step = times.len() - 1;
            times[last_step] = interpolated_time_delta;
        }
        if new_begin_pos != f64::NEG_INFINITY {
            if new_begin_speed.is_nan() {
                new_begin_speed = self.interpolate_speed_in_step(new_begin_step_index, new_begin_pos);
            }
            let interpolated_time_delta = self.interpolate_time_delta(new_begin_step_index, new_begin_pos);
            positions[0] = new_begin_pos;
            let mut first_time_delta = times[0] - interpolated_time_delta;
            if first_time_delta < 0.0 {
                // This is unlikely but can happen on slices within epsilon of steps
                assert!(-first_time_delta < TIME_EPSILON);
                first_time_delta = 0.0;
            }
            times[0] = first_time_delta;
        }
        if !new_begin_speed.is_nan() {
            speeds[0] = new_begin_speed;
        }
        if !new_end_speed.is_nan() {
            let last_point = speeds.len() - 1;
            speeds[last_point] = new_end_speed;
        }
        Some(EnvelopePart::new(self.attrs, positions, speeds, times))
    }

    /// Returns a new EnvelopePart, where all positions are shifted by
    /// positionDelta. Resulting positions are clipped to [minPosition; maxPosition].
    pub fn copy_and_shift(&self, position_delta: f64, min_position: Option<f64>, max_position: Option<f64>) -> EnvelopePart {
        let mut new_positions = vec![self.positions[0] + position_delta];
        let mut new_speeds = vec![self.speeds[0]];
        let mut new_time_deltas = Vec::new();
        for i in 1..self.positions.len() {
            let mut p = self.positions[i] + position_delta;
            if let Some(min) = min_position {
                p = p.max(min);
            }
            if let Some(max) = max_position {
                p = p.min(max);
            }
            if *new_positions.last().unwrap() != p {
                // Positions that are an epsilon away may be overlapping after
                // the shift, we only add the distinct ones
                new_positions.push(p);
                new_speeds.push(self.speeds[i]);
                new_time_deltas.push(self.time_deltas[i - 1]);
            }
        }
        EnvelopePart::new(self.attrs, new_positions, new_speeds, new_time_deltas)
    }

    /// The same part under other attributes.
    pub fn with_attrs(&self, attrs: Attrs) -> EnvelopePart {
        EnvelopePart { attrs, ..self.clone() }
    }
}

/// Compute the minimum EnvelopePart between 2 envelope parts starting and
/// ending at the exact same positions.
pub fn min_envelope_parts(a: &EnvelopePart, b: &EnvelopePart, attrs: Attrs) -> EnvelopePart {
    debug_assert!(a.begin_pos() == b.begin_pos());
    debug_assert!(a.end_pos() == b.end_pos());
    let mut key_positions: BTreeSet<OrdF64> = a.positions().iter().map(|p| OrdF64(*p)).collect();
    key_positions.extend(b.positions().iter().map(|p| OrdF64(*p)));

    let mut new_positions = Vec::new();
    let mut new_speeds = Vec::new();
    let mut prev: Option<(f64, f64, f64)> = None;
    for OrdF64(pos) in key_positions {
        let speed_a = a.interpolate_speed(pos);
        let speed_b = b.interpolate_speed(pos);
        let min_speed_at_pos = speed_a.min(speed_b);
        if let Some((prev_pos, prev_speed_a, prev_speed_b)) = prev
            && (prev_speed_a - prev_speed_b) * (speed_a - speed_b) < 0.0
        {
            let intersection = intersect_steps(prev_pos, prev_speed_a, pos, speed_a, prev_pos, prev_speed_b, pos, speed_b);
            // Add intersection point if it isn't either the previous or the current position.
            if !are_positions_equal(intersection.position, pos) && !are_positions_equal(intersection.position, prev_pos) {
                new_positions.push(intersection.position);
                new_speeds.push(intersection.speed);
            }
        }
        new_positions.push(pos);
        new_speeds.push(min_speed_at_pos);
        prev = Some((pos, speed_a, speed_b));
    }
    EnvelopePart::generate_times(attrs, new_positions, new_speeds)
}

/// A double ordered as Java's `Double.compare`, for sorted sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OrdF64(pub f64);

impl Eq for OrdF64 {}

impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

fn check_strictly_monotonic_increasing(values: &[f64]) -> bool {
    values.windows(2).all(|w| w[0] < w[1])
}

fn check_strictly_monotonic_decreasing(values: &[f64]) -> bool {
    values.windows(2).all(|w| w[0] > w[1])
}

fn check_strictly_monotonic(values: &[f64]) -> bool {
    check_strictly_monotonic_increasing(values) || check_strictly_monotonic_decreasing(values)
}

/// Compute the time deltas between positions.
fn compute_times(positions: &[f64], speeds: &[f64]) -> Vec<f64> {
    (0..positions.len() - 1)
        .map(|i| {
            let pos_delta = positions[i + 1] - positions[i];
            interpolate_step_time_at(positions[i], positions[i + 1], speeds[i], speeds[i + 1], pos_delta)
        })
        .collect()
}

/// Check if a is in the interval [b, c] or [c, b].
fn is_between(a: f64, b: f64, c: f64) -> bool {
    let min = b.min(c);
    let max = b.max(c);
    a >= min && a <= max
}
