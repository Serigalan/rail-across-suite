// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/osrd-mp/.../utils/UnitEquality.kt and
// core/envelope-sim/.../envelope_utils/DoubleUtils.java, CmpOperator.java.

//! Tolerances, and comparisons of doubles with them.

/// A position delta lower than this value will be considered zero.
pub const POSITION_EPSILON: f64 = 1e-2;
/// A speed lower than this value will be considered zero.
pub const SPEED_EPSILON: f64 = 1e-5;
/// An acceleration lower than this value will be considered zero.
pub const ACCELERATION_EPSILON: f64 = 1e-5;
/// Time epsilon matching a 0.01 m epsilon on position at 360 km/h.
pub const TIME_EPSILON: f64 = 1e-4;

pub fn are_doubles_equal(a: f64, b: f64, delta: f64) -> bool {
    (a - b).abs() < delta
}

pub fn are_positions_equal(a: f64, b: f64) -> bool {
    are_doubles_equal(a, b, POSITION_EPSILON)
}

pub fn are_speeds_equal(a: f64, b: f64) -> bool {
    are_doubles_equal(a, b, SPEED_EPSILON)
}

pub fn are_accelerations_equal(a: f64, b: f64) -> bool {
    are_doubles_equal(a, b, ACCELERATION_EPSILON)
}

pub fn are_times_equal(a: f64, b: f64) -> bool {
    are_doubles_equal(a, b, TIME_EPSILON)
}

/// Clamp a value between a given range.
pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
    debug_assert!(min <= max);
    if val < min {
        return min;
    }
    val.min(max)
}

/// Flips the sign of value if flip is negative.
pub fn conditional_negate(value: f64, flip: f64) -> f64 {
    f64::from_bits(value.to_bits() ^ (flip.to_bits() & 0x8000_0000_0000_0000))
}

/// Compare a and b along a given direction.
pub fn dir_compare(direction: f64, a: f64, b: f64) -> f64 {
    conditional_negate(a - b, direction)
}

/// Java's `Math.signum`.
fn signum(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOperator {
    StrictlyLower,
    Lower,
    StrictlyHigher,
    Higher,
}

impl CmpOperator {
    fn is_strict(self) -> bool {
        matches!(self, CmpOperator::StrictlyLower | CmpOperator::StrictlyHigher)
    }

    fn expected_sign(self) -> f64 {
        match self {
            CmpOperator::StrictlyLower | CmpOperator::Lower => -1.0,
            CmpOperator::StrictlyHigher | CmpOperator::Higher => 1.0,
        }
    }

    /// Compares value with reference.
    pub fn compare(value_a: f64, operator: CmpOperator, value_b: f64) -> bool {
        let res_sign = signum(value_a - value_b);
        if res_sign == 0.0 {
            return !operator.is_strict();
        }
        res_sign == operator.expected_sign()
    }
}

/// Java's `Arrays.binarySearch` on a sorted array of doubles: the index of
/// the value, or `-(insertion point) - 1`. Doubles compare as Java's
/// `Double.compare` does (`-0.0` below `0.0`).
pub fn binary_search(values: &[f64], key: f64) -> isize {
    match values.binary_search_by(|v| v.total_cmp(&key)) {
        Ok(i) => i as isize,
        Err(i) => -(i as isize) - 1,
    }
}

/// A 1D monotonic polyline of positions, searched for the segment holding a
/// position: the points of an envelope part, the transitions of an envelope
/// (OSRD's `SearchableEnvelope`).
pub trait SearchableEnvelope {
    /// Runs a binary search on the positions array.
    fn binary_search_positions(&self, position: f64) -> isize;
    /// Returns the number of positions.
    fn position_points_count(&self) -> usize;

    /// Find the leftmost segment (with the lowest position) which contains the given position.
    fn find_left(&self, position: f64) -> isize {
        let point_index = self.binary_search_positions(position);
        // if the position matches one of the data points, return the range on the left, if any
        if point_index >= 0 {
            if point_index == 0 {
                return 0;
            }
            return point_index - 1;
        }
        // when the position isn't found, binarySearch returns -(insertion point) - 1
        let insertion_point = -(point_index + 1);
        if insertion_point == 0 || insertion_point == self.position_points_count() as isize {
            return -insertion_point - 1;
        }
        // the index of the step is the index of the point which starts the range
        insertion_point - 1
    }

    /// Find the leftmost index (with the lowest position) which contains the given position.
    fn find_right(&self, position: f64) -> isize {
        let point_index = self.binary_search_positions(position);
        if point_index >= 0 {
            if point_index == self.position_points_count() as isize - 1 {
                return point_index - 1;
            }
            return point_index;
        }
        let insertion_point = -(point_index + 1);
        if insertion_point == 0 || insertion_point == self.position_points_count() as isize {
            return -insertion_point - 1;
        }
        insertion_point - 1
    }

    /// Returns the first index which contains a position along a given direction.
    fn find_left_dir(&self, position: f64, direction: f64) -> isize {
        if direction < 0.0 {
            return self.find_right(position);
        }
        self.find_left(position)
    }

    /// Returns the last index which contains a position along a given direction.
    fn find_right_dir(&self, position: f64, direction: f64) -> isize {
        if direction < 0.0 {
            return self.find_left(position);
        }
        self.find_right(position)
    }
}
