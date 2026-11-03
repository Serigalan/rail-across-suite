import { describe, it, expect, vi, afterEach } from 'vitest'
import { optimizerReachable } from './optimizerService'

// What the service promises over the wire is checked against a running one in
// services/optimizer/tests/verify_service.py; here, that the status asks
// without ever throwing.

const json = (body, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
})

afterEach(() => { vi.unstubAllGlobals() })

describe('asking whether the service is there', () => {
  it('answers true where it is', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => json({ status: 'ok' })))
    await expect(optimizerReachable()).resolves.toBe(true)
  })

  // The panel asks this to decide whether to offer the run at all, so it may
  // not throw — a dead service is an answer, not an exception.
  it('answers false instead of throwing where it is not', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => { throw new TypeError('Failed to fetch') }))
    await expect(optimizerReachable()).resolves.toBe(false)
  })
})
