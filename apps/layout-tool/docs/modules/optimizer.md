# optimizer — the track optimizer

`src/core/utils/optimizer/` · needs nothing of the other modules

The computations that build or change an alignment against the rule catalogue (decision 0019 of the suite):
"Zwei Gleise zusammenfügen" with or without a spacing to another track, "Bestehende Elemente neu verbinden",
optimizing a track or one of its curves, the alignment and gradient fit from measured axis points.
They are Rust — `optimizer/` of the repository, built to WebAssembly by `npm run optimizer` into
`public/optimizer-module/` — and run in a worker here, off the page's thread, in both builds. The requests are the
core's (`commands/splice.js` `spliceRequest`, `commands/reconnect.js` `reconnectRequest`,
`commands/optimizeApply.js` `optimizeRequest`, `survey/alignmentFit.js` `alignRequest`), and so is reading the
answers.

The module rechecks nothing of the app's rules on its own account: the worker hands it the catalogue the app
bundles (`KATALOG` of `@rail-across/core/rules`) with the first request, and it applies that. Its rule check
is a second implementation beside `packages/core/src/rules`, held to the same answers by the shared vectors
(`packages/core/constraints/tests/`).

## Files

| File | What it does |
|---|---|
| `optimizerJob.js` | `runOptimizer(op, request, { signal })`: one worker kept between requests; a request given up takes the worker with it. Rejects where the module cannot answer (`optimizer_missing`, `internal`, `invalid_request`) |
| `optimizerWorker.js` | The worker: fetches the module beside the page once, hands it the catalogue, runs each request |
| `optimizerRun.js` | The module's interface without a bindings generator: one message's JSON (`{ op, catalogue?, request }`) into its memory, the answer's out |
| `optimizerRegelwerke.js` | The catalogue the module applies and its limits at both levels, for the regelwerk popup |

## Operations

| `op` | Request | Answer |
|---|---|---|
| `splice` | `spliceRequest`, with `clearance` (`ref`, `dMin`, `profile`, `maximize`, …) where a spacing is kept | `{ solutions, requested? }` or `{ error, params, lengths }` |
| `reconnect` | `reconnectRequest` | `{ solutions, tolerance }` or `{ error, params }` |
| `optimize` | `optimizeRequest` | `{ elements, report, variant, vBestand, vBaseline, vNeu, … }` or `{ error: 'unsupported_topology', message }` |
| `align` | `alignRequest` | `{ stations, kappa, straights, curves, elements, offsets, gradient, elementCants, … }`, with `error` where no chain fits |
| `limits` | — | `{ id, name, version, grenzwerte: { reg, discretion } }` |

## Tests

The Rust side is held to what the Python service answered: 243 splice requests, the spacing cases and the
reconnect round trips, the optimizer on the service's test tracks, the alignment fits
(`optimizer/crates/optimizer/tests/`). Here `splice.test.js`, `optimize.test.js` and
`spliceRemedies.test.js` of [splice](splice.md) run the module cargo built (`src/core/test/optimizerModule.js`, skipped
where there is none) on the app's own cases and build the tracks the answers lead to.
