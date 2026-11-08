import { readFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { loadNtv2Grid, nadgridsList } from './ntv2Grid.js'
import { transformPlanePoint, utmToWgs84, wgs84ToUTM } from './coordinateUtils.js'
import { callNative } from '../native/index.js'
import { loadNativeFile } from '../native/node.js'

// The proj4 port of the geometry crate (decision 0020, step 2) on the NTv2
// grids, held to proj4js: the grids are read once by the JavaScript and handed
// to the module, and both convert the same points — inside the grids, and past
// their edge, where both fall back to the 7-parameter shift.
/**
 * Alike to 4e-9 m — two units in the last place of 10⁷ m, which a difference of
 * two such numbers keeps — or to four units in their own last place: what a
 * chain of sin, cos and atan2 differing in its last digit between V8 and
 * Rust's libm leaves.
 */
const ulp = (x) => (x === 0 ? Number.MIN_VALUE : 2 ** (Math.floor(Math.log2(Math.abs(x))) - 52))
const off = (a, b) => (Math.abs(a - b) <= 4 * ulp(Math.max(Math.abs(a), Math.abs(b))) ? 0 : Math.abs(a - b))

// The grid files are data the Layout-Tool fetches (tools/data.mjs), not part of
// the core: found under RA_NTV2_GRIDS, else where the Layout-Tool keeps them.
const GRIDS = process.env.RA_NTV2_GRIDS ?? resolve(dirname(fileURLToPath(import.meta.url)), '../../../..', 'apps/layout-tool/grids')
const hasGrids = existsSync(join(GRIDS, 'de_adv_BETA2007.tif'))

describe.skipIf(!hasGrids || !loadNativeFile())('the proj4 port on the NTv2 grids', () => {
  const fetchBefore = globalThis.fetch
  beforeAll(async () => {
    globalThis.fetch = async (url) => {
      const name = String(url).split('/').pop()
      const bytes = readFileSync(join(GRIDS, name))
      return new Response(bytes)
    }
    await loadNtv2Grid()
  })
  afterAll(() => { globalThis.fetch = fetchBefore })

  it('has both base grids in force', () => {
    expect(nadgridsList('DHDN')).toBe('BETA2007')
    expect(callNative('geometry.nadgridsList', { datum: 'DHDN' })).toBe('BETA2007')
    expect(callNative('geometry.nadgridsList', { datum: 'PD83' })).toBe('NTV2TH')
  })

  // DHDN GK 2–5, PD/83 GK 3–4, Soldner Berlin; the last point of each lies past the grid.
  const CASES = [
    [5676, [[7.1, 50.2], [6.2, 51.1], [3.0, 45.0]]],
    [5677, [[9.0, 49.3], [10.1, 53.4], [9.0, 45.5]]],
    [5678, [[11.8, 48.1], [12.9, 54.0], [12.0, 46.0]]],
    [5679, [[14.6, 51.0], [14.2, 53.0], [16.5, 51.0]]],
    [3397, [[11.6, 50.9], [12.2, 51.3], [11.0, 49.0]]],
    [3396, [[10.2, 50.7], [9.9, 51.0], [8.0, 50.0]]],
    [3068, [[13.4, 52.5], [13.3, 52.4]]],
  ]

  it('projects into the planes alike, and back', () => {
    for (const [crs, points] of CASES) {
      for (const lngLat of points) {
        const js = wgs84ToUTM(lngLat, crs)
        const rust = callNative('geometry.wgs84ToUTM', { lngLat, crs })
        expect(off(rust.easting, js.easting), `${crs} ${lngLat}`).toBeLessThanOrEqual(4e-9)
        expect(off(rust.northing, js.northing), `${crs} ${lngLat}`).toBeLessThanOrEqual(4e-9)
        const back = utmToWgs84(js.easting, js.northing, crs)
        const rustBack = callNative('geometry.utmToWgs84', { easting: js.easting, northing: js.northing, crs })
        expect(off(rustBack[0], back[0])).toBeLessThan(1e-12)
        expect(off(rustBack[1], back[1])).toBeLessThan(1e-12)
      }
    }
  })

  // A point past a grid's edge, moved into a plane of another datum, makes
  // proj4js throw — its detour through WGS 84 hands `undefined` on rather than
  // NaN, so coordinateUtils never gets to convert it again without the grid.
  // The port does what coordinateUtils means to: the 7-parameter shift there.
  it('moves points between the planes alike, and past a grid where proj4js throws', () => {
    let thrown = 0
    for (const [crs, points] of CASES) {
      for (const lngLat of points) {
        const p = wgs84ToUTM(lngLat, crs)
        for (const to of [5683, 25832, 5677, 3397]) {
          const rust = callNative('geometry.transformPlanePoint', { easting: p.easting, northing: p.northing, fromCrs: crs, toCrs: to })
          let js
          try { js = transformPlanePoint(p.easting, p.northing, crs, to) } catch (err) {
            expect(err).toBeInstanceOf(TypeError)
            expect(rust.every(Number.isFinite), `${crs}→${to} ${lngLat}`).toBe(true)
            thrown += 1
            continue
          }
          expect(off(rust[0], js[0]), `${crs}→${to} ${lngLat}`).toBeLessThanOrEqual(4e-9)
          expect(off(rust[1], js[1]), `${crs}→${to} ${lngLat}`).toBeLessThanOrEqual(4e-9)
        }
      }
    }
    expect(thrown).toBeGreaterThan(0)
  })
})
