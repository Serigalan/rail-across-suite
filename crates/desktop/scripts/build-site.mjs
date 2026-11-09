#!/usr/bin/env node
// Build the suite's site for the desktop app: the start page at the root of
// crates/desktop/site, each module's app under its path — the layout
// deploy/deploy.sh builds into RA_SITE_DIR (MODULE_APPS in deploy/lib.sh) —
// with the core's WebAssembly module and the modules' Rust modules beside
// their pages. The app serves it from its own process (src/main.rs).
//
// Run after `npm ci` at the repository root:   node scripts/build-site.mjs
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const CRATE = resolve(HERE, '..')
const REPO = resolve(CRATE, '../..')
const SITE = join(CRATE, 'site')
const VITE = join(REPO, 'node_modules/vite/bin/vite.js')

// "<app directory under apps/>:<path in the site>" — as deploy/lib.sh MODULE_APPS.
const MODULE_APPS = [
  ['layout-tool', 'layout'], ['signalling-tool', 'signalling'], ['simulation-tool', 'simulation'],
  ['vehicle-editor', 'vehicles'], ['timetable-editor', 'timetable'],
]

function run(args, opts) {
  const r = spawnSync(process.execPath, args, { stdio: 'inherit', ...opts })
  if (r.status !== 0) throw new Error(`failed: node ${args.join(' ')}`)
}

/** Optional beside a page: warn and go on where it is missing. */
function tryRun(label, args, opts) {
  const r = spawnSync(process.execPath, args, { stdio: 'inherit', ...opts })
  if (r.status !== 0) console.error(`WARNING: ${label} failed — the app builds without it, its functions say the module is missing`)
}

if (!existsSync(VITE)) {
  console.error('no vite found — run `npm ci` at the repository root first')
  process.exit(1)
}

tryRun('km data', [join(REPO, 'apps/layout-tool/tools/data.mjs')], { cwd: join(REPO, 'apps/layout-tool') })
const coreDirs = ['apps/home/public/core-module', ...MODULE_APPS.map(([app]) => `apps/${app}/public/core-module`)]
tryRun('core module', [join(REPO, 'packages/core/scripts/wasm.mjs'), ...coreDirs], { cwd: REPO })
tryRun('crossover module', [join(REPO, 'apps/layout-tool/tools/crossover.mjs')], { cwd: join(REPO, 'apps/layout-tool') })
tryRun('optimizer module', [join(REPO, 'apps/layout-tool/tools/optimizer.mjs')], { cwd: join(REPO, 'apps/layout-tool') })
tryRun('simulation module', [join(REPO, 'packages/running-times/scripts/sim.mjs')], { cwd: join(REPO, 'apps/timetable-editor') })
// The point cloud import, which the Layout-Tool's worker bundles: its build needs it.
run([join(REPO, 'packages/pointcloud/scripts/wasm.mjs')], { cwd: REPO })

rmSync(SITE, { recursive: true, force: true })
mkdirSync(SITE, { recursive: true })
for (const [app, path] of MODULE_APPS) {
  console.log(`\n== ${app} → site/${path}/`)
  run([VITE, 'build', '--outDir', join(SITE, path), '--emptyOutDir'], { cwd: join(REPO, 'apps', app) })
}

// The start page at the root, merged in beside the modules (as deploy.sh does with rsync --exclude).
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

// The start screen's wordmark, from the start page's source.
cpSync(join(REPO, 'apps/home/src/logoWords.js'), join(CRATE, 'ui/logoWords.js'))
console.log(`\nsite built in ${SITE}`)
