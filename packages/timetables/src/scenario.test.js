import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { WASM } from '../../running-times/test/net.js'
import { crossing } from '../test/crossing.js'
import { newPoint, newTrain, parseClock, runTrain } from './timetable.js'
import { closuresAround, closureTimes, dayAfter, scenarioRows, scenarioRuns, touchedBy } from './scenario.js'

const DAY = '2027-05-03'
const closure = (id, trackId, lo, hi, patch = {}) => ({ id, at: { anchor: 'station', trackId, startStation: lo, endStation: hi }, firstDay: DAY, lastDay: DAY, ...patch })

describe('a closure in time', () => {
  it('holds all day, or in a window that may run past midnight', () => {
    expect(dayAfter('2027-02-28')).toBe('2027-03-01')
    expect(dayAfter('2027-03-01', -1)).toBe('2027-02-28')
    expect(closureTimes(closure('a', 'L1', 0, 10), DAY)).toEqual([[0, 86400]])
    // 22:00–05:00 on the 3rd: that evening into the 4th
    expect(closureTimes(closure('a', 'L1', 0, 10, { window: { from: 79200, to: 18000 } }), DAY)).toEqual([[79200, 104400]])
    // and on the 4th, from the night before
    expect(closureTimes(closure('a', 'L1', 0, 10, { window: { from: 79200, to: 18000 } }), dayAfter(DAY))).toEqual([[-7200, 18000]])
    const scenario = { closures: [closure('a', 'L1', 0, 10, { window: { from: 79200, to: 18000 } }), closure('b', 'L1', 0, 10, { firstDay: '2027-06-01', lastDay: '2027-06-02' })] }
    expect(closuresAround(scenario, dayAfter(DAY)).map(c => c.id)).toEqual(['a'])
    expect(closuresAround(scenario, dayAfter(DAY, 2))).toEqual([])
  })

  it('touches a train that is on its stretch while it holds', () => {
    // on L1 from 500 to 3000: at 10 m/s from 7:00
    const run = { route: { path: [{ trackId: 'L1', start: 0, end: 2500 }] }, clock: { first: (o) => 25200 + o / 10, last: (o) => 25200 + o / 10 } }
    const night = closure('n', 'L1', 1000, 1200, { window: { from: 79200, to: 18000 } })
    const day = closure('d', 'L1', 1000, 1200)
    const morning = closure('m', 'L1', 1000, 1200, { window: { from: 25200, to: 25250 } })
    expect(touchedBy(run, [night, day, morning], DAY, 100).map(c => c.id)).toEqual(['d'])
    // 7:01:40 to 7:02:10 the train is on it (head in at 1000 m, tail out at 1300 m)
    expect(touchedBy(run, [closure('w', 'L1', 1000, 1200, { window: { from: 25300, to: 25310 } })], DAY, 100).map(c => c.id)).toEqual(['w'])
  })
})

describe.skipIf(!existsSync(WASM))('a timetable in a construction scenario', () => {
  const simulate = async (request) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    return runWith(instance.exports, request)
  }
  const stations = stationsOf(crossing)
  const named = (name) => stations.find(s => s.name === name)
  const vehicle = newVehicle('Triebzug')
  const run = (record, train, holds, routes) => runTrain(record, stations, train, vehicle, simulate, holds, routes)
  const train = (id, clock, points) => newTrain({ id, number: id, vehicle: 'v', departure: parseClock(clock), points })
  const there = train('1', '7:00', [newPoint(named('H-Stadt')), newPoint(named('K-Stadt'))])
  const back = train('2', '7:30', [newPoint(named('K-Stadt')), newPoint(named('H-Stadt'))])
  const tracksOf = (r) => r.run.route.path.map(p => p.trackId)

  it('diverts a train around a closed track, keeps one off a closure in the night as it is, and leaves out a train taken out', async () => {
    const scenario = { closures: [closure('w1', 'W1', 0, 600), closure('n', 'L2', 100, 300, { window: { from: 79200, to: 18000 } })], cancelledTrainIds: ['2'] }
    const runs = await scenarioRuns(crossing, [there, back], scenario, DAY, { run })
    expect(runs.get('1')).toMatchObject({ state: 'diverted', closures: ['w1'] })
    expect(tracksOf(runs.get('1'))).toEqual(['H1', 'L1', 'W2', 'L2', 'K1'])
    expect(runs.get('2').state).toBe('cancelled')
    const regular = new Map([['1', await run(crossing, there, [], [])]])
    expect(tracksOf({ run: regular.get('1') })).toContain('W1')
    const rows = scenarioRows([there, back], regular, runs)
    expect(rows[0].delay).toBeGreaterThanOrEqual(0)
    expect(rows[1]).toMatchObject({ state: 'cancelled', arrival: null, delay: null })
  })

  it('says which train has no way left, and runs one diverted by hand on its points', async () => {
    const line = { closures: [closure('l1', 'L1', 1000, 1200)] }
    const runs = await scenarioRuns(crossing, [there], line, DAY, { run })
    expect(runs.get('1')).toMatchObject({ state: 'not_runnable', closures: ['l1'] })
    expect(runs.get('1').run.failedAt).toBe(1)
    // by hand through W-Dorf's track 2, passing
    const viaW2 = { diversions: [{ trainId: '1', points: [newPoint(named('H-Stadt')), newPoint(named('W-Dorf'), { stop: false, trackId: 'W2' }), newPoint(named('K-Stadt'))] }] }
    const byHand = await scenarioRuns(crossing, [there], viaW2, DAY, { run })
    expect(byHand.get('1').state).toBe('by_hand')
    expect(tracksOf(byHand.get('1'))).toContain('W2')
  })
})
