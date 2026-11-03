// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The search of a switch connection — "Erweiterte Weichenverbindung".
//!
//! Two turnouts facing each other on two tracks, one to three elements between
//! their branches: straights, arcs and transitions, a transition running
//! between the curvatures of its neighbours. The search moves **both** toes on
//! the elements they were picked on, independently, and chooses the elements
//! between — those of every sequence it is given, each value sought or given.
//!
//! The turnout geometry is not computed here: the app samples both branch ends
//! over the elements (`connectionBranchEnds`) and sends them with the limits
//! its dialog holds the middle to. Each sample is a point, a tangent and a
//! curvature at B1E (first turnout) or B2A (second), in the direction the
//! connection runs; this crate interpolates them and closes a chain of
//! elements between the two:
//!
//! ```text
//! chain(B1E(s1), pieces) = B2A(s2)        position and tangent, three equations
//! ```
//!
//! over s1, s2 and the open values of the pieces — lengths, and the curvature
//! of each arc — held to
//!
//! * l_min (LP.EL.01) for every element, a straight counted with a straight end
//!   piece of the branch it runs on from;
//! * the comparison radius (LP.KS.01) at every joint with a jump in curvature,
//!   1/|Δκ| at least the Ermessensgrenze; under the Regelwert is a hint, which
//!   the search avoids where it can;
//! * the cant deficiency of every piece at the cant of the middle — the mean of
//!   both turnouts' cant at the ldS — and those two cants agreeing;
//! * each turnout where its own checks hold (the samples' `ok`).
//!
//! Least squares and then SLSQP from a few starts per sequence; among what
//! closes, per sequence the one the ranking puts first: nearer the click (in
//! whole metres), without a hint, fewer elements, the gentlest and shortest. The app solves
//! an adopted answer again with its own geometry.

pub mod branch;
pub mod lm;
pub mod piece;
pub mod problem;
pub mod solve;

use branch::{BranchEnd, Sample};
use problem::{Field, K_SCALE, Kind, Limits, Problem, Template};
use serde::Deserialize;
use serde_json::json;
use solve::{Solution, answer, better, order, search_starts, solve_one, starts};
use std::cell::Cell;
use std::collections::HashMap;

/// The sequences of middle elements tried where the request names none,
/// fewest first. A lone straight or a lone transition has a single value and
/// cannot close beside the second toe; two straights in a row are one.
pub const TEMPLATES: &[&[Kind]] = {
    use Kind::{Arc, Straight, Transition};
    &[
        &[Arc],
        &[Arc, Arc],
        &[Straight, Arc],
        &[Arc, Straight],
        &[Transition, Arc],
        &[Arc, Transition],
        &[Transition, Transition],
        &[Straight, Transition],
        &[Transition, Straight],
        &[Arc, Straight, Arc],
        &[Transition, Straight, Transition],
        &[Transition, Arc, Transition],
        &[Arc, Transition, Arc],
        &[Arc, Arc, Arc],
        &[Straight, Arc, Straight],
    ]
};

/// This many answers at most, unless the request says otherwise.
const MAX_SOLUTIONS: usize = 8;
/// What the whole search may spend, in evaluations of the chain — it stops
/// with what it has found by then.
const BUDGET: usize = 4_000_000;

/// One pair of branch ends: the first turnout's form and the second's.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    pub label: Option<String>,
    pub label2: Option<String>,
    #[serde(default)]
    pub first: Vec<Sample>,
    #[serde(default)]
    pub second: Vec<Sample>,
}

/// An element as the dialog states it: its kind, and its length and an arc's
/// radius where they are given (the radius as a magnitude: the side it bends
/// to is sought).
#[derive(Debug, Deserialize)]
pub struct Element {
    #[serde(rename = "type")]
    pub kind: Kind,
    pub length: Option<f64>,
    pub radius: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub speed: f64,
    #[serde(default)]
    pub limits: Limits,
    pub forms: Vec<Form>,
    /// The sequences to try; TEMPLATES where none are named.
    pub templates: Option<Vec<Vec<Kind>>>,
    /// One sequence with its values given where they are — instead of `templates`.
    pub elements: Option<Vec<Element>>,
    pub max_solutions: Option<usize>,
    pub budget: Option<usize>,
}

