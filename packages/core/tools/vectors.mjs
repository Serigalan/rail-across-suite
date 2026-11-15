#!/usr/bin/env node
// The shared test vectors of schemas/vectors/: what the core makes of
// canonical JSON (its JavaScript), the schema check (frozen since its
// JavaScript went, decision 0020 step 10) and whole documents (through the
// door, the core's module), written down so crates/doc is held to them (data
// model, S-R3; CONTRIBUTING).
//
//   node tools/vectors.mjs           write them
//   node tools/vectors.mjs --check   exit 1 where a file is not what the core makes now
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  canonicalJson, checkDocument, checkOperations, contentHash, derivedId, documentHash, manifestOf, PayloadError, toDocument, toOperations,
} from '../src/document/index.js'
import { loadNativeFile } from '../src/native/node.js'
import { ksStation, ksStoppingPlaces } from '../test/signallingFixture.js'

const OUT = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'schemas', 'vectors')

// ── Canonical JSON ──────────────────────────────────────────────────────────

/** Inputs as JSON text, so both languages parse the same thing. */
const CANONICAL_INPUTS = [
  '0', '-0', '-0.0', '1', '100.0', '1e2', '0.1', '-1.5', '1e-7', '0.000001', '0.0000012', '1e21', '1e20',
  '123456789012345680000', '1.2345678901234567e-300', '5e-324', '1.7976931348623157e308', '4.35', '0.5e-6',
  '9007199254740991', '-9007199254740991', '6050.000001', '108.5512', '-1.234e-5', '2.5e+25',
  'true', 'false', 'null', '""', '"a"', '"\\u0000\\u001f\\u007f"', '"\\b\\f\\n\\r\\t\\"\\\\/"', '"äöü ß €"',
  '"\\ud83d\\ude00"', '"\\u2028\\u2029"',
  '[]', '{}', '[1,[2,[3]],{"b":1,"a":[]}]',
  '{"b":2,"a":1,"c":{"z":null,"y":[true,false]}}',
  '{"\\u20ac":1,"\\r":2,"1":3,"\\ud83d\\ude00":4,"\\ufb33":5,"\\u0080":6}',
  '{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001]}',
  '{"elements":[{"elementType":1,"startNode":[4470000.123456,5332000.654321],"radius":-500,"r1":null}]}',
]

const canonicalVectors = () => CANONICAL_INPUTS.map(json => {
  const value = JSON.parse(json)
  return { json, canonical: canonicalJson(value), hash: contentHash(value) }
})

// ── The schema check ────────────────────────────────────────────────────────

// Its JavaScript is gone (decision 0020, step 10): schema.json stays as it was
// last written from it, and crates/doc is held to it.
const schemaVectors = () => JSON.parse(readFileSync(join(OUT, 'schema.json'), 'utf8'))


// ── Documents ───────────────────────────────────────────────────────────────
// Through the door — the core's module since step 10: --check sees that it
// still says of each what it said when the JavaScript was the measure.

// The station of the sample, under the id the door drew from its name while the record stated it as text.
const HALLE_TROTHA = 'b64cfd92-f881-83ad-82d7-f0d78ef78747'

