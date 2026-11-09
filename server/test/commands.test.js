import { afterEach, describe, expect, it } from 'vitest'
import { execFileSync, spawn } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { gunzipSync, gzipSync } from 'node:zlib'
import Database from 'better-sqlite3'
import { makeLas } from '@rail-across/pointcloud/test/pointCloudFixture.js'
import { gaugeProfile, gaugeProfileAreas, gaugeProfileRing } from '@rail-across/core/crossSection'
import { PW, rustServer, SERVER_BIN, signIn } from './helpers.js'

// What of the server is not its API: the schema and its migrations, the data
// steps, the command line (create-admin, user-count, backup, migrate, the
// cloud jobs' run and exec), its log, and the MDB import of a real file.

const MIGRATIONS = resolve(import.meta.dirname, '../migrations')

let dir, ctx
afterEach(async () => {
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
  if (dir) rmSync(dir, { recursive: true, force: true })
  dir = null
})

const fresh = () => (dir = mkdtempSync(join(tmpdir(), 'ra-commands-')))
const cli = (file, args, env = {}) => execFileSync(SERVER_BIN, args, {
  env: { ...process.env, RA_SERVER_DB: file, RA_SERVER_CLOUDS: join(dir, 'clouds'), ...env }, encoding: 'utf8',
})

const straight = (a, b) => ({
  elementType: 0, startNode: a, endNode: b, length: Math.hypot(b[0] - a[0], b[1] - a[1]),
  bearing: (Math.atan2(b[0] - a[0], b[1] - a[1]) * 180 / Math.PI + 360) % 360,
})

describe.skipIf(!SERVER_BIN)('the database', () => {
  it('is brought up to date once, with every migration, and opened again without applying anything twice', () => {
    const file = join(fresh(), 'a.sqlite')
    cli(file, ['migrate'])
    const db = new Database(file)
    const files = readdirSync(MIGRATIONS).filter(f => /^\d+_.*\.sql$/.test(f)).sort()
    expect(db.prepare('SELECT version, name FROM schema_migration ORDER BY version').all())
      .toEqual(files.map(f => ({ version: Number.parseInt(f, 10), name: f })))
    expect(db.pragma('journal_mode', { simple: true })).toBe('wal')
    db.close()
    cli(file, ['migrate'])
    const again = new Database(file)
    expect(again.prepare('SELECT COUNT(*) FROM schema_migration').pluck().get()).toBe(files.length)
    again.close()
  })

  it('keeps whoever already worked on a project on it when members come in (migration 005)', () => {
    const file = join(fresh(), 'a.sqlite')
    // The database as it was before 005, with a project of Ada's that Max
    // checked into and Eva branched off, and one Max has not touched.
    const old = new Database(file)
    old.exec('CREATE TABLE schema_migration (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL)')
    for (const f of readdirSync(MIGRATIONS).filter(f => /^00[1-4]_.*\.sql$/.test(f)).sort()) {
      old.exec(readFileSync(join(MIGRATIONS, f), 'utf8'))
      old.prepare('INSERT INTO schema_migration VALUES (?, ?, ?)').run(Number.parseInt(f, 10), f, 'x')
    }
    const t = '2026-10-01T00:00:00Z'
    for (const [id, login] of [[1, 'ada'], [2, 'max'], [3, 'eva'], [4, 'tom']]) {
      old.prepare("INSERT INTO user (id, login, name, password_hash, role, created_at) VALUES (?, ?, ?, 'x', 'user', ?)").run(id, login, login, t)
    }
    for (const p of ['p1', 'p2']) old.prepare('INSERT INTO project (id, title, created_by, created_at) VALUES (?, ?, 1, ?)').run(p, p, t)
    const rev = old.prepare("INSERT INTO revision (id, project_id, number, variant_id, author_id, created_at, schema_version, payload) VALUES (?, ?, ?, ?, ?, ?, 2, x'00')")
    rev.run(1, 'p1', 1, 'v1', 1, t)
    rev.run(2, 'p1', 2, 'v1', 2, t)
    rev.run(3, 'p2', 1, 'v3', 1, t)
    const variant = old.prepare('INSERT INTO variant (id, project_id, name, head_revision_id, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?)')
    variant.run('v1', 'p1', 'Bestand', 2, 1, t)
    variant.run('v2', 'p1', '2030', 1, 3, t)
    variant.run('v3', 'p2', 'Bestand', 3, 1, t)
    old.close()
    cli(file, ['migrate'])
    const db = new Database(file)
    expect(db.prepare('SELECT project_id, user_id FROM project_member ORDER BY project_id, user_id').all())
      .toEqual([{ project_id: 'p1', user_id: 2 }, { project_id: 'p1', user_id: 3 }])
    db.close()
  })
})

