import { describe, it, expect } from 'vitest'
import { carryAfterWrite, carryDomain } from './carryLocations.js'
import { carryReferences } from './remap.js'
import { straightElement } from '../geometry/elementFactory.js'

const EPSG = 25832
const P = (x) => ({ easting: 500000 + x, northing: 5700000, zone: EPSG })
// A straight track along the x axis from `x0` to `x1` (east, or west when x1 < x0).
const track = (id, x0, x1) => ({ id, epsg: EPSG, elements: [straightElement(P(x0), P(x1))] })
const on = (trackId, station, extra = {}) => ({ anchor: 'station', trackId, station, side: 'right', offset: 3, direction: 'up', ...extra })

// A domain the core does not know, with a signal on track a at 150 m and a run over a then b.
const domain = () => ({
  fields: {},
  collections: {
    signals: [{ id: 's1', kind: 'real', at: on('a', 150) }],
    routes: [{ id: 'r1', path: { trackIds: ['a', 'b'] } }],
  },
})
const a = track('a', 0, 200)
const b = track('b', 200, 300)
const before = { tracks: [a, b], switches: [], signalling: domain() }

describe('carrying a domain\'s locations across a write of the net (R4)', () => {
  it('follows a split onto the piece the place lies on, at its station there', () => {
    const after = { ...before, tracks: [track('a1', 0, 100), track('a2', 100, 200), b] }
    const out = carryAfterWrite(before, after)
    expect(out.signalling.collections.signals[0].at).toEqual(on('a2', 50))
    expect(out.signalling.collections.routes[0].path).toEqual({ trackIds: ['a1', 'a2', 'b'] })
  })

  it('swaps side and direction where the track was turned round', () => {
    const after = { ...before, tracks: [track('a', 200, 0), b] }
    const out = carryAfterWrite(before, after)
    expect(out.signalling.collections.signals[0].at).toEqual(on('a', 50, { side: 'left', direction: 'down' }))
  })

  it('moves the station where a piece was put in front', () => {
    const after = { ...before, tracks: [track('a', -40, 200), b] }
    expect(carryAfterWrite(before, after).signalling.collections.signals[0].at.station).toBe(190)
  })

  it('orphans what stood on a deleted track in the plane, its id kept (R7)', () => {
    const after = { ...before, tracks: [b] }
    const sig = carryAfterWrite(before, after).signalling.collections.signals[0]
    expect(sig).toMatchObject({ id: 's1', at: { anchor: 'plane', easting: 500150, northing: 5700000, epsg: EPSG, bearing: 90 } })
  })

  it('keeps the station where the track was only reshaped away from the place', () => {
    const after = { ...before, tracks: [{ ...track('a', 0, 200), elements: [straightElement(P(0), { ...P(200), northing: 5700030 })] }, b] }
    expect(carryAfterWrite(before, after).signalling.collections.signals[0].at).toEqual(on('a', 150))
  })

  it('changes nothing, not even the object, where the write left the tracks alone', () => {
    const after = { ...before, title: 'x' }
    expect(carryAfterWrite(before, after)).toBe(after)
    const touched = { ...before, tracks: [a, track('b', 200, 300)] }
    expect(carryAfterWrite(before, touched).signalling).toBe(before.signalling)
  })

  it('reports what it carried', () => {
    const ctx = {
      present: new Set(['a1', 'a2', 'b']), changed: (id) => id === 'a', oldTrack: (id) => (id === 'a' ? a : null),
      candidates: () => [track('a1', 0, 100), track('a2', 100, 200)], tracks: [track('a1', 0, 100), track('a2', 100, 200), b], switches: [],
    }
    const { carried } = carryDomain(domain(), ctx)
    expect(carried).toEqual([
      { collection: 'signals', id: 's1', field: 'at', from: 'a', to: 'a2' },
      { collection: 'routes', id: 'r1', field: 'path', from: 'a', to: 'a1,a2,b' },
    ])
  })
})

describe('carrying a domain\'s locations in a merge, through the id log', () => {
  it('carries the record\'s signalling like its platforms', () => {
    const project = { tracks: [track('a1', 0, 100), track('a2', 100, 200), b], switches: [], signalling: domain() }
    const { project: out, carried } = carryReferences(project, [[{ from: 'a', to: ['a1', 'a2'] }]], (id) => (id === 'a' ? a : null))
    expect(out.signalling.collections.signals[0].at).toEqual(on('a2', 50))
    expect(carried).toContainEqual({ collection: 'signalling.signals', id: 's1', from: 'a', to: 'a2' })
  })
})
