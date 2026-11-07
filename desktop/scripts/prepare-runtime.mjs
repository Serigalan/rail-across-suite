#!/usr/bin/env node
// Assemble the self-contained runtime of the local package into desktop/runtime/:
// the project server (server/) and the two packages it shares with the apps
// (@rail-across/core, @rail-across/pointcloud) as one small npm workspace, with
// the dependencies installed (--omit=dev). The Tauri bundle carries runtime/ as
// a resource (tauri.conf.json), the zip package (scripts/pack.mjs) ships it beside
// the site — either way the packaged app needs nothing from the repository
// checkout. fetch-node.mjs additionally stages a standalone Node.js binary in
// runtime/node/bin/, so the packaged app and the zip run without Node.js
// installed on the target machine.
//
//   node scripts/prepare-runtime.mjs        (or: npm run runtime)
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, rmSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '../..')
const OUT = resolve(HERE, '../runtime')

// What the server imports at run time (its package.json names the rest).
const COPY = [
  ['server/bin', 'server/bin'],
  ['server/src', 'server/src'],
  ['server/migrations', 'server/migrations'],
  ['server/package.json', 'server/package.json'],
  ['packages/core/package.json', 'packages/core/package.json'],
  ['packages/core/src', 'packages/core/src'],
  ['packages/core/constraints', 'packages/core/constraints'],
  ['packages/core/schemas', 'packages/core/schemas'],
  ['packages/core/types', 'packages/core/types'],
  ['packages/pointcloud/package.json', 'packages/pointcloud/package.json'],
  ['packages/pointcloud/src', 'packages/pointcloud/src'],
]

rmSync(OUT, { recursive: true, force: true })
for (const [from, to] of COPY) {
  const src = join(REPO, from)
  if (!existsSync(src)) throw new Error(`missing in the repository: ${from}`)
  cpSync(src, join(OUT, to), { recursive: true })
}

// One workspace, so the server's "@rail-across/*": "*" resolve to the copies beside it.
writeFileSync(join(OUT, 'package.json'), JSON.stringify({
  name: 'rail-across-suite-local-runtime',
  private: true,
  type: 'module',
  workspaces: ['server', 'packages/core', 'packages/pointcloud'],
  allowScripts: { esbuild: true, 'better-sqlite3': true },
}, null, 2) + '\n')

// shell: true on Windows, where npm is npm.cmd and CreateProcess cannot run it directly.
const r = spawnSync('npm', ['install', '--omit=dev', '--no-audit', '--no-fund'], { cwd: OUT, stdio: 'inherit', shell: process.platform === 'win32' })
if (r.status !== 0) throw new Error('npm install in runtime/ failed')

// Prove it: the server answers in local mode on a database in memory.
const check = spawnSync(process.execPath, [
  join(OUT, 'server/bin/rail-across-server.mjs'), 'serve', '--local', '--port', '0',
], { env: { ...process.env, RA_SERVER_DB: ':memory:', RA_SERVER_CLOUDS: join(OUT, '.clouds-check') }, stdio: ['ignore', 'pipe', 'inherit'], timeout: 15000 })
const line = (check.stdout ?? Buffer.alloc(0)).toString().split('\n').find(l => l.startsWith('RA_DESKTOP_URL='))
if (!line) throw new Error('the staged server did not answer with RA_DESKTOP_URL')
console.log(`\nruntime staged in ${OUT} — ${line}`)

// Standalone Node.js binary for machines without Node installed.
const fn = spawnSync(process.execPath, [join(HERE, 'fetch-node.mjs')], { stdio: 'inherit' })
if (fn.status !== 0) throw new Error('staging the bundled Node.js binary failed')
