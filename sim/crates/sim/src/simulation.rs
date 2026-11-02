// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// Ported from OSRD core/src/.../standalone_sim/StandaloneSimulation.kt
// (runStandaloneSimulation, buildProvisionalEnvelope, buildFinalEnvelope,
// distributeAllowance), standalone_sim/EnvelopeStopWrapper.java and
// envelope/EnvelopePhysics.kt (the mechanical energy); the request and the
// report are this crate's own.

//! One train over one path: the fastest run, the run with the standard
//! allowance (provisional) and the run that keeps the scheduled times
//! (final), as OSRD's standalone simulation.

use serde::{Deserialize, Serialize};

use crate::SimError;
use crate::allowances::{Allowance, AllowanceRange, AllowanceValue, Distribution};
use crate::envelope::{Envelope, EnvelopeProfile, LimitKind};
use crate::physics::integrator::GRAVITY_ACCELERATION;
use crate::physics::{EnvelopeSimContext, RollingStock, SimPath, TractiveEffortPoint};
use crate::pipelines::{SpeedLimit, compute_mrsp, max_effort_envelope_from, max_speed_envelope_from};
use crate::units::are_positions_equal;

// ── the request ─────────────────────────────────────────────────────────

/// A value over a range of the path.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeValue {
    pub begin: f64,
    pub end: f64,
    pub value: f64,
}

/// A speed limit over a range of the path, in m/s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeedLimitInput {
    pub begin: f64,
    pub end: f64,
    pub speed: f64,
    /// What sets it — passed through to the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// The path, as the physics needs it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathInput {
    /// in m
    pub length: f64,
    /// Grades in m/km over ranges, the resistance of curves counted in.
    /// Where none is given the path is flat.
    #[serde(default)]
    pub gradients: Vec<RangeValue>,
    /// The speed limits of the infrastructure.
    #[serde(default)]
    pub speed_limits: Vec<SpeedLimitInput>,
}

/// The resistance to movement: A + B·v + C·v², v in m/s.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RollingResistance {
    #[serde(rename = "A")]
    pub a: f64,
    #[serde(rename = "B")]
    pub b: f64,
    #[serde(rename = "C")]
    pub c: f64,
}

/// A train's physics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RollingStockInput {
    /// in kg
    pub mass: f64,
    /// in m
    pub length: f64,
    /// in m/s
    pub max_speed: f64,
    #[serde(default = "default_inertia_coefficient")]
    pub inertia_coefficient: f64,
    pub rolling_resistance: RollingResistance,
    /// The constant braking deceleration, in m/s² (positive).
    pub const_gamma: f64,
    /// The tractive effort over speed, by increasing speed.
    pub effort_curve: Vec<TractiveEffortPoint>,
}

fn default_inertia_coefficient() -> f64 {
    1.05
}

/// A point of the schedule: a stop, a time to arrive at, or both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduleItem {
    /// in m from the start of the path
    pub path_offset: f64,
    /// The time to arrive at, in s from the departure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival: Option<f64>,
    /// The train stops here, for so many seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_duration: Option<f64>,
}

/// A margin over a range: `"5%"`, `"4.5min/100km"`, or `"none"`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Margins {
    /// The positions between the ranges.
    #[serde(default)]
    pub boundaries: Vec<f64>,
    /// One value per range — one more than the boundaries.
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionInput {
    /// Speeds scaled linearly.
    Linear,
    /// Energy saving (MARECO), as OSRD's standard.
    #[default]
    Mareco,
}

/// One train to simulate on one path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub path: PathInput,
    pub rolling_stock: RollingStockInput,
    #[serde(default)]
    pub schedule: Vec<ScheduleItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<Margins>,
    #[serde(default)]
    pub distribution: DistributionInput,
    /// in m/s
    #[serde(default)]
    pub initial_speed: f64,
    /// The integration time step, in s.
    #[serde(default = "default_time_step")]
    pub time_step: f64,
}

