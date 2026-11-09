// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A rule catalogue — DB Ril 800.0110 Linienführung as the repo holds it
//! (packages/core/constraints/db-ril-800-0110.json), read and applied. The Rust
//! half of the app's packages/core/src/rules/regelkatalog.js, and meant to
//! read like it: the same file, the same expression language ([`crate::expr`]),
//! the same evaluation order, so the optimizer builds to exactly the limits the
//! app's element table judges by.
//!
//! The catalogue is not compiled in: the app hands over the file it bundles,
//! so the two can never hold different rules.
//!
//! This module knows nothing about tracks. It is handed a scope of the names a
//! rule declares (`element.design_speed`, `physics.u_f`, …) and gives back a
//! severity, or one threshold. [`crate::grenzen`] is what turns that into the
//! bounds a run is held to.

use crate::expr::{Basic, Error, Expr, Functions, NoFunctions, Scope, Sym, Symbols, Value};
use serde_json::Value as Json;
use std::collections::HashMap;

/// Where an input of a rule comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// A name of the model (`element.length`), kept as its slot and its text.
    From(Sym, String),
    /// An expression over what came before.
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub name: Sym,
    pub source: Source,
}

/// One line of a rule's `evaluation`.
#[derive(Debug, Clone)]
pub enum Branch {
    If { test: Expr, severity: String },
    Else(String),
}

impl Branch {
    pub fn severity(&self) -> &str {
        match self {
            Branch::If { severity, .. } | Branch::Else(severity) => severity,
        }
    }
}

/// One rule of the catalogue.
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub title: String,
    /// What it judges: 'element', 'boundary', 'cant_ramp', …
    pub scope: Option<String>,
    pub element_types: Option<Vec<String>>,
    pub forms: Option<Vec<String>>,
    pub excludes: Vec<String>,
    pub inputs: Vec<Input>,
    pub condition: Option<Expr>,
    pub thresholds: Vec<(Sym, Expr)>,
    pub evaluation: Vec<Branch>,
}

impl Rule {
    /// The model names the rule reads its inputs from — enough to tell what
    /// kind of object it judges and what it bounds.
    pub fn froms(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .inputs
            .iter()
            .filter_map(|i| match &i.source {
                Source::From(_, from) => Some(from.as_str()),
                Source::Expr(_) => None,
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// The input `name`, if the rule has one.
    pub fn input(&self, name: Sym) -> Option<&Input> {
        self.inputs.iter().find(|i| i.name == name)
    }

    pub fn has_threshold(&self, name: Sym) -> bool {
        self.thresholds.iter().any(|(t, _)| *t == name)
    }
}

/// The result of one rule on one object. `applied: false` means the rule
/// does not speak about the object, which is not the same as `ok`.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub applied: bool,
    pub severity: Option<String>,
    pub values: Scope,
    pub out_of_range: bool,
}

#[derive(Debug, Clone)]
struct Piece {
    low: Option<(f64, bool)>,
    high: Option<f64>,
    expr: Expr,
}

#[derive(Debug, Clone)]
enum KeyMode {
    NextHigher,
    Formula {
        key_max: Option<f64>,
        expr: Expr,
        inputs: Vec<(Sym, String)>,
        rounding: Option<f64>,
    },
}

#[derive(Debug, Clone)]
enum Table {
    Piecewise {
        key: Sym,
        pieces: Vec<Piece>,
        out_of_range: String,
    },
    Speed {
        key: String,
        key_sym: Sym,
        rows: Vec<HashMap<String, f64>>,
        modes: HashMap<String, KeyMode>,
        out_of_range: String,
    },
}

fn catalogue_error(msg: impl Into<String>) -> Error {
    Error::Catalogue(msg.into())
}

fn text(value: &Json, what: &str) -> Result<String, Error> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| catalogue_error(format!("{what} is not a string")))
}

fn strings(value: Option<&Json>) -> Option<Vec<String>> {
    value
        .and_then(Json::as_array)
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
}

