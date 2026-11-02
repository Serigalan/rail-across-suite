# 0016 — Timetables belong to a project; a train begins and ends at a station

Status: **proposed** — goes with [0013](0013-running-times-in-rust.md) and [0014](0014-vehicles-in-osrds-form.md) ·
amended 2026-11-02: one tile with the Simulation-Tool, the Fahrplan-Sim; a way a train prefers (6); a train runs
only into a free block (7); the register's stations are stations (3)

## Decision

1. **A timetable belongs to a project, not to a variant's record.** The server keeps a project's timetables in a
   table of their own (`timetable`, migration 010) under `/api/projects/<id>/timetables` and `/api/timetables/<id>`.
   Whoever sees the project sees its timetables and changes them; a template's are the admins'. A save names the
   version it changed; a save on an older one is refused with the timetable as it is now (409), as for vehicles.
2. **A train names its places by their ids.** A train is its number, kind and name, a vehicle of the catalogue
   (0014) with its traction mode and comfort, its departure, its margin and how it is spread, and its points —
   each a place by its id (`place`), a track of it or any, whether it stops there, how long, and a departure it
   waits for. A station's or halt's id is that of its point of the operational point register (R10) — drawn from
   the project and the name for a new one, so the same on every variant, and kept when the station is renamed
   (in the Layout-Tool under *Gleise zuordnen*, or in the Signalling-Tool); a buffer stop's or boundary's is its
   end mark's. The name is only what a place is shown by: a point keeps the one it was chosen under
   (`station`) to say which place a variant lacks, and a point written before places had ids is found by it,
   and takes the id once its train's way is edited. So one timetable is laid on any variant of its project:
   the Fahrplan-Editor opens a variant — this browser's working copy, else its head — and runs the trains over
   its net. A place the variant lacks is said, not dropped.
3. **A station is the layout's or the register's**: the station tracks of one name (a track's `trackType` and
   `stationName`, as the Layout-Tool groups them); a halt is a name only platforms carry. A station of the register
   of operational points ([0017](0017-stations-in-the-signalling-tool.md)) — one the Signalling-Tool made — is a
   station too where no station tracks name it: on the tracks of its stopping places and platforms, as far as they
   reach, a train stopping in the middle of its stopping place. So a station is made in one place, not twice. **A train begins and ends at a station only**
   — on one of its tracks, the one asked for or the nearest —, **or at an end of the layout**: a buffer stop or a
   boundary of the planning area, under the end mark's name (else one made of its kind, track and end). It stands
   before a buffer stop's face; it crosses a boundary at the boundary's speed, or what the line allows there,
   running in or out, as the part of its run beyond the planning area is not modelled. On the way it stops or
   passes at stations and halts, never at an end. The editor offers no other for a train's first and last point,
   and says so for a train that has one.
4. **The times come from the running time simulation** (0013), not from the user: a train's way is the shortest
   over the net through its points, never turning back; its run gives the arrival and departure at every stop and
   the time it passes every station on the way. A departure waited for at a stop moves the run on from there.
5. **The way, the stations along it and the simulation module are a package** (`@rail-across/running-times`) the
   Simulation-Tool and the Fahrplan-Editor share; the timetable's model, its runs and its API another
   (`@rail-across/timetables`). The Fahrplan-Editor is an app of its own (`apps/timetable-editor/`, `timetable/`);
   on the start page it is one module with the Simulation-Tool, the *Fahrplan-Sim*, whose tile lists the
   projects and opens a variant in either app.

6. **A train may prefer a way** from one of its points to the next: an itinerary of the variant's signalling
   ([0011](0011-signalling-domain.md)), from a stopping place of the one station to one of the other, named by its
   id on the point it starts at (`itinerary`). The run takes it where the variant has it and its run of tracks
   leads there, from the middle of the one stopping place to the middle of the other; else the train runs the
   shortest way, and the editor says so under the leg. Other ways stay open: a leg without one, or one whose
   itinerary does not lead there, runs the shortest.
7. **A train runs only into a free block.** A block is the stretch of a train's way from one main signal of the
   variant's signalling acting its way to the next, the overlap beyond the second with it; without main signals
   the whole way is one. A train holds a block from when it passes the distant signal before it (or leaves where
   it stood within that distance), less the time to set the route, until its tail has left it and it is released
   — its blocking time after Ril 405, as OSRD's spacing requirements are; two trains conflict where they hold a
   stretch of the same track at the same time. The timetable says what is done (`dispatch`): **only show the
   conflicts**, each train run for itself as OSRD does, or **let the trains wait** — given the net one after the
   other, by departure or by their kinds in an order the timetable sets, each run again waiting for a free block
   where a train before it holds one: longer at its last stop before the block (its start among them), else
   before the block's signal. The waits are the run's, not the timetable's: the trains' departures stay as they
   are written. The times a block is held beyond the running (route setting, the distant signal's distance,
   release) are the timetable's too.

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
- Blocks are held by stretches of track, not by the detection sections and routes they are set and released in:
  no sectional release, no flank protection, no routes set early. A train waiting takes the place it is given;
  nothing looks for a way round or reorders the trains, and what waiting cannot clear is said as a conflict.
- There are no connections, no periodic patterns beyond copying a train at an interval; and no exchange with
  OSRD's timetables (0006) yet.

## Alternatives considered

- **Timetables in the record (core 1.0)**: a format change for data that is not infrastructure, a copy per variant.
- **Points as track positions**: exact, but lost with every edit of the tracks and unreadable as a timetable.
- **Times typed in, running times only checked**: the classic way of timetable construction; it comes once the
  simulation can say where a typed time cannot be kept — the times from the simulation are its base.
