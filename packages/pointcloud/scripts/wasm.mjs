// npm run wasm — builds the point cloud import as a WebAssembly module
// (crates/pointcloud-wasm; decision 0020) and puts it in wasm/, where the
// browser's import worker takes it from (bundled as an asset) and the tests
// load it in Node. RA_POINTCLOUD_WASM names a module file to take as it is.
//
//   node scripts/wasm.mjs
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repo = resolve(here, '../../..')
const out = resolve(here, '../wasm')
const built = join(repo, 'target/wasm32-unknown-unknown/release/rail_across_pointcloud_wasm.wasm')

let source = process.env.RA_POINTCLOUD_WASM ? resolve(process.env.RA_POINTCLOUD_WASM) : null
if (!source) {
  try {
    // cargo as rustup installs it, where it is not on the PATH (a deploy's shell)
    const cargo = process.env.CARGO ?? (existsSync(join(homedir(), '.cargo/bin/cargo')) ? join(homedir(), '.cargo/bin/cargo') : 'cargo')
    execFileSync(cargo, ['build', '--release', '-p', 'rail-across-pointcloud-wasm', '--target', 'wasm32-unknown-unknown'], { cwd: repo, stdio: 'inherit' })
  } catch (err) {
    if (!existsSync(built)) {
      console.error(`could not build the point cloud module in ${repo}: ${err.message}`)
      process.exit(1)
    }
    console.warn('cargo failed; taking the module built before')
  }
  source = built
}
mkdirSync(out, { recursive: true })
copyFileSync(source, join(out, 'rail_across_pointcloud.wasm'))
console.log(`point cloud module: ${join(out, 'rail_across_pointcloud.wasm')}`)
