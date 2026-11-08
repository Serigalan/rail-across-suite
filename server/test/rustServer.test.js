import { afterEach, describe, expect, it } from 'vitest'
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { gunzipSync } from 'node:zlib'
import Database from 'better-sqlite3'
import { makeLas } from '@rail-across/pointcloud/test/pointCloudFixture.js'
import { decodeCloudSegment, segmentPlacement } from '@rail-across/pointcloud/src/tiles.js'
import { gaugeProfile, gaugeProfileAreas, gaugeProfileRing } from '@rail-across/core/src/crossSection/gaugeProfiles.js'
import { openDatabase, schemaVersion } from '../src/db.js'
import { createStore } from '../src/store.js'
import { createAuth, verifyPassword } from '../src/auth.js'
import { cloudStorage } from '../src/clouds/storage.js'
import { createCloudStore } from '../src/clouds/cloudStore.js'
import { createRunStore } from '../src/clouds/runStore.js'
import { prepareCloud } from '../src/clouds/prepare.js'
import { executeRun } from '../src/clouds/runs.js'
import { runDataMigrations } from '../src/dataMigrations.js'

// The Rust server's commands and jobs held to this server's (decision 0020,
// step 8): the HTTP API is held by the other tests run with
// RA_SERVER_IMPL=rust; here what is not HTTP — the migrations, the data
// steps, the command line, and the cloud jobs, whose tiles and results must
// come out as this server's do. Needs the Rust server built.

const BIN = process.env.RA_SERVER_BIN ?? ['release', 'debug']
  .map(p => resolve(import.meta.dirname, `../../target/${p}/rail-across-server`)).find(existsSync)
const MIGRATIONS = resolve(import.meta.dirname, '../migrations')

let dir
afterEach(() => { if (dir) rmSync(dir, { recursive: true, force: true }); dir = null })

const rust = (file, args, env = {}) => execFileSync(BIN, args, {
  env: { ...process.env, RA_SERVER_DB: file, RA_SERVER_CLOUDS: join(dir, 'clouds'), ...env }, encoding: 'utf8',
})

