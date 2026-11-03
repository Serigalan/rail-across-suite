# 0018 — The switch connection: one element in JavaScript, the extended one searched in Rust

Status: **accepted** · 2026-11-03 · replaces the Layout-Tool's decisions 357–358 (the search in the optimizer service)

## Decision

1. **The switch connection ("Weichenverbindung") is the two turnouts and one element between their branches,**
   computed in the Layout-Tool's core in JavaScript as before: a straight, or an arc where the toes leave a turn.
   The straight is the connection one wants; the dialog lays it on one first where the tracks allow it, and its
   slider snaps there. It has no chain of elements and asks no service any more.
2. **Every other configuration is the extended switch connection ("Erweiterte Weichenverbindung")**, its own
   entry in the panel: each turnout of any form of the catalogue — laid into its track, so in a curve an inner-
   or outer-bent turnout —, one to three elements between them, each a straight, an arc or a transition, each
   length and radius sought or given, or every sequence tried. It offers no slider: it lists what it finds,
   nearest to the click first, and builds the one taken.
3. **Its search is written in Rust** (`crossover/`) and runs in the browser as WebAssembly, beside the page
   (`public/crossover/`), as the running time simulation does ([0013](0013-running-times-in-rust.md)). The
   turnout geometry stays in the core: it samples the branch ends and solves an answer again before it builds
   it. The search in the optimizer service (`/crossover`, Python) is removed; the service keeps its other work.

## Why

- The one-element connection is closed and quick; a chain of elements needs a search, which belongs in its own
  dialog rather than behind the slider of the simple one.
- In the browser the search needs no server and works in the local build too; in Rust it is one source, quick
  (a few hundred milliseconds where Python took ten seconds), with SLSQP from Kraft's code as scipy has it.

## What it costs

- A second Rust build for the deploy (cargo with `wasm32-unknown-unknown`, as for `sim/`), and one third-party
  crate in the module (slsqp, MIT, THIRD-PARTY.md).
- A connection built with two different forms or more than one element is edited in the extended dialog.

## Alternatives considered

- **Keep the Python search**: a service round trip, only in the server build, and scipy as its dependency.
- **The search in JavaScript**: SLSQP would have to be written again; Rust has it ported and tested.
