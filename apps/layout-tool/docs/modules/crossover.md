# crossover — the search of the extended switch connection

`src/core/utils/crossover/` · needs nothing of the other modules

The search behind "Erweiterte Weichenverbindung" (decision 0018 of the suite): two turnouts, each of its own form,
and one to three elements between their branches, both toes moved along the elements picked. The search itself
is Rust — `crossover/` of the repository, built to WebAssembly by `npm run crossover` into `public/crossover/` —
and runs in a worker here, off the page's thread. The request is the core's (`commands/crossover.js`
`crossoverRequest`, with `elements` or `templates` where the dialog sets them); an answer taken is solved again
by the core (`adoptCrossover`) before the dialog builds it.

## Files

| File | What it does |
|---|---|
| `crossoverSearch.js` | `searchCrossover(request, { signal })`: one worker per search, ended once it has answered or when `signal` aborts |
| `crossoverWorker.js` | The worker: fetches the module beside the page and runs the request — `{ error: 'crossover_missing' }` where it is not there |
| `crossoverRun.js` | The module's interface without a bindings generator: the request's JSON into its memory, the answer's out (as the simulation module's) |

## Tests

`crossoverRun.test.js` runs the module cargo built (`crossover/target/…`, skipped where there is none) on a pair
of tracks no one arc can join: the best answer is solved again and built without an error of LP.EL.01 or
LP.KS.01, elements set are kept, each turnout is built of its own form, and nothing that closes says so.