describe.skipIf(!SERVER_BIN)('the data steps', () => {
  /** A database with one project of these elements, made through the API, its data step to run again. */
  async function withProject(elements) {
    const file = join(fresh(), 'a.sqlite')
    ctx = await rustServer({ file })
    await ctx.app.auth.createUser({ login: 'anna', name: 'Anna', password: PW, role: 'admin', mustChangePassword: false })
    const anna = await signIn(ctx.app, 'anna')
    const made = await anna('POST', '/api/projects', {
      title: 'Halle', payload: { id: 'p1', tracks: [{ id: 't1', name: 'T1', epsg: 25832, elements }], switches: [], platforms: [] },
    })
    expect(made.statusCode, made.body).toBe(201)
    ctx.db.prepare("DELETE FROM data_migration WHERE name = '0020-node-tolerance-1um'").run()
    await ctx.app.close()
    ctx.db.close()
    ctx = null
    return file
  }

  it('close a millimetre joint as a revision of their own, by the server, once', async () => {
    const file = await withProject([straight([500000, 5600000], [500100, 5600000]), straight([500100.0005, 5600000], [500200, 5600000])])
    expect(cli(file, ['migrate'])).toMatch(/data step 0020-node-tolerance-1um: 1 Revisionen, 1 Stöße/)
    const db = new Database(file)
    const head = db.prepare('SELECT number, message, author_id, payload FROM revision ORDER BY id DESC LIMIT 1').get()
    expect(head.number).toBe(2)
    expect(head.message).toMatch(/1 Stöße bis 1 mm geschlossen/)
    const system = db.prepare("SELECT id, active FROM user WHERE login = 'system'").get()
    expect(head.author_id).toBe(system.id)
    expect(system.active).toBe(0)
    const [first, second] = JSON.parse(gunzipSync(head.payload).toString()).domains.core.collections.tracks[0].elements
    expect(second.startNode).toEqual(first.endNode)
    db.close()
    expect(cli(file, ['migrate'])).toBe('')
  })

  it('write nothing, and make no user, where every joint is closed', async () => {
    const file = await withProject([straight([500000, 5600000], [500100, 5600000]), straight([500100, 5600000], [500200, 5600000])])
    expect(cli(file, ['migrate'])).toMatch(/data step 0020-node-tolerance-1um: 0 Revisionen, 0 Stöße/)
    const db = new Database(file)
    expect(db.prepare('SELECT COUNT(*) FROM user').pluck().get()).toBe(1)
    expect(db.prepare('SELECT COUNT(*) FROM revision').pluck().get()).toBe(1)
    db.close()
  })
})

describe.skipIf(!SERVER_BIN)('the step to infra-data-1.0 (decision 0021)', () => {
  it('rewrites the revisions kept as the core 1.0 document and as oltjson-1.1 records, once, in place', async () => {
    const file = join(fresh(), 'a.sqlite')
    ctx = await rustServer({ file })
    await ctx.app.auth.createUser({ login: 'anna', name: 'Anna', password: PW, role: 'admin', mustChangePassword: false })
    const anna = await signIn(ctx.app, 'anna')
    const track = { id: 't1', name: 'T1', epsg: 25832, elements: [straight([500000, 5600000], [500100, 5600000])] }
    const made = (await anna('POST', '/api/projects', { title: 'Halle', payload: { id: 'p1', tracks: [track], switches: [], platforms: [] } })).json()
    const second = (await anna('POST', `/api/variants/${made.variantId}/revisions`, {
      base: made.revisionId, payload: { id: made.project.id, title: 'Halle', tracks: [track], switches: [], platforms: [] }, remaps: [],
    })).json().revision.id
    await ctx.app.close()
    ctx.db.close()
    ctx = null
    // As the two revisions were kept before: the old envelope, and an oltjson-1.1 record whose track is new.
    const db = new Database(file)
    const { format: _f, version: _v, ...doc } = JSON.parse(gunzipSync(db.prepare('SELECT payload FROM revision WHERE id = ?').pluck().get(made.revisionId)).toString())
    db.prepare('UPDATE revision SET schema_version = 3, payload = ? WHERE id = ?').run(gzipSync(JSON.stringify({ format: 'rail-across', ...doc })), made.revisionId)
    const record = { id: made.project.id, title: 'Halle', tracks: [{ ...track, status: 'new' }], switches: [], platforms: [] }
    db.prepare('UPDATE revision SET schema_version = 2, payload = ? WHERE id = ?').run(gzipSync(JSON.stringify(record)), second)
    db.prepare("DELETE FROM data_migration WHERE name = '0021-infra-data'").run()
    db.close()
    expect(cli(file, ['migrate'])).toMatch(/data step 0021-infra-data: 2 Revisionen umgeschrieben, 0 nicht lesbar/)
    const after = new Database(file)
    const rows = after.prepare('SELECT schema_version, payload FROM revision ORDER BY id').all()
    after.close()
    expect(rows.map(r => r.schema_version)).toEqual([4, 4])
    const [old, kept] = rows.map(r => JSON.parse(gunzipSync(r.payload).toString()))
    expect(old).toMatchObject({ format: 'infra-data', version: '1.0', domains: doc.domains })
    const stages = kept.domains.core.fields.stages
    expect(kept).toMatchObject({ format: 'infra-data', version: '1.0' })
    expect(kept.domains.core.collections.tracks[0].lifespan).toEqual({ from: stages[1].id })
    expect(cli(file, ['migrate'])).toBe('')
  })
})

