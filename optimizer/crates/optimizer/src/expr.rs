// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The little expression language the rule catalogues are written in
//! (`expression_language` in packages/core/constraints/*.json) — the Rust half
//! of the app's packages/core/src/rules/ruleExpr.js, and meant to read like it.
//!
//! It is parsed, not translated: `^` is exponentiation here, `and`/`or`/`not`
//! short-circuit on values the catalogue may leave undefined, and a rule that
//! silently computes the wrong limit is the one failure a rule checker must not
//! have — worse here than in the app, since the optimizer does not report the
//! limit, it builds to it.
//!
//! Names are interned once, when the catalogue is read ([`Symbols`]); a scope
//! is a slot per name ([`Scope`]), so the thresholds the optimizer asks for in
//! its innermost loop cost no hashing. An unknown name is an error: the
//! catalogue names its inputs itself, so a name the scope does not answer is a
//! mistake in the catalogue or in the code that fills the scope — never
//! something to paper over.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// An interned name of a scope.
pub type Sym = usize;

/// The names a catalogue's expressions use, each with its slot.
#[derive(Debug, Default, Clone)]
pub struct Symbols {
    by_name: HashMap<String, Sym>,
    names: Vec<String>,
}

impl Symbols {
    /// The slot of `name`, made if it has none yet.
    pub fn intern(&mut self, name: &str) -> Sym {
        if let Some(&sym) = self.by_name.get(name) {
            return sym;
        }
        let sym = self.names.len();
        self.names.push(name.to_string());
        self.by_name.insert(name.to_string(), sym);
        sym
    }

    /// The slot of `name`, if any expression uses it.
    pub fn get(&self, name: &str) -> Option<Sym> {
        self.by_name.get(name).copied()
    }

    pub fn name(&self, sym: Sym) -> &str {
        &self.names[sym]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// A value of the language: a number, a truth value or a string — as in
/// JavaScript, `==` holds only between two of the same kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Str(Arc<str>),
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Arc::from(s))
    }

    /// JavaScript's truthiness: 0, NaN, false and '' are false.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Num(x) => *x != 0.0 && !x.is_nan(),
            Value::Bool(b) => *b,
            Value::Str(s) => !s.is_empty(),
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(x) => Some(*x),
            _ => None,
        }
    }

    /// A value from JSON, as the shared vectors and the requests carry it.
    pub fn from_json(value: &serde_json::Value) -> Option<Value> {
        match value {
            serde_json::Value::Number(n) => n.as_f64().map(Value::Num),
            serde_json::Value::Bool(b) => Some(Value::Bool(*b)),
            serde_json::Value::String(s) => Some(Value::str(s)),
            _ => None,
        }
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Value {
        Value::Num(x)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::str(s)
    }
}

/// What can go wrong reading or applying a catalogue.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// An expression the language cannot read or answer.
    Expr(String),
    /// A table asked for a key it does not cover, with the table's own
    /// `out_of_range` severity — the catalogue says what a missing value
    /// means, and it is never "pass".
    OutOfRange(String),
    /// A catalogue that is not what this code can apply.
    Catalogue(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Expr(msg) | Error::Catalogue(msg) => f.write_str(msg),
            Error::OutOfRange(severity) => write!(f, "out of range → {severity}"),
        }
    }
}

impl std::error::Error for Error {}

fn expr_error(msg: impl Into<String>) -> Error {
    Error::Expr(msg.into())
}

/// The values of one evaluation, a slot per name.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    values: Vec<Option<Value>>,
}

impl Scope {
    pub fn new(len: usize) -> Scope {
        Scope { values: vec![None; len] }
    }

    pub fn get(&self, sym: Sym) -> Option<&Value> {
        self.values.get(sym).and_then(Option::as_ref)
    }

    pub fn set(&mut self, sym: Sym, value: Value) {
        if sym >= self.values.len() {
            self.values.resize(sym + 1, None);
        }
        self.values[sym] = Some(value);
    }
}

