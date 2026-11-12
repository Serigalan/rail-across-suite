# 0026 — A project's operations in revisions, as one operations-1.0 document

Status: **accepted** · 2026-11-12 · changes point 1 of [0016](0016-timetables-of-a-project.md) (the counter per
timetable); carries out O2 of the [roadmap](../../ROADMAP.md), phase 4

## Decision

1. **A project's operations are one document, kept in revisions.** The whole of a project's timetables, their
   trains, the kinds of trains, the formations and what comes later (connections, turnarounds, scenarios,
   studies) is one operations-1.0 document (`schemas/operations/1.0.schema.json`, the door in `crates/doc`). The
   server keeps it in revisions (`operations_revision`, migration 012): immutable, numbered per project, by hash,
   each naming the timetables it changed. The door checks every revision before it is kept, in place of the
   checks by hand of before (`timetable_fault` keeps only what the schema cannot say: a list of trains, a train's
   id once in its timetable).
2. **A timetable's version is the number of the revision it was read from.** Saving one makes a new revision.
   A save on an older revision is **taken where the timetable is as it was there** — someone saved another
   timetable meanwhile — and **refused (409) where that timetable changed since**, with the timetable as it is now.
   So two people on two timetables of a project do not refuse each other, and one timetable is never saved over
   unseen.
3. **The API stays as it was** — `/api/projects/<id>/timetables`, `/api/timetables/<id>` —, reading the head and
   writing revisions; who made a timetable and who saved it last is kept beside the revisions
   (`operations_timetable`). New: `/api/projects/<id>/operations` (the head as its document, number and hash),
   `…/operations/revisions` (the history) and `…/operations/revisions/<n>` (one revision's document).
4. **The rows of before became each project's first revision** (data step `0026-operations`), each timetable under
   its id, its maker and last saver kept; a timetable the door refuses stays in its row, counted, and is read no
   more. The table `timetable` stays, unread, for a step back.
5. **The file is an operations file** (`.operations.json`) carrying the vehicles its trains run with, each a blob
   named by its hash, the formation naming it pinned to that hash. Read in, a vehicle the catalogue has under its
   key as it is is taken as it is; one it lacks or has with another content is made in the catalogue as a new
   one, beside its own, the trains naming that, and the user is told. A `rail-across-timetables` 1.0 file is still
   read.

## Why

- The capacity simulation's caches are keyed by the hash of what they were computed from (roadmap, phase 4); a
  counter per timetable names no state of a project's operations, a revision does.
- Kinds of trains, formations, connections and studies belong to all of a project's timetables; they need one
  document, not one row each.
- Refusing every save after any other one would make two planners on two timetables wait for each other; taking
  a save whose timetable is unchanged keeps the rule of 0016 — nothing is saved over unseen — without that.

## Consequences

- A train's order in its timetable is not kept: the document's collections are sets, and a timetable's trains are
  listed by departure, as the Fahrplan-Editor showed them already.
- A train's id twice in a project — a file brought in twice — is drawn anew from its timetable's on the way in.
- A field of a train or point the format does not know is dropped on the way in; the suite wrote none.

## Alternatives considered

- **Keep the rows and add the revisions beside them**: two truths of a project's timetables to keep alike.
- **A revision per timetable**: no state of the operations as a whole to hash, compare or export.
