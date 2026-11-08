// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! A small XML reader, enough for PlanPro (`xml.js`): elements, attributes,
//! text, the five named entities and character references; declarations,
//! comments, processing instructions and doctype skipped, CDATA taken as
//! text. A name keeps its prefix (`xsi:type`).

use serde_json::{Map, Value, json};

/// An element: its name, attributes in their order, child elements and the
/// text directly inside it, trimmed.
#[derive(Clone, Debug, Default)]
pub struct El {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<El>,
    pub text: String,
}

impl El {
    /// The value of an attribute, the last where it is given twice.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn to_json(&self) -> Value {
        let mut attrs = Map::new();
        for (k, v) in &self.attrs {
            attrs.insert(k.clone(), json!(v));
        }
        json!({
            "name": self.name,
            "attrs": attrs,
            "children": self.children.iter().map(El::to_json).collect::<Vec<_>>(),
            "text": self.text,
        })
    }
}

/// JavaScript's `\s`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn from_code_point(n: f64) -> Result<char, String> {
    if !(0.0..=1_114_111.0).contains(&n) || n.fract() != 0.0 {
        return Err(format!("Invalid code point {}", rail_across_js::js_string(Some(&rail_across_js::number(n)))));
    }
    // A lone surrogate has no place in a Rust string.
    Ok(char::from_u32(n as u32).unwrap_or('\u{fffd}'))
}

fn parse_int(digits: &str, radix: u32) -> f64 {
    digits
        .chars()
        .fold(0.0, |acc, c| acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0)))
}

