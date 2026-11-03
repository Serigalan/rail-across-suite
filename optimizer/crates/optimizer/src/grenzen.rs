// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The limits a run is held to, read out of the rule catalogue for one level.
//!
//! A run is asked for a *Stufe*: the Regelwert (`reg`) or the Ermessensgrenze
//! (`discretion`). What that means is not a table of numbers kept here — it is
//! a severity. Under the Regelwert nothing the run proposes may be judged worse
//! than a hint; under the Ermessensgrenze nothing worse than a warning. Which
//! threshold of a rule that comes down to is read off the rule's own
//! `evaluation`: the last line whose severity is still admitted is the line
//! that bounds the run, and the threshold it compares against is the bound.
//! LP.UB.03 under the Regelwert is `l_R >= reg`, i.e. 10·v·Δu/1000; under the
//! Ermessensgrenze `l_R >= discretion`, i.e. 8·v·Δu/1000. A rule whose
//! admitted lines run into an `else` never binds the run at all — a hint, at
//! either level.
//!
//! What the bound then *is* for the geometry is the one piece of model
//! knowledge here, read from where the rule takes its input from, the way the
//! app's trassierungCheck.js fills the same names:
//!
//! * `element.length` — a lower (or upper) bound on an element's length
//! * `physics.ramp_slope` — the mean ramp gradient 1:m, turned into a length
//!   by l = m·Δu/1000 (derived_inputs in the catalogue)
//! * `element.cant` — the cants a curve may carry
//! * `physics.u_f` — the cant deficiency, and with it the speed of a curve
//! * `physics.r_w` — the comparison radius at a curvature jump
//! * `element.design_speed` alone — the design speeds that exist at all
//!
//! A rule this module cannot place in one of those is refused when the limits
//! are built, not skipped: a run that quietly ignored a rule of the catalogue
//! it names would be the one thing this module exists to prevent.

use crate::element::Element;
use crate::expr::{Error, Op, Scope, Sym, Value};
use crate::geometry::{CANT_DEFICIENCY_COEFF, permissible_speed};
use crate::katalog::{Branch, Katalog, Rule, Source};
use crate::pruefung::{Context, element_scope};
use serde_json::json;
use std::fmt;

/// The level a run is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stufe {
    /// The Regelwert: nothing worse than a hint.
    Reg,
    /// The Ermessensgrenze: nothing worse than a warning.
    Discretion,
}

impl Stufe {
    pub fn parse(s: &str) -> Result<Stufe, GrenzenError> {
        match s {
            "reg" => Ok(Stufe::Reg),
            "discretion" => Ok(Stufe::Discretion),
            _ => Err(GrenzenError(format!("unbekannte Grenzwertstufe: {s}"))),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Stufe::Reg => "reg",
            Stufe::Discretion => "discretion",
        }
    }

    /// The worst severity a run may produce.
    pub fn worst(self) -> &'static str {
        match self {
            Stufe::Reg => "hint",
            Stufe::Discretion => "warning",
        }
    }

    /// The rule scopes this level lets a run past whatever they say. At the
    /// Ermessensgrenze that is the cant ramp (LP.UB.02): it judges the ramp's
    /// *form* against the transition's, and in this app every ramp is
    /// straight (decision 47), so a Bloss transition is a Sonderfall there by
    /// the app's model, not by a limit of the Ril. The client decided
    /// (decision 57) that a run at the Ermessensgrenze still proposes Bloss
    /// transitions — their lengths and gradients are held to LP.UB.04/06/07/08
    /// like any other — and the element table then shows each one as the
    /// Sonderfall it is.
    fn tolerated(self) -> &'static [&'static str] {
        match self {
            Stufe::Reg => &[],
            Stufe::Discretion => &["cant_ramp"],
        }
    }
}

/// A level that does not exist, or a rule a run cannot be held to.
#[derive(Debug, Clone, PartialEq)]
pub struct GrenzenError(pub String);

impl fmt::Display for GrenzenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GrenzenError {}

