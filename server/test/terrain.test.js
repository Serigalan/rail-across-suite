import { afterEach, describe, expect, it } from 'vitest'
import { mkdtempSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { inflateSync } from 'node:zlib'
import { zipSync, strToU8 } from 'fflate'
import { writeArrayBuffer } from 'geotiff'
import { parseTif, parseXyz, readTile } from '../src/terrain/tile.js'
import { TileStore } from '../src/terrain/store.js'
import { SOURCES, sample, toPlane } from '../src/terrain/index.js'
import { render } from '../src/terrain/dem.js'
import { parseCsv } from '../src/mdb.js'
import { setup, signIn } from './helpers.js'

const TH = SOURCES.find(s => s.id === 'dgm1-th')

/** An XYZ list of a `size` m square from (e0, n0) on half metres, heights h(e, n). */
function xyz(e0, n0, size, h) {
  const lines = []
  for (let n = n0 + size - 0.5; n > n0; n -= 1) {
    for (let e = e0 + 0.5; e < e0 + size; e += 1) lines.push(`${e} ${n} ${h(e, n).toFixed(3)}`)
  }
  return lines.join('\n')
}

const dirs = []
const tmp = () => {
  const d = mkdtempSync(join(tmpdir(), 'ra-terrain-'))
  dirs.push(d)
  return d
}
let ctx
afterEach(async () => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
  await ctx?.app.close()
  ctx?.db.close()
  ctx = null
})

describe('a DGM1 tile', () => {
  it('is placed by the coordinates of an XYZ list and read bilinearly', () => {
    const tile = parseXyz(xyz(1000, 2000, 4, (e, n) => e + 10 * n), 'x')
    expect([tile.rows, tile.cols, tile.x0, tile.y0]).toEqual([4, 4, 1000.5, 2003.5])
    expect(tile.height(1001.25, 2001.75)).toBeCloseTo(1001.25 + 20017.5, 3)
    // Clamped at the edge, not extrapolated.
    expect(tile.height(999, 2001.5)).toBeCloseTo(1000.5 + 20015, 3)
  })

  it('has no height where a cell with a say in it has no data', () => {
    const tile = parseXyz(xyz(0, 0, 3, (e) => (e > 2 ? -9999 : 5)), 'x')
    expect(tile.height(1, 1)).toBe(5)
    expect(tile.height(2.2, 1)).toBeNull()
  })

  it('is placed by the tie point of a GeoTIFF, corner or centre', async () => {
    const values = Array.from({ length: 6 }, (_, i) => i)
    const tif = (raster) => new Uint8Array(writeArrayBuffer(values, {
      width: 3, height: 2, ModelPixelScale: [1, 1, 0], ModelTiepoint: [0, 0, 0, 500, 1002, 0],
      GTRasterTypeGeoKey: raster, GeographicTypeGeoKey: 4326,
    }))
    const area = await parseTif(tif(1), 'a')
    expect([area.rows, area.cols, area.x0, area.y0]).toEqual([2, 3, 500.5, 1001.5])
    expect(area.height(501.5, 1000.5)).toBe(4)
    const point = await parseTif(tif(2), 'p')
    expect([point.x0, point.y0]).toEqual([500, 1002])
  })

  it('comes out of a ZIP of several lists as one grid', async () => {
    const zip = zipSync({ 'a.xyz': strToU8(xyz(0, 0, 2, () => 1)), 'b.xyz': strToU8(xyz(2, 0, 2, () => 2)) })
    const tile = await readTile(zip, 'xyz-zip', 'z')
    expect([tile.rows, tile.cols]).toEqual([2, 4])
    expect(tile.height(3, 1)).toBe(2)
  })
})

describe('the tile store', () => {
  it('downloads once, keeps the tile on disk, and remembers a tile no source has', async () => {
    const asked = []
    const fetch = async (url) => {
      asked.push(url)
      return url.includes('2020-2025') && url.includes('_640_5700_') ? zipSync({ 't.xyz': strToU8(xyz(640000, 5700000, 3, () => 7)) }) : null
    }
    const cacheDir = tmp()
    const store = new TileStore({ cacheDir, fetch })
    const [a, b] = await Promise.all([store.get(TH, 640, 5700), store.get(TH, 640, 5700)])
    expect(a.tile.label).toBe('dgm1-th 2020-2025')
    expect(b.tile).toBe(a.tile)
    expect(asked).toHaveLength(1)
    expect(readdirSync(cacheDir)).toEqual(['dgm1-th_640_5700.dgm1'])
    const again = await new TileStore({ cacheDir, fetch }).get(TH, 640, 5700)
    expect(again.tile.height(640001, 5700001)).toBe(7)
    expect(asked).toHaveLength(1)
    // Both vintages asked, neither has it: not asked again for a day.
    expect((await store.get(TH, 1, 1)).tile).toBeNull()
    expect((await store.get(TH, 1, 1)).failed).toBe(false)
    expect(asked).toHaveLength(3)
  })

  it('says a download failed rather than that there is no tile, and asks again', async () => {
    let calls = 0
    const store = new TileStore({ cacheDir: tmp(), fetch: async () => { calls++; throw new Error('portal down') } })
    expect(await store.get(TH, 2, 2)).toEqual({ tile: null, failed: true })
    await store.get(TH, 2, 2)
    expect(calls).toBe(2)
  })
})

