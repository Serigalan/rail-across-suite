# Simulation-Tool

The running times of the suite's Fahrplan-Sim: one train over the net of a variant, simulated in the browser with
OSRD's simulation core ported to Rust ([decision 0013](../../docs/decisions/0013-running-times-in-rust.md)). It
reads the variant — this browser's working copy where there is one, else its head — and writes nothing.

The start page lists the projects under *Fahrplan-Sim* and opens a variant here by its address,
`simulation/?project=<id>&variant=<id>` (`&action=view` its head alone).

## What it does

- **The train**: an example vehicle (a regional multiple unit, a locomotive with coaches, a freight train) —
  mass, length, top speed, resistance to movement, braking deceleration, tractive effort — until the suite has its
  vehicle editor (`src/model/vehicles.js`).
- **Where it runs**: a start, stops with their dwell times and an end — signals of the signalling domain (in the
  way they act), platforms of the layout (at their middle), or points picked on the map. The path is the
  shortest over the net, over switches along their routes, links and joints; at a stop the train never turns
  back (`src/model/path.js`, `route.js`).
- **What the physics gets** (`src/model/request.js`): the path's length; its grades from the tracks' heights,
  the curves' resistance counted in after Röckl (650 / (r − 55) ‰); the layout's design speeds and the speeds
  the signals signal per route (a logical signal's `signallingSpeeds`, from the start signal to the target);
  a standard margin (`5 %`, `3 min/100km`), spread energy-saving after MARECO or linearly. Where heights or
  design speeds are missing it says so: flat there, only the vehicle limiting.
- **The result**: the fastest run, the run with its margin, arrival and departure at every stop, the energy at
  the wheels; the speed and the time over the distance with the speed allowed; the run played on the map.

## The simulation module

`rail-across-sim` is a repository of its own under LGPL-3.0: OSRD's envelope simulation in Rust, held to OSRD's
own test cases. This tool loads its WebAssembly module from `public/sim/rail_across_sim.wasm`, a file of its own
beside the page (`src/model/engine.js`: JSON in, JSON out, no bindings generator), with its notice. It is not in
the history:

```sh
npm run sim   # builds it with cargo from ../../../rail-across-sim (RA_SIM_DIR), or takes RA_SIM_WASM
```

The deploy runs the same step (`deploy/deploy.sh`, `RA_SIM_DIR`). Without the module the tool loads and says it
is missing; its tests of the module are skipped.

## Development

```sh
npm run sim && npm run dev   # with a project server on 127.0.0.1:8787 (OLT_API_TARGET)
npm test
npm run lint
```
