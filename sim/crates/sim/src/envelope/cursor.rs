// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope/EnvelopeCursor.java.

use super::envelope::Envelope;
use super::part::EnvelopePart;
use super::physics::intersect_step_with_speed;
use crate::units::CmpOperator;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextStepResult {
    NextStep,
    NextPart,
    NextReachedEnd,
}

/// Walks an envelope, forward or backward, step by step.
#[derive(Clone, Debug)]
pub struct EnvelopeCursor<'a> {
    /// The envelope the cursor iterates over.
    pub envelope: &'a Envelope,
    /// Whether the cursor works in reverse.
    pub reverse: bool,
    /// The index of the part inside the envelope, or -1 if the end was reached.
    part_index: isize,
    /// The current index inside the part, or -1 if the end was reached.
    step_index: isize,
    /// The position in the envelope, or NaN if the end was reached.
    position: f64,
    /// The speed at the current position, or NaN if it should be interpolated.
    speed: f64,
    /// The number of times this cursor was moved.
    revision: u64,
}

impl<'a> EnvelopeCursor<'a> {
    /// Creates a cursor at a given part, step and position.
    pub fn at(envelope: &'a Envelope, reverse: bool, part_index: usize, step_index: usize, position: f64) -> Self {
        EnvelopeCursor {
            envelope,
            reverse,
            part_index: part_index as isize,
            step_index: step_index as isize,
            position,
            speed: f64::NAN,
            revision: 0,
        }
    }

    /// Creates a cursor at the start of the envelope, along its direction.
    pub fn new(envelope: &'a Envelope, reverse: bool) -> Self {
        let mut cursor = EnvelopeCursor {
            envelope,
            reverse,
            part_index: 0,
            step_index: 0,
            position: f64::NAN,
            speed: f64::NAN,
            revision: 0,
        };
        cursor.part_index = cursor.first_index(envelope.size());
        cursor.step_index = cursor.first_index(cursor.part().step_count());
        cursor.position = cursor.step_begin_pos();
        cursor
    }

    pub fn forward(envelope: &'a Envelope) -> Self {
        EnvelopeCursor::new(envelope, false)
    }

    pub fn backward(envelope: &'a Envelope) -> Self {
        EnvelopeCursor::new(envelope, true)
    }

    pub fn has_reached_end(&self) -> bool {
        self.part_index == -1
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn step_index(&self) -> usize {
        debug_assert!(!self.has_reached_end());
        self.step_index as usize
    }

    pub fn part_index(&self) -> usize {
        debug_assert!(!self.has_reached_end());
        self.part_index as usize
    }

    pub fn position(&self) -> f64 {
        debug_assert!(!self.has_reached_end());
        self.position
    }

    pub fn part(&self) -> &'a EnvelopePart {
        debug_assert!(!self.has_reached_end());
        self.envelope.get(self.part_index as usize)
    }

    fn first_index(&self, size: usize) -> isize {
        if self.reverse { size as isize - 1 } else { 0 }
    }

    fn last_index(&self, size: usize) -> isize {
        if self.reverse { 0 } else { size as isize - 1 }
    }

    /// Returns the next index, or -1 if there is no valid next index.
    fn next_index(&self, cur: isize, size: usize) -> isize {
        if self.reverse {
            let res = cur - 1;
            if res < 0 { -1 } else { res }
        } else {
            let res = cur + 1;
            if res >= size as isize { -1 } else { res }
        }
    }

    fn step_begin_pos_of(&self, part: &EnvelopePart, step_index: usize) -> f64 {
        if self.reverse {
            part.end_pos_at(step_index)
        } else {
            part.begin_pos_at(step_index)
        }
    }

    pub fn step_begin_pos(&self) -> f64 {
        self.step_begin_pos_of(self.part(), self.step_index as usize)
    }

    fn step_end_pos_of(&self, part: &EnvelopePart, step_index: usize) -> f64 {
        if self.reverse {
            part.begin_pos_at(step_index)
        } else {
            part.end_pos_at(step_index)
        }
    }

    pub fn step_end_pos(&self) -> f64 {
        self.step_end_pos_of(self.part(), self.step_index as usize)
    }

    pub fn step_length(&self) -> f64 {
        self.step_end_pos() - self.step_begin_pos()
    }

    pub fn step_begin_speed(&self) -> f64 {
        self.step_begin_speed_of(self.part(), self.step_index as usize)
    }

