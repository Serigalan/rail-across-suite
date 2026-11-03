# crossover — rail-across-crossover

The search of the Layout-Tool's extended switch connection ("Erweiterte Weichenverbindung"), in Rust, for the
browser (WebAssembly): two turnouts facing each other on two tracks — each of any form of the catalogue, laid
into its track, so in a curve an inner- or outer-bent one — and one to three elements between their branches.
It moves both toes on the elements they were picked on and chooses the elements between: straights, arcs and
transitions, each value sought or given ([decision 0018](../docs/decisions/0018-crossover-search-in-rust.md)).

The turnout geometry stays in the app's core: it samples both branch ends over the elements
(`connectionBranchEnds`, `crossoverRequest` in `packages/core/src/commands/crossover.js`) and sends them with the
limits its dialog holds the middle to — l_min (LP.EL.01), the comparison radius (LP.KS.01), the cant deficiency
and the cants at the ldS. The search interpolates the samples (a not-a-knot cubic spline, as scipy's), closes
chains of elements between the two branch ends by least squares (Levenberg–Marquardt in a box) and moves each
along what closes with SLSQP ([slsqp](https://crates.io/crates/slsqp)) to the best place: near the click, on
gentle curves, short. Answers come best first — nearer the click in whole metres, then those that keep the
Regelwert, then fewer elements. The app solves an adopted answer again with its own geometry
(`adoptCrossover`) before it builds it.

It replaces the search the optimizer service ran in Python (`crossover.py`, decisions 357–358 of the
Layout-Tool); its acceptance cases and two of its answers are the tests here.

## The request

```json
{
  "speed": 80,
  "limits": { "lMin": 20, "rwReg": 1100, "rwDiscretion": 760, "maxCantDef": 130, "cantDefCoeff": 11.8, "cantTol": 1 },
  "forms": [{ "label": "760 – 1:14", "label2": "500 – 1:12", "first": [sample], "second": [sample] }],
  "elements": [{ "type": "arc", "radius": 1500 }, { "type": "straight" }, { "type": "arc" }]
}
```

A sample is `{ s, x, y, psi, kappa, tail, cantLds, ok }` as `crossoverRequest` makes it. Instead of `elements` the
request may name `templates` — sequences of kinds to try, every value sought —, and without either the search
tries every sequence of one to three elements it knows (`TEMPLATES`). A radius given bends either way.

The answer is `{ "solutions": [{ form, label, label2, template, s1, s2, pieces: [{ type, length, kappa }], hints,
rwMin, score }] }`, or `{ "error": "no_solution" }`, or `{ "error": "invalid_request", "detail" }`.

## Build and tests

```sh
cargo test --release
cargo clippy --all-targets
cd ../apps/layout-tool && npm run crossover   # builds the module into public/crossover/
```
