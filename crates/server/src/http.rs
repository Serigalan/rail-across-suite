// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Requests and answers as the routes see them, apart from the HTTP library:
//! a request with its body parsed the way Fastify parses it, an answer with
//! JSON, bytes or a piece of a file, and the refusal (`ApiError`).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

/// A request's body: none, JSON, bytes (an upload), or one Fastify refuses.
#[derive(Debug, Clone)]
pub enum Body {
    None,
    Json(Value),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Req {
    pub method: String,
    pub path: String,
    pub url: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
    pub body: Body,
    pub ip: String,
    /// The route's parameters, as `:name` in its pattern.
    pub params: HashMap<String, String>,
}

impl Req {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
    pub fn param(&self, name: &str) -> &str {
        self.params.get(name).map(String::as_str).unwrap_or("")
    }
    /// The JSON body, `{}`-less: Null where there is none.
    pub fn json(&self) -> &Value {
        static NULL: Value = Value::Null;
        match &self.body {
            Body::Json(v) => v,
            _ => &NULL,
        }
    }
    /// A field of the JSON body (`req.body?.[key]`), absent as None.
    pub fn field(&self, key: &str) -> Option<&Value> {
        self.json().get(key)
    }
}

/// What an answer carries.
#[derive(Debug)]
pub enum Payload {
    Empty,
    Json(Value),
    Bytes(Vec<u8>),
    /// A part of a file: path, start, length.
    File(std::path::PathBuf, u64, u64),
}

#[derive(Debug)]
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub payload: Payload,
}

impl Resp {
    pub fn json(status: u16, v: Value) -> Resp {
        Resp {
            status,
            headers: vec![("content-type".into(), "application/json; charset=utf-8".into())],
            payload: Payload::Json(v),
        }
    }
    pub fn ok(v: Value) -> Resp {
        Resp::json(200, v)
    }
    pub fn empty(status: u16) -> Resp {
        Resp {
            status,
            headers: vec![],
            payload: Payload::Empty,
        }
    }
    pub fn bytes(status: u16, content_type: &str, b: Vec<u8>) -> Resp {
        Resp {
            status,
            headers: vec![("content-type".into(), content_type.into())],
            payload: Payload::Bytes(b),
        }
    }
    pub fn header(mut self, k: &str, v: impl Into<String>) -> Resp {
        let k = k.to_ascii_lowercase();
        self.headers.retain(|(h, _)| *h != k || h == "set-cookie");
        self.headers.push((k, v.into()));
        self
    }
    pub fn cookie(mut self, v: String) -> Resp {
        self.headers.push(("set-cookie".into(), v));
        self
    }
}

/// A refusal with an `error` code the browser translates, and what else it says.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    pub error: String,
    pub extra: Map<String, Value>,
}

impl ApiError {
    pub fn new(status: u16, error: &str) -> ApiError {
        ApiError {
            status,
            error: error.into(),
            extra: Map::new(),
        }
    }
    pub fn with(mut self, key: &str, v: Value) -> ApiError {
        self.extra.insert(key.into(), v);
        self
    }
    pub fn into_resp(self) -> Resp {
        let mut o = Map::new();
        o.insert("error".into(), json!(self.error));
        for (k, v) in self.extra {
            o.insert(k, v);
        }
        Resp::json(self.status, Value::Object(o))
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("database: {e}");
        ApiError::new(500, "internal")
    }
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        eprintln!("io: {e}");
        ApiError::new(500, "internal")
    }
}

pub type Res = Result<Resp, ApiError>;

/// Match `path` against a pattern of `/`-separated parts, `:name` binding one.
pub fn matches(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let (p, s): (Vec<&str>, Vec<&str>) = (pattern.split('/').collect(), path.split('/').collect());
    if p.len() != s.len() {
        return None;
    }
    let mut out = HashMap::new();
    for (a, b) in p.iter().zip(&s) {
        if let Some(name) = a.strip_prefix(':') {
            if b.is_empty() {
                return None;
            }
            out.insert(name.to_string(), percent_decode(b));
        } else if a != b {
            return None;
        }
    }
    Some(out)
}

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A query string as Fastify reads it (the first of a repeated key).
pub fn parse_query(q: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in q.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let dec = |s: &str| percent_decode(&s.replace('+', " "));
        out.entry(dec(k)).or_insert_with(|| dec(v));
    }
    out
}

