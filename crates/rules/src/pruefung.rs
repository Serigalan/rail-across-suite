// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The rule catalogue applied to an element chain — the app's
//! packages/core/src/rules/trassierungCheck.js, ported. The shared vectors
//! (packages/core/constraints/tests/checks.json) hold the two alike.
//!
//! [`crate::grenzen`] derives the bounds a run builds to; this judges what a
//! run built, the way the app's element table does — every rule, full scopes,
//! severities — without asking the bounds anything. A run that obeys its own
//! bounds but lands on a finding the table would show is exactly what it is
//! here to catch.
//!
//! Like the app it fills the scope from the elements and nothing else: a
//! transition's cant at its ends is its neighbours' (transitionCantEnds), the
//! deficiency is rounded to the millimetre (computeCantDefSigned).

use crate::element::Element;
use crate::expr::{Error, Value};
use crate::katalog::{Katalog, Rule};

pub const CANT_DEF_COEFF: f64 = 11.8;
const CURVATURE_EPS: f64 = 1e-6;

fn curv(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 => 1.0 / r,
        _ => 0.0,
    }
}

fn k_start(el: &Element) -> f64 {
    curv(if el.is_transition() { el.r1 } else { el.radius })
}

fn k_end(el: &Element) -> f64 {
    curv(if el.is_transition() { el.r2 } else { el.radius })
}

/// The cant an element states at one end itself, else its one cant.
fn own_end(el: Option<&Element>, end: Option<f64>) -> f64 {
    match el {
        None => 0.0,
        Some(el) => end.or(el.cant).unwrap_or(0.0),
    }
}

/// The one cant of an element. A record states it as `cant` where it is the
/// same at both ends and as `cantStart` + `cantEnd` where it ramps (decision
/// 369); then it is the end that helps the curve least, as
/// geometry.withCantRamp derives it.
fn cant(el: &Element) -> f64 {
    if el.cant.is_some() || el.is_transition() {
        return el.cant.unwrap_or(0.0);
    }
    let (Some(start), Some(end)) = (el.cant_start, el.cant_end) else {
        return 0.0;
    };
    match el.radius {
        Some(r) if r != 0.0 && r.signum() * end < r.signum() * start => end,
        _ => start,
    }
}

fn cant_ends(els: &[Element], i: usize) -> (f64, f64) {
    // Every element may state the cant at its ends (decision 355): an arc or a
    // straight where it changes along it, a transition where its neighbours do
    // not imply it.
    let el = &els[i];
    if !el.is_transition() {
        return (own_end(Some(el), el.cant_start), own_end(Some(el), el.cant_end));
    }
    let prev = i.checked_sub(1).map(|j| &els[j]);
    let next = els.get(i + 1);
    (
        el.cant_start.unwrap_or_else(|| own_end(prev, prev.and_then(|p| p.cant_end))),
        el.cant_end.unwrap_or_else(|| own_end(next, next.and_then(|n| n.cant_start))),
    )
}

/// The cant deficiency [mm], rounded to the millimetre half up as Math.round.
fn deficiency(v: f64, radius: Option<f64>, cant: f64) -> f64 {
    match radius {
        Some(r) if r != 0.0 => (CANT_DEF_COEFF * v * v / r.abs() - cant * r.signum() + 0.5).floor(),
        _ => 0.0,
    }
}

/// Does `b` run on as `a` does — no transition, the same curvature?
fn same_geometry(a: Option<&Element>, b: &Element) -> bool {
    a.is_some_and(|a| !a.is_transition() && !b.is_transition() && (curv(a.radius) - curv(b.radius)).abs() < CURVATURE_EPS)
}

/// The runs of one curvature beside a chain that is only part of its track
/// (trassierungCheck.runBeside), for LP.EL.01.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context<'a> {
    pub before: Option<&'a Element>,
    pub after: Option<&'a Element>,
}

/// The element's length as LP.EL.01 reads it: with its neighbours of the
/// same curvature, which are one straight or one arc on the ground
/// (Entscheidungen 356, 363). A run reaching the chain's end goes on into
/// `context`.
fn run_length(els: &[Element], i: usize, context: Context) -> f64 {
    let el = &els[i];
    let mut length = el.length.unwrap_or(0.0);
    if el.is_transition() {
        return length;
    }
    let mut j = i;
    while j > 0 && same_geometry(Some(&els[j - 1]), el) {
        length += els[j - 1].length.unwrap_or(0.0);
        j -= 1;
    }
    if j == 0 && same_geometry(context.before, el) {
        length += context.before.and_then(|b| b.length).unwrap_or(0.0);
    }
    let mut j = i + 1;
    while j < els.len() && same_geometry(Some(&els[j]), el) {
        length += els[j].length.unwrap_or(0.0);
        j += 1;
    }
    if j >= els.len() && same_geometry(context.after, el) {
        length += context.after.and_then(|a| a.length).unwrap_or(0.0);
    }
    length
}

