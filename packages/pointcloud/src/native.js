// The point cloud import in Rust (crates/pointcloud-wasm; decision 0005): LAS, LAZ and E57 read, converted to the project's plane and tiled
// as the server's jobs do. The module is loaded once — in the browser's import
// worker (loadPointCloudModule), in Node from its file (./nativeNode.js) — and
// from then on every call is synchronous: the file is read through
// `source.readSync(offset, length)` (a worker's FileReaderSync, bytes in a
// test) and each level's tiles go to `writers[level].append(bytes)` (an OPFS
// sync access handle), which answers where it put them. `bytes` is a view into
// the module's memory, valid during the call only.

let exports_ = null
let current = null
const decoder = new TextDecoder()
const encoder = new TextEncoder()

const memory = () => exports_.memory.buffer

/** What the module calls back into: the call's file, tiles, progress, and the clock. */
const imports = {
  ra: {
    read(offset, length, ptr) {
      const bytes = current.source.readSync(offset, length)
      new Uint8Array(memory(), ptr, bytes.length).set(bytes)
      return bytes.length
    },
    write(level, ptr, length) {
      const writer = current.writers[level]
      return writer ? writer.append(new Uint8Array(memory(), ptr, length)) : -1
    },
    progress(ptr, length) {
      current.onProgress?.(JSON.parse(decoder.decode(new Uint8Array(memory(), ptr, length))))
    },
    now: () => performance.now(),
  },
}

/** Whether the module is loaded. */
export const pointCloudModuleReady = () => exports_ != null

/** The module's instance, from a compiled WebAssembly.Module. */
export function usePointCloudModule(module) {
  exports_ = new WebAssembly.Instance(module, imports).exports
  return true
}

/** Load the module from a URL, a Response, bytes or a compiled module. */
export async function loadPointCloudModule(source) {
  if (exports_) return true
  let module
  if (source instanceof WebAssembly.Module) module = source
  else if (source instanceof ArrayBuffer || ArrayBuffer.isView(source)) module = await WebAssembly.compile(source)
  else {
    const response = source instanceof Response ? source : await fetch(source)
    if (!response.ok) throw new Error(`the point cloud module: ${response.status} ${response.statusText}`)
    module = await WebAssembly.compile(await response.arrayBuffer())
  }
  return usePointCloudModule(module)
}

/** Operation `op` of the module on `input`, with the call's file, tiles and progress. */
function run(op, input, call = {}) {
  if (!exports_) throw new Error('the point cloud module is not loaded')
  const { alloc, dealloc, run: runOp, result_len: resultLen } = exports_
  const bytes = encoder.encode(JSON.stringify({ op, input }))
  const ptr = alloc(bytes.length)
  new Uint8Array(memory(), ptr, bytes.length).set(bytes)
  current = call
  let out
  try {
    out = runOp(ptr, bytes.length)
  } finally {
    current = null
  }
  const len = resultLen()
  const answer = JSON.parse(decoder.decode(new Uint8Array(memory(), out, len)))
  dealloc(out, len)
  if (answer && 'ok' in answer) return answer.ok
  throw new Error(answer?.error ?? 'the point cloud module gave no answer')
}

/** The header of the file behind `source` — LAS, LAZ or E57, told apart by its first bytes. */
export const readCloudHeader = (source) => run('header', { size: source.size }, { source })

/** The first `n` points of the file, as `[{ x, y, z, intensity }]` (lasText). */
export const readFirstPoints = (source, n) => run('firstPoints', { size: source.size, n }, { source })

/** A grid proj4js has read (`{ key, subgrids }`), for converting on it. */
export const loadPointCloudGrid = (grid) => run('loadGrid', grid)

/**
 * The levels `levels` of the file in one pass, each into `writers[level]`, the
 * points converted from `sourceCrs` to `targetCrs` (the original level stays in
 * the file's plane). `onProgress({ bytes, totalBytes, points, totalPoints,
 * kept, bytesWritten, elapsed, remaining })` a few times a second. Returns each
 * level's index body, by level.
 */
export function importLevels({ source, levels, writers, sourceCrs = 0, targetCrs = sourceCrs, idlePoints, onProgress }) {
  return run('importLevels', { size: source.size, levels, sourceCrs: Number(sourceCrs), targetCrs: Number(targetCrs), idlePoints }, {
    source, writers, onProgress,
  })
}

/** One level of the file — the original (0) or the first voxel level (1) — into `writer`; its index body. */
export function importPointCloud({ writer, original = false, ...rest }) {
  const level = original ? 0 : 1
  return importLevels({ ...rest, levels: [level], writers: { [level]: writer } })[level]
}
