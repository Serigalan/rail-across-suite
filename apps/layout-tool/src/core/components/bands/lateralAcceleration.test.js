import { describe, it, expect } from 'vitest'
import { bandSpecs } from './bandSpecs'
import { REFERENCE_VEHICLE, lateralInputsComplete } from './lateralAcceleration'

const t = (key) => key
const at = (band, s) => {
  for (let i = 1; i < band.length; i++) {
    const a = band[i - 1], b = band[i]
    if (s >= a.s && s <= b.s && b.s > a.s) return a.v + (b.v - a.v) * (s - a.s) / (b.s - a.s)
  }
  return null
}

/** 200 m straight, a 100 m transition into R 500 with 100 mm of cant, 300 m of arc, all at 100 km/h. */
const elements = [
  { elementType: 'straight', length: 200, speed: 100, cant: 0, start: { easting: 0, northing: 0 }, azimuth: 0 },
  { elementType: 'transition', length: 100, r1: null, r2: 500, speed: 100 },
  { elementType: 'arc', length: 300, radius: 500, cant: 100, speed: 100 },
]
const track = { id: 'x', name: 'Gl', epsg: 25832, elements }

describe('the lateral acceleration of the reference vehicle', () => {
  const v = 100 / 3.6
  const phi = Math.asin(0.1 / 1.5)

  it('is v²/R · cos φ − g · sin φ in the arc, where the cant does not change', () => {
    const { specs, order } = bandSpecs(track, t)
    expect(order).toContain('lateral')
    expect(at(specs.lateral.points, 450)).toBeCloseTo(v * v / 500 * Math.cos(phi) - 9.81 * Math.sin(phi), 6)
    expect(at(specs.lateral.points, 450)).toBeCloseTo(0.886, 3)
  })

  it('takes in the roll of the ramp, h · v² · Δφ′ / L, from half a vehicle before it begins', () => {
    const { specs } = bandSpecs(track, t)
    const { height: h, base: L } = REFERENCE_VEHICLE
    const roll = h * v * v * (phi / 100) / L
    // on the straight, 5 m before the ramp: no curve, no cant, only the roll
    expect(at(specs.lateral.points, 195)).toBeCloseTo(roll, 6)
    // farther back the vehicle does not roll yet
    expect(at(specs.lateral.points, 180)).toBeCloseTo(0, 9)
    // half a vehicle past the ramp's end it is the arc's value alone, and before that less by the roll
    const arc = v * v / 500 * Math.cos(phi) - 9.81 * Math.sin(phi)
    expect(at(specs.lateral.points, 312)).toBeCloseTo(arc, 6)
    expect(at(specs.lateral.points, 305)).toBeCloseTo(arc - roll, 6)
  })

  it('steps where the roll comes in and goes, two points at one station', () => {
    const { specs } = bandSpecs(track, t)
    const step = specs.lateral.points.filter((p, i, all) => i > 0 && all[i - 1].s === p.s)
    expect(step.map(p => p.s)).toEqual(expect.arrayContaining([191, 209, 291, 309]))
  })

  it('is no band where an element has no speed or an arc no cant', () => {
    expect(lateralInputsComplete(elements)).toBe(true)
    const noSpeed = elements.map((el, i) => (i === 0 ? { ...el, speed: undefined } : el))
    const noCant = elements.map((el, i) => (i === 2 ? { ...el, cant: undefined } : el))
    for (const els of [noSpeed, noCant]) {
      const { specs, order } = bandSpecs({ ...track, elements: els }, t)
      expect(specs.lateral).toBeUndefined()
      expect(order).not.toContain('lateral')
    }
    // a straight without a cant has none: the band stands
    const straightBare = elements.map((el, i) => (i === 0 ? { ...el, cant: undefined } : el))
    expect(bandSpecs({ ...track, elements: straightBare }, t).order).toContain('lateral')
  })
})