fn default_time_step() -> f64 {
    2.0
}

// ── the report ──────────────────────────────────────────────────────────

/// A run of the train: its points, with the stops' durations counted in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// in m
    pub positions: Vec<f64>,
    /// in s from the departure
    pub times: Vec<f64>,
    /// in m/s
    pub speeds: Vec<f64>,
    /// The arrival and departure at every point of the schedule.
    pub scheduled_points: Vec<ScheduledPoint>,
    /// What the train does over each range: accelerating, braking …
    pub profiles: Vec<ProfileRange>,
    /// The mechanical energy consumed at the wheels, in J.
    pub energy_consumption: f64,
    /// The running time including stops, in s.
    pub total_time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduledPoint {
    pub path_offset: f64,
    pub arrival: f64,
    pub departure: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfileRange {
    pub begin: f64,
    pub end: f64,
    pub profile: EnvelopeProfile,
}

/// A range of the most restrictive speed profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MrspRange {
    pub begin: f64,
    pub end: f64,
    /// in m/s
    pub speed: f64,
    /// The source of the limit, or `train` where the train's own max speed limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// The result of a simulation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationResult {
    /// The fastest run.
    pub base: Report,
    /// With the standard margins.
    pub provisional: Report,
    /// Keeping the scheduled times as well.
    #[serde(rename = "final")]
    pub final_output: Report,
    /// The speed limits along the path, without the train's length.
    pub mrsp: Vec<MrspRange>,
    pub warnings: Vec<String>,
}

// ── the simulation ──────────────────────────────────────────────────────

/// Run a simulation for a single train.
pub fn run_simulation(request: &SimulationRequest) -> Result<SimulationResult, SimError> {
    let path_length = request.path.length;
    if path_length.is_nan() || path_length <= 0.0 {
        return Err(SimError::InvalidInput("the path has no length".into()));
    }
    if request.time_step.is_nan() || request.time_step <= 0.0 {
        return Err(SimError::InvalidInput("the time step must be positive".into()));
    }
    let mut warnings = Vec::new();
    let rolling_stock = rolling_stock_from(&request.rolling_stock)?;
    let path = path_from(&request.path)?;
    let schedule = dedup_schedule_items(&request.schedule, path_length);

    // MRSP & speed limits
    let sources: Vec<Option<String>> = request.path.speed_limits.iter().map(|l| l.source.clone()).collect();
    let speed_limits: Vec<SpeedLimit> = request
        .path
        .speed_limits
        .iter()
        .enumerate()
        .map(|(i, l)| SpeedLimit {
            begin: l.begin,
            end: l.end,
            speed: l.speed,
            source: Some(i as u32),
        })
        .collect();
    let mrsp = compute_mrsp(path_length, rolling_stock.max_speed, rolling_stock.length, true, &speed_limits);
    // the MRSP shown does not add the train's length
    let shown_mrsp = compute_mrsp(path_length, rolling_stock.max_speed, rolling_stock.length, false, &speed_limits);

    let context = EnvelopeSimContext::new(rolling_stock, path, request.time_step);

    // Max speed envelope
    let stops: Vec<f64> = schedule.iter().filter(|s| s.stop_duration.is_some()).map(|s| s.path_offset).collect();
    let max_speed_envelope = max_speed_envelope_from(&context, &stops, &mrsp)?;
    // Max effort envelope: the train goes as fast as possible
    let max_effort_envelope = max_effort_envelope_from(&context, request.initial_speed, &max_speed_envelope)?;

    // Provisional envelope: the train matches the standard allowances
    let distribution = match request.distribution {
        DistributionInput::Linear => Distribution::Linear,
        DistributionInput::Mareco => Distribution::Mareco,
    };
    let margins = parse_margins(request.margins.as_ref(), path_length)?;
    let provisional_envelope = if margins.is_empty() {
        max_effort_envelope.clone()
    } else {
        build_provisional_envelope(&max_effort_envelope, &context, &margins, distribution)?
    };
    // Final envelope: the train matches the standard allowances and given scheduled points
    let final_envelope = build_final_envelope(
        &max_effort_envelope,
        &provisional_envelope,
        &context,
        &margins,
        distribution,
        &schedule,
        &mut warnings,
    )?;

    Ok(SimulationResult {
        base: make_report(&max_effort_envelope, &context, &schedule),
        provisional: make_report(&provisional_envelope, &context, &schedule),
        final_output: make_report(&final_envelope, &context, &schedule),
        mrsp: shown_mrsp
            .iter()
            .map(|part| MrspRange {
                begin: part.begin_pos(),
                end: part.end_pos(),
                speed: part.begin_speed(),
                source: match part.attrs().limit {
                    Some(LimitKind::TrainLimit) => Some("train".into()),
                    _ => part.attrs().source.and_then(|i| sources.get(i as usize).cloned().flatten()),
                },
            })
            .collect(),
        warnings,
    })
}

