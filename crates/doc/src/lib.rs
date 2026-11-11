// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The infra-data-1.0 document (decision 0020, step 1) — the `document` module of
//! the core and the door of its `project` module in Rust: canonical JSON and
//! content hashes, the manifest, the part of JSON Schema the domain schemas
//! use, the record ↔ document conversion with its operational point register,
//! the door with the schema steps of earlier documents, and the record as it
//! is kept (`dehydrateProjects`, `parseProjectsPayload`, `projectDocument`).
//! Each function answers as its namesake in `packages/core/src` does; the
//! vectors the JavaScript records hold it to that.

pub mod canonical;
pub mod core_format;
pub mod door;
pub mod error;
pub mod layers;
pub mod manifest;
pub mod persistence;
pub mod schema;
pub mod stations;

pub use error::PayloadError;