describe('heights and relief tiles', () => {
  // A plane rising to the east, wherever Thüringen is asked.
  const plane = {
    async get(source, ek, nk) {
      if (source.id !== 'dgm1-th') return { tile: null, failed: false }
      return { tile: parseXyz(xyz(ek * 1000, nk * 1000, 1000, (e) => 100 + (e - ek * 1000) / 100), 'dgm1-th plane'), failed: false }
    },
  }
  const memo = new Map()
  const store = { get: async (s, ek, nk) => { const k = `${s.id}${ek}_${nk}`; if (!memo.has(k)) memo.set(k, await plane.get(s, ek, nk)); return memo.get(k) } }

  it('answers a point in Thüringen from its tile, and nothing far from every Land', async () => {
    const [e] = toPlane(25832, 11.97, 51.48)
    const { heights, sources } = await sample([[11.97, 51.48], [-3, 40], ['x']], store)
    expect(heights[0]).toBeCloseTo(100 + (e - Math.floor(e / 1000) * 1000) / 100, 1)
    expect(sources).toEqual(['dgm1-th plane', null, null])
    expect(heights.slice(1)).toEqual([null, null])
  })

  it('draws a relief tile as terrain-RGB to the centimetre', async () => {
    // Zoom 16 over Weimar.
    const z = 16
    const x = Math.floor((11.33 + 180) / 360 * 2 ** z)
    const y = Math.floor((1 - Math.asinh(Math.tan(50.98 * Math.PI / 180)) / Math.PI) / 2 * 2 ** z)
    const { png, keep } = await render(z, x, y, store, { heights: async () => null })
    expect(keep).toBe(true)
    expect([...png.subarray(1, 4)].map(c => String.fromCharCode(c)).join('')).toBe('PNG')
    // The one IDAT, unfiltered rows of RGB.
    const at = png.indexOf('IDAT')
    const raw = inflateSync(png.subarray(at + 4, at + 4 + png.readUInt32BE(at - 4)))
    const h = (r, c) => (raw[r * 769 + 1 + 3 * c] * 65536 + raw[r * 769 + 2 + 3 * c] * 256 + raw[r * 769 + 3 + 3 * c]) * 0.01 - 10000
    expect(h(0, 0)).toBeGreaterThan(100)
    expect(h(0, 0)).toBeLessThan(110)
    expect(h(128, 255)).toBeGreaterThan(h(128, 0))
    // Nowhere near a Land: no tile.
    expect((await render(16, 0, 0, store)).png).toBeNull()
  })
})

describe('the MDB conversion', () => {
  it('reads mdb-export\'s CSV, quotes, doubled quotes and line breaks in fields', () => {
    expect(parseCsv('A,B,C\n1,"x, ""y""",\n2,"two\nlines",3\n')).toEqual([
      { A: '1', B: 'x, "y"', C: '' }, { A: '2', B: 'two\nlines', C: '3' },
    ])
  })

  it('takes an Access file as bytes, and refuses one that is none', async () => {
    ctx = await setup()
    const max = await signIn(ctx.app, 'max')
    const post = (headers) => ctx.app.inject({
      method: 'POST', url: '/api/mdb', cookies: { ra_session: max.token }, payload: Buffer.from('not a database at all'), headers,
    })
    expect((await post({ 'content-type': 'application/octet-stream' })).statusCode).toBe(415)
    const res = await post({ 'content-type': 'application/octet-stream', 'x-olt-upload': '1' })
    expect([res.statusCode, res.json().error]).toEqual([400, 'not_a_database'])
  })

  it('asks for a session', async () => {
    ctx = await setup()
    const res = await ctx.app.inject({ method: 'POST', url: '/api/terrain', payload: { lnglat: [[11, 51]] } })
    expect(res.statusCode).toBe(401)
    const bad = await (await signIn(ctx.app, 'max'))('GET', '/api/dem/12/0/0.png')
    expect(bad.statusCode).toBe(400)
  })
})
