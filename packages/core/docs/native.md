# native — the core's Rust crates as one WebAssembly module

`packages/core/src/native/` · depends on nothing else in the core

Decision 0020 moves the core to Rust module by module. The crates (`crates/` of the repository) are built
as one WebAssembly module, `crates/core-wasm`, and this module is the face the JavaScript calls it
through: a request `{ op, input }` as JSON in, `{ ok }` or `{ error, detail }` out — the protocol the
simulation and the optimizer already use, with no bindings generator.

The module is loaded **once**, and every call after that is synchronous, as the core's calls always were:

- in the browser, a page awaits `loadNative(url)` before it renders;
- in Node — the server, the tests — `loadNativeFile()` (`@rail-across/core/native/node`) reads `packages/core/wasm/` synchronously.

Since step 10 the module is not optional: the functions that went to Rust have no JavaScript any more,
and every page, the desktop app's and every vitest of the repository load it first (`test/setupNative.js`).
`viaRust(op, revive)` makes such a function: the arguments go across as the replay recorded them (a Map as
an object, a Set as an array), and `revive` gives back what JSON does not carry — `asMap`, `keepSame` and
`reuseFrom` (what the module passed through untouched, given back as the very object it was). What the
JavaScript threw comes back as an Error with its message. A worker that does not load the module uses
only what is still JavaScript; one that calls the core — the Layout-Tool's MDB import — is handed the page's
compiled module (`nativeModule()`, carried by `postMessage`) and loads it with `loadNative` first.

## The held project (decision 0021, phase C)

The page hands the module the project it has open — `holdProject(record)`, which the Layout-Tool's store calls
on every write — and the module keeps it (`crates/core-wasm/src/ops_session.rs`). The first call opens it; each
after that writes only what changed, found by reference (the record is immutable and frozen as it is written): a
member that is the same object is left out, and of an array that changed, an item that is the same object is
named by its place in the held one (`{"$k": i}`) and one the module answered by its answer (`{"$a": [n, i]}`),
so a track a command made is not sent back. From then on an argument of any operation that *is* a member of
the held project, or the project itself — up to two levels into the arguments — crosses as a marker
(`{"$held": "tracks"}`), and the module lends the operation what it holds there for the call, moved rather than
copied. `heldProjectDocument` and `heldKeptProject` (`@rail-across/core/project`) make the document and the
kept record from what the module holds; `planElementChange` on the held tracks asks `session.planElementChange`,
which answers only the tracks it touched, and `reuseTracks` gives back what of them is the same as the page's
own objects.

Measured on the PEK Bestand (113 tracks, 69 switches), one drag step of the element table (`planElementChange`
and the step written back), on a machine under load:

