// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope/Envelope.kt and EnvelopeBuilder.kt.

use std::collections::BTreeSet;

use super::part::{EnvelopePart, OrdF64, min_envelope_parts};
use crate::units::{SearchableEnvelope, are_speeds_equal, binary_search, clamp};

/// A point of an envelope with the time it is passed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedPoint {
    pub time: f64,
    pub speed: f64,
    pub position: f64,
}

/// Speed over space for a whole path: envelope parts end to end.
#[derive(Clone, Debug)]
pub struct Envelope {
    parts: Vec<EnvelopePart>,
    pub continuous: bool,
    /// The position of all transitions, including the beginning and end positions.
    part_positions: Vec<f64>,
    max_speed: f64,
    min_speed: f64,
    /// The time from the start of the envelope to envelope part transitions, in microseconds.
    cumulative_times_us: Vec<i64>,
}

impl SearchableEnvelope for Envelope {
    fn binary_search_positions(&self, position: f64) -> isize {
        binary_search(&self.part_positions, position)
    }

    fn position_points_count(&self) -> usize {
        self.part_positions.len()
    }
}

impl Envelope {
    /// Creates an envelope. The parts must touch; their speeds may jump.
    pub fn new(parts: Vec<EnvelopePart>) -> Envelope {
        assert!(!parts.is_empty(), "an envelope needs a part");
        // check for space and speed continuity. space continuity is mandatory, speed continuity is not
        let mut continuous = true;
        for i in 0..parts.len() - 1 {
            assert!(parts[i].end_pos() == parts[i + 1].begin_pos(), "envelope parts are not contiguous");
            if !are_speeds_equal(parts[i].end_speed(), parts[i + 1].begin_speed()) {
                continuous = false;
            }
        }
        let min_speed = parts.iter().map(|p| p.min_speed()).fold(f64::INFINITY, f64::min);
        let max_speed = parts.iter().map(|p| p.max_speed()).fold(f64::NEG_INFINITY, f64::max);
        let mut part_positions = Vec::with_capacity(parts.len() + 1);
        part_positions.push(parts[0].begin_pos());
        part_positions.extend(parts.iter().map(|p| p.end_pos()));
        let mut cumulative_times_us = Vec::with_capacity(parts.len() + 1);
        cumulative_times_us.push(0);
        let mut total = 0i64;
        for part in &parts {
            total += part.total_time_us();
            cumulative_times_us.push(total);
        }
        Envelope {
            parts,
            continuous,
            part_positions,
            max_speed,
            min_speed,
            cumulative_times_us,
        }
    }

    pub fn size(&self) -> usize {
        self.parts.len()
    }

    pub fn get(&self, i: usize) -> &EnvelopePart {
        &self.parts[i]
    }

    pub fn parts(&self) -> &[EnvelopePart] {
        &self.parts
    }

