import { existsSync, readFileSync } from 'node:fs'
import { useNativeModule } from './index.js'

/** Where scripts/wasm.mjs puts the module. */
export const NATIVE_FILE = new URL('../../wasm/rail_across_core.wasm', import.meta.url)

/**
 * Load the module from its file, synchronously — the server and the tests in
 * Node, where no page renders first. Returns whether it is loaded: without the
 * file (not built) the core runs on its JavaScript.
 */
export function loadNativeFile(file = NATIVE_FILE) {
  if (!existsSync(file)) return false
  return useNativeModule(new WebAssembly.Module(readFileSync(file)))
}
