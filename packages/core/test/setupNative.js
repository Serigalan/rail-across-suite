import { loadNativeFile } from '../src/native/node.js'

// Tests run on the core's Rust module (decision 0020): the functions that have
// moved there have no JavaScript any more. Every vitest of the repository that
// reaches the core loads it here (setupFiles). RA_NATIVE=0 leaves it out, for
// the core's own tests of what is still JavaScript alone.
if (process.env.RA_NATIVE !== '0' && !loadNativeFile()) {
  throw new Error('the core\'s module is not built: npm run wasm in packages/core')
}
