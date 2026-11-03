import { describe, it, expect } from 'vitest'
import { schematicLayout } from './layout.js'
import { schematicStrip } from './strip.js'
import { schematicPlacer } from './place.js'
import { tracks, switches, platforms } from '../../test/schematicFixture.js'

const GAP = 10
const layout = schematicLayout({ tracks, switches, platforms })
const strip = schematicStrip(layout, { laneY: (lane) => -lane * GAP, dxOf: (dy) => Math.abs(dy), leg: 3, gap: GAP })
const place = schematicPlacer(layout, strip)

describe('placing on the schematic strip', () => {
  it('puts a station of a straight track where it is along the line, on its lane', () => {
    const p = place.at('1b', 500)
    expect(p.x).toBeCloseTo(900, 0)
    expect(p.y).toBeCloseTo(0)
    expect(p.dir).toBe(1)
  })

  it('keeps a station of the crossover on its diagonal', () => {
    const p = place.at('X', 30)
    expect(p.y).toBeLessThan(0)
    expect(p.y).toBeGreaterThan(-GAP)
  })

  it('gives a stretch of a track between two stations', () => {
    const pts = place.stretch('1b', 100, 300)
    expect(pts[0][0]).toBeCloseTo(500, 0)
    expect(pts[pts.length - 1][0]).toBeCloseTo(700, 0)
  })

  it('has no place on a track the strip does not draw', () => {
    expect(place.at('nowhere', 10)).toBeNull()
  })
})
