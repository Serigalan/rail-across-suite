# optimizer — the track optimizer

`src/core/utils/optimizer/` · needs nothing of the other modules

The computations that build or change an alignment against the rule catalogue (decision 0019 of the suite):
"Zwei Gleise zusammenfügen" with or without a spacing to another track, "Bestehende Elemente neu verbinden".
They are Rust — `optimizer/` of the repository, built to WebAssembly by `npm run optimizer` into
`public/optimizer-module/` — and run in a worker here, off the page's thread, in both builds. The requests are the
core's (`commands/splice.js` `spliceRequest`, `commands/reconnect.js` `reconnectRequest`), and so is reading the
answers (`spliceFromAnswer`, `reconnectFromAnswer`).

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

## Operations

| `op` | Request | Answer |
|---|---|---|
| `splice` | `spliceRequest`, with `clearance` (`ref`, `dMin`, `profile`, `maximize`, …) where a spacing is kept | `{ solutions, requested? }` or `{ error, params, lengths }` |
| `reconnect` | `reconnectRequest` | `{ solutions, tolerance }` or `{ error, params }` |

## Tests

The Rust side is held to what the Python service answered: 243 splice requests, the spacing cases and the
reconnect round trips (`optimizer/crates/optimizer/tests/`). Here `splice.test.js` and
`spliceRemedies.test.js` of [splice](splice.md) run the module cargo built (`src/core/test/optimizerModule.js`, skipped
where there is none) on the app's own cases and build the merged tracks the answers lead to.
