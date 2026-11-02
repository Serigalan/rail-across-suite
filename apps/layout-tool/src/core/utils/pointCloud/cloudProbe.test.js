import { describe, it, expect } from 'vitest'
import { probeExtent, projectPlane } from './cloudProbe'
import { utmToWgs84 } from '@rail-across/core/src/geometry/coordinateUtils.js'

describe('probeExtent', () => {
  const header = { min: [4470661, 5332178, 531], max: [4470713, 5332239, 538] }
  const trackAt = (crs, e, n) => ({ epsg: crs, coordinates: [utmToWgs84(e, n, crs), utmToWgs84(e + 100, n + 100, crs)] })

  it('accepts a file that lies at the tracks in the chosen system', () => {
    expect(probeExtent(header, 5678, [trackAt(5678, 4470600, 5332100)]).ok).toBe(true)
  })

  it('warns when the chosen system puts it somewhere else', () => {
    // The same numbers read as GK zone 3 land some 200 km to the west.
    expect(probeExtent(header, 5677, [trackAt(5678, 4470600, 5332100)]).ok).toBe(false)
  })

  it('cannot say anything for a project without tracks', () => {
    expect(probeExtent(header, 5678, []).ok).toBe(null)
  })

  it('keeps the tiles in the plane most tracks are in', () => {
    expect(projectPlane([{ epsg: 25832 }, { epsg: 5683 }, { epsg: 25832 }])).toBe(25832)
    expect(projectPlane([])).toBe(null)
  })
})