/** A record with one of everything the format holds — with the signalling of a small station, its second stopping place at the platform, and an electrification where asked. */
function sampleRecord({ signalling = false, electrification = false } = {}) {
  const el = (x0, x1, extra = {}) => ({ elementType: 0, startNode: [500000 + x0, 5700000], endNode: [500000 + x1, 5700000], bearing: 90, length: x1 - x0, ...extra })
  return {
    id: 'b0d5c1a2-6f3e-4c77-9d0a-2f1e3c4b5a69', title: 'Halle-Trotha', description: 'vectors', creator: 'core',
    createdAt: '2026-10-29T13:21:07.630Z', crsSystem: 'ETRS89_UTM', lineCategory: 'secondary', gaugeProfile: 'hauptgleis',
    planHeader: { subtitle: 'Lageplan' }, imageHash: 'a'.repeat(64),
    tracks: [
      { id: 't1', name: '6050.001', trackType: 'line', lineNumber: 6050, side: 'regular', epsg: 25832, heightEpsg: 7837,
        elements: [el(0, 100, { speed: 80 }), el(100, 250, { cant: 0, role: 'follow' })],
        heights: [{ station: 0, z: 108.55 }, { station: 250, z: 109.1, rv: 15000 }] },
      { id: 't2', name: '1', trackType: 'station', trackNumber: '1', operationalPointId: HALLE_TROTHA,
        lifespan: { from: '2027-05-01' }, epsg: 25832, elements: [el(250, 400)] },
      { id: 't3', name: 'alt', lifespan: { to: 'open' }, epsg: 25832, elements: [el(400, 450)] },
    ],
    switches: [{ switchId: 'w1', kind: 'link', formVersion: 1, name: 'link.001',
      ports: { A: { trackId: 't1', endpoint: 'END' }, B: { trackId: 't2', endpoint: 'BEGIN' } } }],
    platforms: [{ id: 'p1', trackId: 't2', startStation: 10, endStation: 140, side: 'left', frontOffset: 1.67,
      height: 550, operationalPointId: HALLE_TROTHA, ...(signalling ? { stoppingPlaceIds: ['hp-2'] } : {}) }],
    operationalPoints: [{ id: HALLE_TROTHA, name: 'Halle-Trotha', rl100: 'LHT', uic: '8023111' }],
    endMarks: [{ id: 'e1', kind: 'buffer_stop', trackId: 't3', endpoint: 'END', type: 4, brakeLength: 4 }],
    routes: [{ id: 'r1', name: 'Gleis 1 durch', trackIds: ['t1', 't2'] }],
    kmLines: [{ id: 'km-6050', lineNumber: 6050, crs: 'EPSG:4326', clip: [11.9, 51.5, 12.0, 51.6], runs: [[[11.95, 51.55, 1200.5], [11.96, 51.56, 1300]]] }],
    axisSurveys: [{ id: 'a1', name: 'Trace', epsg: 25832, rail: '54E4', guide: { kind: 'track', trackId: 't1' },
      cloudNames: ['W0'], createdAt: '2026-10-29T13:21:07.630Z', step: 0.5, gaps: [],
      points: { e0: 500000, n0: 5700000, z0: 108, st: [0, 500], de: [0, 500], dn: [0, 0], zl: [550, 560], zr: [550, 561], ga: [1435, 1436], q: [1, 1] } }],
    referenceAxes: [{ id: 'x1', name: 'Bestand', epsg: 25832, heightEpsg: null, source: { tra: 'a.TRA', gra: null },
      points: { s0: 0, step: 0.01, count: 3, e0: 500000, n0: 5700000, de: [0, 10, 10], dn: [0, 0, 0], z: null } }],
    ...(signalling ? atPlatform({ signalling: ksStation({ operationalPointId: HALLE_TROTHA }), stoppingPlaces: ksStoppingPlaces({ operationalPointId: HALLE_TROTHA }) }) : {}),
    ...(electrification ? { electrification: { fields: {}, collections: {
      contactWires: [{ id: 'cw1', name: 'Oberleitung', voltage: '15000V' }],
      masts: [0, 250].map(station => ({ id: `m${station}`, kind: 'cantilever', at: { anchor: 'station', trackId: 't1', station, side: 'right', offset: 3.2 } })),
      suspensionPoints: [0, 250].map(station => ({
        id: `sp${station}`, mastId: `m${station}`, at: { anchor: 'station', trackId: 't1', station }, contactWireId: 'cw1', height: 5.5,
      })),
      sectionInsulators: [],
    } } } : {}),
  }
}

/** The station's stopping places, the second a passenger stop at the sample's platform, which names it (stoppingPlaceIds). */
function atPlatform(parts) {
  return { ...parts, stoppingPlaces: parts.stoppingPlaces.map(h => (h.id === 'hp-2' ? { ...h, passengerStop: true } : h)) }
}

// infra-data 1.0 documents as the core wrote them before 2.0, frozen: the door reads them as today's.
const BEFORE = JSON.parse(readFileSync(join(OUT, 'infra-data-1.0.json'), 'utf8')).documents
// infra-data 2.0 documents as the core wrote them last before decision 0028, frozen: a register of stages, the lifespans naming them.
const STAGES = JSON.parse(readFileSync(join(OUT, 'infra-data-2.0-stages.json'), 'utf8')).documents

