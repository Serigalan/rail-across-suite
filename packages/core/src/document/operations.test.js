import { describe, expect, it } from 'vitest'
import { checkOperations, contentHash, derivedId, fromOperations, OPERATIONS_SCHEMA_HASHES, PayloadError, toOperations } from './index.js'
import { loadNativeFile } from '../native/node.js'
import TIMETABLES_1_0 from '../../schemas/timetables/1.0.schema.json' with { type: 'json' }

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
      timetables: { version: '1.1', schema: OPERATIONS_SCHEMA_HASHES.timetables, dependsOn: ['vehicles'] },
    } })
    expect(checkOperations(doc)).toBe(doc)
    const back = fromOperations(doc)
    expect(back.timetables[0].data.trains.map(t => t.id)).toEqual([derivedId('a'), derivedId('b')])
    expect(back.timetables[0].data.trains[0]).toEqual(train('a', 3600))
  })

  it('keeps where a train turns and its turnaround (timetables 1.1), and reads a document of timetables 1.0 as it is', () => {
    const turning = train('a', 3600)
    Object.assign(turning.points[0], { reverse: true, turnaround: 420 })
    const doc = toOperations({ timetables: [{ id: 't', name: 'Takt', data: { trains: [turning] } }] })
    expect(doc.domains.timetables.collections.trains[0].points[0]).toMatchObject({ reverse: true, turnaround: 420 })
    expect(fromOperations(doc).timetables[0].data.trains[0].points[0]).toMatchObject({ reverse: true, turnaround: 420 })
    // a domain of 1.0 is brought forward; one that states a turn is no 1.0
    const plain = toOperations({ timetables: [{ id: 't', name: 'Takt', data: { trains: [train('a', 3600)] } }] })
    const old = { ...plain, domains: { ...plain.domains, timetables: { ...plain.domains.timetables, version: '1.0', schema: contentHash(TIMETABLES_1_0) } } }
    expect(checkOperations(old).domains.timetables).toMatchObject({ version: '1.1', schema: OPERATIONS_SCHEMA_HASHES.timetables })
    const turned = { ...doc, domains: { ...doc.domains, timetables: { ...doc.domains.timetables, version: '1.0', schema: contentHash(TIMETABLES_1_0) } } }
    expect(() => checkOperations(turned)).toThrow(PayloadError)
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
