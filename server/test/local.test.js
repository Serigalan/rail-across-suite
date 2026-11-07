import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { parseProjectsPayload } from '@rail-across/core/project'
import { openDatabase } from '../src/db.js'
import { buildApp } from '../src/app.js'
import { as } from './helpers.js'

let ctx
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  if (ctx?.dir) rmSync(ctx.dir, { recursive: true, force: true })
  ctx = null
})

/** A server in the local mode of the local package, on a database in memory. */
async function startLocal(staticRoot = null) {
  const db = openDatabase(':memory:')
  const app = buildApp({ db, local: true, secureCookie: false, staticRoot })
  await app.ready()
  ctx = { app, db, dir: staticRoot }
  return as(app, null)
}

/** A site directory as the desktop package builds it. */
function makeSite() {
  const dir = mkdtempSync(join(tmpdir(), 'ra-local-site-'))
  writeFileSync(join(dir, 'index.html'), '<html>suite</html>')
  mkdirSync(join(dir, 'layout'))
  writeFileSync(join(dir, 'layout', 'index.html'), '<html>layout</html>')
  mkdirSync(join(dir, 'layout', 'assets'))
  writeFileSync(join(dir, 'layout', 'assets', 'app.js'), 'console.log(1)')
  writeFileSync(join(dir, 'data.bin'), Buffer.from('0123456789'))
  return dir
}

describe('the local package (serve --local)', () => {
  it('answers /api/me with the one implicit local user, without a session', async () => {
    const call = await startLocal()
    const res = await call('GET', '/api/me')
    expect(res.statusCode).toBe(200)
    expect(res.json().user).toMatchObject({ login: 'local', role: 'admin', local: true, mustChangePassword: false })
    // Exactly one user row, made once — the records' author references stand on it.
    expect(ctx.db.prepare('SELECT COUNT(*) FROM user').pluck().get()).toBe(1)
    const again = await call('GET', '/api/me')
    expect(again.json().user.id).toBe(res.json().user.id)
    expect(ctx.db.prepare('SELECT COUNT(*) FROM user').pluck().get()).toBe(1)
  })

  it('has no user management: no sign-in, no password, no admin or user routes', async () => {
    const call = await startLocal()
    for (const [method, url] of [
      ['POST', '/api/login'], ['POST', '/api/logout'], ['POST', '/api/me/password'],
      ['GET', '/api/admin/users'], ['POST', '/api/admin/users'], ['GET', '/api/users?q=lo'],
    ]) {
      const res = await call(method, url, method === 'GET' ? undefined : { login: 'x', password: 'y' })
      expect(res.statusCode).toBe(404)
    }
    expect((await call('GET', '/api/health')).json()).toEqual({ status: 'ok', local: true })
  })

  it('works on projects without a session — create, list, members', async () => {
    const call = await startLocal()
    const made = await call('POST', '/api/projects', { title: 'Halle–Könnern', payload: { tracks: [], switches: [], platforms: [] } })
    expect(made.statusCode).toBe(201)
    const { project, variantId } = made.json()
    expect(project.createdBy).toMatchObject({ login: 'local' })
    expect((await call('GET', '/api/projects')).json().projects.map(p => p.id)).toEqual([project.id])
    const members = await call('GET', `/api/projects/${project.id}/members`)
    expect(members.json()).toEqual({ members: [], canManage: true })
    // A check-in on the variant's head goes through, as the local admin.
    const h = (await call('GET', `/api/variants/${variantId}/head`)).json()
    const record = parseProjectsPayload(h.payload).projects[0]
    const checkIn = await call('POST', `/api/variants/${variantId}/revisions`, {
      base: h.revision.id, message: 'lokal', payload: { ...record, description: 'lokal' },
    })
    expect(checkIn.statusCode).toBe(201)
    expect(checkIn.json().revision).toMatchObject({ number: 2, author: { name: 'Lokal' } })
  })

  it('serves the built suite beside the API, one origin', async () => {
    const dir = makeSite()
    const call = await startLocal(dir)
    const root = await call('GET', '/')
    expect(root.statusCode).toBe(200)
    expect(root.headers['content-type']).toContain('text/html')
    expect(root.body).toBe('<html>suite</html>')
    expect((await call('GET', '/layout/')).body).toBe('<html>layout</html>')
    expect((await call('GET', '/layout/assets/app.js')).headers['cache-control']).toContain('immutable')
    // The API still wins over the wildcard, and answers its own 404.
    expect((await call('GET', '/api/health')).statusCode).toBe(200)
    expect((await call('GET', '/api/nope')).statusCode).toBe(404)
    // Nothing leaves the site directory.
    expect((await call('GET', '/%2e%2e/package.json')).statusCode).toBe(404)
    expect((await call('GET', '/nope')).statusCode).toBe(404)
  })

  it('answers Range requests and HEAD, for the km data and the cloud tiles', async () => {
    const dir = makeSite()
    const call = await startLocal(dir)
    const res = await call('GET', '/data.bin')
    expect(res.statusCode).toBe(200)
    expect(res.body).toBe('0123456789')
    const part = await ctx.app.inject({ method: 'GET', url: '/data.bin', headers: { range: 'bytes=2-5' } })
    expect(part.statusCode).toBe(206)
    expect(part.headers['content-range']).toBe('bytes 2-5/10')
    expect(part.body).toBe('2345')
    const head = await ctx.app.inject({ method: 'HEAD', url: '/' })
    expect(head.statusCode).toBe(200)
    expect(head.headers['content-length']).toBe(String('<html>suite</html>'.length))
    expect(head.body).toBe('')
  })
})
