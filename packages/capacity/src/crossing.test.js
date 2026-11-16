import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { DISPATCH, newPoint, newTrain, occupancy, parseClock, runTrain } from '@rail-across/timetables'
import { WASM } from '../../running-times/test/net.js'
import { crossing } from '../../timetables/test/crossing.js'
import { analyseStretch, proposedStretches, recordAtStage, stationOccupancy } from './index.js'

// The whole way, with the simulation module: a timetable run over a single line with a crossing station, its
// stretches proposed and compressed.
describe.skipIf(!existsSync(WASM))('the capacity of a single line with a crossing station', () => {
  const simulate = async (request) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    return runWith(instance.exports, request)
  }
  const record = recordAtStage({ id: 'p', ...crossing }, 'final')
  const stations = stationsOf(record)
  const vehicle = newVehicle('Triebzug')
  const runOf = (t, holds, routes) => runTrain(record, stations, t, vehicle, simulate, holds, routes)
  // A point names its place by id (timetables 1.0): the station of that name.
  const named = (name) => stations.find(s => s.name === name)
  const train = (id, clock, from, to, category) => newTrain({ id, number: id, category, vehicle: 'v', departure: parseClock(clock), points: [newPoint(named(from)), newPoint(named(to))] })
  const trains = [
    train('1', '7:00', 'H-Stadt', 'K-Stadt', 'RB'), train('2', '7:00', 'K-Stadt', 'H-Stadt', 'RB'),
    train('3', '7:30', 'H-Stadt', 'K-Stadt', 'Gz'), train('4', '7:40', 'K-Stadt', 'H-Stadt', 'RB'),
  ]

  it('proposes its two stretches, either side of the crossing station, and compresses each', async () => {
    const runs = await occupancy(record, trains, runOf, DISPATCH)
    const stretches = proposedStretches(stations, trains, runs)
    expect(stretches.map(s => s.id).sort()).toEqual(['H-Stadt|W-Dorf', 'K-Stadt|W-Dorf'].map(id => id.split('|').map(n => stations.find(s => s.name === n).key).sort().join('|')).sort())
    for (const s of stretches) {
      const r = analyseStretch(s, trains, runs, { from: parseClock('7:00'), to: parseClock('8:00') })
      expect(r.trains).toHaveLength(4)
      // On one track, each train holds what the next holds: compressed, they follow each other at their headways.
      expect(r.occupancy).toBeGreaterThan(0)
      expect(r.rate).toBeGreaterThan(0)
      expect(r.rate).toBeLessThan(1)
      expect(r.critical.length).toBeGreaterThan(1)
      // each train of the critical chain after the first bound by an element both hold, the areas held by two or more
      expect(r.binding.map(b => b.after)).toEqual(r.critical.slice(1))
      for (const b of r.binding) {
        expect(b.resource).toMatch(/^(section|track):/)
        expect(r.areas.some(a => a.resource === b.resource || a.trains.includes(b.after))).toBe(true)
      }
      expect(r.areas.every(a => a.trains.length >= 2)).toBe(true)
      expect(r.headways.map(h => `${h.before}>${h.after}`)).toEqual(expect.arrayContaining(['RB>RB', 'RB>Gz', 'Gz>RB']))
      // A train is never compressed to before the one before it.
      const at = r.trains.map(t => t.compressedAt)
      expect(at).toEqual([...at].sort((x, y) => x - y))
    }
    const w = stations.find(s => s.name === 'W-Dorf')
    const held = stationOccupancy(w, trains, runs, record, { from: parseClock('7:00'), to: parseClock('8:00') })
    expect(held.map(h => h.resource)).toEqual(expect.arrayContaining(['section:track1']))
  })
})
