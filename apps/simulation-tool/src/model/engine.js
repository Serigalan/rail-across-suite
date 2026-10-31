/**
 * The running time simulation: rail-across-sim, OSRD's simulation core
 * ported to Rust and built to WebAssembly (LGPL-3.0, its own repository) —
 * a module beside the page, loaded the first time it is needed. It takes a
 * request's JSON and gives back the result's, or { error: { code, message } }.
 */

let loading = null

const moduleUrl = () => new URL('sim/rail_across_sim.wasm', globalThis.document?.baseURI ?? globalThis.location?.href)

async function instantiate(url = moduleUrl()) {
  const response = await fetch(url)
  if (!response.ok) throw Object.assign(new Error('sim_missing'), { code: 'sim_missing' })
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer())
  return instance.exports
}

/** The module's exports; one load for the page, again after a failure. */
function exportsOf() {
  loading ??= instantiate().catch(err => { loading = null; throw err })
  return loading
}

/** Runs one request through a module's exports: its JSON in, the result's out. */
export function runWith(exports, request) {
  const { memory, alloc, dealloc, simulate, result_len } = exports
  const input = new TextEncoder().encode(JSON.stringify(request))
  const ptr = alloc(input.length)
  new Uint8Array(memory.buffer, ptr, input.length).set(input)
  const out = simulate(ptr, input.length)
  const len = result_len()
  const json = new TextDecoder().decode(new Uint8Array(memory.buffer, out, len))
  dealloc(out, len)
  return JSON.parse(json)
}

/**
 * Simulates a request. A trap in the module (an internal error) leaves it
 * unusable: it is loaded again for the next run, and this one answers with
 * an error.
 */
export async function simulate(request) {
  const exports = await exportsOf()
  try {
    return runWith(exports, request)
  } catch (err) {
    if (err instanceof WebAssembly.RuntimeError) {
      loading = null
      return { error: { code: 'internal', message: err.message } }
    }
    throw err
  }
}
