# 0013 — Capacity: the suite's own simulation, with conflict areas and binding elements

Status: **accepted** · as of 2026-11-17 · what it computes: [assumptions of the simulation](../simulation/simulation-assumptions.md)

## Decision

1. **The capacity simulation is the suite's own**, built on what the Fahrplan-Sim has: the running times (0011) and
   a timetable's blocks, blocking times, conflicts and dispatching (0012). It is not built on OSRD run as a
   service; the project server runs no OSRD.
2. **Capacity after UIC 406** (the Kapazitäts-Sim): a stretch runs from node to node along the trains' ways; the
   trains of a period are compressed as early behind each other as their blocking times let them; the occupancy
   time and rate are set against the recommended bounds; the minimum headways are summed up by kinds of trains;
   every track and section of a station has its share of the period. It is computed on a variant at a state of the
   project (0007); another variant or state can be laid beside it.
3. **Where the trains meet, on a plan.** For a stretch compressed, the Kapazitäts-Sim shows on a schematic plan of
   the variant (the core's schematic strip, as the Signalling-Tool draws it):
   - the **conflict areas**: what two or more trains of the period both hold — a track section, else a stretch of
     track no section covers —, coloured by how many trains hold it;
   - the **binding elements** (Belegungselemente): for each two trains of the critical chain, the element where the
     second's blocking time touches the first's — the one that sets the minimum headway —, with the two trains, the
     block and the moment one releases it and the other takes it. They are listed beside the plan and highlighted
     on it.
4. **What computes is Rust** (`crates/capacity`, 0005): what answers while a timetable is planned runs in the module
   in the page, on the working copy.
5. **Its results stay derived**: computed from their inputs and never part of a revision; where they are kept, by
   the hashes of their inputs and the state of the project.
6. **What it needs and the formats lack enters the formats**, not the simulation.
7. **OSRD stays what the suite takes from and exchanges with**: the source of the running time simulation's port,
   the form of the vehicles, RailJSON as the Layout-Tool reads and writes it.

## Decided, not built yet

- **The search for a free train path**: a train laid into a timetable where its blocking times find room, as OSRD's
  STDCM does, on the variant's own signalling.
- **Studies of many runs as a job on the server**: a study's runs — variants, states, periods, timetables compared
  — computed in the server's crate (0003), their results kept by the hashes of their inputs.

The order is the [roadmap](../../ROADMAP.md)'s, phase 4.

## Why

- The blocking times are there, on the variant's own signalling, held section by section with the overlaps. What
  OSRD would have needed first — DB's signalling systems, the alignment as designed, one infrastructure per state
  of the project — was each an agreement upstream or a fork.
- It answers where the planning is done: on a working copy in the browser, with no service, no import, no round
  trip.
- One model: nothing is translated to RailJSON and back, so variants and states are compared as they are.
- A planner asks not only how full a stretch is but where on the net the trains get in each other's way, and which
  element to change.

## What it costs

- The suite maintains a blocking time model, a dispatcher and a compression of its own; OSRD's conflict detection
  and STDCM are not reused.
- Its results are not an established tool's. They are held to reference cases worked out by hand
  (`crates/capacity/tests/reference.rs`), and compared with another tool's results for a project where there are
  any.

## Alternatives considered

- **OSRD as a service**: its whole stack on the server, an import of every state, and its adaptations first.
- **OSRD for the search of free train paths only**: the whole stack for one function, on a signalling model that is
  not the variant's.
