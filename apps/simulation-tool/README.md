# Simulation-Tool

The running times of the suite's Fahrplan-Sim: one train over the net of a variant, simulated in the browser with
OSRD's simulation core ported to Rust ([decision 0013](../../docs/decisions/0013-running-times-in-rust.md)). It
reads the variant — this browser's working copy where there is one, else its head — and writes nothing.

The start page lists the projects under *Fahrplan-Sim* and opens a variant here by its address,
`simulation/?project=<id>&variant=<id>` (`&action=view` its head alone); `simulation/?vehicles` opens the
vehicles on their own.

## What it does

- **The train**: a vehicle of the catalogue, in the traction mode and comfort chosen — the net does not say yet
  where there is which overhead line.
- **Where it runs**: a start, stops with their dwell times and an end — signals of the signalling domain (in the
  way they act), platforms of the layout (at their middle), or points picked on the map. The path is the
  shortest over the net, over switches along their routes, links and joints; at a stop the train never turns
  back (`path.js`, `route.js` of `@rail-across/running-times`).
- **What the physics gets** (`request.js` of `@rail-across/running-times`, which the Fahrplan-Editor shares): the path's length; its grades from the tracks' heights,
  the curves' resistance counted in after Röckl (650 / (r − 55) ‰); the layout's design speeds and the speeds
  the signals signal per route (a logical signal's `signallingSpeeds`, from the start signal to the target);
  a standard margin (`5 %`, `3 min/100km`), spread energy-saving after MARECO or linearly. Where heights or
  design speeds are missing it says so: flat there, only the vehicle limiting.
- **The result**: the fastest run, the run with its margin, arrival and departure at every stop, the energy at
  the wheels; the speed and the time over the distance with the speed allowed; the run played on the map.
- **The way's profile**: under the map, on one axis of distance, the speed over the way (or the time) and under
  it the height of the tracks along the same stretch, from their height points (`heightProfile` of
  `@rail-across/running-times`), shaded where a track has none; the stops marked, the moment played, and where
  the mouse is its distance, speed and height. The height is shown as soon as the way is found, before the run.

## The vehicles

A train's vehicle is one of the suite's catalogue — OSRD's RailJSON rolling stock on the server and OSRD's
example vehicles ([decision 0014](../../docs/decisions/0014-vehicles-in-osrds-form.md)) —, through
`@rail-across/vehicles`. They are edited in the Fahrzeug-Editor (`apps/vehicle-editor/`); *Ansehen* opens the
chosen one there in a tab of its own, and the catalogue is loaded again when this tab is shown again.

## The simulation module

The simulation is OSRD's envelope simulation in Rust, held to OSRD's own test cases (`sim/`,
[decision 0013](../../docs/decisions/0013-running-times-in-rust.md)). This tool loads its WebAssembly module from
`public/sim/rail_across_sim.wasm`, a file of its own beside the page (`engine.js` of `@rail-across/running-times`:
JSON in, JSON out, no bindings generator), with its notice. The built module is not in the history:

```sh
npm run sim   # builds it with cargo from sim/ (rustup, target wasm32-unknown-unknown), or takes RA_SIM_WASM
```

The deploy runs the same step (`deploy/deploy.sh`). Without the module the tool loads and says it is missing; its
tests of the module are skipped.

## Development

```sh
npm run sim && npm run dev   # with a project server on 127.0.0.1:8787 (OLT_API_TARGET)
npm test
npm run lint
```
