# 0019 — The optimizer in Rust, in the browser; the Python service goes

Status: **accepted** · 2026-11-03 · carries on [0018](0018-crossover-search-in-rust.md)

## Decision

1. **Everything the optimizer service computes is written again in Rust** (`optimizer/`) and runs in the browser
   as WebAssembly, in a worker, as the running time simulation and the crossover search do: optimizing a track
   (`/optimize`), the splice with a spacing to a neighbouring track (`/splice`, `clearance.py`), reconnecting
   existing elements (`/reconnect`) and the alignment and gradient fit from axis points (`/align`).
2. **The splice has one implementation, in Rust.** The Layout-Tool's own (`utils/splice/solveSplice.js`,
   decision 294 of the Layout-Tool) is replaced with it; its worker loads the module instead. The service's
   answers it is held to today (`serviceAnswers.json`) become the Rust side's test cases.
3. **The rule catalogue is the one the app bundles.** It is handed to the module with each run, not compiled
   in, so the run and the element table can never hold different rules; listing the service's catalogues
   (`/regelwerke`) and comparing their hash with the app's falls away. The app's own check
   (`packages/core/src/rules`) stays in JavaScript: Rust has its own for its searches, and the shared vectors
   (`packages/core/constraints/tests/`) hold both to the same answers, as they held JavaScript and Python.
4. **What is not computing moves to the Node server**: the Länder's DGM1 (`/terrain`, `/dem/…`: fetching tiles,
   keeping them, GeoTIFF, plane transforms) and the Access import (`/mdb`), unless a reader in JavaScript
   proves as good on real files — then it runs in the browser.
5. **Then the Python service is removed** — its unit, its venv, its step in the deploy, its route, its CI job.

It is done in steps, each deployable on its own, the Python part of each removed with it: the catalogue and
the geometry kernel; the splice, the spacing and reconnecting; optimizing; the fits; DGM1 and the Access import;
the service itself.

## Why

- In the browser the optimizer works in the local build too, needs no queue, no timeout and no second service
  to run, and a run can be followed and cancelled.
- One source of rules: the catalogue the run builds to is the one the app shows.
- Rust is quick where the Python search was slow, and the same crate can be built for a server should that
  ever be wanted.
- Fetching and converting files is not computing; the Node server does it already for other data, and one
  server is less to run than two.

## What it costs

- A Rust port of about 7,000 lines of Python, scipy's Differential Evolution, Nelder–Mead and Brent written
  for it; a run's numbers will not be scipy's to the last digit, so its tests hold properties (never worse
  than the start, every rule kept, ends fixed) where they cannot hold numbers.
- The rule check twice, in JavaScript and in Rust, held together by the shared vectors.
- A third Rust build for the deploy.

## Alternatives considered

- **Rust on the server**: keeps a second service, its queue and timeout, and the local build without the
  optimizer.
- **Keep Python**: scipy is the reason it was there; Rust has what is needed or it is short to write.
