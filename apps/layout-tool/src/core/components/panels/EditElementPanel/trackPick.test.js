import { describe, expect, it } from 'vitest'
import { utmToWgs84 } from '@rail-across/core/geometry'
import { passPoint, shiftBetween, SNAP_PX } from './trackPick'

// One straight in UTM 32 from (691000, 5711000) to (691300, 5711000); the
// screen is a plain scaling of WGS84, so pixels are easy to say.
const EPSG = 25832
const a = utmToWgs84(691000, 5711000, EPSG), b = utmToWgs84(691300, 5711000, EPSG)
const track = { id: 't', epsg: EPSG, elements: [{ startNode: [691000, 5711000], endNode: [691300, 5711000], geometry: { coordinates: [a, b] } }] }
const toScreen = ([lon, lat]) => ({ x: lon * 1e5, y: -lat * 1e5 })
const clickAt = (wgs, dxPx = 0) => ({ lngLat: wgs, point: { x: toScreen(wgs).x + dxPx, y: toScreen(wgs).y } })

describe('pass points', () => {
  it('takes a node near the click, exactly as the plane has it', () => {
    const p = passPoint([track], EPSG, clickAt(b, SNAP_PX - 2), toScreen)
    expect(p).toMatchObject({ easting: 691300, northing: 5711000, snapped: true, wgs: b })
  })

  it('takes the clicked point where no node is near', () => {
    const far = utmToWgs84(691150, 5711040, EPSG)
    const p = passPoint([track], EPSG, clickAt(far), toScreen)
    expect(p.snapped).toBe(false)
    expect(p.easting).toBeCloseTo(691150, 3)
    expect(p.northing).toBeCloseTo(5711040, 3)
  })

  it('gives the shift from one to the other to the micrometre', () => {
    expect(shiftBetween({ easting: 691000, northing: 5711000 }, { easting: 691012.3456789, northing: 5710996.75 }))
      .toEqual({ dx: 12.345679, dy: -3.25 })
  })
})
