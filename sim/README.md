# sim — rail-across-sim

The running time simulation of [OSRD](https://github.com/OpenRailAssociation/osrd), the Open Source Railway
Designer of the OpenRail Association, ported from Kotlin/Java to Rust — for the browser (WebAssembly) and the
command line. It is the simulation behind the suite's Fahrplan-Sim ([decision
0013](../docs/decisions/0013-running-times-in-rust.md)); it was a repository of its own until the suite came under
OSRD's licence ([decision 0015](../docs/decisions/0015-lgpl.md)).

Ported from OSRD `core/` at commit `fda69ee1f8027175096371d258c563fc98f9295e` (2026-10-30). Every source file
names the OSRD files it comes from.

## What is ported

| Here | OSRD |
|---|---|
| `units.rs` | `osrd-mp/.../utils/UnitEquality.kt`, `envelope_utils/DoubleUtils.java`, `CmpOperator.java`, `envelope/SearchableEnvelope.java` |
| `envelope/` — parts, envelopes, cursors, builders, constraints, MRSP, speed cap | `envelope-sim/.../envelope/` |
| `physics/` — rolling stock, path grades, integrator, overlays | `osrd-mp/.../envelope_sim/`, `kt-osrd-path/.../EnvelopeSimPath.kt`, `train/RollingStock.kt`, `envelope_sim/overlays/` |
| `pipelines.rs` — MRSP, max speed and max effort envelopes | `envelope_sim_infra/MRSP.kt`, `envelope_sim/pipelines/` |
| `allowances/` — linear and MARECO | `envelope_sim/allowances/`, `envelope_utils/DoubleBinarySearch.java` |
| `simulation.rs` — one train with its schedule and margins | `standalone_sim/StandaloneSimulation.kt`, `EnvelopeStopWrapper.java` |

Not ported: ETCS braking curves, electrification modes, electrical profiles and power restrictions, neutral
sections, the driver behaviour of SNCF's signalling systems, the signalling simulation, pathfinding, conflict
detection and STDCM. The path comes in as its length, grades and speed limits; the request and the result are
this crate's own JSON.

The vehicles it is run with are OSRD's RailJSON rolling stock; OSRD's example vehicles are in
`packages/vehicles/data/osrd/`.

## Tests

OSRD's own cases are the reference (`crates/sim/tests/osrd_cases.rs`, from `MaxEffortEnvelopeTest.kt` and
`AllowanceTests.kt`): the positions where the run changes from accelerating to holding to braking, and the
shape of MARECO's coasting, each within OSRD's tolerance of the values OSRD computed. `tests/simulation.rs` runs
the whole simulation through its JSON.

```sh
cargo test
cargo clippy --all-targets
```

## Use

```sh
cargo run -p rail-across-sim-cli --release -- request.json      # ra-sim: the result's JSON on stdout
cargo build -p rail-across-sim-wasm --release --target wasm32-unknown-unknown
```

The WebAssembly module needs no bindings generator: `alloc`, `simulate`, `result_len`, `dealloc` and its
`memory` (see `crates/wasm/src/lib.rs`). A request:

```json
{
  "path": {
    "length": 20000,
    "gradients": [{ "begin": 2000, "end": 6000, "value": 12.5 }],
    "speed_limits": [{ "begin": 0, "end": 20000, "speed": 44.4, "source": "design" }]
  },
  "rolling_stock": {
    "mass": 120000, "length": 70, "max_speed": 44.4, "inertia_coefficient": 1.08,
    "rolling_resistance": { "A": 1200, "B": 25, "C": 6 }, "const_gamma": 0.6,
    "effort_curve": [{ "speed": 0, "max_effort": 150000 }, { "speed": 45, "max_effort": 50000 }]
  },
  "schedule": [{ "path_offset": 10000, "stop_duration": 30 }, { "path_offset": 20000, "stop_duration": 0, "arrival": 900 }],
  "margins": { "boundaries": [], "values": ["5%"] },
  "distribution": "mareco",
  "time_step": 2.0
}
```

Units are SI: metres, seconds, m/s, kg, N; grades in m/km, the resistance of curves counted in. The result
holds the fastest run (`base`), the run with the margins (`provisional`) and the run that also keeps the
scheduled arrivals (`final`) — each its positions, times (stops counted in), speeds, the times at the scheduled
points, its profiles and the mechanical energy —, the speed limits along the path (`mrsp`) and warnings; or
`{ "error": { "code", "message" } }`.

## Licence

LGPL-3.0-only, as OSRD and the whole suite (`../LICENSE`, with the GPL-3.0 it builds on in `../LICENSE-GPL`).
Copyright 2022 The OSRD Contributors; 2026 Serigalan and the Rail-Across-Suite contributors.
