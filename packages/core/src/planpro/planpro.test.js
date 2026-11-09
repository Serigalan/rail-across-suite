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
    expect(counts).toMatchObject({ TOP_Kante: 3, TOP_Knoten: 4, GEO_Kante: 3, W_Kr_Anlage: 1, Gleis_Abschluss: 1 })
    expect(counts.Signal).toBeGreaterThanOrEqual(2)
    expect(counts.Fstr_Zug_Rangier).toBeGreaterThanOrEqual(1)
    // What PlanPro has no place for here is named, not dropped silently.
    const why = new Set(report.notes.map(n => `${n.collection}:${n.why}`))
    expect(why).toContain('balises:system_not_written')
    expect(why).toContain('stoppingPlaces:kind_not_written')
  })

  it('refuses a project without a track in a plane', () => {
    expect(() => writePlanPro({ tracks: [] })).toThrow(/no track/)
  })
})
