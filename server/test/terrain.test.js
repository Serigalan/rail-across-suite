import { afterEach, describe, expect, it } from 'vitest'
import { setup, signIn } from './helpers.js'

// The terrain and the MDB import over HTTP; their pieces — the DGM1 tiles, the
// tile store, heights and relief tiles, mdb-export's CSV — are the crate's
// own tests (crates/server/tests/terrain.rs).

let ctx
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
})

describe('the terrain and the MDB conversion', () => {
  it('takes an Access file as bytes, and refuses one that is none', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const post = (headers) => ctx.app.inject({
      method: 'POST', url: '/api/mdb', cookies: { ra_session: max.token }, payload: Buffer.from('not a database at all'), headers,
    })
    expect((await post({ 'content-type': 'application/octet-stream' })).statusCode).toBe(415)
    const res = await post({ 'content-type': 'application/octet-stream', 'x-olt-upload': '1' })
    expect([res.statusCode, res.json().error]).toEqual([400, 'not_a_database'])
  })

  it('asks for a session', async () => {
    ctx = await setup()
    const res = await ctx.app.inject({ method: 'POST', url: '/api/terrain', payload: { lnglat: [[11, 51]] } })
    expect(res.statusCode).toBe(401)
    const bad = await (await signIn(ctx.app, 'max'))('GET', '/api/dem/12/0/0.png')
    expect(bad.statusCode).toBe(400)
  })
})
