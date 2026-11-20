import { describe, it, expect } from 'vitest'
import { bandSvg, bandSvgSize } from './bandSvg'
import { bandSpecs } from '../../components/bands/bandSpecs'

const t = (key) => key

/** A track of a straight, an arc of R 500 to the right with 100 mm of cant, and a straight; heights rising 6 ‰. */
const track = {
  id: 'track-1', name: 'Gl 1', epsg: 25832,
  elements: [
    { elementType: 'straight', length: 500, speed: 120, start: { easting: 0, northing: 0 }, azimuth: 0 },
    { elementType: 'arc', length: 300, radius: 500, cant: 100, speed: 100 },
    { elementType: 'straight', length: 400, speed: 120 },
  ],
  heights: [{ station: 0, z: 100 }, { station: 1200, z: 107.2 }],
}

describe('a band given out as an SVG drawing to scale', () => {
  it('is as long as its stretch at the length scale and as high as its values at the height scale', () => {
    const { specs } = bandSpecs(track, t)
    const size = bandSvgSize(specs.curvature, { from: 0, to: 1200 }, { mmPerKm: 100, mmPerUnit: 10 })
    expect(size.plotW).toBeCloseTo(120)
    // signed: ±2/km padded by 15 %, 10 mm a unit
    expect(size.lo).toBeCloseTo(-2.3)
    expect(size.hi).toBeCloseTo(2.3)
    expect(size.plotH).toBeCloseTo(46)
    expect(size.W).toBeCloseTo(size.left + 120 + 12)
  })

  it('draws the band, its zero line, the joints and the labels, in millimetres', () => {
    const { specs, spans } = bandSpecs(track, t)
    const svg = bandSvg(specs.cant, {
      boundaries: spans.slice(1).map(sp => sp.from), range: { from: 0, to: 1200 }, scale: { mmPerKm: 100, mmPerUnit: 0.2 },
      texts: { title: 'Überhöhungsband', subtitle: 'Gleis Gl 1', note: 'Rechtsbogen oben', station: 'Station' },
    })
    expect(svg).toMatch(/^<svg xmlns="http:\/\/www.w3.org\/2000\/svg" width="[\d.]+mm" height="[\d.]+mm"/)
    expect(svg).toContain('Überhöhungsband <tspan font-weight="normal" fill="#666">[mm]</tspan>')
    expect(svg).toContain('Gleis Gl 1 · Rechtsbogen oben')
    expect(svg).toContain('>u 100</text>')
    expect(svg.match(/stroke-dasharray="1 1"/g)).toHaveLength(2)
    expect(svg).toContain('Station [m]')
  })

  it('draws a stretch alone, a label that has no room there left out', () => {
    const { specs } = bandSpecs(track, t)
    const range = { from: 450, to: 560 }
    const size = bandSvgSize(specs.speed, range, { mmPerKm: 100, mmPerUnit: 0.5 })
    expect(size.plotW).toBeCloseTo(11)   // to scale, the frame wider for its title
    expect(size.W).toBeCloseTo(size.left + 60 + 12)
    const label = (v) => new RegExp(`font-size="2.4" fill="#333">${v}</text>`)
    const svg = bandSvg(specs.speed, { range, scale: { mmPerKm: 1000, mmPerUnit: 0.5 }, texts: { title: 'V', station: 'Station' } })
    expect(svg).toMatch(label(120))
    expect(svg).toMatch(label(100))
    const narrow = bandSvg(specs.speed, { range: { from: 0, to: 1200 }, scale: { mmPerKm: 10, mmPerUnit: 0.5 }, texts: { title: 'V', station: 'Station' } })
    expect(narrow).not.toMatch(label(100))
  })
})
