import { spawn } from 'node:child_process'
import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { openDatabase } from '../src/db.js'
import { buildApp } from '../src/app.js'
import { createAuth } from '../src/auth.js'
import { cloudStorage } from '../src/clouds/storage.js'

export const PW = 'correct horse battery'

/**
 * Which server the tests run against: this one in process (Fastify's inject),
 * or with RA_SERVER_IMPL=rust the Rust server (crates/server, decision 0020
 * step 8) over HTTP — the same tests, the same database file, the Rust server
 * started in its test mode, where it takes the clock from x-ra-test-now and
 * the peer's address from x-ra-test-remote.
 */
export const RUST = process.env.RA_SERVER_IMPL === 'rust'
const RUST_BIN = process.env.RA_SERVER_BIN ?? ['release', 'debug']
  .map(p => resolve(import.meta.dirname, `../../target/${p}/rail-across-server`)).find(existsSync)

/**
 * A server on a fresh database with a clock the test turns, and an admin
 * and a user who have changed their start passwords already.
 */
export async function setup({ routes, cloudRoot = null } = {}) {
  const clock = { t: Date.parse('2026-10-01T08:00:00Z') }
  const now = () => clock.t
  if (!RUST) {
    const db = openDatabase(':memory:')
    const auth = createAuth(db, { now })
    await auth.createUser({ login: 'ada', name: 'Ada Admin', password: PW, role: 'admin', mustChangePassword: false })
    await auth.createUser({ login: 'max', name: 'Max Muster', password: PW, role: 'user', mustChangePassword: false })
    const app = buildApp({ db, secureCookie: false, now, routes, cloudRoot })
    await app.ready()
    return { app, db, clock }
  }
  const dir = mkdtempSync(join(tmpdir(), 'ra-rust-server-'))
  const file = join(dir, 'rail-across.sqlite')
  const db = openDatabase(file)
  const auth = createAuth(db, { now })
  await auth.createUser({ login: 'ada', name: 'Ada Admin', password: PW, role: 'admin', mustChangePassword: false })
  await auth.createUser({ login: 'max', name: 'Max Muster', password: PW, role: 'user', mustChangePassword: false })
  const app = await rustServer({ file, dir, cloudRoot, clock, db })
  return { app, db, clock }
}

/** The Rust server on `file`, answering as Fastify's app does for the tests. */
async function rustServer({ file, dir, cloudRoot, clock, db }) {
  if (!RUST_BIN) throw new Error('the Rust server is not built (cargo build --release -p rail-across-server)')
  const child = spawn(RUST_BIN, ['serve'], {
    env: {
      ...process.env, RA_SERVER_DB: file, RA_SERVER_PORT: '0', RA_SERVER_TEST: '1', RA_SERVER_INSECURE_COOKIE: '1',
      ...(cloudRoot ? { RA_SERVER_CLOUDS: cloudRoot } : { RA_SERVER_NO_CLOUDS: '1' }),
    },
    stdio: ['ignore', 'pipe', 'inherit'],
  })
  const port = await new Promise((done, fail) => {
    let out = ''
    child.stdout.on('data', (d) => {
      out += d
      const m = /listening on port (\d+)/.exec(out)
      if (m) done(Number(m[1]))
    })
    child.on('exit', (code) => fail(new Error(`the Rust server stopped (${code}): ${out}`)))
  })
  const base = `http://127.0.0.1:${port}`

  async function inject({ method = 'GET', url, payload, cookies = {}, headers = {}, remoteAddress }) {
    const h = { 'x-ra-test-now': String(clock.t), ...headers }
    if (remoteAddress) h['x-ra-test-remote'] = remoteAddress
    const jar = Object.entries(cookies).map(([k, v]) => `${k}=${v}`).join('; ')
    if (jar) h.cookie = jar
    let body
    if (payload !== undefined) {
      if (Buffer.isBuffer(payload) || payload instanceof Uint8Array || typeof payload === 'string') {
        body = payload
      } else {
        body = JSON.stringify(payload)
        if (!Object.keys(h).some(k => k.toLowerCase() === 'content-type')) h['content-type'] = 'application/json'
      }
    }
    const res = await fetch(base + url, { method, headers: h, body, redirect: 'manual' })
    const raw = Buffer.from(await res.arrayBuffer())
    const text = raw.toString('utf8')
    const out = {
      statusCode: res.status,
      headers: Object.fromEntries([...res.headers]),
      rawPayload: raw,
      payload: text,
      body: text,
      json: () => JSON.parse(text),
      // As light-my-request lists them: name, value and the attributes, camel-cased.
      cookies: res.headers.getSetCookie().map((c) => {
        const [pair, ...attrs] = c.split(';').map(s => s.trim())
        const at = pair.indexOf('=')
        const cookie = { name: pair.slice(0, at), value: decodeURIComponent(pair.slice(at + 1)) }
        for (const a of attrs) {
          const [k, v] = a.split('=')
          const key = { 'max-age': 'maxAge', httponly: 'httpOnly', samesite: 'sameSite' }[k.toLowerCase()] ?? k.toLowerCase()
          cookie[key] = v === undefined ? true : key === 'maxAge' ? Number(v) : key === 'expires' ? new Date(v) : v
        }
        return cookie
      }),
    }
    return out
  }

  const call = async (method, url, payload) => (await inject({ method, url, payload: method === 'GET' ? undefined : payload ?? {} })).json()
  return {
    rust: true,
    inject,
    cloudStorage: cloudRoot ? cloudStorage(cloudRoot) : null,
    auth: {
      failureCount: async () => (await call('GET', '/api/__test/failures', undefined)).count,
      sweep: async () => { await call('POST', '/api/__test/sweep') },
      createUser: async (u) => (await call('POST', '/api/__test/users', u)).user,
      userByLogin: (login) => db.prepare('SELECT * FROM user WHERE login = ?').get(String(login ?? '').trim()),
    },
    async close() {
      if (child.exitCode == null) {
        const gone = new Promise(done => child.on('exit', done))
        child.kill('SIGTERM')
        await gone
      }
      rmSync(dir, { recursive: true, force: true })
    },
  }
}

/** Sign in and return a request function carrying the session. */
export async function signIn(app, login, password = PW) {
  const res = await app.inject({ method: 'POST', url: '/api/login', payload: { login, password } })
  if (res.statusCode !== 200) throw new Error(`sign-in failed: ${res.statusCode} ${res.body}`)
  const cookie = res.cookies.find(c => c.name === 'ra_session')
  return as(app, cookie.value)
}

export function as(app, token) {
  const call = (method, url, payload) => app.inject({
    method, url,
    cookies: token ? { ra_session: token } : {},
    ...(payload !== undefined ? { payload } : method === 'GET' ? {} : { payload: {} }),
  })
  call.token = token
  return call
}
