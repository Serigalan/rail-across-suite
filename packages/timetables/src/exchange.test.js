import { describe, expect, it } from 'vitest'
import {
  isTimetablesData, isTimetablesFile, operationsFile, operationsFileName, parseTimetablesFile, readTimetables, timetablesFile, timetablesFileName,
  timetablesOf, timetablesOfText, withCarriedVehicles,
} from './exchange.js'

const listed = [
  { id: 'a', name: 'Takt', version: 3, data: { trains: [{ id: 't1' }], dispatch: { mode: 'wait' } }, mayEdit: true },
  { id: 'b', name: 'Nacht', version: 1, data: { trains: [] } },
]

describe('a file of timetables', () => {
  it('holds their names and data, not what the server keeps besides, and reads back as it went out', () => {
    const file = timetablesFile(listed, { project: 'Halle–Könnern' })
    expect(file).toEqual({
      format: 'rail-across-timetables', version: '1.0', project: 'Halle–Könnern',
      timetables: [{ name: 'Takt', data: { trains: [{ id: 't1' }], dispatch: { mode: 'wait' } } }, { name: 'Nacht', data: { trains: [] } }],
    })
    expect(isTimetablesFile(file)).toBe(true)
    expect(timetablesOfText(JSON.stringify(file))).toEqual(file.timetables)
    expect(timetablesFileName('Halle–Könnern')).toBe('Halle–Könnern – Fahrpläne.json')
  })

  it('is refused where it is another version, another file or not of its shape', () => {
    const file = timetablesFile(listed)
    const code = (f) => { try { f(); return null } catch (err) { return err.code } }
    expect(code(() => parseTimetablesFile({ ...file, version: '2.0' }))).toBe('timetables_version')
    expect(code(() => parseTimetablesFile({ format: 'infra-data', version: '1.0', domains: {} }))).toBe('timetables_invalid')
    expect(code(() => parseTimetablesFile({ ...file, timetables: [{ name: ' ', data: { trains: [] } }] }))).toBe('timetables_invalid')
    expect(code(() => parseTimetablesFile({ ...file, timetables: [{ name: 'F', data: {} }] }))).toBe('timetables_invalid')
    expect(code(() => timetablesOfText('{'))).toBe('timetables_invalid')
    expect(isTimetablesFile(null)).toBe(false)
  })
})

describe('an operations file (operations-1.0, decision 0026)', () => {
  const point = (id, place) => ({ id, place, station: place, track: null, stop: true, dwell: 60, departure: null, itinerary: null })
  const train = (id, vehicle) => ({
    id, number: '1', category: 'RE', name: '', vehicle, mode: null, comfort: 'STANDARD', departure: 3600, margin: '5 %', distribution: 'mareco',
    points: [point(`${id}a`, 'a'), point(`${id}b`, 'b')],
  })
  const timetables = [{ id: 'x', name: 'Takt', version: 4, data: { trains: [train('t1', 'osrd:loco'), train('t2', 'own-1')] } }]
  const loco = { name: 'Lok', mass: 84000 }
  const own = { name: 'Eigener Triebwagen', mass: 50000 }
  const entries = [{ key: 'osrd:loco', vehicle: loco }, { key: 'own-1', vehicle: own }, { key: 'osrd:other', vehicle: { name: 'X' } }]

  it('carries the vehicles its trains run with, and reads back as it went out', () => {
    const file = operationsFile(timetables, entries)
    expect(file).toMatchObject({ format: 'operations', version: '1.0' })
    expect(Object.values(file.blobs)).toEqual(expect.arrayContaining([loco, own]))
    expect(Object.values(file.blobs)).toHaveLength(2)
    const read = readTimetables(JSON.stringify(file))
    expect(read.timetables).toEqual([{ name: 'Takt', data: { trains: [train('t1', 'osrd:loco'), train('t2', 'own-1')] } }])
    expect(read.vehicles.map(v => v.key).sort()).toEqual(['osrd:loco', 'own-1'])
    expect(operationsFileName('Halle')).toBe('Halle – Fahrpläne.operations.json')
    // the file of before is still read
    expect(readTimetables(JSON.stringify(timetablesFile(timetables))).timetables).toEqual([{ name: 'Takt', data: timetables[0].data }])
  })

  it('takes a vehicle the catalogue has as it is, and makes one it lacks or has otherwise, beside its own', async () => {
    const { timetables: read, vehicles } = readTimetables(JSON.stringify(operationsFile(timetables, entries)))
    const made = []
    const create = async (vehicle) => { made.push(vehicle); return { id: `new-${made.length}` } }
    const here = [{ key: 'osrd:loco', vehicle: loco }, { key: 'own-1', vehicle: { ...own, mass: 1 } }]
    const { timetables: taken, added } = await withCarriedVehicles(read, vehicles, { entries: here, create })
    expect(made).toEqual([own])
    expect(added).toEqual([{ key: 'own-1', id: 'new-1', name: 'Eigener Triebwagen' }])
    expect(taken[0].data.trains.map(t => t.vehicle)).toEqual(['osrd:loco', 'new-1'])
  })

  it('is refused where it is not one to read', () => {
    const file = operationsFile(timetables, entries)
    const code = (f) => { try { f(); return null } catch (err) { return err.code } }
    expect(code(() => timetablesOf({ ...file, version: '2.0' }))).toBe('timetables_version')
    expect(code(() => timetablesOf({ ...file, blobs: {} }))).toBe('timetables_invalid')
    expect(isTimetablesData(file)).toBe(true)
  })
})
