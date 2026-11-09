/** A worker that reads a chosen file (pointCloudWorker): the Rust module reads files only there, synchronously. */
const readWorker = () => new Worker(new URL('./pointCloudWorker.js', import.meta.url), { type: 'module' })

/**
 * A file chosen to be loaded onto the server, read in the browser only so far
 * as the form needs it: its header (format, points, box, the system it
 * states), sent along with the file, and its first points, to read the
 * coordinates before choosing their system. The tiles are the server's.
 */

/** One question to a worker of its own, answered once: `{ type, header, … }`. */
function ask(message) {
  const worker = readWorker()
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
