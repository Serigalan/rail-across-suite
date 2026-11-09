import { describe, it, expect } from 'vitest'
import { existsSync, readFileSync } from 'node:fs'
import { parseXml, placeSignalling, planproRecord } from './index.js'
import { checkSignalling } from '../signalling/index.js'
import { projectDocument, validateProject } from '../project/index.js'
import { straightElement } from '../geometry/index.js'
import { e3Length, plan } from '../../test/planproFixture.js'

const PROJECT = 'b3c4d5e6-0000-4000-8000-000000000001'
const read = () => planproRecord(plan, { id: PROJECT })
const byId = (list, id) => list.find(o => o.id === id)

describe('the XML reader', () => {
  it('reads elements, attributes, text and entities, and skips the rest', () => {
    const doc = parseXml('<?xml version="1.0"?><!-- c --><a x="1 &amp; 2"><b>t&lt;u&#x41;</b><c/><![CDATA[<raw>]]></a>')
    expect(doc).toMatchObject({ name: 'a', attrs: { x: '1 & 2' }, text: '<raw>' })
    expect(doc.children.map(c => [c.name, c.text])).toEqual([['b', 't<uA'], ['c', '']])
    expect(() => parseXml('<a><b></a>')).toThrow(/closes/)
  })
})

describe('a PlanPro plan as a record', () => {
  it('makes every edge a track along its geometry, every node of tip, left and right a turnout', () => {
    const { record } = read()
    expect(record.title).toBe('Neubau ESTW Testdorf')
    expect(record.tracks.map(t => [t.id, t.epsg, Math.round(t.elements.reduce((s, e) => s + e.length, 0) * 1000) / 1000]))
      .toEqual([['e1', 5684, 200], ['e2', 5684, 100], ['e3', 5684, Number(e3Length)]])
    // e2's geometry ran from its end to the switch: turned to run A to B.
    expect(record.tracks[1].elements[0].startNode).toEqual([4500200, 5600000])
    expect(record.switches).toEqual([expect.objectContaining({
      switchId: 'k1', kind: 'turnout', name: '60W1', label: 'EW 54-500-1:12',
      ports: { A: { trackId: 'e1', endpoint: 'END' }, B1: { trackId: 'e3', endpoint: 'BEGIN' }, B2: { trackId: 'e2', endpoint: 'BEGIN' } },
    })])
    expect(record.endMarks.map(m => [m.trackId, m.endpoint, m.kind]).sort()).toEqual([
      ['e1', 'BEGIN', 'boundary'], ['e2', 'END', 'buffer_stop'], ['e3', 'END', 'boundary'],
    ])
    expect(validateProject(record).errors).toEqual([])
  })

  it('reads its signals with their frames, places and states', () => {
    const { signals } = read().record.signalling.collections
    expect(byId(signals, 'sig-a')).toMatchObject({
      name: 'A', areaNumber: '60', kind: 'real', mounting: 'mast',
      at: { anchor: 'station', trackId: 'e1', station: 20, side: 'right', offset: 3.1, direction: 'up' }, sight: { required: 400 },
      // Its screen and indicator are its Ks signal's, with the speed it signals for its route; the name plate its own.
      logicalSignals: [{
        system: 'ks', type: 'multiSection', function: 'entry',
        frames: [
          { kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Ks_2' }, { aspect: 'Zs_7' }] },
          { kind: 'indicator', aspects: [{ aspect: 'Zs_3', symbol: '6' }] },
        ],
        signallingSpeeds: [{ routeId: expect.any(String), speed: 60 }],
      }],
      frames: [{ kind: 'nameplate', aspects: [] }],
    })
    expect(byId(signals, 'sig-a')).not.toHaveProperty('function')
    expect(byId(signals, 'sig-a').lifespan).toBeUndefined()
    expect(byId(signals, 'sig-gz')).toMatchObject({ at: { trackId: 'e2', station: 40, direction: 'down' }, alsoAt: [{ trackId: 'e3', station: 40 }] })
    expect(byId(signals, 'sig-x')).toMatchObject({ kind: 'virtual', virtualFunction: 'trainTargetLine', lifespan: { to: expect.any(String) } })
  })

  it('reads train detection, the switch element, the route with its overlap, flank protection and PZB as balises', () => {
    const c = read().record.signalling.collections
    expect(c.trackSections).toEqual([{ id: 'fma-w1', name: 'W1', areaNumber: '60', kind: 'axleCounting', boundedBy: ['dp-1', 'dp-2', 'dp-3'] }])
    expect(c.switchElements).toEqual([expect.objectContaining({ id: 'we-1', switchId: 'k1', operation: 'electricRemote', preferredPosition: 'right', clearanceMarkerId: 'sig-gz' })])
    expect(c.routes).toEqual([expect.objectContaining({
      kind: 'train', startSignalId: 'sig-a', targetSignalId: 'sig-n1', path: { trackIds: ['e1', 'e2'] }, rank: 0,
      overlaps: [{ overlapId: 'dw-1', preferred: true }],
      aspects: [{ signalId: 'sig-a', aspect: 'Ks_1' }, { signalId: 'sig-a', aspect: 'Zs_3', symbol: '6' }],
    })])
    expect(c.overlaps).toEqual([expect.objectContaining({
      name: 'D1', signalId: 'sig-n1', path: { trackIds: ['e2'] }, end: { anchor: 'station', trackId: 'e2', station: 95 }, length: 15, gradient: -1.5, clearSections: ['fma-w1'],
    })])
    expect(c.flankProtection).toEqual([expect.objectContaining({ requester: { switchElementId: 'we-1', via: 'left' }, protection: { signalId: 'sig-n1' } })])
    expect(c.balises).toEqual([expect.objectContaining({ system: 'pzb', kind: 'magnet', settings: { frequency: '1000' }, signalId: 'sig-a', effect: 'switchedBySignal' })])
  })

  it('holds together as the check sees it, and reports what it left out', () => {
    const { record, report } = read()
    const doc = projectDocument(record)
    const { errors } = checkSignalling({ core: doc.domains.core, signalling: doc.domains.signalling })
    expect(errors).toEqual([])
    expect(report.skipped).toEqual({ BUE_Anlage: 1, Sonstiger_Punkt: 1 })
  })
})

describe('a plan\'s signalling on another net', () => {
  it('carries every location onto the track lying there, and orphans what has none', () => {
    const { record } = read()
    const P = (x, y = 0) => ({ easting: 4500000 + x, northing: 5600000 + y, zone: 5684 })
    // The project's own net: one track along e1 and e2, nothing where e3 is.
    const onto = { tracks: [{ id: 'main', epsg: 5684, elements: [straightElement(P(0), P(300))] }], switches: [] }
    const { signalling } = placeSignalling(record.signalling, record.tracks, onto)
    const c = signalling.collections
    expect(byId(c.signals, 'sig-n1').at).toMatchObject({ trackId: 'main', station: 280, direction: 'up' })
    expect(byId(c.routes, 'fs-1').path).toEqual({ trackIds: ['main'] })
    expect(byId(c.signals, 'sig-x').at).toMatchObject({ anchor: 'plane', epsg: 5684 })
  })
})

// The plan P-Hausen (ProSig) is not in the repository; where it is at hand,
// PLANPRO_SAMPLE names its content.xml and the whole plan is read and checked.
const sample = process.env.PLANPRO_SAMPLE
describe.skipIf(!sample || !existsSync(sample))('a real plan', () => {
  it('reads without an error of the record or of its signalling', () => {
    const { record } = planproRecord(readFileSync(sample, 'utf8'), { id: PROJECT })
    expect(validateProject(record).errors).toEqual([])
    const doc = projectDocument(record)
    expect(checkSignalling({ core: doc.domains.core, signalling: doc.domains.signalling }).errors).toEqual([])
  })
})
