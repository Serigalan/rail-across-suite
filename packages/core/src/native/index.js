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
let module_ = null
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
  module_ = module
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
    module_ = module
    seed()
    return true
  } catch {
    return false
  }
}

/**
 * The compiled module, null where none is loaded — what a page hands a worker
 * (postMessage carries it), which loads it with loadNative before it calls the core.
 */
export const nativeModule = () => module_

/** Forget the module (tests that hold the JavaScript to the same answers). */
export function unloadNative() {
  exports_ = null
  module_ = null
  held = null
  heldNames.clear()
  answers = new WeakMap()
}

/** As the replay recorded a call's arguments: a Map as an object, a Set as an array. */
const mapsAndSets = (_, x) => (x instanceof Map ? Object.fromEntries(x) : x instanceof Set ? [...x] : x)

// ── The project the module holds (decision 0021, phase C) ───────────────────
//
// The page hands the module the project it has open once (holdProject) and,
// after every step, only what the step changed — by reference: the record is
// immutable, so a member or a track that is the same object is the same.
// From then on an argument that *is* one of the held project's members (its
// tracks, its switches, …), an item of one (a track), a member of that (its
// elements) or the project itself, crosses as a marker (`{"$held": "tracks"}`,
// `{"$held": ["tracks", 3]}`, `{"$held": ["tracks", 3, "elements"]}`) and
// the module lends the operation what it holds there, instead of the page
// carrying it across as JSON on every call.

let held = null
const heldNames = new Map()
// What the module answered on the held project and still keeps, by the page's
// object it became: written back, it is named (`{"$a": [answer, i]}`), not sent.
let answers = new WeakMap()

/** `item` is item `i` of answer `answer` of an operation on the held project (session ops). */
export function answeredBy(item, answer, i) {
  if (item && typeof item === 'object') answers.set(item, [answer, i])
}

/**
 * What changed from `before` to `after`: members set and members removed; of a
 * changed array, the items it kept named by their place in the held one and
 * (`named`) those the module answered named by their answer.
 */
function patchOf(before, after, named = true) {
  const patch = {}
  const removed = []
  for (const k of Object.keys(before)) if (!(k in after) || after[k] === undefined) removed.push(k)
  for (const [k, v] of Object.entries(after)) {
    if (v === undefined || v === before[k]) continue
    const old = before[k]
    if (Array.isArray(v) && Array.isArray(old) && old.length) {
      const at = new Map(old.map((item, i) => [item, i]))
      let kept = 0
      const items = v.map((item) => {
        if (!item || typeof item !== 'object') return item
        const i = at.get(item)
        if (i !== undefined) { kept++; return { $k: i } }
        const a = named && answers.get(item)
        if (a) { kept++; return { $a: a } }
        return item
      })
      patch[k] = kept ? { $items: items } : v
    } else {
      patch[k] = v
    }
  }
  return { patch, removed }
}

/**
 * Hold `project` in the module — the record the page has open, or null to let
 * it go. After the first, a call writes only what changed since the last.
 */
export function holdProject(project) {
  if (!exports_) return
  if (project === held) return
  if (!project) {
    callNative('session.close', [])
  } else if (!held) {
    callNative('session.open', [project])
  } else {
    try {
      const { patch, removed } = patchOf(held, project)
      callNative('session.write', [patch, removed])
    } catch (err) {
      if (!(err instanceof NativeError && err.code === 'session_answer_gone')) throw err
      const { patch, removed } = patchOf(held, project, false)
      callNative('session.write', [patch, removed])
    }
  }
  held = project ?? null
  heldNames.clear()
  if (held) {
    heldNames.set(held, '')
    for (const [k, v] of Object.entries(held)) {
      if (!v || typeof v !== 'object') continue
      heldNames.set(v, k)
      // An item of a collection — a track, a switch — is named by its place, and its members by theirs.
      if (!Array.isArray(v)) continue
      v.forEach((item, i) => {
        if (!item || typeof item !== 'object' || heldNames.has(item)) return
        heldNames.set(item, [k, i])
        if (Array.isArray(item)) return
        for (const [m, x] of Object.entries(item)) if (x && typeof x === 'object' && !heldNames.has(x)) heldNames.set(x, [k, i, m])
      })
    }
  }
}

/** The project the module holds, as the page last handed it (null for none). */
export const heldProject = () => held

/** Arguments with what is held replaced by its marker — an argument, its members and theirs. */
function marked(v, depth) {
  if (!v || typeof v !== 'object') return v
  const name = heldNames.get(v)
  if (name !== undefined) return { $held: name }
  if (depth >= 3 || v instanceof Map || v instanceof Set || ArrayBuffer.isView(v)) return v
  if (Array.isArray(v)) {
    let changed = false
    const out = v.map((x) => { const m = marked(x, depth + 1); if (m !== x) changed = true; return m })
    return changed ? out : v
  }
  if (Object.getPrototypeOf(v) !== Object.prototype) return v
  let out = null
  for (const [k, x] of Object.entries(v)) {
    const m = marked(x, depth + 1)
    if (m !== x) (out ??= { ...v })[k] = m
  }
  return out ?? v
}

/** An input given as an object: its members marked as arguments are. */
function markedMembers(input) {
  let out = null
  for (const [k, x] of Object.entries(input)) {
    const m = marked(x, 1)
    if (m !== x) (out ??= { ...input })[k] = m
  }
  return out ?? input
}

