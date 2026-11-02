import { describe, it, expect } from 'vitest'
import { checkSignalling } from '@rail-across/core/signalling'
import { parseProjectsPayload, projectDocument } from '@rail-across/core/project'
import { deleted, domainOf, recordOf, signallingOf } from './domain.js'
import {
  boundaryOf, entryMarks, nameTaken, newStation, stationsOf, withBoundary, withPlatforms, withSignals, withStation, withoutStation,
} from './stations.js'

const el = { elementType: 0, startNode: [500000, 5700000], endNode: [500200, 5700000], bearing: 90, length: 200 }
const on = (station) => ({ anchor: 'station', trackId: 't1', station, side: 'right', direction: 'up' })
const ks = (fn) => [{ system: 'ks', type: 'multiSection', function: fn, frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }] }]

const record = {
  id: 'p', title: 'P',
  tracks: [{ id: 't1', name: 't1', epsg: 25832, elements: [el] }],
  switches: [],
  platforms: [{ id: 'pf1', trackId: 't1', startStation: 50, endStation: 150, side: 'right' }],
  signalling: {
    fields: {},
    collections: {
      signals: [
        { id: 's-a', name: 'A', kind: 'real', at: on(10), logicalSignals: ks('entry') },
        { id: 's-n', name: 'N1', kind: 'real', at: on(180), logicalSignals: ks('exit') },
        { id: 's-ne1', kind: 'real', at: on(5), frames: [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }] },
      ],
    },
  },
}

/** The core of the record the state makes, as the tool reads it. */
const coreOf = (state) => projectDocument({ ...recordOf(record, state), signalling: undefined }).domains.core

describe('the stations', () => {
  it('creates one in the register that nothing names yet, and gives it a platform, which then names it', () => {
    let state = domainOf(record)
    const station = newStation({ name: ' Könnern ', rl100: 'LKO' })
    expect(station).toMatchObject({ name: 'Könnern', rl100: 'LKO' })
    state = withStation(state, station)
    expect(stationsOf(coreOf(state)).get(station.id)).toMatchObject({ point: station, tracks: [], platforms: [] })
    state = withPlatforms(state, station, ['pf1'], [])
    expect(state.collections.platforms[0]).toMatchObject({ stationName: 'Könnern', code: 'LKO' })
    expect(stationsOf(coreOf(state)).get(station.id).platforms).toEqual(['pf1'])
    // Renamed, its platform follows; taken off, the platform names none.
    state = withStation(state, { ...station, name: 'Könnern (Saale)' }, ['pf1'])
    expect(state.collections.platforms[0].stationName).toBe('Könnern (Saale)')
    expect(stationsOf(coreOf(state)).get(station.id).platforms).toEqual(['pf1'])
    state = withPlatforms(state, station, [], ['pf1'])
    expect(state.collections.platforms[0]).not.toHaveProperty('stationName')
    expect(nameTaken(coreOf(state).collections.operationalPoints, 'könnern (saale)')).toBe(true)
  })

  it('gives a station its signals, the others of the open line, and its boundary by the rule', () => {
    const station = newStation({ name: 'Könnern' })
    let state = withSignals(withStation(domainOf(record), station), station.id, ['s-a', 's-n', 's-ne1'])
    expect(state.collections.signals.every(s => s.operationalPointId === station.id)).toBe(true)
    state = withSignals(state, station.id, ['s-a', 's-ne1'])
    expect(state.collections.signals.find(s => s.id === 's-n')).not.toHaveProperty('operationalPointId')
    // Its entry signal and its board Ne 1 bound it, not its exit signal.
    expect(entryMarks(state, station.id)).toEqual([{ signalId: 's-a' }, { signalId: 's-ne1' }])
    state = withBoundary(state, station.id, entryMarks(state, station.id))
    expect(boundaryOf(state, station.id).marks).toHaveLength(2)
    expect(checkSignalling({ core: coreOf(state), signalling: signallingOf(state) }).warnings.map(w => w.code)).not.toContain('boundary_mark_elsewhere')
    state = withBoundary(state, station.id, [])
    expect(boundaryOf(state, station.id)).toBeNull()
  })

  it('deletes a station with what is its: signals of the open line, platforms of none, its boundary gone', () => {
    const station = newStation({ name: 'Könnern' })
    let state = withStation(domainOf(record), station)
    state = withPlatforms(withSignals(state, station.id, ['s-a']), station, ['pf1'], [])
    state = withBoundary(state, station.id, [{ signalId: 's-a' }])
    state = withoutStation(state, station.id, ['pf1'])
    expect(state.collections.operationalPoints).toEqual([])
    expect(state.collections.signals.some(s => s.operationalPointId)).toBe(false)
    expect(state.collections.platforms[0]).not.toHaveProperty('stationName')
    expect(state.collections.stationBoundaries).toEqual([])
  })

  it('travels in the record through the door: the station, its platform and its signals', () => {
    const station = newStation({ name: 'Könnern', rl100: 'LKO' })
    let state = withStation(domainOf(record), station)
    state = withSignals(withPlatforms(state, station, ['pf1'], []), station.id, ['s-a'])
    const doc = JSON.parse(JSON.stringify(projectDocument(recordOf(record, state), { carrySchemas: true })))
    const back = parseProjectsPayload(doc).projects[0]
    expect(back.operationalPoints).toEqual([station])
    expect(back.platforms[0]).toMatchObject({ stationName: 'Könnern', code: 'LKO' })
    expect(back.signalling.collections.signals.find(s => s.id === 's-a').operationalPointId).toBe(station.id)
    // Deleting a signal leaves the parts of the core alone.
    expect(deleted(state, 'signals', 's-a').collections.operationalPoints).toEqual([station])
  })
})
