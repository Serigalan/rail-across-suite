// npm run wasm — builds the core's Rust crates as one WebAssembly module
// (crates/core-wasm; decision 0005) and puts it in packages/core/wasm/, where
// the core's face loads it from in Node (src/native), and in every directory
// named — an app's public/core-module/, which its page loads it from.
// RA_CORE_WASM names a module file to take as it is. Without a module the core
// runs on its JavaScript.
//
//   node scripts/wasm.mjs [dir …]
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repo = resolve(here, '../../..')
const out = resolve(here, '../wasm')
const built = join(repo, 'target/wasm32-unknown-unknown/release/rail_across_core_wasm.wasm')

let source = process.env.RA_CORE_WASM ? resolve(process.env.RA_CORE_WASM) : null
if (!source) {
  try {
    // cargo as rustup installs it, where it is not on the PATH (a deploy's shell)
    const cargo = process.env.CARGO ?? (existsSync(join(homedir(), '.cargo/bin/cargo')) ? join(homedir(), '.cargo/bin/cargo') : 'cargo')
    execFileSync(cargo, ['build', '--release', '-p', 'rail-across-core-wasm', '--target', 'wasm32-unknown-unknown'], { cwd: repo, stdio: 'inherit' })
  } catch (err) {
    if (!existsSync(built)) {
      console.error(`could not build the core's module in ${repo}: ${err.message}`)
      process.exit(1)
    }
    console.warn('cargo failed; taking the module built before')
  }
  source = built
}
for (const dir of [out, ...process.argv.slice(2).map(d => resolve(d))]) {
  mkdirSync(dir, { recursive: true })
  copyFileSync(source, join(dir, 'rail_across_core.wasm'))
  console.log(`core module: ${join(dir, 'rail_across_core.wasm')}`)
}
