import { instantiateCrossover, runWith } from './crossoverRun'

// One search, off the page's thread: the module fetched from `url`, the
// request run through it, the answer posted back — or { error } with a code
// the dialog translates: `crossover_missing` where the module is not there.

self.onmessage = async ({ data: { url, request } }) => {
  let exports
  try {
    const response = await fetch(url)
    if (!response.ok) throw new Error(String(response.status))
    exports = await instantiateCrossover(await response.arrayBuffer())
  } catch (err) {
    self.postMessage({ error: 'crossover_missing', detail: err.message })
    return
  }
  try {
    self.postMessage(runWith(exports, request))
  } catch (err) {
    self.postMessage({ error: 'internal', detail: err.message })
  }
}
