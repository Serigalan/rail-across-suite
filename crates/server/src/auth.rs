// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Users and sessions (`auth.js`): argon2id passwords in PHC strings (the
//! Node server's @node-rs/argon2 writes the same), session tokens of which
//! only the SHA-256 reaches the database, the login brake in memory.

use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::util::{base64url, iso, random_bytes, sha256_hex};

const PASSWORD_MIN: usize = 12;
/// A session lives this long without being used [ms].
pub const SESSION_IDLE: i64 = 30 * 24 * 3600 * 1000;
pub const BRAKE_ATTEMPTS: u32 = 5;
const BRAKE_MS: i64 = 60 * 1000;
pub const COOKIE: &str = "ra_session";

/// A user row.
#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
    pub active: bool,
    pub must_change_password: bool,
    pub can_edit_clouds: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

impl User {
    pub fn from_row(r: &Row) -> rusqlite::Result<User> {
        Ok(User {
            id: r.get("id")?,
            login: r.get("login")?,
            name: r.get("name")?,
            password_hash: r.get("password_hash")?,
            role: r.get("role")?,
            active: r.get::<_, i64>("active")? != 0,
            must_change_password: r.get::<_, i64>("must_change_password")? != 0,
            can_edit_clouds: r.get::<_, Option<i64>>("can_edit_clouds").unwrap_or(Some(0)).unwrap_or(0) != 0,
            created_at: r.get("created_at")?,
            last_login_at: r.get("last_login_at")?,
        })
    }

    /// The user as the API shows them — never the hash (`publicUser`).
    pub fn public(&self) -> Value {
        json!({
            "id": self.id, "login": self.login, "name": self.name, "role": self.role, "active": self.active,
            "mustChangePassword": self.must_change_password, "canEditClouds": self.can_edit_clouds,
            "createdAt": self.created_at, "lastLoginAt": self.last_login_at,
        })
    }

    /// Whether the user may change point clouds (`mayEditClouds`, decision 203).
    pub fn may_edit_clouds(&self) -> bool {
        self.role == "admin" || self.can_edit_clouds
    }
}

/// The login of the one implicit user of the local package.
pub const LOCAL_LOGIN: &str = "local";

/// The one user of the local package (`ensureLocalUser`): made once, never signed in as.
pub fn ensure_local_user(db: &Connection, now_iso: String) -> rusqlite::Result<i64> {
    if let Some(u) = user_by_login(db, LOCAL_LOGIN)? {
        return Ok(u.id);
    }
    db.execute(
        "INSERT INTO user (login, name, password_hash, role, active, must_change_password, created_at) VALUES (?, 'Lokal', ?, 'admin', 1, 0, ?)",
        params![LOCAL_LOGIN, crate::util::hex(&random_bytes(32)), now_iso],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::encode_b64(&random_bytes(16)).expect("salt");
    Argon2::default().hash_password(password.as_bytes(), &salt).expect("argon2").to_string()
}

pub fn verify_password(stored: &str, password: &str) -> bool {
    PasswordHash::new(stored).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

/// `passwordAcceptable`: a string of at least 12 code points.
pub fn password_acceptable(p: Option<&Value>) -> bool {
    p.and_then(Value::as_str).is_some_and(|s| s.chars().count() >= PASSWORD_MIN)
}

pub fn random_password() -> String {
    base64url(&random_bytes(15))
}

pub fn user_by_id(db: &Connection, id: i64) -> rusqlite::Result<Option<User>> {
    db.query_row("SELECT * FROM user WHERE id = ?", [id], User::from_row).optional()
}

pub fn user_by_login(db: &Connection, login: &str) -> rusqlite::Result<Option<User>> {
    db.query_row("SELECT * FROM user WHERE login = ?", [login.trim()], User::from_row).optional()
}

pub fn create_user(db: &Connection, login: &str, name: &str, password: &str, role: &str, must_change: bool, now: i64) -> rusqlite::Result<User> {
    db.execute(
        "INSERT INTO user (login, name, password_hash, role, active, must_change_password, created_at) VALUES (?, ?, ?, ?, 1, ?, ?)",
        params![login.trim(), name.trim(), hash_password(password), role, must_change as i64, iso(now)],
    )?;
    Ok(user_by_id(db, db.last_insert_rowid())?.unwrap())
}

pub fn set_password(db: &Connection, user_id: i64, password: &str, must_change: bool) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE user SET password_hash = ?, must_change_password = ? WHERE id = ?",
        params![hash_password(password), must_change as i64, user_id],
    )?;
    Ok(())
}

