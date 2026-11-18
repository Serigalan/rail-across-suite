import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { setup, signIn } from './helpers.js'

let ctx, dir
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
  if (dir) rmSync(dir, { recursive: true, force: true })
  dir = null
})

describe('OSRD nets', () => {
  it('are the RailJSON files laid into the directory, listed and handed out to whoever is signed in', async () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-osrd-'))
    const infra = { version: '3.5.3', track_sections: [{ id: 't', length: 10 }] }
    writeFileSync(join(dir, 'morocco.railjson.json'), JSON.stringify(infra))
    writeFileSync(join(dir, 'Alpha.json'), '{}')
    writeFileSync(join(dir, 'notes.txt'), 'not a net')
    writeFileSync(join(dir, '.hidden.json'), '{}')
    ctx = await setup({ env: { RA_SERVER_OSRD: dir } })
    const max = await signIn(ctx.app, 'max')

    const listed = (await max('GET', '/api/osrd')).json().nets
    expect(listed.map(n => [n.id, n.name])).toEqual([['Alpha', 'Alpha'], ['morocco.railjson', 'morocco']])
    expect(listed[1]).toMatchObject({ size: JSON.stringify(infra).length })

    const got = await max('GET', '/api/osrd/morocco.railjson')
    expect(got.statusCode).toBe(200)
    expect(got.headers['content-type']).toMatch(/^application\/json/)
    expect(got.json()).toEqual(infra)

    expect((await max('GET', '/api/osrd/notes')).statusCode).toBe(404)
    expect((await max('GET', '/api/osrd/..%2Fmorocco.railjson')).statusCode).toBe(404)
    expect((await ctx.app.inject({ url: '/api/osrd' })).statusCode).toBe(401)
  })

  it('are none where the directory is not there', async () => {
    ctx = await setup({ env: { RA_SERVER_OSRD: join(tmpdir(), 'ra-osrd-none-here') } })
    const max = await signIn(ctx.app, 'max')
    expect((await max('GET', '/api/osrd')).json()).toEqual({ nets: [] })
  })
})
