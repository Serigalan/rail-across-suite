/**
 * The search of the extended switch connection: rail-across-crossover
 * (crossover/ of the repository, decision 0018), built to WebAssembly and
 * put beside the page (`npm run crossover`). It takes the request's JSON —
 * crossoverRequest of the core, with the elements or sequences asked for —
 * and gives back the answer's: { solutions } best first, or { error }.
 */

/** Where the module lies, beside the page. */
export const crossoverModuleUrl = (base) => new URL('crossover/rail_across_crossover.wasm', base).href

/** The module's exports, from its bytes. */
export async function instantiateCrossover(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes)
  return instance.exports
}

/** Runs one request through a module's exports: its JSON in, the answer's out. */
export function runWith(exports, request) {
  const { memory, alloc, dealloc, solve, result_len } = exports
  const input = new TextEncoder().encode(JSON.stringify(request))
  const ptr = alloc(input.length)
  new Uint8Array(memory.buffer, ptr, input.length).set(input)
  const out = solve(ptr, input.length)
  const len = result_len()
  const json = new TextDecoder().decode(new Uint8Array(memory.buffer, out, len))
  dealloc(out, len)
  return JSON.parse(json)
}
