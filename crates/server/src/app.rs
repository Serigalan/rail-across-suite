// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The server's state and what every request goes through (`app.js`): the
//! JSON-only rule for writing requests, the session, the body as Fastify
//! parses it, the refusals as JSON.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::auth::{COOKIE, Failure, User};
use crate::http::{ApiError, Body, Req, Resp};

/// Largest request body [bytes].
pub const BODY_LIMIT: usize = 20 * 1024 * 1024;

/// How the server was started.
#[derive(Clone, Debug)]
pub struct Settings {
    pub secure_cookie: bool,
    pub trust_proxy: Vec<String>,
    pub cloud_root: Option<PathBuf>,
    /// Tests: the clock comes from `x-ra-test-now`, the peer from `x-ra-test-remote`, and /api/__test answers.
    pub test_mode: bool,
    /// The local package (decision 0020 point 5): no user management, one implicit local user.
    pub local: bool,
    /// The built suite served beside the API, one origin (as Caddy does in production).
    pub static_root: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            secure_cookie: true,
            trust_proxy: vec!["127.0.0.1".into(), "::1".into()],
            cloud_root: None,
            test_mode: false,
            local: false,
            static_root: None,
        }
    }
}

pub struct App {
    pub db: Mutex<Connection>,
    pub settings: Settings,
    pub clock: Mutex<Option<i64>>,
    pub failures: Mutex<HashMap<String, Failure>>,
    /// Membership by session and project, believed for a minute (clouds).
    pub seen: Mutex<HashMap<String, i64>>,
    /// Clouds a piece is being written to.
    pub busy: Mutex<HashSet<String>>,
    pub terrain: crate::terrain::store::TileStore,
    pub dgm5: crate::terrain::dem::Dgm5,
    /// The one user of the local package.
    pub local_user: Option<i64>,
}

/// A request as a route handles it: the request, the signed-in user, the database.
pub struct Ctx<'a> {
    pub app: &'a App,
    pub req: &'a Req,
    pub user: Option<User>,
    pub db: MutexGuard<'a, Connection>,
}

impl Ctx<'_> {
    pub fn now(&self) -> i64 {
        self.app.now()
    }
    pub fn iso(&self) -> String {
        crate::util::iso(self.now())
    }
    /// preHandler requireUser: a signed-in user who may work.
    pub fn user(&self) -> Result<&User, ApiError> {
        let u = self.user.as_ref().ok_or_else(|| ApiError::new(401, "unauthenticated"))?;
        if u.must_change_password {
            return Err(ApiError::new(403, "password_change_required"));
        }
        Ok(u)
    }
    pub fn admin(&self) -> Result<&User, ApiError> {
        let u = self.user()?;
        if u.role != "admin" {
            return Err(ApiError::new(403, "admin_only"));
        }
        Ok(u)
    }
}

const WRITING: [&str; 4] = ["POST", "PUT", "PATCH", "DELETE"];

/// The writing requests that carry bytes: a piece of an upload, an Access file.
pub fn upload_limit(method: &str, path: &str) -> Option<usize> {
    let parts: Vec<&str> = path.split('/').collect();
    if method == "PUT" && parts.len() == 7 && parts[1] == "api" && parts[2] == "projects" && parts[4] == "clouds" && parts[6] == "raw" {
        return Some(crate::routes::clouds::CHUNK_LIMIT);
    }
    if method == "POST" && path == "/api/mdb" {
        return Some(crate::routes::terrain::max_mdb_bytes());
    }
    None
}

impl App {
    pub fn new(db: Connection, settings: Settings) -> App {
        let local_user = settings
            .local
            .then(|| crate::auth::ensure_local_user(&db, crate::util::iso(Self::clock_now(None))).expect("the local user"));
        App {
            local_user,
            db: Mutex::new(db),
            settings,
            clock: Mutex::new(None),
            failures: Mutex::new(HashMap::new()),
            seen: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            terrain: crate::terrain::store::TileStore::from_env(),
            dgm5: crate::terrain::dem::Dgm5::default(),
        }
    }

    fn clock_now(test: Option<i64>) -> i64 {
        test.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
    }

    pub fn now(&self) -> i64 {
        Self::clock_now(*self.clock.lock().unwrap())
    }

