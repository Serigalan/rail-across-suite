// native — the core's Rust crates as one WebAssembly module (decision 0020):
// the face through which a module of the core that has moved to Rust is
// called. The module is loaded once — in the browser before the first page
// renders (loadNative), in Node from its file (./node.js) — and from then on
// every call is synchronous, as the core's calls always were. Where it is not
// loaded, a ported function stays with its JavaScript (nativeReady), so the
// core works the same either way while the port goes on.
//
// This file is the module's public face: code outside packages/core/src/native/
// imports from here and nowhere else (tools/modules.mjs). See docs/native.md.

let exports_ = null
const encoder = new TextEncoder()
const decoder = new TextDecoder()

/** What the module answered instead of a result: its code, what it found, and what rebuilds the error. */
export class NativeError extends Error {
  constructor(code, detail, data) {
    super(detail ? `${code}: ${detail}` : code)
    this.code = code
    this.detail = detail ?? null
    this.data = data ?? null
  }
}

/** Whether the module is loaded, so a ported function runs in Rust. */
export const nativeReady = () => exports_ != null

/** The module's id generator seeded with this device's randomness, as crypto.randomUUID draws it. */
function seed() {
  const words = new Uint32Array(4)
  globalThis.crypto?.getRandomValues?.(words)
  callNative('core.seed', [...words])
}

/** The module's instance taken as the face's, from a compiled WebAssembly.Module. */
export function useNativeModule(module) {
  exports_ = new WebAssembly.Instance(module, {}).exports
  seed()
  return true
}

/**
 * Load the module from a URL, a Response, bytes or a compiled module — what a
 * page does once before it renders. Resolves to whether it is loaded; a page
 * without it runs on the JavaScript.
 */
export async function loadNative(source) {
  if (exports_) return true
  try {
    let module
    if (source instanceof WebAssembly.Module) module = source
    else if (source instanceof ArrayBuffer || ArrayBuffer.isView(source)) module = await WebAssembly.compile(source)
    else {
      const response = source instanceof Response ? source : await fetch(source)
      module = await WebAssembly.compile(await response.arrayBuffer())
    }
    exports_ = (await WebAssembly.instantiate(module, {})).exports
    seed()
    return true
  } catch {
    return false
  }
}

/** Forget the module (tests that hold the JavaScript to the same answers). */
export function unloadNative() {
  exports_ = null
}

/** As the replay recorded a call's arguments: a Map as an object, a Set as an array. */
const mapsAndSets = (_, x) => (x instanceof Map ? Object.fromEntries(x) : x instanceof Set ? [...x] : x)

/** Operation `op` of the module on `input` (JSON-able); its result, or a NativeError. */
export function callNative(op, input = null) {
  if (!exports_) throw new NativeError('native_not_loaded')
  const { memory, alloc, dealloc, run, result_len: resultLen } = exports_
  const bytes = encoder.encode(JSON.stringify({ op, input }, mapsAndSets))
  const ptr = alloc(bytes.length)
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes)
  const out = run(ptr, bytes.length)
  const len = resultLen()
  const text = decoder.decode(new Uint8Array(memory.buffer, out, len))
  dealloc(out, len)
  const answer = JSON.parse(text)
  if (answer && 'ok' in answer) return answer.ok
  throw new NativeError(answer?.error ?? 'invalid_answer', answer?.detail, answer?.data)
}

/**
 * A function of the core that has moved to Rust (decision 0020, step 10):
 * called with its arguments as the JavaScript took them — operation `op`
 * takes them as the replay recorded them, positional — and answering what the
 * JavaScript answered, as JSON. `revive(result, args)` gives back what JSON
 * does not carry: a Map, a Set, a typed array.
 */
export function viaRust(op, revive = null) {
  const call = (args) => {
    try {
      return callNative(op, args)
    } catch (err) {
      // What the JavaScript threw comes back as such: an Error with its message.
      if (err instanceof NativeError && err.code === 'thrown') throw Object.assign(new Error(err.detail ?? 'thrown'), { data: err.data })
      throw err
    }
  }
  return revive ? (...args) => revive(call(args), args) : (...args) => call(args)
}

/**
 * `out`, or `given` itself where the two are the same as JSON — and of an
 * object, each member of `given` that is the same: the JavaScript answered
 * what it left alone as it was, and identity checks and memos rely on that.
 */
export function keepSame(out, given) {
  if (out === given) return given
  const json = (v) => JSON.stringify(v)
  if (json(out) === json(given)) return given
  if (out && given && typeof out === 'object' && typeof given === 'object' && !Array.isArray(out)) {
    const kept = { ...out }
    for (const k of Object.keys(kept)) if (k in given && json(kept[k]) === json(given[k])) kept[k] = given[k]
    return kept
  }
  return out
}

/**
 * `out` with every object or array that is the same, as JSON, as one of the
 * arguments' — the arguments themselves, their members, the items of those
 * and their members — replaced by that very object: what the JavaScript
 * passed through untouched comes back as it was, for identity checks and the
 * page's memos.
 */
export function reuseFrom(out, args) {
  const known = new Map()
  // What holds a member set to undefined anywhere says more than its JSON (`'heights' in track`): its
  // key keeps the undefined, which no answer of the module has, so it is never taken.
  const keyOf = (v) => JSON.stringify(v, (_, x) => (x === undefined ? '\u0000undefined' : x))
  const note = (v, depth) => {
    if (!v || typeof v !== 'object' || ArrayBuffer.isView(v)) return
    const key = keyOf(v)
    if (!known.has(key)) known.set(key, v)
    if (depth <= 0) return
    for (const c of Array.isArray(v) ? v : Object.values(v)) note(c, depth - 1)
  }
  for (const a of args) note(a, 3)
  const walk = (v, depth) => {
    if (!v || typeof v !== 'object') return v
    const same = known.get(JSON.stringify(v))
    if (same) return same
    if (depth <= 0) return v
    if (Array.isArray(v)) return v.map(c => walk(c, depth - 1))
    return Object.fromEntries(Object.entries(v).map(([k, c]) => [k, walk(c, depth - 1)]))
  }
  return walk(out, 4)
}

/** A JSON object as the Map the JavaScript answered with. */
export const asMap = (object) => new Map(Object.entries(object ?? {}))
