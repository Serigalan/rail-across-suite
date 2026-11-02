import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from './api'

describe('the project server\'s client', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('sends a DELETE as JSON, an empty object, as the server takes nothing else', async () => {
    const fetch = vi.fn(async () => new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetch)
    await api.deleteProject('p1')
    const [, init] = fetch.mock.calls[0]
    expect(init.method).toBe('DELETE')
    expect(init.headers['content-type']).toBe('application/json')
    expect(init.body).toBe('{}')
  })
})
