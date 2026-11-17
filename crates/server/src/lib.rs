// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The project server of Rail-Across-Suite (decision 0005; it took
//! the place of the Node server, whose database, migrations and API it kept) —
//! users and sessions, projects with their variants and revisions checked by
//! the core's crates, vehicles and timetables, point clouds with their
//! uploads, tiles, share links and jobs (as threads), the Länder's DGM1 and
//! the MDB import. Its HTTP tests are `server/test/`.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::wrong_self_convention
)]

pub mod app;
pub mod auth;
pub mod checks;
pub mod clouds;
pub mod data_migrations;
pub mod db;
pub mod http;
pub mod mdb;
pub mod operations;
pub mod routes;
pub mod serve;
pub mod store;
pub mod terrain;
pub mod util;
