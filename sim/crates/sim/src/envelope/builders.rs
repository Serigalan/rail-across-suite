// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/envelope-sim/.../envelope/part/EnvelopePartBuilder.java,
// ConstrainedEnvelopePartBuilder.java, constraints/*.java,
// envelope/OverlayEnvelopeBuilder.kt, MRSPEnvelopeBuilder.java and EnvelopeSpeedCap.java.

use std::collections::VecDeque;

use super::cursor::{EnvelopeCursor, NextStepResult};
use super::envelope::{Envelope, EnvelopeBuilder};
use super::part::{Attrs, EnvelopePart, EnvelopeProfile, LimitKind};
use super::physics::{EnvelopePoint, interpolate_step_speed, interpolate_step_time, intersect_step_with_speed, intersect_steps};
use crate::units::{CmpOperator, SearchableEnvelope, are_speeds_equal, dir_compare};

// ── EnvelopePartBuilder ─────────────────────────────────────────────────

/// Collects the points of one envelope part, in the order they are
/// simulated; a part simulated backwards is reversed when built.
#[derive(Clone, Debug)]
pub struct EnvelopePartBuilder {
    positions: Vec<f64>,
    speeds: Vec<f64>,
    times: Vec<f64>,
    attrs: Attrs,
    direction: f64,
    last_pos: f64,
    last_speed: f64,
}

impl Default for EnvelopePartBuilder {
    fn default() -> Self {
        EnvelopePartBuilder {
            positions: Vec::new(),
            speeds: Vec::new(),
            times: Vec::new(),
            attrs: Attrs::default(),
            direction: f64::NAN,
            last_pos: f64::NAN,
            last_speed: f64::NAN,
        }
    }
}

impl EnvelopePartBuilder {
    pub fn new() -> Self {
        EnvelopePartBuilder::default()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.len() < 2
    }

    fn add_point(&mut self, position: f64, speed: f64) {
        debug_assert!(!position.is_nan());
        debug_assert!(!speed.is_nan());
        self.positions.push(position);
        self.speeds.push(speed);
        self.last_pos = position;
        self.last_speed = speed;
    }

    pub fn init_envelope_part(&mut self, position: f64, speed: f64, direction: f64) {
        debug_assert!(!direction.is_nan());
        debug_assert!(self.is_empty());
        self.add_point(position, speed);
        self.direction = direction;
    }

    /// Add a point to the envelope part, computing the step time.
    pub fn add_step(&mut self, position: f64, speed: f64) {
        let step_time = interpolate_step_time(self.last_pos, position, self.last_speed, speed);
        self.add_step_with_time(position, speed, step_time);
    }

    /// Add a point to the envelope part.
    pub fn add_step_with_time(&mut self, position: f64, speed: f64, time_delta: f64) {
        debug_assert!(!position.is_nan());
        debug_assert!(!speed.is_nan());
        debug_assert!(!time_delta.is_nan());
        debug_assert!(self.times.len() + 1 == self.positions.len());
        if position == self.last_pos && speed == self.last_speed {
            debug_assert!(time_delta == 0.0);
            return;
        }
        self.add_point(position, speed);
        self.times.push(time_delta);
    }

    pub fn set_profile(&mut self, profile: EnvelopeProfile) {
        self.attrs.profile = Some(profile);
    }

    pub fn set_attrs(&mut self, attrs: &Attrs) {
        self.attrs.merge(attrs);
    }

    /// Creates an envelope part.
    pub fn build(&self) -> EnvelopePart {
        let mut positions = self.positions.clone();
        let mut speeds = self.speeds.clone();
        let mut times = self.times.clone();
        if self.direction < 0.0 {
            positions.reverse();
            speeds.reverse();
            times.reverse();
        }
        EnvelopePart::new(self.attrs, positions, speeds, times)
    }

    /// Return the number of steps currently in the builder.
    pub fn step_count(&self) -> usize {
        self.times.len()
    }
}

// ── constraints ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintType {
    Floor,
    Ceiling,
    Equal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NextPointKind {
    /// The base has a point at this position.
    BasePoint,
    /// The overlay has a point at this position.
    OverlayPoint,
    /// Both the base and the overlay curves have a point at this position.
    BothPoints,
}

