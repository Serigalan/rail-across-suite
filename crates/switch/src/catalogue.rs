// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The switch and crossing forms of DB Ril 800.0120 and what the geometry
//! reads of them (`catalogue.js`).

use std::sync::LazyLock;

use rail_across_js::{get, js_round, number, truthy};
use serde_json::{Value, json};

const FILE: &str = include_str!("../../../packages/core/constraints/db-ril-800-0120.json");

/// The rulebook itself (`SWITCH_CATALOGUE`).
pub static SWITCH_CATALOGUE: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(FILE).expect("db-ril-800-0120.json"));

fn formen() -> &'static [Value] {
    SWITCH_CATALOGUE["formen"].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn f(v: &Value, k: &str) -> Option<f64> {
    get(v, k).and_then(Value::as_f64)
}

fn turnouts(klasse: &str) -> Vec<&'static Value> {
    formen()
        .iter()
        .filter(|x| s(x, "art") == Some("weiche") && s(x, "klasse") == Some(klasse))
        .collect()
}

/// The Regelformen (`SWITCH_TYPES`).
pub static SWITCH_TYPES: LazyLock<Vec<&'static Value>> = LazyLock::new(|| turnouts("regel"));
/// The Sonderbauformen (`SWITCH_TYPES_SPECIAL`).
pub static SWITCH_TYPES_SPECIAL: LazyLock<Vec<&'static Value>> = LazyLock::new(|| turnouts("sonderbauform"));
/// Every turnout form a dialog offers (`SWITCH_PICK_TYPES`).
pub static SWITCH_PICK_TYPES: LazyLock<Vec<&'static Value>> = LazyLock::new(|| SWITCH_TYPES.iter().chain(SWITCH_TYPES_SPECIAL.iter()).copied().collect());
/// Every turnout form whatever its class (`ALL_SWITCH_TYPES`).
pub static ALL_SWITCH_TYPES: LazyLock<Vec<&'static Value>> = LazyLock::new(|| formen().iter().filter(|x| s(x, "art") == Some("weiche")).collect());
/// Crossings and crossing switches (`CROSSING_TYPES`).
pub static CROSSING_TYPES: LazyLock<Vec<&'static Value>> = LazyLock::new(|| formen().iter().filter(|x| s(x, "art") == Some("kreuzung")).collect());

/// What a picker starts on (`DEFAULT_SWITCH_TYPE_IDX`).
pub fn default_switch_type_idx() -> i64 {
    SWITCH_PICK_TYPES
        .iter()
        .position(|x| s(x, "label") == Some("300 – 1:9"))
        .map(|i| i as i64)
        .unwrap_or(-1)
}

/// The stages a switch connection falls back through (`SWITCH_CONNECTION_STAGES`).
pub static SWITCH_CONNECTION_STAGES: LazyLock<[Vec<&'static Value>; 3]> = LazyLock::new(|| {
    [1.0, 2.0, 3.0].map(|n| {
        let mut v: Vec<&Value> = formen().iter().filter(|x| x.get("verbindung").and_then(|c| f(c, "stufe")) == Some(n)).collect();
        v.sort_by(|a, b| {
            let r = |x: &Value| x["verbindung"].get("rang").and_then(Value::as_f64).unwrap_or(f64::NAN);
            (r(a) - r(b)).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    })
});

/// Any form looked up by its label (`switchTypeByLabel`).
pub fn switch_type_by_label(label: Option<&str>) -> Option<&'static Value> {
    let label = label?;
    formen().iter().find(|x| s(x, "label") == Some(label))
}

/// The speed a turnout allows on its branch (`switchBranchSpeed`).
pub fn switch_branch_speed(sw: &Value) -> Option<f64> {
    match sw.get("speed").and_then(Value::as_f64) {
        Some(v) if v > 0.0 => Some(v),
        _ => switch_type_by_label(sw.get("label").and_then(Value::as_str)).and_then(|t| f(t, "speed")),
    }
}

pub fn switch_arc_length(r: f64, ratio: f64) -> f64 {
    r * (1.0 / ratio).atan()
}

/// The kind of a section of a branch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Arc,
    Straight,
    Clothoid,
}

/// A section of a branch: `r` None for a straight one; `r_end` a clothoid's end radius.
#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub kind: Kind,
    pub r: Option<f64>,
    pub r_end: Option<f64>,
    pub length: f64,
}

impl Section {
    pub fn to_json(self) -> Value {
        let opt = |x: Option<f64>| x.map(number).unwrap_or(Value::Null);
        match self.kind {
            Kind::Clothoid => json!({ "type": "clothoid", "R": opt(self.r), "REnd": opt(self.r_end), "length": number(self.length) }),
            Kind::Arc => json!({ "type": "arc", "R": opt(self.r), "length": number(self.length) }),
            Kind::Straight => json!({ "type": "straight", "R": Value::Null, "length": number(self.length) }),
        }
    }
}

fn kappa(r: Option<f64>) -> f64 {
    match r {
        Some(r) if r != 0.0 => 1.0 / r,
        _ => 0.0,
    }
}