/// The functions an expression may call. `if` is not one of them — the
/// language evaluates it itself, so that only the chosen branch is computed.
pub trait Functions {
    fn call(&self, name: &str, args: &[Value]) -> Result<Value, Error>;
}

/// No functions at all — what a table's own formula is evaluated with.
pub struct NoFunctions;

impl Functions for NoFunctions {
    fn call(&self, name: &str, _: &[Value]) -> Result<Value, Error> {
        Err(expr_error(format!("unknown function {name}")))
    }
}

/// min, max, abs and round_to — the functions every catalogue has.
pub struct Basic;

fn numbers(name: &str, args: &[Value]) -> Result<Vec<f64>, Error> {
    args.iter()
        .map(|a| a.as_num().ok_or_else(|| expr_error(format!("argument of {name} is not a number: {a:?}"))))
        .collect()
}

impl Functions for Basic {
    fn call(&self, name: &str, args: &[Value]) -> Result<Value, Error> {
        let xs = || numbers(name, args);
        let two = |xs: Vec<f64>| -> Result<(f64, f64), Error> {
            match xs[..] {
                [a, b] => Ok((a, b)),
                _ => Err(expr_error(format!("{name} takes two arguments"))),
            }
        };
        match name {
            "min" => Ok(Value::Num(xs()?.into_iter().fold(f64::INFINITY, f64::min))),
            "max" => Ok(Value::Num(xs()?.into_iter().fold(f64::NEG_INFINITY, f64::max))),
            "abs" => match xs()?[..] {
                [x] => Ok(Value::Num(x.abs())),
                _ => Err(expr_error("abs takes one argument")),
            },
            // Half a step rounds up, as the catalogue states (and as the app's
            // Math.round does).
            "round_to" => {
                let (x, step) = two(xs()?)?;
                Ok(Value::Num((x / step + 0.5).floor() * step))
            }
            _ => Err(expr_error(format!("unknown function {name}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
}

impl Op {
    fn text(self) -> &'static str {
        match self {
            Op::Or => "or",
            Op::And => "and",
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Rem => "%",
            Op::Pow => "^",
        }
    }

    /// The comparison seen from the other side: `a < b` is `b > a`.
    pub fn flip(self) -> Op {
        match self {
            Op::Lt => Op::Gt,
            Op::Le => Op::Ge,
            Op::Gt => Op::Lt,
            Op::Ge => Op::Le,
            other => other,
        }
    }

    /// Is this a lower bound when the input stands on its left?
    pub fn is_lower(self) -> bool {
        matches!(self, Op::Gt | Op::Ge)
    }
}

#[derive(Debug, Clone)]
enum Node {
    Const(Value),
    Name(Sym, Box<str>),
    Not(Box<Node>),
    Negate(Box<Node>),
    Call(Box<str>, Vec<Node>),
    Binary(Op, Box<Node>, Box<Node>),
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Str(String),
    Name(String),
    Keyword(&'static str),
    Punct(char),
    Op(Op),
    End,
}

impl Token {
    fn describe(&self) -> String {
        match self {
            Token::Number(_) => "number".into(),
            Token::Str(_) => "string".into(),
            Token::Name(_) => "name".into(),
            Token::Keyword(k) => (*k).into(),
            Token::Punct(c) => c.to_string(),
            Token::Op(op) => op.text().into(),
            Token::End => "end".into(),
        }
    }
}

const KEYWORDS: [&str; 5] = ["and", "or", "not", "true", "false"];
// Longest first: `<=` must be read before `<`.
const OPERATORS: [(&str, Op); 12] = [
    ("==", Op::Eq),
    ("!=", Op::Ne),
    ("<=", Op::Le),
    (">=", Op::Ge),
    ("+", Op::Add),
    ("-", Op::Sub),
    ("*", Op::Mul),
    ("/", Op::Div),
    ("^", Op::Pow),
    ("%", Op::Rem),
    ("<", Op::Lt),
    (">", Op::Gt),
];

fn tokenize(src: &str) -> Result<Vec<Token>, Error> {
    let bytes = src.as_bytes();
    let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = src[i..].chars().next().unwrap();
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if digit(i) || (c == '.' && digit(i + 1)) {
            // [0-9]*\.?[0-9]+ — a dot belongs to the number only with a digit after it.
            let mut end = i;
            while digit(end) {
                end += 1;
            }
            if bytes.get(end) == Some(&b'.') && digit(end + 1) {
                end += 1;
                while digit(end) {
                    end += 1;
                }
            }
            let value = src[i..end].parse::<f64>().map_err(|_| expr_error(format!("bad number in {src}")))?;
            tokens.push(Token::Number(value));
            i = end;
            continue;
        }
        if c == '\'' {
            let Some(len) = src[i + 1..].find('\'') else {
                return Err(expr_error(format!("unterminated string in {src}")));
            };
            tokens.push(Token::Str(src[i + 1..i + 1 + len].to_string()));
            i += len + 2;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            // A name may carry dots — `prev.design_speed` is one name, not a
            // field access: the scope is flat and answers exactly the names
            // the rule lists.
            let mut end = i + 1;
            while bytes.get(end).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'.') {
                end += 1;
            }
            let word = &src[i..end];
            tokens.push(match KEYWORDS.iter().find(|k| **k == word) {
                Some(k) => Token::Keyword(k),
                None => Token::Name(word.to_string()),
            });
            i = end;
            continue;
        }
        if matches!(c, '(' | ')' | ',') {
            tokens.push(Token::Punct(c));
            i += 1;
            continue;
        }
        let Some(&(text, op)) = OPERATORS.iter().find(|(text, _)| src[i..].starts_with(text)) else {
            return Err(expr_error(format!("unexpected character {c:?} in {src}")));
        };
        tokens.push(Token::Op(op));
        i += text.len();
    }
    tokens.push(Token::End);
    Ok(tokens)
}

// Binary levels from loosest to tightest. `^` is right-associative — a^b^c is
// a^(b^c) — and binds tighter than a unary minus on its left: -2^2 is -4.
const BINARY_LEVELS: [&[Op]; 5] = [
    &[Op::Or],
    &[Op::And],
    &[Op::Eq, Op::Ne, Op::Lt, Op::Le, Op::Gt, Op::Ge],
    &[Op::Add, Op::Sub],
    &[Op::Mul, Op::Div, Op::Rem],
];

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    src: &'a str,
    symbols: &'a mut Symbols,
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }

    fn take(&mut self, want: &Token) -> Result<(), Error> {
        if self.peek() != want {
            return Err(expr_error(format!(
                "expected {}, found {} in {}",
                want.describe(),
                self.peek().describe(),
                self.src
            )));
        }
        self.at += 1;
        Ok(())
    }

    fn binary(&mut self, level: usize) -> Result<Node, Error> {
        if level >= BINARY_LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        loop {
            let op = match self.peek() {
                Token::Op(op) if BINARY_LEVELS[level].contains(op) => *op,
                Token::Keyword("and") if level == 1 => Op::And,
                Token::Keyword("or") if level == 0 => Op::Or,
                _ => break,
            };
            self.at += 1;
            let right = self.binary(level + 1)?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Node, Error> {
        match self.peek() {
            Token::Keyword("not") => {
                self.at += 1;
                Ok(Node::Not(Box::new(self.unary()?)))
            }
            Token::Op(Op::Sub) => {
                self.at += 1;
                Ok(Node::Negate(Box::new(self.unary()?)))
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Node, Error> {
        let base = self.primary()?;
        if self.peek() != &Token::Op(Op::Pow) {
            return Ok(base);
        }
        self.at += 1;
        Ok(Node::Binary(Op::Pow, Box::new(base), Box::new(self.unary()?)))
    }

    fn primary(&mut self) -> Result<Node, Error> {
        let token = self.peek().clone();
        self.at += 1;
        match token {
            Token::Number(x) => Ok(Node::Const(Value::Num(x))),
            Token::Str(s) => Ok(Node::Const(Value::str(&s))),
            Token::Keyword("true") => Ok(Node::Const(Value::Bool(true))),
            Token::Keyword("false") => Ok(Node::Const(Value::Bool(false))),
            Token::Punct('(') => {
                let inner = self.binary(0)?;
                self.take(&Token::Punct(')'))?;
                Ok(inner)
            }
            Token::Name(name) => {
                if self.peek() != &Token::Punct('(') {
                    let sym = self.symbols.intern(&name);
                    return Ok(Node::Name(sym, name.into()));
                }
                self.at += 1;
                let mut args = Vec::new();
                if self.peek() != &Token::Punct(')') {
                    args.push(self.binary(0)?);
                    while self.peek() == &Token::Punct(',') {
                        self.at += 1;
                        args.push(self.binary(0)?);
                    }
                }
                self.take(&Token::Punct(')'))?;
                if &*name == "if" && args.len() != 3 {
                    return Err(expr_error("if takes three arguments"));
                }
                Ok(Node::Call(name.into(), args))
            }
            other => Err(expr_error(format!("unexpected {} in {}", other.describe(), self.src))),
        }
    }
}

fn number(value: Value, what: impl FnOnce() -> String) -> Result<f64, Error> {
    match value {
        Value::Num(x) if x.is_finite() => Ok(x),
        other => Err(expr_error(format!("{} is not a number: {other:?}", what()))),
    }
}

fn run(node: &Node, scope: &Scope, fns: &dyn Functions) -> Result<Value, Error> {
    match node {
        Node::Const(value) => Ok(value.clone()),
        Node::Name(sym, name) => scope.get(*sym).cloned().ok_or_else(|| expr_error(format!("unknown name {name}"))),
        Node::Not(inner) => Ok(Value::Bool(!run(inner, scope, fns)?.truthy())),
        Node::Negate(inner) => Ok(Value::Num(-number(run(inner, scope, fns)?, || "negated value".into())?)),
        Node::Call(name, args) => {
            if &**name == "if" {
                return if run(&args[0], scope, fns)?.truthy() {
                    run(&args[1], scope, fns)
                } else {
                    run(&args[2], scope, fns)
                };
            }
            let values = args.iter().map(|a| run(a, scope, fns)).collect::<Result<Vec<_>, _>>()?;
            fns.call(name, &values)
        }
        Node::Binary(op, left, right) => {
            // Short-circuit, so `jump and r_w >= reg` may be written where r_w
            // only exists at a jump.
            match op {
                Op::And => {
                    let a = run(left, scope, fns)?;
                    return if a.truthy() { run(right, scope, fns) } else { Ok(a) };
                }
                Op::Or => {
                    let a = run(left, scope, fns)?;
                    return if a.truthy() { Ok(a) } else { run(right, scope, fns) };
                }
                _ => {}
            }
            let a = run(left, scope, fns)?;
            let b = run(right, scope, fns)?;
            match op {
                Op::Eq => return Ok(Value::Bool(a == b)),
                Op::Ne => return Ok(Value::Bool(a != b)),
                _ => {}
            }
            let x = number(a, || format!("left of {}", op.text()))?;
            let y = number(b, || format!("right of {}", op.text()))?;
            Ok(match op {
                Op::Add => Value::Num(x + y),
                Op::Sub => Value::Num(x - y),
                Op::Mul => Value::Num(x * y),
                Op::Div => Value::Num(x / y),
                // The remainder keeps the sign of the dividend, as in JavaScript.
                Op::Rem => Value::Num(x % y),
                Op::Pow => Value::Num(x.powf(y)),
                Op::Lt => Value::Bool(x < y),
                Op::Le => Value::Bool(x <= y),
                Op::Gt => Value::Bool(x > y),
                Op::Ge => Value::Bool(x >= y),
                Op::And | Op::Or | Op::Eq | Op::Ne => unreachable!(),
            })
        }
    }
}

/// One parsed expression.
#[derive(Debug, Clone)]
pub struct Expr {
    pub src: String,
    node: Node,
}

impl Expr {
    /// `src` parsed, its names interned in `symbols`.
    pub fn parse(src: &str, symbols: &mut Symbols) -> Result<Expr, Error> {
        let tokens = tokenize(src)?;
        let mut parser = Parser { tokens, at: 0, src, symbols };
        let node = parser.binary(0)?;
        parser.take(&Token::End)?;
        Ok(Expr { src: src.to_string(), node })
    }

    /// The value over `scope` with `fns`.
    pub fn eval(&self, scope: &Scope, fns: &dyn Functions) -> Result<Value, Error> {
        run(&self.node, scope, fns)
    }

    /// The expression read as a single comparison `a <op> b` between two
    /// names — what a line of a rule's `evaluation` looks like when it
    /// compares one input against one threshold, the shape the optimizer
    /// turns into a bound. None where it is anything else.
    pub fn comparison(&self) -> Option<(Sym, Op, Sym)> {
        match &self.node {
            Node::Binary(op @ (Op::Lt | Op::Le | Op::Gt | Op::Ge), left, right) => match (&**left, &**right) {
                (Node::Name(a, _), Node::Name(b, _)) => Some((*a, *op, *b)),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Evaluate `src` once over a scope of named values — for tests and the
/// shared vectors; the catalogue parses its expressions once instead.
pub fn eval_expr(src: &str, scope: &[(&str, Value)], fns: &dyn Functions) -> Result<Value, Error> {
    let mut symbols = Symbols::default();
    let expr = Expr::parse(src, &mut symbols)?;
    let mut values = Scope::new(symbols.len());
    for (name, value) in scope {
        if let Some(sym) = symbols.get(name) {
            values.set(sym, value.clone());
        }
    }
    expr.eval(&values, fns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, scope: &[(&str, Value)]) -> Result<Value, Error> {
        eval_expr(src, scope, &Basic)
    }

    #[test]
    fn precedence_and_associativity() {
        assert_eq!(eval("2 + 3 * 4", &[]), Ok(Value::Num(14.0)));
        assert_eq!(eval("2 ^ 3 ^ 2", &[]), Ok(Value::Num(512.0)));
        assert_eq!(eval("-2 ^ 2", &[]), Ok(Value::Num(-4.0)));
        assert_eq!(eval("prev.design_speed * 2", &[("prev.design_speed", 40.0.into())]), Ok(Value::Num(80.0)));
    }

    #[test]
    fn unknown_things_are_errors() {
        for src in ["unbekannt + 1", "1 +", "3 & 4", "'offen", "wurzel(4)", "1.", "if(1, 2)"] {
            assert!(matches!(eval(src, &[]), Err(Error::Expr(_))), "{src}");
        }
    }

    #[test]
    fn equality_is_strict() {
        assert_eq!(eval("true == 1", &[]), Ok(Value::Bool(false)));
        assert_eq!(eval("form == 'clothoid'", &[("form", "clothoid".into())]), Ok(Value::Bool(true)));
    }

    #[test]
    fn comparison_reads_input_against_threshold() {
        let mut symbols = Symbols::default();
        let expr = Expr::parse("l_R >= reg", &mut symbols).unwrap();
        let (a, op, b) = expr.comparison().unwrap();
        assert_eq!((symbols.name(a), op, symbols.name(b)), ("l_R", Op::Ge, "reg"));
        assert!(Expr::parse("u >= 0 and u % step == 0", &mut symbols).unwrap().comparison().is_none());
    }
}