struct NextPoint {
    kind: NextPointKind,
    point_position: f64,
    base_speed: f64,
    overlay_speed: f64,
}

/// A limit a simulated envelope part must keep: a range of positions, a
/// speed, or another envelope.
#[derive(Clone, Debug)]
pub enum Constraint<'a> {
    Position {
        range_begin: f64,
        range_end: f64,
    },
    Speed {
        speed: f64,
        kind: ConstraintType,
    },
    Envelope {
        envelope: &'a Envelope,
        kind: ConstraintType,
        cursor: Option<EnvelopeCursor<'a>>,
    },
}

impl<'a> Constraint<'a> {
    pub fn position(range_begin: f64, range_end: f64) -> Self {
        Constraint::Position { range_begin, range_end }
    }

    pub fn speed(speed: f64, kind: ConstraintType) -> Self {
        Constraint::Speed { speed, kind }
    }

    pub fn envelope(envelope: &'a Envelope, kind: ConstraintType) -> Self {
        Constraint::Envelope { envelope, kind, cursor: None }
    }

    /// Returns whether the first point of an envelope part satisfies a constraint.
    pub fn init_check(&mut self, position: f64, speed: f64, direction: f64) -> bool {
        match self {
            Constraint::Position { range_begin, range_end } => position >= *range_begin && position <= *range_end,
            Constraint::Speed { speed: limit, kind } => match kind {
                ConstraintType::Ceiling => speed <= *limit,
                ConstraintType::Floor => speed >= *limit,
                ConstraintType::Equal => speed == *limit,
            },
            Constraint::Envelope { envelope, kind, cursor } => {
                let part_index = envelope.find_right_dir(position, direction);
                // if the position is off the envelope, fail
                if part_index < 0 {
                    return false;
                }
                let part = envelope.get(part_index as usize);
                let step_index = part.find_right_dir(position, direction);
                let envelope_speed = part.interpolate_speed_in_step(step_index as usize, position);
                *cursor = Some(EnvelopeCursor::at(
                    envelope,
                    direction < 0.0,
                    part_index as usize,
                    step_index as usize,
                    position,
                ));
                match kind {
                    ConstraintType::Ceiling => envelope_speed > speed || are_speeds_equal(envelope_speed, speed),
                    ConstraintType::Floor => envelope_speed < speed || are_speeds_equal(envelope_speed, speed),
                    ConstraintType::Equal => are_speeds_equal(envelope_speed, speed),
                }
            }
        }
    }

    /// Returns whether the constraint sets a minimum value on the simulation speed.
    pub fn is_floor(&self) -> bool {
        match self {
            Constraint::Position { .. } => false,
            Constraint::Speed { kind, .. } | Constraint::Envelope { kind, .. } => *kind != ConstraintType::Ceiling,
        }
    }

    /// Returns whether the constraint sets a position limit.
    pub fn has_position_constraint(&self) -> bool {
        !matches!(self, Constraint::Speed { .. })
    }

    /// Intersects a segment (start excluded, end included) with this constraint.
    pub fn step_check(&mut self, start_pos: f64, start_speed: f64, end_pos: f64, end_speed: f64) -> Option<EnvelopePoint> {
        match self {
            Constraint::Position { range_begin, range_end } => {
                let (range_begin, range_end) = (*range_begin, *range_end);
                if end_pos > range_begin && end_pos < range_end {
                    return None;
                }
                if end_pos == range_begin || end_pos == range_end {
                    return Some(EnvelopePoint::new(end_pos, end_speed));
                }
                let inter_pos = if end_pos < range_begin { range_begin } else { range_end };
                let offset = inter_pos - start_pos;
                let speed = interpolate_step_speed(start_pos, end_pos, start_speed, end_speed, offset);
                Some(EnvelopePoint::new(inter_pos, speed))
            }
            Constraint::Speed { speed, kind } => {
                match kind {
                    ConstraintType::Ceiling if end_speed < *speed => return None,
                    ConstraintType::Floor if end_speed > *speed => return None,
                    ConstraintType::Equal if end_speed == *speed => return None,
                    _ => {}
                }
                let inter_position = intersect_step_with_speed(start_pos, start_speed, end_pos, end_speed, *speed);
                Some(EnvelopePoint::new(inter_position, *speed))
            }
            Constraint::Envelope { kind, cursor, .. } => {
                let cursor = cursor.as_mut().expect("init_check sets the cursor");
                envelope_step_check(*kind, cursor, start_pos, start_speed, end_pos, end_speed)
            }
        }
    }
}