fn number(value: Option<&Json>) -> Option<f64> {
    value.and_then(Json::as_f64)
}

/// One catalogue file, loaded.
#[derive(Debug, Clone)]
pub struct Katalog {
    pub id: String,
    pub title: String,
    pub version: String,
    symbols: Symbols,
    rank: HashMap<String, i32>,
    rules: Vec<Rule>,
    tables: HashMap<String, Table>,
    physics: HashMap<String, f64>,
}

impl Katalog {
    /// The catalogue `data` with the physics provider `physics` (the one
    /// number the catalogue asks of the physics document is u0_factor).
    pub fn new(data: &Json, physics: HashMap<String, f64>) -> Result<Katalog, Error> {
        let mut symbols = Symbols::default();
        let catalog = data.get("catalog").ok_or_else(|| catalogue_error("no catalog"))?;
        let rank = data
            .get("severity_levels")
            .and_then(Json::as_array)
            .ok_or_else(|| catalogue_error("no severity_levels"))?
            .iter()
            .map(|s| Ok((text(&s["id"], "severity id")?, s["rank"].as_i64().unwrap_or(-1) as i32)))
            .collect::<Result<HashMap<_, _>, Error>>()?;

        let mut tables = HashMap::new();
        if let Some(map) = data.get("tables").and_then(Json::as_object) {
            for (id, table) in map {
                tables.insert(id.clone(), read_table(id, table, &mut symbols)?);
            }
        }

        let rules = data
            .get("rules")
            .and_then(Json::as_array)
            .ok_or_else(|| catalogue_error("no rules"))?
            .iter()
            .map(|r| read_rule(r, &mut symbols))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Katalog {
            id: text(&catalog["id"], "catalog id")?,
            title: catalog["title"].as_str().unwrap_or_default().to_string(),
            version: catalog["katalog_version"].as_str().unwrap_or_default().to_string(),
            symbols,
            rank,
            rules,
            tables,
            physics,
        })
    }

    // ── reading ──────────────────────────────────────────────────────────

    pub fn symbols(&self) -> &Symbols {
        &self.symbols
    }

    /// Rank of a severity — higher is worse; -1 for none.
    pub fn rank(&self, severity: &str) -> i32 {
        self.rank.get(severity).copied().unwrap_or(-1)
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn rule(&self, id: &str) -> Result<&Rule, Error> {
        self.rules
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| catalogue_error(format!("unknown rule {id}")))
    }

    /// The rules written for one kind of object: 'element', 'boundary', 'cant_ramp'.
    pub fn rules_for_scope(&self, scope: &str) -> Vec<&Rule> {
        self.rules.iter().filter(|r| r.scope.as_deref() == Some(scope)).collect()
    }

    /// The element rules that speak about this kind of element. A rule
    /// without `element_types` speaks about all of them; one that names
    /// `forms` speaks only about a transition curve of that form.
    pub fn rules_for_element(&self, type_id: &str, form: Option<&str>) -> Vec<&Rule> {
        self.rules_for_scope("element")
            .into_iter()
            .filter(|r| r.element_types.as_ref().is_none_or(|t| t.iter().any(|t| t == type_id)))
            .filter(|r| r.forms.as_ref().is_none_or(|f| form.is_some_and(|form| f.iter().any(|f| f == form))))
            .collect()
    }

    /// A scope with `values` set — the names no rule reads are left out.
    pub fn scope(&self, values: &[(&str, Value)]) -> Scope {
        let mut scope = Scope::new(self.symbols.len());
        for (name, value) in values {
            if let Some(sym) = self.symbols.get(name) {
                scope.set(sym, value.clone());
            }
        }
        scope
    }

    // ── applying ─────────────────────────────────────────────────────────

    fn fns<'a>(&'a self, in_context: Option<&'a dyn Fn(&str) -> bool>) -> KatalogFunctions<'a> {
        KatalogFunctions { katalog: self, in_context }
    }

    fn inputs(&self, rule: &Rule, base: &Scope, fns: &dyn Functions) -> Result<Scope, Error> {
        let mut scope = base.clone();
        for input in &rule.inputs {
            let value = match &input.source {
                Source::Expr(expr) => expr.eval(&scope, fns)?,
                Source::From(sym, from) => scope
                    .get(*sym)
                    .cloned()
                    .ok_or_else(|| catalogue_error(format!("{}: nothing provides {from}", rule.id)))?,
            };
            scope.set(input.name, value);
        }
        Ok(scope)
    }

    /// Apply one rule to one object: inputs, then the condition, then the
    /// thresholds in the order the file lists them (a later one may use an
    /// earlier), then the first matching line of `evaluation`.
    pub fn evaluate_rule(&self, rule: &Rule, base: &Scope, in_context: Option<&dyn Fn(&str) -> bool>) -> Result<Outcome, Error> {
        let fns = self.fns(in_context);
        let mut scope = self.inputs(rule, base, &fns)?;
        let decided = (|| -> Result<Option<String>, Error> {
            if let Some(condition) = &rule.condition
                && !condition.eval(&scope, &fns)?.truthy()
            {
                return Ok(None);
            }
            for (name, expr) in &rule.thresholds {
                let value = expr.eval(&scope, &fns)?;
                scope.set(*name, value);
            }
            for branch in &rule.evaluation {
                match branch {
                    Branch::Else(severity) => return Ok(Some(severity.clone())),
                    Branch::If { test, severity } => {
                        if test.eval(&scope, &fns)?.truthy() {
                            return Ok(Some(severity.clone()));
                        }
                    }
                }
            }
            // A catalogue whose last line is not an `else` leaves a case unanswered.
            Err(catalogue_error(format!("{}: no branch of evaluation matched", rule.id)))
        })();
        match decided {
            Ok(severity) => Ok(Outcome {
                applied: severity.is_some(),
                severity,
                values: scope,
                out_of_range: false,
            }),
            Err(Error::OutOfRange(severity)) => Ok(Outcome {
                applied: true,
                severity: Some(severity),
                values: scope,
                out_of_range: true,
            }),
            Err(e) => Err(e),
        }
    }

    /// One threshold of one rule for one object — the number the checker
    /// would really compare against. None where the rule does not speak about
    /// the object (its condition is false); an out-of-range table propagates,
    /// so the caller decides what a missing value means for it.
    ///
    /// Stops at `name`: the optimizer asks for one bound in its innermost
    /// loop, and the thresholds after it are not needed to compute it.
    pub fn threshold(&self, rule: &Rule, name: Sym, base: &Scope, in_context: Option<&dyn Fn(&str) -> bool>) -> Result<Option<Value>, Error> {
        let fns = self.fns(in_context);
        let mut scope = self.inputs(rule, base, &fns)?;
        if let Some(condition) = &rule.condition
            && !condition.eval(&scope, &fns)?.truthy()
        {
            return Ok(None);
        }
        for (key, expr) in &rule.thresholds {
            let value = expr.eval(&scope, &fns)?;
            if *key == name {
                return Ok(Some(value));
            }
            scope.set(*key, value);
        }
        Err(catalogue_error(format!("{} has no threshold {}", rule.id, self.symbols.name(name))))
    }

    fn lookup(&self, table_id: &str, key: f64, column: Option<&str>) -> Result<Value, Error> {
        let table = self.tables.get(table_id).ok_or_else(|| catalogue_error(format!("unknown table {table_id}")))?;
        match table {
            Table::Piecewise {
                key: key_sym,
                pieces,
                out_of_range,
            } => {
                for piece in pieces {
                    let low_ok = match piece.low {
                        Some((low, true)) => key > low,
                        Some((low, false)) => key >= low,
                        None => true,
                    };
                    let high_ok = piece.high.is_none_or(|high| key <= high);
                    if low_ok && high_ok {
                        let mut scope = Scope::new(self.symbols.len());
                        scope.set(*key_sym, Value::Num(key));
                        return piece.expr.eval(&scope, &NoFunctions);
                    }
                }
                Err(Error::OutOfRange(out_of_range.clone()))
            }
            Table::Speed {
                key: key_name,
                key_sym,
                rows,
                modes,
                out_of_range,
            } => {
                // A tabulated speed always gives the tabulated value; `key_mode`
                // only governs what happens between two rows, and past the last
                // row there is no value at all.
                let column = column.ok_or_else(|| catalogue_error(format!("{table_id} needs a column")))?;
                let cell = |row: &HashMap<String, f64>| {
                    row.get(column)
                        .copied()
                        .map(Value::Num)
                        .ok_or_else(|| catalogue_error(format!("{table_id} has no column {column}")))
                };
                if let Some(row) = rows.iter().find(|r| r.get(key_name) == Some(&key)) {
                    return cell(row);
                }
                let Some(higher) = rows.iter().find(|r| r.get(key_name).is_some_and(|k| *k > key)) else {
                    return Err(Error::OutOfRange(out_of_range.clone()));
                };
                if let Some(KeyMode::Formula {
                    key_max,
                    expr,
                    inputs,
                    rounding,
                }) = modes.get(column)
                    && key_max.is_none_or(|max| key <= max)
                {
                    let mut scope = Scope::new(self.symbols.len());
                    scope.set(*key_sym, Value::Num(key));
                    for (name, from) in inputs {
                        let (provider, field) = from.split_once('.').unwrap_or((from, ""));
                        let value = (provider == "physics")
                            .then(|| self.physics.get(field))
                            .flatten()
                            .ok_or_else(|| catalogue_error(format!("no value for {from}")))?;
                        scope.set(*name, Value::Num(*value));
                    }
                    let value = expr.eval(&scope, &NoFunctions)?;
                    return match (rounding, value) {
                        (Some(step), Value::Num(v)) => Ok(Value::Num((v / step + 0.5).floor() * step)),
                        (_, value) => Ok(value),
                    };
                }
                cell(higher)
            }
        }
    }
}

