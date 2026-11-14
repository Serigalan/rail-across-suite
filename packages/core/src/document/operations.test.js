import { describe, expect, it } from 'vitest'
import { checkOperations, derivedId, fromOperations, OPERATIONS_SCHEMA_HASHES, PayloadError, toOperations } from './index.js'
import { loadNativeFile } from '../native/node.js'

// Ids as the door writes them: UUIDs (decision 0029); a point names its place by id (timetables 1.0).
const point = (id, place) => ({ id: derivedId(id), operationalPointId: derivedId(place), trackId: null, stop: true, dwell: 60, departure: null, itineraryId: null })
const train = (id, departure) => ({
  id: derivedId(id), number: '1', category: 'RE', name: '', vehicle: 'osrd:x', mode: null, comfort: 'STANDARD',
  departure, margin: '5 %', distribution: 'mareco', points: [point(`${id}a`, 'a'), point(`${id}b`, 'b')],
})

describe.skipIf(!loadNativeFile())('operations-1.0 (O1; decision 0029, phase 4)', () => {
  it('takes a project\'s timetables into a document and gives them back, the trains by departure', () => {
    const record = { timetables: [{ id: 't', name: 'Takt', data: { trains: [train('b', 7200), train('a', 3600)] } }] }
    const doc = toOperations(record)
    expect(doc).toMatchObject({ format: 'operations', version: '1.0', domains: {
      vehicles: { version: '1.0', schema: OPERATIONS_SCHEMA_HASHES.vehicles },
      timetables: { version: '1.0', schema: OPERATIONS_SCHEMA_HASHES.timetables, dependsOn: ['vehicles'] },
    } })
    expect(checkOperations(doc)).toBe(doc)
    const back = fromOperations(doc)
    expect(back.timetables[0].data.trains.map(t => t.id)).toEqual([derivedId('a'), derivedId('b')])
    expect(back.timetables[0].data.trains[0]).toEqual(train('a', 3600))
  })

  it('reads a train\'s point of before, its place in one member, as naming an operational point', () => {
    const before = { id: 'p', place: 'a', station: 'A', track: 't', stop: true, dwell: 60, departure: null, itinerary: null }
    const doc = toOperations({ timetables: [{ id: 't', name: 'Takt', data: { trains: [{ ...train('a', 3600), points: [before, point('q', 'b')] }] } }] })
    expect(doc.domains.timetables.collections.trains[0].points[0]).toEqual({ id: derivedId('p'), operationalPointId: derivedId('a'), trackId: derivedId('t'), stop: true, dwell: 60 })
  })

  it('refuses what is not as the schema says', () => {
    const doc = toOperations({ timetables: [{ id: 't', name: 'Takt', data: { trains: [train('a', 3600)] } }] })
    doc.domains.timetables.collections.trains[0].departure = 'früh'
    expect(() => checkOperations(doc)).toThrow(PayloadError)
  })
})
