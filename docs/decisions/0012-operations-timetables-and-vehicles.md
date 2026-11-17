# 0012 — The operation: timetables and vehicles of a project, in revisions

Status: **accepted** · as of 2026-11-17 · what it computes: [assumptions of the simulation](../simulation/simulation-assumptions.md)

## Decision

1. **A project's operation is one operations-1.0 document** (0006): the domain `timetables` — timetables, trains,
   kinds of trains, formations, connections, turnarounds, delay scenarios, studies, construction scenarios — and
   the domain `vehicles`. The project server keeps it in **revisions per project** (`operations_revision`):
   immutable, numbered, by hash, each naming the timetables it changed; the door checks every revision before it is
   kept. A file of its own (`.operations.json`) or the part of a rail-across-1.0 file carries the vehicles its
   trains run with.
2. **A timetable's version is the number of the revision it was read from.** A save on an older revision is taken
   where that timetable is as it was there — someone saved another timetable meanwhile — and refused (409) where it
   changed since, with the timetable as it is now. The API: `/api/projects/<id>/timetables`, `/api/timetables/<id>`,
   `/api/projects/<id>/operations` with its revisions.
3. **A timetable belongs to a project, not to a variant**, and is laid on any variant of it: the Fahrplan-Editor
   opens a variant — this browser's working copy, else its head — and runs the trains over its net. A place the
   variant lacks is said, not dropped.
4. **A train** is its number, kind and name, a vehicle with its traction mode and comfort, its departure, its margin
   and how it is spread, and its points. **A point names an operational point or a track end by id**
   (`operationalPointId`, `endMarkId`), a track of it or any, whether it stops there and how long, a departure it
   waits for, a way it prefers to the next point (an itinerary of the signalling, 0009), and whether it reverses
   there, standing its turnaround time (300 s where none is given).
5. **A train begins and ends at a station or at a track end** — a buffer stop or a boundary of the planning area;
   on the way it stops or passes at stations and halts. **It stops only at a stopping place**: a passenger train
   only at one at a platform, a train without passengers (Gz, Lz, Dz, GC, KLV, EK, DGS) at any; 10 m before its end
   in the direction of travel.
6. **The times come from the running time simulation** (0011): a train's way is the shortest over the net through
   its points, reversing only where a point asks for it; its run gives the arrival and departure at every stop and
   the time it passes every station.
7. **A train runs only into a free block.** A block runs from one main signal acting its way to the next, its
   overlap with it, held section by section where there is track detection and released section by section; its
   blocking time after Ril 405, as OSRD's spacing requirements are. The timetable says what is done where trains
   meet: **show the conflicts**, each train run for itself, or **let the trains wait** — given the net one after
   the other, as an interlocking sets routes; a train held takes another free route from the same signal, else
   waits at its last stop before the block or before the signal. The trains' departures stay as they are written.
8. **A vehicle is OSRD's RailJSON rolling stock**, kept in the object (`vehicles`: its `key` and its
   `rollingStock`): physics, tractive effort curves per traction mode and comfort, and whatever else OSRD's format
   holds, passed on unchanged. Import and export are the file itself. OSRD's eleven example vehicles are built in,
   unchanged; their Davis coefficients per kilogram are multiplied by the mass where they are taken in, as for any
   file whose coefficients are all below one. The **Fahrzeug-Editor** (`apps/vehicle-editor`) edits them, its
   fields and units after OSRD's rolling stock editor.
9. **The packages**: the way, the stations along it and the simulation module (`@rail-across/running-times`), the
   timetable's model, runs and API (`@rail-across/timetables`), the vehicles (`@rail-across/vehicles`). The
   Fahrplan-Editor and the Simulation-Tool are apps of their own, one tile on the start page, the *Fahrplan-Sim*.

## Decided, not built yet

- **Vehicles per project.** Each project holds its vehicles in its operation; a timetable's trains name those.
  The suite's catalogue (the server's table `vehicle`, `/api/vehicles`, the built-in vehicles) is a store of
  templates: a vehicle is copied from it into a project and changed there, and a change in the catalogue changes no
  project. Today the catalogue is shared by every project, and a file only pins the vehicles it carries.
- **Target times.** A train's points may state the times it is to keep — arrival and departure as timetabled. The
  simulation runs them as given arrival times (OSRD's `final` run) and says where and by how much a time cannot
  be kept.
- **Turnarounds from one train into another** (Umläufe): a train continues as another at its end, after a
  turnaround time; the dispatching holds the second until the first has arrived and stood it. The format holds
  turnarounds already (`turnarounds`); they are not computed.

## Why

- A timetable is planned against an infrastructure that is itself being planned: one timetable over two variants is
  how they are compared. Kept in a variant, it would be copied into every one and drift.
- Kinds of trains, formations, connections and studies belong to all of a project's timetables; they need one
  document, and a revision names one state of it, which the capacity's caches are keyed by.
- Ids of operational points survive a station's renaming and a variant's tracks being split and joined; names and
  track positions would not.
- A train that begins on the open line has no place to be prepared, turned or stabled (EBO §4).
- OSRD's format already holds what a running time needs, and OSRD's editor shows how to edit it.
- A project's vehicles belong to its planning as its trains do: a changed vehicle in a shared catalogue would
  change every project's running times unseen.

## What it costs

- A train's order in its timetable is not kept: collections are sets, and a timetable's trains are listed by
  departure.
- OSRD's format carries French notions (loading gauges, BAL/BAPR/TVM, power classes) the suite shows but does not
  use.
- The dispatching does not optimise: a train waits or takes another route; nothing reorders the trains.

## Open

- **Connections**, kept by the dispatching.
- **Periodic patterns** as objects of their own, beyond copying a train at an interval.
- **Delay scenarios and the stochastic runs of a study** (`delayScenarios`, `runs`, `seed`): stored, not computed.