impl From<Error> for GrenzenError {
    fn from(e: Error) -> GrenzenError {
        GrenzenError(e.to_string())
    }
}

// The scopes of the Höhenplan — the stretch between two height points and the
// gradient change at one. They judge the gradient, which no run touches.
const VERTICAL_SCOPES: [&str; 2] = ["gradient", "vertical_curve"];

// The names a rule may take its input from, and what they are to a run.
const SPEED_ONLY: &[&str] = &["element.design_speed"];
const CANT_ONLY: &[&str] = &["element.cant"];
const CONTINUITY: &[&str] = &["next.cant_start", "prev.cant_end"];

/// What a design speed or a cant is scanned over when the catalogue is asked
/// which of them it admits. Wide on purpose: the catalogue says where its own
/// range ends, this only has to reach past it.
const SPEED_SCAN: std::ops::RangeInclusive<u32> = 1..=1000;
const CANT_SCAN: std::ops::RangeInclusive<u32> = 0..=2000; // 0 … 1000 mm in 0.5 mm

/// What a bound on an element is to the geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Length,
    Slope,
    Deficiency,
}

/// Which line of `evaluation` bounds a run at this level.
enum Bound {
    /// The rule admits every case: it never binds.
    Never,
    /// The bounding line is not one comparison of an input against a threshold.
    Complex,
    /// Input `op` threshold, the input on the left.
    Cmp(Sym, Op, Sym),
}

struct ElementBound {
    rule: usize,
    op: Op,
    threshold: Sym,
    kind: Kind,
}

struct JumpBound {
    rule: usize,
    threshold: Sym,
}

/// One rule on the length of a transition: its id, the length it asks for,
/// and whether that is a shortest (else a longest) length.
#[derive(Debug, Clone, PartialEq)]
pub struct RampRule {
    pub id: String,
    pub length: f64,
    pub lower: bool,
}

/// The shortest and longest a transition may be, and the rule that asked for the shortest.
#[derive(Debug, Clone, PartialEq)]
pub struct RampBounds {
    pub shortest: f64,
    pub longest: f64,
    pub by: Option<String>,
}

const ELEMENT_KEYS: [(&str, Option<&str>); 4] = [
    ("straight", None),
    ("circular_arc", None),
    ("transition_curve", Some("clothoid")),
    ("transition_curve", Some("bloss")),
];

/// The limits of one catalogue at one level, ready for the optimizer.
pub struct Grenzen<'k> {
    pub katalog: &'k Katalog,
    pub stufe: Stufe,
    pub max_rank: i32,
    /// Rules a run at this level is let past, by id — what the acceptance's
    /// own check has to know to judge a run fairly.
    pub toleriert: Vec<String>,
    /// The design speeds the catalogue admits, ascending.
    pub speeds: Vec<f64>,
    pub v_top: f64,
    pub u_step: f64,
    /// Whether a run may hand out a clothoid and a Bloss transition.
    pub forms: [(&'static str, bool); 2],
    cants: [Vec<f64>; 2],
    element_bounds: Vec<Vec<ElementBound>>,
    jump_rules: Vec<JumpBound>,
    ramp_rules: Vec<usize>,
    uf: [Vec<f64>; 2],
    uf_levels: [Vec<f64>; 2],
    rw_min: [Vec<f64>; 2],
}

fn switch_area(id: &str) -> bool {
    id == "switch_area"
}

fn context(on_switch: bool) -> Option<&'static dyn Fn(&str) -> bool> {
    if on_switch { Some(&switch_area) } else { None }
}

fn num(value: Option<Value>) -> Result<Option<f64>, Error> {
    match value {
        None => Ok(None),
        Some(Value::Num(x)) => Ok(Some(x)),
        Some(other) => Err(Error::Expr(format!("threshold is not a number: {other:?}"))),
    }
}

