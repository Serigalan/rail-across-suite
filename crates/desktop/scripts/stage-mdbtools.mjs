#!/usr/bin/env node
// mdb-export (mdbtools) bundled with the desktop app, for the MDB import
// without anything installed: the binary and the libraries it needs that a
// plain system does not have (libmdb, libglib on macOS …) copied into
// crates/desktop/mdbtools/ (mdb-export, lib/). The server sets the library
// path for it (RA_MDB_LIBS). Where mdb-export is not installed here, the app
// is built without it and the MDB import says the tool is missing.
//
//   node scripts/stage-mdbtools.mjs [path to mdb-export]
import { execFileSync } from 'node:child_process'
import { chmodSync, copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const OUT = resolve(fileURLToPath(new URL('.', import.meta.url)), '../mdbtools')
const which = (cmd) => { try { return execFileSync(process.platform === 'win32' ? 'where' : 'which', [cmd], { encoding: 'utf8' }).split('\n')[0].trim() } catch { return null } }
const tool = process.argv[2] ?? process.env.RA_MDB_EXPORT ?? which('mdb-export')

rmSync(OUT, { recursive: true, force: true })
mkdirSync(join(OUT, 'lib'), { recursive: true })
if (!tool || !existsSync(tool)) {
  console.error('WARNING: no mdb-export found — the app is built without it (install mdbtools to bundle it)')
  process.exit(0)
}
const exe = process.platform === 'win32' ? 'mdb-export.exe' : 'mdb-export'
copyFileSync(tool, join(OUT, exe))
chmodSync(join(OUT, exe), 0o755)

// The shared libraries it links, but not those every system has.
const SYSTEM = /^(libc|libm|libpthread|libdl|librt|ld-linux|linux-vdso|libgcc_s|libstdc\+\+)[.-]/
let libs = []
if (process.platform === 'linux') {
  libs = execFileSync('ldd', [tool], { encoding: 'utf8' }).split('\n')
    .map(l => /=>\s*(\S+)/.exec(l)?.[1]).filter(p => p && existsSync(p) && !SYSTEM.test(basename(p)))
} else if (process.platform === 'darwin') {
  libs = execFileSync('otool', ['-L', tool], { encoding: 'utf8' }).split('\n').slice(1)
    .map(l => l.trim().split(' ')[0]).filter(p => p && !p.startsWith('/usr/lib/') && !p.startsWith('/System/') && existsSync(p))
}
// Writable copies: Homebrew installs its libraries read-only, and Tauri's build
// copies the resources over the ones of the build before.
for (const lib of libs) {
  copyFileSync(lib, join(OUT, 'lib', basename(lib)))
  chmodSync(join(OUT, 'lib', basename(lib)), 0o644)
}
console.log(`mdb-export staged in ${OUT} with ${libs.length} libraries: ${libs.map(l => basename(l)).join(', ')}`)