fn rolling_stock_from(input: &RollingStockInput) -> Result<RollingStock, SimError> {
    let positive = |v: f64| v > 0.0;
    if !(positive(input.mass) && input.length >= 0.0 && positive(input.max_speed) && positive(input.const_gamma) && positive(input.inertia_coefficient)) {
        return Err(SimError::InvalidInput(
            "the rolling stock needs a mass, a max speed and a deceleration above zero".into(),
        ));
    }
    if input.effort_curve.is_empty() || input.effort_curve.windows(2).any(|w| w[0].speed >= w[1].speed) {
        return Err(SimError::InvalidInput("the effort curve needs points by increasing speed".into()));
    }
    let r = input.rolling_resistance;
    Ok(RollingStock::new(
        input.mass,
        input.inertia_coefficient,
        input.length,
        input.max_speed,
        r.a,
        r.b,
        r.c,
        input.const_gamma,
        input.effort_curve.clone(),
    ))
}

/// The grades as positions and values over the whole path: the ranges
/// given, flat between them.
fn path_from(input: &PathInput) -> Result<SimPath, SimError> {
    let length = input.length;
    let mut ranges: Vec<RangeValue> = input
        .gradients
        .iter()
        .map(|g| RangeValue {
            begin: g.begin.max(0.0),
            end: g.end.min(length),
            value: g.value,
        })
        .filter(|g| g.begin < g.end)
        .collect();
    ranges.sort_by(|a, b| a.begin.total_cmp(&b.begin));
    let mut positions = vec![0.0];
    let mut values = Vec::new();
    let mut at = 0.0;
    for g in ranges {
        if g.begin < at {
            return Err(SimError::InvalidInput(format!("gradients overlap at {} m", g.begin)));
        }
        if g.begin > at {
            positions.push(g.begin);
            values.push(0.0);
        }
        positions.push(g.end);
        values.push(g.value);
        at = g.end;
    }
    if at < length {
        positions.push(length);
        values.push(0.0);
    }
    Ok(SimPath::new(length, positions, values))
}

/// Merges the schedule items at the same place, as OSRD does.
fn dedup_schedule_items(schedule: &[ScheduleItem], path_length: f64) -> Vec<ScheduleItem> {
    let mut items: Vec<ScheduleItem> = schedule
        .iter()
        .map(|s| ScheduleItem {
            path_offset: s.path_offset.clamp(0.0, path_length),
            ..s.clone()
        })
        .collect();
    items.sort_by(|a, b| a.path_offset.total_cmp(&b.path_offset));
    let mut res: Vec<ScheduleItem> = Vec::new();
    for item in items {
        if let Some(last) = res.last_mut()
            && last.path_offset == item.path_offset
        {
            last.arrival = item.arrival.or(last.arrival);
            last.stop_duration = match (last.stop_duration, item.stop_duration) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            continue;
        }
        res.push(item);
    }
    res
}

