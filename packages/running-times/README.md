# @rail-across/running-times

The running times of trains over a variant's net ([decision 0013](../../docs/decisions/0013-running-times-in-rust.md)),
shared by the Simulation-Tool and the Fahrplan-Editor:

- **the way** (`path.js`, `route.js`): the shortest path over the net — over switches along their routes, links
  and joints —, through a train's points without turning back; a point may be one of several places (the tracks of
  a station, `routeVia`), the others tried where the nearest leads nowhere further on; a leg may be given its run
  of tracks (`pathAlong`, `routeVia`'s `legPath`), the way a timetable prefers;
- **what the physics needs of it** (`path.js`, `request.js`): its length, its heights along it for a profile (`heightProfile`), its grades from the tracks' heights and
  the curves' resistance (Röckl), the layout's design speeds and the speeds the signals signal, and the request
  the simulation takes;
- **the stations** (`stations.js`), each by its id — its point of the core's register (`stationRegister`), an end's mark —, found by it (`stationById`, `placeOf`), the name only shown: a station is the station tracks of one point, as the Layout-Tool groups them, or
  a station of the register on the tracks of its stopping places and platforms, as far as they reach; a halt a name
  only platforms carry; an end a buffer stop or a boundary of the planning area, a place at a track's end where a
  train may begin and end, over a boundary at its speed (`crossingSpeed`, the request's `startSpeed`, `endSpeed`); where a train may be at one, and the stations along a way;
- **the simulation module** (`engine.js`): OSRD's simulation core in Rust (`sim/`) as WebAssembly, loaded from
  `sim/rail_across_sim.wasm` beside the app's page;
- the map's lines and the formats of times and distances (`places.js`, `format.js`).

An app that runs trains puts the module beside its page with `npm run sim`, which runs
[`scripts/sim.mjs`](scripts/sim.mjs) in the app's directory: it builds `sim/` with cargo (rustup, target
`wasm32-unknown-unknown`), or takes `RA_SIM_WASM`, and writes `public/sim/` with the module's notice and the licence texts.

```sh
npm test   # the tests that run the module where cargo built it (sim/target/…)
```
