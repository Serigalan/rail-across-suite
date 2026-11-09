// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The XML section of an E57 file as a tree (`e57Xml.js`): elements,
//! attributes, text and CDATA; declarations, comments and doctypes passed
//! over. A node's name as written, an extension's prefix included.

use serde_json::{Map, Value, json};

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub text: String,
}

impl Node {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
    pub fn to_json(&self) -> Value {
        let mut attrs = Map::new();
        for (k, v) in &self.attrs {
            attrs.insert(k.clone(), json!(v));
        }
        json!({ "name": self.name, "attrs": attrs, "children": self.children.iter().map(Node::to_json).collect::<Vec<_>>(), "text": self.text })
    }
}

/// The child of an optional node named `name`.
pub fn child<'a>(node: Option<&'a Node>, name: &str) -> Option<&'a Node> {
    node?.child(name)
}

fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn code_point(n: f64) -> Result<char, String> {
    if !(0.0..=1_114_111.0).contains(&n) || n.fract() != 0.0 {
        return Err(format!("Invalid code point {}", rail_across_js::js_string(Some(&rail_across_js::number(n)))));
    }
    Ok(char::from_u32(n as u32).unwrap_or('\u{fffd}'))
}

/// Entities and character references decoded, as `/&(#x[0-9a-f]+|#\d+|\w+);/gi`.
fn decode(s: &str) -> Result<String, String> {
    if !s.contains('&') {
        return Ok(s.to_string());
    }
    let b = s.as_bytes();
    let mut out = String::new();
    let (mut i, mut copied) = (0, 0);
    while let Some(off) = s[i..].find('&') {
        let amp = i + off;
        let run = |from: usize, ok: &dyn Fn(u8) -> bool| {
            let mut j = from;
            while j < b.len() && ok(b[j]) {
                j += 1;
            }
            j
        };
        let mut hit: Option<(usize, String)> = None;
        if b.get(amp + 1) == Some(&b'#') && matches!(b.get(amp + 2), Some(b'x') | Some(b'X')) {
            let end = run(amp + 3, &|c| c.is_ascii_hexdigit());
            if end > amp + 3 && b.get(end) == Some(&b';') {
                let n = s[amp + 3..end].chars().fold(0.0, |a, c| a * 16.0 + f64::from(c.to_digit(16).unwrap()));
                hit = Some((end + 1, code_point(n)?.to_string()));
            }
        }
        if hit.is_none() && b.get(amp + 1) == Some(&b'#') {
            let end = run(amp + 2, &|c| c.is_ascii_digit());
            if end > amp + 2 && b.get(end) == Some(&b';') {
                let n = s[amp + 2..end].chars().fold(0.0, |a, c| a * 10.0 + f64::from(c.to_digit(10).unwrap()));
                hit = Some((end + 1, code_point(n)?.to_string()));
            }
        }
        if hit.is_none() {
            let end = run(amp + 1, &|c| c.is_ascii_alphanumeric() || c == b'_');
            if end > amp + 1 && b.get(end) == Some(&b';') {
                let named = match &s[amp + 1..end] {
                    "lt" => "<",
                    "gt" => ">",
                    "amp" => "&",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => &s[amp..=end],
                };
                hit = Some((end + 1, named.to_string()));
            }
        }
        match hit {
            Some((next, text)) => {
                out.push_str(&s[copied..amp]);
                out.push_str(&text);
                i = next;
                copied = next;
            }
            None => i = amp + 1,
        }
    }
    out.push_str(&s[copied..]);
    Ok(out)
}

fn attributes(s: &str) -> Result<Vec<(String, String)>, String> {
    let name_char = |c: char| !(is_space(c) || c == '=' || c == '/' || c == '>');
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let name_len: usize = rest.chars().take_while(|c| name_char(*c)).map(char::len_utf8).sum();
        let step = rest.chars().next().map(char::len_utf8).unwrap_or(1);
        if name_len == 0 {
            i += step;
            continue;
        }
        let a = rest[name_len..].trim_start_matches(is_space);
        let matched = a.strip_prefix('=').and_then(|a| {
            let a = a.trim_start_matches(is_space);
            let q = a.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let close = a[1..].find(q)?;
            Some((&a[1..1 + close], s.len() - (a.len() - close - 2)))
        });
        match matched {
            Some((value, consumed)) => {
                let name = rest[..name_len].to_string();
                let value = decode(value)?;
                match out.iter_mut().find(|(k, _)| *k == name) {
                    Some(slot) => slot.1 = value,
                    None => out.push((name, value)),
                }
                i = consumed;
            }
            None => i += step,
        }
    }
    Ok(out)
}

