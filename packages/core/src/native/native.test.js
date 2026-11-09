import { describe, expect, it } from 'vitest'
import { callNative, nativeReady, NativeError } from './index.js'
import { loadNativeFile } from './node.js'

describe.skipIf(!loadNativeFile())('the core\'s WebAssembly module', () => {
  it('is loaded and answers', () => {
    expect(nativeReady()).toBe(true)
    expect(callNative('version')).toMatchObject({ module: 'rail-across-core' })
  })

  it('says what it does not know as an error of its own', () => {
    expect(() => callNative('nonsense')).toThrow(NativeError)
    try { callNative('nonsense') } catch (err) { expect(err.code).toBe('unknown_op') }
  })
})
