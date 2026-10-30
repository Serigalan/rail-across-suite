#!/usr/bin/env node
// The shared test vectors of schemas/vectors/: what the JavaScript core makes
// of canonical JSON, the schema check and whole documents, written down so
// the Python core (packages/core-py) is held to the same results (data model,
// S-R3; CONTRIBUTING: one implementation per language).
//
//   node tools/vectors.mjs           write them
//   node tools/vectors.mjs --check   exit 1 where a file is not what the core makes now
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  canonicalJson, checkDocument, contentHash, documentHash, manifestOf, PayloadError, SchemaSet, schemaOf, toDocument,
} from '../src/document/index.js'
import { ksStation } from '../test/signallingFixture.js'

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

const SCHEMAS = new SchemaSet(['rail-across', 'core', 'layout', 'signalling'].map(schemaOf))
const STATION = ksStation().collections
const SIGNAL = STATION.signals[0]
const TRACK = {
  id: 't1', epsg: 25832,
  elements: [{ elementType: 0, startNode: [500000, 5700000], endNode: [500100, 5700000], bearing: 90, length: 100 }],
}

/** [ref, value] — a value and the part of a schema it is checked against. */
const SCHEMA_CASES = [
  ['core#/$defs/TrackEnd', { trackId: 't1', endpoint: 'END' }],
  ['core#/$defs/TrackEnd', { trackId: 't1', endpoint: 'MIDDLE' }],
  ['core#/$defs/TrackEnd', { trackId: 't1' }],
  ['core#/$defs/TrackEnd', { trackId: '', endpoint: 'BEGIN', extra: 1 }],
  ['core#/$defs/TrackRange', { anchor: 'station', trackId: 't1', startStation: 120, endStation: 260, side: 'left' }],
  ['core#/$defs/TrackRange', { anchor: 'plane', trackId: 't1', startStation: 120, endStation: 260 }],
  ['core#/$defs/TrackLocation', { anchor: 'station', trackId: 't1', station: 812.4, side: 'right', offset: 2.9, direction: 'down' }],
  ['core#/$defs/TrackLocation', { anchor: 'plane', easting: 500000, northing: 5700000, epsg: 25832, bearing: 12 }],
  ['core#/$defs/TrackLocation', { anchor: 'station', trackId: 't1', station: 'km 3' }],
  ['core#/$defs/TrackLocation', { anchor: 'plane', easting: 500000, northing: 5700000 }],
  ['core#/$defs/TrackLocation', 'nowhere'],
  ['core#/$defs/Lifespan', { from: 's1' }],
  ['core#/$defs/Lifespan', {}],
  ['core#/$defs/Lifespan', { from: 's1', until: 's2' }],
  ['core#/$defs/TrackPath', { trackIds: ['a', 'b'] }],
  ['core#/$defs/TrackPath', { trackIds: 'a' }],
  ['core#/$defs/Track', TRACK],
  ['core#/$defs/Track', { ...TRACK, elements: [] }],
  ['core#/$defs/Track', { ...TRACK, epsg: 25832.5, trackType: 3 }],
  ['core#/$defs/Track', { ...TRACK, elements: [{ ...TRACK.elements[0], radius: null, r1: 'x', geometry: {} }] }],
  ['core#/$defs/Switch', { id: 's', kind: 'turnout', formVersion: 1, ports: { A: { trackId: 't1', endpoint: 'END' }, B1: null } }],
  ['core#/$defs/Switch', { id: 's', kind: 'turnout', formVersion: 1, ports: { E: { trackId: 't1', endpoint: 'END' } } }],
  ['core#/$defs/Switch', { id: 's', kind: 'turnout', formVersion: 1, ports: { A: { trackId: 't1' } } }],
  ['core#/$defs/OperationalPoint', { id: 'o', rl100: 'LHT' }],
  ['core#/$defs/OperationalPoint', { id: 'o' }],
  ['core#/$defs/KmLine', { id: 'km-6050', lineNumber: 6050, runs: [[[11.9, 51.5, 1200.5]]] }],
  ['core#/$defs/KmLine', { id: 'km-6050', lineNumber: 6050, runs: [[[11.9, 51.5]]] }],
  ['layout#/$defs/NamedTrackPath', { id: 'r', name: 'Route 1', path: { trackIds: ['t1'] } }],
  ['layout#/$defs/AxisSurvey', { id: 'a', epsg: 25832, points: 'sha256:abc' }],
  ['layout#/$defs/Fields', { planHeader: {}, osrd: { version: '4.0.0' }, imageHash: 'f00' }],
  ['signalling#/$defs/Signal', SIGNAL],
  ['signalling#/$defs/Signal', { ...SIGNAL, at: { anchor: 'station', trackId: 't1', station: 20 } }],
  ['signalling#/$defs/Signal', { ...SIGNAL, at: { anchor: 'plane', easting: 500000, northing: 5700000, epsg: 25832, bearing: 90 } }],
  ['signalling#/$defs/Signal', { ...SIGNAL, frames: [{ kind: 'screen', aspects: [] }] }],
  ['signalling#/$defs/Signal', { ...SIGNAL, kind: 'fictive', function: 'gate' }],
  ['signalling#/$defs/Route', STATION.routes[0]],
  ['signalling#/$defs/Route', { ...STATION.routes[0], overlaps: [{ preferred: true }] }],
  ['signalling#/$defs/Overlap', STATION.overlaps[0]],
  ['signalling#/$defs/FlankProtection', STATION.flankProtection[0]],
  ['signalling#/$defs/FlankProtection', STATION.flankProtection[1]],
  ['signalling#/$defs/FlankProtection', { ...STATION.flankProtection[0], protection: { signalId: 'sig-n1', switchElementId: 'we-1', position: 'left' } }],
  ['signalling#/$defs/FlankProtection', { ...STATION.flankProtection[0], protection: { waived: false } }],
  ['signalling#/$defs/PzbElement', STATION.pzbElements[3]],
  ['signalling#/$defs/PzbElement', { ...STATION.pzbElements[0], frequency: '1500' }],
  ['signalling#/$defs/StationBoundary', STATION.stationBoundaries[0]],
  ['signalling#/$defs/StationBoundary', { ...STATION.stationBoundaries[0], marks: [{ signalId: 'sig-a', switchId: 'w1' }] }],
  ['signalling#/$defs/TrackSection', { id: 'ts', kind: 'axleCounting', boundedBy: [] }],
]

