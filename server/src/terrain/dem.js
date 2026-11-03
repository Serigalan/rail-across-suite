import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { crc32, deflateSync } from 'node:zlib'
import decodeWebp, { init as initWebp } from '@jsquash/webp/decode.js'
import { SOURCES, terrainStore, toPlane } from './index.js'
import { download } from './store.js'

// The Länder's DGM1 as map tiles: terrain-RGB for the map's fine relief. The
// map colours and shades the ground from raster-dem tiles — 256 × 256 pixels
// in Web Mercator, each pixel a height packed into its red, green and blue.
// DGM5 (BKG) comes that way ready-made; the Länder's DGM1 does not, it lies in
// the kilometre tiles `sample` reads for single points. So this draws the map
// tile from those: the height of every pixel's centre, read bilinearly from
// the DGM1 tile it falls in.
//
//     z 16  ~ 1.5 m a pixel in Germany, a map tile ~ 385 m — the coarsest the
//           1 m grid is worth drawing; below it DGM5 (5 m) shows as much
//     z 17  ~ 0.75 m a pixel, the finest; the map scales it up beyond
//
// Heights are packed to the centimetre (MapLibre's `custom` encoding),
// h = (R · 65536 + G · 256 + B) · 0.01 − 10000: the 0.1 m of the usual
// terrain-RGB would draw terraces into the hillshade of flat ground.
//
// A tile no Land has any of is no tile (null — answered 404, the map keeps
// showing DGM5 there). Pixels in a tile only partly covered — a Land's border,
// a DGM1 tile the portal did not hand out — take DGM5's height, so the relief
// runs on without a cliff; where even that is missing, the nearest pixel's.

const SIZE = 256
export const MIN_ZOOM = 16
export const MAX_ZOOM = 17
const BASE_SHIFT = 10000
const STEP = 0.01
// DGM5's tiles in the usual terrain-RGB (0.1 m), up to zoom 15 — lossless
// WebP of 512 × 512 pixels, though they are named .png.
const DGM5_URL = 'https://sg.geodatenzentrum.de/gdz_basemapde_3d_gelaende/dgm5_rgb_tiles/{z}/{x}/{y}.png'
const DGM5_MAX_ZOOM = 15
const DGM5_MEMORY = 64
// DGM1 tiles one map tile may need, over all Länder: at zoom 16 four at most.
const MAX_SOURCE_TILES = 16

/** WGS84 of the centre of each pixel column and row of map tile z/x/y. */
function pixelLngLat(z, x, y) {
  const n = 2 ** z
  const lng = []
  const lat = []
  for (let i = 0; i < SIZE; i++) {
    const t = (i + 0.5) / SIZE
    lng.push((x + t) / n * 360 - 180)
    lat.push(Math.atan(Math.sinh(Math.PI * (1 - 2 * (y + t) / n))) * 180 / Math.PI)
  }
  return { lng, lat }
}

/** { heights, failed }: the DGM1 height of every pixel (NaN where no Land has one), and whether a tile could not be had for now. */
async function dgm1Heights(z, x, y, store) {
  const { lng, lat } = pixelLngLat(z, x, y)
  const heights = new Float64Array(SIZE * SIZE).fill(NaN)
  let failed = false
  let wanted = 0
  for (const source of SOURCES) {
    const [w, s, e, n] = source.box
    const todo = new Map()      // tile key → [[pixel, east, north]]
    for (let r = 0; r < SIZE; r++) {
      if (lat[r] < s || lat[r] > n) continue
      for (let c = 0; c < SIZE; c++) {
        const i = r * SIZE + c
        if (!Number.isNaN(heights[i]) || lng[c] < w || lng[c] > e) continue
        const [east, north] = toPlane(source.crs, lng[c], lat[r])
        const key = source.tileOf(east, north).join('_')
        if (!todo.has(key)) todo.set(key, [])
        todo.get(key).push([i, east, north])
      }
    }
    wanted += todo.size
    if (wanted > MAX_SOURCE_TILES) throw new Error('map tile too large for the DGM1')
    for (const [key, pixels] of todo) {
      const [ek, nk] = key.split('_').map(Number)
      const { tile, failed: tileFailed } = await store.get(source, ek, nk)
      failed ||= tileFailed
      if (!tile) continue
      for (const [i, east, north] of pixels) heights[i] = tile.height(east, north) ?? NaN
    }
  }
  return { heights, failed }
}

