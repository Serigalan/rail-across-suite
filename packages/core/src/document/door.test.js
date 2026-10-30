import { describe, it, expect } from 'vitest'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { canonicalJson, contentHash } from './canonical.js'
import { defaultStages } from './coreFormat.js'
import { checkDocument, fromDocument, SCHEMA_HASHES, toDocument } from './door.js'
import { documentHash, manifestOf } from './manifest.js'
import { hasPek, loadPek } from '../../test/pekFixture.js'
import { ksStation } from '../../test/signallingFixture.js'

const el = (x0, x1, extra = {}) => ({
  elementType: 0, startNode: [500000 + x0, 5700000], endNode: [500000 + x1, 5700000], bearing: 90, length: x1 - x0, ...extra,
})
const track = (id, extra = {}) => ({ id, name: id, epsg: 25832, elements: [el(0, 100)], ...extra })
const record = (extra = {}) => ({ id: 'p', title: 'P', tracks: [track('t1')], switches: [], platforms: [], ...extra })

/** The record a record comes back as, through its document. */
const roundTrip = (r) => fromDocument(JSON.parse(JSON.stringify(toDocument(r, { carrySchemas: true }))))

describe('a record as a core 1.0 document', () => {
  it('holds the domains core and layout, each with its version and schema', () => {
    const doc = toDocument(record({ planHeader: { subtitle: 'x' } }))
    expect(doc.format).toBe('rail-across')
    expect(doc.domains.core).toMatchObject({ version: '1.0', schema: SCHEMA_HASHES.core, fields: { id: 'p', title: 'P' } })
    expect(doc.domains.layout).toMatchObject({ version: '1.0', schema: SCHEMA_HASHES.layout, dependsOn: ['core'], fields: { planHeader: { subtitle: 'x' } } })
    expect(doc.domains.core.collections.tracks.map(t => t.id)).toEqual(['t1'])
  })

  it('carries its schemas where it travels, under the hashes its domains name', () => {
    const doc = toDocument(record(), { carrySchemas: true })
    expect(Object.keys(doc.schemas).sort()).toEqual([SCHEMA_HASHES.core, SCHEMA_HASHES.layout].sort())
    for (const [hash, schema] of Object.entries(doc.schemas)) expect(contentHash(schema)).toBe(hash)
    expect(toDocument(record()).schemas).toBeUndefined()
  })

  it('turns the planning status into a lifespan over the two stages, and back', () => {
    const r = record({ tracks: [track('a'), track('b', { status: 'new' }), track('c', { status: 'removal' }), track('d', { status: 'existing' })] })
    const doc = toDocument(r)
    const [existing, final] = doc.domains.core.fields.stages
    expect([existing, final]).toEqual(defaultStages('p'))
    expect(doc.domains.core.collections.tracks.map(t => t.lifespan)).toEqual([undefined, { from: final.id }, { to: final.id }, undefined])
    expect(fromDocument(doc).tracks.map(t => t.status)).toEqual([undefined, 'new', 'removal', undefined])
  })

  it('gives a platform its location as one value, and an end mark its track end', () => {
    const r = record({
      platforms: [{ id: 'pf', trackId: 't1', startStation: 10, endStation: 90, side: 'left', frontOffset: 1.67 }],
      endMarks: [{ id: 'e', kind: 'buffer_stop', trackId: 't1', endpoint: 'END', type: 4, brakeLength: 4 }],
    })
    const { platforms, endMarks } = toDocument(r).domains.core.collections
    expect(platforms).toEqual([{ id: 'pf', at: { anchor: 'station', trackId: 't1', startStation: 10, endStation: 90, side: 'left' }, frontOffset: 1.67 }])
    expect(endMarks).toEqual([{ id: 'e', kind: 'buffer_stop', type: 4, brakeLength: 4, at: { trackId: 't1', endpoint: 'END' } }])
    expect(roundTrip(r)).toEqual(r)
  })

  it('names stations in a register: one point per name, the codes stated for it', () => {
    const r = record({
      tracks: [track('a', { stationName: 'Halle-Trotha', uicStation: '8023111' }), track('b', { stationName: 'Halle-Trotha' })],
      platforms: [{ id: 'pf', trackId: 'a', startStation: 0, endStation: 50, stationName: 'Halle-Trotha', code: 'LHT' }],
    })
    const { tracks, platforms, operationalPoints } = toDocument(r).domains.core.collections
    expect(operationalPoints).toEqual([{ id: expect.any(String), name: 'Halle-Trotha', rl100: 'LHT', uic: '8023111' }])
    expect([...tracks, ...platforms].map(o => o.operationalPoint)).toEqual(Array(3).fill(operationalPoints[0].id))
    // A station's codes are the station's: every track naming it shows them.
    expect(roundTrip(r).tracks.map(t => t.uicStation)).toEqual(['8023111', '8023111'])
    expect(roundTrip(r).platforms[0]).toMatchObject({ stationName: 'Halle-Trotha', code: 'LHT' })
  })

  it('keeps two codes under one name apart, as two points', () => {
    const r = record({ tracks: [track('a', { stationName: 'X', uicStation: '1' }), track('b', { stationName: 'X', uicStation: '2' })] })
    expect(toDocument(r).domains.core.collections.operationalPoints.map(p => p.uic).sort()).toEqual(['1', '2'])
    expect(roundTrip(r).tracks.map(t => t.uicStation)).toEqual(['1', '2'])
  })

  it('makes runs of tracks TrackPaths and the bulk point arrays blobs', () => {
    const points = { s0: 0, step: 0.01, count: 2, e0: 1, n0: 2, de: [0, 1], dn: [0, 0], z: null }
    const r = record({
      routes: [{ id: 'r', name: 'R', trackIds: ['t1'] }],
      referenceAxes: [{ id: 'x', name: 'X', epsg: 25832, heightEpsg: null, source: { tra: 'a.TRA' }, points }],
    })
    const doc = toDocument(r)
    expect(doc.domains.layout.collections.trackPaths).toEqual([{ id: 'r', name: 'R', path: { trackIds: ['t1'] } }])
    const [axis] = doc.domains.layout.collections.referenceAxes
    expect(axis.points).toBe(contentHash(points))
    expect(doc.blobs[axis.points]).toEqual(points)
    expect(roundTrip(r)).toEqual(r)
  })

  it('sorts every collection by id, so the same state is the same bytes', () => {
    const a = toDocument(record({ tracks: [track('b'), track('a')] }))
    const b = toDocument(record({ tracks: [track('a'), track('b')] }))
    expect(canonicalJson(a)).toBe(canonicalJson(b))
    expect(a.domains.core.collections.tracks.map(t => t.id)).toEqual(['a', 'b'])
  })

  it('says what is the same in one way: no null fields, no empty station', () => {
    const r = record({ tracks: [track('a', { lineNumber: null, stationName: '', uicStation: '' })] })
    expect(roundTrip(r).tracks[0]).toEqual(track('a'))
  })

  it('refuses a field the format does not have, whole', () => {
    expect(() => toDocument(record({ name: 'p' }))).toThrow(expect.objectContaining({ code: 'invalid_payload', details: ['/name: is not a field of the record'] }))
    expect(() => toDocument(record({ tracks: [track('a', { colour: 'red' })] })))
      .toThrow(expect.objectContaining({ details: ['/domains/core/collections/tracks/0: has colour, which it may not'] }))
  })
})

