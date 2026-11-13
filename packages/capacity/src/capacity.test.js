import { describe, expect, it } from 'vitest'
import { analyseStretch, boundOf, headwayTable, proposedStretches, recordAtStage, stationOccupancy, trainsOnStretch } from './index.js'

// Reference case A (docs/reference-cases/a-two-track-line.md): trains A and B on track L from X to Y.
const A = [[0, 1500, 1500, -12, 88.5], [1500, 3500, 3700, 5, 187.5], [3500, 5500, 5700, 63, 288.5], [5500, 6000, 6000, 163, 306]]
const B = [[0, 1500, 1500, -12, 171], [1500, 3500, 3700, 20, 370], [3500, 5500, 5700, 138, 571], [5500, 6000, 6000, 338, 606]]
const station = (key, tracks = ['L', `${key}2`]) => ({ key, name: key, kind: 'station', tracks: tracks.map(id => ({ id })) })
const stations = [station('X'), station('Y'), { key: 'H', name: 'H', kind: 'halt', tracks: [] }]

function run(at, blocks, end) {
  return {
    rows: [
      { key: 'X', name: 'X', offset: 0, arrival: null, departure: at },
      { key: 'H', name: 'H', offset: 3000, arrival: at + 150, departure: at + 150 },
      { key: 'Y', name: 'Y', offset: 6000, arrival: at + end, departure: null },
    ],
    blocks: blocks.map(([from, to, reach, begin, endAt], index) => ({
      index, from, to, reach, begin: begin + at, end: endAt + at, stretches: [{ trackId: 'L', lo: from, hi: reach }],
    })),
  }
}
const trains = [
  { id: 'A1', category: 'RE' }, { id: 'B1', category: 'Gz' }, { id: 'A2', category: 'RE' }, { id: 'A3', category: 'RE' },
]
const runs = new Map([['A1', run(0, A, 300)], ['B1', run(300, B, 600)], ['A2', run(1200, A, 300)], ['A3', run(1800, A, 300)]])

describe('the capacity of a stretch (K1)', () => {
  it('proposes the stretches from a station of more tracks to the next, a halt between them none', () => {
    expect(proposedStretches(stations, trains, runs)).toEqual([{ id: 'X|Y', from: 'X', to: 'Y', fromName: 'X', toName: 'Y', trains: 4 }])
  })

  it('compresses reference case A\'s hour as worked out by hand', () => {
    const [stretch] = proposedStretches(stations, trains, runs)
    expect(trainsOnStretch(stretch, trains, runs)[1]).toMatchObject({ id: 'B1', kind: 'Gz', at: 300 })
    const r = analyseStretch(stretch, trains, runs, { from: 0, to: 3600 })
    expect(r.trains.map(t => t.compressedAt)).toEqual([0, 268.5, 834.5, 1118])
    expect(r.occupancy).toBe(1436)
    expect(r.critical).toEqual(['A1', 'B1', 'A2', 'A3'])
    expect(r.bound).toBe(0.75)
    expect(r.headways).toEqual([
      { before: 'Gz', after: 'RE', least: 566, most: 566, pairs: 3 },
      { before: 'RE', after: 'Gz', least: 268.5, most: 268.5, pairs: 3 },
      { before: 'RE', after: 'RE', least: 283.5, most: 283.5, pairs: 6 },
    ])
  })

  it('says how long a station\'s tracks are held', () => {
    const held = stationOccupancy(station('Y', ['L']), trains, runs, {}, { from: 0, to: 3600 })
    expect(held.length).toBeGreaterThan(0)
    expect(held[0].trains).toEqual(expect.arrayContaining(['A1', 'B1']))
  })

  it('takes the bounds of UIC Code 406 by line and period', () => {
    expect(boundOf('suburban', 3600)).toBe(0.85)
    expect(boundOf('suburban', 86400)).toBe(0.7)
    expect(boundOf('unknown', 3600)).toBe(0.75)
    expect(headwayTable([])).toEqual([])
  })
})

describe('a variant at a stage', () => {
  it('leaves out what is new at the existing stage, what is removed at the final', () => {
    const record = { id: 'p', tracks: [{ id: 'a' }, { id: 'n', lifespan: { from: 'open' } }, { id: 'r', lifespan: { to: 'open' } }], switches: [], platforms: [] }
    expect(recordAtStage(record, 'existing').tracks.map(t => t.id)).toEqual(['a', 'r'])
    expect(recordAtStage(record, 'final').tracks.map(t => t.id)).toEqual(['a', 'n'])
    expect(recordAtStage(record, 'as planned')).toEqual(record)
  })
})
