# optimizer — rail-across-optimizer

The Layout-Tool's track optimizer in Rust, for the browser (WebAssembly). It replaced the Python service of the
Layout-Tool ([decision 0019](../docs/decisions/0019-optimizer-in-rust.md)): optimizing a track, the splice with a
spacing to a neighbouring track, reconnecting existing elements, the alignment and gradient fit from axis points.
The Python service's tests and answers it was held to are kept here as test data (`tests/data/`).

## What is here

Ported from the service and held to what it was held to:

| Module | Python | What |
|---|---|---|
| `expr` | `ruleexpr.py` | the expression language of the rule catalogues |
| `katalog` | `katalog.py` | a catalogue read and applied: one rule to one object, one threshold |
| `pruefung` | `pruefung.py` | the catalogue applied to an element chain, as the app's element table judges it |
| `grenzen` | `grenzen.py` | the limits a run is held to at the Regelwert or the Ermessensgrenze; the shortest transitions |
| `geometry` | `geometry.py` | the geometry kernel: transitions, arcs, curve groups (simple, compound, S) |
| `splice` | `splice.py` | "Zwei Gleise zusammenfügen": every pair of ends, transitions in their modes, judged and ranked, what would fit |
| `clearance` | `clearance.py` | the spacing to a neighbouring track, widened by the cant; the largest radius that keeps it |
| `reconnect` | `reconnect.py` | "Bestehende Elemente neu verbinden": the splice nearest the old axis within a tolerance |
| `track`, `optimize` | `track_io.py`, `optimize.py`, `api.py` | a track's curve groups, the per-curve baseline, the joint search over windows; the report |
| `alignment`, `gradient` | `alignment_fit.py`, `gradient_fit.py` | an alignment, its gradient and cant from measured axis points |
| `numerics` | scipy | Nelder–Mead, Brent's bounded search and Differential Evolution as scipy has them, with a seeded generator of its own |
| `poly` | `alignment_fit.py` | points measured against a polyline, with a k-d tree |
| `rules` | — | the catalogue with its limits at both levels, built once per catalogue |

`crates/wasm` is the module the app loads (`npm run optimizer` in `apps/layout-tool`, into
`public/optimizer-module/`): one export `run` taking `{"op", "catalogue"?, "request"}` as JSON — the ops
`splice`, `reconnect`, `optimize`, `align` and `limits` (`run_json` in `lib.rs`).

The catalogue is not compiled in. The app hands over the file it bundles (`packages/core/constraints/`), so a
run builds to the rules the element table shows.

## Tests

```bash
cargo test            # in optimizer/
```

- `tests/vectors.rs` — the shared vectors of `packages/core/constraints/tests/`: the expression language, the
  catalogue check on element chains, the shortest transition lengths. The app answers the same cases
  (`packages/core/src/rules/*.vectors.test.js`).
- `tests/grenzen.rs` — the catalogue and the limits at both levels, the physics against `physics.json`: the
  cases of the service's `tests/verify.py`.
- `tests/splice.rs` — 243 splice requests and what the service answered (`tests/data/splice_answers.json`).
- `tests/clearance.rs` — the spacing: the service's `verify_clearance.py`.
- `tests/reconnect.rs` — reconnecting round trips against what the service found (`tests/data/reconnect_python.json`).
- `tests/optimize.rs` — the test tracks of the service's `verify.py`: the existing speed and the baseline exactly as
  Python found them, the joint search held to what any result has to keep (`tests/data/optimize_python.json`).
- `tests/alignment.rs` — the fits of `verify_align.py` and the app's fixture, as Python found them
  (`tests/data/align_python.json`).
- The kernel's own tests hold it to the reference values of the app it was once ported from.