describe.skipIf(!SERVER_BIN)('the command line', () => {
  it('creates admins who sign in with their start password, counts users and writes backups', async () => {
    const file = join(fresh(), 'a.sqlite')
    expect(cli(file, ['user-count']).trim()).toBe('0')
    cli(file, ['create-admin', 'lena', 'Lena', 'Schulz'], { RA_ADMIN_PASSWORD: 'a long start password' })
    expect(cli(file, ['user-count']).trim()).toBe('1')
    ctx = await rustServer({ file })
    const res = await ctx.app.inject({ method: 'POST', url: '/api/login', payload: { login: 'lena', password: 'a long start password' } })
    expect(res.statusCode).toBe(200)
    expect(res.json().user).toMatchObject({ name: 'Lena Schulz', role: 'admin', mustChangePassword: true })
    cli(file, ['backup', join(dir, 'copy.sqlite')])
    const copy = new Database(join(dir, 'copy.sqlite'))
    expect(copy.prepare('SELECT login FROM user').pluck().all()).toEqual(['lena'])
    copy.close()
  })

  it('writes every request to its log, the path without its query', async () => {
    fresh()
    const child = spawn(SERVER_BIN, ['serve', '--port', '0'], {
      env: { ...process.env, RA_SERVER_DB: join(dir, 'a.sqlite'), RA_SERVER_NO_CLOUDS: '1', RA_SERVER_LOG: 'info' },
      stdio: ['ignore', 'pipe', 'inherit'],
    })
    let out = ''
    child.stdout.on('data', d => { out += d })
    try {
      const port = await new Promise((done, fail) => {
        const check = setInterval(() => {
          const m = /listening on port (\d+)/.exec(out)
          if (m) { clearInterval(check); done(m[1]) }
        }, 20)
        child.on('exit', code => { clearInterval(check); fail(new Error(`exited ${code}`)) })
      })
      expect((await fetch(`http://127.0.0.1:${port}/api/health?token=secret`)).status).toBe(200)
      expect((await fetch(`http://127.0.0.1:${port}/api/nope`)).status).toBe(404)
      await new Promise(r => setTimeout(r, 100))
      expect(out).toMatch(/^\S+Z 127\.0\.0\.1 GET \/api\/health 200 [\d.]+ ms$/m)
      expect(out).toMatch(/ GET \/api\/nope 404 /)
      expect(out).not.toContain('secret')
    } finally {
      child.kill()
    }
  })
})

// ── the cloud jobs' commands ────────────────────────────────────────────────

const E0 = 4470000, N0 = 5332000
const HALF = 1.435 / 2
/** A track bed with two rails and a post beside them at 15–16 m. */
function trackCloud() {
  const pts = []
  for (let s = 0; s <= 30; s += 0.05) {
    for (let y = -2.5; y <= 2.5; y += 0.05) pts.push({ x: E0 + y, y: N0 + s, z: 99.83, intensity: Math.round(s * 100) % 4096, red: 200 << 8, green: (s * 1000) % 65536, blue: 77 })
    for (const side of [-1, 1]) {
      const inner = side * HALF
      for (let d = 0; d <= 0.067; d += 0.005) pts.push({ x: E0 + inner + side * d, y: N0 + s, z: 100, intensity: 3000 })
      for (let dz = 0.005; dz <= 0.05; dz += 0.005) pts.push({ x: E0 + inner, y: N0 + s, z: 100 - dz, intensity: 2500 })
    }
    if (s >= 15 && s <= 16) for (let z = 100.5; z <= 102; z += 0.05) pts.push({ x: E0 + 1, y: N0 + s, z, intensity: 4000 })
  }
  return makeLas(pts, { offset: [E0, N0, 0], format: 3 })
}

const TRACK = {
  id: 't1', name: 'Gleis 1', epsg: 5678,
  elements: [{ elementType: 0, startNode: [E0, N0], endNode: [E0, N0 + 30], bearing: 0, length: 30, cant: 0 }],
  heights: [{ station: 0, z: 100 }, { station: 30, z: 100 }],
}

