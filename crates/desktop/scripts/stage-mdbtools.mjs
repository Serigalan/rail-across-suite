#!/usr/bin/env node
// mdb-export (mdbtools) bundled with the desktop app, for the MDB import
// without anything installed: the binary and the libraries it needs that a
// plain system does not have (libmdb, libglib on macOS …) copied into
// crates/desktop/mdbtools/ (mdb-export, lib/). The server sets the library
// path for it (RA_MDB_LIBS; on Windows PATH). Where mdb-export is not
// installed here, the app is built without it and the MDB import says the tool
// is missing.
//
// On Windows mdb-export comes from MSYS2 (pacman -S mingw-w64-x86_64-mdbtools,
// C:\msys64\mingw64\bin\mdb-export.exe): MSYS2's ldd (RA_LDD, default
// C:\msys64\usr\bin\ldd.exe) names the DLLs it needs, those beside it are copied.
//
//   node scripts/stage-mdbtools.mjs [path to mdb-export]
import { execFileSync } from 'node:child_process'
import { chmodSync, copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs'
import { basename, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const OUT = resolve(fileURLToPath(new URL('.', import.meta.url)), '../mdbtools')
const which = (cmd) => { try { return execFileSync(process.platform === 'win32' ? 'where' : 'which', [cmd], { encoding: 'utf8' }).split('\n')[0].trim() } catch { return null } }
const MSYS = 'C:\\msys64'
const tool = process.argv[2] ?? process.env.RA_MDB_EXPORT ?? which('mdb-export')
  ?? (process.platform === 'win32' && existsSync(join(MSYS, 'mingw64/bin/mdb-export.exe')) ? join(MSYS, 'mingw64/bin/mdb-export.exe') : null)

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
  // otool names only what a file links itself (unlike ldd): followed through the libraries, as libglib needs
  // libintl and libpcre2 on a Mac without Homebrew. @loader_path is the linking file's folder.
  const linked = (file) => execFileSync('otool', ['-L', file], { encoding: 'utf8' }).split('\n').slice(1)
    .map(l => l.trim().split(' ')[0].replace(/^@loader_path\//, dirname(file) + '/'))
    .filter(p => p && !p.startsWith('/usr/lib/') && !p.startsWith('/System/') && existsSync(p))
  const seen = new Set()
  for (const queue = linked(tool); queue.length;) {
    const lib = queue.shift()
    if (seen.has(basename(lib))) continue
    seen.add(basename(lib))
    libs.push(lib)
    queue.push(...linked(lib))
  }
} else if (process.platform === 'win32') {
  // "libglib-2.0-0.dll => /mingw64/bin/libglib-2.0-0.dll (0x…)": Windows' own DLLs do not lie beside the tool.
  const ldd = process.env.RA_LDD ?? join(MSYS, 'usr/bin/ldd.exe')
  libs = execFileSync(ldd, [tool], { encoding: 'utf8' }).split('\n')
    .map(l => /^\s*(\S+\.dll)\s*=>/i.exec(l)?.[1]).filter(Boolean)
    .map(name => join(dirname(tool), name)).filter(p => existsSync(p))
}
// Writable copies: Homebrew installs its libraries read-only, and Tauri's build
// copies the resources over the ones of the build before.
for (const lib of libs) {
  copyFileSync(lib, join(OUT, 'lib', basename(lib)))
  chmodSync(join(OUT, 'lib', basename(lib)), 0o644)
}
console.log(`mdb-export staged in ${OUT} with ${libs.length} libraries: ${libs.map(l => basename(l)).join(', ')}`)