/// Given the next point of the overlay, finds the next point after the
/// cursor, which could either be part of the base curve or the overlay curve.
fn next_point(cursor: &EnvelopeCursor, last_overlay_pos: f64, last_overlay_speed: f64, overlay_pos: f64, overlay_speed: f64) -> NextPoint {
    // either the overlay step ends first, and interpolation needs to be
    // performed on the base curve side, or the base curve step ends first,
    // and interpolation has to be performed on the overlay step
    let base_step_end = cursor.step_end_pos();
    let delta = cursor.compare_pos(overlay_pos, base_step_end);
    if delta == 0.0 {
        NextPoint {
            kind: NextPointKind::BothPoints,
            point_position: overlay_pos,
            base_speed: cursor.step_end_speed(),
            overlay_speed,
        }
    } else if delta < 0.0 {
        // the overlay point comes first, interpolate on the base
        let base_begin_pos = cursor.step_begin_pos();
        let base_speed = interpolate_step_speed(
            base_begin_pos,
            cursor.step_end_pos(),
            cursor.step_begin_speed(),
            cursor.step_end_speed(),
            overlay_pos - base_begin_pos,
        );
        NextPoint {
            kind: NextPointKind::OverlayPoint,
            point_position: overlay_pos,
            base_speed,
            overlay_speed,
        }
    } else {
        // the base point comes first, interpolate on the overlay
        let overlay_speed = interpolate_step_speed(
            last_overlay_pos,
            overlay_pos,
            last_overlay_speed,
            overlay_speed,
            base_step_end - last_overlay_pos,
        );
        NextPoint {
            kind: NextPointKind::BasePoint,
            point_position: base_step_end,
            base_speed: cursor.step_end_speed(),
            overlay_speed,
        }
    }
}

/// Intersects the base curve with the overlay curve: takes the overlay
/// step, and returns the intersection point if any.
fn envelope_intersect(kind: ConstraintType, cursor: &EnvelopeCursor, last_pos: f64, last_speed: f64, position: f64, speed: f64) -> Option<EnvelopePoint> {
    // if the speed ranges do not even intersect, there is no intersection
    let base_min = cursor.step_end_speed().min(cursor.step_begin_speed());
    let base_max = cursor.step_end_speed().max(cursor.step_begin_speed());
    let overlay_min = last_speed.min(speed);
    let overlay_max = last_speed.max(speed);
    if kind == ConstraintType::Ceiling && overlay_max < base_min {
        return None;
    }
    if kind == ConstraintType::Floor && overlay_min > base_max {
        return None;
    }
    // look for the next point by position, and interpolate both curves to find the minimum
    let curve_event = next_point(cursor, last_pos, last_speed, position, speed);
    let speed_delta = curve_event.overlay_speed - curve_event.base_speed;
    // if the constraint is a ceiling and the overlay is still the minimum, it's all good
    if kind == ConstraintType::Ceiling && speed_delta < 0.0 {
        return None;
    }
    // if the constraint is a floor and the overlay is still the maximum, it's all good
    if kind == ConstraintType::Floor && speed_delta > 0.0 {
        return None;
    }
    // if the curves intersect exactly at the next point, use some simplifications
    if speed_delta.abs() < 1e-8 {
        if curve_event.kind != NextPointKind::BasePoint {
            return Some(EnvelopePoint::new(position, speed));
        }
        return Some(EnvelopePoint::new(curve_event.point_position, curve_event.overlay_speed));
    }
    // otherwise, find the intersection point the hard way
    Some(intersect_steps(
        last_pos,
        last_speed,
        position,
        speed,
        cursor.step_begin_pos(),
        cursor.step_begin_speed(),
        cursor.step_end_pos(),
        cursor.step_end_speed(),
    ))
}

