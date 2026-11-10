import { describe, it, expect } from 'vitest'
import { writePlanPro } from './index.js'
import { straightElement } from '../geometry/index.js'
import { ksStation } from '../../test/signallingFixture.js'

// The Ks station of the signalling tests on a net of its own in UTM: t1 250 m
// east into the tip of w1, t2 going on straight, t3 branching off north-east.
const P = (e, n = 0) => ({ easting: 690000 + e, northing: 5710000 + n, zone: 25832 })
const project = () => ({
  id: 'p', title: 'Halle-Trotha',
  tracks: [
    { id: 't1', epsg: 25832, name: '1', elements: [straightElement(P(0), P(250))] },
    { id: 't2', epsg: 25832, name: '1', elements: [straightElement(P(250), P(400))] },
    { id: 't3', epsg: 25832, name: '3', elements: [straightElement(P(250), P(340, 20))] },
  ],
  switches: [{
    switchId: 'w1', kind: 'turnout', name: 'W1', label: '300 – 1:9',
    ports: { A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't3', endpoint: 'BEGIN' }, B2: { trackId: 't2', endpoint: 'BEGIN' } },
  }],
  endMarks: [{ id: 'e1', kind: 'buffer_stop', trackId: 't2', endpoint: 'END' }],
})

describe('a project as a PlanPro plan', () => {
  it('writes the net in DB_REF, the turnout with its element, and the signalling', () => {
    const { text, counts, report } = writePlanPro(project(), { signalling: ksStation(), now: '2026-11-09T12:00:00Z', version: '1' })
    expect(text.startsWith('<?xml version="1.0" encoding="UTF-8"?>')).toBe(true)
    expect(text).toContain('<GEO_Koordinatensystem><Wert>ER0</Wert></GEO_Koordinatensystem>')
    expect(text).toContain('<Erzeugung_Zeitstempel><Wert>2026-11-09T12:00:00Z</Wert></Erzeugung_Zeitstempel>')
    // Every object with its base, the interlocking and the geodata each a part of the plan.
    expect(text).toContain('<Basis_Objekt_Allg><Datum_Regelwerk><Wert>2026-11-09</Wert></Datum_Regelwerk></Basis_Objekt_Allg>')
    expect(text).toContain('<Untergewerk_Art><Wert>ESTW</Wert></Untergewerk_Art>')
    expect(text).toContain('<Untergewerk_Art><Wert>Geo</Wert></Untergewerk_Art>')
    expect(counts).toMatchObject({ TOP_Kante: 3, TOP_Knoten: 4, GEO_Kante: 3, W_Kr_Anlage: 1, Gleis_Abschluss: 1 })
    expect(counts.Signal).toBeGreaterThanOrEqual(2)
    expect(counts.Fstr_Zug_Rangier).toBeGreaterThanOrEqual(1)
    // What PlanPro has no place for here is named, not dropped silently.
    const why = new Set(report.notes.map(n => `${n.collection}:${n.why}`))
    expect(why).toContain('balises:system_not_written')
    expect(why).toContain('stoppingPlaces:kind_not_written')
  })

  it('packs the plan with its manifest as a .planpro file', () => {
    const { archive, manifest } = writePlanPro(project(), { signalling: ksStation(), now: '2026-11-09T12:00:00Z' })
    const bytes = Uint8Array.from(atob(archive), c => c.charCodeAt(0))
    expect([...bytes.slice(0, 4)]).toEqual([0x50, 0x4b, 0x03, 0x04])
    const names = new TextDecoder().decode(bytes)
    expect(names).toContain('content.xml')
    expect(names).toContain('manifest.xml')
    expect(manifest).toContain('<content name="content.xml" type="text/xml"/>')
  })

  it('takes planning data where it is given', () => {
    const { text } = writePlanPro(project(), {
      now: '2026-11-09T12:00:00Z',
      planning: { projectNumber: 'P-4711', planner: 'A. Planer', organisation: 'Büro', location: 'LHT', stage: 'Bauzustand 1' },
    })
    expect(text).toContain('<Projekt_Nummer><Wert>P-4711</Wert></Projekt_Nummer>')
    expect(text).toContain('<Name_Akteur><Wert>A. Planer</Wert></Name_Akteur>')
    expect(text).toContain('<Fuehrende_Oertlichkeit><Wert>LHT</Wert></Fuehrende_Oertlichkeit>')
    expect(text).toContain('<Bauzustand_Kurzbezeichnung><Wert>Bauzustand 1</Wert></Bauzustand_Kurzbezeichnung>')
  })

  it('refuses a project without a track in a plane', () => {
    expect(() => writePlanPro({ id: 'p', tracks: [] })).toThrow(/no track/)
  })
})