/** [name, change, options] — a document made from the sample, changed so the door refuses it (or not). */
const DOCUMENT_CASES = [
  ['the sample', () => {}],
  ['the sample, carrying its schemas', (d, carry) => carry()],
  ['another format', (d) => { d.format = 'oltjson' }],
  ['a format version this build does not read', (d) => { d.version = '3.0' }],
  ['a domain version this build does not read', (d) => { d.domains.core.version = '2.5' }],
  ['a domain this build does not read', (d) => { d.domains.catenary = { ...d.domains.civil, version: '0.3' } }],
  ['a schema hash that is not the version\'s', (d) => { d.domains.civil.schema = `sha256:${'0'.repeat(64)}` }],
  ['a carried schema under another hash', (d, carry) => { carry(); d.schemas[d.domains.core.schema] = { $id: 'core' } }],
  ['a blob that is not what its hash says', (d) => { const [h] = Object.keys(d.blobs); d.blobs[h] = { e0: 1 } }],
  ['civil depending on nothing', (d) => { delete d.domains.civil.dependsOn }],
  ['a track twice', (d) => { d.domains.core.collections.tracks.push(d.domains.core.collections.tracks[0]) }],
  ['an element with derived geometry', (d) => { d.domains.core.collections.tracks[0].elements[0].renderCoords = [] }],
  ['a register of stages', (d) => { d.domains.core.fields.stages = [{ id: 's0' }, { id: 's1' }] }],
  ['a lifespan naming a stage', (d) => { d.domains.core.collections.tracks[1].lifespan = { from: 's1' } }],
  ['a lifespan that ends before it begins', (d) => { d.domains.core.collections.tracks[1].lifespan = { from: '2027-05-01', to: '2027-04-30' } }],
  ['a lifespan on a day there is none of', (d) => { d.domains.core.collections.tracks[1].lifespan = { from: '2027-02-30' } }],
  ['built for a while, the days open', (d) => { d.domains.core.collections.tracks[1].lifespan = { from: 'open', to: 'open' } }],
  ['a domain of a later version', (d) => { d.domains.core.version = '5.0' }],
  ['a platform with its stations loose', (d) => { const p = d.domains.civil.collections.platforms[0]; p.startStation = p.at.startStation }],
  ['a platform in the core', (d) => { d.domains.core.collections.platforms = d.domains.civil.collections.platforms }],
  ['a collection missing', (d) => { delete d.domains.core.collections.kmLines }],
  ['no core', (d) => { delete d.domains.core }],
  ['the core alone', (d) => { delete d.domains.civil }],
  ['the sample with its signalling', () => {}, { signalling: true }],
  ['the sample with its signalling, carrying its schemas', (d, carry) => carry(), { signalling: true }],
  ['signalling depending on nothing', (d) => { delete d.domains.signalling.dependsOn }, { signalling: true }],
  ['a signal acting in no direction', (d) => { delete d.domains.signalling.collections.signals.find(s => s.at.direction).at.direction }, { signalling: true }],
  ['a frame that shows nothing', (d) => { d.domains.signalling.collections.signals.find(s => s.frames?.length).frames[0].aspects = [] }, { signalling: true }],
  ['signalling without its overlaps', (d) => { delete d.domains.signalling.collections.overlaps }, { signalling: true }],
  ['stopping places in signalling', (d) => { d.domains.signalling.collections.stoppingPlaces = d.domains.core.collections.stoppingPlaces }, { signalling: true }],
  ['a boundary marked by a platform', (d) => { d.domains.signalling.collections.stationBoundaries[0].marks.push({ platformId: derivedId('p1') }) }, { signalling: true }],
  ['a route with its times, train protection', (d) => {
    d.domains.signalling.fields.routeTimes = { setting: 12, release: 6 }
    d.domains.signalling.collections.routes[0].settingTime = 8
    d.domains.signalling.collections.trainProtection = [{ id: derivedId('zb1'), at: { anchor: 'station', trackId: derivedId('t1'), startStation: 0, endStation: 250 }, system: 'pzb' }]
  }, { signalling: true }],
  ['the sample with its electrification', () => {}, { electrification: true }],
  ['a contact wire of no voltage', (d) => { delete d.domains.electrification.collections.contactWires[0].voltage }, { electrification: true }],
  ['a suspension point on a mast that is not there', (d) => { d.domains.electrification.collections.suspensionPoints[0].mastId = derivedId('m9') }, { electrification: true }],
  ['a boundary\'s headway', (d) => { d.domains.core.collections.endMarks[0].headway = 120 }],
  ['a track of a station that is not there', (d) => { d.domains.core.collections.tracks[1].operationalPointId = derivedId('op-nowhere') }],
  ['a switch element of a switch that is not there', (d) => { d.domains.signalling.collections.switchElements[0].switchId = derivedId('w9') }, { signalling: true }],
]

