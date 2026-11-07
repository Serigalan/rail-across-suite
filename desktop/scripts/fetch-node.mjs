#!/usr/bin/env node
// Bundle a standalone Node.js binary into desktop/runtime/node/bin/node, so the
// packaged app (dmg/app bundle) and the zip package (scripts/pack.mjs) run on a
// machine without Node.js installed. runtime/ is carried as a Tauri bundle
// resource and shipped at the zip root, so the binary lands in both packages.
//
//   node scripts/fetch-node.mjs
//
// The binary comes from nodejs.org (the "latest" release of the major line the
// repository's engines field asks for; override with RA_NODE_VERSION=vX.Y.Z).
// If the download fails, the binary of the Node running this script is copied
// instead — as long as it satisfies engines. The result is verified by running
// `node --version`.
import { spawnSync } from 'node:child_process'
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = fileURLToPath(new URL('.', import.meta.url))
const REPO = resolve(HERE, '../..')
const OUT = resolve(HERE, '../runtime/node')
const BIN = process.platform === 'win32' ? 'node.exe' : 'node'

const DIST = 'https://nodejs.org/dist'
// nodejs.org archive naming per platform/arch ("win-x64", not "win32-x64").
const KEY = `${process.platform === 'win32' ? 'win' : process.platform}-${process.arch}`
const EXT = process.platform === 'win32' ? 'zip' : 'tar.xz'

// The major line the repository requires (root package.json: engines.node).
const engines = JSON.parse(readFileSync(join(REPO, 'package.json'), 'utf8')).engines?.node ?? '>=22'
const wantMajor = Number(engines.match(/\d+/)?.[0] ?? 22)

const majorOf = (v) => Number(v.match(/v?(\d+)/)?.[1])

async function resolveVersion() {
  if (process.env.RA_NODE_VERSION) return process.env.RA_NODE_VERSION
  const res = await fetch(`${DIST}/latest-v${wantMajor}.x/`)
  if (!res.ok) throw new Error(`cannot list ${DIST}/latest-v${wantMajor}.x/ (${res.status})`)
  const html = await res.text()
  const m = html.match(new RegExp(`node-(v\\d+\\.\\d+\\.\\d+)-${KEY.replace('-', '-')}\\.${EXT}`))
  if (!m) throw new Error(`no ${EXT} build for ${KEY} in latest-v${wantMajor}.x`)
  return m[1]
}

async function stage(version) {
  const tmp = mkdtempSync(join(tmpdir(), 'rail-across-node-'))
  try {
    const url = `${DIST}/${version}/node-${version}-${KEY}.${EXT}`
    console.log(`downloading ${url}`)
    const res = await fetch(url)
    if (!res.ok) throw new Error(`download failed: ${res.status}`)
    const archive = join(tmp, `node-${version}-${KEY}.${EXT}`)
    writeFileSync(archive, Buffer.from(await res.arrayBuffer()))
    // bsdtar (macOS/Windows) and GNU tar both auto-detect the compression.
    const t = spawnSync('tar', ['-xf', archive, '-C', tmp], { stdio: 'inherit' })
    if (t.status !== 0) throw new Error('tar failed')
    const src = join(tmp, `node-${version}-${KEY}`, 'bin', BIN)
    if (!existsSync(src)) throw new Error(`no bin/${BIN} in the archive`)
    mkdirSync(join(OUT, 'bin'), { recursive: true })
    rmSync(join(OUT, 'bin', BIN), { force: true })
    cpSync(src, join(OUT, 'bin', BIN))
  } finally {
    rmSync(tmp, { recursive: true, force: true })
  }
}

let done = false
try {
  const version = await resolveVersion()
  await stage(version)
  done = true
} catch (err) {
  console.warn(`download failed (${err.message}) — falling back to the Node running this script`)
  if (majorOf(process.version) < wantMajor) throw new Error(`local Node ${process.version} does not satisfy engines "${engines}"`)
  mkdirSync(join(OUT, 'bin'), { recursive: true })
  cpSync(process.execPath, join(OUT, 'bin', BIN))
  done = true
}

if (!done) throw new Error('no Node.js binary staged')
const bin = join(OUT, 'bin', BIN)
chmodSync(bin, 0o755)
const v = spawnSync(bin, ['--version'], { encoding: 'utf8' })
if (v.status !== 0 || majorOf(v.stdout.trim()) < wantMajor) {
  throw new Error(`staged ${bin} does not run or is too old: ${v.stdout} ${v.stderr}`)
}
console.log(`${bin}: ${v.stdout.trim()} (${(statSync(bin).size / 1e6).toFixed(1)} MB)`)