describe.skipIf(!SERVER_BIN)('the cloud jobs\' commands', () => {
  it('prepare an upload into five levels (cloudjobs run), and carry out a clearance scan and a trace (cloudjobs exec)', async () => {
    const file = join(fresh(), 'a.sqlite')
    ctx = await rustServer({ file, cloudRoot: join(dir, 'clouds') })
    await ctx.app.auth.createUser({ login: 'ada', name: 'Ada', password: PW, role: 'admin', mustChangePassword: false })
    const ada = await signIn(ctx.app, 'ada')
    const { project } = (await ada('POST', '/api/projects', { title: 'P' })).json()
    const las = trackCloud()
    const { cloud } = (await ada('POST', `/api/projects/${project.id}/clouds`, {
      name: 'Gleis', fileName: 'g.las', fileSize: las.length, format: 'las', crs: 5678, heightEpsg: 7837, pointCount: 0, rgb: true,
    })).json()
    const url = `/api/projects/${project.id}/clouds/${cloud.id}`
    const { createHash } = await import('node:crypto')
    for (let at = 0; at < las.length; at += 1 << 20) {
      const part = las.subarray(at, at + (1 << 20))
      const res = await ctx.app.inject({
        method: 'PUT', url: `${url}/raw?offset=${at}`, cookies: { ra_session: ada.token },
        headers: { 'content-type': 'application/octet-stream', 'x-olt-upload': '1', 'x-olt-sha256': createHash('sha256').update(part).digest('hex') },
        payload: Buffer.from(part),
      })
      expect(res.statusCode, res.body).toBe(200)
    }
    await ada('POST', `${url}/complete`)
    const job = ctx.db.prepare("SELECT id FROM cloud_job WHERE status = 'queued'").pluck().get()
    cli(file, ['cloudjobs', 'run', String(job)])
    const ready = (await ada('GET', `/api/projects/${project.id}/clouds`)).json().clouds[0]
    expect(ready).toMatchObject({ status: 'ready', crs: 5678, rgb: true })
    expect(ready.levels.map(l => l.level)).toEqual([0, 1, 2, 3, 4])
    expect(existsSync(join(dir, 'clouds', project.id, cloud.id, 'raw.part'))).toBe(false)

    const profile = gaugeProfile('hauptgleis')
    const runs = {
      clearance: { kind: 'clearance', subject: { trackId: 't1' }, params: { track: TRACK, tracks: [TRACK], switches: [], ring: gaugeProfileRing(profile.points), areas: gaugeProfileAreas(profile.einragungen) } },
      trace: { kind: 'trace', subject: { name: 'Linie' }, params: { rail: '54E4', guide: { kind: 'line', epsg: 5678, vertices: [[E0 + 0.6, N0], [E0 + 0.9, N0 + 30]] } } },
    }
    for (const body of Object.values(runs)) {
      const made = await ada('POST', `/api/projects/${project.id}/runs`, body)
      expect(made.statusCode, made.body).toBe(201)
      const { run } = made.json()
      ctx.db.prepare("UPDATE cloud_run SET status = 'running' WHERE id = ?").run(run.id)
      cli(file, ['cloudjobs', 'exec', String(run.id)])
      const done = (await ada('GET', `/api/projects/${project.id}/runs/${run.id}`)).json().run
      expect(done.status, done.error).toBe('done')
      if (body.kind === 'clearance') {
        expect(done.result.stretches).toHaveLength(1)
        expect(done.result.stretches[0].from).toBeGreaterThan(14.5)
        expect(done.result.stretches[0].to).toBeLessThan(16.5)
      } else {
        expect(done.result.points.length).toBeGreaterThan(50)
      }
    }
  }, 120000)
})

// An Access file of real planning data, kept out of the repository like the
// PEK file: looked for under RA_MDB_SAMPLE or the old repository's test data.
const MDB = [process.env.RA_MDB_SAMPLE, '/root/open-layout-tool/Testdaten.MDB'].find(p => p && existsSync(p))

describe.skipIf(!SERVER_BIN || !MDB)('the MDB import of a real file', () => {
  it('turns an Access file into its Satzarten, each with what it read', async () => {
    ctx = await rustServer()
    await ctx.app.auth.createUser({ login: 'max', name: 'Max', password: PW, role: 'user', mustChangePassword: false })
    const max = await signIn(ctx.app, 'max')
    const res = await ctx.app.inject({
      method: 'POST', url: '/api/mdb', cookies: { ra_session: max.token },
      headers: { 'content-type': 'application/octet-stream', 'x-olt-upload': '1' }, payload: readFileSync(MDB),
    })
    expect(res.statusCode).toBe(200)
    const payload = res.json()
    expect(payload.elements.length).toBeGreaterThan(0)
    expect(Object.values(payload.counts).some(n => n > 0)).toBe(true)
  }, 120000)
})