/** [name, change] — an infra-data 1.0 document (frozen), changed or not: the door reads it as today's. */
const BEFORE_CASES = [
  ['infra-data 1.0: the sample', 'the sample', () => {}],
  ['infra-data 1.0: the sample with its signalling', 'the sample with its signalling', () => {}],
  ['infra-data 1.0: a stopping place at a platform, a boundary marked by it', 'the sample with its signalling', (d) => {
    d.domains.signalling.collections.stoppingPlaces.find(h => h.id === 'hp-2').platformId = 'p1'
    d.domains.signalling.collections.stationBoundaries[0].marks.push({ platformId: 'p1' })
  }],
  ['infra-data 1.0: a boundary marked by a platform no stopping place is at', 'the sample with its signalling', (d) => {
    d.domains.signalling.collections.stationBoundaries[0].marks.push({ platformId: 'p1' })
  }],
  ['infra-data 1.0: core as written before the design speed took its name', 'the sample', (d) => {
    d.domains.core.schema = 'sha256:c237af09e3ac3915398d9c7795088518df4023bfd0b954053dc9f04112fbbaf6'
    for (const el of d.domains.core.collections.tracks.flatMap(t => t.elements)) {
      if (Object.hasOwn(el, 'designSpeed')) { el.speed = el.designSpeed; delete el.designSpeed }
    }
  }],
  ['infra-data 1.0: a domain this build does not read', 'the sample', (d) => { d.domains.catenary = { ...d.domains.layout, version: '0.3' } }],
  ['infra-data 1.0: layout of another schema', 'the sample', (d) => { d.domains.layout.schema = `sha256:${'0'.repeat(64)}` }],
  ['infra-data 1.0: no layout', 'the sample', (d) => { delete d.domains.layout }],
]

/** [name, change] — a document with its register of stages (frozen), changed or not: the door reads its lifespans by day (decision 0028). */
const STAGES_CASES = [
  ['stages: the sample', 'the sample', () => {}],
  ['stages: the sample, carrying its schemas', 'the sample, carrying its schemas', () => {}],
  ['stages: the sample with its signalling', 'the sample with its signalling', () => {}],
  ['stages: the sample with its electrification', 'the sample with its electrification', () => {}],
  ['stages: a stage with its day, a track out of service with it and one into', 'the sample', (d) => {
    const core = d.domains.core
    core.fields.stages.splice(1, 0, { id: 'umbau', name: 'Umbau', date: '2028-06-02' })
    core.collections.tracks[1].lifespan = { from: 'umbau' }
    core.collections.tracks[2].lifespan = { to: 'umbau' }
  }],
  ['stages: a lifespan naming a stage that is not there', 'the sample', (d) => { d.domains.core.collections.tracks[1].lifespan = { from: 'of-another-project' } }],
  ['stages: a signal of the final state', 'the sample with its signalling', (d) => {
    d.domains.signalling.collections.signals[0].lifespan = { from: d.domains.core.fields.stages[1].id }
  }],
  ['stages: civil of a version that does not go with the core\'s', 'the sample', (d) => { d.domains.civil.version = '2.0' }],
  ['stages: the core of another schema', 'the sample', (d) => { d.domains.core.schema = `sha256:${'0'.repeat(64)}` }],
  // A document of before could name what was not there; brought forward, it is cut as a deletion would have cut it.
  ['stages: references to nothing, cut as a deletion cuts them', 'the sample with its signalling', (d) => {
    const c = d.domains.signalling.collections
    c.signals[0].operationalPointId = 'op-nowhere'
    c.switchElements[0].switchId = 'w9'
    c.stationBoundaries[0].marks = [{ switchId: 'w9' }]
  }],
]

/** The verdict on a document: the manifest and hash of what the door takes, an earlier schema brought to this one. */
function verdict(doc) {
  try {
    const taken = checkDocument(doc)
    return { valid: true, manifest: manifestOf(taken), hash: documentHash(taken) }
  } catch (err) {
    if (!(err instanceof PayloadError)) throw err
    return { valid: false, code: err.code }
  }
}

const documentVectors = () => [
  ...DOCUMENT_CASES.map(([name, change, options = {}]) => {
    const doc = structuredClone(toDocument(sampleRecord(options)))
    const carry = () => { doc.schemas = structuredClone(toDocument(sampleRecord(options), { carrySchemas: true }).schemas) }
    change(doc, carry)
    return { name, document: doc, ...verdict(doc) }
  }),
  ...BEFORE_CASES.map(([name, from, change]) => {
    const doc = structuredClone(BEFORE[from])
    change(doc)
    return { name, document: doc, ...verdict(doc) }
  }),
  ...STAGES_CASES.map(([name, from, change]) => {
    const doc = structuredClone(STAGES[from])
    change(doc)
    return { name, document: doc, ...verdict(doc) }
  }),
]

// ── operations-1.0 ──────────────────────────────────────────────────────────

const stop = (id, place, extra = {}) => ({ id, operationalPointId: place, trackId: null, stop: true, dwell: 60, departure: null, itineraryId: null, ...extra })