fn handle_new_part(
    kind: ConstraintType,
    cursor: &EnvelopeCursor,
    last_overlay_pos: f64,
    last_overlay_speed: f64,
    position: f64,
    speed: f64,
) -> Option<EnvelopePoint> {
    let part_start = cursor.step_begin_pos();
    let part_start_speed = cursor.step_begin_speed();
    let overlay_speed = interpolate_step_speed(last_overlay_pos, position, last_overlay_speed, speed, part_start - last_overlay_pos);
    if kind == ConstraintType::Ceiling && part_start_speed > overlay_speed {
        return None;
    }
    if kind == ConstraintType::Floor && part_start_speed < overlay_speed {
        return None;
    }
    Some(EnvelopePoint::new(part_start, overlay_speed))
}

fn envelope_step_check(
    kind: ConstraintType,
    cursor: &mut EnvelopeCursor,
    last_overlay_pos: f64,
    last_overlay_speed: f64,
    position: f64,
    speed: f64,
) -> Option<EnvelopePoint> {
    while cursor.compare_pos(position, cursor.step_begin_pos()) > 0.0 {
        // attempt to find an intersection
        if let Some(inter) = envelope_intersect(kind, cursor, last_overlay_pos, last_overlay_speed, position, speed) {
            return Some(inter);
        }
        let step_end_pos = cursor.step_end_pos();
        if cursor.compare_pos(position, step_end_pos) < 0.0 {
            break;
        }
        let step_res = cursor.next_step();
        // if stepping resulted in switching to a new base part, check if the
        // first point of the envelope part starts over the overlay
        if step_res == NextStepResult::NextPart
            && let Some(inter) = handle_new_part(kind, cursor, last_overlay_pos, last_overlay_speed, position, speed)
        {
            return Some(inter);
        }
        // if the overlay step ends after the curve, cut it
        if step_res == NextStepResult::NextReachedEnd {
            let inter_speed = interpolate_step_speed(last_overlay_pos, position, last_overlay_speed, speed, step_end_pos - last_overlay_pos);
            return Some(EnvelopePoint::new(step_end_pos, inter_speed));
        }
    }
    // no intersection with the base curve was found
    None
}

// ── ConstrainedEnvelopePartBuilder ──────────────────────────────────────

/// Takes the steps of a simulation and cuts it at the first constraint it
/// meets; the steps go to its own part builder, `sink`.
pub struct ConstrainedEnvelopePartBuilder<'a> {
    pub constraints: Vec<Constraint<'a>>,
    pub sink: EnvelopePartBuilder,
    /// The index of the constraint met last, or -1.
    pub last_intersection: isize,
    last_pos: f64,
    last_speed: f64,
    direction: f64,
}

impl<'a> ConstrainedEnvelopePartBuilder<'a> {
    pub fn new(sink: EnvelopePartBuilder, constraints: Vec<Constraint<'a>>) -> Self {
        ConstrainedEnvelopePartBuilder {
            constraints,
            sink,
            last_intersection: -1,
            last_pos: f64::NAN,
            last_speed: f64::NAN,
            direction: f64::NAN,
        }
    }

    /// A builder over a new part of the given profile.
    pub fn with_profile(profile: EnvelopeProfile, constraints: Vec<Constraint<'a>>) -> Self {
        let mut sink = EnvelopePartBuilder::new();
        sink.set_profile(profile);
        ConstrainedEnvelopePartBuilder::new(sink, constraints)
    }

    pub fn had_intersection(&self) -> bool {
        self.last_intersection != -1
    }

    pub fn last_pos(&self) -> f64 {
        self.last_pos
    }

    pub fn last_speed(&self) -> f64 {
        self.last_speed
    }

    fn intersect(&mut self, position: f64, speed: f64) -> Option<EnvelopePoint> {
        // check if this step intersects with any constraint
        let mut next_inter: Option<EnvelopePoint> = None;
        let mut next_inter_constraint: isize = -1;
        let (last_pos, last_speed, direction) = (self.last_pos, self.last_speed, self.direction);
        for (i, constraint) in self.constraints.iter_mut().enumerate() {
            let Some(cur_inter) = constraint.step_check(last_pos, last_speed, position, speed) else {
                continue;
            };
            // if this is the first intersection, or it is closer than the previous one, keep it
            if let Some(prev) = next_inter
                && dir_compare(direction, cur_inter.position, prev.position) >= 0.0
            {
                continue;
            }
            next_inter = Some(cur_inter);
            next_inter_constraint = i as isize;
        }
        self.last_intersection = next_inter_constraint;
        next_inter
    }

