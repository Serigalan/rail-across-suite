/**
 * The track optimizer: rail-across-optimizer (optimizer/ of the repository,
 * decision 0019), built to WebAssembly and put beside the page
 * (`npm run optimizer`). It takes one message's JSON — { op, catalogue?,
 * request } — and gives back the answer's. The catalogue is the one the app
 * bundles; the module keeps it once handed over.
 */

/** Where the module lies, beside the page. */
export const optimizerModuleUrl = (base) => new URL('optimizer-module/rail_across_optimizer.wasm', base).href

/** The module's exports, from its bytes. */
export async function instantiateOptimizer(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes)
  return instance.exports
}

/** Runs one message through a module's exports: its JSON in, the answer's out. */
export function runWith(exports, message) {
  const { memory, alloc, dealloc, run, result_len } = exports
  const input = new TextEncoder().encode(JSON.stringify(message))
  const ptr = alloc(input.length)
  new Uint8Array(memory.buffer, ptr, input.length).set(input)
  const out = run(ptr, input.length)
  const len = result_len()
  const json = new TextDecoder().decode(new Uint8Array(memory.buffer, out, len))
  dealloc(out, len)
  return JSON.parse(json)
}