impl<'k> Grenzen<'k> {
    pub fn new(katalog: &'k Katalog, stufe: Stufe) -> Result<Grenzen<'k>, GrenzenError> {
        let max_rank = katalog.rank(stufe.worst());
        let toleriert = stufe
            .tolerated()
            .iter()
            .flat_map(|scope| katalog.rules_for_scope(scope))
            .map(|r| r.id.clone())
            .collect();
        let mut g = Grenzen {
            katalog,
            stufe,
            max_rank,
            toleriert,
            speeds: Vec::new(),
            v_top: 0.0,
            u_step: 0.0,
            forms: [("clothoid", false), ("bloss", false)],
            cants: [Vec::new(), Vec::new()],
            element_bounds: Vec::new(),
            jump_rules: Vec::new(),
            ramp_rules: Vec::new(),
            uf: [Vec::new(), Vec::new()],
            uf_levels: [Vec::new(), Vec::new()],
            rw_min: [Vec::new(), Vec::new()],
        };

        g.speeds = g.design_speeds()?;
        let Some(&v_top) = g.speeds.last() else {
            return Err(GrenzenError(format!("{}: der Katalog lässt keine Entwurfsgeschwindigkeit zu", katalog.id)));
        };
        g.v_top = v_top;

        // The cants a curve may carry, on the line and in a turnout.
        g.cants = [g.cant_grid(false)?, g.cant_grid(true)?];
        g.u_step = g.cants[0][1] - g.cants[0][0];
        for grid in &g.cants {
            if grid.windows(2).any(|w| ((w[1] - w[0]) - g.u_step).abs() > 1e-9) {
                return Err(GrenzenError(format!("{}: das Überhöhungsraster ist nicht gleichmäßig", katalog.id)));
            }
        }

        g.classify()?;

        // Per design speed, once: everything that depends on nothing else.
        for sw in [false, true] {
            let uf = g.speeds.iter().map(|&v| g.deficiency_bound(v, sw)).collect::<Result<Vec<_>, _>>()?;
            let mut levels = uf.clone();
            levels.sort_by(|a, b| b.total_cmp(a));
            levels.dedup();
            g.uf[sw as usize] = uf;
            g.uf_levels[sw as usize] = levels;
            g.rw_min[sw as usize] = g.speeds.iter().map(|&v| g.jump_bound(v, sw)).collect::<Result<Vec<_>, _>>()?;
        }
        g.forms = [("clothoid", g.form_admitted("clothoid")?), ("bloss", g.form_admitted("bloss")?)];
        Ok(g)
    }

    // ── building ─────────────────────────────────────────────────────────

    fn admits(&self, severity: Option<&str>) -> bool {
        severity.is_some_and(|s| self.katalog.rank(s) <= self.max_rank)
    }

    fn all_admit(&self, rules: &[&Rule], base: &Scope, in_context: Option<&dyn Fn(&str) -> bool>) -> Result<bool, Error> {
        for rule in rules {
            let outcome = self.katalog.evaluate_rule(rule, base, in_context)?;
            if outcome.applied && !self.admits(outcome.severity.as_deref()) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The design speeds the catalogue admits at all — asked of the rules
    /// that read nothing but the speed (LP.ALL.01/02), not read out of their
    /// text, the same way the app's catalogSpeedRange finds them.
    fn design_speeds(&self) -> Result<Vec<f64>, Error> {
        let rules: Vec<&Rule> = self
            .katalog
            .rules_for_scope("element")
            .into_iter()
            .filter(|r| r.froms() == SPEED_ONLY)
            .collect();
        let mut out = Vec::new();
        for v in SPEED_SCAN {
            let v = f64::from(v);
            if self.all_admit(&rules, &self.katalog.scope(&[("element.design_speed", v.into())]), None)? {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// The cants a curve may carry — asked of the rules that read nothing but
    /// the cant (LP.KB.01, LP.KB.03 and, in a turnout, LP.KB.05).
    fn cant_grid(&self, on_switch: bool) -> Result<Vec<f64>, GrenzenError> {
        let rules: Vec<&Rule> = self
            .katalog
            .rules_for_element("circular_arc", None)
            .into_iter()
            .filter(|r| r.froms() == CANT_ONLY)
            .collect();
        let mut grid = Vec::new();
        for half in CANT_SCAN {
            let u = f64::from(half) / 2.0;
            if self.all_admit(&rules, &self.katalog.scope(&[("element.cant", u.into())]), context(on_switch))? {
                grid.push(u);
            }
        }
        if grid.len() < 2 || grid[0] != 0.0 {
            return Err(GrenzenError(format!("{}: kein Überhöhungsraster ab 0 mm", self.katalog.id)));
        }
        Ok(grid)
    }

    fn bound(&self, rule: &Rule) -> Bound {
        let mut admitted = None;
        let mut stopped = false;
        for branch in &rule.evaluation {
            if !self.admits(Some(branch.severity())) {
                stopped = true;
                break;
            }
            match branch {
                Branch::Else(_) => return Bound::Never,
                Branch::If { test, .. } => admitted = Some(test),
            }
        }
        if !stopped {
            return Bound::Never;
        }
        let Some((a, op, b)) = admitted.and_then(|test| test.comparison()) else {
            return Bound::Complex;
        };
        if rule.input(a).is_some() && rule.has_threshold(b) {
            return Bound::Cmp(a, op, b);
        }
        if rule.input(b).is_some() && rule.has_threshold(a) {
            return Bound::Cmp(b, op.flip(), a);
        }
        Bound::Complex
    }

    fn refuse(rule: &Rule, why: &str) -> GrenzenError {
        GrenzenError(format!("{} ({}): {why} — der Optimierer kann diese Regel nicht einhalten", rule.id, rule.title))
    }

    fn rule_index(&self, rule: &Rule) -> usize {
        self.katalog
            .rules()
            .iter()
            .position(|r| std::ptr::eq(r, rule))
            .expect("a rule of the catalogue")
    }

    fn from_of(rule: &Rule, input: Sym) -> Option<&str> {
        match rule.input(input).map(|i| &i.source) {
            Some(Source::From(_, from)) => Some(from),
            _ => None,
        }
    }

    fn classify(&mut self) -> Result<(), GrenzenError> {
        let k = self.katalog;
        for (etype, form) in ELEMENT_KEYS {
            let mut bounds = Vec::new();
            for rule in k.rules_for_element(etype, form) {
                let froms = rule.froms();
                if froms == SPEED_ONLY || froms == CANT_ONLY {
                    continue; // the speed and cant grids
                }
                let (input, op, threshold) = match self.bound(rule) {
                    Bound::Never => continue,
                    Bound::Complex => return Err(Self::refuse(rule, "die maßgebende Zeile ist kein einfacher Vergleich")),
                    Bound::Cmp(i, op, t) => (i, op, t),
                };
                let what = Self::from_of(rule, input);
                let kind = match what {
                    Some("element.length") => Kind::Length,
                    Some("physics.ramp_slope") if etype == "transition_curve" => Kind::Slope,
                    Some("physics.u_f") if etype == "circular_arc" && !op.is_lower() => Kind::Deficiency,
                    _ => return Err(Self::refuse(rule, &format!("begrenzt {}", what.unwrap_or("None")))),
                };
                bounds.push(ElementBound {
                    rule: self.rule_index(rule),
                    op,
                    threshold,
                    kind,
                });
            }
            self.element_bounds.push(bounds);
        }

        for rule in k.rules_for_scope("boundary") {
            let (input, op, threshold) = match self.bound(rule) {
                Bound::Never => continue,
                Bound::Complex => {
                    // A cant that jumps at a joint: every ramp a run hands out
                    // starts and ends on the cant of its neighbours, by construction.
                    if rule.froms().iter().all(|f| CONTINUITY.contains(f)) {
                        continue;
                    }
                    return Err(Self::refuse(rule, "die maßgebende Zeile ist kein einfacher Vergleich"));
                }
                Bound::Cmp(i, op, t) => (i, op, t),
            };
            let what = Self::from_of(rule, input);
            if what != Some("physics.r_w") || !op.is_lower() {
                return Err(Self::refuse(rule, &format!("begrenzt {}", what.unwrap_or("None"))));
            }
            self.jump_rules.push(JumpBound {
                rule: self.rule_index(rule),
                threshold,
            });
        }

        self.ramp_rules = k.rules_for_scope("cant_ramp").into_iter().map(|r| self.rule_index(r)).collect();
        for rule in k.rules() {
            // The vertical alignment (HP) is not something a run builds: it
            // rearranges the plan view and leaves the heights to the profile.
            match rule.scope.as_deref() {
                Some("element" | "boundary" | "cant_ramp") => {}
                Some(scope) if VERTICAL_SCOPES.contains(&scope) => {}
                scope => {
                    return Err(GrenzenError(format!(
                        "{}: Regeln für '{}' kennt der Optimierer nicht",
                        k.id,
                        scope.unwrap_or("None")
                    )));
                }
            }
        }
        Ok(())
    }

    fn bounds_of(&self, etype: &str, form: Option<&str>) -> &[ElementBound] {
        let i = ELEMENT_KEYS.iter().position(|(t, f)| *t == etype && *f == form).expect("an element kind");
        &self.element_bounds[i]
    }

    /// The flat scope an element rule reads, as the app's elementScope fills
    /// it. Only what the run is asking about is set; the rest are the values
    /// of an element that has none of it.
    fn base(&self, v: f64, extra: &[(&str, Value)]) -> Scope {
        let mut values: Vec<(&str, Value)> = vec![
            ("element.design_speed", v.into()),
            ("element.length", 0.0.into()),
            ("element.cant", 0.0.into()),
            ("element.radius", 0.0.into()),
            ("element.transition_form", "".into()),
            ("prev.design_speed", v.into()),
            ("next.design_speed", v.into()),
            ("physics.u_f", 0.0.into()),
            ("physics.delta_u", 0.0.into()),
            ("physics.delta_u_f", 0.0.into()),
            ("physics.ramp_slope", 0.0.into()),
            ("model.reverse_curve_straight", false.into()),
        ];
        values.extend(extra.iter().cloned());
        self.katalog.scope(&values)
    }

    fn deficiency_bound(&self, v: f64, on_switch: bool) -> Result<f64, GrenzenError> {
        let mut limit = f64::INFINITY;
        for b in self.bounds_of("circular_arc", None).iter().filter(|b| b.kind == Kind::Deficiency) {
            let rule = &self.katalog.rules()[b.rule];
            if let Some(value) = num(self.katalog.threshold(rule, b.threshold, &self.base(v, &[]), context(on_switch))?)? {
                limit = limit.min(value);
            }
        }
        if !limit.is_finite() {
            return Err(GrenzenError(format!("{}: kein Überhöhungsfehlbetrag bei {v} km/h", self.katalog.id)));
        }
        Ok(limit)
    }

    /// The smallest comparison radius a curvature jump may have at v.
    fn jump_bound(&self, v: f64, internal: bool) -> Result<f64, GrenzenError> {
        let base = self.katalog.scope(&[
            ("prev.design_speed", v.into()),
            ("next.design_speed", v.into()),
            ("physics.curvature_jump", true.into()),
            ("physics.r_w", 0.0.into()),
            ("prev.cant_end", 0.0.into()),
            ("next.cant_start", 0.0.into()),
        ]);
        let mut need: f64 = 0.0;
        for b in &self.jump_rules {
            let rule = &self.katalog.rules()[b.rule];
            if internal && rule.excludes.iter().any(|e| e == "switch_internal") {
                continue;
            }
            match self.katalog.threshold(rule, b.threshold, &base, None) {
                // No value in the table: no jump admitted.
                Err(Error::OutOfRange(_)) => return Ok(f64::INFINITY),
                Err(e) => return Err(e.into()),
                Ok(value) => {
                    if let Some(value) = num(value)? {
                        need = need.max(value);
                    }
                }
            }
        }
        Ok(need)
    }

    /// May a run hand out a transition of this form? LP.UB.02 asks whether
    /// the ramp matches it, and in this app the ramp is always straight — so
    /// it matches a clothoid only. At the Ermessensgrenze that question is
    /// let past.
    fn form_admitted(&self, form: &str) -> Result<bool, Error> {
        let base = self.katalog.scope(&[
            ("model.ramp_on_transition", true.into()),
            ("model.ramp_form_matches", (form == "clothoid").into()),
        ]);
        let rules: Vec<&Rule> = self
            .ramp_rules
            .iter()
            .map(|&i| &self.katalog.rules()[i])
            .filter(|r| !self.toleriert.contains(&r.id))
            .collect();
        self.all_admit(&rules, &base, None)
    }

    // ── asking ───────────────────────────────────────────────────────────

    fn speed_index(&self, v: f64) -> usize {
        self.speeds
            .iter()
            .position(|&s| s == v)
            .unwrap_or_else(|| panic!("{v} km/h is no design speed of {}", self.katalog.id))
    }

    /// The design speed a run states for a curve that permits v: the fastest
    /// one the catalogue admits that is not above it. None below the slowest
    /// — the catalogue has nothing to say about such a curve.
    pub fn design_speed(&self, v: f64) -> Option<f64> {
        let i = self.speeds.partition_point(|&s| s <= v + 1e-9);
        i.checked_sub(1).map(|i| self.speeds[i])
    }

    pub fn u_max(&self, on_switch: bool) -> f64 {
        *self.cants[on_switch as usize].last().unwrap()
    }

    /// The cant deficiency admitted at design speed v.
    pub fn uf_max(&self, v: f64, on_switch: bool) -> f64 {
        self.uf[on_switch as usize][self.speed_index(v)]
    }

    /// The speed [km/h] a curve of this radius and cant permits.
    ///
    /// Continuous, as the kernel works with it — but the deficiency it is
    /// computed with is the one admitted at the design speed that answer
    /// would be stated at. LP.KB.02 is a step (130 mm, and above 150 km/h up
    /// to 150 mm at the Ermessensgrenze), so each level of the step is tried
    /// from the most generous down, and taken where its own speed lands in
    /// the range it holds for — the same question the app's maxSpeedFor asks.
    pub fn speed(&self, radius: f64, cant: f64, on_switch: bool) -> f64 {
        let levels = &self.uf_levels[on_switch as usize];
        for &level in levels {
            let v = permissible_speed(radius, cant, level);
            if let Some(design) = self.design_speed(v)
                && self.uf_max(design, on_switch) >= level - 1e-9
            {
                return v;
            }
        }
        permissible_speed(radius, cant, *levels.last().unwrap())
    }

    /// The radius at which a curve with this cant reaches v.
    pub fn radius_for(&self, v: f64, cant: f64, on_switch: bool) -> f64 {
        let design = self.design_speed(v).unwrap_or(self.speeds[0]);
        v * v * CANT_DEFICIENCY_COEFF / (cant + self.uf_max(design, on_switch))
    }

    /// The speed a curvature jump with comparison radius r_w leaves a curve
    /// running at v: v itself where the jump passes at v's design speed, else
    /// the fastest design speed below at which it does — and None where it
    /// passes at none (LP.KS.01).
    pub fn jump_speed(&self, r_w: f64, v: f64, internal: bool) -> Option<f64> {
        let design = self.design_speed(v)?;
        let table = &self.rw_min[internal as usize];
        let i = self.speed_index(design);
        if table[i] <= r_w {
            return Some(v);
        }
        (0..i).rev().find(|&j| table[j] <= r_w).map(|j| self.speeds[j])
    }

    /// The comparison radius a curvature jump needs at design speed v.
    pub fn jump_need(&self, v: f64, internal: bool) -> f64 {
        self.rw_min[internal as usize][self.speed_index(v)]
    }

    /// The shortest element of this type the catalogue admits at v (LP.EL.01;
    /// for a straight between reverse curves also LP.EL.02).
    pub fn min_length(&self, etype: &str, v: f64, reverse: bool) -> Result<f64, Error> {
        const FLOOR_INPUTS: [&str; 5] = [
            "element.design_speed",
            "element.length",
            "model.reverse_curve_straight",
            "next.design_speed",
            "prev.design_speed",
        ];
        let form = if etype == "transition_curve" { Some("clothoid") } else { None };
        let base = self.base(v, &[("model.reverse_curve_straight", reverse.into())]);
        let mut need: f64 = 0.0;
        for b in self.bounds_of(etype, form).iter().filter(|b| b.kind == Kind::Length && b.op.is_lower()) {
            let rule = &self.katalog.rules()[b.rule];
            if rule.froms().iter().any(|f| !FLOOR_INPUTS.contains(f)) {
                continue; // depends on more than the speed: not a floor
            }
            if let Some(value) = num(self.katalog.threshold(rule, b.threshold, &base, None)?)? {
                need = need.max(value);
            }
        }
        Ok(need)
    }

    /// Every length and gradient rule of a transition of this form at design
    /// speed v, carrying a cant step `delta_u` and a deficiency step
    /// `delta_uf` [mm], with the bound it sets on the length at this level, in
    /// the catalogue's order — `lower` for a shortest length (LP.EL.01,
    /// LP.UB.03…07), else a longest (LP.UB.08). None where a rule admits no
    /// length at all.
    pub fn ramp_rules(&self, form: &str, v: f64, delta_u: f64, delta_uf: f64) -> Result<Option<Vec<RampRule>>, Error> {
        let base = self.base(
            v,
            &[
                ("element.transition_form", form.into()),
                ("physics.delta_u", delta_u.into()),
                ("physics.delta_u_f", delta_uf.into()),
            ],
        );
        let mut out = Vec::new();
        for b in self.bounds_of("transition_curve", Some(form)) {
            let rule = &self.katalog.rules()[b.rule];
            let value = match self.katalog.threshold(rule, b.threshold, &base, None) {
                Err(Error::OutOfRange(_)) => return Ok(None),
                other => num(other?)?,
            };
            let Some(value) = value else { continue };
            let length = if b.kind == Kind::Length { value } else { value * delta_u / 1000.0 };
            out.push(RampRule {
                id: rule.id.clone(),
                length,
                lower: b.op.is_lower(),
            });
        }
        Ok(Some(out))
    }

    /// The shortest and longest a transition of this form may be at design
    /// speed v (see [`Grenzen::ramp_rules`]).
    pub fn ramp_bounds(&self, form: &str, v: f64, delta_u: f64, delta_uf: f64) -> Result<Option<RampBounds>, Error> {
        let Some(rules) = self.ramp_rules(form, v, delta_u, delta_uf)? else {
            return Ok(None);
        };
        let mut out = RampBounds {
            shortest: 0.0,
            longest: f64::INFINITY,
            by: None,
        };
        for rule in rules {
            if rule.lower {
                if rule.length > out.shortest {
                    out.shortest = rule.length;
                    out.by = Some(rule.id);
                }
            } else {
                out.longest = out.longest.min(rule.length);
            }
        }
        Ok(Some(out))
    }

    /// What a run at this level is held to — the numbers that do not depend on
    /// a curve.
    pub fn describe(&self) -> serde_json::Value {
        let sorted = |values: &[f64]| {
            let mut v = values.to_vec();
            v.sort_by(f64::total_cmp);
            v.dedup();
            v
        };
        json!({
            "stufe": self.stufe.id(),
            "schlechtesteStufe": self.stufe.worst(),
            "geschwindigkeiten": [self.speeds[0], self.v_top],
            "uMax": self.u_max(false), "uMaxWeiche": self.u_max(true), "uStep": self.u_step,
            "ufMax": sorted(&self.uf[0]),
            "ufMaxWeiche": sorted(&self.uf[1]),
            "uebergangsbogen": self.forms.iter().filter(|(_, ok)| *ok).map(|(f, _)| *f).collect::<Vec<_>>(),
        })
    }
}

// A length set by the rules is rounded up to a full decimetre (Entscheidung 176).
const STEPS_PER_M: f64 = 10.0;

/// One rule on a transition's shortest length, as the app lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthBy {
    pub id: String,
    pub length: f64,
    /// Does this rule set the length?
    pub binding: bool,
}

/// The shortest transition at both levels (see [`transition_lengths`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TransitionLengths {
    pub regular: Option<f64>,
    pub minimum: Option<f64>,
    pub regular_by: Vec<LengthBy>,
    pub minimum_by: Vec<LengthBy>,
}

fn shortest(grenzen: &Grenzen, form: &str, v: f64, delta_u: f64, delta_uf: f64) -> Result<(Option<f64>, Vec<LengthBy>), Error> {
    let Some(rules) = grenzen.ramp_rules(form, v, delta_u, delta_uf)? else {
        return Ok((None, Vec::new()));
    };
    let lower: Vec<&RampRule> = rules.iter().filter(|r| r.lower).collect();
    let need = lower.iter().map(|r| r.length).fold(0.0, f64::max);
    let steps = (need * STEPS_PER_M - 1e-3).ceil();
    let length = if steps / STEPS_PER_M >= need - 1e-9 {
        steps / STEPS_PER_M
    } else {
        (steps + 1.0) / STEPS_PER_M
    };
    let mut by: Vec<LengthBy> = lower
        .iter()
        .map(|r| LengthBy {
            id: r.id.clone(),
            length: r.length,
            binding: r.length >= need - 1e-9 && need > 0.0,
        })
        .collect();
    by.sort_by(|a, b| b.length.total_cmp(&a.length));
    Ok((Some(length), by))
}

/// The shortest transition the rules allow between `prev` and `next` (two
/// elements with radius, cant and speed): the Regellänge — every rule on the
/// length at its Regelwert — and the Mindestlänge, down to the Ermessensgrenze
/// where a rule has one (Entscheidung 176). The transition runs from curvature
/// `r1` to the radius `next` starts with, its cant from `prev`'s to `next`'s,
/// at `speed` — Δu and Δu_f read as the app's check reads them.
///
/// The lengths are rounded up to a full decimetre; every rule on the length
/// comes with the bound it sets, the longest first. The same answer as the
/// app's rules/transitionLength.js, which the shared vectors
/// (packages/core/constraints/tests/transition_lengths.json) hold it to; a
/// rule that only a shorter transition keeps (LP.UB.08) is left to the check.
pub fn transition_lengths(
    prev: Option<&Element>,
    next: Option<&Element>,
    r1: Option<f64>,
    form: &str,
    speed: f64,
    reg: &Grenzen,
    discretion: &Grenzen,
) -> Result<TransitionLengths, Error> {
    if speed.is_nan() || speed <= 0.0 {
        return Ok(TransitionLengths::default());
    }
    let straight = Element {
        element_type: Some(0),
        cant: Some(0.0),
        speed: Some(speed),
        ..Element::default()
    };
    let r2 = match next {
        Some(n) if n.is_transition() => n.r1,
        Some(n) => n.radius,
        None => None,
    };
    let transition = Element {
        element_type: Some(2),
        transition_type: Some(form.to_string()),
        r1,
        r2,
        length: Some(0.0),
        speed: Some(speed),
        ..Element::default()
    };
    let chain = [prev.cloned().unwrap_or_else(|| straight.clone()), transition, next.cloned().unwrap_or(straight)];
    let scope = element_scope(&chain, 1, Context::default());
    let get = |name: &str| scope.iter().find(|(n, _)| *n == name).and_then(|(_, v)| v.as_num()).unwrap_or(0.0);
    let (du, duf) = (get("physics.delta_u"), get("physics.delta_u_f"));
    let (regular, regular_by) = shortest(reg, form, speed, du, duf)?;
    let (minimum, minimum_by) = shortest(discretion, form, speed, du, duf)?;
    Ok(TransitionLengths {
        regular,
        minimum,
        regular_by,
        minimum_by,
    })
}
