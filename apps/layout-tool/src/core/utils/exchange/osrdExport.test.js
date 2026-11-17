import { describe, expect, it } from 'vitest'
import { electrificationsOf } from './osrdExport'

const range = (id, trackId, startStation, endStation, voltage) => ({ id, trackId, startStation, endStation, voltage })
// Contact wires over ranges of tracks, each carried at its ends (electrification 3.0, decision 0006).
const wired = (ranges) => ({ fields: {}, collections: {
  contactWires: ranges.map(r => ({ id: r.id, voltage: r.voltage })),
  masts: [],
  suspensionPoints: ranges.flatMap(r => [r.startStation, r.endStation].map((station, i) => ({
    id: `${r.id}-${i}`, mastId: `m-${r.id}-${i}`, at: { anchor: 'station', trackId: r.trackId, station }, contactWireId: r.id,
  }))),
  sectionInsulators: [],
} })

describe('the electrification as RailJSON (decisions 0008 and 0006)', () => {
  it('writes one electrification per system, its ranges both ways, on the tracks the export holds', () => {
    const project = {
      electrification: wired([
        range('a', 't1', 0, 400, '15000V'), range('b', 't2', 100, 200.123456789012, '15000V'), range('c', 't1', 400, 900, '25000V'), range('x', 'gone', 0, 10, '15000V'),
      ]),
      osrd: { electrifications: [{ id: 'kept', voltage: '1500V', track_ranges: [] }, { id: 'old', voltage: '15000V', track_ranges: [] }] },
    }
    const out = electrificationsOf(project, { t1: {}, t2: {} })
    expect(out).toEqual([
      { id: 'electrification-15000V', voltage: '15000V', track_ranges: [
        { track: 't1', begin: 0, end: 400, applicable_directions: 'BOTH' }, { track: 't2', begin: 100, end: 200.123456789, applicable_directions: 'BOTH' },
      ] },
      { id: 'electrification-25000V', voltage: '25000V', track_ranges: [{ track: 't1', begin: 400, end: 900, applicable_directions: 'BOTH' }] },
      // What an imported file brought along stays where the layer has none of its system.
      { id: 'kept', voltage: '1500V', track_ranges: [] },
    ])
    expect(electrificationsOf({}, { t1: {} })).toEqual([])
  })
})