/// The standard margins as allowance values over ranges covering the path.
fn parse_margins(margins: Option<&Margins>, path_length: f64) -> Result<Vec<AllowanceRange>, SimError> {
    let Some(margins) = margins else { return Ok(Vec::new()) };
    if margins.values.is_empty() {
        return Ok(Vec::new());
    }
    if margins.values.len() != margins.boundaries.len() + 1 {
        return Err(SimError::InvalidInput("margins need one value more than boundaries".into()));
    }
    let mut bounds = vec![0.0];
    bounds.extend(margins.boundaries.iter().copied());
    bounds.push(path_length);
    let mut res = Vec::new();
    for (i, raw) in margins.values.iter().enumerate() {
        let (begin, end) = (bounds[i], bounds[i + 1]);
        if begin >= end {
            continue;
        }
        res.push(AllowanceRange {
            begin_pos: begin,
            end_pos: end,
            value: parse_margin_value(raw)?,
        });
    }
    Ok(res)
}

fn parse_margin_value(raw: &str) -> Result<AllowanceValue, SimError> {
    let raw = raw.trim();
    if raw == "none" {
        return Ok(AllowanceValue::Percentage(0.0));
    }
    let number = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| *v >= 0.0)
            .ok_or_else(|| SimError::InvalidInput(format!("margin value {raw:?}")))
    };
    if let Some(p) = raw.strip_suffix('%') {
        return Ok(AllowanceValue::Percentage(number(p)?));
    }
    if let Some(m) = raw.strip_suffix("min/100km") {
        return Ok(AllowanceValue::TimePerDistance(number(m)?));
    }
    Err(SimError::InvalidInput(format!("margin value {raw:?}: \"5%\", \"4.5min/100km\" or \"none\"")))
}

fn allowance(distribution: Distribution, end_pos: f64, ranges: Vec<AllowanceRange>) -> Result<Allowance, SimError> {
    match distribution {
        Distribution::Mareco => Allowance::mareco(0.0, end_pos, 1.0, ranges),
        Distribution::Linear => Allowance::linear(0.0, end_pos, 0.0, ranges),
    }
}

/// Build the provisional envelope based on the max effort envelope, adding
/// the standard margin ranges.
fn build_provisional_envelope(
    max_effort_envelope: &Envelope,
    context: &EnvelopeSimContext,
    margins: &[AllowanceRange],
    distribution: Distribution,
) -> Result<Envelope, SimError> {
    allowance(distribution, max_effort_envelope.end_pos(), margins.to_vec())?.apply(max_effort_envelope, context)
}