describe('the door', () => {
  it('takes a document of this build and gives the record back', () => {
    const doc = toDocument(record())
    expect(checkDocument(doc)).toBe(doc)
    expect(fromDocument(doc)).toEqual(record())
  })

  it('refuses a version or a domain it does not read', () => {
    const doc = toDocument(record())
    expect(() => checkDocument({ ...doc, domains: { ...doc.domains, core: { ...doc.domains.core, version: '1.1' } } })).toThrow('unsupported_version')
    expect(() => checkDocument({ ...doc, domains: { ...doc.domains, catenary: doc.domains.layout } })).toThrow('unsupported_version')
  })

  it('carries the signalling domain through the record untouched, and carries its schema only where it is', () => {
    const signalling = ksStation()
    const doc = toDocument(record({ signalling }), { carrySchemas: true })
    expect(doc.domains.signalling).toMatchObject({ version: '1.0', schema: SCHEMA_HASHES.signalling, dependsOn: ['core'] })
    expect(Object.keys(doc.schemas)).toContain(SCHEMA_HASHES.signalling)
    expect(roundTrip(record({ signalling }))).toEqual(record({ signalling: {
      fields: signalling.fields,
      collections: Object.fromEntries(Object.entries(signalling.collections).map(([k, v]) => [k, [...v].sort((a, b) => (a.id < b.id ? -1 : 1))])),
    } }))
    expect(Object.keys(toDocument(record(), { carrySchemas: true }).schemas)).not.toContain(SCHEMA_HASHES.signalling)
  })

  it('refuses a signalling domain its schema does not take', () => {
    const signalling = ksStation()
    delete signalling.collections.signals[0].at.direction
    expect(() => toDocument(record({ signalling }))).toThrow('invalid_payload')
  })

  it('refuses stages of a project\'s own, which this build cannot hold yet', () => {
    const doc = toDocument(record())
    doc.domains.core.fields.stages.splice(1, 0, { id: 'stage-2' })
    expect(() => fromDocument(doc)).toThrow('stages_unsupported')
  })

  it('refuses a document whose blob is not what its hash says', () => {
    const doc = toDocument(record({ axisSurveys: [{ id: 'a', epsg: 25832, points: { st: [0] } }] }))
    const [hash] = Object.keys(doc.blobs)
    doc.blobs[hash] = { st: [1] }
    expect(() => checkDocument(doc)).toThrow(expect.objectContaining({ details: [`/blobs/${hash}: holds other content`] }))
  })
})

