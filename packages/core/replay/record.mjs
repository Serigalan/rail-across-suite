// The recorder's tools: a call recorded with its arguments as the JavaScript
// function takes them, the ids it drew, and what came out — the shape every
// crate from step 4 on is replayed in (crates/core-wasm/src/args.rs).

const clone = (v) => (v === undefined ? null : JSON.parse(JSON.stringify(v, (_, x) => (x instanceof Map ? Object.fromEntries(x) : x instanceof Set ? [...x] : x))))

let drawn = null
let counter = 0
const randomUUID = globalThis.crypto.randomUUID.bind(globalThis.crypto)
// Ids drawn from a counter, so a recording is the same every time and the
// replay can hand the module the very ids the JavaScript took.
globalThis.crypto.randomUUID = () => {
  const id = `00000000-0000-4000-8000-${String(++counter).padStart(12, '0')}`
  drawn?.push(id)
  return id
}
export const realRandomUUID = randomUUID

/** One call of `fn` with `args`, recorded as operation `op`. */
export function rec(op, fn, ...args) {
  const input = clone(args)
  drawn = []
  const output = clone(fn(...args))
  const ids = drawn
  drawn = null
  return { op, input, output, ...(ids.length ? { ids } : {}) }
}

/** A fixed sequence of numbers in [0, 1). */
export function rng(seed) {
  return () => {
    seed |= 0; seed = (seed + 0x6D2B79F5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