/// Failed logins of one login and address.
#[derive(Clone, Copy, Debug, Default)]
pub struct Failure {
    pub count: u32,
    pub until: i64,
    pub last: i64,
}

fn key(login: &str, address: &str) -> String {
    format!("{}|{address}", login.to_lowercase())
}

/// Seconds until this login may be tried again from this address, or 0.
pub fn braked(app: &crate::app::App, login: &str, address: &str) -> i64 {
    let f = app.failures.lock().unwrap();
    match f.get(&key(login, address)) {
        Some(f) if f.until != 0 => {
            let left = f.until - app.now();
            if left > 0 { (left + 999) / 1000 } else { 0 }
        }
        _ => 0,
    }
}

/// Check a login: the user, or none — the same for an unknown login, a wrong password and a deactivated user.
pub fn check(app: &crate::app::App, db: &Connection, login: &str, password: &str, address: &str) -> rusqlite::Result<Option<User>> {
    let k = key(login, address);
    let user = user_by_login(db, login)?;
    if let Some(u) = user.filter(|u| u.active && verify_password(&u.password_hash, password)) {
        app.failures.lock().unwrap().remove(&k);
        return Ok(Some(u));
    }
    record_failure(app, login, address);
    Ok(None)
}

/// A failed login of `login` from `address`, counted toward the brake.
pub fn record_failure(app: &crate::app::App, login: &str, address: &str) {
    let now = app.now();
    let mut all = app.failures.lock().unwrap();
    let f = all.entry(key(login, address)).or_default();
    if f.until != 0 && f.until <= now {
        f.count = 0;
        f.until = 0;
    }
    f.count += 1;
    f.last = now;
    if f.count >= BRAKE_ATTEMPTS {
        f.until = now + BRAKE_MS;
    }
}

/// Open a session; the token for the cookie.
pub fn open_session(db: &Connection, user_id: i64, now: i64) -> rusqlite::Result<String> {
    let token = base64url(&random_bytes(32));
    db.execute(
        "INSERT INTO session (token_hash, user_id, created_at, last_seen_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        params![sha256_hex(token.as_bytes()), user_id, iso(now), iso(now), iso(now + SESSION_IDLE)],
    )?;
    db.execute("UPDATE user SET last_login_at = ? WHERE id = ?", params![iso(now), user_id])?;
    Ok(token)
}

/// The user behind a session token, the session renewed.
pub fn resolve_session(db: &Connection, token: Option<&str>, now: i64) -> rusqlite::Result<Option<User>> {
    let Some(token) = token.filter(|t| !t.is_empty()) else { return Ok(None) };
    let h = sha256_hex(token.as_bytes());
    let Some((user_id, expires)) = db
        .query_row("SELECT user_id, expires_at FROM session WHERE token_hash = ?", [&h], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .optional()?
    else {
        return Ok(None);
    };
    if expires < iso(now) {
        db.execute("DELETE FROM session WHERE token_hash = ?", [&h])?;
        return Ok(None);
    }
    let user = user_by_id(db, user_id)?;
    let Some(user) = user.filter(|u| u.active) else {
        db.execute("DELETE FROM session WHERE user_id = ?", [user_id])?;
        return Ok(None);
    };
    db.execute(
        "UPDATE session SET last_seen_at = ?, expires_at = ? WHERE token_hash = ?",
        params![iso(now), iso(now + SESSION_IDLE), h],
    )?;
    Ok(Some(user))
}

pub fn close_session(db: &Connection, token: Option<&str>) -> rusqlite::Result<()> {
    if let Some(t) = token.filter(|t| !t.is_empty()) {
        db.execute("DELETE FROM session WHERE token_hash = ?", [sha256_hex(t.as_bytes())])?;
    }
    Ok(())
}

pub fn close_all_sessions(db: &Connection, user_id: i64) -> rusqlite::Result<()> {
    db.execute("DELETE FROM session WHERE user_id = ?", [user_id])?;
    Ok(())
}

/// Drop expired sessions, and failed logins nobody has repeated for a brake's length.
pub fn sweep(app: &crate::app::App, db: &Connection) -> rusqlite::Result<()> {
    let now = app.now();
    db.execute("DELETE FROM session WHERE expires_at < ?", [iso(now)])?;
    app.failures
        .lock()
        .unwrap()
        .retain(|_, f| !(if f.until != 0 { f.until <= now } else { f.last + BRAKE_MS <= now }));
    Ok(())
}
