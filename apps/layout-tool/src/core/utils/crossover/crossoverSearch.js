import { crossoverModuleUrl } from './crossoverRun'

/**
 * Searches an extended switch connection in a worker of its own, ended once
 * it has answered: resolves to the module's answer ({ solutions } or
 * { error }). `signal` stops it — the worker is ended where it stands — and
 * the promise rejects with an AbortError.
 */
export function searchCrossover(request, { signal } = {}) {
  const worker = new Worker(new URL('./crossoverWorker.js', import.meta.url), { type: 'module' })
  return new Promise((resolve, reject) => {
    const abort = () => {
      worker.terminate()
      reject(new DOMException('aborted', 'AbortError'))
    }
    if (signal?.aborted) { abort(); return }
    signal?.addEventListener('abort', abort, { once: true })
    const done = () => { worker.terminate(); signal?.removeEventListener('abort', abort) }
    worker.onmessage = ({ data }) => { done(); resolve(data) }
    worker.onerror = (e) => { done(); resolve({ error: 'internal', detail: e.message }) }
    worker.postMessage({ url: crossoverModuleUrl(document.baseURI), request })
  })
}
