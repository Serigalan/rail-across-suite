# native — the core's Rust crates as one WebAssembly module

`packages/core/src/native/` · depends on nothing else in the core

Decision 0020 moves the core to Rust module by module. The crates (`crates/` of the repository) are built
as one WebAssembly module, `crates/core-wasm`, and this module is the face the JavaScript calls it
through: a request `{ op, input }` as JSON in, `{ ok }` or `{ error, detail }` out — the protocol the
simulation and the optimizer already use, with no bindings generator.

The module is loaded **once**, and every call after that is synchronous, as the core's calls always were:

- in the browser, a page awaits `loadNative(url)` before it renders;
- in Node — the server, the tests — `loadNativeFile()` (`./node.js`) reads `packages/core/wasm/` synchronously.

Where it is not loaded, a function that has moved stays with its JavaScript (`nativeReady()`), so the core
answers the same either way while the port goes on; the tests hold both to the same answers.

## Files

| File | What |
|---|---|
| `index.js` | `loadNative`, `useNativeModule`, `callNative`, `nativeReady`, `unloadNative`, `NativeError` |
| `node.js` | `loadNativeFile` — the module from its file, synchronously (Node only) |

`npm run wasm` (`scripts/wasm.mjs`) builds the module with cargo and puts it in `packages/core/wasm/`
(not in the repository); `RA_CORE_WASM` names a module file to take instead.

## What has moved

| Step | Crate | Functions the face hands to it |
|---|---|---|
| 1 | `crates/doc` (with `crates/js`, the JavaScript meaning of JSON values, and the first part of `crates/geom`) | `checkDocument`, `toDocument`, `fromDocument`, `manifestOf`, `documentHash` (document); `parseProjectsPayload`, `projectDocument`, `dehydrateProjects`, `parseTracksPayload` (project) |
| 2 | `crates/geom`: planes and the proj4js port (extended transverse Mercator, Cassini-Soldner, Helmert, NTv2 grids — a grid the page reads is handed to the module), transitions, element values, the factory, display geometry, parallels, plane changes, chain checks and closing, cants | none yet: the map calls these tens of thousands of times a frame, one at a time across JSON would be slower than proj4js. They serve the crates that call them (commands, the server) and reach the page through the coarser operations of the later steps. |
| 3 | `crates/rules`: the catalogue's expression language, the catalogue, the check (`trassierungCheck`), the limits a run is held to — taken over from the optimizer, which now uses them from here —, and the cant, speed, element length and field rules | `ruleMaxSpeed` and so `maxSpeeds` (235 ms against 388 ms for all 113 elements of a PEK track). `checkTrack` stays here: it answers with every value every rule read, and carrying those across JSON costs more than evaluating them (48 ms against 6 ms). |
| 4 | `crates/track` (the track module, the cross section, the topology graph, its lanes and label placement), `crates/survey` (measured and reference axes, shift values, the fit's bookkeeping), `crates/switch` (the catalogue and its tables, routes and chains, crossings, the symbol, sleepers and the turnout's plane, coupled cants, placement, connecting to an open end, switch connections, deletion, the detail drawing) and `crates/project` (`validateProject`) | none yet: the pages call these in small pieces; they serve the commands (step 5) and the server (step 8). Replayed over 6,400 calls; `validateProject`, `rebuildSwitchSymbol`, `planSwitchDeletion` and `switchCouplings` are also held to the JavaScript on the PEK data at run time (`src/switch/native.test.js`), where Rust validates in half the time. |
| 5 | `crates/commands`: what the dialogs commit (tracks, element roles and the table, geometry edits, the S-curve, the switch dialogs, connecting to an open end and its remedies, splicing, reconnecting, the extended switch connection's request and adoption, editing a connection, the optimizer's apply) and `merge/` (diff, the three-way merge, carrying references and the domains' locations) | none yet. Measured on the PEK Bestand (113 tracks): `planElementChange` across JSON with the whole project 114 ms against 5–7 ms in JavaScript — the project crosses the boundary on every call. Held in the module (`session.open`, `session.write`, `session.planElementChange`, copied on write) one drag step is 4.3 ms and only the tracks it touched come back; `solveConnection` 1.4 ms against 1.3, `mergeProject` 2.5 s against 3.9 s. Replayed over 7,000 calls; the held edit and the merge are also held to the JavaScript on the PEK data (`src/commands/native.test.js`). |
| 6 | `crates/signalling` (the catalogue of DB Ril 301, the check, overlaps, the speed a turnout allows a run, the track sections' extent) and `crates/planpro` (the XML reader, PlanPro's enumerations, a plan as a record, its signalling placed on another net) | none yet: the page checks a station in 0.3 ms in JavaScript against 0.7 ms across JSON, and reads the test plan in 2.3 ms against 2.0 — no gain worth the boundary; they serve the server (step 8). Replayed over 4,200 calls: the check on the Ks station of the tests and every variation the tests make of it, the catalogue's lookups, the XML reader on good and broken text, and the plan of the import's tests with variations (curved geometry, a node that is no turnout, every object the import reads and those it cannot place). |
| 7 | `crates/pointcloud` (LAS, LAZ with `laz`, E57, the tiles with `flate2`, the import into five levels, plane mapping, registration, slices, the clearance check and scan, rail detection and tracing) — not part of this module: it serves the server, and replays its vectors itself (`crates/pointcloud/tests/replay.rs`) | none: the browser's worker keeps the JavaScript. Replayed over 137 calls: files made from a description on both sides and the package's E57 fixtures, read and imported and held by sums over every point and segment (zlib's bytes differ from fflate's, what they hold does not); the track scene's section, clearance scan and trace. On the real LAZ sample (49 million points, format 6) the first million come out of laz-rs as out of laz-perf (`packages/pointcloud/src/native.test.js`), ten million in 3.5 s against 8.8 s. Not to the digit: the registration's tilts on points along a straight line, which the pairs hardly determine. |
| 8 | `crates/server` (the project server: users and sessions, projects, variants and revisions checked by the core's crates — `checkRecord` with `validateProject` and `checkSignalling` —, vehicles, timetables, point clouds with uploads, tiles, share links and jobs, the DGM1, the MDB import) — not part of this module | none: a server. The server's own tests run against it over HTTP (`RA_SERVER_IMPL=rust`). |

The crates are held to the JavaScript three ways: the shared vectors (`schemas/vectors/`), the calls the
JavaScript records (`npm run replay` → `crates/*/tests/vectors/`, replayed by `crates/core-wasm/tests/replay.rs`),
and the core's own tests, which run on the module where it is built (and with `RA_NATIVE=0` on the JavaScript).

Not ported, on purpose: the schematic strip (`schematic/`) takes a JavaScript callback (`kmOf`) and answers
with a structure of closures and back references — drawing code like the map's, it stays in the page. The
functions of step 4 that search with a callback (`connectionRoots`, `solveEndChain`, `validIntervals`,
`solveMidChain`) are reachable only from Rust; the commands that call them are held to the JavaScript in
step 5 (the connect dialogs, the S-curve, the extended connection), which is what they are replayed through.
