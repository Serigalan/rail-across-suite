// npm run sim, in an app that runs trains (the Simulation-Tool, the Fahrplan-
// Editor) — puts the simulation module beside the app's page, in public/sim/
// of the directory it is run in (or the one named): OSRD's simulation core
// ported to Rust (sim/, decision 0013), built to WebAssembly with cargo. The
// app loads it as a file of its own, which can be swapped for another build
// of it — RA_SIM_WASM names a module file to take as it is.
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const out = resolve(process.argv[2] ?? 'public/sim')
const target = join(out, 'rail_across_sim.wasm')
const sim = resolve(here, '../../../sim')
const built = join(sim, 'target/wasm32-unknown-unknown/release/rail_across_sim_wasm.wasm')

let source = process.env.RA_SIM_WASM ? resolve(process.env.RA_SIM_WASM) : null
if (!source) {
  try {
    // cargo as rustup installs it, where it is not on the PATH (a deploy's shell)
    const cargo = process.env.CARGO ?? (existsSync(join(homedir(), '.cargo/bin/cargo')) ? join(homedir(), '.cargo/bin/cargo') : 'cargo')
    execFileSync(cargo, ['build', '--release', '-p', 'rail-across-sim-wasm', '--target', 'wasm32-unknown-unknown'], { cwd: sim, stdio: 'inherit' })
  } catch (err) {
    if (!existsSync(built)) {
      console.error(`could not build the simulation module in ${sim}: ${err.message}`)
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
  revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: sim, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim()
} catch { /* not a checkout */ }
writeFileSync(join(out, 'NOTICE.txt'), [
  'rail_across_sim.wasm — the running time simulation of OSRD',
  '(https://github.com/OpenRailAssociation/osrd, core/) ported to Rust.',
  'Copyright 2022 The OSRD Contributors; 2026 Serigalan and the Rail-Across-Suite contributors.',
  'Licensed under the GNU Lesser General Public License v3.0 only (LGPL-3.0-only).',
  'Source: https://github.com/Serigalan/rail-across-suite, sim/' + (revision ? ` at ${revision}` : ''),
  '',
].join('\n'))
console.log(`simulation module: ${target}`)