/// Build the final envelope from the max effort / provisional envelopes: the
/// margin ranges are modified to match the scheduled points, the added time
/// distributed over them (see OSRD's design documents, "combining margins
/// and schedule").
fn build_final_envelope(
    max_effort_envelope: &Envelope,
    provisional_envelope: &Envelope,
    context: &EnvelopeSimContext,
    margins: &[AllowanceRange],
    distribution: Distribution,
    scheduled_points: &[ScheduleItem],
    warnings: &mut Vec<String>,
) -> Result<Envelope, SimError> {
    let envelope_time_at = |offset: f64| provisional_envelope.interpolate_departure_from_clamp(offset);
    let max_effort_time_at = |offset: f64| max_effort_envelope.interpolate_departure_from_clamp(offset);
    let margin_boundaries: Vec<f64> = margins.iter().skip(1).map(|r| r.begin_pos).collect();

    let mut prev_fixed_point_offset = 0.0;
    let mut prev_fixed_point_departure_time = 0.0;
    let mut margin_ranges = Vec::new();
    for point in scheduled_points {
        let stop_duration = point.stop_duration.unwrap_or(0.0);
        let Some(arrival) = point.arrival else {
            // No specified arrival time: account for the stop duration and move on
            prev_fixed_point_departure_time += stop_duration;
            continue;
        };
        let section_time = envelope_time_at(point.path_offset) - envelope_time_at(prev_fixed_point_offset);
        let arrival_time = prev_fixed_point_departure_time + section_time;
        let extra_time = arrival - arrival_time;
        if extra_time.abs() < context.time_step {
            // The time diff is already within the margin computation tolerance
            prev_fixed_point_departure_time += stop_duration;
            continue;
        }
        if extra_time >= 0.0 {
            margin_ranges.extend(distribute_allowance(
                max_effort_envelope,
                provisional_envelope,
                extra_time,
                &margin_boundaries,
                prev_fixed_point_offset,
                point.path_offset,
                warnings,
            ));
            prev_fixed_point_departure_time = arrival_time + extra_time + stop_duration;
        } else {
            // Time is *removed* compared to the provisional envelope: as
            // OSRD, the distribution over the margin ranges is ignored, the
            // time is only set for the scheduled point
            let max_effort_section_time = max_effort_time_at(point.path_offset) - max_effort_time_at(prev_fixed_point_offset);
            let earliest_possible_arrival = prev_fixed_point_departure_time + max_effort_section_time;
            let mut max_effort_extra_time = arrival - earliest_possible_arrival;
            if max_effort_extra_time < 0.0 {
                warnings.push(format!(
                    "the arrival at {:.0} m cannot be kept: {:.0} s too early",
                    point.path_offset, -max_effort_extra_time
                ));
                max_effort_extra_time = 0.0;
            } else {
                warnings.push(format!("the arrival at {:.0} m leaves less than the standard margin", point.path_offset));
            }
            margin_ranges.push(AllowanceRange {
                begin_pos: prev_fixed_point_offset,
                end_pos: point.path_offset,
                value: AllowanceValue::FixedTime(max_effort_extra_time),
            });
            prev_fixed_point_departure_time = earliest_possible_arrival + max_effort_extra_time + stop_duration;
        }
        prev_fixed_point_offset = point.path_offset;
    }
    let path_end = max_effort_envelope.end_pos();
    if prev_fixed_point_offset < path_end {
        // the last margin call is based on the max effort envelope: all ranges
        // are covered to keep the standard margin
        margin_ranges.extend(distribute_allowance(
            max_effort_envelope,
            provisional_envelope,
            0.0,
            &margin_boundaries,
            prev_fixed_point_offset,
            path_end,
            warnings,
        ));
    }
    allowance(distribution, max_effort_envelope.end_pos(), margin_ranges)?.apply(max_effort_envelope, context)
}

/// Distributes the extra time in the given path section across the standard
/// margin ranges, proportionally to the time spent in each range (with the
/// margin applied). Returns the margin ranges covering the section.
fn distribute_allowance(
    max_effort_envelope: &Envelope,
    provisional_envelope: &Envelope,
    extra_time: f64,
    margin_boundaries: &[f64],
    start_offset: f64,
    end_offset: f64,
    warnings: &mut Vec<String>,
) -> Vec<AllowanceRange> {
    debug_assert!(start_offset <= end_offset);
    if start_offset == end_offset {
        warnings.push(format!("different scheduled points at the same location ({start_offset:.0} m)"));
        return Vec::new();
    }
    let range_time = |from: f64, to: f64, envelope: &Envelope| envelope.interpolate_departure_from_clamp(to) - envelope.interpolate_departure_from_clamp(from);
    let mut range_ends: Vec<f64> = margin_boundaries.iter().copied().filter(|b| *b > start_offset && *b < end_offset).collect();
    range_ends.push(end_offset);
    let base_total_time = range_time(start_offset, end_offset, provisional_envelope);
    let mut res = Vec::new();
    let mut range_start = start_offset;
    for range_end in range_ends {
        let base_range_time = range_time(range_start, range_end, provisional_envelope);
        let ratio = base_range_time / base_total_time;
        let base_allowance_value = base_range_time - range_time(range_start, range_end, max_effort_envelope);
        res.push(AllowanceRange {
            begin_pos: range_start,
            end_pos: range_end,
            value: AllowanceValue::FixedTime(base_allowance_value + extra_time * ratio),
        });
        range_start = range_end;
    }
    res
}