/** A project's timetables as the suite keeps them: a Takt of two trains and a freight train, a vehicle carried. */
const operationsRecord = ({ vehicles = false } = {}) => ({
  timetables: [
    { id: 'tt-1', name: 'Grundtakt', data: {
      dispatch: { mode: 'wait', priority: 'category', categories: ['ICE', 'RB'], times: { setup: 12, approach: 1000, release: 6 } },
      trains: [
        { id: 'z-1', number: '16001', category: 'RB', name: '', vehicle: 'osrd:loco', mode: null, comfort: 'STANDARD',
          departure: 28800, margin: '5 %', distribution: 'mareco', points: [stop('a', 'halle'), stop('b', 'leipzig', { trackId: 't2', dwell: 120 })] },
        { id: 'z-2', number: '16003', category: 'RB', name: '', vehicle: 'osrd:loco', mode: null, comfort: 'STANDARD',
          departure: 32400, margin: '5 %', distribution: 'mareco', points: [stop('c', 'halle'), stop('d', 'leipzig')] },
      ] } },
    { id: 'tt-2', name: 'Güterverkehr', data: { trains: [
      { id: 'g-1', number: '', category: 'Gz', name: 'Kies', vehicle: null, mode: null, comfort: 'STANDARD',
        departure: 3600, margin: '', distribution: 'linear', points: [stop('e', 'leipzig', { stop: false }), stop('f', 'halle')] },
    ] } },
  ],
  ...(vehicles ? { vehicles: [{ key: 'osrd:loco', vehicle: {
    railjson_version: '3.3', name: 'Lok', mass: 84000, length: 19.5, effort_curves: { modes: { '15000V': { is_electric: true, curves: [{}] } } },
  } }] } : {}),
})

const OPERATIONS_CASES = [
  ['the timetables of a project', () => {}],
  ['with its vehicle carried', () => {}, { vehicles: true }],
  ['no timetables', (doc) => { for (const d of Object.values(doc.domains)) for (const c of Object.values(d.collections)) c.length = 0 }],
  ['a connection and a turnaround', (doc) => {
    const c = doc.domains.timetables.collections
    c.connections.push({ id: derivedId('an-1'), fromTrainId: derivedId('z-1'), toTrainId: derivedId('g-1'), operationalPointId: derivedId('leipzig'), changeTime: 300, waitUpTo: 180 })
    c.turnarounds.push({ id: derivedId('w-1'), fromTrainId: derivedId('z-1'), toTrainId: derivedId('z-2'), leastTime: 600 })
  }],
  ['another version', (doc) => { doc.version = '2.0' }],
  ['a train of no timetable', (doc) => { doc.domains.timetables.collections.trains[0].timetableId = derivedId('tt-9') }],
  ['a departure as text', (doc) => { doc.domains.timetables.collections.trains[0].departure = '8:00' }],
  ['a vehicle without its rolling stock and its key', (doc) => { delete doc.domains.vehicles.collections.vehicles[0].key; delete doc.domains.vehicles.collections.vehicles[0].rollingStock }],
  ['a formation of a vehicle that is not there', (doc) => { doc.domains.timetables.collections.formations[0].tractionUnits[0].vehicleId = derivedId('v-9') }],
  ['a collection missing', (doc) => { delete doc.domains.timetables.collections.studies }],
]

function operationsVerdict(doc) {
  try {
    return { valid: true, hash: documentHash(checkOperations(doc)) }
  } catch (err) {
    if (!(err instanceof PayloadError)) throw err
    return { valid: false, code: err.code }
  }
}

const operationsVectors = () => OPERATIONS_CASES.map(([name, change, options = {}]) => {
  const doc = structuredClone(toOperations(operationsRecord(options)))
  change(doc)
  return { name, document: doc, ...operationsVerdict(doc) }
})

// ── Write or check ──────────────────────────────────────────────────────────

const FILES = {
  'canonical.json': canonicalVectors,
  'schema.json': schemaVectors,
  'documents.json': documentVectors,
  'operations.json': operationsVectors,
}

function main() {
  if (!loadNativeFile()) throw new Error('the core\'s module is not built: npm run wasm')
  const check = process.argv[2] === '--check'
  let stale = 0
  for (const [file, make] of Object.entries(FILES)) {
    const text = `${JSON.stringify(make(), null, 1)}\n`
    const path = join(OUT, file)
    if (check) {
      let now = null
      try { now = readFileSync(path, 'utf8') } catch { /* not there */ }
      if (now !== text) { console.log(`schemas/vectors/${file} is not what the core makes — node tools/vectors.mjs`); stale++ }
    } else {
      writeFileSync(path, text)
      console.log(`schemas/vectors/${file}`)
    }
  }
  process.exitCode = stale ? 1 : 0
}

main()