| | across JSON (before) | held |
|---|---|---|
| Node (V8) | — | 9 ms + 1.4 ms written |
| Chrome 154 | 184–237 ms | 10–15 ms |
| WebKitGTK 2.50 (the desktop app's engine on Linux) | 200–213 ms | 13–15 ms |

Opening it (the first `holdProject`) takes 60–160 ms, once.

Since phase D the track and switch modules, the cross section, the element table, the station register and
loading a record (`project.hydrateProjects`) are the module's. A held part crosses as its path — a member, an
item of one, a member of that (`{"$held": ["tracks", 3, "elements"]}`) —, and where an operation is asked on
every write the page names what changed rather than the module comparing everything
(`switch.coupleSwitchCantsChanged`, which answers only the tracks the coupling changed). A write of the
Layout-Tool's store on the PEK Bestand takes 1–2 ms; opening it 1.2 s (1.0 s with the JavaScript before).

## Files

| File | What |
|---|---|
| `index.js` | `loadNative`, `useNativeModule`, `callNative`, `viaRust`, `asMap`, `keepSame`, `reuseFrom`, `reuseTracks`, `nativeReady`, `unloadNative`, `NativeError`; the held project: `holdProject`, `heldProject`, `answeredBy` |
| `node.js` | `loadNativeFile` — the module from its file, synchronously (Node only) |

`npm run wasm` (`scripts/wasm.mjs`) builds the module with cargo and puts it in `packages/core/wasm/`
(not in the repository); `RA_CORE_WASM` names a module file to take instead.

## What has moved

| Step | Crate | Functions the face hands to it |
|---|---|---|
| 1 | `crates/doc` (with `crates/js`, the JavaScript meaning of JSON values, and the first part of `crates/geom`) | `checkDocument`, `toDocument`, `fromDocument`, `manifestOf`, `documentHash` (document); `parseProjectsPayload`, `projectDocument`, `dehydrateProjects`, `parseTracksPayload` (project) |
| 2 | `crates/geom`: planes and the proj4js port (extended transverse Mercator, Cassini-Soldner, Helmert, NTv2 grids — a grid the page reads is handed to the module), transitions, element values, the factory, display geometry, parallels, plane changes, chain checks and closing, cants | none yet: the map calls these tens of thousands of times a frame, one at a time across JSON would be slower than proj4js. They serve the crates that call them (commands, the server) and reach the page through the coarser operations of the later steps. |
| 3 | `crates/rules`: the catalogue's expression language, the catalogue, the check (`alignmentCheck`), the limits a run is held to — taken over from the optimizer, which now uses them from here —, and the cant, speed, element length and field rules | `ruleMaxSpeed` and so `maxSpeeds` (235 ms against 388 ms for all 113 elements of a PEK track). `checkTrack` stays here: it answers with every value every rule read, and carrying those across JSON costs more than evaluating them (48 ms against 6 ms). |
| 4 | `crates/track` (the track module, the cross section, the topology graph, its lanes and label placement), `crates/survey` (measured and reference axes, shift values, the fit's bookkeeping), `crates/switch` (the catalogue and its tables, routes and chains, crossings, the symbol, sleepers and the turnout's plane, coupled cants, placement, connecting to an open end, switch connections, deletion, the detail drawing) and `crates/project` (`validateProject`) | none yet: the pages call these in small pieces; they serve the commands (step 5) and the server (step 8). Replayed over 6,400 calls; `validateProject`, `rebuildSwitchSymbol`, `planSwitchDeletion` and `switchCouplings` are also held to the JavaScript on the PEK data at run time (`src/switch/native.test.js`), where Rust validates in half the time. |
| 5 | `crates/commands`: what the dialogs commit (tracks, element roles and the table, geometry edits, the S-curve, the switch dialogs, connecting to an open end and its remedies, splicing, reconnecting, the extended switch connection's request and adoption, editing a connection, the optimizer's apply) and `merge/` (diff, the three-way merge, carrying references and the domains' locations) | none yet. Measured on the PEK Bestand (113 tracks): `planElementChange` across JSON with the whole project 114 ms against 5–7 ms in JavaScript — the project crosses the boundary on every call. Held in the module (`session.open`, `session.write`, `session.planElementChange`, copied on write) one drag step is 4.3 ms and only the tracks it touched come back; `solveConnection` 1.4 ms against 1.3, `mergeProject` 2.5 s against 3.9 s. Replayed over 7,000 calls; the held edit and the merge were also held to the JavaScript on the PEK data until step 10 took that JavaScript out. |
| 6 | `crates/signalling` (the catalogue of DB Ril 301, the check, overlaps, the speed a turnout allows a run, the track sections' extent) and `crates/planpro` (the XML reader, PlanPro's enumerations, a plan as a record — since decision 0022 no longer in the module, the check of the PlanPro export, replaying its vectors in the crate) | none yet: the page checks a station in 0.3 ms in JavaScript against 0.7 ms across JSON, and reads the test plan in 2.3 ms against 2.0 — no gain worth the boundary; they serve the server (step 8). Replayed over 4,200 calls: the check on the Ks station of the tests and every variation the tests make of it, the catalogue's lookups, the XML reader on good and broken text, and the plan of the import's tests with variations (curved geometry, a node that is no turnout, every object the import reads and those it cannot place). |
| 7 | `crates/pointcloud` (LAS, LAZ with `laz`, E57, the tiles with `flate2`, the import into five levels, plane mapping, registration, slices, the clearance check and scan, rail detection and tracing) — not part of this module: it serves the server, and replays its vectors itself (`crates/pointcloud/tests/replay.rs`) | the import in the browser's worker, as `crates/pointcloud-wasm` (`@rail-across/pointcloud`'s `native.js`, step 10; the JavaScript readers and import and laz-perf are gone, their vectors frozen). Replayed over 137 calls: files made from a description on both sides and the package's E57 fixtures, read and imported and held by sums over every point and segment (zlib's bytes differ from fflate's, what they hold does not); the track scene's section, clearance scan and trace. On the real LAZ sample (49 million points, format 6) the first million came out of laz-rs as out of laz-perf, ten million in 3.5 s against 8.8 s; in the worker the 3.2-million-point sample imports in 3.4 s against laz-perf's 6.1 s. Not to the digit: the registration's tilts on points along a straight line, which the pairs hardly determine. |
| 8 | `crates/server` (the project server: users and sessions, projects, variants and revisions checked by the core's crates — `checkRecord` with `validateProject` and `checkSignalling` —, vehicles, timetables, point clouds with uploads, tiles, share links and jobs, the DGM1, the MDB import) — not part of this module | none: a server. The server's own tests run against it over HTTP; it is the only project server that runs. |
| 9 | `crates/desktop` (Tauri: the suite in one window, `crates/server` in the same process behind the URI scheme `rail`, local mode, start screen and settings, `mdb-export` bundled) — not part of this module | none: the desktop app loads this module like any page. |
| 10 | none new: the JavaScript of what had moved goes where the page does not ask it while it draws, drags or renders — topology, survey, merge, PlanPro, the signalling check and track sections, the document's door and keeping a record, the validation, the commands, the switch solvers — and `core-py` and the Node server go | the page calls the module through `viaRust`; the vectors of what went are frozen. What stays JavaScript and why: decision 0020, step 10. |

The crates are held to the JavaScript three ways: the shared vectors (`schemas/vectors/`), the calls the
JavaScript records (`npm run replay` → `crates/*/tests/vectors/`, replayed by `crates/core-wasm/tests/replay.rs`),
and the core's own tests, which run on the module where it is built (and with `RA_NATIVE=0` on the JavaScript).

Not ported, on purpose: the schematic strip (`schematic/`) takes a JavaScript callback (`kmOf`) and answers
with a structure of closures and back references — drawing code like the map's, it stays in the page. The
functions of step 4 that search with a callback (`connectionRoots`, `solveEndChain`, `validIntervals`,
`solveMidChain`) are reachable only from Rust; the commands that call them are held to the JavaScript in
step 5 (the connect dialogs, the S-curve, the extended connection), which is what they are replayed through.
