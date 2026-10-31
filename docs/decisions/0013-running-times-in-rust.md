# 0013 — Running times with OSRD's simulation core, ported to Rust

Status: **proposed** — changes [0006](0006-capacity-on-osrd.md) for the running time of one train

## Decision

1. **The running time of one train is computed in the browser**, by a port of OSRD's envelope simulation to Rust:
   `rail-across-sim`, built to WebAssembly. What is ported is OSRD's `core/envelope-sim` with the parts of
   `osrd-mp`, `kt-osrd-path` and the standalone simulation it needs — the most restrictive speed profile, braking
   and acceleration curves, the speed held on grades, the linear and MARECO allowances, schedule points. Each
   ported file names its OSRD original and the commit it was taken from.
2. **`rail-across-sim` is a repository of its own, under LGPL-3.0**, as OSRD is — a port is a derived work. This
   repository holds none of its code: the Simulation-Tool (`apps/simulation-tool/`) loads its module as a file
   beside the page (`npm run sim` builds or fetches it), which can be swapped for another build of it.
3. **The adapter is ours** (Apache-2.0, `apps/simulation-tool/src/model/`): the path over the net of a project
   (over switches along their routes, links and joints, never turning back at a stop), its grades from the heights
   with the curves' resistance after Röckl, the layout's design speeds, the speeds the signals signal per route,
   the stops at signals, platforms or points on the map.
4. **0006 stands for the capacity simulation**: conflicts, blocking times, the signalling simulation and STDCM
   stay OSRD's, run as a service. The port follows OSRD; corrections found while porting go upstream.

## Why

- The suite works in the browser, on a working copy that is not checked in yet: a running time that needs no
  server, no JVM and no import into OSRD answers while the layout is drawn.
- The envelope simulation is the part of OSRD that is pure computation, a few thousand lines, and settled; its
  own test cases (MaxEffortEnvelopeTest, AllowanceTests) are the reference the port is held to.
- Rust gives one source for the browser (WebAssembly, 0.4 MB) and a command line (`ra-sim`), with OSRD's
  numbers unchanged — no garbage collector, the same doubles.

## What it costs

- A port drifts from upstream: changes to OSRD's envelope simulation have to be carried over by hand. The
  commit a file was taken from is named in it.
- A second language and toolchain (cargo, the `wasm32-unknown-unknown` target) for the deploy.
- LGPL-3.0 code next to the suite: kept in its own repository and loaded as a replaceable module; its notice
  goes with the build (`public/sim/NOTICE.txt`, THIRD-PARTY.md).
- Not ported: ETCS braking curves, electrification and power restrictions, DB's driver behaviour; vehicles are
  examples until the suite has its vehicle editor.

## Alternatives considered

- **OSRD as a service only (0006)**: needs the whole stack, an import of every state, and a server round trip
  for each run.
- **A running time calculation of our own in JavaScript**: repeats OSRD's work without its tests.
- **OSRD's Kotlin compiled to JavaScript or WebAssembly**: the envelope simulation depends on JVM-only modules;
  Kotlin/Wasm is not yet stable.
