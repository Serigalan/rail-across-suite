# 0021 — One format, one build, the project in the core's module

Status: **accepted** · 2026-11-09 · carries on [0020](0020-everything-in-rust.md), replaces the record of
[0009](0009-core-1-0-in-the-data-model.md) as the app's model

## Decision

1. **There is one build of the Layout-Tool: the one with the project server.** It runs behind Caddy on the server,
   or bundled with the server in the desktop app (Tauri, `crates/desktop`). The local build that ran in the browser
   alone (Paket L: `main.local.jsx`, `vite --mode standalone`, IndexedDB `olt-local`, the release zip) goes; working
   without a network is the desktop app's.
2. **The project format is called infra-data-1.0** (`format: "infra-data"`, `version: "1.0"`), what was "the core 1.0
   document" (envelope `rail-across`). The domains (`core`, `layout`, `signalling`), their schemas and the crates keep
   their names. A document of the old envelope is taken once, as the server migrates its revisions; after that the
   door takes infra-data-1.0 only.
3. **oltjson-1.1 goes completely.** No file, no revision, no working copy and no message is in it any more: the
   server migrates its revisions once (schema version 2 and 3 into 4), the browser its working copies when it opens
   them, and an oltjson file is refused (`unsupported_version`). The migration chain from oltjson goes with it.
4. **The open project lives in the core's module.** The page opens a session on an infra-data document, sends it
   commands and asks it for views (what the map, the table and the dialogs show); it holds no record of its own.
   That ends the JSON carried across per call (0020, step 10: 12 µs at the least) that kept the hot paths in
   JavaScript.
5. **One code base per thing.** What computes or keeps data is Rust; what shows or takes input is JavaScript
   (React, MapLibre, three.js, workers, OPFS, IndexedDB). Where it is not clear, a measurement decides — in Chrome
   **and** in WebKit, which the desktop app runs on macOS. A module is held twice only while it is ported, and its
   JavaScript goes in the same step.
6. **Work done in one go runs in the server's crate**, native on the server and native in the desktop app: the
   MDB import, the plan's PDF, the point cloud jobs. What answers while someone edits runs in the module in the page,
   the same code in the browser and in the window. With the local build gone there is always a server — remote or
   in the process.

## Phases

| Phase | What | Done when |
|---|---|---|
| A | the local build goes | one build; CI builds the desktop app in its place (done) |
| B | infra-data-1.0 the only format at every boundary | the server's revisions migrated, working copies on open, oltjson refused, its reader gone (done: data step `0021-infra-data`, schema version 4; `@rail-across/working-copy` keeps documents; the API, files and track files take documents only; the door holds a switch's branch to its `switchId`, which the schema cannot say) |
| C | the session in the module: the project held there, written by what changed, lent to every operation handed it | no slower than before on the PEK data set in Chrome and WebKit (done: `holdProject`, the store writes only what changed — a track the module answered named, not sent back —; an operation handed the open project's tracks or switches is lent them; the working copy's document is made from what the module holds. A drag step of the element table on the PEK Bestand: 184–237 ms across JSON before, 10–15 ms held, in Chrome and WebKitGTK alike — see `packages/core/docs/native.md`) |
| D | the core's remaining JavaScript as queries of the session: geometry, track, switch, cross section, rule check, signalling catalogue, element table, station register; the display geometry worked out in the module; the page's views in the document's terms | the core package is the bridge; the record gone from the page (done as far as it measures: the track and switch modules, the cross section, the element table, the station register and loading a record are the module's, their JavaScript gone and their vectors frozen; a store write on the PEK Bestand 1–2 ms, opening 1.2 s against 1.0 s before. Stays JavaScript, measured: the track module's small queries asked point by point (a point or a height at a station, a switch's ports, a label — through the module the plan export of the PEK Bestand took 4.5 s, in JavaScript 0.35 s; held to the crate's vectors), the fine-grained geometry — a coordinate conversion 2 µs here, 12.7 µs through the module, and the map converts point by point —, the signalling catalogue's lookups, the rules module — nine of its functions have no Rust yet, so it cannot go whole and half of it moved would only be slower —, the schematic strip. Open: the page's views in the document's terms; the record still is what the page's store holds, now as the view of what the module holds) |
| E | the Layout-Tool's own computing (`src/core/utils`: exchange, plan, elevation, km) and the packages running times, timetables and vehicles | in Rust where it is not slower (assessed and measured: none of them is held twice — each is JavaScript only —, so point 5 asks only whether Rust would be faster where it is felt. On the PEK Bestand: the plan export 0.35 s, the OSRD export 25 ms, the VermESN export 128 ms, a store write with the heights worked out again 1–2 ms; the kilometrage is a hover's arithmetic and tile reading; the running times' simulation is Rust already (`sim/`), what stays is the request built from the net; vehicles and timetables are reading, writing and lists. Nothing moves on that: a port would be thousands of lines for no time a user notices. Measured again where data says otherwise — an MDB import of a large database, which this machine has none of, is the first candidate, for the server's crate) |

## Why

- Two builds kept every part of the app split in two (`src/core` never importing `src/server`) and a second store
  beside the working copies; the desktop app does what the local build did, with the server's checks and jobs.
- A format the app keeps beside the one the server keeps is a conversion on every save and a migration chain to
  carry; one format is one door.
- The measurements of 0020 showed the hot paths lose in Rust only by the record carried across per call. Kept in the
  module, they do not carry it.

## What it costs

- The Layout-Tool's components are rebuilt onto views; that is most of phase C.
- Files of oltjson-1.1 no longer open; a project has to be exported again from a server that migrated it.
- No more use of the Layout-Tool without either a server or the desktop app installed.
