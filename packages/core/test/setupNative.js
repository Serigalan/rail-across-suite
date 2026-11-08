import { loadNativeFile } from '../src/native/node.js'

// The core's tests run on the Rust module where it is built and a function has
// moved there (decision 0020); RA_NATIVE=0 holds them to the JavaScript alone.
if (process.env.RA_NATIVE !== '0') loadNativeFile()
