import { describe, it, expect } from 'vitest'
import { netParts, trackSectionExtents } from './trackSections.js'
import { straightElement } from '../geometry/elementFactory.js'

const EPSG = 25832
const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
const track = (id, a, b) => ({ id, epsg: EPSG, elements: [straightElement(P(...a), P(...b))] })

// t1 runs 200 m east into the toe of turnout w1; t2 goes on straight, t3 to the north-east.
const core = {
  fields: {},
  collections: {
    tracks: [track('t1', [0, 0], [200, 0]), track('t2', [200, 0], [400, 0]), track('t3', [200, 0], [380, 40])],
    switches: [{ id: 'w1', switchId: 'w1', kind: 'turnout', formVersion: 1, ports: {
      A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't3', endpoint: 'BEGIN' }, B2: { trackId: 't2', endpoint: 'BEGIN' },
    } }],
  },
}
const point = (id, trackId, station) => ({ id, kind: 'axleCounter', at: { anchor: 'station', trackId, station } })
const signalling = (sections) => ({
  fields: {},
  collections: {
    detectionPoints: [point('a', 't1', 150), point('b', 't2', 50), point('c', 't3', 60), point('d', 't2', 180)],
    trackSections: sections,
  },
})

describe('the parts the detection points cut the net into', () => {
  it('joins the pieces across a switch, bounded by the points around it', () => {
    const parts = netParts({ core, signalling: signalling([]) })
    const switchArea = parts.find(p => p.boundary.join() === 'a,b,c')
    expect(switchArea.parts).toEqual([
      { trackId: 't1', from: 150, to: 200 }, { trackId: 't2', from: 0, to: 50 }, { trackId: 't3', from: 0, to: expect.any(Number) },
    ].map(x => x.trackId === 't3' ? { ...x, to: 60 } : x))
    expect(parts.map(p => p.boundary.join()).sort()).toEqual(['a', 'a,b,c', 'b,d', 'c', 'd'])
  })
})

describe('the extent of a stored section', () => {
  it('is the part bounded by all of its points', () => {
    const extents = trackSectionExtents({ core, signalling: signalling([{ id: 'W1', kind: 'axleCounting', boundedBy: ['a', 'b', 'c'] }]) })
    expect(extents.get('W1')).toMatchObject({ matches: true, missing: [] })
    expect(extents.get('W1').parts.map(p => p.trackId)).toEqual(['t1', 't2', 't3'])
  })

  it('is cut by its own points only, and runs to a track end where it has none', () => {
    // c bounds no section: W1 runs past it, up t3 to its end.
    const extents = trackSectionExtents({ core, signalling: signalling([
      { id: 'W1', kind: 'axleCounting', boundedBy: ['a', 'b'] }, { id: 'G2', kind: 'axleCounting', boundedBy: ['b', 'd'] },
    ]) })
    expect(extents.get('W1')).toMatchObject({ matches: true })
    expect(extents.get('W1').parts.find(p => p.trackId === 't3')).toEqual({ trackId: 't3', from: 0, to: expect.closeTo(Math.hypot(180, 40), 6) })
    expect(extents.get('G2').parts).toEqual([{ trackId: 't2', from: 50, to: 180 }])
  })

  it('names the points that enclose nothing with the others', () => {
    const apart = { ...core, collections: { ...core.collections, tracks: [...core.collections.tracks, track('t4', [0, 500], [100, 500])] } }
    const sig = signalling([{ id: 'X', kind: 'axleCounting', boundedBy: ['a', 'e'] }])
    sig.collections.detectionPoints.push(point('e', 't4', 50))
    expect(trackSectionExtents({ core: apart, signalling: sig }).get('X')).toMatchObject({ matches: false, missing: [expect.any(String)] })
  })
})