/// The functions the catalogue's expressions may call.
struct KatalogFunctions<'a> {
    katalog: &'a Katalog,
    in_context: Option<&'a dyn Fn(&str) -> bool>,
}

impl Functions for KatalogFunctions<'_> {
    fn call(&self, name: &str, args: &[Value]) -> Result<Value, Error> {
        match name {
            "lookup" => {
                let (Some(Value::Str(table)), Some(Value::Num(key))) = (args.first(), args.get(1)) else {
                    return Err(Error::Expr(format!("lookup takes a table and a key: {args:?}")));
                };
                let column = match args.get(2) {
                    Some(Value::Str(c)) => Some(&**c),
                    None => None,
                    Some(other) => return Err(Error::Expr(format!("lookup column is not a string: {other:?}"))),
                };
                self.katalog.lookup(table, *key, column)
            }
            "in_context" => {
                let Some(Value::Str(id)) = args.first() else {
                    return Err(Error::Expr("in_context takes a context id".into()));
                };
                Ok(Value::Bool(self.in_context.is_some_and(|f| f(id))))
            }
            _ => Basic.call(name, args),
        }
    }
}

fn read_table(id: &str, table: &Json, symbols: &mut Symbols) -> Result<Table, Error> {
    let out_of_range = text(&table["out_of_range"], "out_of_range")?;
    let key = text(&table["key"], "table key")?;
    let key_sym = symbols.intern(&key);
    match table["type"].as_str() {
        Some("piecewise") => {
            let pieces = table["pieces"]
                .as_array()
                .ok_or_else(|| catalogue_error(format!("{id}: no pieces")))?
                .iter()
                .map(|p| {
                    let low = number(p.get("key_min"))
                        .map(|k| (k, false))
                        .or_else(|| number(p.get("key_min_exclusive")).map(|k| (k, true)));
                    Ok(Piece {
                        low,
                        high: number(p.get("key_max")),
                        expr: Expr::parse(&text(&p["expr"], "piece expr")?, symbols)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(Table::Piecewise {
                key: key_sym,
                pieces,
                out_of_range,
            })
        }
        Some("speed_table") => {
            let rows = table["rows"]
                .as_array()
                .ok_or_else(|| catalogue_error(format!("{id}: no rows")))?
                .iter()
                .map(|row| {
                    row.as_object()
                        .map(|o| o.iter().filter_map(|(k, v)| v.as_f64().map(|v| (k.clone(), v))).collect())
                        .ok_or_else(|| catalogue_error(format!("{id}: a row is not an object")))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let mut modes = HashMap::new();
            if let Some(map) = table.get("key_mode").and_then(Json::as_object) {
                for (column, mode) in map {
                    if !mode.is_object() {
                        continue; // the note
                    }
                    let parsed = match mode["mode"].as_str() {
                        Some("formula") => KeyMode::Formula {
                            key_max: number(mode.get("key_max")),
                            expr: Expr::parse(&text(&mode["expr"], "key_mode expr")?, symbols)?,
                            inputs: mode
                                .get("inputs")
                                .and_then(Json::as_object)
                                .map(|o| {
                                    o.iter()
                                        .map(|(name, spec)| Ok((symbols.intern(name), text(&spec["from"], "from")?)))
                                        .collect::<Result<Vec<_>, Error>>()
                                })
                                .transpose()?
                                .unwrap_or_default(),
                            rounding: number(mode.get("rounding")),
                        },
                        _ => KeyMode::NextHigher,
                    };
                    modes.insert(column.clone(), parsed);
                }
            }
            Ok(Table::Speed {
                key,
                key_sym,
                rows,
                modes,
                out_of_range,
            })
        }
        other => Err(catalogue_error(format!("{id}: unknown table type {other:?}"))),
    }
}

fn read_rule(rule: &Json, symbols: &mut Symbols) -> Result<Rule, Error> {
    let id = text(&rule["id"], "rule id")?;
    let applies = rule.get("applies_to");
    let inputs = rule
        .get("inputs")
        .and_then(Json::as_object)
        .map(|o| {
            o.iter()
                .map(|(name, spec)| {
                    let source = if let Some(expr) = spec.get("expr").and_then(Json::as_str) {
                        Source::Expr(Expr::parse(expr, symbols)?)
                    } else {
                        let from = text(&spec["from"], &format!("{id}: input {name}"))?;
                        Source::From(symbols.intern(&from), from)
                    };
                    Ok(Input {
                        name: symbols.intern(name),
                        source,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()?
        .unwrap_or_default();
    let condition = rule.get("condition").and_then(Json::as_str).map(|c| Expr::parse(c, symbols)).transpose()?;
    let thresholds = rule
        .get("thresholds")
        .and_then(Json::as_object)
        .map(|o| {
            o.iter()
                .map(|(name, spec)| Ok((symbols.intern(name), Expr::parse(&text(&spec["expr"], "threshold")?, symbols)?)))
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()?
        .unwrap_or_default();
    let evaluation = rule["evaluation"]
        .as_array()
        .ok_or_else(|| catalogue_error(format!("{id}: no evaluation")))?
        .iter()
        .map(|entry| {
            if let Some(severity) = entry.get("else") {
                return Ok(Branch::Else(text(severity, "else")?));
            }
            Ok(Branch::If {
                test: Expr::parse(&text(&entry["if"], "if")?, symbols)?,
                severity: text(&entry["severity"], "severity")?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Rule {
        title: rule.get("title").and_then(Json::as_str).unwrap_or_default().to_string(),
        scope: applies.and_then(|a| a.get("scope")).and_then(Json::as_str).map(str::to_string),
        element_types: strings(applies.and_then(|a| a.get("element_types"))),
        forms: strings(applies.and_then(|a| a.get("forms"))),
        excludes: strings(applies.and_then(|a| a.get("excludes"))).unwrap_or_default(),
        id,
        inputs,
        condition,
        thresholds,
        evaluation,
    })
}
