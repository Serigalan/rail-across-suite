import { describe, expect, it } from 'vitest'
import { checkOperations, fromOperations, OPERATIONS_SCHEMA_HASH, PayloadError, toOperations } from './index.js'
import { loadNativeFile } from '../native/node.js'

const point = (id, place) => ({ id, place, station: place, track: null, stop: true, dwell: 60, departure: null, itinerary: null })
const train = (id, departure) => ({
  id, number: '1', category: 'RE', name: '', vehicle: 'osrd:x', mode: null, comfort: 'STANDARD',
  departure, margin: '5 %', distribution: 'mareco', points: [point(`${id}a`, 'a'), point(`${id}b`, 'b')],
})

describe.skipIf(!loadNativeFile())('operations-1.0 (O1)', () => {
  it('takes a project\'s timetables into a document and gives them back, the trains by departure', () => {
    const record = { timetables: [{ id: 't', name: 'Takt', data: { trains: [train('b', 7200), train('a', 3600)] } }] }
    const doc = toOperations(record)
    expect(doc).toMatchObject({ format: 'operations', version: '1.0', domains: { operations: { version: '1.0', schema: OPERATIONS_SCHEMA_HASH } } })
    expect(checkOperations(doc)).toBe(doc)
    const back = fromOperations(doc)
    expect(back.timetables[0].data.trains.map(t => t.id)).toEqual(['a', 'b'])
    expect(back.timetables[0].data.trains[0]).toEqual(train('a', 3600))
  })

  it('refuses what is not as the schema says', () => {
    const doc = toOperations({ timetables: [{ id: 't', name: 'Takt', data: { trains: [train('a', 3600)] } }] })
    doc.domains.operations.collections.trains[0].departure = 'früh'
    expect(() => checkOperations(doc)).toThrow(PayloadError)
  })
})