    fn step_begin_speed_of(&self, part: &EnvelopePart, step_index: usize) -> f64 {
        if self.reverse {
            part.end_speed_at(step_index)
        } else {
            part.begin_speed_at(step_index)
        }
    }

    pub fn step_end_speed(&self) -> f64 {
        self.step_end_speed_of(self.part(), self.step_index as usize)
    }

    fn step_end_speed_of(&self, part: &EnvelopePart, step_index: usize) -> f64 {
        if self.reverse {
            part.begin_speed_at(step_index)
        } else {
            part.end_speed_at(step_index)
        }
    }

    fn part_begin_pos(&self) -> f64 {
        let part = self.part();
        self.step_begin_pos_of(part, self.first_index(part.step_count()) as usize)
    }

    fn part_end_pos(&self) -> f64 {
        let part = self.part();
        self.step_end_pos_of(part, self.last_index(part.step_count()) as usize)
    }

    /// Returns the end position of the envelope.
    pub fn envelope_end_pos(&self) -> f64 {
        if self.reverse { self.envelope.begin_pos() } else { self.envelope.end_pos() }
    }

    /// Returns the speed at the end of the envelope, where "end" depends on cursor direction.
    pub fn envelope_end_speed(&self) -> f64 {
        if self.reverse {
            self.envelope.begin_speed()
        } else {
            self.envelope.end_speed()
        }
    }

    /// Compares positions in a direction aware manner.
    pub fn compare_pos(&self, a: f64, b: f64) -> f64 {
        if self.reverse { b - a } else { a - b }
    }

    /// Moves the cursor to the end of the envelope.
    pub fn move_to_end(&mut self) {
        self.part_index = -1;
        self.step_index = -1;
        self.set_position(f64::NAN, f64::NAN);
    }

    pub fn next_part_index(&self) -> isize {
        self.next_index(self.part_index, self.envelope.size())
    }

    /// Moves the cursor to the next part.
    pub fn next_part(&mut self) -> bool {
        if self.has_reached_end() {
            return false;
        }
        self.part_index = self.next_part_index();
        if self.part_index == -1 {
            self.move_to_end();
            return false;
        }
        self.step_index = self.first_index(self.part().step_count());
        let pos = self.step_begin_pos();
        self.set_position(pos, f64::NAN);
        true
    }

    /// Moves the cursor to the beginning of the next step.
    pub fn next_step(&mut self) -> NextStepResult {
        if self.has_reached_end() {
            return NextStepResult::NextReachedEnd;
        }
        self.step_index = self.next_index(self.step_index, self.part().step_count());
        if self.step_index == -1 {
            return if self.next_part() {
                NextStepResult::NextPart
            } else {
                NextStepResult::NextReachedEnd
            };
        }
        let pos = self.step_begin_pos();
        self.set_position(pos, f64::NAN);
        NextStepResult::NextStep
    }

    /// Returns the speed at the location of the cursor.
    pub fn speed(&self) -> f64 {
        if !self.speed.is_nan() {
            return self.speed;
        }
        self.part().interpolate_speed_in_step(self.step_index as usize, self.position)
    }

    /// Moves the cursor to this position, by looking for the envelope part
    /// and step which contain it. Returns whether a step with this position was found.
    pub fn find_position(&mut self, new_position: f64) -> bool {
        if self.has_reached_end() {
            return false;
        }
        debug_assert!(
            self.compare_pos(new_position, self.position) >= 0.0,
            "the new position can only move the cursor forward"
        );
        // find the EnvelopePart which contains the new position
        while self.compare_pos(self.part_end_pos(), new_position) < 0.0 {
            if !self.next_part() {
                return false;
            }
        }
        debug_assert!(self.compare_pos(self.part_begin_pos(), new_position) <= 0.0);
        // now that we found the envelope part, find the exact step
        while self.compare_pos(self.step_end_pos(), new_position) < 0.0 {
            if self.next_step() == NextStepResult::NextReachedEnd {
                return false;
            }
        }
        debug_assert!(self.compare_pos(self.step_begin_pos(), new_position) <= 0.0);
        debug_assert!(self.compare_pos(new_position, self.step_end_pos()) <= 0.0);
        self.set_position(new_position, f64::NAN);
        true
    }