    /// Initializes the envelope part, if its first point keeps every constraint.
    pub fn init_envelope_part(&mut self, position: f64, speed: f64, direction: f64) -> bool {
        for constraint in self.constraints.iter_mut() {
            if !constraint.init_check(position, speed, direction) {
                return false;
            }
        }
        self.last_pos = position;
        self.last_speed = speed;
        self.direction = direction;
        self.sink.init_envelope_part(position, speed, direction);
        true
    }

    /// Adds a step, computing its time. Returns whether more steps are accepted.
    pub fn add_step(&mut self, position: f64, speed: f64) -> bool {
        debug_assert!(!self.had_intersection());
        let intersection = self.intersect(position, speed);
        let (position, speed) = match intersection {
            Some(p) => (p.position, p.speed),
            None => (position, speed),
        };
        self.last_pos = position;
        self.last_speed = speed;
        self.sink.add_step(position, speed);
        intersection.is_none()
    }

    /// Adds a step. Returns whether more steps are accepted.
    pub fn add_step_with_time(&mut self, position: f64, speed: f64, time_delta: f64) -> bool {
        debug_assert!(!self.had_intersection());
        let intersection = self.intersect(position, speed);
        let (mut position, mut speed, mut time_delta) = (position, speed, time_delta);
        if let Some(p) = intersection {
            position = p.position;
            speed = p.speed;
            if position == self.last_pos {
                debug_assert!(are_speeds_equal(speed, self.last_speed));
                speed = self.last_speed;
                time_delta = 0.0;
            } else {
                time_delta = interpolate_step_time(self.last_pos, position, self.last_speed, speed);
            }
        }
        self.last_pos = position;
        self.last_speed = speed;
        self.sink.add_step_with_time(position, speed, time_delta);
        intersection.is_none()
    }

    /// Asserts on the constraints, to make sure there's no risk of infinite loops.
    pub fn check_constraints(&self) {
        debug_assert!(
            self.constraints.iter().any(|c| c.is_floor()),
            "Simulation without speed floor can cause infinite loops at speed=0"
        );
        debug_assert!(
            self.constraints.iter().any(|c| c.has_position_constraint()),
            "Simulations without position constraints can cause infinite loops"
        );
    }
}

// ── OverlayEnvelopeBuilder ──────────────────────────────────────────────

/// Creates an overlay over an envelope by combining slices of the base
/// envelope and overlay envelope parts.
pub struct OverlayEnvelopeBuilder<'a> {
    pub base: &'a Envelope,
    reverse: bool,
    overlay_parts: VecDeque<EnvelopePart>,
}

impl<'a> OverlayEnvelopeBuilder<'a> {
    pub fn with_direction(base: &'a Envelope, reverse: bool) -> Self {
        OverlayEnvelopeBuilder {
            base,
            reverse,
            overlay_parts: VecDeque::new(),
        }
    }

    pub fn forward(base: &'a Envelope) -> Self {
        Self::with_direction(base, false)
    }

    pub fn backward(base: &'a Envelope) -> Self {
        Self::with_direction(base, true)
    }

    /// Adds an overlay envelope part to the builder.
    pub fn add_part(&mut self, part: EnvelopePart) {
        if self.reverse {
            debug_assert!(self.overlay_parts.front().is_none_or(|first| part.end_pos() <= first.begin_pos()));
            self.overlay_parts.push_front(part);
        } else {
            debug_assert!(self.overlay_parts.back().is_none_or(|last| last.end_pos() <= part.begin_pos()));
            self.overlay_parts.push_back(part);
        }
    }

