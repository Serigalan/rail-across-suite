# 0014 — The suite's vehicles, in OSRD's form, beside the projects

Status: **proposed** — goes with [0013](0013-running-times-in-rust.md)

## Decision

1. **A vehicle is OSRD's RailJSON rolling stock** (`railjson_version` 3.4): its physics — mass, length, top
   speed, inertia coefficient, braking deceleration, Davis coefficients in N, N/(m/s), N/(m/s)² —, its tractive
   effort curves per traction mode and comfort, its metadata and categories, and whatever else OSRD's format
   holds. The suite keeps it as it is; what the editor does not edit (electrical profiles, power restrictions,
   supported signalling systems, energy sources) goes along unchanged. Import and export are the file itself.
2. **The vehicles are a catalogue of the suite, not part of a project's record.** The server keeps them in a
   table of their own (`vehicle`, migration 009) under `/api/vehicles`. Everyone signed in sees them all and adds
   their own; changing and deleting one is its creator's and the admins' — the others copy it. A save names the
   version it changed; a save on an older one is refused with the vehicle as it is now (409), as a check-in on a
   stale base is.
3. **OSRD's eleven example vehicles are the built-in ones.** They lie unchanged in `packages/vehicles/data/osrd/`
   and go into the build of every module that uses the catalogue. They are read and copied, not changed. Their Davis coefficients are per kilogram — OSRD reads them as newtons for the whole
   train, which makes the resistance vanish —; the tool multiplies them by the mass when it takes them in, and
   says so. The same goes for any imported file whose coefficients are all below one.
4. **The Fahrzeug-Editor is a module of its own** (`apps/vehicle-editor/`, `vehicles/`), opened by its tile on the
   start page without a project — a panel with the catalogue, a page with the vehicle, `?vehicle=<key>` one of
   them. Its fields and units follow OSRD's rolling stock editor; its look is the suite's (`@rail-across/ui`). The
   model, the built-in vehicles and the catalogue on the server are a package (`@rail-across/vehicles`) the
   Fahrplan-Sim shares; it chooses a train's vehicle from the catalogue and shows it in the editor.
5. **The simulation runs a vehicle in a traction mode and comfort the user chooses**, with the curve OSRD would
   choose for them without electrical profiles and power restrictions. The net does not say yet where there is
   which overhead line.

## Why

- OSRD's format already holds what a running time needs, and OSRD's editor shows how to edit it. Keeping it as it
  is lets a vehicle go to OSRD (0006) and come from it without a mapping that loses anything.
- A vehicle is not infrastructure: the same train runs on every project's net. Kept beside the projects, it is
  made once and used everywhere; kept in a record, it would be copied into each and drift.
- Core 1.0 (0009) stays as it is: the exchange format of the infrastructure does not grow a rolling stock part
  before the suite knows what it needs of one.

## What it costs

- Vehicles have no history and no variants; a change is for everyone at once. The version number only keeps two
  saves from overwriting each other unseen.
- They are not in a project's export: a project that names a vehicle (a timetable, later) needs it from the
  catalogue.
- OSRD's format carries French notions (loading gauges, BAL/BAPR/TVM signalling systems, power classes) the suite
  shows but does not use.
- OSRD's example data has its flaws (the per-kilogram resistance, the heavy freight train's `A`); the suite works
  around them rather than changing OSRD's files.

## Alternatives considered

- **Vehicles in the project record (core 1.0)**: a format change for data that is not infrastructure, and a copy
  per project.
- **Vehicles in the browser only**: nothing shared, gone with the browser's storage.
- **OSRD's database as the catalogue**: needs OSRD running for every edit and every run, against 0013's
  in-browser simulation.
- **A format of our own**: a mapping to and from OSRD, and a second editor's worth of design.
