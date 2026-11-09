import { deleteCloud } from './cloudStore'

/** A worker of the import (pointCloudWorker): the Rust module reads files only there, synchronously. */
const importWorker = () => new Worker(new URL('./pointCloudWorker.js', import.meta.url), { type: 'module' })

/**
 * Starting an import from the page: one worker per import
 * (pointCloudWorker), ended once it has answered. `done` resolves to
 * `{ status: 'done', index }` or `{ status: 'aborted' }` and rejects on an
 * error; `abort()` ends the worker at once — the module in it hears no
 * message while it runs — and clears away what it had written. `original`
 * keeps every point on the file's grid and in its plane instead of thinning
 * and converting it (Entscheidung 145).
 */
export function startImport({ file, projectId, meta, sourceCrs, targetCrs, original = false, onProgress }) {
  const worker = importWorker()
  const cloudId = crypto.randomUUID()
  let settle
  const done = new Promise((resolve, reject) => {
    settle = { resolve, reject }
    worker.onmessage = ({ data }) => {
      if (data.type === 'progress') { onProgress?.(data); return }
      worker.terminate()
      if (data.type === 'done') resolve({ status: 'done', index: data.index })
      else reject(new Error(data.message))
    }
    worker.onerror = (e) => { worker.terminate(); reject(new Error(e.message || 'worker failed')) }
  })
  worker.postMessage({
    type: 'import', file, projectId: String(projectId), cloudId, meta, sourceCrs, targetCrs, original,
    base: document.baseURI,
  })
  const abort = async () => {
    worker.terminate()
    // The worker's access handle goes with it; until the browser has let go,
    // the directory may still refuse to go.
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        await deleteCloud(projectId, cloudId)
        break
      } catch (err) {
        if (err?.name === 'NotFoundError') break
        await new Promise(r => setTimeout(r, 100))
      }
    }
    settle.resolve({ status: 'aborted' })
  }
  return { cloudId, done, abort }
}

/** One question to a worker of its own, answered once: `{ type, header, … }`. */
function ask(message) {
  const worker = importWorker()
  return new Promise((resolve, reject) => {
    worker.onmessage = ({ data }) => {
      worker.terminate()
      if (data.type === message.type) resolve(data)
      else reject(new Error(data.message))
    }
    worker.onerror = (e) => { worker.terminate(); reject(new Error(e.message || 'worker failed')) }
    worker.postMessage({ ...message, base: document.baseURI })
  })
}

/** The header of a file — LAS, LAZ or E57 — read in a worker. */
export const readFileHeader = async (file) => (await ask({ type: 'header', file })).header

/** The first `count` points of a file, read in a worker: resolves to `{ header, points }`. */
export async function previewPoints(file, count) {
  const { header, points } = await ask({ type: 'preview', file, count })
  return { header, points }
}
