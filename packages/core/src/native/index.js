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

/** Operation `op` of the module on `input` (JSON-able); its result, or a NativeError. */
export function callNative(op, input = null) {
  if (!exports_) throw new NativeError('native_not_loaded')
  const { memory, alloc, dealloc, run, result_len: resultLen } = exports_
  const bytes = encoder.encode(JSON.stringify({ op, input }))
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
