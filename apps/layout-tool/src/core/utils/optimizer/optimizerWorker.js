import { KATALOG } from '@rail-across/core/rules'
import { instantiateOptimizer, runWith } from './optimizerRun'

// The optimizer off the page's thread, kept between requests: the module
// fetched from `url` the first time, the catalogue handed over with the
// first request, every request after it run through the same instance. An
// answer is the module's — or { error } with a code the job turns into a
// failure: `optimizer_missing` where the module is not there, `internal`
// where it broke off (the instance is then dropped and the next request
// loads it again).

let exports = null
let handedOver = false

async function load(url) {
  const response = await fetch(url)
  if (!response.ok) throw new Error(String(response.status))
  handedOver = false
  return instantiateOptimizer(await response.arrayBuffer())
}

self.onmessage = async ({ data: { id, url, op, request } }) => {
  try {
    exports ??= await load(url)
  } catch (err) {
    self.postMessage({ id, answer: { error: 'optimizer_missing', detail: err.message } })
    return
  }
  try {
    const answer = runWith(exports, { op, request, catalogue: handedOver ? undefined : KATALOG })
    handedOver = true
    self.postMessage({ id, answer })
  } catch (err) {
    exports = null
    self.postMessage({ id, answer: { error: 'internal', detail: err.message } })
  }
}
