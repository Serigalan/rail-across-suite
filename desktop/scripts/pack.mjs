#!/usr/bin/env node
// Pack the local package as one zip — rail-across-suite-local-<date>.zip — containing
// the built suite (app/), the self-contained server runtime (server/, with its
// dependencies installed) and the start scripts. Runs without compiling anything;
// Node.js 22+ is the only prerequisite on the target machine. If the Tauri app was
// built (src-tauri/target/release/bundle), its .app/.dmg goes in as desktop-app/.
//
//   node scripts/pack.mjs [out.zip]     after npm run build:site && npm run runtime
import { readdirSync, readFileSync, statSync, writeFileSync, existsSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { zipSync } from 'fflate'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const DESKTOP = resolve(HERE, '..')
const SITE = join(DESKTOP, 'site')
const RUNTIME = join(DESKTOP, 'runtime')
const BUNDLE = join(DESKTOP, 'src-tauri/target/release/bundle')

if (!existsSync(join(SITE, 'index.html'))) {
  console.error('site missing — build it first: npm run build:site')
  process.exit(2)
}
if (!existsSync(join(RUNTIME, 'server/bin/rail-across-server.mjs'))) {
  console.error('runtime missing — stage it first: npm run runtime')
  process.exit(2)
}

const now = new Date()
const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
const out = process.argv[2] ?? join(DESKTOP, `rail-across-suite-local-${date}.zip`)
const TOP = `rail-across-suite-local-${date}`

const files = (d) => readdirSync(d).flatMap((n) => {
  const p = join(d, n)
  return statSync(p).isDirectory() ? files(p) : [p]
})
// Already compressed, or too big to be worth the time: stored as they are.
// (Also the bundled Node.js binary at node/bin/node.)
const STORED = /\.(png|webp|jpg|jpeg|pdf|pmtiles|tif|tiff|zip|gz|woff2|node|dmg)$/i
const isStored = (p) => STORED.test(p) || /(^|\/)node(\.exe)?$/.test(p)

const entries = {}
const add = (name, path, mode) => {
  entries[`${TOP}/${name}`] = [readFileSync(path), { level: isStored(path) ? 0 : 6, ...(mode ? { os: 3, attrs: mode << 16 } : {}) }]
}
const addDir = (name, dir) => { for (const p of files(dir)) add(`${name}/${relative(dir, p)}`, p) }

addDir('app', SITE)
// The runtime at the zip's root: server/, packages/, node_modules/ — so the
// server's imports resolve against <root>/node_modules as in the staging.
for (const p of files(RUNTIME)) add(relative(RUNTIME, p), p)
add('README.md', join(DESKTOP, 'pack/README.md'))
add('start.sh', join(DESKTOP, 'pack/start.sh'), 0o100755)
add('start.cmd', join(DESKTOP, 'pack/start.cmd'))

// The desktop app, when it was built (tauri build): the .app and the installers beside it.
if (existsSync(BUNDLE)) {
  for (const fmt of ['macos', 'dmg']) {
    const dir = join(BUNDLE, fmt)
    if (existsSync(dir)) addDir('desktop-app', dir)
  }
}

writeFileSync(out, zipSync(entries))
console.log(`${relative(process.cwd(), out)}: ${(statSync(out).size / 1e6).toFixed(1)} MB, ${Object.keys(entries).length} files`)