/// The report of a run: its points with the stops' durations counted in
/// (as OSRD's `EnvelopeStopWrapper`), the times at the scheduled points,
/// the profiles and the energy.
fn make_report(envelope: &Envelope, context: &EnvelopeSimContext, schedule: &[ScheduleItem]) -> Report {
    let stops: Vec<(f64, f64)> = schedule.iter().filter_map(|s| s.stop_duration.map(|d| (s.path_offset, d))).collect();
    let mut positions = Vec::new();
    let mut times = Vec::new();
    let mut speeds = Vec::new();
    let mut sum_previous_stop_duration = 0.0;
    let mut stop_index = 0;
    for point in envelope.iterate_points() {
        let shifted_time = point.time + sum_previous_stop_duration;
        positions.push(point.position);
        times.push(shifted_time);
        speeds.push(point.speed);
        if stop_index < stops.len() && point.position >= stops[stop_index].0 {
            let (stop_position, stop_duration) = stops[stop_index];
            positions.push(stop_position);
            times.push(shifted_time + stop_duration);
            speeds.push(point.speed);
            stop_index += 1;
            sum_previous_stop_duration += stop_duration;
        }
    }
    // the time at a position, a stop there included or not
    let time_at = |position: f64, arrival: bool| {
        let mut stop_time = 0.0;
        for (stop_position, duration) in &stops {
            if are_positions_equal(*stop_position, position) {
                if arrival {
                    break;
                }
            } else if position < *stop_position {
                break;
            }
            stop_time += duration;
        }
        stop_time + envelope.interpolate_departure_from_clamp(position)
    };
    let scheduled_points = schedule
        .iter()
        .map(|s| ScheduledPoint {
            path_offset: s.path_offset,
            arrival: time_at(s.path_offset, true),
            departure: time_at(s.path_offset, false),
        })
        .collect();
    let mut profiles: Vec<ProfileRange> = Vec::new();
    for part in envelope.iter() {
        let profile = part.profile().unwrap_or(EnvelopeProfile::ConstantSpeed);
        match profiles.last_mut() {
            Some(last) if last.profile == profile && last.end == part.begin_pos() => last.end = part.end_pos(),
            _ => profiles.push(ProfileRange {
                begin: part.begin_pos(),
                end: part.end_pos(),
                profile,
            }),
        }
    }
    Report {
        positions,
        times,
        speeds,
        scheduled_points,
        profiles,
        energy_consumption: mechanical_energy_consumed(envelope, context),
        total_time: envelope.total_time() + stops.iter().map(|(_, d)| d).sum::<f64>(),
    }
}

/// The cumulative energy consumed at the wheels of the train: per part, the
/// kinetic energy delta less the work of gravity and of the resistance to
/// movement.
fn mechanical_energy_consumed(envelope: &Envelope, context: &EnvelopeSimContext) -> f64 {
    let rolling_stock = &context.rolling_stock;
    let mut cumulative_energy = 0.0;
    for part in envelope.iter() {
        let mean_grade = 0.001 * context.path.average_grade(part.begin_pos(), part.end_pos());
        let altitude_delta = mean_grade * part.total_distance();
        let work_gravity = -rolling_stock.mass * GRAVITY_ACCELERATION * altitude_delta;
        let kinetic_energy_delta = 0.5 * rolling_stock.inertia * (part.end_speed() * part.end_speed() - part.begin_speed() * part.begin_speed());
        let mut work_drag = 0.0;
        for i in 0..part.point_count() - 1 {
            let mean_speed = (part.point_speed(i) + part.point_speed(i + 1)) / 2.0;
            let position_delta = part.point_pos(i + 1) - part.point_pos(i);
            work_drag -= rolling_stock.rolling_resistance(mean_speed) * position_delta;
        }
        cumulative_energy += (kinetic_energy_delta - work_gravity - work_drag).max(0.0);
    }
    cumulative_energy
}
