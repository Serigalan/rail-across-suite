import { describe, it, expect } from 'vitest'
import { existsSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import { readCloudHeader, readCloudPoints } from './cloudReader.js'
import { nodeFileSource, nodeLazPerf } from '../test/pointCloudFixture.js'

// The Rust crate (crates/pointcloud, decision 0020 step 7) against this
// package on real survey data, which stays out of the repository like the PEK
// file: LAZ 1.4, point format 6, decoded by laz-rs there and laz-perf here.
// Needs the crate's command line built (cargo build --release -p
// rail-across-pointcloud); skipped where it or the file is not there.

const BIN = resolve(import.meta.dirname, '../../../target/release/ra-pointcloud')
const LAZ = process.env.RA_LAZ_SAMPLE ?? join(homedir(), '.claude/projects/-root-open-layout-tool/memory/testdata/5550L_00000500_DB_Ref_4.laz')
const ready = existsSync(BIN) && existsSync(LAZ)

const rust = (op, input) => {
  const r = spawnSync(BIN, { input: JSON.stringify({ op, input }), maxBuffer: 1 << 30 })
  const answer = JSON.parse(r.stdout.toString())
  if (answer.error) throw new Error(answer.error)
  return answer.ok
}

function summary(b) {
  let x = 0, y = 0, z = 0, i = 0
  const sample = []
  for (let k = 0; k < b.count; k++) {
    x += b.x[k]; y += b.y[k]; z += b.z[k]; i += b.intensity[k]
    if (k % 97 === 0) sample.push([k, b.x[k], b.y[k], b.z[k], b.intensity[k]])
  }
  return { count: b.count, x, y, z, i, rgb: null, sample }
}

describe.skipIf(!ready)('the point cloud crate on a real LAZ file', () => {
  it('reads its header and its first chunks point for point as laz-perf does', async () => {
    const source = await nodeFileSource(LAZ)
    try {
      const header = await readCloudHeader(source)
      expect(rust('pointcloud.readCloudHeader', [{ path: LAZ }])).toEqual(JSON.parse(JSON.stringify(header)))
      const batches = []
      const lazPerf = await nodeLazPerf()
      for await (const b of readCloudPoints(source, header, { lazPerf })) {
        batches.push(summary(b))
        if (batches.length >= 20) break
      }
      expect(rust('pointcloud.readPointsSummary', [{ path: LAZ }, 20]).batches).toEqual(batches)
    } finally {
      await source.close()
    }
  }, 120000)
})
