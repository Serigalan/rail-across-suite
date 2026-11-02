// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/kt-osrd-path/.../implementations/EnvelopeSimPath.kt,
// without electrification.

use crate::units::{are_positions_equal, binary_search};

/// The physics of a path taken by a train: its length and grades.
#[derive(Clone, Debug)]
pub struct SimPath {
    /// The length of the path, in meters.
    pub length: f64,
    /// The points at which the grade changes, from 0 to the length.
    grade_positions: Vec<f64>,
    /// The grade between each pair of consecutive points, in m/km.
    grade_values: Vec<f64>,
    /// The cumulative sum of the gradient at each grade position.
    grade_cum_sum: Vec<f64>,
    /// One slope everywhere, whatever the range asked (OSRD's test `FlatPath`).
    constant_slope: Option<f64>,
}

impl SimPath {
    /// A path of the given length and grades: `grade_positions` from 0 to
    /// the length, `grade_values` between consecutive positions, in m/km
    /// (curves already counted in as an equivalent grade).
    pub fn new(length: f64, grade_positions: Vec<f64>, grade_values: Vec<f64>) -> Self {
        assert!(grade_positions.len() == grade_values.len() + 1);
        assert!(grade_positions[0] == 0.0);
        assert!(grade_positions[grade_positions.len() - 1] == length);
        assert!(grade_positions.windows(2).all(|w| w[0] < w[1]));
        let mut grade_cum_sum = Vec::with_capacity(grade_positions.len());
        grade_cum_sum.push(0.0);
        let mut cum_sum = 0.0;
        for i in 0..grade_positions.len() - 1 {
            cum_sum += grade_values[i] * (grade_positions[i + 1] - grade_positions[i]);
            grade_cum_sum.push(cum_sum);
        }
        SimPath {
            length,
            grade_positions,
            grade_values,
            grade_cum_sum,
            constant_slope: None,
        }
    }

    /// A flat path.
    pub fn flat(length: f64) -> Self {
        SimPath::new(length, vec![0.0, length], vec![0.0])
    }

    /// A path of one slope, in m/km, which is also the average and lowest
    /// grade of every range, even an empty one.
    pub fn constant_slope(length: f64, slope: f64) -> Self {
        SimPath {
            constant_slope: Some(slope),
            ..SimPath::new(length, vec![0.0, length], vec![slope])
        }
    }

    fn cum_grade(&self, position: f64) -> f64 {
        let mut position = position;
        if position > self.length && are_positions_equal(position, self.length) {
            position = self.length;
        }
        debug_assert!(position >= 0.0 && position <= self.length);
        let point_index = binary_search(&self.grade_positions, position);
        if point_index >= 0 {
            return self.grade_cum_sum[point_index as usize];
        }
        // the cumulative gradient at the point before the given position,
        // plus the gradient change since then
        let insertion_point = (-(point_index + 1)) as usize;
        let range_index = insertion_point - 1;
        let range_start = self.grade_positions[range_index];
        self.grade_cum_sum[range_index] + self.grade_values[range_index] * (position - range_start)
    }

    /// The average slope on a given range, in m/km.
    pub fn average_grade(&self, begin: f64, end: f64) -> f64 {
        if let Some(slope) = self.constant_slope {
            return slope;
        }
        if begin == end {
            return self.cum_grade(begin);
        }
        (self.cum_grade(end) - self.cum_grade(begin)) / (end - begin)
    }

    /// The lowest slope on a given range, in m/km.
    pub fn min_grade(&self, begin: f64, end: f64) -> f64 {
        if let Some(slope) = self.constant_slope {
            return slope;
        }
        let index_begin = self.index_before_pos(begin);
        let index_end = self.index_before_pos(end);
        if index_begin == index_end && index_begin == self.grade_positions.len() - 1 {
            return self.grade_values[self.grade_values.len() - 1];
        }
        let mut lowest = self.grade_values[index_begin];
        for i in index_begin..index_end {
            lowest = lowest.min(self.grade_values[i]);
        }
        lowest
    }

    /// For a given position, the index of the position just before in grade_positions.
    fn index_before_pos(&self, position: f64) -> usize {
        if position <= self.grade_positions[0] {
            return 0;
        }
        if position >= self.grade_positions[self.grade_positions.len() - 1] {
            return self.grade_positions.len() - 1;
        }
        self.grade_positions
            .iter()
            .position(|p| *p > position)
            .map_or(self.grade_positions.len() - 1, |i| i - 1)
    }

    pub fn grade_positions(&self) -> &[f64] {
        &self.grade_positions
    }

    pub fn grade_values(&self) -> &[f64] {
        &self.grade_values
    }
}
