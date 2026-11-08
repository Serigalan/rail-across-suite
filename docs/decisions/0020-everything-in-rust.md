# 0020 — Everything in Rust: the core, the point clouds and the server, one crate set for server and desktop

Status: **proposed** · 2026-11-08 · carries on [0013](0013-running-times-in-rust.md), [0018](0018-crossover-search-in-rust.md),
[0019](0019-optimizer-in-rust.md)

## Decision

1. **Everything that computes or keeps data is written again in Rust**: the core (`packages/core`, ~25,000 lines),
   the point clouds (`packages/pointcloud`, ~3,100), timetables, vehicles and running times (~2,500) and the project
   server (`server/`, ~3,800 lines and its migrations). The modules' interfaces stay React and JavaScript: MapLibre,
   three.js and the pages are not ported.
2. **One Cargo workspace** takes in `sim/`, `crossover/` and `optimizer/` and the new crates: `ra-doc` (the core 1.0
   document, the door and its migrations, canonical JSON, hashes), `ra-geom` (elements, clothoids, UTM and NTv2, the
   one tolerance table), `ra-rules`, `ra-track`, `ra-switch`, `ra-commands`, `ra-signalling`, `ra-planpro`,
   `ra-pointcloud`, `ra-server` (axum, rusqlite), `ra-wasm` (the browser's binding) and `ra-desktop` (Tauri).
3. **What is computed twice today is computed once**: the optimizer's own geometry kernel and rule check (0019,
   point 3) go into `ra-geom` and `ra-rules`.
4. **The browser runs the same crates as WebAssembly behind the core's JavaScript face.** The import paths
   (`@rail-across/core/track`, …) stay, so the modules do not change when a module of the core changes language.
   The module is loaded once before a page renders, so the core's calls stay synchronous.
5. **Server and local package are one crate.** `ra-server` runs behind Caddy as one program in place of the Node
   service; the desktop app calls the same router in its own process through its own URI scheme, without a port and
   without Node. The local mode of the branch `desktop-standalone` (`serve --local`, no user management) becomes a
   switch of the crate.
6. **The API, the database and the accounts stay.** The same routes and JSON shapes, the same SQL migrations on the
   same file, argon2id hashes read as they are (PHC strings), the same session cookie: the server is exchanged under
   running modules and signed-in users.
7. **The tolerances come as one table** (`packages/core/src/geometry/tolerances.js`, then `ra-geom`): the same place
   is 1 µm (`NODE_TOL`), positions are kept to 0.1 µm. A solver's stopping criterion, an import's matching of foreign
   data, a rule's own distance and anything in another unit keep their own values.

Done module by module, each step shipped on its own, the JavaScript removed when its port has taken over:

| Step | What | Done when |
|---|---|---|
| 0 | One workspace and CI job, the WebAssembly pipeline and the core's face, recording the JavaScript core's calls for replay; a Tauri test of the point cloud and the map on macOS (WKWebView) and Linux (WebKitGTK) | the face loads a module; the test decides Tauri or Electron for step 9 |
| 1 | `ra-doc`: types from `schemas/`, `parseProjectsPayload` and the migration from oltjson-1.1, `dehydrateProjects`, canonical JSON and hashes | `schemas/vectors/` pass, every fixture reads and writes back byte for byte |
| 2 | `ra-geom`: `geometry/` and the optimizer's kernel, UTM and the NTv2 grids | replay within 10⁻⁹ m |
| 3 | `ra-rules`: `rules/` and the optimizer's catalogue and check | `packages/core/constraints/tests/` pass |
| 4 | track, switch, topology, cross section, survey, schematic (~13,000 lines) | replay, the chain invariants |
| 5 | `commands/`, `merge/`: the editing, measured for speed — coordinates as `Float64Array`, one call per drag | replay; the document kept in WebAssembly memory if the measurement asks for it |
| 6 | signalling, PlanPro | replay |
| 7 | `ra-pointcloud` with `las`/`laz`: native on the server, WebAssembly in the browser's worker | the point cloud tests |
| 8 | `ra-server`: the server's tests as HTTP tests run against both servers; terrain with `tiff`, `zip`, `image-webp`; jobs as threads; `mdb-export` called as before | both servers answer the same; then Caddy is switched, the Node unit kept for going back until the next migration |
| 9 | `ra-desktop`: Tauri with `ra-server` in process, the start screen, settings and export folder of `desktop-standalone`, `mdb-export` bundled | the app runs without Node |
| 10 | the JavaScript implementations and the server's Node dependencies go; whether `core-py` is still wanted | — |

The JavaScript is the measure until its port has taken over. Results are compared within 10⁻⁹ m, since `sin`,
`cos` and `atan2` may differ in the last digit between V8 and Rust's libm; canonical JSON and hashes are compared byte
for byte, numbers written as JavaScript writes them (RFC 8785; `ryu-js`). A module being ported is frozen, or a
change goes into both.

## Why

- The server and the local package are the same code; what is checked in the browser is checked on the server by the
  same crate, so the door `server/src/checks.js` goes through stays the browser's own.
- The desktop app needs no Node beside it: one process, no port, no child process to start and stop.
- The point clouds, the optimizer and the editing get Rust's speed; the types catch what JavaScript lets through.
- The repository already goes this way with the simulation, the crossover search and the optimizer.

Not a reason: precision. JavaScript's numbers and Rust's `f64` are the same IEEE 754 doubles; the precision comes from
the algorithms and the tolerances (point 7).

## What it costs

- Some 40,000–50,000 lines of Rust over months; meanwhile modules are held twice and the core's features come slower.
- A boundary between JavaScript and WebAssembly: data converted per call unless calls are bundled, a module loaded
  before the first page, 1–3 MB more to download, Rust harder to debug in the browser than JavaScript.
- WebKit for the desktop app until step 0's test has shown the point clouds and the map run there.
- Rust for everyone who changes a rule or the geometry.

## Alternatives considered

- **Rust for the desktop server only, Node in production**: two servers to keep alike route by route; worse than
  either alone.
- **Electron or Tauri with the Node server beside it** (branch `desktop-standalone`): quicker to the local package,
  but the core stays JavaScript and Node is bundled.
- **A JavaScript engine in Rust (QuickJS, V8) running the core**: Rust in name, JavaScript at run time, the point
  clouds slower under QuickJS.
- **Skipping the server's checks in the local package**: no trust boundary there, but only possible if the Rust
  server runs nowhere else.