let webpReady = null
async function decodeDgm5(bytes) {
  webpReady ??= (async () => {
    const require = createRequire(import.meta.url)
    await initWebp(await WebAssembly.compile(readFileSync(require.resolve('@jsquash/webp/codec/dec/webp_dec.wasm'))))
  })()
  await webpReady
  const img = await decodeWebp(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength))
  const out = new Float64Array(img.width * img.height)
  for (let i = 0; i < out.length; i++) {
    const raw = img.data[4 * i] * 65536 + img.data[4 * i + 1] * 256 + img.data[4 * i + 2]
    // DGM5 marks "no data" as exactly 0 m.
    out[i] = raw === 100000 ? NaN : raw * 0.1 - 10000
  }
  return { grid: out, size: img.width }
}

/** DGM5 tiles, decoded to heights, the last few kept. */
export class Dgm5 {
  constructor(fetch = download) {
    this.fetch = fetch
    this.memory = new Map()
  }

  async tile(z, x, y) {
    const key = `${z}/${x}/${y}`
    if (this.memory.has(key)) return this.memory.get(key)
    let grid
    try {
      const bytes = await this.fetch(DGM5_URL.replace('{z}', z).replace('{x}', x).replace('{y}', y))
      grid = bytes ? await decodeDgm5(bytes) : null
    } catch {
      return null      // not kept: the BKG may answer next time
    }
    this.memory.set(key, grid)
    while (this.memory.size > DGM5_MEMORY) this.memory.delete(this.memory.keys().next().value)
    return grid
  }

  /** DGM5 under map tile z/x/y at its pixel centres — beyond zoom 15 read bilinearly from the tile it lies in. */
  async heights(z, x, y) {
    const dz = Math.max(0, z - DGM5_MAX_ZOOM)
    const tile = await this.tile(z - dz, x >> dz, y >> dz)
    if (!tile) return null
    const k = 2 ** dz
    const { grid, size } = tile
    const at = (v, base) => Math.min(Math.max((v - (base << dz)) / k * size - 0.5, 0), size - 1)
    const out = new Float64Array(SIZE * SIZE)
    for (let r = 0; r < SIZE; r++) {
      const row = at(y + (r + 0.5) / SIZE, y >> dz)
      const r0 = Math.floor(row)
      const r1 = Math.min(r0 + 1, size - 1)
      const tr = row - r0
      for (let c = 0; c < SIZE; c++) {
        const col = at(x + (c + 0.5) / SIZE, x >> dz)
        const c0 = Math.floor(col)
        const c1 = Math.min(c0 + 1, size - 1)
        const tc = col - c0
        out[r * SIZE + c] = grid[r0 * size + c0] * (1 - tc) * (1 - tr) + grid[r0 * size + c1] * tc * (1 - tr)
          + grid[r1 * size + c0] * (1 - tc) * tr + grid[r1 * size + c1] * tc * tr
      }
    }
    return out
  }
}

/**
 * Every NaN set to the height of the nearest pixel that has one — the exact
 * Euclidean nearest (Felzenszwalb and Huttenlocher's distance transform: down
 * each column, then the lower envelope of parabolas along each row).
 */