/// The sequences a request asks for, each value it gives fixed. A given
/// radius bends either way: one template per side of every such arc.
fn templates_of(req: &Request) -> Result<Vec<Template>, String> {
    if let Some(elements) = &req.elements {
        if elements.is_empty() || elements.len() > 3 {
            return Err("elements".into());
        }
        let kinds: Vec<Kind> = elements.iter().map(|e| e.kind).collect();
        let mut base = Template::new(&kinds);
        let mut radii = Vec::new();
        for (i, e) in elements.iter().enumerate() {
            if let Some(length) = e.length.filter(|v| v.is_finite() && *v > 0.0) {
                base = base.with_fixed(i, Field::Length, length);
            }
            if e.kind == Kind::Arc
                && let Some(r) = e.radius.filter(|v| v.is_finite() && *v > 0.0)
            {
                radii.push((i, r));
            }
        }
        let mut out = vec![base];
        for (i, r) in radii {
            out = out
                .into_iter()
                .flat_map(|t| [1.0, -1.0].map(|sign| t.clone().with_fixed(i, Field::K, sign * K_SCALE / r)))
                .collect();
        }
        return Ok(out);
    }
    match &req.templates {
        Some(list) => {
            if list.iter().any(|t| t.is_empty() || t.len() > 3) {
                return Err("templates".into());
            }
            Ok(list.iter().map(|t| Template::new(t)).collect())
        }
        None => Ok(TEMPLATES.iter().map(|t| Template::new(t)).collect()),
    }
}

/// The solutions for one request, best first, from the first form that has
/// any; empty where none closes.
pub fn solve(req: &Request) -> Result<Vec<Solution>, String> {
    if !(req.speed.is_finite() && req.speed > 0.0) {
        return Err("speed".into());
    }
    if req.forms.is_empty() {
        return Err("forms".into());
    }
    let templates = templates_of(req)?;
    let budget = req.budget.unwrap_or(BUDGET);
    let evals = Cell::new(0usize);
    for (index, form) in req.forms.iter().enumerate() {
        let (Some(first), Some(second)) = (BranchEnd::new(&form.first), BranchEnd::new(&form.second)) else {
            continue;
        };
        let problem = Problem::new(&first, &second, &req.limits, req.speed);
        let begin = search_starts(&problem, &first, &second);
        // The best per sequence of kinds: a given radius tried either way is one sequence.
        let mut best: HashMap<Vec<Kind>, Solution> = HashMap::new();
        'search: for tpl in &templates {
            for &(i1, i2, s1, s2) in &begin {
                for start in starts(tpl, &problem, s1, s2) {
                    if evals.get() > budget {
                        break 'search;
                    }
                    let Some(z) = solve_one(tpl, &problem, i1, i2, &start, &evals) else {
                        continue;
                    };
                    let sol = answer(tpl, &problem, &z, index, form.label.clone(), form.label2.clone());
                    if best.get(&tpl.kinds).is_none_or(|b| better(&sol, b)) {
                        best.insert(tpl.kinds.clone(), sol);
                    }
                }
            }
        }
        if !best.is_empty() {
            let mut out: Vec<Solution> = best.into_values().collect();
            order(&mut out);
            out.truncate(req.max_solutions.unwrap_or(MAX_SOLUTIONS));
            return Ok(out);
        }
    }
    Ok(Vec::new())
}

/// The answer to a request's JSON: `{"solutions": [...]}` best first,
/// `{"error": "no_solution"}` where none closes, or `{"error":
/// "invalid_request", "detail": ...}`.
pub fn solve_json(request: &str) -> String {
    let req: Request = match serde_json::from_str(request) {
        Ok(req) => req,
        Err(err) => return json!({ "error": "invalid_request", "detail": err.to_string() }).to_string(),
    };
    match solve(&req) {
        Ok(sols) if sols.is_empty() => json!({ "error": "no_solution" }).to_string(),
        Ok(sols) => json!({ "solutions": sols }).to_string(),
        Err(detail) => json!({ "error": "invalid_request", "detail": detail }).to_string(),
    }
}