/// The root element of `xml` (`parseXml`).
pub fn parse_xml(xml: &str) -> Result<Node, String> {
    let mut stack: Vec<Node> = vec![Node {
        name: "#document".into(),
        ..Node::default()
    }];
    let mut i = 0;
    let fail = |what: String, i: usize| -> String { format!("E57 XML: {what} at {}", xml[..i].encode_utf16().count()) };
    let find = |from: usize, pat: &str| xml[from..].find(pat).map(|o| from + o);
    let bytes = xml.as_bytes();
    while i < xml.len() {
        let Some(lt) = find(i, "<") else {
            let t = decode(&xml[i..])?;
            stack.last_mut().unwrap().text.push_str(&t);
            break;
        };
        if lt > i {
            let t = decode(&xml[i..lt])?;
            stack.last_mut().unwrap().text.push_str(&t);
        }
        i = lt;
        let rest = &xml[i..];
        if rest.starts_with("<![CDATA[") {
            let end = find(i, "]]>").ok_or_else(|| fail("open CDATA".into(), i))?;
            let t = xml[i + 9..end].to_string();
            stack.last_mut().unwrap().text.push_str(&t);
            i = end + 3;
        } else if rest.starts_with("<!--") {
            let end = find(i, "-->").ok_or_else(|| fail("open comment".into(), i))?;
            i = end + 3;
        } else if rest.starts_with("<?") {
            let end = find(i, "?>").ok_or_else(|| fail("open declaration".into(), i))?;
            i = end + 2;
        } else if rest.starts_with("<!") {
            let end = find(i, ">").ok_or_else(|| fail("open doctype".into(), i))?;
            i = end + 1;
        } else if bytes.get(i + 1) == Some(&b'/') {
            let end = find(i, ">").ok_or_else(|| fail("open end tag".into(), i))?;
            let name = xml[i + 2..end].trim_matches(is_space);
            if stack.len() < 2 || stack.last().unwrap().name != name {
                return Err(fail(format!("unexpected </{name}>"), i));
            }
            let node = stack.pop().unwrap();
            stack.last_mut().unwrap().children.push(node);
            i = end + 1;
        } else {
            let mut j = i + 1;
            let mut quote: Option<u8> = None;
            while j < bytes.len() {
                let c = bytes[j];
                match quote {
                    Some(q) => {
                        if c == q {
                            quote = None;
                        }
                    }
                    None if c == b'"' || c == b'\'' => quote = Some(c),
                    None if c == b'>' => break,
                    None => {}
                }
                j += 1;
            }
            if j >= bytes.len() {
                return Err(fail("open tag".into(), i));
            }
            let self_closing = bytes[j - 1] == b'/';
            let body = &xml[i + 1..if self_closing { j - 1 } else { j }];
            let name_len: usize = body.chars().take_while(|c| !(is_space(*c) || *c == '/' || *c == '>')).map(char::len_utf8).sum();
            if name_len == 0 {
                return Err(fail("nameless tag".into(), i));
            }
            let name = body[..name_len].to_string();
            let attrs = attributes(&body[name_len..])?;
            let node = Node {
                name,
                attrs,
                children: vec![],
                text: String::new(),
            };
            if self_closing {
                stack.last_mut().unwrap().children.push(node);
            } else {
                stack.push(node);
            }
            i = j + 1;
        }
    }
    if stack.len() != 1 {
        return Err(fail(format!("unclosed <{}>", stack.last().unwrap().name), i.min(xml.len())));
    }
    stack
        .pop()
        .unwrap()
        .children
        .into_iter()
        .next()
        .ok_or_else(|| fail("no root element".into(), xml.len()))
}
