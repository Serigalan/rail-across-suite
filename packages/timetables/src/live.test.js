import { describe, expect, it } from 'vitest'
import { net } from '../../running-times/test/net.js'
import { offsetAt, playSpan, sectionsAt, trainAt } from './live.js'

// The line L sectioned at 300 and 700 m: S1 between, S0 before, S2 beyond.
const point = (id, station) => ({ id, kind: 'axleCounter', at: { anchor: 'station', trackId: 'L', station } })
const record = {
  ...net,
  signalling: {
    fields: {},
    collections: {
      signals: [], detectionPoints: [point('p1', 300), point('p2', 700)],
      trackSections: [{ id: 'S0', kind: 'axleCounting', boundedBy: ['p1'] }, { id: 'S1', kind: 'axleCounting', boundedBy: ['p1', 'p2'] }, { id: 'S2', kind: 'axleCounting', boundedBy: ['p2'] }],
    },
  },
}

// A train of 100 m along L from 0 to 2000 m at 10 m/s from 7:00, standing a minute at 1000 m.
const at0 = 7 * 3600
const first = (x) => at0 + x / 10 + (x > 1000 ? 60 : 0)
const last = (x) => at0 + x / 10 + (x >= 1000 ? 60 : 0)
const run = {
  clock: { at0, length: 2000, end: first(2000), first, last },
  route: { path: [{ trackId: 'L', start: 0, end: 2000 }] },
  request: { rolling_stock: { length: 100 } },
  result: { final: { positions: [0, 2000], speeds: [10, 10] } },
  blocks: [{ begin: at0 - 12, sections: [{ id: 'S0', end: at0 + 36 }, { id: 'S1', end: at0 + 86 }, { id: 'S2', end: at0 + 300 }] }],
}

describe('the timetable played', () => {
  it('finds where a train is at a moment', () => {
    expect(offsetAt(run.clock, at0 + 50)).toBeCloseTo(500, 0)
    expect(trainAt(run, at0 - 1)).toMatchObject({ state: 'before', offset: 0 })
    expect(trainAt(run, at0 + 50)).toMatchObject({ state: 'running', speed: 10 })
    expect(trainAt(run, at0 + 130)).toMatchObject({ state: 'standing' })
    expect(trainAt(run, at0 + 130).offset).toBeCloseTo(1000, 0)
    expect(trainAt(run, at0 + 400)).toMatchObject({ state: 'arrived', offset: 2000 })
    expect(playSpan(new Map([['a', run], ['b', { problems: [] }]]))).toEqual({ from: at0, to: at0 + 260 })
  })

  it('says which track sections a train occupies, and which its routes hold for it ahead', () => {
    // At 7:00:35 its head is at 350 m, its tail at 250 m: across S0 and S1.
    expect(sectionsAt(record, run, at0 + 35)).toEqual({ occupied: ['S0', 'S1'], reserved: [] })
    expect(sectionsAt(record, run, at0 + 35, { reserved: true })).toEqual({ occupied: ['S0', 'S1'], reserved: ['S2'] })
    // S0 released once the tail has left it; nothing once it has arrived.
    expect(sectionsAt(record, run, at0 + 60, { reserved: true })).toEqual({ occupied: ['S1'], reserved: ['S2'] })
    expect(sectionsAt(record, run, at0 + 400, { reserved: true })).toEqual({ occupied: [], reserved: [] })
  })
})
