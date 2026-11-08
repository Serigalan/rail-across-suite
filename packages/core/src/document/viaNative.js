import { callNative, NativeError } from '../native/index.js'
import { PayloadError } from './payloadError.js'

/**
 * Operation `op` of the core's module, a PayloadError it answers with thrown
 * as the PayloadError the JavaScript would throw — so a caller cannot tell
 * which of the two answered (decision 0020).
 */
export function viaNative(op, input) {
  try {
    return callNative(op, input)
  } catch (err) {
    if (err instanceof NativeError && err.code === 'payload_error') throw new PayloadError(err.data.code, err.data.details)
    throw err
  }
}
