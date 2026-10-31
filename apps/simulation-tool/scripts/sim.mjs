// npm run sim — puts the simulation module beside the page: rail-across-sim,
// OSRD's simulation core ported to Rust, built to WebAssembly. It is a
// project of its own under LGPL-3.0 (THIRD-PARTY.md); this tool loads it as a
// file of its own, which can be swapped for another build of it.
//
// From a checkout of rail-across-sim — RA_SIM_DIR, else beside this
// repository — it is built with cargo, or a module built there already is
// taken. RA_SIM_WASM names a module file to take as it is.
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const out = resolve(here, '../public/sim')
const target = join(out, 'rail_across_sim.wasm')
const repo = resolve(process.env.RA_SIM_DIR ?? resolve(here, '../../../../rail-across-sim'))
const built = join(repo, 'target/wasm32-unknown-unknown/release/rail_across_sim_wasm.wasm')

let source = process.env.RA_SIM_WASM ? resolve(process.env.RA_SIM_WASM) : null
if (!source) {
  if (!existsSync(join(repo, 'Cargo.toml'))) {
    console.error(`rail-across-sim not found at ${repo} (set RA_SIM_DIR, or RA_SIM_WASM to a built module)`)
    process.exit(1)
  }
  try {
    // cargo as rustup installs it, where it is not on the PATH (a deploy's shell)
    const cargo = process.env.CARGO ?? (existsSync(join(homedir(), '.cargo/bin/cargo')) ? join(homedir(), '.cargo/bin/cargo') : 'cargo')
    execFileSync(cargo, ['build', '--release', '-p', 'rail-across-sim-wasm', '--target', 'wasm32-unknown-unknown'], { cwd: repo, stdio: 'inherit' })
  } catch (err) {
    if (!existsSync(built)) {
      console.error(`could not build rail-across-sim: ${err.message}`)
      process.exit(1)
    }
    console.warn('cargo failed; taking the module built before')
  }
  source = built
}
mkdirSync(out, { recursive: true })
copyFileSync(source, target)
let revision = null
try {
  revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim()
} catch { /* not a checkout */ }
writeFileSync(join(out, 'NOTICE.txt'), [
  'rail_across_sim.wasm — rail-across-sim, the running time simulation of OSRD',
  '(https://github.com/OpenRailAssociation/osrd, core/) ported to Rust.',
  'Copyright 2022 The OSRD Contributors; 2026 Serigalan and the Rail-Across-Suite contributors.',
  'Licensed under the GNU Lesser General Public License v3.0 only (LGPL-3.0-only).',
  'Source: https://github.com/Serigalan/rail-across-sim' + (revision ? ` at ${revision}` : ''),
  '',
].join('\n'))
console.log(`simulation module: ${target}`)
