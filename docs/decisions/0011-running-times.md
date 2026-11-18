# 0011 — Running times with OSRD's envelope simulation, ported to Rust

Status: **accepted** · as of 2026-11-18 · what it computes: [assumptions of the simulation](../simulation/simulation-assumptions.md)

## Decision

1. **The running time of a train is computed in the browser** by a port of OSRD's envelope simulation to Rust
   (`sim/`, `rail-across-sim`), built to WebAssembly and loaded as a module of its own beside the page, which can
   be replaced. What is ported is OSRD's `core/envelope-sim` with the parts of `osrd-mp`, `kt-osrd-path` and the
   standalone simulation it needs: the most restrictive speed profile, braking and acceleration curves, the speed
   held on grades, the linear and MARECO margins, stops. Each ported file names its OSRD original and the commit it
   was taken from (OSRD `fda69ee1`). The same crate builds a command line (`ra-sim`).
2. **It is in this repository**, under the suite's licence, as OSRD's own is LGPL-3.0 (0001).
3. **The adapter is ours** (`@rail-across/running-times`): the way over the net of a variant through a train's
   points, its grades from the heights with the curves' resistance after Röckl, the design speeds and the speeds the
   signals signal per route, the stopping places a train may use, a reversal on the way, the traction mode from the
   electrification.
4. **One deliberate difference from OSRD: braking depends on the grade.** A train brakes at its vehicle's service
   deceleration on the level plus the force of the grade under it, at least a tenth of the deceleration left on
   any fall. OSRD's constant braking stays selectable in the calculation (`braking: "constant"`); the suite does
   not use it.
5. **Its results are held to OSRD's own test cases** (MaxEffortEnvelopeTest, AllowanceTests) where the port
   follows OSRD, and to cases worked out by hand where it differs.
6. **A running time is also computed over a net of OSRD's.** The server holds RailJSON infrastructures laid into
   a directory of its own by hand (`RA_SERVER_OSRD`); *Fahrzeit rechnen* runs over one of them instead of the
   variant's net: its operational points the places, the way over its switches' routes and links, its slopes,
   curves, speed sections and electrification what the train runs with. Its signals signal no speed, as in OSRD's
   own run.

## Why

- The suite works in the browser, on a working copy not checked in yet: a running time that needs no server, no
  JVM and no import into OSRD answers while the layout is drawn.
- The envelope simulation is the part of OSRD that is pure computation, a few thousand lines, and settled; its own
  tests are the reference.
- Rust gives one source for the browser (0.4 MB of WebAssembly) and a command line, with OSRD's numbers unchanged.
- Braking on the grade is what a braking distance does on a fall; a constant deceleration makes a train stop as
  short downhill as on the level.

## What it costs

- A port drifts from upstream: changes to OSRD's envelope simulation are carried over by hand.
- The difference in braking is ours to keep and to test.
- A second language and toolchain for the deploy (cargo, the `wasm32-unknown-unknown` target).

## Open

Not ported, because OSRD's parts are not or the suite does not use them yet: ETCS and LZB braking curves, driver
behaviour at signals and the signalling simulation, electrical profiles and power restrictions, a change of
traction mode on the way, formations of several vehicles, adhesion.

## Alternatives considered

- **OSRD as a service**: its whole stack, an import of every state, and a server round trip for each run.
- **A running time calculation of our own in JavaScript**: repeats OSRD's work without its tests.
- **OSRD's Kotlin compiled to JavaScript or WebAssembly**: the envelope simulation depends on JVM-only modules.
