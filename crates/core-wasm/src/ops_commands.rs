// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! The commands' operations, named and called as the JavaScript's.

use rail_across_commands::{edit, roles, table, tracks as tr};
use rail_across_js::number;
use serde_json::{Value, json};

use crate::args::{Args, opt_num};
use crate::ops::Failure;

/// Runs command operation `op`, where it is one.
pub fn run(op: &str, input: &Value) -> Option<Result<Value, Failure>> {
    let a = Args::new(input);
    let ok = |v: Value| Some(Ok(v));
    match op {
        // tracks
        "commands.trackMeta" => ok(tr::track_meta(a.v(0), a.v(1))),
        "commands.trackOf" => ok(tr::track_of(a.list(0), a.v(1), a.opt(2).unwrap_or(&json!({})), a.opt(3))),
        "commands.buildLineTrack" => ok(tr::build_line_track(a.v(0))),
        "commands.curveCant" => ok(number(tr::curve_cant(a.f(0), a.opt(1)))),
        "commands.buildCurvedLineTrack" => ok(tr::build_curved_line_track(a.v(0))),
        "commands.buildParallelLineTrack" => ok(tr::build_parallel_line_track(a.v(0))),
        "commands.buildParallelTrack" => ok(tr::build_parallel_track(a.v(0))),
        "commands.buildConnectStraight" => ok(Value::Array(tr::build_connect_straight(a.v(0)))),
        "commands.cantAtTrackEnd" => ok(opt_num(tr::cant_at_track_end(a.opt(0)))),
        "commands.buildConnectCurved" => ok(Value::Array(tr::build_connect_curved(a.v(0)))),
        "commands.translateTracks" => ok(tr::translate_tracks(a.v(0), &a.list(1), a.f(2), a.f(3))),
        // elementRoles
        "commands.intermediateSpan" => ok(roles::span_json(roles::intermediate_span(&a.list(0), a.u(1)))),
        "commands.elementRoles" => ok(Value::Array(roles::element_roles(&a.list(0)))),
        "commands.rolesFor" => ok(json!(roles::roles_for(a.v(0)))),
        "commands.withRole" => {
            let (elements, error) = roles::with_role(&a.list(0), a.u(1), a.s(2));
            ok(json!({ "elements": elements, "error": error }))
        }
        // tableView
        "commands.tableParts" => ok(table::table_parts(a.v(0), &a.list(1), &a.list(2))),
        "commands.tableRows" => ok(table::table_rows(&a.list(0), &a.list(1))),
        "commands.rowOf" => ok(json!(table::row_of(&a.list(0), a.v(1), a.u(2)))),
        "commands.tableRoles" => ok(Value::Array(table::table_roles(&a.list(0), &a.list(1)))),
        "commands.trackEdit" => ok(table::track_edit(a.v(0), a.s(1), a.v(2))),
        "commands.writeRows" => ok(Value::Array(table::write_rows(&a.list(0), &a.list(1), &a.list(2)))),
        "commands.inRunElementChange" => {
            let (track_id, change) = (a.v(0)["trackId"].clone(), a.v(3).clone());
            ok(table::in_run(a.v(0), &a.list(1), &a.list(2), &|t, s, i| {
                edit::plan_element_change(t, s, &track_id, i, &change)
            }))
        }
        // editGeometry
        "commands.planElementChange" => ok(edit::plan_element_change(&a.list(0), &a.list(1), a.v(2), a.u(3), a.v(4))),
        "commands.planIntermediateChange" => ok(edit::plan_intermediate_change(&a.list(0), a.v(1), a.u(2), a.of(3))),
        "commands.mergeElementEdits" => {
            let list = |k: &str| a.option(2, k).and_then(Value::as_array).cloned().unwrap_or_default();
            ok(Value::Array(edit::merge_element_edits(
                &a.list(0),
                &a.list(1),
                &list("changed"),
                &list("reshaped"),
            )))
        }
        _ => None,
    }
}
