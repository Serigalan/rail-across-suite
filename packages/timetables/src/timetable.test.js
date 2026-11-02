import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { net, WASM } from '../../running-times/test/net.js'
import { copyTrain, formatClock, newPoint, newTrain, numberAfter, parseClock, runTrain, taktCopies, trainProblems, trainTitle } from './timetable.js'

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

  it('says why it cannot run', async () => {
    expect(await runTrain(net, stations, train([['A-Dorf'], ['B-Halt']]), vehicle, simulate)).toEqual({ problems: [{ key: 'end_not_station', point: 1 }] })
    expect((await runTrain(net, stations, train([['A-Dorf'], ['C-Stadt'], ['B-Halt'], ['C-Stadt']]), vehicle, simulate)).failedAt).toBe(2)
  })
})