describe.skipIf(!BIN)('the Rust server beside this one', () => {
  it('brings a database up to date as this server does, and opens this server\'s', () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const file = join(dir, 'a.sqlite')
    rust(file, ['migrate'])
    const db = new Database(file)
    const js = openDatabase(':memory:')
    expect(db.prepare('SELECT version, name FROM schema_migration ORDER BY version').all())
      .toEqual(js.prepare('SELECT version, name FROM schema_migration ORDER BY version').all())
    const tables = (d) => d.prepare("SELECT name, sql FROM sqlite_master WHERE type IN ('table', 'index') AND name NOT LIKE 'sqlite_%' ORDER BY name").all()
    expect(tables(db)).toEqual(tables(js))
    expect(db.pragma('journal_mode', { simple: true })).toBe('wal')
    db.close()
    js.close()
    // Opened again, nothing applied twice; this server opens it as its own.
    rust(file, ['migrate'])
    const again = openDatabase(file)
    expect(again.prepare('SELECT COUNT(*) FROM schema_migration').pluck().get()).toBe(schemaVersion(again))
    again.close()
  })

  it('keeps whoever already worked on a project on it when members come in (migration 005)', () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const file = join(dir, 'a.sqlite')
    const old = new Database(file)
    old.exec('CREATE TABLE schema_migration (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL)')
    for (const f of readdirSync(MIGRATIONS).filter(f => /^00[1-4]_.*\.sql$/.test(f)).sort()) {
      old.exec(readFileSync(join(MIGRATIONS, f), 'utf8'))
      old.prepare('INSERT INTO schema_migration VALUES (?, ?, ?)').run(Number.parseInt(f, 10), f, 'x')
    }
    const t = '2026-10-01T00:00:00Z'
    for (const [id, login] of [[1, 'ada'], [2, 'max'], [3, 'eva']]) {
      old.prepare("INSERT INTO user (id, login, name, password_hash, role, created_at) VALUES (?, ?, ?, 'x', 'user', ?)").run(id, login, login, t)
    }
    old.prepare('INSERT INTO project (id, title, created_by, created_at) VALUES (?, ?, 1, ?)').run('p1', 'p1', t)
    const rev = old.prepare("INSERT INTO revision (id, project_id, number, variant_id, author_id, created_at, schema_version, payload) VALUES (?, 'p1', ?, 'v1', ?, ?, 2, x'00')")
    rev.run(1, 1, 1, t)
    rev.run(2, 2, 2, t)
    old.prepare("INSERT INTO variant (id, project_id, name, head_revision_id, created_by, created_at) VALUES ('v1', 'p1', 'Bestand', 2, 1, ?), ('v2', 'p1', '2030', 1, 3, ?)").run(t, t)
    old.close()
    rust(file, ['user-count'])
    const db = new Database(file)
    expect(db.prepare('SELECT project_id, user_id FROM project_member ORDER BY user_id').all())
      .toEqual([{ project_id: 'p1', user_id: 2 }, { project_id: 'p1', user_id: 3 }])
    db.close()
  })

  it('closes millimetre joints in its data step as this server does', async () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const straight = (a, b) => ({
      elementType: 0, startNode: a, endNode: b, length: Math.hypot(b[0] - a[0], b[1] - a[1]),
      bearing: (Math.atan2(b[0] - a[0], b[1] - a[1]) * 180 / Math.PI + 360) % 360,
    })
    const make = (file) => {
      const db = openDatabase(file)
      db.prepare(`INSERT INTO user (login, name, password_hash, role, created_at) VALUES ('anna', 'Anna', 'x', 'admin', '2026-01-01')`).run()
      createStore(db).createProject({
        title: 'Halle', authorId: 1, errorKeys: [],
        payload: { id: 'p1', tracks: [{ id: 't1', name: 'T1', epsg: 25832, elements: [straight([500000, 5600000], [500100, 5600000]), straight([500100.0005, 5600000], [500200, 5600000])] }], switches: [], platforms: [] },
      })
      return db
    }
    const a = join(dir, 'js.sqlite'), b = join(dir, 'rust.sqlite')
    const js = make(a)
    make(b).close()
    expect(runDataMigrations(js, createStore(js)).map(r => r.note)).toEqual(['1 Revisionen, 1 Stöße'])
    expect(rust(b, ['migrate'])).toMatch(/data step 0020-node-tolerance-1um: 1 Revisionen, 1 Stöße/)
    const rs = new Database(b)
    const head = (d) => d.prepare('SELECT number, message, payload FROM revision ORDER BY id DESC LIMIT 1').get()
    const [hj, hr] = [head(js), head(rs)]
    expect([hr.number, hr.message]).toEqual([hj.number, hj.message])
    const tracks = (h) => JSON.parse(gunzipSync(h.payload).toString()).domains.core.collections.tracks
    const nodes = (h) => tracks(h)[0].elements.map(e => [e.startNode, e.endNode])
    expect(nodes(hr)).toEqual(nodes(hj))
    expect(rs.prepare("SELECT active FROM user WHERE login = 'system'").pluck().get()).toBe(0)
    expect(rust(b, ['migrate'])).toBe('')
    rs.close()
    js.close()
  })

  it('creates admins whose passwords this server checks, counts users and writes backups', async () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const file = join(dir, 'a.sqlite')
    expect(rust(file, ['user-count']).trim()).toBe('0')
    rust(file, ['create-admin', 'lena', 'Lena', 'Schulz'], { RA_ADMIN_PASSWORD: 'a long start password' })
    expect(rust(file, ['user-count']).trim()).toBe('1')
    const db = openDatabase(file)
    const lena = createAuth(db).userByLogin('lena')
    expect(lena).toMatchObject({ name: 'Lena Schulz', role: 'admin', must_change_password: 1 })
    expect(await verifyPassword(lena.password_hash, 'a long start password')).toBe(true)
    db.close()
    rust(file, ['backup', join(dir, 'copy.sqlite')])
    const copy = new Database(join(dir, 'copy.sqlite'))
    expect(copy.prepare('SELECT login FROM user').pluck().all()).toEqual(['lena'])
    copy.close()
  })
})