/// The cookies of a `Cookie` header.
pub fn parse_cookies(h: Option<&str>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for part in h.unwrap_or("").split(';') {
        if let Some((k, v)) = part.split_once('=') {
            out.entry(k.trim().to_string()).or_insert_with(|| percent_decode(v.trim().trim_matches('"')));
        }
    }
    out
}

/// Whether address `a` is trusted by entry `t`: an address, or a CIDR range (`proxy-addr`'s).
fn trusted_by(t: &str, a: &str) -> bool {
    use std::net::IpAddr;
    let norm = |ip: IpAddr| match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    };
    let Ok(addr) = a.parse::<IpAddr>().map(norm) else { return t == a };
    let (net, bits) = match t.split_once('/') {
        Some((n, b)) => (n, b.parse::<u32>().ok()),
        None => (t, None),
    };
    let Ok(net) = net.parse::<IpAddr>().map(norm) else { return false };
    match (net, addr) {
        (IpAddr::V4(n), IpAddr::V4(x)) => {
            let b = bits.unwrap_or(32).min(32);
            let mask = if b == 0 { 0 } else { u32::MAX << (32 - b) };
            u32::from(n) & mask == u32::from(x) & mask
        }
        (IpAddr::V6(n), IpAddr::V6(x)) => {
            let b = bits.unwrap_or(128).min(128);
            let mask = if b == 0 { 0 } else { u128::MAX << (128 - b) };
            u128::from(n) & mask == u128::from(x) & mask
        }
        _ => false,
    }
}

/// The client's address (`req.ip` with Fastify's trustProxy list): the first
/// of the socket's and the forwarded addresses, nearest first, not trusted.
pub fn client_ip(remote: &str, forwarded: Option<&str>, trusted: &[String]) -> String {
    let mut addrs = vec![remote.to_string()];
    if let Some(f) = forwarded {
        addrs.extend(f.split(',').rev().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
    }
    let trust = |a: &str| trusted.iter().any(|t| trusted_by(t, a));
    for a in &addrs[..addrs.len() - 1] {
        if !trust(a) {
            return a.clone();
        }
    }
    addrs.last().unwrap().clone()
}

/// A request from its method, URL, headers (lower-case names) and the peer's address.
pub fn build_req(method: &str, url: &str, headers: HashMap<String, String>, remote: &str, trusted: &[String]) -> Req {
    let path = url.split('?').next().unwrap_or("/").to_string();
    let ip = client_ip(remote, headers.get("x-forwarded-for").map(String::as_str), trusted);
    Req {
        method: method.to_string(),
        query: parse_query(url.split_once('?').map(|(_, q)| q).unwrap_or("")),
        cookies: parse_cookies(headers.get("cookie").map(String::as_str)),
        path,
        url: url.to_string(),
        headers,
        body: Body::None,
        ip,
        params: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::client_ip;

    #[test]
    fn believes_forwarded_addresses_only_from_trusted_proxies() {
        let t = vec!["127.0.0.1".to_string(), "::1".to_string(), "10.0.0.0/8".to_string()];
        assert_eq!(client_ip("203.0.113.9", Some("198.51.100.1"), &t), "203.0.113.9");
        assert_eq!(client_ip("127.0.0.1", Some("198.51.100.77"), &t), "198.51.100.77");
        assert_eq!(client_ip("10.1.2.3", Some("198.51.100.5, 10.9.9.9"), &t), "198.51.100.5");
        assert_eq!(client_ip("::1", None, &t), "::1");
        assert_eq!(client_ip("127.0.0.1", Some("10.0.0.1"), &t), "10.0.0.1");
    }
}
