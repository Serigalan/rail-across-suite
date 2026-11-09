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
| A | the local build goes | one build; CI builds the desktop app in its place |
| B | infra-data-1.0 the only format at every boundary | the server's revisions migrated, working copies on open, oltjson refused, its reader gone |
| C | the session in the module: the project held as the document, commands in, views out; the Layout-Tool rebuilt onto it | the record gone from the page; no slower than before on the PEK data set in Chrome and WebKit |
| D | the core's remaining JavaScript as queries of the session: geometry, track, switch, cross section, rule check, signalling catalogue, element table, station register | the core package is the bridge |
| E | the Layout-Tool's own computing (`src/core/utils`: exchange, plan, elevation, km) and the packages running times, timetables and vehicles | in Rust where it is not slower |

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