// ── the cloud jobs ──────────────────────────────────────────────────────────

const E0 = 4470000, N0 = 5332000
const HALF = 1.435 / 2
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

/** Every point of a level as text, sorted — what two zlibs must agree on. */
function points(storage, c, level) {
  const index = JSON.parse(readFileSync(storage.index(c.project_id, c.id, level), 'utf8'))
  const bytes = readFileSync(storage.tiles(c.project_id, c.id, level))
  const out = []
  for (const [tx, ty, segs] of index.tiles) {
    for (const [at, length, count, z0] of segs) {
      const s = decodeCloudSegment(index, bytes.subarray(at, at + length), count)
      const p = segmentPlacement(index, tx, ty, z0)
      for (let k = 0; k < count; k++) {
        out.push([tx, ty, z0, s.x[k], s.y[k], s.z[k], s.intensity?.[k] ?? s.i[k], s.r?.[k] ?? '', s.g?.[k] ?? '', s.b?.[k] ?? '', p.ox, p.oy].join(' '))
      }
    }
  }
  const { tiles: _t, bytes: _b, ...rest } = index
  return { meta: rest, tiles: index.tiles.map(([tx, ty, segs]) => [tx, ty, segs.map(s => [s[2], s[3]])]), points: out.sort() }
}

describe.skipIf(!BIN)('the Rust cloud jobs beside this server\'s', () => {
  it('prepare a cloud into the same five levels, and run the same clearance scan and trace', async () => {
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const file = join(dir, 'a.sqlite')
    const db = openDatabase(file)
    const auth = createAuth(db)
    const ada = await auth.createUser({ login: 'ada', name: 'Ada', password: 'x'.repeat(12), role: 'admin', mustChangePassword: false })
    const { projectId } = createStore(db).createProject({ title: 'P', payload: { tracks: [], switches: [], platforms: [] }, authorId: ada.id, errorKeys: [] })
    const storage = cloudStorage(join(dir, 'clouds'))
    const clouds = createCloudStore(db)
    const las = trackCloud()
    const cloud = () => {
      const id = clouds.create({ projectId, name: 'Gleis', fileName: 'g.las', fileSize: las.length, format: 'las', crs: 5678, heightEpsg: 7837, keepRaw: false, sourcePoints: 0, rgb: true, userId: ada.id })
      storage.ensure(projectId, id)
      writeFileSync(storage.raw(projectId, id), las)
      clouds.received(id, 0, las.length)
      clouds.enqueue(id, ada.id, 'x')
      return id
    }
    const viaJs = cloud(), viaRust = cloud()
    // The JavaScript job takes the first, the Rust job the second.
    const jobJs = clouds.claimJob()
    await prepareCloud({ cloudId: jobJs.cloud_id, clouds, storage })
    clouds.finishJob(jobJs.id, 'done')
    const jobRust = db.prepare("SELECT id FROM cloud_job WHERE status = 'queued'").pluck().get()
    rust(file, ['cloudjobs', 'run', String(jobRust)])
    const [a, b] = [clouds.get(viaJs), clouds.get(viaRust)]
    expect(b.status).toBe('ready')
    expect(existsSync(storage.raw(projectId, viaRust))).toBe(false)
    expect({ ...b, id: 0, bytes: 0, levels: 0, ready_at: 0, created_at: 0 }).toEqual({ ...a, id: 0, bytes: 0, levels: 0, ready_at: 0, created_at: 0 })
    const levels = (c) => JSON.parse(c.levels).map(({ bytes: _b, ...l }) => l)
    expect(levels(b)).toEqual(levels(a))
    for (const level of [0, 1, 2, 3, 4]) {
      const [pa, pb] = [points(storage, a, level), points(storage, b, level)]
      expect({ ...pb.meta, id: 0, createdAt: 0 }).toEqual({ ...pa.meta, id: 0, createdAt: 0 })
      expect(pb.tiles).toEqual(pa.tiles)
      expect(pb.points).toEqual(pa.points)
    }

    // The runs over the cloud the Rust job made: one each way.
    const runs = createRunStore(db)
    const profile = gaugeProfile('hauptgleis')
    const params = {
      clearance: { track: TRACK, tracks: [TRACK], switches: [], ring: gaugeProfileRing(profile.points), areas: gaugeProfileAreas(profile.einragungen) },
      trace: { rail: '54E4', guide: { kind: 'line', epsg: 5678, vertices: [[E0 + 0.6, N0], [E0 + 0.9, N0 + 30]] } },
    }
    for (const kind of ['clearance', 'trace']) {
      const one = () => runs.create({ projectId, kind, params: params[kind], clouds: [viaRust], userId: ada.id })
      const [rj, rr] = [one(), one()]
      runs.claim()
      runs.claim()
      await executeRun({ runId: rj, runs, clouds, storage })
      rust(file, ['cloudjobs', 'exec', String(rr)])
      const [doneJs, doneRust] = [runs.get(rj), runs.get(rr)]
      expect(doneRust.status).toBe('done')
      const result = JSON.parse(doneRust.result)
      expect(result).toEqual(JSON.parse(doneJs.result))
      expect(kind === 'clearance' ? result.stretches.length : result.points.length).toBeGreaterThan(0)
    }
    db.close()
    copyFileSync(file, join(dir, 'kept.sqlite'))
  }, 120000)
})

