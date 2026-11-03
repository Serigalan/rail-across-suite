import { optimizerModuleUrl } from './optimizerRun'

// One worker for the optimizer, kept between requests (the module and the
// catalogue stay loaded in it). A request given up (`signal`) while the worker
// still computes it takes the worker with it — a computation cannot be
// stopped otherwise — and every other request then waiting fails with it;
// the next request starts a fresh worker.
let worker = null
const waiting = new Map()   // id → { resolve, reject }
let next = 0

/** A request the module could not answer: `code` 'optimizer_missing', 'internal' or 'invalid_request'. */
class OptimizerModuleError extends Error {
  constructor(code, detail) {
    super(detail || code)
    this.name = 'OptimizerModuleError'
    this.code = code
  }
}

const FAILURES = new Set(['optimizer_missing', 'internal', 'invalid_request'])

function stopWorker(error) {
  worker?.terminate()
  worker = null
  for (const { reject } of waiting.values()) reject(error)
  waiting.clear()
}

function startWorker() {
  worker = new Worker(new URL('./optimizerWorker.js', import.meta.url), { type: 'module' })
  worker.onmessage = ({ data: { id, answer } }) => {
    const job = waiting.get(id)
    if (!job) return
    waiting.delete(id)
    if (FAILURES.has(answer?.error)) job.reject(new OptimizerModuleError(answer.error, answer.detail))
    else job.resolve(answer)
  }
  worker.onerror = (e) => stopWorker(new OptimizerModuleError('internal', e.message || 'worker failed'))
}

/**
 * The answer of the optimizer module to `request` under `op` ('splice',
 * 'reconnect', 'optimize', 'align', 'limits'), computed in its worker.
 * Rejects with an OptimizerModuleError where the module cannot answer, with
 * an AbortError when `signal` aborts.
 */
export function runOptimizer(op, request, { signal } = {}) {
  if (typeof Worker === 'undefined') return Promise.reject(new OptimizerModuleError('optimizer_missing', 'no workers here'))
  if (signal?.aborted) return Promise.reject(new DOMException('stopped', 'AbortError'))
  if (!worker) startWorker()
  const id = ++next
  return new Promise((resolve, reject) => {
    const abort = () => {
      if (waiting.has(id)) stopWorker(new DOMException('stopped', 'AbortError'))
    }
    signal?.addEventListener('abort', abort, { once: true })
    const done = (fn) => (value) => { signal?.removeEventListener('abort', abort); fn(value) }
    waiting.set(id, { resolve: done(resolve), reject: done(reject) })
    worker.postMessage({ id, url: optimizerModuleUrl(document.baseURI), op, request })
  })
}