    /// Slice the base curve between the end of the previous overlay and the
    /// beginning of the current one.
    fn slice_base(&self, previous_overlay: Option<&EnvelopePart>, current_overlay: Option<&EnvelopePart>) -> Vec<EnvelopePart> {
        let base = self.base;
        let mut slice_begin_pos = f64::NEG_INFINITY;
        let mut slice_begin_speed = f64::NAN;
        if let Some(previous) = previous_overlay {
            slice_begin_pos = previous.end_pos();
            let part_index = base.find_right(slice_begin_pos);
            let base_speed = base.get(part_index as usize).interpolate_speed(slice_begin_pos);
            if (base_speed - previous.end_speed()).abs() < 1e-6 {
                slice_begin_speed = previous.end_speed();
            }
        }
        let mut slice_end_pos = f64::INFINITY;
        let mut slice_end_speed = f64::NAN;
        if let Some(current) = current_overlay {
            slice_end_pos = current.begin_pos();
            let part_index = base.find_left(slice_end_pos);
            let base_speed = base.get(part_index as usize).interpolate_speed(slice_end_pos);
            if (base_speed - current.begin_speed()).abs() < 1e-6 {
                slice_end_speed = current.begin_speed();
            }
        }
        if slice_begin_pos == slice_end_pos {
            return Vec::new(); // Prevents 0-length parts
        }
        base.slice_with_speeds(slice_begin_pos, slice_begin_speed, slice_end_pos, slice_end_speed)
    }

    /// Create the envelope.
    pub fn build(self) -> Envelope {
        if self.overlay_parts.is_empty() {
            return self.base.clone();
        }
        let mut builder = EnvelopeBuilder::new();
        let mut previous_overlay: Option<&EnvelopePart> = None;
        for overlay_part in &self.overlay_parts {
            builder.add_parts(self.slice_base(previous_overlay, Some(overlay_part)));
            builder.add_part(overlay_part.clone());
            previous_overlay = Some(overlay_part);
        }
        builder.add_parts(self.slice_base(previous_overlay, None));
        builder.build()
    }
}

// ── MRSPEnvelopeBuilder ─────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EnvelopeChangeType {
    Begin = 0,
    End = 1,
}

struct EnvelopeChange {
    position: f64,
    part: usize,
    kind: EnvelopeChangeType,
}

/// Given a set of overlapping constant speed parts, finds the minimum speed envelope.
#[derive(Default)]
pub struct MrspEnvelopeBuilder {
    parts: Vec<EnvelopePart>,
    changes: Vec<EnvelopeChange>,
}

impl MrspEnvelopeBuilder {
    pub fn new() -> Self {
        MrspEnvelopeBuilder::default()
    }

    /// Add an envelope part to the envelope.
    pub fn add_part(&mut self, part: EnvelopePart) -> &mut Self {
        debug_assert!(part.step_count() == 1);
        debug_assert!(part.min_speed() == part.max_speed());
        let index = self.parts.len();
        self.changes.push(EnvelopeChange {
            position: part.begin_pos(),
            part: index,
            kind: EnvelopeChangeType::Begin,
        });
        self.changes.push(EnvelopeChange {
            position: part.end_pos(),
            part: index,
            kind: EnvelopeChangeType::End,
        });
        self.parts.push(part);
        self
    }

    /// Build the envelope from all the given parts.
    pub fn build(mut self) -> Envelope {
        self.changes.sort_by(|a, b| a.position.total_cmp(&b.position).then(a.kind.cmp(&b.kind)));
        let mut state = MrspState::default();
        let mut change_index = 0;
        while change_index < self.changes.len() {
            let current_position = self.changes[change_index].position;
            // process curve ends
            while change_index < self.changes.len() {
                let change = &self.changes[change_index];
                if change.kind != EnvelopeChangeType::End || change.position > current_position {
                    break;
                }
                state.active_parts.retain(|p| *p != change.part);
                // if the current min part ends, finalize it in the output as well
                if Some(change.part) == state.current_min_part {
                    state.current_part_builder.as_mut().unwrap().add_step(current_position, state.current_min_speed);
                    state.flush_result_part();
                    state.current_min_part = None;
                }
                change_index += 1;
            }
            // process curve starts
            while change_index < self.changes.len() {
                let change = &self.changes[change_index];
                if change.kind != EnvelopeChangeType::Begin || change.position > current_position {
                    break;
                }
                state.active_parts.push(change.part);
                change_index += 1;
            }
            state.update_min_part(&self.parts, current_position);
        }
        Envelope::new(state.build_result)
    }
}

