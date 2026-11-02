# 0016 — Timetables belong to a project; a train begins and ends at a station

Status: **proposed** — goes with [0013](0013-running-times-in-rust.md) and [0014](0014-vehicles-in-osrds-form.md) ·
amended 2026-11-02: one tile with the Simulation-Tool, the Fahrplan-Sim

## Decision

1. **A timetable belongs to a project, not to a variant's record.** The server keeps a project's timetables in a
   table of their own (`timetable`, migration 010) under `/api/projects/<id>/timetables` and `/api/timetables/<id>`.
   Whoever sees the project sees its timetables and changes them; a template's are the admins'. A save names the
   version it changed; a save on an older one is refused with the timetable as it is now (409), as for vehicles.
2. **A train names its stations by their names.** A train is its number, kind and name, a vehicle of the catalogue
   (0014) with its traction mode and comfort, its departure, its margin and how it is spread, and its points —
   each a station or halt by name, a track of it or any, whether it stops there, how long, and a departure it
   waits for. So one timetable is laid on any variant of its project: the Fahrplan-Editor opens a variant —
   this browser's working copy, else its head — and runs the trains over its net. A station the variant lacks is
   said, not dropped.
3. **A station is the layout's**: the station tracks of one name (a track's `trackType` and `stationName`, as the
   Layout-Tool groups them); a halt is a name only platforms carry. **A train begins and ends at a station only**
   — on one of its tracks, the one asked for or the nearest —; on the way it stops or passes at stations and
   halts. The editor offers no other for a train's first and last point, and says so for a train that has one.
4. **The times come from the running time simulation** (0013), not from the user: a train's way is the shortest
   over the net through its points, never turning back; its run gives the arrival and departure at every stop and
   the time it passes every station on the way. A departure waited for at a stop moves the run on from there.
5. **The way, the stations along it and the simulation module are a package** (`@rail-across/running-times`) the
   Simulation-Tool and the Fahrplan-Editor share; the timetable's model, its runs and its API another
   (`@rail-across/timetables`). The Fahrplan-Editor is an app of its own (`apps/timetable-editor/`, `timetable/`);
   on the start page it is one module with the Simulation-Tool, the *Fahrplan-Sim*, whose tile lists the
   projects and opens a variant in either app.

## Why

- A timetable is planned against an infrastructure that is itself being planned: one timetable over two variants
  is how they are compared. Kept in a record, it would be copied into every variant and drift.
- Names are what a timetable speaks in (Betriebsstellen), and what survives a variant's tracks being split and
  joined; ids of tracks would not.
- A train that begins on the open line has no place to be prepared, turned or stabled. Stations are where trains
  begin and end (EBO §4: a station has at least one switch; a halt has none) — the layout already tells them apart
  by station tracks.

## What it costs

- Timetables have no history and no variants of their own; the version number only keeps two saves from
  overwriting each other unseen.
- A station renamed in the layout leaves the trains that name it pointing at nothing until they are changed.
- There is no conflict detection yet (blocking times, headways), no connections, no periodic patterns beyond
  copying a train at an interval; and no exchange with OSRD's timetables (0006) yet.

## Alternatives considered

- **Timetables in the record (core 1.0)**: a format change for data that is not infrastructure, a copy per variant.
- **Points as track positions**: exact, but lost with every edit of the tracks and unreadable as a timetable.
- **Times typed in, running times only checked**: the classic way of timetable construction; it comes once the
  simulation can say where a typed time cannot be kept — the times from the simulation are its base.