    /// Set the position / speed and bumps the revision.
    fn set_position(&mut self, new_position: f64, new_speed: f64) {
        // when setting newPosition is the same as current and the speed is
        // force-set, avoid overriding the forced value
        if new_position == self.position && !self.speed.is_nan() && new_speed.is_nan() {
            return;
        }
        debug_assert!(!new_position.is_infinite());
        debug_assert!(new_position.is_nan() || self.position.is_nan() || self.compare_pos(self.position, new_position) <= 0.0);
        self.position = new_position;
        self.speed = new_speed;
        self.revision += 1;
    }

    /// Attempts to find a transition between envelope parts which matches a predicate.
    pub fn find_part_transition(&mut self, predicate: impl Fn(f64, f64, f64, f64) -> bool) -> bool {
        loop {
            if self.check_part_transition(&predicate) {
                return true;
            }
            if !self.next_part() {
                return false;
            }
        }
    }

    /// Attempts to find a part which matches a predicate.
    pub fn find_part(&mut self, predicate: impl Fn(&EnvelopePart) -> bool) -> bool {
        if self.has_reached_end() {
            return false;
        }
        loop {
            if predicate(self.part()) {
                return true;
            }
            if !self.next_part() {
                return false;
            }
        }
    }

    fn part_bound(part: &EnvelopePart, operation: CmpOperator) -> f64 {
        if operation == CmpOperator::StrictlyHigher {
            part.max_speed()
        } else {
            part.min_speed()
        }
    }

    /// Find the next point with a speed satisfying a given condition.
    pub fn find_speed(&mut self, speed: f64, operator: CmpOperator) -> bool {
        if self.has_reached_end() {
            return false;
        }
        // this predicate only matches envelope parts which contain a speed
        // which matches the search requirement
        let part_predicate = |part: &EnvelopePart| CmpOperator::compare(Self::part_bound(part, operator), operator, speed);
        // look for the next envelope part which contains a speed which
        // matches. The current part may contain such a point, but it may be
        // **before the cursor**.
        if !self.find_part(part_predicate) {
            return false;
        }
        // scan until a step matching the requirement is found
        while !CmpOperator::compare(self.speed(), operator, speed) && !CmpOperator::compare(self.step_end_speed(), operator, speed) {
            let next_step_res = self.next_step();
            if next_step_res == NextStepResult::NextReachedEnd {
                return false;
            }
            // if the current part was scanned and contains no matching step,
            // scan for another matching envelope part.
            if next_step_res == NextStepResult::NextPart && !self.find_part(part_predicate) {
                return false;
            }
        }
        // if the start of the step satisfies the condition already, there's a
        // discontinuity: return immediately, the cursor is at the start of the step.
        if CmpOperator::compare(self.speed(), operator, speed) {
            return true;
        }
        // otherwise, find the intersecting position inside the step
        let intersection_pos = intersect_step_with_speed(
            self.step_begin_pos(),
            self.step_begin_speed(),
            self.step_end_pos(),
            self.step_end_speed(),
            speed,
        );
        self.set_position(intersection_pos, speed);
        true
    }

    /// Check if the next transition between envelope parts matches the given predicate.
    pub fn check_part_transition(&mut self, predicate: &impl Fn(f64, f64, f64, f64) -> bool) -> bool {
        if self.has_reached_end() {
            return false;
        }
        let next_part_ind = self.next_part_index();
        if next_part_ind == -1 {
            return false;
        }
        let part = self.part();
        let next_part = self.envelope.get(next_part_ind as usize);
        let cur_end_index = self.last_index(part.step_count()) as usize;
        let next_start_index = self.first_index(next_part.step_count()) as usize;
        // the last point of this part and the first point of the next one
        let cur_pos = self.step_end_pos_of(part, cur_end_index);
        let cur_speed = self.step_end_speed_of(part, cur_end_index);
        let next_pos = self.step_begin_pos_of(next_part, next_start_index);
        let next_speed = self.step_begin_speed_of(next_part, next_start_index);
        // move the cursor to the end of the current part
        self.step_index = cur_end_index as isize;
        self.set_position(cur_pos, f64::NAN);
        predicate(cur_pos, cur_speed, next_pos, next_speed)
    }

    /// Check if the current part matches the given predicate.
    pub fn check_part(&self, predicate: impl Fn(&EnvelopePart) -> bool) -> bool {
        if self.position() < self.part_end_pos() {
            return predicate(self.part());
        }
        false
    }
}