// An Access file of real planning data, kept out of the repository like the
// PEK file: looked for under RA_MDB_SAMPLE or the old repository's test data.
const MDB = [process.env.RA_MDB_SAMPLE, '/root/open-layout-tool/Testdaten.MDB'].find(p => p && existsSync(p))

describe.skipIf(!BIN || !MDB)('the MDB conversion of both servers', () => {
  it('turns a real Access file into the same import payload', async () => {
    const { convertMdb } = await import('../src/mdb.js')
    const { spawn } = await import('node:child_process')
    const bytes = readFileSync(MDB)
    const js = await convertMdb(bytes)
    dir = mkdtempSync(join(tmpdir(), 'ra-rust-'))
    const file = join(dir, 'a.sqlite')
    const db = openDatabase(file)
    await createAuth(db).createUser({ login: 'max', name: 'Max', password: 'x'.repeat(12), role: 'user', mustChangePassword: false })
    db.close()
    const child = spawn(BIN, ['serve'], {
      env: { ...process.env, RA_SERVER_DB: file, RA_SERVER_PORT: '0', RA_SERVER_INSECURE_COOKIE: '1', RA_SERVER_NO_CLOUDS: '1' },
      stdio: ['ignore', 'pipe', 'inherit'],
    })
    const port = await new Promise((done) => {
      let out = ''
      child.stdout.on('data', (d) => { out += d; const m = /listening on port (\d+)/.exec(out); if (m) done(Number(m[1])) })
    })
    try {
      const base = `http://127.0.0.1:${port}`
      const login = await fetch(`${base}/api/login`, {
        method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ login: 'max', password: 'x'.repeat(12) }),
      })
      const cookie = login.headers.getSetCookie()[0].split(';')[0]
      const res = await fetch(`${base}/api/mdb`, {
        method: 'POST', headers: { cookie, 'content-type': 'application/octet-stream', 'x-olt-upload': '1' }, body: bytes,
      })
      expect(res.status).toBe(200)
      const rs = await res.json()
      expect(rs.counts).toEqual(js.counts)
      expect(rs).toEqual(js)
      expect(js.elements.length).toBeGreaterThan(0)
    } finally {
      child.kill('SIGTERM')
    }
  }, 120000)
})
