import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { importLevels, importPointCloud, readCloudHeader, readFirstPoints } from './native.js'
import { decodeCloudSegment, segmentPlacement } from './tiles.js'
import { E57_FIXTURES, LAZ_PATH, bytesSource, makeLas, memoryWriter, nodeFileSource } from '../test/pointCloudFixture.js'

// The import module (crates/pointcloud-wasm) through its JavaScript face: the
// file read synchronously, the tiles written through the writers, progress
// posted. What it computes is crates/pointcloud's, held by its replay and its
// tests; here how the page talks to it.

/** Every point of a level's tiles, in the plane. */
function pointsOf(index, bytes) {
  const out = []
  for (const [tx, ty, segs] of index.tiles) {
    for (const [at, length, count, z0] of segs) {
      const s = decodeCloudSegment(index, bytes.subarray(at, at + length), count)
      const p = segmentPlacement(index, tx, ty, z0)
      for (let k = 0; k < count; k++) out.push([p.ox + s.x[k] * p.sx, p.oy + s.y[k] * p.sy, p.oz + s.z[k] * p.sz])
    }
  }
  return out
}

const PTS = Array.from({ length: 5000 }, (_, k) => ({
  x: 4470000 + (k % 100) * 0.07, y: 5332000 + Math.floor(k / 100) * 0.09, z: 530 + (k % 13) * 0.1, intensity: (k * 37) % 65536,
}))
const LAS = makeLas(PTS, { offset: [4470000, 5332000, 500] })

describe('the import module', () => {
  it('reads the header and the first points of a LAS file', () => {
    const source = bytesSource(LAS)
    const header = readCloudHeader(source)
    expect(header).toMatchObject({ pointCount: 5000, pointFormat: 1, compressed: false })
    expect(header.format).toBeUndefined()
    const first = readFirstPoints(source, 3)
    expect(first).toHaveLength(3)
    expect(first[1].x).toBeCloseTo(PTS[1].x, 3)
    expect(first[2].intensity).toBe(PTS[2].intensity)
  })

  it('imports the original resolution point for point, its tiles through the writer, with progress to the end', () => {
    const writer = memoryWriter()
    const seen = []
    const index = importPointCloud({ source: bytesSource(LAS), writer, original: true, onProgress: p => seen.push(p) })
    expect(index).toMatchObject({ level: 0, resolution: 'original', points: 5000, sourcePoints: 5000 })
    const pts = pointsOf(index, writer.bytes())
    expect(pts).toHaveLength(5000)
    const key = ([x, y, z]) => `${x.toFixed(3)} ${y.toFixed(3)} ${z.toFixed(3)}`
    expect(new Set(pts.map(key))).toEqual(new Set(PTS.map(p => key([p.x, p.y, p.z]))))
    const last = seen.at(-1)
    expect(last).toMatchObject({ points: 5000, totalPoints: 5000, kept: expect.any(Number) })
    expect(last.bytes).toBe(last.totalBytes)
    // The last tiles are written once the file is read, after its last progress.
    expect(last.bytesWritten).toBeLessThanOrEqual(index.bytes)
  })

  it('thins and converts the voxel levels in one pass, each into its own writer', () => {
    const writers = { 1: memoryWriter(), 3: memoryWriter() }
    const out = importLevels({ source: bytesSource(LAS), levels: [1, 3], writers, sourceCrs: 5678, targetCrs: 25832 })
    expect(Object.keys(out)).toEqual(['1', '3'])
    expect(out[1].points).toBeGreaterThan(out[3].points)
    // Gauss-Krüger zone 4 to UTM 32: the east moves by some 0.3 million metres.
    expect(out[1].bounds.minE).toBeLessThan(4300000)
    expect(pointsOf(out[1], writers[1].bytes())).toHaveLength(out[1].points)
  })

  it('reads E57 as well, told apart by its first bytes', () => {
    const source = bytesSource(new Uint8Array(readFileSync(`${E57_FIXTURES}scaled_pose.e57`)))
    expect(readCloudHeader(source)).toMatchObject({ format: 'e57', pointCount: 30000 })
    const index = importPointCloud({ source, writer: memoryWriter(), original: true })
    expect(index.points).toBe(28507)
  })

  it('says what is wrong with a file it cannot read', () => {
    expect(() => readCloudHeader(bytesSource(new Uint8Array(500).fill(7)))).toThrow(/not a LAS file/)
  })
})

describe.skipIf(!LAZ_PATH)('the import module on the LAZ sample', () => {
  it('decodes LAZ: every point of the file in the original level', () => {
    const source = nodeFileSource(LAZ_PATH)
    try {
      const header = readCloudHeader(source)
      expect(header.compressed).toBe(true)
      const index = importPointCloud({ source, writer: memoryWriter(), original: true })
      expect(index.points).toBe(header.pointCount)
    } finally {
      source.close()
    }
  }, 60000)
})
