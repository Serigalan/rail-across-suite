# optimizer — rail-across-optimizer

The Layout-Tool's track optimizer in Rust, for the browser (WebAssembly). It replaces the Python service
`services/optimizer` step by step ([decision 0019](../docs/decisions/0019-optimizer-in-rust.md)): optimizing a
track, the splice with a spacing to a neighbouring track, reconnecting existing elements, the alignment and
gradient fit from axis points.

## What is here

The foundation, ported from the service and held to what it was held to:

| Module | Python | What |
|---|---|---|
| `expr` | `ruleexpr.py` | the expression language of the rule catalogues |
| `katalog` | `katalog.py` | a catalogue read and applied: one rule to one object, one threshold |
| `pruefung` | `pruefung.py` | the catalogue applied to an element chain, as the app's element table judges it |
| `grenzen` | `grenzen.py` | the limits a run is held to at the Regelwert or the Ermessensgrenze; the shortest transitions |
| `geometry` | `geometry.py` | the geometry kernel: transitions, arcs, curve groups (simple, compound, S) |

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
- The kernel's own tests hold it to the reference values of the app it was once ported from.
