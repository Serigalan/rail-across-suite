// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The domain schemas checked as JSON Schema (`schemaCheck.js`) — the part of
//! it they use, no more, so the JavaScript, the Python and this core read them
//! exactly alike. A schema that uses another keyword is refused when it is
//! added rather than half understood.

use std::collections::HashMap;

use rail_across_js::{as_f64, es_number, stringify};
use regex::Regex;
use serde_json::Value;

/// The keywords understood; annotations (`x-…`) are ignored.
pub const KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "$ref",
    "$defs",
    "$comment",
    "title",
    "description",
    "type",
    "enum",
    "const",
    "properties",
    "required",
    "additionalProperties",
    "patternProperties",
    "minProperties",
    "items",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "minLength",
    "pattern",
    "oneOf",
    "anyOf",
    "allOf",
];

/// Every keyword of `schema` this check does not understand, as "path: keyword".
pub fn unknown_keywords(schema: &Value, path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(obj) = schema.as_object() else { return out };
    for (k, v) in obj {
        if !KEYWORDS.contains(&k.as_str()) && !k.starts_with("x-") {
            out.push(format!("{}: {k}", if path.is_empty() { "/" } else { path }));
        }
        match k.as_str() {
            "properties" | "patternProperties" | "$defs" => {
                if let Some(subs) = v.as_object() {
                    for (name, sub) in subs {
                        out.extend(unknown_keywords(sub, &format!("{path}/{k}/{name}")));
                    }
                }
            }
            "items" | "additionalProperties" => out.extend(unknown_keywords(v, &format!("{path}/{k}"))),
            "oneOf" | "anyOf" | "allOf" => {
                if let Some(subs) = v.as_array() {
                    for (i, sub) in subs.iter().enumerate() {
                        out.extend(unknown_keywords(sub, &format!("{path}/{k}/{i}")));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Two JSON values alike, numbers by value (`sameJson`).
pub fn same_json(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => as_f64(x) == as_f64(y),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_json(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same_json(v, w))),
        _ => a == b,
    }
}

fn is_type(t: &str, v: &Value) -> bool {
    match t {
        "null" => v.is_null(),
        "boolean" => v.is_boolean(),
        "number" => v.as_f64().is_some_and(f64::is_finite),
        "integer" => v.as_f64().is_some_and(|x| x.is_finite() && x == x.trunc()),
        "string" => v.is_string(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        _ => false,
    }
}

/// One breach: where, and what.
#[derive(Debug, Clone, PartialEq)]
pub struct Breach {
    pub path: String,
    pub message: String,
}

/// A reference a value holds where its schema names the collection it points
/// into (`x-target`, "domain.collection"): where it is, what it names, and what
/// goes where the object it names is not there — `cut`, the path of the
/// reference itself where it may go alone (an optional member, an item of a
/// list), else of the object it is required by, up to the whole value ("").
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub path: String,
    pub target: String,
    pub id: String,
    pub cut: String,
}

/// How the walk came to a value: by a member (`required` in its object) or by an item.
#[derive(Clone)]
enum Step {
    Member(String, bool),
    Item(usize),
}

fn path_of(steps: &[Step]) -> String {
    steps
        .iter()
        .map(|s| match s {
            Step::Member(k, _) => format!("/{k}"),
            Step::Item(i) => format!("/{i}"),
        })
        .collect()
}

/// Where a dangling reference at `steps` is cut: at itself where it may go alone, else at the object requiring it.
fn cut_of(steps: &[Step]) -> String {
    let mut n = steps.len();
    while n > 0 {
        match &steps[n - 1] {
            Step::Member(_, true) => n -= 1,
            _ => break,
        }
    }
    path_of(&steps[..n])
}

/// A set of schemas that refer to each other by `$id` ("core#/$defs/Track").
/// Every pattern is compiled when its schema is added, so a set is read-only
/// once made and may be shared between threads.
pub struct SchemaSet {
    by_id: HashMap<String, Value>,
    patterns: HashMap<String, Regex>,
}

/// Every pattern a schema states: `pattern` values and `patternProperties` names.
fn patterns_of(schema: &Value, out: &mut Vec<String>) {
    match schema {
        Value::Object(o) => {
            for (k, v) in o {
                if k == "pattern"
                    && let Some(p) = v.as_str()
                {
                    out.push(p.to_string());
                }
                if k == "patternProperties"
                    && let Some(props) = v.as_object()
                {
                    out.extend(props.keys().cloned());
                }
                patterns_of(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| patterns_of(v, out)),
        _ => {}
    }
}

impl Default for SchemaSet {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaSet {
    pub fn new() -> Self {
        SchemaSet {
            by_id: HashMap::new(),
            patterns: HashMap::new(),
        }
    }

    /// Adds a schema; refused without its `$id` or with a keyword this check does not know.
    pub fn add(&mut self, schema: Value) -> Result<(), String> {
        let Some(id) = schema.get("$id").and_then(Value::as_str).map(str::to_string) else {
            return Err("a schema needs its $id".into());
        };
        let unknown = unknown_keywords(&schema, "");
        if !unknown.is_empty() {
            return Err(format!("schema {id} uses what this check does not know: {}", unknown.join(", ")));
        }
        let mut found = Vec::new();
        patterns_of(&schema, &mut found);
        for p in found {
            if let std::collections::hash_map::Entry::Vacant(slot) = self.patterns.entry(p) {
                let re = Regex::new(slot.key()).map_err(|e| format!("schema {id}: pattern {}: {e}", slot.key()))?;
                slot.insert(re);
            }
        }
        self.by_id.insert(id, schema);
        Ok(())
    }

    /// A schema by its `$id`.
    pub fn get(&self, id: &str) -> Option<&Value> {
        self.by_id.get(id)
    }

    /// The breaches of `value` against `reference` — a schema's `$id` or a part
    /// of one ("core#/$defs/TrackRange") —, at most `limit`.
    pub fn check(&self, reference: &str, value: &Value, limit: usize) -> Result<Vec<Breach>, String> {
        let (root, node) = if reference.contains('#') {
            self.resolve(None, reference)?
        } else {
            let s = self.by_id.get(reference).ok_or_else(|| format!("no schema {reference}"))?;
            (s, s)
        };
        let mut errors = Vec::new();
        self.walk(root, node, value, "", &mut errors, limit)?;
        Ok(errors)
    }

    /// The references `value` holds against `reference` (see `Reference`), in
    /// the order the walk meets them. A value the schema does not take holds
    /// those of the parts it takes; of alternatives (`oneOf`, `anyOf`), those of
    /// the ones it matches.
    pub fn references(&self, reference: &str, value: &Value) -> Result<Vec<Reference>, String> {
        let (root, node) = if reference.contains('#') {
            self.resolve(None, reference)?
        } else {
            let s = self.by_id.get(reference).ok_or_else(|| format!("no schema {reference}"))?;
            (s, s)
        };
        let mut out = Vec::new();
        self.collect(root, node, value, &mut Vec::new(), &mut out)?;
        Ok(out)
    }

    fn collect(&self, root: &Value, schema: &Value, value: &Value, steps: &mut Vec<Step>, out: &mut Vec<Reference>) -> Result<(), String> {
        if let (Some(target), Some(id)) = (schema.get("x-target").and_then(Value::as_str), value.as_str())
            && target.split_once('.').is_some_and(|(d, c)| !d.is_empty() && !c.is_empty() && !c.contains(' '))
        {
            out.push(Reference {
                path: path_of(steps),
                target: target.into(),
                id: id.into(),
                cut: cut_of(steps),
            });
        }
        if let Some(r) = schema.get("$ref").and_then(Value::as_str).filter(|r| !r.is_empty()) {
            let (r_root, node) = self.resolve(Some(root), r)?;
            self.collect(r_root, node, value, steps, out)?;
        }
        if let (Some(items), Some(item_schema)) = (value.as_array(), schema.get("items").filter(|s| s.is_object())) {
            for (i, v) in items.iter().enumerate() {
                steps.push(Step::Item(i));
                self.collect(root, item_schema, v, steps, out)?;
                steps.pop();
            }
        }
        if let Some(obj) = value.as_object() {
            let empty = serde_json::Map::new();
            let props = schema.get("properties").and_then(Value::as_object).unwrap_or(&empty);
            let patterns = schema.get("patternProperties").and_then(Value::as_object).unwrap_or(&empty);
            let additional = schema.as_object().and_then(|o| o.get("additionalProperties")).filter(|a| a.is_object());
            let required = |k: &str| {
                schema
                    .get("required")
                    .and_then(Value::as_array)
                    .is_some_and(|r| r.iter().any(|x| x.as_str() == Some(k)))
            };
            for (k, v) in obj {
                let mut subs: Vec<&Value> = props.get(k).into_iter().collect();
                subs.extend(patterns.iter().filter(|(p, _)| self.matches(p, k)).map(|(_, sub)| sub));
                if subs.is_empty() {
                    subs.extend(additional);
                }
                steps.push(Step::Member(k.clone(), required(k)));
                for sub in subs {
                    self.collect(root, sub, v, steps, out)?;
                }
                steps.pop();
            }
        }
        if let Some(Value::Array(all)) = schema.get("allOf") {
            for sub in all {
                self.collect(root, sub, value, steps, out)?;
            }
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(Value::Array(alternatives)) = schema.get(key) {
                for sub in alternatives {
                    if self.passes(root, sub, value, &path_of(steps))? {
                        self.collect(root, sub, value, steps, out)?;
                        if key == "oneOf" {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve<'a>(&'a self, root: Option<&'a Value>, reference: &str) -> Result<(&'a Value, &'a Value), String> {
        let hash = reference.find('#').unwrap_or(0);
        let target = if hash > 0 {
            self.by_id.get(&reference[..hash]).ok_or_else(|| format!("no schema {}", &reference[..hash]))?
        } else {
            root.ok_or_else(|| format!("no schema {}", &reference[..hash]))?
        };
        let mut node = Some(target);
        for part in reference[hash + 1..].split('/').filter(|p| !p.is_empty()) {
            node = node.and_then(|n| match n {
                Value::Object(o) => o.get(part),
                Value::Array(a) => part.parse::<usize>().ok().and_then(|i| a.get(i)),
                _ => None,
            });
        }
        match node {
            Some(n) if n.is_object() => Ok((target, n)),
            _ => Err(format!("cannot resolve {reference}")),
        }
    }

    fn matches(&self, pattern: &str, text: &str) -> bool {
        self.patterns.get(pattern).is_some_and(|re| re.is_match(text))
    }

    fn passes(&self, root: &Value, schema: &Value, value: &Value, path: &str) -> Result<bool, String> {
        let mut errors = Vec::new();
        self.walk(root, schema, value, path, &mut errors, 1)?;
        Ok(errors.is_empty())
    }

    fn walk(&self, root: &Value, schema: &Value, value: &Value, path: &str, errors: &mut Vec<Breach>, limit: usize) -> Result<(), String> {
        if errors.len() >= limit {
            return Ok(());
        }
        let fail = |errors: &mut Vec<Breach>, message: String| {
            if errors.len() < limit {
                errors.push(Breach {
                    path: if path.is_empty() { "/".into() } else { path.into() },
                    message,
                });
            }
        };

        if let Some(r) = schema.get("$ref").and_then(Value::as_str).filter(|r| !r.is_empty()) {
            let (r_root, node) = self.resolve(Some(root), r)?;
            self.walk(r_root, node, value, path, errors, limit)?;
        }
        if let Some(t) = schema.get("type") {
            let types: Vec<&str> = match t {
                Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
                Value::String(s) => vec![s.as_str()],
                _ => vec![],
            };
            if !types.iter().any(|t| is_type(t, value)) {
                fail(errors, format!("is not {}", types.join(" or ")));
                return Ok(());
            }
        }
        if let Some(Value::Array(options)) = schema.get("enum")
            && !options.iter().any(|e| same_json(e, value))
        {
            fail(errors, format!("is not one of {}", stringify(&Value::Array(options.clone()))));
        }
        if let Some(c) = schema.as_object().and_then(|o| o.get("const"))
            && !same_json(c, value)
        {
            fail(errors, format!("is not {}", stringify(c)));
        }

        if let Some(x) = value.as_f64() {
            if let Some(min) = schema.get("minimum").and_then(Value::as_f64)
                && x < min
            {
                fail(errors, format!("is less than {}", es_number(min)));
            }
            if let Some(max) = schema.get("maximum").and_then(Value::as_f64)
                && x > max
            {
                fail(errors, format!("is more than {}", es_number(max)));
            }
        }
        if let Some(s) = value.as_str() {
            if let Some(min) = schema.get("minLength").and_then(Value::as_f64)
                && (s.chars().count() as f64) < min
            {
                fail(errors, format!("is shorter than {}", es_number(min)));
            }
            if let Some(p) = schema.get("pattern").and_then(Value::as_str)
                && !self.matches(p, s)
            {
                fail(errors, format!("does not match {p}"));
            }
        }
        if let Some(items) = value.as_array() {
            if let Some(min) = schema.get("minItems").and_then(Value::as_f64)
                && (items.len() as f64) < min
            {
                fail(errors, format!("has fewer than {} items", es_number(min)));
            }
            if let Some(max) = schema.get("maxItems").and_then(Value::as_f64)
                && (items.len() as f64) > max
            {
                fail(errors, format!("has more than {} items", es_number(max)));
            }
            if let Some(item_schema) = schema.get("items").filter(|s| !s.is_null() && *s != &Value::Bool(false)) {
                for (i, v) in items.iter().enumerate() {
                    self.walk(root, item_schema, v, &format!("{path}/{i}"), errors, limit)?;
                }
            }
        }
        if let Some(obj) = value.as_object() {
            if let Some(Value::Array(required)) = schema.get("required") {
                for k in required.iter().filter_map(Value::as_str) {
                    if !obj.contains_key(k) {
                        fail(errors, format!("lacks {k}"));
                    }
                }
            }
            if let Some(min) = schema.get("minProperties").and_then(Value::as_f64)
                && (obj.len() as f64) < min
            {
                fail(errors, format!("has fewer than {} members", es_number(min)));
            }
            let empty = serde_json::Map::new();
            let props = schema.get("properties").and_then(Value::as_object).unwrap_or(&empty);
            let patterns = schema.get("patternProperties").and_then(Value::as_object).unwrap_or(&empty);
            let additional = schema.as_object().and_then(|o| o.get("additionalProperties"));
            for (k, v) in obj {
                let at = format!("{path}/{k}");
                let mut known = false;
                if let Some(sub) = props.get(k) {
                    known = true;
                    self.walk(root, sub, v, &at, errors, limit)?;
                }
                for (p, sub) in patterns {
                    if self.matches(p, k) {
                        known = true;
                        self.walk(root, sub, v, &at, errors, limit)?;
                    }
                }
                if !known {
                    match additional {
                        Some(Value::Bool(false)) => fail(errors, format!("has {k}, which it may not")),
                        Some(sub @ Value::Object(_)) => self.walk(root, sub, v, &at, errors, limit)?,
                        _ => {}
                    }
                }
            }
        }

        if let Some(Value::Array(all)) = schema.get("allOf") {
            for sub in all {
                self.walk(root, sub, value, path, errors, limit)?;
            }
        }
        if let Some(Value::Array(any)) = schema.get("anyOf") {
            let mut ok = false;
            for sub in any {
                if self.passes(root, sub, value, path)? {
                    ok = true;
                    break;
                }
            }
            if !ok {
                fail(errors, "matches none of anyOf".into());
            }
        }
        if let Some(Value::Array(one)) = schema.get("oneOf") {
            let mut n = 0;
            for sub in one {
                if self.passes(root, sub, value, path)? {
                    n += 1;
                }
            }
            if n != 1 {
                // Where exactly one alternative has the type, its own breaches say more.
                let typed: Vec<&Value> = one
                    .iter()
                    .filter(|sub| match sub.get("type") {
                        None | Some(Value::Null) => true,
                        Some(Value::String(t)) => is_type(t, value),
                        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).any(|t| is_type(t, value)),
                        Some(_) => false,
                    })
                    .collect();
                if n == 0 && typed.len() == 1 {
                    self.walk(root, typed[0], value, path, errors, limit)?;
                } else {
                    fail(
                        errors,
                        if n == 0 {
                            "matches none of oneOf".into()
                        } else {
                            "matches more than one of oneOf".into()
                        },
                    );
                }
            }
        }
        Ok(())
    }
}