    pub fn iter(&self) -> std::slice::Iter<'_, EnvelopePart> {
        self.parts.iter()
    }

    pub fn into_parts(self) -> Vec<EnvelopePart> {
        self.parts
    }

    pub fn max_speed(&self) -> f64 {
        self.max_speed
    }

    pub fn min_speed(&self) -> f64 {
        self.min_speed
    }

    pub fn begin_pos(&self) -> f64 {
        self.parts[0].begin_pos()
    }

    pub fn end_pos(&self) -> f64 {
        self.parts.last().unwrap().end_pos()
    }

    pub fn total_distance(&self) -> f64 {
        self.end_pos() - self.begin_pos()
    }

    pub fn begin_speed(&self) -> f64 {
        self.parts[0].begin_speed()
    }

    pub fn end_speed(&self) -> f64 {
        self.parts.last().unwrap().end_speed()
    }

    pub fn part_positions(&self) -> &[f64] {
        &self.part_positions
    }

    /// Returns the interpolated speed at a given position. Assumes the envelope is continuous.
    pub fn interpolate_speed(&self, position: f64) -> f64 {
        debug_assert!(self.continuous, "interpolating speeds on a non continuous envelope is a risky business");
        let index = self.find_left(position);
        debug_assert!(index != -1);
        self.get(index as usize).interpolate_speed(position)
    }

    /// Interpolates speeds, prefers parts coming from the left, along the given direction.
    pub fn interpolate_speed_left_dir(&self, position: f64, direction: f64) -> f64 {
        let index = self.find_left_dir(position, direction);
        self.get(index as usize).interpolate_speed(position)
    }

    /// Interpolates speeds, prefers parts coming from the right, along the given direction.
    pub fn interpolate_speed_right_dir(&self, position: f64, direction: f64) -> f64 {
        let index = self.find_right_dir(position, direction);
        self.get(index as usize).interpolate_speed(position)
    }

    /// Return the maximum speed in the range [beginPos, endPos]. Assumes the envelope is continuous.
    pub fn max_speed_in_range(&self, begin_pos: f64, end_pos: f64) -> f64 {
        let begin_position = clamp(begin_pos, self.begin_pos(), self.end_pos());
        let end_position = clamp(end_pos, self.begin_pos(), self.end_pos());
        let begin_part_index = self.find_right(begin_position) as usize;
        let end_part_index = self.find_left(end_position) as usize;
        let mut max_speed = self.get(begin_part_index).interpolate_speed(begin_position);
        for i in begin_part_index + 1..end_part_index {
            max_speed = max_speed.max(self.get(i).max_speed());
        }
        max_speed.max(self.get(end_part_index).interpolate_speed(end_position))
    }

    fn interpolate_us(&self, position: f64) -> i64 {
        let index = self.find_left(position);
        assert!(index >= 0, "Trying to interpolate time outside of the envelope");
        let index = index as usize;
        self.cumulative_times_us[index] + self.get(index).interpolate_total_time_us(position)
    }

    /// The time from the start of the envelope to the given position, in seconds.
    pub fn interpolate_arrival_at(&self, position: f64) -> f64 {
        self.interpolate_us(position) as f64 / 1_000_000.0
    }

    pub fn interpolate_departure_from(&self, position: f64) -> f64 {
        self.interpolate_us(position) as f64 / 1_000_000.0
    }

    pub fn interpolate_arrival_at_clamp(&self, position: f64) -> f64 {
        self.interpolate_us(clamp(position, 0.0, self.end_pos())) as f64 / 1_000_000.0
    }

    pub fn interpolate_departure_from_clamp(&self, position: f64) -> f64 {
        self.interpolate_us(clamp(position, 0.0, self.end_pos())) as f64 / 1_000_000.0
    }

    /// Returns the time between the two positions of the envelope (no stop
    /// included in the envelope).
    pub fn time_between(&self, begin_pos: f64, end_pos: f64) -> f64 {
        self.interpolate_departure_from(end_pos) - self.interpolate_departure_from(begin_pos)
    }

    /// Returns the total time of the envelope, in seconds.
    pub fn total_time(&self) -> f64 {
        *self.cumulative_times_us.last().unwrap() as f64 / 1_000_000.0
    }

    /// Cuts an envelope, interpolating new points if required.
    pub fn slice(&self, begin_position: f64, end_position: f64) -> Vec<EnvelopePart> {
        self.slice_with_speeds(begin_position, f64::NAN, end_position, f64::NAN)
    }

    /// Cuts an envelope, interpolating new points if required, forcing the
    /// speeds at the edges unless they are NaN.
    pub fn slice_with_speeds(&self, begin_position: f64, begin_speed: f64, end_position: f64, end_speed: f64) -> Vec<EnvelopePart> {
        let mut begin_index = 0usize;
        let mut begin_part_index = 0usize;
        if begin_position != f64::NEG_INFINITY {
            begin_part_index = self.find_right(begin_position) as usize;
            begin_index = self.parts[begin_part_index].find_right(begin_position) as usize;
        }
        let mut end_part_index = self.parts.len() - 1;
        let mut end_index = self.parts[end_part_index].step_count() - 1;
        if end_position != f64::INFINITY {
            end_part_index = self.find_left(end_position) as usize;
            end_index = self.parts[end_part_index].find_left(end_position) as usize;
        }
        self.slice_full(
            begin_part_index,
            begin_index,
            begin_position,
            begin_speed,
            end_part_index,
            end_index,
            end_position,
            end_speed,
        )
    }

    /// Cuts an envelope.
    #[allow(clippy::too_many_arguments)]
    pub fn slice_full(
        &self,
        begin_part_index: usize,
        begin_step_index: usize,
        begin_position: f64,
        begin_speed: f64,
        end_part_index: usize,
        end_step_index: usize,
        end_position: f64,
        end_speed: f64,
    ) -> Vec<EnvelopePart> {
        debug_assert!(begin_part_index <= end_part_index);
        if begin_part_index == end_part_index {
            let part = &self.parts[begin_part_index];
            return part
                .slice_full(begin_step_index, begin_position, begin_speed, end_step_index, end_position, end_speed)
                .into_iter()
                .collect();
        }
        let begin_part_sliced = self.parts[begin_part_index].slice_end(begin_step_index, begin_position, begin_speed);
        let end_part_sliced = self.parts[end_part_index].slice_beginning(end_step_index, end_position, end_speed);
        let mut res = Vec::new();
        res.extend(begin_part_sliced);
        res.extend(self.parts[begin_part_index + 1..end_part_index].iter().cloned());
        res.extend(end_part_sliced);
        res
    }

    /// All the points as (time, speed, position).
    pub fn iterate_points(&self) -> Vec<TimedPoint> {
        let mut res = Vec::new();
        let mut time = 0.0;
        for part in &self.parts {
            for i in 0..part.point_count() {
                res.push(TimedPoint {
                    time,
                    speed: part.point_speed(i),
                    position: part.point_pos(i),
                });
                if i < part.step_count() {
                    time += part.step_time(i);
                }
            }
        }
        res
    }
}

