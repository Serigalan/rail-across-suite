import { existsSync, readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { KATALOG } from '@rail-across/core/rules'
import { runWith } from '../utils/optimizer/optimizerRun'

// The optimizer module (optimizer/ of the repository, decision 0019) for the
// tests, as cargo built it — `built` false where it has not been, and the
// tests that need it are skipped. `optimizer(op, request)` answers in place,
// the catalogue handed over with every request.

const MODULE = fileURLToPath(new URL(
  '../../../../../optimizer/target/wasm32-unknown-unknown/release/rail_across_optimizer_wasm.wasm', import.meta.url))

export const built = existsSync(MODULE)

let exports = null
if (built) {
  const { instance } = await WebAssembly.instantiate(readFileSync(MODULE))
  exports = instance.exports
}

const optimizer = (op, request) => runWith(exports, { op, request, catalogue: KATALOG })

/** The splice of `request`, as the dialog's worker answers it. */
export const solveSplice = (request) => optimizer('splice', request)