function fillNearest(heights) {
  const INF = 1e20
  // Down each column: the nearest row with a height, and how far.
  const colRow = new Int32Array(SIZE * SIZE).fill(-1)
  for (let c = 0; c < SIZE; c++) {
    let last = -1
    for (let r = 0; r < SIZE; r++) {
      if (!Number.isNaN(heights[r * SIZE + c])) last = r
      colRow[r * SIZE + c] = last
    }
    last = -1
    for (let r = SIZE - 1; r >= 0; r--) {
      if (!Number.isNaN(heights[r * SIZE + c])) last = r
      const up = colRow[r * SIZE + c]
      if (last >= 0 && (up < 0 || last - r < r - up)) colRow[r * SIZE + c] = last
    }
  }
  const out = Float64Array.from(heights)
  const f = new Float64Array(SIZE)
  const v = new Int32Array(SIZE)
  const zb = new Float64Array(SIZE + 1)
  for (let r = 0; r < SIZE; r++) {
    for (let c = 0; c < SIZE; c++) {
      const near = colRow[r * SIZE + c]
      f[c] = near < 0 ? INF : (near - r) ** 2
    }
    // The lower envelope of the parabolas f(q) + (c − q)².
    let k = 0
    v[0] = 0
    zb[0] = -Infinity
    zb[1] = Infinity
    for (let q = 1; q < SIZE; q++) {
      let s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k])
      while (s <= zb[k]) {
        k--
        s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k])
      }
      k++
      v[k] = q
      zb[k] = s
      zb[k + 1] = Infinity
    }
    k = 0
    for (let c = 0; c < SIZE; c++) {
      while (zb[k + 1] < c) k++
      const i = r * SIZE + c
      if (Number.isNaN(heights[i]) && f[v[k]] < INF) out[i] = heights[colRow[r * SIZE + v[k]] * SIZE + v[k]]
    }
  }
  heights.set(out)
  return heights
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'latin1'), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([len, body, crc])
}

/** Heights (no NaN left) as a terrain-RGB PNG in the centimetre encoding. */
export function encode(heights) {
  const raw = Buffer.alloc(SIZE * (1 + 3 * SIZE))
  for (let r = 0; r < SIZE; r++) {
    const row = r * (1 + 3 * SIZE)
    for (let c = 0; c < SIZE; c++) {
      const v = Math.min(Math.max(Math.round((heights[r * SIZE + c] + BASE_SHIFT) / STEP), 0), 2 ** 24 - 1)
      raw[row + 1 + 3 * c] = (v >> 16) & 255
      raw[row + 2 + 3 * c] = (v >> 8) & 255
      raw[row + 3 + 3 * c] = v & 255
    }
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(SIZE, 0)
  header.writeUInt32BE(SIZE, 4)
  header[8] = 8      // bit depth
  header[9] = 2      // RGB
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0)),
  ])
}

/** Whether map tile z/x/y touches any Land's box at all. */
function covers(z, x, y) {
  const n = 2 ** z
  const west = x / n * 360 - 180
  const east = (x + 1) / n * 360 - 180
  const north = Math.atan(Math.sinh(Math.PI * (1 - 2 * y / n))) * 180 / Math.PI
  const south = Math.atan(Math.sinh(Math.PI * (1 - 2 * (y + 1) / n))) * 180 / Math.PI
  return SOURCES.some(({ box: [w, s, e, nb] }) => w <= east && west <= e && s <= north && south <= nb)
}

let sharedDgm5 = null

/**
 * { png, keep } for map tile z/x/y: the PNG, or null where no Land has any of
 * it, and whether the answer may be kept — not while a DGM1 tile it needs
 * could not be had.
 */
export async function render(z, x, y, store = terrainStore(), dgm5 = (sharedDgm5 ??= new Dgm5())) {
  if (!covers(z, x, y)) return { png: null, keep: true }
  const { heights, failed } = await dgm1Heights(z, x, y, store)
  const gaps = heights.reduce((count, h) => count + (Number.isNaN(h) ? 1 : 0), 0)
  if (gaps === heights.length) return { png: null, keep: !failed }
  if (gaps) {
    const under = await dgm5.heights(z, x, y)
    if (under) for (let i = 0; i < heights.length; i++) if (Number.isNaN(heights[i])) heights[i] = under[i]
    fillNearest(heights)
  }
  return { png: encode(heights), keep: !failed }
}