describe('the manifest', () => {
  it('names each collection by its hash: an unchanged collection, the same hash', () => {
    const a = toDocument(record())
    const b = toDocument(record({ title: 'Q' }))
    expect(manifestOf(a).domains.core.collections.tracks).toBe(manifestOf(b).domains.core.collections.tracks)
    expect(manifestOf(a).domains.core.fields).not.toBe(manifestOf(b).domains.core.fields)
    expect(documentHash(a)).not.toBe(documentHash(b))
  })
})

describe.skipIf(!hasPek)('the PEK data set as a core 1.0 document', () => {
  it('opens and saves losslessly — nulls in its metadata aside, which the document leaves out', () => {
    const pek = loadPek()
    const back = roundTrip(pek)
    const withoutNull = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v !== null))
    const byId = (list, key = 'id') => Object.fromEntries(list.map(o => [o[key], o]))
    expect(byId(back.tracks)).toEqual(byId(pek.tracks.map(withoutNull)))
    expect(byId(back.switches, 'switchId')).toEqual(byId(pek.switches, 'switchId'))
    expect(byId(back.platforms)).toEqual(byId(pek.platforms))
    const { tracks: _t, switches: _s, platforms: _p, ...fields } = pek
    expect(back).toMatchObject(fields)
    // And the document of what came back is the same bytes.
    expect(canonicalJson(toDocument(back))).toBe(canonicalJson(toDocument(pek)))
  })

  it('names its four stations once each', () => {
    const points = toDocument(loadPek()).domains.core.collections.operationalPoints
    expect(points.map(p => p.name).sort()).toEqual(['Halle (Saale) Hbf', 'Halle-Trotha', 'Könnern', 'Wallwitz (Saalkr)'])
  })
})

describe('what is written from the schemas and the core', () => {
  const current = (name) => () => {
    const tool = fileURLToPath(new URL(`../../tools/${name}`, import.meta.url))
    expect(() => execFileSync(process.execPath, [tool, '--check'], { encoding: 'utf8' })).not.toThrow()
  }
  it('the shared vectors (schemas/vectors) are what the core makes now', current('vectors.mjs'))
  it('the types (types/document.d.ts) are what the schemas give now', current('types.mjs'))
})
