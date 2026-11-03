import { wgs84ToUTM } from '@rail-across/core/geometry'
import { SOURCES } from './sources.js'
import { TileStore } from './store.js'

// Ground heights from the DGM1 of the Länder, sampled here: the Länder publish
// their 1 m terrain models as tiles of one or two kilometres — zipped XYZ
// lists or GeoTIFFs of several megabytes each, and without the CORS header
// that would let a browser read them at all. So the page sends the points it
// wants a height for and the server answers them; it fetches the tile a point
// lies in once, keeps it, and reads every later point from there.
//
// A point outside every source, or in a tile no source has, answers null; the
// page then asks the next source it knows (DGM5, then the worldwide tiles).

export { SOURCES }

let shared = null
/** The server's one tile store. */
export const terrainStore = () => (shared ??= new TileStore())

/** A WGS84 point in the plane of a source. */
export function toPlane(crs, lng, lat) {
  const p = wgs84ToUTM([lng, lat], crs)
  return [p.easting, p.northing]
}

/**
 * Heights for WGS84 points [[lng, lat], …]: { heights, sources } — per point
 * the height in metres to the centimetre and the dataset it came from
 * ('dgm1-th 2020-2025', 'dgm1-by', …), or null for both where no Land's DGM1
 * has the point.
 */
export async function sample(lngLats, store = terrainStore()) {
  const heights = []
  const sources = []
  for (const p of lngLats) {
    let z = null
    let label = null
    const [lng, lat] = Array.isArray(p) ? [Number(p[0]), Number(p[1])] : [NaN, NaN]
    if (Number.isFinite(lng) && Number.isFinite(lat)) {
      for (const source of SOURCES) {
        if (!source.covers(lng, lat)) continue
        const [e, n] = toPlane(source.crs, lng, lat)
        const { tile } = await store.get(source, ...source.tileOf(e, n))
        const h = tile ? tile.height(e, n) : null
        if (h != null) {
          z = Math.round(h * 100) / 100
          label = tile.label
          break
        }
      }
    }
    heights.push(z)
    sources.push(label)
  }
  return { heights, sources }
}
