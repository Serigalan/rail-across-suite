# 0005 — What computes is Rust, what shows is JavaScript; the open project lives in the core's module

Status: **accepted** · as of 2026-11-17

## Decision

1. **What computes or keeps data is Rust**, in one Cargo workspace (`Cargo.toml` at the root, each crate
   `rail-across-<name>`): the core — the document and its door (`crates/doc`), geometry with UTM and NTv2
   (`geom`), rules, tracks, switches, the editing commands and the merge, signalling, PlanPro —, the point clouds,
   the capacity, the server and the desktop app, and beside them the running time simulation (`sim/`), the
   crossover search (`crossover/`) and the optimizer (`optimizer/`). **What shows or takes input is JavaScript**:
   React, MapLibre, three.js, workers, OPFS, IndexedDB.
2. **The browser runs the same crates as WebAssembly** behind the core's JavaScript face. The import paths
   (`@rail-across/core/track`, …) stay, so a module of the core can change language without the pages changing.
   The core's module is loaded once before a page renders, so its calls stay synchronous.
3. **The open project lives in the core's module.** The page opens a session on a document, sends it commands
   and asks it for views; the store writes only what changed, and an operation handed the open project's tracks or
   switches is lent them. No record is carried across per call.
4. **One code base per thing.** Where it is not clear which side a thing belongs to, a measurement decides — in
   Chrome **and** in WebKit, which the desktop app runs on macOS. A thing is held twice only while it is ported,
   and its JavaScript goes in the same step. Measured, and so JavaScript: the fine-grained geometry (a coordinate
   conversion 2 µs in JavaScript, 12.7 µs through the module, and the map converts point by point), the track
   module's small queries asked point by point, the signalling catalogue's lookups, the rule check of a track, the
   schematic strip.
5. **What was computed twice is computed once**: the optimizer's geometry kernel and rule check are the crates
   `geom` and `rules`. The rule catalogue is the one the app bundles, handed to the module with each run; shared
   vectors (`packages/core/constraints/tests/`) hold the JavaScript check and the Rust one to the same answers.
6. **The searches run in the browser, in a worker**, with no service: the extended switch connection (one to
   three elements between two turnouts of any form, every sequence tried, listed nearest first), optimizing a
   track, the splice with a spacing to a neighbour, reconnecting elements, and the alignment and gradient fit from
   axis points. The simple switch connection — two turnouts and one straight or arc between their branches — has a
   dialog of its own with a slider. Fetching and converting the Länder's terrain (DGM1) and the Access import
   (`mdb-export`) are the server's.
7. **The tolerances are one table** (`crates/geom`): the same place is 1 µm, positions are kept to 0.1 µm. A
   solver's stopping criterion, an import's matching of foreign data, a rule's own distance keep their own values.
   A port is held to its JavaScript within 4·10⁻⁹ m or four units in the last place; canonical JSON and hashes byte
   for byte.

## Why

- The server and the desktop app are the same code; what is checked in the browser is checked on the server by
  the same crate.
- The desktop app needs no Node beside it: one process, no port.
- The point clouds, the searches and the editing get Rust's speed; the types catch what JavaScript lets through.
- Kept in the module, the project is not carried across per call — the cost that kept the hot paths in
  JavaScript (184–237 ms for a drag step of the element table on the PEK Bestand across JSON, 10–15 ms held).

Not a reason: precision. JavaScript's numbers and Rust's `f64` are the same IEEE 754 doubles; the precision comes
from the algorithms and the tolerances.

## What it costs

- A boundary between JavaScript and WebAssembly, a module loaded before the first page, 1–3 MB more to download,
  Rust harder to debug in the browser.
- The rule check twice, in JavaScript and in Rust, held together by the shared vectors.
- A search's numbers are not scipy's to the last digit; its tests hold properties (never worse than the start,
  every rule kept, ends fixed) where they cannot hold numbers.
- Rust for everyone who changes a rule or the geometry.

## Open

- **The page's views in the document's terms.** The page's store still holds a record — the view of what the
  module holds —, not views in the document's words.
- **The rest of the JavaScript core.** The rules module has nine functions without Rust; it moves as a whole or
  not at all. Measured again where data says otherwise — an MDB import of a large database is the first
  candidate, for the server's crate.

## Alternatives considered

- **Rust for the desktop server only, Node in production**: two servers to keep alike route by route.
- **Electron or Tauri with the Node server beside it**: the core stays JavaScript and Node is bundled.
- **A JavaScript engine in Rust running the core**: Rust in name, JavaScript at run time.
- **The Python optimizer service**: a queue, a timeout, a second service to run, and no optimizer without a
  network.