/// The text with its entities and character references decoded.
fn decode(s: &str) -> Result<String, String> {
    if !s.contains('&') {
        return Ok(s.to_string());
    }
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut copied = 0;
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
        if b.get(amp + 1) == Some(&b'#') && b.get(amp + 2) == Some(&b'x') {
            let end = run(amp + 3, &|c| c.is_ascii_hexdigit());
            if end > amp + 3 && b.get(end) == Some(&b';') {
                hit = Some((end + 1, from_code_point(parse_int(&s[amp + 3..end], 16))?.to_string()));
            }
        }
        if hit.is_none() && b.get(amp + 1) == Some(&b'#') {
            let end = run(amp + 2, &|c| c.is_ascii_digit());
            if end > amp + 2 && b.get(end) == Some(&b';') {
                hit = Some((end + 1, from_code_point(parse_int(&s[amp + 2..end], 10))?.to_string()));
            }
        }
        if hit.is_none() {
            let end = run(amp + 1, &|c| c.is_ascii_lowercase());
            if end > amp + 1 && b.get(end) == Some(&b';') {
                let named = match &s[amp + 1..end] {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
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

/// The attributes of a tag's inside after its name, as `ATTR` of `xml.js` finds them.
fn attributes(s: &str) -> Result<Vec<(String, String)>, String> {
    let name_char = |c: char| !(is_js_space(c) || c == '=' || c == '/' || c == '>');
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
        let after = &rest[name_len..];
        let a = after.trim_start_matches(is_js_space);
        let matched = a.strip_prefix('=').and_then(|a| {
            let a = a.trim_start_matches(is_js_space);
            let q = a.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let close = a[1..].find(q)?;
            let value = &a[1..1 + close];
            let consumed = s.len() - (a.len() - close - 2);
            Some((value, consumed))
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

enum Part<'a> {
    Text(&'a str),
    Raw(&'a str),
}

fn text_of(parts: &[Part]) -> Result<String, String> {
    let mut s = String::new();
    for p in parts {
        match p {
            Part::Text(t) => s.push_str(&decode(t)?),
            Part::Raw(r) => s.push_str(r),
        }
    }
    Ok(js_trim(&s).to_string())
}

/// The document element of `text` (`parseXml`); the error names the offset
/// (in UTF-16 units, as the JavaScript counts) where the text is no XML.
pub fn parse_xml(text: &str) -> Result<El, String> {
    struct Frame<'a> {
        el: El,
        parts: Vec<Part<'a>>,
    }
    let mut stack: Vec<Frame> = vec![Frame {
        el: El {
            name: "#document".into(),
            ..El::default()
        },
        parts: vec![],
    }];
    let n = text.len();
    let mut i = 0;
    let fail = |what: String, i: usize| -> String { format!("xml: {what} at {}", text[..i].encode_utf16().count()) };
    let find = |from: usize, pat: &str| text[from..].find(pat).map(|o| from + o);
    while i < n {
        let Some(lt) = find(i, "<") else {
            stack.last_mut().unwrap().parts.push(Part::Text(&text[i..]));
            break;
        };
        if lt > i {
            stack.last_mut().unwrap().parts.push(Part::Text(&text[i..lt]));
        }
        i = lt;
        let next = text.as_bytes().get(i + 1).copied();
        if text[i..].starts_with("<!--") {
            let end = find(i + 4, "-->").ok_or_else(|| fail("comment without end".into(), i))?;
            i = end + 3;
        } else if text[i..].starts_with("<![CDATA[") {
            let end = find(i + 9, "]]>").ok_or_else(|| fail("CDATA without end".into(), i))?;
            stack.last_mut().unwrap().parts.push(Part::Raw(&text[i + 9..end]));
            i = end + 3;
        } else if next == Some(b'?') || next == Some(b'!') {
            let end = find(i, ">").ok_or_else(|| fail("declaration without end".into(), i))?;
            i = end + 1;
        } else if next == Some(b'/') {
            let end = find(i, ">").ok_or_else(|| fail("end tag without end".into(), i))?;
            let name = js_trim(&text[i + 2..end]);
            let frame = stack.pop().unwrap();
            if stack.is_empty() || frame.el.name != name {
                return Err(fail(format!("</{name}> closes <{}>", frame.el.name), i));
            }
            let mut el = frame.el;
            el.text = text_of(&frame.parts)?;
            stack.last_mut().unwrap().el.children.push(el);
            i = end + 1;
        } else {
            let end = find(i, ">").ok_or_else(|| fail("tag without end".into(), i))?;
            let self_closing = end > 0 && text.as_bytes()[end - 1] == b'/' && end - 1 > i;
            let inner = &text[i + 1..if self_closing { end - 1 } else { end }];
            let space = inner.char_indices().find(|(_, c)| is_js_space(*c)).map(|(k, _)| k);
            let name = space.map(|k| &inner[..k]).unwrap_or(inner).to_string();
            let attrs = match space {
                Some(k) => attributes(&inner[k..])?,
                None => vec![],
            };
            let el = El {
                name,
                attrs,
                children: vec![],
                text: String::new(),
            };
            if self_closing {
                stack.last_mut().unwrap().el.children.push(el);
            } else {
                stack.push(Frame { el, parts: vec![] });
            }
            i = end + 1;
        }
    }
    if stack.len() > 1 {
        return Err(format!("xml: <{}> without end", stack.last().unwrap().el.name));
    }
    let root = stack.pop().unwrap().el;
    root.children.into_iter().next().ok_or_else(|| "xml: no element".to_string())
}

/// The first child element named `name`.
pub fn child<'a>(el: Option<&'a El>, name: &str) -> Option<&'a El> {
    el?.children.iter().find(|c| c.name == name)
}

/// The child elements named `name`.
pub fn children_named<'a>(el: Option<&'a El>, name: &str) -> Vec<&'a El> {
    el.map(|e| e.children.iter().filter(|c| c.name == name).collect()).unwrap_or_default()
}

/// The element at a path of child names (`'A/B/C'`).
pub fn at<'a>(el: Option<&'a El>, path: &str) -> Option<&'a El> {
    let mut cur = el;
    for name in path.split('/') {
        cur = child(cur, name);
        cur?;
    }
    cur
}

/// The text of `path`'s `Wert`, none where it is missing or nil (`value`).
pub fn value(el: Option<&El>, path: &str) -> Option<String> {
    let w = child(at(el, path), "Wert")?;
    if w.attr("xsi:nil") == Some("true") {
        return None;
    }
    Some(w.text.clone())
}
