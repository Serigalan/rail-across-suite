import { unloadNative } from '../src/native/index.js'
import { loadNativeFile } from '../src/native/node.js'

/**
 * `body` run once on the Rust module and once on the JavaScript — a ported
 * function held to the answers of the one it replaces. `rust` is undefined
 * where the module is not built.
 */
export function bothWays(body) {
  const native = loadNativeFile()
  const rust = native ? body() : undefined
  unloadNative()
  try {
    return { rust, js: body(), native }
  } finally {
    if (native) loadNativeFile()
  }
}