const schemaVectors = () => SCHEMA_CASES.map(([ref, value]) => {
  const errors = SCHEMAS.check(ref, value, { limit: 1000 })
  return { ref, value, valid: errors.length === 0, paths: [...new Set(errors.map(e => e.path))].sort() }
})

// ── Documents ───────────────────────────────────────────────────────────────

/** A record with one of everything the format holds — with the signalling of a small station where asked. */
function sampleRecord({ signalling = false } = {}) {
  const el = (x0, x1, extra = {}) => ({ elementType: 0, startNode: [500000 + x0, 5700000], endNode: [500000 + x1, 5700000], bearing: 90, length: x1 - x0, ...extra })
  return {
    id: 'b0d5c1a2-6f3e-4c77-9d0a-2f1e3c4b5a69', title: 'Halle-Trotha', description: 'vectors', creator: 'core',
    createdAt: '2026-10-29T13:21:07.630Z', crsSystem: 'ETRS89_UTM', lineCategory: 'secondary', gaugeProfile: 'hauptgleis',
    planHeader: { subtitle: 'Lageplan' }, imageHash: 'a'.repeat(64),
    tracks: [
      { id: 't1', name: '6050.001', trackType: 1, lineNumber: 6050, side: 1, epsg: 25832, heightEpsg: 7837,
        elements: [el(0, 100, { speed: 80 }), el(100, 250, { cant: 0, role: 'follow' })],
        heights: [{ station: 0, z: 108.55 }, { station: 250, z: 109.1, rv: 15000 }] },
      { id: 't2', name: '1', trackType: 2, trackNumber: '1', stationName: 'Halle-Trotha', uicStation: '8023111',
        status: 'new', epsg: 25832, elements: [el(250, 400)] },
      { id: 't3', name: 'alt', status: 'removal', epsg: 25832, elements: [el(400, 450)] },
    ],
    switches: [{ switchId: 'w1', kind: 'link', formVersion: 1, name: 'link.001',
      ports: { A: { trackId: 't1', endpoint: 'END' }, B: { trackId: 't2', endpoint: 'BEGIN' } } }],
    platforms: [{ id: 'p1', trackId: 't2', startStation: 10, endStation: 140, side: 'left', frontOffset: 1.67,
      height: 550, stationName: 'Halle-Trotha', code: 'LHT' }],
    endMarks: [{ id: 'e1', kind: 'buffer_stop', trackId: 't3', endpoint: 'END', type: 4, brakeLength: 4 }],
    routes: [{ id: 'r1', name: 'Gleis 1 durch', trackIds: ['t1', 't2'] }],
    kmLines: [{ id: 'km-6050', lineNumber: 6050, crs: 'EPSG:4326', clip: [11.9, 51.5, 12.0, 51.6], runs: [[[11.95, 51.55, 1200.5], [11.96, 51.56, 1300]]] }],
    axisSurveys: [{ id: 'a1', name: 'Trace', epsg: 25832, rail: '54E4', guide: { kind: 'track', trackId: 't1' },
      cloudNames: ['W0'], createdAt: '2026-10-29T13:21:07.630Z', step: 0.5, gaps: [],
      points: { e0: 500000, n0: 5700000, z0: 108, st: [0, 500], de: [0, 500], dn: [0, 0], zl: [550, 560], zr: [550, 561], ga: [1435, 1436], q: [1, 1] } }],
    referenceAxes: [{ id: 'x1', name: 'Bestand', epsg: 25832, heightEpsg: null, source: { tra: 'a.TRA', gra: null },
      points: { s0: 0, step: 0.01, count: 3, e0: 500000, n0: 5700000, de: [0, 10, 10], dn: [0, 0, 0], z: null } }],
    ...(signalling ? { signalling: ksStation({ operationalPoint: 'Halle-Trotha' }) } : {}),
  }
}

