import { loadNativeFile } from '@rail-across/core/native/node'

if (process.env.RA_NATIVE !== '0') loadNativeFile()
