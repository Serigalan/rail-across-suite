#!/usr/bin/env node
// Build the suite's site for the local package: the start page at the root of
// desktop/site, each module's app under its path — the same layout
// deploy/deploy.sh builds into RA_SITE_DIR (MODULE_APPS in deploy/lib.sh), so
// the server can serve it with one `--static desktop/site` and the Tauri
// window loads it from there.
//
// Run after `npm install` at the repository root (vite comes from there):
//
//   node scripts/build-site.mjs          (or: npm run build:site)
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '../..')
const SITE = resolve(HERE, '../site')
const VITE = join(REPO, 'node_modules/vite/bin/vite.js')

// "<app directory under apps/>:<path in the site>" — as deploy/lib.sh MODULE_APPS.
const MODULE_APPS = [
  ['layout-tool', 'layout'],
  ['signalling-tool', 'signalling'],
  ['simulation-tool', 'simulation'],
  ['vehicle-editor', 'vehicles'],
  ['timetable-editor', 'timetable'],
]

function run(args, opts) {
  const r = spawnSync(process.execPath, args, { stdio: 'inherit', ...opts })
  if (r.status !== 0) throw new Error(`failed: node ${args.join(' ')}`)
}

/** Optional beside the page: warn and go on where it is missing. */
function tryRun(label, args, opts) {
  const r = spawnSync(process.execPath, args, { stdio: 'inherit', ...opts })
  if (r.status !== 0) console.error(`WARNING: ${label} failed — the app builds without it, those functions say the module is missing`)
}

if (!existsSync(VITE)) {
  console.error('no vite found — run `npm install` at the repository root first')
  process.exit(1)
}

// The Layout-Tool's km line data (decision 0008) and its two Rust modules in
// WebAssembly, beside the page — as deploy.sh and the CI build them.
tryRun('km data (apps/layout-tool/tools/data.mjs)', [join(REPO, 'apps/layout-tool/tools/data.mjs')])
tryRun('crossover module', [join(REPO, 'apps/layout-tool/tools/crossover.mjs')], { cwd: join(REPO, 'apps/layout-tool') })
tryRun('optimizer module', [join(REPO, 'apps/layout-tool/tools/optimizer.mjs')], { cwd: join(REPO, 'apps/layout-tool') })
// The simulation module (sim/) beside the Fahrplan-Editor's page — as
// deploy/deploy.sh builds it; without it no running times can be computed.
tryRun('simulation module', [join(REPO, 'packages/running-times/scripts/sim.mjs')], { cwd: join(REPO, 'apps/timetable-editor') })

rmSync(SITE, { recursive: true, force: true })
mkdirSync(SITE, { recursive: true })
for (const [app, path] of MODULE_APPS) {
  console.log(`\n== ${app} → site/${path}/`)
  run([VITE, 'build', '--outDir', join(SITE, path), '--emptyOutDir'], { cwd: join(REPO, 'apps', app) })
}

// The start page at the root — built beside the site and merged in, leaving
// the modules' directories alone (as deploy.sh does with rsync --exclude).
console.log('\n== home → site/')
const tmp = mkdtempSync(join(tmpdir(), 'ra-desktop-home-'))
try {
  run([VITE, 'build', '--outDir', tmp, '--emptyOutDir'], { cwd: join(REPO, 'apps/home') })
  for (const name of readdirSync(tmp)) {
    if (MODULE_APPS.some(([, p]) => p === name)) continue
    cpSync(join(tmp, name), join(SITE, name), { recursive: true })
  }
} finally {
  rmSync(tmp, { recursive: true, force: true })
}

// The desktop shell's start screen: src-tauri loads splash.html from the site
// root; logoWords.js is the suite's wordmark it imports (apps/home/src).
cpSync(join(HERE, '../splash.html'), join(SITE, 'splash.html'))
cpSync(join(REPO, 'apps/home/src/logoWords.js'), join(SITE, 'logoWords.js'))

console.log(`\nsite built in ${SITE}`)
