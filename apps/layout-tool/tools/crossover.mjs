// npm run crossover — puts the search of the extended switch connection
// beside the Layout-Tool's page, in public/crossover/ (or the directory
// named): crossover/ of the repository, built to WebAssembly with cargo
// (rustup, target wasm32-unknown-unknown). The app loads it as a file of its
// own; RA_CROSSOVER_WASM names a module file to take as it is.
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const out = resolve(process.argv[2] ?? join(here, '../public/crossover'))
const target = join(out, 'rail_across_crossover.wasm')
const crate = resolve(here, '../../../crossover')
const built = join(crate, 'target/wasm32-unknown-unknown/release/rail_across_crossover_wasm.wasm')

let source = process.env.RA_CROSSOVER_WASM ? resolve(process.env.RA_CROSSOVER_WASM) : null
if (!source) {
  try {
    // cargo as rustup installs it, where it is not on the PATH (a deploy's shell)
    const cargo = process.env.CARGO ?? (existsSync(join(homedir(), '.cargo/bin/cargo')) ? join(homedir(), '.cargo/bin/cargo') : 'cargo')
    execFileSync(cargo, ['build', '--release', '-p', 'rail-across-crossover-wasm', '--target', 'wasm32-unknown-unknown'], { cwd: crate, stdio: 'inherit' })
  } catch (err) {
    if (!existsSync(built)) {
      console.error(`could not build the crossover module in ${crate}: ${err.message}`)
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
  revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: crate, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim()
} catch { /* not a checkout */ }
writeFileSync(join(out, 'NOTICE.txt'), [
  'rail_across_crossover.wasm — the search of the extended switch connection of the Layout-Tool.',
  'Copyright 2026 Serigalan and the Rail-Across-Suite contributors.',
  'Licensed under the GNU Lesser General Public License v3.0 only (LGPL-3.0-only),',
  'beside it in LICENSE.txt, with the GNU GPL v3.0 it builds on in LICENSE-GPL.txt.',
  'Source: https://github.com/Serigalan/rail-across-suite, crossover/' + (revision ? ` at ${revision}` : ''),
  '',
  'It contains the crate slsqp (https://github.com/relf/slsqp), © 2023 Rémi Lafage, under the MIT licence',
  'in LICENSE-slsqp.txt: a translation of the SLSQP of NLopt (S. G. Johnson), from Dieter Kraft\'s',
  'ACM Algorithm 733, released by the ACM under the BSD licence.',
  '',
].join('\n'))
copyFileSync(resolve(crate, '../LICENSE'), join(out, 'LICENSE.txt'))
copyFileSync(resolve(crate, '../LICENSE-GPL'), join(out, 'LICENSE-GPL.txt'))
copyFileSync(resolve(crate, 'LICENSES/slsqp-MIT.txt'), join(out, 'LICENSE-slsqp.txt'))