/// Build the min envelopes between 2 envelopes starting and ending at the exact same positions.
pub fn min_envelopes(a: &Envelope, b: &Envelope) -> Envelope {
    debug_assert!(a.begin_pos() == b.begin_pos());
    debug_assert!(a.end_pos() == b.end_pos());
    let mut key_positions: BTreeSet<OrdF64> = a.part_positions().iter().map(|p| OrdF64(*p)).collect();
    key_positions.extend(b.part_positions().iter().map(|p| OrdF64(*p)));
    let keys: Vec<f64> = key_positions.into_iter().map(|p| p.0).collect();
    let mut min_parts = Vec::new();
    for w in keys.windows(2) {
        let (begin_pos, end_pos) = (w[0], w[1]);
        let sliced_a = a.get(a.find_right(begin_pos) as usize).slice(begin_pos, end_pos).unwrap();
        let sliced_b = b.get(b.find_right(begin_pos) as usize).slice(begin_pos, end_pos).unwrap();
        let attrs = *sliced_a.attrs();
        min_parts.push(min_envelope_parts(&sliced_a, &sliced_b, attrs));
    }
    Envelope::new(min_parts)
}

/// Creates an envelope by concatenating envelope parts. Envelope parts must not overlap.
#[derive(Default)]
pub struct EnvelopeBuilder {
    parts: Vec<EnvelopePart>,
}

impl EnvelopeBuilder {
    pub fn new() -> Self {
        EnvelopeBuilder::default()
    }

    pub fn add_part(&mut self, part: EnvelopePart) {
        self.parts.push(part);
    }

    pub fn add_parts(&mut self, parts: impl IntoIterator<Item = EnvelopePart>) {
        self.parts.extend(parts);
    }

    pub fn add_envelope(&mut self, envelope: &Envelope) {
        self.parts.extend(envelope.iter().cloned());
    }

    pub fn reverse(&mut self) {
        self.parts.reverse();
    }

    pub fn build(self) -> Envelope {
        Envelope::new(self.parts)
    }
}
