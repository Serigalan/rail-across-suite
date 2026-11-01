// SPDX-FileCopyrightText: 2022 The OSRD Contributors
// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only
//
// OSRD's own test cases, ported from core/envelope-sim/src/test/
// (MaxEffortEnvelopeTest.kt, AllowanceTests.kt) and its fixtures
// (SimpleRollingStock.kt, SimpleContextBuilder.java, TestMRSPBuilder.java,
// MaxEffortEnvelopeBuilder.kt). The expected transitions are OSRD's: it
// computed them with a time step of 0.01 s / 0.1 s, the tolerance absorbs
// the difference for the time step of 4 s used here.

use rail_across_sim::allowances::{Allowance, AllowanceRange, AllowanceValue};
use rail_across_sim::envelope::{Attrs, Envelope, MrspEnvelopeBuilder, constant_speed_part};
use rail_across_sim::physics::{EnvelopeSimContext, RollingStock, SimPath, TractiveEffortPoint};
use rail_across_sim::pipelines::{max_effort_envelope_from, max_speed_envelope_from};

const TIME_STEP: f64 = 4.0;
const MAX_SPEED: f64 = 300.0 / 3.6;

fn linear_effort_curve() -> Vec<TractiveEffortPoint> {
    let effort = |speed: f64| {
        let (max_effort, min_effort) = (450_000.0, 180_000.0);
        max_effort + (min_effort - max_effort) * (speed / MAX_SPEED)
    };
    let mut curve: Vec<TractiveEffortPoint> = (0..)
        .map(f64::from)
        .take_while(|s| *s < MAX_SPEED)
        .map(|speed| TractiveEffortPoint {
            speed,
            max_effort: effort(speed),
        })
        .collect();
    curve.push(TractiveEffortPoint {
        speed: MAX_SPEED,
        max_effort: effort(MAX_SPEED),
    });
    curve
}

fn train(length: f64, const_gamma: f64) -> RollingStock {
    let mass = 850_000.0;
    RollingStock::new(
        mass,
        1.05,
        length,
        MAX_SPEED,
        0.65 * mass / 100.0,
        0.008 * mass / 100.0 * 3.6,
        0.00012 * mass / 100.0 * 3.6 * 3.6,
        const_gamma,
        linear_effort_curve(),
    )
}

fn standard_train() -> RollingStock {
    train(400.0, 0.5)
}

fn max_dec_train() -> RollingStock {
    train(400.0, 0.95)
}

fn context(rolling_stock: RollingStock, length: f64, slope: f64) -> EnvelopeSimContext {
    EnvelopeSimContext::new(rolling_stock, SimPath::constant_slope(length, slope), TIME_STEP)
}

fn mrsp(context: &EnvelopeSimContext, limits: &[(f64, f64, f64)]) -> Envelope {
    let mut builder = MrspEnvelopeBuilder::new();
    let max = context.rolling_stock.max_speed;
    builder.add_part(constant_speed_part(Attrs::default(), 0.0, context.path.length, max));
    for &(begin, end, speed) in limits {
        builder.add_part(constant_speed_part(Attrs::default(), begin, end, speed));
    }
    builder.build()
}

fn simple_max_effort_envelope(context: &EnvelopeSimContext, speed: f64, stops: &[f64]) -> Envelope {
    let mrsp = mrsp(context, &[(0.0, context.path.length, speed)]);
    let max_speed = max_speed_envelope_from(context, stops, &mrsp).unwrap();
    max_effort_envelope_from(context, 0.0, &max_speed).unwrap()
}

