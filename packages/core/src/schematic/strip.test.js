import { describe, it, expect } from 'vitest'
import { schematicLayout } from './layout.js'
import { schematicStrip, switchNumber } from './strip.js'
import { tracks, switches, platforms } from '../../test/schematicFixture.js'

const GAP = 10
const layout = schematicLayout({ tracks, switches, platforms })
const strip = schematicStrip(layout, { laneY: (lane) => -lane * GAP, dxOf: (dy) => Math.abs(dy), leg: 3, gap: GAP })
const lines = strip.items.filter(i => i.kind === 'line')

describe('schematic strip', () => {
  it('draws every track of a lane as a line that names it', () => {
    const own = lines.find(l => l.trackIds.length === 1 && l.trackIds[0] === '1b')
    expect(own.pts[0][1]).toBeCloseTo(0)
    expect(own.pts[1][0]).toBeGreaterThan(own.pts[0][0])
  })

  it('draws the crossover at 45°, spread about its middle', () => {
    const x = lines.find(l => l.trackIds.includes('X'))
    expect(x.connector).toBe(true)
    const [[x1, y1], [x2, y2]] = x.pts
    expect(Math.abs(x2 - x1)).toBeCloseTo(Math.abs(y2 - y1), 6)
  })

  it('gives a switch with a diagonal its triangle and its number', () => {
    expect(strip.items.filter(i => i.kind === 'fill').length).toBeGreaterThan(0)
    expect(strip.items.some(i => i.kind === 'switch' && i.t === '1')).toBe(true)
  })

  it('states the planning status, not a colour', () => {
    expect(lines.every(l => l.status === 'existing')).toBe(true)
  })

  it('reads a switch number off its name', () => {
    expect(switchNumber('switch.003')).toBe('3')
    expect(switchNumber('W12a')).toBe('12a')
  })
})
