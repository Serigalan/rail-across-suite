import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { marked, net, WASM } from '../../running-times/test/net.js'
import {
  copyTrain, formatClock, itineraryOptions, newPoint, newTrain, numberAfter, parseClock, runTrain, taktCopies, trainProblems, trainRequest, trainTitle,
} from './timetable.js'

const stations = stationsOf(net)
const train = (points, patch = {}) => newTrain({ number: '16001', vehicle: 'v', departure: parseClock('7:00'), points: points.map(([station, p]) => newPoint(station, p)), ...patch })

describe('a train of the timetable', () => {
  it('begins and ends at a station only', () => {
    expect(trainProblems(train([['A-Dorf'], ['C-Stadt']]), stations)).toEqual([])
    expect(trainProblems(train([['A-Dorf'], ['B-Halt']]), stations)).toEqual([{ key: 'end_not_station', point: 1 }])
    expect(trainProblems(train([['B-Halt'], ['C-Stadt']]), stations)).toEqual([{ key: 'start_not_station', point: 0 }])
    // a halt on the way is a stop like any
    expect(trainProblems(train([['A-Dorf'], ['B-Halt'], ['C-Stadt']]), stations)).toEqual([])
  })

  it('begins and ends at a buffer stop or a boundary too, and passes neither', () => {
    const ends = stationsOf(marked)
    const stop = 'Prellbock C-Stadt Gl. 2 Ende'
    expect(trainProblems(train([['West'], [stop]]), ends)).toEqual([])
    expect(trainProblems(train([['A-Dorf'], ['West'], ['C-Stadt']]), ends)).toEqual([{ key: 'end_on_the_way', point: 1 }])
  })

  it('names stations and tracks the variant has, a vehicle and a margin that reads', () => {
    expect(trainProblems(train([['A-Dorf']]), stations)).toEqual([{ key: 'too_few_points' }])
    expect(trainProblems(train([['A-Dorf'], ['Z-Ort']]), stations)).toEqual([{ key: 'station_missing', point: 1 }])
    expect(trainProblems(train([['A-Dorf', { track: 'C2' }], ['C-Stadt']]), stations)).toEqual([{ key: 'track_missing', point: 0 }])
    expect(trainProblems(train([['A-Dorf'], ['C-Stadt']], { margin: 'viel' }), stations, { vehicle: false }).map(p => p.key)).toEqual(['no_vehicle', 'margin_invalid'])
  })

  it('reads and writes clock times', () => {
    expect(parseClock('7:05')).toBe(25500)
    expect(parseClock('07:05:30')).toBe(25530)
    expect(parseClock('0705')).toBe(25500)
    expect(parseClock('25:10')).toBe(90600)
    expect(parseClock('7:65')).toBeNull()
    expect(formatClock(25530)).toBe('07:05')
    expect(formatClock(25530, { seconds: true })).toBe('07:05:30')
  })

  it('is copied in a Takt, its number counted on', () => {
    const t = train([['A-Dorf', { departure: parseClock('7:00') }], ['C-Stadt']])
    expect(numberAfter('16001', 2)).toBe('16003')
    expect(numberAfter('RB 7', 1)).toBe('RB 8')
    expect(numberAfter('Sonder', 1)).toBe('Sonder')
    const copies = taktCopies(t, { every: 3600, count: 2 })
    expect(copies.map(c => [c.number, formatClock(c.departure), formatClock(c.points[0].departure)])).toEqual([['16003', '08:00', '08:00'], ['16005', '09:00', '09:00']])
    expect(new Set([t.id, ...copies.map(c => c.id)]).size).toBe(3)
    expect(copyTrain(t).points[0].id).not.toBe(t.points[0].id)
    expect(trainTitle({ ...t, name: 'Bummler' })).toBe('RB 16001 · Bummler')
  })
})

describe('the way a train prefers', () => {
  // Stopping places on A-Dorf's track 1 and at C-Stadt's two platforms; an itinerary to each, and one whose run does not lead there.
  const range = (trackId, startStation, endStation) => ({ anchor: 'station', trackId, startStation, endStation })
  const signalled = {
    ...net,
    signalling: {
      fields: {},
      collections: {
        stoppingPlaces: [
          { id: 'hA', name: '1', at: range('A1', 100, 300) },
          { id: 'hC1', name: '1', platformId: 'pC1', at: range('C1', 300, 500) },
          { id: 'hC2', name: '2', platformId: 'pC2', at: range('C2', 300, 500) },
        ],
        itineraries: [
          { id: 'f1', startStoppingPlaceId: 'hA', targetStoppingPlaceId: 'hC1', routeIds: [], path: { trackIds: ['A1', 'L', 'C1'] } },
          { id: 'f2', startStoppingPlaceId: 'hA', targetStoppingPlaceId: 'hC2', routeIds: [], path: { trackIds: ['A1', 'L', 'C2'] } },
          { id: 'f3', startStoppingPlaceId: 'hA', targetStoppingPlaceId: 'hC2', routeIds: [], path: { trackIds: ['A1', 'L', 'C1'] } },
        ],
      },
    },
  }
  const vehicle = newVehicle('Triebzug')
  const ends = (t) => trainRequest(signalled, stations, t, vehicle).route

  it('offers the itineraries from a stopping place of the one station to one of the next, on the tracks asked for', () => {
    expect(itineraryOptions(signalled, stations, train([['A-Dorf'], ['C-Stadt']]), 0).map(i => i.id)).toEqual(['f1', 'f2', 'f3'])
    expect(itineraryOptions(signalled, stations, train([['A-Dorf'], ['C-Stadt', { track: 'C1' }]]), 0).map(i => i.id)).toEqual(['f1'])
  })

  it('takes the way it prefers, else the shortest, and says which', () => {
    const route = ends(train([['A-Dorf', { itinerary: 'f2' }], ['C-Stadt']]))
    expect(route.path.map(p => p.trackId)).toEqual(['A1', 'L', 'C2'])
    expect(route.preferred).toEqual(['taken'])
    // One whose run does not lead to its target is not taken; one the variant lacks neither.
    expect(ends(train([['A-Dorf', { itinerary: 'f3' }], ['C-Stadt']])).preferred).toEqual(['other'])
    expect(ends(train([['A-Dorf', { itinerary: 'gone' }], ['C-Stadt']])).preferred).toEqual(['missing'])
    expect(ends(train([['A-Dorf'], ['C-Stadt']])).preferred).toEqual([null])
  })
})

