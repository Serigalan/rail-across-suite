# 0023 — The capacity simulation is the suite's own

Status: **accepted** · 2026-11-11 · replaces [0006](0006-capacity-on-osrd.md); changes point 4 of
[0013](0013-running-times-in-rust.md)

## Decision

1. **0006 no longer holds.** The capacity simulation is not built on OSRD run as a service: there is no adapter
   to it, and the project server runs no OSRD.
2. **It is built on what the Fahrplan-Sim has**: the running times ([0013](0013-running-times-in-rust.md)) and a
   timetable's blocks, blocking times, conflicts and dispatching
   ([0016](0016-timetables-of-a-project.md), point 7). What 0013 left to OSRD — conflicts, blocking times, the
   search for a free train path — is the suite's.
3. **What computes is Rust** ([0021](0021-one-format-one-build-the-project-in-the-module.md), points 5 and 6): what
   answers while a timetable is planned runs in the module in the page, on the working copy; the many runs of a
   study run as a job in the server's crate.
4. **Its results stay derived**: caches keyed by the hashes of their inputs and the construction stage, never part
   of a revision (data model, S-R6 and case S7) — as 0006 had it.
5. **What it needs and the formats lack enters the formats**, not the simulation: the infrastructure's part in
   infra-data-1.1, the operations' part — timetables, formations, what a study assumes — in a format of its own
   beside it, operations-1.0.
6. **OSRD stays what the suite takes from and exchanges with**: the source of the running time simulation's port
   (0013), the form of the vehicles ([0014](0014-vehicles-in-osrds-form.md)), RailJSON as the Layout-Tool reads and
   writes it. The licence ([0015](0015-lgpl.md)) is unchanged.

How it is built, and in which order, is the [roadmap](../../ROADMAP.md)'s, phase 4.

## Why

- The blocking times are there, on the variant's own signalling: blocks from main signal to main signal, held
  section by section, with the overlaps (0016, point 7). 0006 itself named what OSRD would need first — DB's
  signalling systems, the alignment as designed, one infrastructure per construction stage — and each of them an
  agreement upstream or a fork.
- It answers where the planning is done: on a working copy in the browser, with no service, no import and no round
  trip — what 0013 was decided for.
- One model. Nothing is translated to RailJSON and back, so a variant and a construction stage are compared as
  they are.

## What it costs

- The suite maintains a blocking time model, a dispatcher and a search for free train paths of its own; OSRD's
  conflict detection and STDCM are not reused.
- Its results are not an established tool's. They are held to reference cases worked out by hand, and compared
  with another tool's results for a project where there are any.
- The dispatcher is simple — a train waits or takes another route —; nothing optimises the order of the trains.
- The work with the OpenRail Association narrows to the port and the exchange.

## Alternatives considered

- **OSRD as a service (0006)**: its whole stack on the server, an import of every state, and its adaptations first.
- **OSRD for the search of free train paths only**: the whole stack for one function, on a signalling model that is
  not the variant's.
