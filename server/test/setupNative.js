import { loadNativeFile } from '@rail-across/core/src/native/node.js'

if (process.env.RA_NATIVE !== '0') loadNativeFile()
