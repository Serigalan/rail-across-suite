// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Projects, variants and revisions on one database (`store.js`, AP 10.5): a
//! revision holds the whole project as its infra-data-1.0 document, gzipped; a
//! variant is a name and a head; the graph of revisions is walked in memory.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde_json::{Value, json};

use crate::auth::User;
use crate::http::ApiError;
use crate::util::{iso, uuid};

/// What `revision.schema_version` says of a payload: 4, an infra-data-1.0
/// document (decision 0021). 3 (the core 1.0 document) and 2 (oltjson-1.1)
/// are migrated into it once (data migration `0021-infra-data`); anything
/// else is refused as `format_outdated`.
pub const STORED_FORMAT: i64 = 4;

pub fn pack(payload: &Value) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(rail_across_js::stringify(payload).as_bytes()).unwrap();
    e.finish().unwrap()
}

pub fn unpack(buf: &[u8]) -> Option<Value> {
    let mut s = String::new();
    GzDecoder::new(buf).read_to_string(&mut s).ok()?;
    serde_json::from_str(&s).ok()
}

#[derive(Clone, Debug)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub created_at: String,
    pub template: bool,
    pub creator_name: Option<String>,
    pub creator_login: Option<String>,
}

impl Project {
    fn from_row(r: &Row) -> rusqlite::Result<Project> {
        Ok(Project {
            id: r.get("id")?,
            title: r.get("title")?,
            description: r.get("description")?,
            created_by: r.get("created_by")?,
            created_at: r.get("created_at")?,
            template: r.get::<_, i64>("template")? != 0,
            creator_name: r.get("creator_name").ok(),
            creator_login: r.get("creator_login").ok(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub parent_variant_id: Option<String>,
    pub head_revision_id: i64,
    pub created_by: i64,
    pub created_at: String,
    pub archived: bool,
}

impl Variant {
    fn from_row(r: &Row) -> rusqlite::Result<Variant> {
        Ok(Variant {
            id: r.get("id")?,
            project_id: r.get("project_id")?,
            name: r.get("name")?,
            parent_variant_id: r.get("parent_variant_id")?,
            head_revision_id: r.get("head_revision_id")?,
            created_by: r.get("created_by")?,
            created_at: r.get("created_at")?,
            archived: r.get::<_, i64>("archived")? != 0,
        })
    }
}

/// A variant as the API shows it (`publicVariant`).
pub fn public_variant(v: &Variant, head: Value, parent_ahead: usize) -> Value {
    json!({
        "id": v.id, "projectId": v.project_id, "name": v.name, "parentVariantId": v.parent_variant_id,
        "archived": v.archived, "createdAt": v.created_at, "head": head, "parentAhead": parent_ahead,
    })
}

#[derive(Clone, Debug)]
struct Member {
    project_id: String,
    user_id: i64,
    login: String,
    name: String,
    active: bool,
}

impl Member {
    fn public(&self) -> Value {
        json!({ "id": self.user_id, "login": self.login, "name": self.name, "active": self.active })
    }
}

fn members_query(db: &Connection, sql: &str, p: &[&dyn rusqlite::ToSql]) -> rusqlite::Result<Vec<Member>> {
    db.prepare(sql)?
        .query_map(p, |r| {
            Ok(Member {
                project_id: r.get("project_id").unwrap_or_default(),
                user_id: r.get("user_id")?,
                login: r.get("login")?,
                name: r.get("name")?,
                active: r.get::<_, i64>("active")? != 0,
            })
        })?
        .collect()
}

/// A revision with its document.
pub struct Revision {
    pub meta: Value,
    pub payload: Value,
    pub remaps: Value,
    pub error_keys: Vec<Value>,
}

type Graph = HashMap<i64, (Option<i64>, Option<i64>)>;

fn ancestors(graph: &Graph, id: i64) -> HashSet<i64> {
    let mut seen = HashSet::new();
    let mut stack = vec![Some(id)];
    while let Some(cur) = stack.pop() {
        let Some(cur) = cur else { continue };
        if !seen.insert(cur) {
            continue;
        }
        if let Some((p, m)) = graph.get(&cur) {
            stack.push(*p);
            stack.push(*m);
        }
    }
    seen
}

pub struct Store<'a> {
    pub db: &'a Connection,
    pub now: i64,
}

impl<'a> Store<'a> {
    pub fn new(db: &'a Connection, now: i64) -> Store<'a> {
        Store { db, now }
    }

    fn iso(&self) -> String {
        iso(self.now)
    }

    pub fn revision_meta(&self, id: i64) -> rusqlite::Result<Value> {
        self.db
            .query_row(
                "SELECT r.id, r.project_id, r.number, r.variant_id, r.parent_id, r.merge_parent_id, r.author_id,
                        r.created_at, r.message, r.image_hash, u.name AS author_name
                 FROM revision r JOIN user u ON u.id = r.author_id WHERE r.id = ?",
                [id],
                |r| {
                    Ok(json!({
                        "id": r.get::<_, i64>("id")?, "projectId": r.get::<_, String>("project_id")?, "number": r.get::<_, i64>("number")?,
                        "variantId": r.get::<_, String>("variant_id")?, "parentId": r.get::<_, Option<i64>>("parent_id")?,
                        "mergeParentId": r.get::<_, Option<i64>>("merge_parent_id")?,
                        "author": { "id": r.get::<_, i64>("author_id")?, "name": r.get::<_, String>("author_name")? },
                        "createdAt": r.get::<_, String>("created_at")?, "message": r.get::<_, String>("message")?,
                        "imageHash": r.get::<_, Option<String>>("image_hash")?,
                    }))
                },
            )
            .optional()
            .map(|v| v.unwrap_or(Value::Null))
    }

    pub fn project(&self, id: &str) -> rusqlite::Result<Option<Project>> {
        self.db
            .query_row("SELECT * FROM project WHERE id = ? AND deleted_at IS NULL", [id], Project::from_row)
            .optional()
    }

    pub fn variant(&self, id: &str) -> rusqlite::Result<Option<Variant>> {
        self.db.query_row("SELECT * FROM variant WHERE id = ?", [id], Variant::from_row).optional()
    }

    pub fn variants(&self, project_id: &str) -> rusqlite::Result<Vec<Variant>> {
        self.db
            .prepare("SELECT * FROM variant WHERE project_id = ? ORDER BY created_at, rowid")?
            .query_map([project_id], Variant::from_row)?
            .collect()
    }

    fn graph_of(&self, project_id: &str) -> rusqlite::Result<Graph> {
        self.db
            .prepare("SELECT id, parent_id, merge_parent_id FROM revision WHERE project_id = ?")?
            .query_map([project_id], |r| Ok((r.get::<_, i64>(0)?, (r.get(1)?, r.get(2)?))))?
            .collect()
    }

    /// A revision with its document; outdated formats refused.
    pub fn revision(&self, id: i64) -> Result<Option<Revision>, ApiError> {
        let Some((version, payload, remaps, keys)) = self
            .db
            .query_row("SELECT schema_version, payload, remaps, error_keys FROM revision WHERE id = ?", [id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
            })
            .optional()?
        else {
            return Ok(None);
        };
        let payload = match version {
            STORED_FORMAT => unpack(&payload).ok_or_else(|| ApiError::new(500, "internal"))?,
            _ => return Err(ApiError::new(409, "format_outdated")),
        };
        Ok(Some(Revision {
            meta: self.revision_meta(id)?,
            payload,
            remaps: serde_json::from_str(&remaps).unwrap_or(json!([])),
            error_keys: serde_json::from_str::<Vec<Value>>(&keys).unwrap_or_default(),
        }))
    }

    /// Whether `user` may see a project (decision 127).
    pub fn can_see(&self, user: &User, p: &Project) -> rusqlite::Result<bool> {
        if user.role == "admin" || p.template || p.created_by == user.id {
            return Ok(true);
        }
        Ok(self
            .db
            .query_row(
                "SELECT 1 FROM project_member WHERE project_id = ? AND user_id = ?",
                params![p.id, user.id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub fn can_manage(user: &User, p: &Project) -> bool {
        user.role == "admin" || p.created_by == user.id
    }

    pub fn members(&self, project_id: &str) -> rusqlite::Result<Value> {
        let m = members_query(
            self.db,
            "SELECT m.user_id, m.added_at, u.login, u.name, u.active FROM project_member m
             JOIN user u ON u.id = m.user_id WHERE m.project_id = ? ORDER BY u.name COLLATE NOCASE",
            &[&project_id],
        )?;
        Ok(Value::Array(m.iter().map(Member::public).collect()))
    }

    pub fn add_member(&self, project_id: &str, user_id: i64, added_by: i64) -> rusqlite::Result<()> {
        self.db.execute(
            "INSERT OR IGNORE INTO project_member (project_id, user_id, added_by, added_at) VALUES (?, ?, ?, ?)",
            params![project_id, user_id, added_by, self.iso()],
        )?;
        Ok(())
    }

    pub fn remove_member(&self, project_id: &str, user_id: i64) -> rusqlite::Result<()> {
        self.db
            .execute("DELETE FROM project_member WHERE project_id = ? AND user_id = ?", params![project_id, user_id])?;
        Ok(())
    }

    fn summary(&self, p: &Project, members: &[Member]) -> rusqlite::Result<Value> {
        let graph = self.graph_of(&p.id)?;
        let rows = self.variants(&p.id)?;
        let mut cache: HashMap<i64, HashSet<i64>> = HashMap::new();
        let mut variants = Vec::new();
        for v in &rows {
            let parent = v.parent_variant_id.as_ref().and_then(|id| rows.iter().find(|r| &r.id == id));
            let mut ahead = 0;
            if let Some(parent) = parent {
                let mine = cache.entry(v.head_revision_id).or_insert_with(|| ancestors(&graph, v.head_revision_id)).clone();
                let theirs = cache
                    .entry(parent.head_revision_id)
                    .or_insert_with(|| ancestors(&graph, parent.head_revision_id));
                ahead = theirs.iter().filter(|id| !mine.contains(id)).count();
            }
            variants.push(public_variant(v, self.revision_meta(v.head_revision_id)?, ahead));
        }
        let image = variants.first().and_then(|v| v["head"].get("imageHash")).cloned().unwrap_or(Value::Null);
        Ok(json!({
            "id": p.id, "title": p.title, "description": p.description.clone().unwrap_or_default(), "template": p.template,
            "createdBy": { "id": p.created_by, "name": p.creator_name, "login": p.creator_login }, "createdAt": p.created_at,
            "variants": variants, "imageHash": image,
            "members": members.iter().map(Member::public).collect::<Vec<_>>(),
        }))
    }

    fn visible_to(viewer: &User, p: &Project, members: &[Member]) -> bool {
        viewer.role == "admin" || p.template || p.created_by == viewer.id || members.iter().any(|m| m.user_id == viewer.id)
    }

    pub fn list_projects(&self, viewer: &User) -> rusqlite::Result<Value> {
        let all = members_query(
            self.db,
            "SELECT m.project_id, m.user_id, u.login, u.name, u.active FROM project_member m
             JOIN user u ON u.id = m.user_id ORDER BY u.name COLLATE NOCASE",
            &[],
        )?;
        let projects: Vec<Project> = self
            .db
            .prepare(
                "SELECT p.*, u.name AS creator_name, u.login AS creator_login FROM project p JOIN user u ON u.id = p.created_by
                 WHERE p.deleted_at IS NULL ORDER BY p.title COLLATE NOCASE",
            )?
            .query_map([], Project::from_row)?
            .collect::<Result<_, _>>()?;
        let mut out = Vec::new();
        for p in &projects {
            let members: Vec<Member> = all.iter().filter(|m| m.project_id == p.id).cloned().collect();
            if Self::visible_to(viewer, p, &members) {
                out.push(self.summary(p, &members)?);
            }
        }
        Ok(Value::Array(out))
    }

    pub fn project_summary(&self, viewer: &User, id: &str) -> rusqlite::Result<Value> {
        let Some(p) = self
            .db
            .query_row(
                "SELECT p.*, u.name AS creator_name, u.login AS creator_login FROM project p JOIN user u ON u.id = p.created_by
                 WHERE p.id = ? AND p.deleted_at IS NULL",
                [id],
                Project::from_row,
            )
            .optional()?
        else {
            return Ok(Value::Null);
        };
        let members = members_query(
            self.db,
            "SELECT m.project_id, m.user_id, u.login, u.name, u.active FROM project_member m
             JOIN user u ON u.id = m.user_id WHERE m.project_id = ? ORDER BY u.name COLLATE NOCASE",
            &[&id],
        )?;
        if Self::visible_to(viewer, &p, &members) {
            self.summary(&p, &members)
        } else {
            Ok(Value::Null)
        }
    }

    fn insert_revision(
        &self,
        project_id: &str,
        variant_id: &str,
        parent: Option<i64>,
        merge_parent: Option<i64>,
        author: i64,
        message: &str,
        payload: &Value,
        remaps: &Value,
        error_keys: &[String],
    ) -> Result<i64, ApiError> {
        let number: i64 = self
            .db
            .query_row("SELECT COALESCE(MAX(number), 0) + 1 FROM revision WHERE project_id = ?", [project_id], |r| {
                r.get(0)
            })?;
        let doc = rail_across_doc::persistence::project_document(payload, false, true).map_err(|e| ApiError::new(422, &e.code))?;
        let image = payload.get("imageHash").and_then(Value::as_str).map(str::to_string);
        self.db.execute(
            "INSERT INTO revision (project_id, number, variant_id, parent_id, merge_parent_id, author_id, created_at,
                                   message, schema_version, payload, remaps, error_keys, image_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                project_id,
                number,
                variant_id,
                parent,
                merge_parent,
                author,
                self.iso(),
                message,
                STORED_FORMAT,
                pack(&doc),
                rail_across_js::stringify(remaps),
                rail_across_js::stringify(&json!(error_keys)),
                image
            ],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// A new project with its first variant at revision 1 (`createProject`): (project, variant, revision).
    pub fn create_project(
        &self,
        title: &str,
        description: Option<&str>,
        payload: &Value,
        author: i64,
        error_keys: &[String],
        variant_name: &str,
        template: bool,
    ) -> Result<(String, String, i64), ApiError> {
        let (project_id, variant_id) = (uuid(), uuid());
        let mut record = payload.as_object().cloned().unwrap_or_default();
        record.insert("id".into(), json!(project_id));
        record.insert("title".into(), json!(title));
        if let Some(d) = description.filter(|d| !d.is_empty()) {
            record.insert("description".into(), json!(d));
        }
        self.db.execute(
            "INSERT INTO project (id, title, description, created_by, created_at, template) VALUES (?, ?, ?, ?, ?, ?)",
            params![project_id, title, description, author, self.iso(), template as i64],
        )?;
        let rev = self.insert_revision(&project_id, &variant_id, None, None, author, "", &Value::Object(record), &json!([]), error_keys)?;
        self.db.execute(
            "INSERT INTO variant (id, project_id, name, parent_variant_id, head_revision_id, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![variant_id, project_id, variant_name, Option::<String>::None, rev, author, self.iso()],
        )?;
        Ok((project_id, variant_id, rev))
    }

    pub fn patch_project(&self, id: &str, title: Option<&str>, description: Option<&str>) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE project SET title = COALESCE(?, title), description = COALESCE(?, description) WHERE id = ?",
            params![title, description, id],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> rusqlite::Result<()> {
        self.db.execute("UPDATE project SET deleted_at = ? WHERE id = ?", params![self.iso(), id])?;
        Ok(())
    }

    pub fn branch(&self, project_id: &str, name: &str, from_variant: &str, from_revision: i64, author: i64) -> rusqlite::Result<String> {
        let id = uuid();
        self.db.execute(
            "INSERT INTO variant (id, project_id, name, parent_variant_id, head_revision_id, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![id, project_id, name, from_variant, from_revision, author, self.iso()],
        )?;
        Ok(id)
    }

    pub fn rename_variant(&self, id: &str, name: &str) -> rusqlite::Result<()> {
        self.db.execute("UPDATE variant SET name = ? WHERE id = ?", params![name, id])?;
        Ok(())
    }

    pub fn archive_variant(&self, id: &str, archived: bool) -> rusqlite::Result<()> {
        self.db.execute("UPDATE variant SET archived = ? WHERE id = ?", params![archived as i64, id])?;
        Ok(())
    }

    /// Write a revision as the variant's new head, only if the head is still `base`: Ok(revision) or Err(head).
    pub fn check_in(
        &self,
        variant: &Variant,
        base: i64,
        merge_parent: Option<i64>,
        author: i64,
        message: &str,
        payload: &Value,
        remaps: &Value,
        error_keys: &[String],
    ) -> Result<Result<i64, i64>, ApiError> {
        self.db.execute_batch("BEGIN IMMEDIATE")?;
        let run = || -> Result<Result<i64, i64>, ApiError> {
            let head: i64 = self
                .db
                .query_row("SELECT head_revision_id FROM variant WHERE id = ?", [&variant.id], |r| r.get(0))?;
            if head != base {
                return Ok(Err(head));
            }
            let rev = self.insert_revision(
                &variant.project_id,
                &variant.id,
                Some(base),
                merge_parent,
                author,
                message,
                payload,
                remaps,
                error_keys,
            )?;
            let changed = self.db.execute(
                "UPDATE variant SET head_revision_id = ? WHERE id = ? AND head_revision_id = ?",
                params![rev, variant.id, base],
            )?;
            if changed != 1 {
                return Err(ApiError::new(500, "internal"));
            }
            Ok(Ok(rev))
        };
        match run() {
            Ok(r) => {
                self.db.execute_batch("COMMIT")?;
                Ok(r)
            }
            Err(e) => {
                let _ = self.db.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    pub fn history(&self, variant: &Variant, limit: usize) -> rusqlite::Result<Value> {
        let mut out = Vec::new();
        let mut id = Some(variant.head_revision_id);
        while let Some(cur) = id {
            if out.len() >= limit {
                break;
            }
            let m = self.revision_meta(cur)?;
            if m.is_null() {
                break;
            }
            id = m["parentId"].as_i64();
            out.push(m);
        }
        Ok(Value::Array(out))
    }

    pub fn common_base(&self, project_id: &str, a: i64, b: i64) -> rusqlite::Result<Option<i64>> {
        let g = self.graph_of(project_id)?;
        let of_a = ancestors(&g, a);
        Ok(ancestors(&g, b).into_iter().filter(|id| of_a.contains(id)).max())
    }

    pub fn remaps_between(&self, project_id: &str, base: Option<i64>, head: i64) -> rusqlite::Result<Value> {
        let g = self.graph_of(project_id)?;
        let known = base.map(|b| ancestors(&g, b)).unwrap_or_default();
        let mut ids: Vec<i64> = ancestors(&g, head).into_iter().filter(|id| !known.contains(id)).collect();
        ids.sort();
        let mut out = Vec::new();
        for id in ids {
            let r: String = self.db.query_row("SELECT remaps FROM revision WHERE id = ?", [id], |r| r.get(0))?;
            if let Ok(Value::Array(a)) = serde_json::from_str(&r) {
                out.extend(a);
            }
        }
        Ok(Value::Array(out))
    }

    /// Timetables ({ name, data }) added to a project as `user`'s; their ids.
    pub fn add_timetables(&self, project_id: &str, timetables: &[(String, String)], user: i64) -> rusqlite::Result<Vec<String>> {
        let at = self.iso();
        let mut ids = Vec::new();
        for (name, data) in timetables {
            let id = uuid();
            self.db.execute(
                "INSERT INTO timetable (id, project_id, name, data, created_by, created_at, updated_by, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![id, project_id, name, data, user, at, user, at],
            )?;
            ids.push(id);
        }
        Ok(ids)
    }

    pub fn copy_timetables(&self, from: &str, to: &str, user: i64) -> rusqlite::Result<usize> {
        let list: Vec<(String, String)> = self
            .db
            .prepare("SELECT name, data FROM timetable WHERE project_id = ? AND deleted_at IS NULL ORDER BY created_at, rowid")?
            .query_map([from], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(self.add_timetables(to, &list, user)?.len())
    }
}

/// Run `f` in a transaction.
pub fn transaction<T>(db: &Connection, f: impl FnOnce() -> Result<T, ApiError>) -> Result<T, ApiError> {
    db.execute_batch("BEGIN")?;
    match f() {
        Ok(v) => {
            db.execute_batch("COMMIT")?;
            Ok(v)
        }
        Err(e) => {
            let _ = db.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}