/** [name, change] — a document made from the sample, changed so the door refuses it (or not). */
const DOCUMENT_CASES = [
  ['the sample', () => {}],
  ['the sample, carrying its schemas', (d, carry) => carry()],
  ['another format', (d) => { d.format = 'oltjson' }],
  ['a domain version this build does not read', (d) => { d.domains.core.version = '2.0' }],
  ['a domain this build does not read', (d) => { d.domains.catenary = { ...d.domains.layout, version: '0.3' } }],
  ['a schema hash that is not the version\'s', (d) => { d.domains.layout.schema = `sha256:${'0'.repeat(64)}` }],
  ['a carried schema under another hash', (d, carry) => { carry(); d.schemas[d.domains.core.schema] = { $id: 'core' } }],
  ['a blob that is not what its hash says', (d) => { const [h] = Object.keys(d.blobs); d.blobs[h] = { e0: 1 } }],
  ['layout depending on nothing', (d) => { delete d.domains.layout.dependsOn }],
  ['a track twice', (d) => { d.domains.core.collections.tracks.push(d.domains.core.collections.tracks[0]) }],
  ['an element with derived geometry', (d) => { d.domains.core.collections.tracks[0].elements[0].renderCoords = [] }],
  ['one stage only', (d) => { d.domains.core.fields.stages.pop() }],
  ['a platform with its stations loose', (d) => { const p = d.domains.core.collections.platforms[0]; p.startStation = p.at.startStation }],
  ['a collection missing', (d) => { delete d.domains.core.collections.kmLines }],
  ['no core', (d) => { delete d.domains.core }],
  ['the sample with its signalling', () => {}, { signalling: true }],
  ['the sample with its signalling, carrying its schemas', (d, carry) => carry(), { signalling: true }],
  ['signalling depending on nothing', (d) => { delete d.domains.signalling.dependsOn }, { signalling: true }],
  ['a signal acting in no direction', (d) => { delete d.domains.signalling.collections.signals[0].at.direction }, { signalling: true }],
  ['a frame that shows nothing', (d) => { d.domains.signalling.collections.signals[0].frames[0].aspects = [] }, { signalling: true }],
  ['signalling without its overlaps', (d) => { delete d.domains.signalling.collections.overlaps }, { signalling: true }],
]

function verdict(doc) {
  try {
    checkDocument(doc)
    return { valid: true, manifest: manifestOf(doc), hash: documentHash(doc) }
  } catch (err) {
    if (!(err instanceof PayloadError)) throw err
    return { valid: false, code: err.code }
  }
}

const documentVectors = () => DOCUMENT_CASES.map(([name, change, options = {}]) => {
  const doc = structuredClone(toDocument(sampleRecord(options)))
  const carry = () => { doc.schemas = structuredClone(toDocument(sampleRecord(options), { carrySchemas: true }).schemas) }
  change(doc, carry)
  return { name, document: doc, ...verdict(doc) }
})

// ── Write or check ──────────────────────────────────────────────────────────

const FILES = {
  'canonical.json': canonicalVectors,
  'schema.json': schemaVectors,
  'documents.json': documentVectors,
}

function main() {
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