/// The scope of the element rules for element `i`, as the app's elementScope fills it.
pub fn element_scope(els: &[Element], i: usize, context: Context) -> Vec<(&'static str, Value)> {
    let el = &els[i];
    let prev = i.checked_sub(1).map(|j| &els[j]);
    let next = els.get(i + 1);
    let v = el.speed.unwrap_or(0.0);
    let (start, end) = cant_ends(els, i);
    let du = (end - start).abs();
    // Signed with the curve like the cant: across a reverse transition the
    // deficiency changes sides, and the ramp carries the sum.
    let side = |r: Option<f64>, c: f64| match r {
        Some(r) if r != 0.0 => r.signum() * deficiency(v, Some(r), c),
        _ => 0.0,
    };
    let duf = if el.is_transition() {
        (side(el.r2, end) - side(el.r1, start)).abs()
    } else {
        0.0
    };
    let length = el.length.unwrap_or(0.0);
    let reverse = el.element_type == Some(0)
        && match (prev, next) {
            (Some(p), Some(n)) => {
                let (a, b) = (k_end(p), k_start(n));
                a.abs() > CURVATURE_EPS && b.abs() > CURVATURE_EPS && a.signum() != b.signum()
            }
            _ => false,
        };
    vec![
        ("element.design_speed", v.into()),
        ("element.length", run_length(els, i, context).into()),
        ("element.cant", cant(el).abs().into()),
        ("element.radius", el.radius.unwrap_or(0.0).abs().into()),
        ("element.transition_form", el.form().unwrap_or("").into()),
        ("prev.design_speed", prev.and_then(|p| p.speed).unwrap_or(v).into()),
        ("next.design_speed", next.and_then(|n| n.speed).unwrap_or(v).into()),
        ("physics.u_f", deficiency(v, el.radius, cant(el)).into()),
        ("physics.delta_u", du.into()),
        ("physics.delta_u_f", duf.into()),
        ("physics.ramp_slope", if du > 0.0 { 1000.0 * length / du } else { f64::INFINITY }.into()),
        ("model.reverse_curve_straight", reverse.into()),
    ]
}

/// The scope of the boundary rules between elements `i` and `i + 1`.
pub fn boundary_scope(els: &[Element], i: usize) -> Vec<(&'static str, Value)> {
    let (a, b) = (&els[i], &els[i + 1]);
    let step = (k_end(a) - k_start(b)).abs();
    let jump = step > CURVATURE_EPS;
    vec![
        ("prev.cant_end", cant_ends(els, i).1.into()),
        ("next.cant_start", cant_ends(els, i + 1).0.into()),
        ("prev.design_speed", a.speed.unwrap_or(0.0).into()),
        ("next.design_speed", b.speed.unwrap_or(0.0).into()),
        ("physics.curvature_jump", jump.into()),
        ("physics.r_w", if jump { 1.0 / step } else { f64::INFINITY }.into()),
    ]
}

/// One finding: where ("#i", or "#i|#j" for a boundary), the rule, the severity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub at: String,
    pub rule: String,
    pub severity: String,
}

fn apply(
    katalog: &Katalog,
    rules: &[&Rule],
    base: &[(&str, Value)],
    in_context: Option<&dyn Fn(&str) -> bool>,
    excluded: &[&str],
    at: &str,
    out: &mut Vec<Finding>,
) -> Result<(), Error> {
    let scope = katalog.scope(base);
    for rule in rules {
        if rule.excludes.iter().any(|e| excluded.contains(&e.as_str())) {
            continue;
        }
        let outcome = katalog.evaluate_rule(rule, &scope, in_context)?;
        if outcome.applied {
            out.push(Finding {
                at: at.to_string(),
                rule: rule.id.clone(),
                severity: outcome.severity.unwrap_or_default(),
            });
        }
    }
    Ok(())
}

/// Every rule applied to the chain. `only`: the elements to judge
/// (boundaries count where either side is one of them), all of them when
/// None. `context`: the runs of one curvature beside a chain that is only
/// part of its track, for LP.EL.01.
pub fn check_track(katalog: &Katalog, els: &[Element], only: Option<&[usize]>, context: Context) -> Result<Vec<Finding>, Error> {
    let judged = |i: usize| only.is_none_or(|o| o.contains(&i));
    let speed = |el: &Element| el.speed.is_some_and(|v| v != 0.0);
    let ramp_rules = katalog.rules_for_scope("cant_ramp");
    let boundary_rules = katalog.rules_for_scope("boundary");
    let mut findings = Vec::new();
    for (i, el) in els.iter().enumerate() {
        if !judged(i) || !speed(el) {
            continue;
        }
        let in_switch = el.in_switch();
        let ctx = move |id: &str| id == "switch_area" && in_switch;
        let base = element_scope(els, i, context);
        let at = format!("#{i}");
        apply(
            katalog,
            &katalog.rules_for_element(el.type_id(), el.form()),
            &base,
            Some(&ctx),
            &[],
            &at,
            &mut findings,
        )?;
        let du = base.iter().find(|(n, _)| *n == "physics.delta_u").and_then(|(_, v)| v.as_num()).unwrap_or(0.0);
        if du > 0.0 {
            // A ramp on an arc or a straight is LP.UB.02's Sonderfall (decision 355).
            let ramp = [
                ("model.ramp_on_transition", el.is_transition().into()),
                ("model.ramp_form_matches", (el.form() == Some("clothoid")).into()),
            ];
            apply(katalog, &ramp_rules, &ramp, None, &[], &at, &mut findings)?;
        }
    }
    for i in 0..els.len().saturating_sub(1) {
        let (a, b) = (&els[i], &els[i + 1]);
        if !(judged(i) || judged(i + 1)) || !speed(a) || !speed(b) {
            continue;
        }
        let internal = a.in_switch() && b.in_switch() && a.switch_id == b.switch_id;
        let either = a.in_switch() || b.in_switch();
        let ctx = move |id: &str| id == "switch_area" && either;
        let excluded: &[&str] = if internal { &["switch_internal"] } else { &[] };
        apply(
            katalog,
            &boundary_rules,
            &boundary_scope(els, i),
            Some(&ctx),
            excluded,
            &format!("#{i}|#{}", i + 1),
            &mut findings,
        )?;
    }
    Ok(findings)
}