    /// One request, its body as bytes (`too_large` where it passed its limit).
    pub fn handle(&self, mut req: Req, raw: Vec<u8>, too_large: bool) -> Resp {
        if self.settings.test_mode {
            if let Some(t) = req.header("x-ra-test-now").and_then(|t| t.parse::<f64>().ok()) {
                *self.clock.lock().unwrap() = Some(t as i64);
            }
        }
        let content_type = req.header("content-type").unwrap_or("").to_string();
        let upload =
            upload_limit(&req.method, &req.path).is_some() && content_type.starts_with("application/octet-stream") && req.header("x-olt-upload") == Some("1");
        if WRITING.contains(&req.method.as_str()) && !content_type.starts_with("application/json") && !upload {
            return ApiError::new(415, "json_required").into_resp();
        }
        let db = self.db.lock().unwrap();
        let user = match self.local_user {
            Some(id) => crate::auth::user_by_id(&db, id),
            None => crate::auth::resolve_session(&db, req.cookies.get(COOKIE).map(String::as_str), self.now()),
        };
        if too_large {
            return ApiError::new(413, "too_large").into_resp();
        }
        if content_type.starts_with("application/json") && req.method != "GET" && req.method != "HEAD" {
            if raw.is_empty() {
                return Resp::json(
                    400,
                    json!({ "error": "bad_request", "message": "Body cannot be empty when content-type is set to 'application/json'" }),
                );
            }
            match serde_json::from_slice::<Value>(&raw) {
                Ok(v) => req.body = Body::Json(v),
                Err(e) => return Resp::json(400, json!({ "error": "bad_request", "message": e.to_string() })),
            }
        } else if content_type.starts_with("application/octet-stream") && upload {
            req.body = Body::Bytes(raw);
        } else if !raw.is_empty() && req.method != "GET" && req.method != "HEAD" {
            return Resp::json(
                415,
                json!({ "error": "bad_request", "message": format!("Unsupported Media Type: {content_type}") }),
            );
        }
        let Some((handler, params)) = crate::routes::find(&req.method, &req.path, &self.settings) else {
            if let (Some(root), "GET" | "HEAD") = (&self.settings.static_root, req.method.as_str()) {
                return crate::routes::site::serve(root, &req);
            }
            return Resp::json(
                404,
                json!({ "message": format!("Route {}:{} not found", req.method, req.url), "error": "Not Found", "statusCode": 404 }),
            );
        };
        req.params = params;
        let mut ctx = Ctx {
            app: self,
            req: &req,
            user: user.ok().flatten(),
            db,
        };
        match handler(&mut ctx) {
            Ok(r) => r,
            Err(e) => e.into_resp(),
        }
    }

    /// The session cookie (`sessionCookie`).
    pub fn session_cookie(&self, token: &str) -> String {
        let secure = if self.settings.secure_cookie { "; Secure" } else { "" };
        format!(
            "{COOKIE}={token}; Max-Age={}; Path=/; HttpOnly{secure}; SameSite=Strict",
            crate::auth::SESSION_IDLE / 1000
        )
    }

    pub fn clear_session_cookie(&self) -> String {
        let secure = if self.settings.secure_cookie { "; Secure" } else { "" };
        format!("{COOKIE}=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; Path=/; HttpOnly{secure}; SameSite=Strict")
    }

    /// One request handled in this process, without HTTP (the desktop app's URI
    /// scheme): (status, headers, body). A file answer is read into the body.
    pub fn call(&self, method: &str, url: &str, headers: Vec<(String, String)>, body: Vec<u8>) -> (u16, Vec<(String, String)>, Vec<u8>) {
        let mut map = HashMap::new();
        for (k, v) in headers {
            map.entry(k.to_ascii_lowercase()).or_insert(v);
        }
        let too_large = body.len() > upload_limit(method, url.split('?').next().unwrap_or("")).unwrap_or(BODY_LIMIT);
        let req = crate::http::build_req(method, url, map, "127.0.0.1", &self.settings.trust_proxy);
        let resp = self.handle(req, if too_large { Vec::new() } else { body }, too_large);
        let mut headers = resp.headers;
        let body = match resp.payload {
            crate::http::Payload::Empty => Vec::new(),
            crate::http::Payload::Json(v) => {
                if !headers.iter().any(|(k, _)| k == "content-type") {
                    headers.push(("content-type".into(), "application/json; charset=utf-8".into()));
                }
                rail_across_js::stringify(&v).into_bytes()
            }
            crate::http::Payload::Bytes(b) => b,
            crate::http::Payload::File(path, start, len) => {
                use std::io::{Read, Seek, SeekFrom};
                let mut out = Vec::new();
                if let Ok(mut f) = std::fs::File::open(path) {
                    let _ = f.seek(SeekFrom::Start(start));
                    let _ = f.take(len).read_to_end(&mut out);
                }
                out
            }
        };
        (resp.status, headers, if method == "HEAD" { Vec::new() } else { body })
    }
}
