import { afterEach, describe, expect, it, vi } from 'vitest'
import { timetableApi } from './api.js'

describe('the timetables\' client', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('sends a DELETE as JSON, an empty object, as the server takes nothing else', async () => {
    const fetch = vi.fn(async () => new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetch)
    await timetableApi.remove('t1')
    const [, init] = fetch.mock.calls[0]
    expect(init.method).toBe('DELETE')
    expect(init.headers['content-type']).toBe('application/json')
    expect(init.body).toBe('{}')
  })
})