fn complex_max_effort_envelope(context: &EnvelopeSimContext, stops: &[f64]) -> Envelope {
    let l = context.path.length;
    let mrsp = mrsp(
        context,
        &[
            (0.0, 10000.0, 44.4),
            (10000.0, 20000.0, 64.4),
            (20000.0, 50000.0, 54.4),
            (50000.0, 55000.0, 14.4),
            (55000.0, 70000.0, 54.4),
            (70000.0, 75000.0, 74.4),
            (75000.0, l, 44.4),
        ],
    );
    let max_speed = max_speed_envelope_from(context, stops, &mrsp).unwrap();
    max_effort_envelope_from(context, 0.0, &max_speed).unwrap()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Shape {
    Increasing,
    Constant,
    Decreasing,
}

fn shapes(envelope: &Envelope) -> Vec<Shape> {
    envelope
        .iter()
        .map(|p| {
            if p.min_speed() == p.max_speed() {
                Shape::Constant
            } else if p.begin_speed() < p.end_speed() {
                Shape::Increasing
            } else {
                Shape::Decreasing
            }
        })
        .collect()
}

/// The transitions between parts, each within delta of OSRD's.
fn check_positions(envelope: &Envelope, delta: f64, expected: &[f64]) {
    let transitions: Vec<f64> = envelope
        .parts()
        .windows(2)
        .map(|w| {
            assert_eq!(w[0].end_pos(), w[1].begin_pos());
            w[0].end_pos()
        })
        .collect();
    assert_eq!(transitions.len(), expected.len(), "transitions {transitions:?}, OSRD {expected:?}");
    for (got, want) in transitions.iter().zip(expected) {
        assert!(
            (got - want).abs() <= delta,
            "transition at {got:.1} m, OSRD's at {want} m ± {delta:.0}: {transitions:?}"
        );
    }
}

use Shape::{Constant as C, Decreasing as D, Increasing as I};

#[test]
fn max_effort_flat() {
    let context = context(standard_train(), 10000.0, 0.0);
    let envelope = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    assert_eq!(shapes(&envelope), [I, C, D, I, D]);
    check_positions(&envelope, 2.0 * envelope.max_speed() * TIME_STEP, &[2726.0, 4029.0, 6000.0, 8292.0]);
    assert!(envelope.continuous);
}

#[test]
fn max_effort_flat_non_const_dec() {
    let context = context(max_dec_train(), 10000.0, 0.0);
    let envelope = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    assert_eq!(shapes(&envelope), [I, C, D, I, C, D]);
    check_positions(&envelope, 2.0 * envelope.max_speed() * TIME_STEP, &[2726.0, 4993.0, 6000.0, 8727.0, 8993.0]);
    assert!(envelope.continuous);
}

#[test]
fn max_effort_steep() {
    let context = context(standard_train(), 10000.0, 20.0);
    let envelope = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    assert_eq!(shapes(&envelope), [I, D, I, D]);
    check_positions(&envelope, 2.0 * envelope.max_speed() * TIME_STEP, &[4380.0, 6000.0, 8827.0]);
    assert!(envelope.continuous);
}

#[test]
fn max_effort_steep_non_const_dec() {
    let context = context(max_dec_train(), 10000.0, 20.0);
    let envelope = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    assert_eq!(shapes(&envelope), [I, D, I, D]);
    check_positions(&envelope, 2.0 * envelope.max_speed() * TIME_STEP, &[5216.0, 6000.0, 9417.0]);
    assert!(envelope.continuous);
}

#[test]
fn max_effort_complex_mrsp() {
    let context = context(standard_train(), 100000.0, 0.0);
    let envelope = complex_max_effort_envelope(&context, &[50000.0, 100000.0]);
    assert!(envelope.continuous);
    assert_eq!(envelope.begin_speed(), 0.0);
    assert_eq!(envelope.end_speed(), 0.0);
    assert_eq!(envelope.interpolate_speed(50000.0), 0.0);
    // never above the limits
    for (pos, limit) in [
        (5000.0, 44.4),
        (15000.0, 64.4),
        (30000.0, 54.4),
        (52000.0, 14.4),
        (60000.0, 54.4),
        (72000.0, 74.4),
        (90000.0, 44.4),
    ] {
        assert!(envelope.interpolate_speed(pos) <= limit + 1e-6, "{pos} m");
    }
}

fn mareco(length: f64, percentage: f64) -> Allowance {
    Allowance::mareco(
        0.0,
        length,
        1.0,
        vec![AllowanceRange {
            begin_pos: 0.0,
            end_pos: length,
            value: AllowanceValue::Percentage(percentage),
        }],
    )
    .unwrap()
}

fn linear(length: f64, percentage: f64) -> Allowance {
    Allowance::linear(
        0.0,
        length,
        1.0,
        vec![AllowanceRange {
            begin_pos: 0.0,
            end_pos: length,
            value: AllowanceValue::Percentage(percentage),
        }],
    )
    .unwrap()
}

#[test]
fn mareco_shape_flat() {
    let context = context(standard_train(), 10000.0, 0.0);
    let base = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    let envelope = mareco(10000.0, 10.0).apply(&base, &context).unwrap();
    assert_eq!(shapes(&envelope), [I, D, D, I, D, D]);
    check_positions(&envelope, 2.0 * envelope.max_speed() * TIME_STEP, &[1411.0, 5094.0, 6000.0, 6931.0, 9339.0]);
    assert!(envelope.continuous);
}

#[test]
fn mareco_shape_steep() {
    let context = context(standard_train(), 10000.0, 20.0);
    let base = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    let envelope = mareco(10000.0, 10.0).apply(&base, &context).unwrap();
    assert_eq!(shapes(&envelope), [I, C, D, D, I, C, D, D]);
    check_positions(
        &envelope,
        2.0 * envelope.max_speed() * TIME_STEP,
        &[1839.0, 4351.0, 5747.0, 6000.0, 7259.0, 8764.0, 9830.0],
    );
    assert!(envelope.continuous);
}

/// The allowance adds its time: 10 % of the fastest run, within a time step.
#[test]
fn allowances_add_their_time() {
    let context = context(standard_train(), 100000.0, 0.0);
    let base = complex_max_effort_envelope(&context, &[50000.0, 100000.0]);
    for allowance in [mareco(100000.0, 10.0), linear(100000.0, 10.0)] {
        let envelope = allowance.apply(&base, &context).unwrap();
        assert!(envelope.continuous);
        let target = base.total_time() * 1.1;
        assert!(
            (envelope.total_time() - target).abs() <= TIME_STEP,
            "{:?}: {} s, target {} s",
            allowance.distribution,
            envelope.total_time(),
            target
        );
        assert_eq!(envelope.interpolate_speed(50000.0), 0.0);
    }
}

/// MARECO caps the speed and coasts before braking; the linear allowance never coasts.
#[test]
fn mareco_coasts_where_linear_does_not() {
    use rail_across_sim::envelope::EnvelopeProfile;
    let context = context(standard_train(), 10000.0, 0.0);
    let base = simple_max_effort_envelope(&context, 44.4, &[6000.0, 10000.0]);
    let m = mareco(10000.0, 10.0).apply(&base, &context).unwrap();
    let l = linear(10000.0, 10.0).apply(&base, &context).unwrap();
    assert!(m.iter().any(|p| p.has_profile(EnvelopeProfile::Coasting)));
    assert!(!l.iter().any(|p| p.has_profile(EnvelopeProfile::Coasting)));
    assert!((m.total_time() - l.total_time()).abs() <= 2.0 * TIME_STEP);
}
