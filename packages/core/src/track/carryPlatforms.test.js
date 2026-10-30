import { describe, it, expect } from 'vitest'
import { carryPlatforms, flipPlatforms } from './platformUtils.js'
import { reverseTrack, rebuildCoords } from './trackModel.js'
import { straightElement } from '../geometry/elementFactory.js'
import { edgeAt } from '../../test/platformEdge.js'

const EPSG = 25832
const P = (x, y) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
const track = (id, elements) => ({ id, name: id, epsg: EPSG, elements, coordinates: rebuildCoords(elements) })
const platform = (id, trackId, startStation, endStation, side) =>
  ({ id, trackId, startStation, endStation, side, frontOffset: 1.68, height: 550 })

describe('carryPlatforms', () => {
  const carry = [{ oldId: 't', newId: 'n', from: 100, to: 300, offset: -100, flip: false }]

  it('re-stations a platform the kept part holds whole', () => {
    const [p] = carryPlatforms([platform('p', 't', 120, 260, 'left')], carry)
    expect(p).toMatchObject({ trackId: 'n', startStation: 20, endStation: 160, side: 'left' })
  })

  it('lets a platform go that the kept part does not hold — as deleting elements does', () => {
    expect(carryPlatforms([platform('p', 't', 50, 150, 'left')], carry)).toEqual([])
  })

  it('leaves the platforms of other tracks as they are', () => {
    const q = platform('q', 'u', 0, 50, 'right')
    expect(carryPlatforms([q], carry)).toEqual([q])
  })

  it('turns a platform round where the new track runs the other way, onto its other side', () => {
    const [p] = carryPlatforms([platform('p', 't', 120, 260, 'left')],
      [{ oldId: 't', newId: 'n', from: 0, to: 400, offset: 400, flip: true }])
    expect(p).toMatchObject({ trackId: 'n', startStation: 140, endStation: 280, side: 'right' })
  })
})

// Case F3 of the data model: a platform at 120–260 m on the left of a 400 m
// track stays where it stands when the track is reversed — at 140–280 m,
// counted from the other end, on its right.
describe('a reversed track carries its platforms (data model, case F3)', () => {
  // Two straights with a bend between them, so the platform's ends run in two
  // directions: 150 m east, then some 252 m east-north-east.
  const t = track('t', [straightElement(P(0, 0), P(150, 0)), straightElement(P(150, 0), P(400, 30))])
  const length = t.elements.reduce((s, el) => s + el.length, 0)

  it('re-stations it from the other end and puts it on the other side', () => {
    const [p] = flipPlatforms([platform('p', 't', 120, 260, 'left')], 't', 400)
    expect(p).toMatchObject({ trackId: 't', startStation: 140, endStation: 280, side: 'right' })
  })

  it('leaves it standing where it stood, edge for edge', () => {
    const before = platform('p', 't', 120, 260, 'left')
    const reversed = reverseTrack(t)
    const [after] = flipPlatforms([before], 't', length)
    for (const [s0, s1] of [[before.startStation, after.endStation], [before.endStation, after.startStation]]) {
      const [e0, n0] = edgeAt(t, before, s0)
      const [e1, n1] = edgeAt(reversed, after, s1)
      expect(Math.hypot(e1 - e0, n1 - n0)).toBeLessThan(1e-6)
    }
  })
})