/** Operation `op` of the module on `input` (JSON-able); its result, or a NativeError. */
export function callNative(op, input = null) {
  if (!exports_) throw new NativeError('native_not_loaded')
  const { memory, alloc, dealloc, run, result_len: resultLen } = exports_
  const given = !held || op.startsWith('session.') ? input
    : Array.isArray(input) ? input.map(a => marked(a, 1))
      : input && Object.getPrototypeOf(input) === Object.prototype ? markedMembers(input) : input
  const bytes = encoder.encode(JSON.stringify({ op, input: given }, mapsAndSets))
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
  // A long array is not keyed as a whole, nor are its items one by one — that
  // would be the project's JSON on every call —: an object of the answer with
  // the id of one of its items is held against that item alone, and an array
  // of the answer that holds the very items of one, in order, is that array.
  const long = []
  const byId = new Map()
  const note = (v, depth) => {
    if (!v || typeof v !== 'object' || ArrayBuffer.isView(v)) return
    if (Array.isArray(v) && v.length > LONG) {
      long.push(v)
      for (const item of v) {
        const id = idOf(item)
        if (id !== undefined && !byId.has(id)) byId.set(id, item)
      }
      return
    }
    const key = keyOf(v)
    if (!known.has(key)) known.set(key, v)
    if (depth <= 0) return
    for (const c of Array.isArray(v) ? v : Object.values(v)) note(c, depth - 1)
  }
  // A later argument first: of a state before and one after that are the same, it is the one after
  // that an operation answers when it leaves it as it is.
  for (const a of [...args].reverse()) note(a, 3)
  const walk = (v, depth) => {
    if (!v || typeof v !== 'object') return v
    if (Array.isArray(v) && v.length > LONG) {
      // A list of points or numbers is the answer's own; a list of objects may hold the arguments' items.
      if (depth <= 0 || !v[0] || typeof v[0] !== 'object' || Array.isArray(v[0])) return v
      const items = v.map(c => walk(c, depth - 1))
      return long.find(a => a.length === items.length && a.every((x, i) => x === items[i])) ?? items
    }
    const text = JSON.stringify(v)
    const same = known.get(text)
    if (same) return same
    const id = Array.isArray(v) ? undefined : idOf(v)
    const was = id === undefined ? undefined : byId.get(id)
    if (was) return text === keyOf(was) ? was : alike(v, was)
    if (depth <= 0) return v
    if (Array.isArray(v)) return v.map(c => walk(c, depth - 1))
    return Object.fromEntries(Object.entries(v).map(([k, c]) => [k, walk(c, depth - 1)]))
  }
  return walk(out, 4)
}

/** What an item of a collection goes by: its id — a switch's `switchId` —, or undefined. */
const idOf = (v) => (v && typeof v === 'object' && !Array.isArray(v) ? (v.id ?? v.switchId) : undefined)

/**
 * `v` — an object of an answer, not the same as `was`, the argument's item of
 * its id — with each member that is the same as `was`'s that very member, and
 * of a list of objects of the same length (a track's elements) each item that
 * is the same.
 */
function alike(v, was) {
  const kept = {}
  for (const [k, x] of Object.entries(v)) {
    const w = was[k]
    if (!x || typeof x !== 'object' || !w || typeof w !== 'object') kept[k] = x
    else if (Array.isArray(x) && Array.isArray(w) && x.length === w.length && x.length && x[0] && typeof x[0] === 'object' && !Array.isArray(x[0])) {
      const items = x.map((e, i) => (JSON.stringify(e) === keyOf(w[i]) ? w[i] : e))
      kept[k] = items.every((e, i) => e === w[i]) ? w : items
    } else kept[k] = !(Array.isArray(x) && x.length > LONG) && JSON.stringify(x) === keyOf(w) ? w : x
  }
  const keys = Object.keys(kept)
  return keys.length === Object.keys(was).length && keys.every(k => kept[k] === was[k]) ? was : kept
}

const LONG = 16

// What holds a member set to undefined anywhere says more than its JSON (`'heights' in track`): its
// key keeps the undefined, which no answer of the module has, so it is never taken. A frozen
// object does not change — the page's record is frozen as it is written —, so its key is worked
// out once.
const keys = new WeakMap()
function keyOf(v) {
  let key = keys.get(v)
  if (key === undefined) {
    key = JSON.stringify(v, (_, x) => (x === undefined ? '\u0000undefined' : x))
    if (Object.isFrozen(v)) keys.set(v, key)
  }
  return key
}

/**
 * The answer's tracks `out` with each that is the same, as JSON, as the track
 * of its id in `tracks` replaced by that very track — and of one that is not,
 * each member and each element that is the same (as reuseFrom does for a long
 * list) — at the cost of the tracks the answer holds, not the project's.
 */
export function reuseTracks(out, tracks) {
  const before = new Map(tracks.map(t => [t.id, t]))
  return out.map((t) => {
    const was = before.get(t?.id)
    if (!was) return t
    return JSON.stringify(t) === keyOf(was) ? was : alike(t, was)
  })
}

/** A JSON object as the Map the JavaScript answered with. */
export const asMap = (object) => new Map(Object.entries(object ?? {}))