fn stated_sections(t: &Value) -> Vec<Section> {
    t["branch"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|sec| match s(sec, "type") {
            Some("straight") => Section {
                kind: Kind::Straight,
                r: None,
                r_end: None,
                length: f(sec, "length").unwrap_or(f64::NAN),
            },
            Some("clothoid") => Section {
                kind: Kind::Clothoid,
                r: f(sec, "R1"),
                r_end: f(sec, "R2"),
                length: f(sec, "length").unwrap_or(f64::NAN),
            },
            _ => {
                let r = f(sec, "R").or_else(|| f(t, "R")).unwrap_or(f64::NAN);
                let length = f(sec, "length").unwrap_or_else(|| switch_arc_length(r, f(sec, "ratio").or_else(|| f(t, "ratio")).unwrap_or(f64::NAN)));
                Section {
                    kind: Kind::Arc,
                    r: Some(r),
                    r_end: None,
                    length,
                }
            }
        })
        .collect()
}

/// A point of a branch laid from WA along +x (`branchPoint`): (x, y, angle).
pub fn branch_point(sections: &[Section], up_to: f64) -> (f64, f64, f64) {
    let (mut x, mut y, mut th, mut run) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for sec in sections {
        if run >= up_to {
            break;
        }
        let full = sec.length;
        let l = full.min(up_to - run);
        run += l;
        if sec.kind == Kind::Clothoid {
            let (k0, k1) = (kappa(sec.r), kappa(sec.r_end));
            let n = 200;
            let angle = |s: f64| th + k0 * s + (k1 - k0) * s * s / (2.0 * full);
            let (mut sx, mut sy) = (0.0, 0.0);
            for i in 0..=n {
                let w = if i == 0 || i == n {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                let a = angle(l * i as f64 / n as f64);
                sx += w * a.cos();
                sy += w * a.sin();
            }
            x += sx * l / (3.0 * n as f64);
            y += sy * l / (3.0 * n as f64);
            th = angle(l);
        } else if let Some(r) = sec.r.filter(|r| *r != 0.0 && !r.is_nan()) {
            let dth = l / r;
            x += r * ((th + dth).sin() - th.sin());
            y += r * (th.cos() - (th + dth).cos());
            th += dth;
        } else {
            x += l * th.cos();
            y += l * th.sin();
        }
    }
    (x, y, th)
}

fn to_we(t: &Value, sections: Vec<Section>) -> Vec<Section> {
    let s_we = f(t, "s_we").unwrap_or(f64::NAN);
    let Some(last) = sections.last().copied() else { return sections };
    if !(s_we > 0.0) || last.kind != Kind::Clothoid || last.r_end.is_some() {
        return sections;
    }
    let end = branch_point(&sections, f64::INFINITY);
    if end.1 <= s_we {
        let mut out = sections;
        out.push(Section {
            kind: Kind::Straight,
            r: None,
            r_end: None,
            length: (s_we - end.1) / end.2.sin(),
        });
        return out;
    }
    let head = &sections[..sections.len() - 1];
    let k0 = kappa(last.r);
    let piece = |u: f64| Section {
        kind: Kind::Clothoid,
        r: last.r,
        r_end: Some(1.0 / (k0 * (1.0 - u / last.length))),
        length: u,
    };
    let (mut lo, mut hi) = (0.0, last.length);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        let mut trial = head.to_vec();
        trial.push(piece(mid));
        if branch_point(&trial, f64::INFINITY).1 < s_we {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut out = head.to_vec();
    out.push(piece((lo + hi) / 2.0));
    out
}

/// The branch of a form as its sections (`switchBranchSections`).
pub fn switch_branch_sections(t: Option<&Value>) -> Vec<Section> {
    let Some(t) = t.filter(|t| truthy(Some(t))) else { return vec![] };
    if !truthy(t.get("branch")) {
        let r = f(t, "R").unwrap_or(f64::NAN);
        let div = if truthy(t.get("symmetric")) { 2.0 } else { 1.0 };
        let length = switch_arc_length(r, f(t, "ratio").unwrap_or(f64::NAN)) / div;
        return vec![Section {
            kind: Kind::Arc,
            r: Some(r),
            r_end: None,
            length,
        }];
    }
    to_we(t, stated_sections(t))
}

/// Is the branch more than one arc and a straight end piece (`hasCompoundBranch`)?
pub fn has_compound_branch(t: Option<&Value>) -> bool {
    switch_branch_sections(t).iter().filter(|s| s.kind != Kind::Straight).count() > 1
}

/// A point of the branch `s` from WA (`switchBranchPointAt`).
pub fn switch_branch_point_at(t: Option<&Value>, s: f64) -> (f64, f64) {
    let (x, y, _) = branch_point(&switch_branch_sections(t), s);
    (x, y)
}

/// The tangents t1, t2 of a turnout (`switchTangents`).
pub fn switch_tangents(t: Option<&Value>) -> (f64, f64) {
    let (x, y, angle) = branch_point(&switch_branch_sections(t), f64::INFINITY);
    let t2 = y / angle.sin();
    (x - t2 * angle.cos(), t2)
}

/// Length of the whole branch (`switchBranchLength`).
pub fn switch_branch_length(t: Option<&Value>) -> f64 {
    switch_branch_sections(t).iter().map(|s| s.length).sum()
}

/// Length of the through route (`switchStraightLength`).
pub fn switch_straight_length(t: &Value) -> f64 {
    if truthy(t.get("symmetric")) {
        return switch_branch_length(Some(t));
    }
    if has_compound_branch(Some(t)) {
        let (t1, t2) = switch_tangents(Some(t));
        return t1 + t2;
    }
    switch_branch_sections(Some(t))
        .iter()
        .map(|s| match s.r {
            None => s.length,
            Some(r) => 2.0 * r * (s.length / (2.0 * r)).tan(),
        })
        .sum()
}

/// The Weichenmarke constructed (`switchMarkDistance`).
pub fn switch_mark_distance(t: &Value) -> f64 {
    let ratio = f(t, "ratio").unwrap_or(f64::NAN);
    let w = (1.0 / ratio).atan();
    let straight: f64 = t["branch"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s.get("type").and_then(Value::as_str) == Some("straight"))
        .map(|s| f(s, "length").unwrap_or(f64::NAN))
        .sum();
    let lt2 = if has_compound_branch(Some(t)) {
        switch_tangents(Some(t)).1
    } else {
        f(t, "R").unwrap_or(f64::NAN) * (w / 2.0).tan() + straight
    };
    let d = 2.272 * ratio - lt2;
    0.30f64.max(js_round((d - 0.30) / 0.60) * 0.60 + 0.30)
}

/// The Weichenmarke of a form (`switchMark`).
pub fn switch_mark(form: Option<&Value>) -> Option<f64> {
    let form = form.filter(|v| truthy(Some(v)))?;
    if let Some(d) = get(form, "dLcs") {
        return d.as_f64();
    }
    (s(form, "art") == Some("weiche") && truthy(form.get("R")) && truthy(form.get("ratio"))).then(|| switch_mark_distance(form))
}

/// The branch as `{ length, signedR }` signed by the side (`switchFormChain`).
pub fn switch_form_chain(t: Option<&Value>, side: &str) -> Vec<Value> {
    let sign = if side == "left" { -1.0 } else { 1.0 };
    let signed = |r: Option<f64>| r.map(|r| number(sign * r)).unwrap_or(Value::Null);
    switch_branch_sections(t)
        .into_iter()
        .map(|s| match s.kind {
            Kind::Clothoid => json!({ "length": number(s.length), "signedR": signed(s.r), "signedREnd": signed(s.r_end) }),
            _ => json!({ "length": number(s.length), "signedR": signed(s.r) }),
        })
        .collect()
}

pub const STRAIGHT_CURVATURE: f64 = 1e-7;

/// A radius usable as one (`asRadius`).
pub fn as_radius(r: Option<f64>) -> Option<f64> {
    r.filter(|r| r.is_finite() && *r != 0.0)
}

/// Branch radius of a bent switch (`branchRadius`).
pub fn branch_radius(form_signed_r: f64, stem_signed_r: Option<f64>) -> Option<f64> {
    let k = stem_signed_r.filter(|r| *r != 0.0 && !r.is_nan()).map(|r| 1.0 / r).unwrap_or(0.0) + 1.0 / form_signed_r;
    if k.abs() < STRAIGHT_CURVATURE { None } else { Some(1.0 / k) }
}

/// Which bent form a stem radius produces (`bentForm`).
pub fn bent_form(stem_signed_r: Option<f64>, branch_signed_r: Option<f64>) -> &'static str {
    let Some(stem) = stem_signed_r.filter(|r| *r != 0.0 && !r.is_nan()) else {
        return "plain";
    };
    let Some(branch) = branch_signed_r else { return "abw_straight" };
    if stem.signum() == branch.signum() { "ibw" } else { "abw" }
}

/// The variants a form states (`switchVariants`).
pub fn switch_variants(form: Option<&Value>) -> Vec<Value> {
    form.and_then(|f| get(f, "varianten")).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn rail_weight(name: Option<&Value>) -> Option<String> {
    let text = match name {
        None | Some(Value::Null) => String::new(),
        Some(v) => rail_across_js::js_string(Some(v)),
    };
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    (!digits.is_empty()).then_some(digits)
}

/// The variant a turnout on a rail profile is built as (`switchVariantFor`).
pub fn switch_variant_for(form: Option<&Value>, rail: Option<&Value>) -> Option<Value> {
    let variants = switch_variants(form);
    if variants.is_empty() {
        return None;
    }
    let weight = rail_weight(rail);
    let rank = |v: &Value| (if s(v, "schwellen") == Some("beton") { 0 } else { 2 }) + (if s(v, "klasse") == Some("regel") { 0 } else { 1 });
    let mut fitting: Vec<&Value> = variants.iter().filter(|v| weight.is_some() && rail_weight(v.get("profil")) == weight).collect();
    fitting.sort_by_key(|v| rank(v));
    Some(fitting.first().map(|v| (*v).clone()).unwrap_or_else(|| variants[0].clone()))
}
