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

The crates are held to the JavaScript three ways: the shared vectors (`schemas/vectors/`), the calls the
JavaScript records (`npm run replay` → `crates/*/tests/vectors/`, replayed by `crates/core-wasm/tests/replay.rs`),
and the core's own tests, which run on the module where it is built (and with `RA_NATIVE=0` on the JavaScript).

Not ported, on purpose: the schematic strip (`schematic/`) takes a JavaScript callback (`kmOf`) and answers
with a structure of closures and back references — drawing code like the map's, it stays in the page. The
functions of step 4 that search with a callback (`connectionRoots`, `solveEndChain`, `validIntervals`,
`solveMidChain`) are ported but reachable only from Rust; the commands that call them are held to the
JavaScript in step 5.