describe.skipIf(!existsSync(WASM))('a train run', () => {
  const simulate = async (request) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    return runWith(instance.exports, request)
  }
  const vehicle = newVehicle('Triebzug')

  it('has a row for each station along its way, stops and passes timed', async () => {
    const t = train([['A-Dorf'], ['B-Halt', { dwell: 30 }], ['C-Stadt', { track: 'C2' }]])
    const run = await runTrain(net, stations, t, vehicle, simulate)
    expect(run.rows.map(r => [r.name, r.point, r.stop, r.track])).toEqual([['A-Dorf', 0, true, 'A1'], ['B-Halt', 1, true, 'L'], ['C-Stadt', 2, true, 'C2']])
    const [a, b, c] = run.rows
    expect(a).toMatchObject({ arrival: null, departure: 25200 })
    expect(b.departure - b.arrival).toBeCloseTo(30, 6)
    expect(b.arrival).toBeGreaterThan(25200 + 60)
    expect(c.departure).toBeNull()
    expect(c.arrival).toBeGreaterThan(b.departure)
  })

  it('passes where it does not stop, and waits for a departure', async () => {
    const pass = await runTrain(net, stations, train([['A-Dorf'], ['B-Halt', { stop: false }], ['C-Stadt']]), vehicle, simulate)
    const b = pass.rows[1]
    expect(b.stop).toBe(false)
    expect(b.arrival).toBe(b.departure)
    const plain = await runTrain(net, stations, train([['A-Dorf'], ['B-Halt', { dwell: 30 }], ['C-Stadt']]), vehicle, simulate)
    const waits = await runTrain(net, stations, train([['A-Dorf'], ['B-Halt', { dwell: 30, departure: parseClock('7:10') }], ['C-Stadt']]), vehicle, simulate)
    expect(waits.rows[1].departure).toBe(parseClock('7:10'))
    expect(waits.rows[1].arrival).toBeCloseTo(plain.rows[1].arrival, 6)
    expect(waits.rows[2].arrival - plain.rows[2].arrival).toBeCloseTo(parseClock('7:10') - plain.rows[1].departure, 6)
  })

  it('runs in over a boundary at its speed, on out over one at the line\'s, and stands at a buffer stop', async () => {
    const ends = stationsOf(marked)
    const stop = 'Prellbock C-Stadt Gl. 2 Ende'
    const kmh = (v) => v * 3.6
    const inward = await runTrain(marked, ends, train([['West'], ['B-Halt', { dwell: 30 }], [stop]]), vehicle, simulate)
    const speeds = inward.result.final.speeds
    expect(kmh(speeds[0])).toBeCloseTo(60, 3)
    expect(speeds.at(-1)).toBeCloseTo(0, 6)
    expect(inward.rows.map(r => [r.name, r.point])).toEqual([['West', 0], ['A-Dorf', null], ['B-Halt', 1], ['C-Stadt', null], [stop, 2]])
    expect(inward.rows[0].departure).toBe(25200)
    // out over a boundary of no speed of its own: it does not brake for it
    const free = { ...marked, endMarks: marked.endMarks.map(m => (m.id === 'mW' ? { ...m, speed: undefined } : m)) }
    const outward = await runTrain(free, stationsOf(free), train([['C-Stadt', { track: 'C1' }], ['West']]), vehicle, simulate)
    expect(kmh(outward.result.final.speeds.at(-1))).toBeGreaterThan(80)
    const slow = await runTrain(marked, ends, train([['C-Stadt', { track: 'C1' }], ['West']]), vehicle, simulate)
    expect(kmh(slow.result.final.speeds.at(-1))).toBeCloseTo(60, 0)
    expect(slow.rows.at(-1).arrival).toBeGreaterThan(outward.rows.at(-1).arrival)
  })

  it('says why it cannot run', async () => {
    expect(await runTrain(net, stations, train([['A-Dorf'], ['B-Halt']]), vehicle, simulate)).toEqual({ problems: [{ key: 'end_not_station', point: 1 }] })
    expect((await runTrain(net, stations, train([['A-Dorf'], ['C-Stadt'], ['B-Halt'], ['C-Stadt']]), vehicle, simulate)).failedAt).toBe(2)
  })
})
