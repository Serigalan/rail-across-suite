# optimizer — rail-across-optimizer

The Layout-Tool's track optimizer in Rust, for the browser (WebAssembly). It replaces the Python service
`services/optimizer` step by step ([decision 0019](../docs/decisions/0019-optimizer-in-rust.md)): optimizing a
track, the splice with a spacing to a neighbouring track, reconnecting existing elements, the alignment and
gradient fit from axis points.

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
| `numerics` | scipy | Nelder–Mead and Brent's bounded search as scipy has them |
| `poly` | `alignment_fit.py` | points measured against a polyline, with a k-d tree |
| `rules` | — | the catalogue with its limits at both levels, built once per catalogue |

`crates/wasm` is the module the app loads (`npm run optimizer` in `apps/layout-tool`, into
`public/optimizer-module/`): one export `run` taking `{"op", "catalogue"?, "request"}` as JSON — the ops
`splice` and `reconnect` (`run_json` in `lib.rs`).

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
- The kernel's own tests hold it to the reference values of the app it was once ported from.
