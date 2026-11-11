import { describe, expect, it } from 'vitest'
import { blockingTimes, blocksAlong } from './occupancy.js'

// Reference case A (docs/reference-cases/a-two-track-line.md): the figures worked out by hand there.
const track = { id: 'L', name: 'L', epsg: 25832, elements: [{ elementType: 0, length: 6000, bearing: 90, startNode: [500000, 5700000], endNode: [506000, 5700000] }] }
const logical = (type, fn) => [{ system: 'ks', type, function: fn, frames: [] }]
const at = (station) => ({ anchor: 'station', trackId: 'L', station, side: 'right', direction: 'up' })
const record = {
  tracks: [track], switches: [], platforms: [],
  signalling: {
    fields: { routeTimes: { setting: 12, release: 6 } },
    collections: {
      signals: [
        { id: 'vS1', name: 'vS1', kind: 'real', at: at(500), sight: { required: 200 }, logicalSignals: logical('distant', 'distant') },
        { id: 'S1', name: 'S1', kind: 'real', at: at(1500), logicalSignals: logical('multiSection', 'block') },
        { id: 'S2', name: 'S2', kind: 'real', at: at(3500), logicalSignals: logical('multiSection', 'block') },
        { id: 'S3', name: 'S3', kind: 'real', at: at(5500), logicalSignals: logical('multiSection', 'block') },
      ],
      overlaps: [{ id: 'd2', signalId: 'S2', length: 200 }, { id: 'd3', signalId: 'S3', length: 200 }],
      routes: [{ id: 'r12', kind: 'train', startSignalId: 'S1', targetSignalId: 'S2', settingTime: 10, releaseTime: 5 }],
    },
  },
}
const path = [{ trackId: 'L', start: 0, end: 6000 }]
// 20 m/s from 0 at 0 s, no stop: at x m at x / 20 s, at the end at 300 s.
const clock = { at0: 0, first: (x) => x / 20, standstills: [], length: 6000, end: 300 }
const TIMETABLE = { setup: 12, approach: 1000, release: 6 }

describe('reference case A: a two-track line with Ks block signals', () => {
  const blocks = blocksAlong(record, path)

  it('has its blocks from main signal to main signal, each with its overlap', () => {
    expect(blocks.map(b => [b.from, b.to, b.reach])).toEqual([[0, 1500, 1500], [1500, 3500, 3700], [3500, 5500, 5700], [5500, 6000, 6000]])
  })

  it('holds each block as worked out by hand after Ril 405', () => {
    const timed = blockingTimes(blocks, clock, 150, TIMETABLE)
    expect(timed.map(b => [b.begin, b.end])).toEqual([[-12, 88.5], [5, 187.5], [63, 288.5], [163, 306]])
  })
})
