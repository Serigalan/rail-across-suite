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
    // the roll rate dφ/dx of 100 mm over 100 m, where the cant is u: d asin(u/s)
    const rate = (u) => (0.1 / 100) / Math.sqrt(1.5 * 1.5 - u * u)
    const roll = (u) => h * v * v * rate(u) / L
    // on the straight, 5 m before the ramp: no curve, no cant, only the roll — read 4 m into the ramp
    expect(at(specs.lateral.points, 195)).toBeCloseTo(roll(0.004), 6)
    // farther back the vehicle does not roll yet
    expect(at(specs.lateral.points, 180)).toBeCloseTo(0, 9)
    // half a vehicle past the ramp's end it is the arc's value alone, and before that less by the roll
    const arc = v * v / 500 * Math.cos(phi) - 9.81 * Math.sin(phi)
    expect(at(specs.lateral.points, 312)).toBeCloseTo(arc, 6)
    expect(at(specs.lateral.points, 305)).toBeCloseTo(arc - roll(0.096), 6)
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

describe('the lateral acceleration over a Bloss transition', () => {
  // 100 m of Bloss into R 500 with its curved ramp to 100 mm, at 100 km/h
  const bloss = elements.map((el, i) => (i === 1 ? { ...el, transitionType: 'bloss' } : el))
  const { specs } = bandSpecs({ ...track, elements: bloss }, t)
  const points = specs.lateral.points

  it('runs smooth: no step anywhere, as the curved ramp starts and ends flat', () => {
    const steps = points.filter((p, i) => i > 0 && points[i - 1].s === p.s && Math.abs(points[i - 1].v - p.v) > 1e-6)
    expect(steps).toEqual([])
    // and no jump between neighbouring points a metre apart
    const jumps = points.slice(1).map((p, i) => Math.abs(p.v - points[i].v))
    expect(Math.max(...jumps)).toBeLessThan(0.03)
  })

  it('does not swing inwards at the start of the transition, as a straight ramp under a Bloss curve did', () => {
    const inRamp = points.filter(p => p.s >= 190 && p.s <= 310)
    expect(Math.min(...inRamp.map(p => p.v))).toBeGreaterThan(-1e-6)
  })
})
