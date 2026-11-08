import { describe, expect, it } from 'vitest'
import { closeLegacyGaps } from './legacyGaps.js'
import { dehydrateProjects } from './persistenceUtils.js'
import { nodeGaps, NODE_TOL } from '../geometry/index.js'
import { hasPekBestand, loadPekBestand } from '../../test/pekFixture.js'

const straight = (a, b) => ({
  elementType: 0, startNode: a, endNode: b, length: Math.hypot(b[0] - a[0], b[1] - a[1]),
  bearing: (Math.atan2(b[0] - a[0], b[1] - a[1]) * 180 / Math.PI + 360) % 360,
})

describe('closing what a millimetre joined', () => {
  it('closes joints within a track and between track ends up to 1 mm, and leaves wider ones', () => {
    const record = {
      id: 'p',
      tracks: [
        { id: 'a', epsg: 25832, elements: [straight([0, 0], [100, 0]), straight([100.0004, 0], [200, 0])] },
        { id: 'b', epsg: 25832, elements: [straight([200.0007, 0], [300, 0])] },
        { id: 'c', epsg: 25832, elements: [straight([300.002, 0], [400, 0])] },
      ],
    }
    const { record: out, closed } = closeLegacyGaps(record)
    expect(closed).toBe(2)
    // The start is the stated point; the end moves onto it.
    expect(out.tracks[0].elements[0].endNode).toEqual([100.0004, 0])
    expect(out.tracks[0].elements[0].length).toBeCloseTo(100.0004, 9)
    expect(out.tracks[0].elements[1].endNode).toEqual([200.0007, 0])
    expect(out.tracks[1].elements[0].endNode).toEqual([300, 0])
  })

  it('leaves a record that is closed as it is', () => {
    const record = { id: 'p', tracks: [{ id: 'a', epsg: 25832, elements: [straight([0, 0], [100, 0]), straight([100, 0], [200, 0])] }] }
    const out = closeLegacyGaps(record)
    expect(out.closed).toBe(0)
    expect(out.record).toBe(record)
  })

  it.skipIf(!hasPekBestand)('closes every joint of the PEK Bestand under a millimetre, and keeps it closed when kept', () => {
    const { record } = closeLegacyGaps(loadPekBestand())
    const [kept] = dehydrateProjects([record])
    for (const track of kept.tracks) expect(nodeGaps(track.elements, NODE_TOL)).toEqual([])
  })
})