struct MrspState {
    current_min_part: Option<usize>,
    current_min_speed: f64,
    build_result: Vec<EnvelopePart>,
    current_part_builder: Option<EnvelopePartBuilder>,
    active_parts: Vec<usize>,
}

impl Default for MrspState {
    fn default() -> Self {
        MrspState {
            current_min_part: None,
            current_min_speed: f64::INFINITY,
            build_result: Vec::new(),
            current_part_builder: None,
            active_parts: Vec::new(),
        }
    }
}

impl MrspState {
    fn flush_result_part(&mut self) {
        if let Some(builder) = self.current_part_builder.take()
            && !builder.is_empty()
        {
            self.build_result.push(builder.build());
        }
    }

    fn new_result_part(&mut self, new_min_part: &EnvelopePart, initial_position: f64, initial_speed: f64) {
        self.flush_result_part();
        let mut builder = EnvelopePartBuilder::new();
        builder.init_envelope_part(initial_position, initial_speed, 1.0);
        builder.set_attrs(new_min_part.attrs());
        self.current_part_builder = Some(builder);
    }

    fn update_min_part(&mut self, parts: &[EnvelopePart], current_position: f64) {
        if self.active_parts.is_empty() {
            debug_assert!(self.current_min_part.is_none());
            return;
        }
        // find the minimum curve
        let old_min_speed = self.current_min_speed;
        let old_min_part = self.current_min_part;
        self.current_min_speed = f64::INFINITY;
        for &index in &self.active_parts {
            let env_speed = parts[index].min_speed();
            if env_speed >= self.current_min_speed {
                continue;
            }
            self.current_min_speed = env_speed;
            self.current_min_part = Some(index);
        }
        if old_min_part == self.current_min_part {
            return;
        }
        // terminate the old min part if a new one starts below it
        if old_min_part.is_some() {
            self.current_part_builder.as_mut().unwrap().add_step(current_position, old_min_speed);
        }
        let new_min = self.current_min_part.unwrap();
        self.new_result_part(&parts[new_min], current_position, self.current_min_speed);
    }
}

/// A part of constant speed over a range, as the MRSP is made of.
pub fn constant_speed_part(attrs: Attrs, begin: f64, end: f64, speed: f64) -> EnvelopePart {
    let mut attrs = attrs;
    attrs.profile = Some(EnvelopeProfile::ConstantSpeed);
    EnvelopePart::generate_times(attrs, vec![begin, end], vec![speed, speed])
}

/// The attributes of a part of the MRSP from a given kind of limit.
pub fn limit_attrs(limit: LimitKind, source: Option<u32>) -> Attrs {
    Attrs {
        profile: Some(EnvelopeProfile::ConstantSpeed),
        limit: Some(limit),
        source,
        ..Attrs::default()
    }
}

// ── EnvelopeSpeedCap ────────────────────────────────────────────────────

/// Adds a global speed limit to an envelope.
pub fn speed_cap(base: &Envelope, attrs: &Attrs, speed_limit: f64) -> Envelope {
    let mut cursor = EnvelopeCursor::new(base, false);
    let mut builder = OverlayEnvelopeBuilder::forward(base);
    let mut cap_attrs = *attrs;
    cap_attrs.profile = Some(EnvelopeProfile::ConstantSpeed);
    while cursor.find_speed(speed_limit, CmpOperator::StrictlyHigher) {
        let start_pos = cursor.position();
        let mut part_builder = EnvelopePartBuilder::new();
        part_builder.set_attrs(&cap_attrs);
        part_builder.init_envelope_part(start_pos, speed_limit, 1.0);
        let has_not_reached_end = cursor.find_speed(speed_limit, CmpOperator::StrictlyLower);
        debug_assert!(has_not_reached_end != cursor.has_reached_end());
        let end_position = if has_not_reached_end { cursor.position() } else { cursor.envelope_end_pos() };
        let plateau_length = (end_position - start_pos).abs();
        let plateau_duration = plateau_length / speed_limit;
        part_builder.add_step_with_time(end_position, speed_limit, plateau_duration);
        cursor.find_position(end_position);
        builder.add_part(part_builder.build());
    }
    let res = builder.build();
    debug_assert!(!base.continuous || res.continuous);
    res
}
