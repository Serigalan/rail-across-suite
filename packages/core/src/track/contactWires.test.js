import { describe, expect, it } from 'vitest'
import { wireStretches } from './contactWires.js'

const at = (trackId, station) => ({ anchor: 'station', trackId, station })

describe('where a contact wire runs', () => {
  it('runs on a track from its first to its last carrying point there, at its voltage', () => {
    const domain = { collections: {
      contactWires: [{ id: 'a', voltage: '15000V' }, { id: 'b', voltage: '25000V' }, { id: 'c', voltage: '15000V' }],
      masts: [],
      suspensionPoints: [
        { id: 'p1', mastId: 'm1', at: at('t1', 120), contactWireId: 'a' },
        { id: 'p2', mastId: 'm2', at: at('t1', 20), contactWireId: 'a' },
        { id: 'p3', mastId: 'm3', at: at('t2', 0), contactWireId: 'a' },
        { id: 'p4', mastId: 'm4', at: at('t1', 260), contactWireId: 'b' },
      ],
      sectionInsulators: [{ id: 's', at: at('t1', 180), contactWireBeforeId: 'a', contactWireAfterId: 'b' }],
    } }
    expect(wireStretches(domain)).toEqual([
      { contactWireId: 'a', voltage: '15000V', trackId: 't1', startStation: 20, endStation: 180 },
      { contactWireId: 'a', voltage: '15000V', trackId: 't2', startStation: 0, endStation: 0 },
      { contactWireId: 'b', voltage: '25000V', trackId: 't1', startStation: 180, endStation: 260 },
    ])
  })

  it('finds nothing where nothing carries a wire, or there is no layer', () => {
    expect(wireStretches({ collections: { contactWires: [{ id: 'a', voltage: '15000V' }] } })).toEqual([])
    expect(wireStretches(undefined)).toEqual([])
  })
})
