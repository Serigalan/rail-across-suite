import { describe, expect, it } from 'vitest'
import {
  checkElectrification, coverage, domainOf, newWire, put, recordOf, statusOf, stretchesOf, wireAlong, withStatus, withStretch, withoutMast, withoutWire,
} from './electrification.js'
import { nearest, netLines, stretch } from './net.js'

// Two straight tracks in UTM 32, 1000 m and 500 m, side by side.
const straight = (id, y, length) => ({
  id, name: id, epsg: 25832,
  elements: [{ elementType: 'straight', length, bearing: 90, startNode: [500000, 5700000 + y], endNode: [500000 + length, 5700000 + y] }],
})
const tracks = [straight('t1', 0, 1000), straight('t2', 50, 500)]
const record = { id: 'p1', tracks, switches: [], platforms: [] }
const empty = () => domainOf(record)

describe('the overhead contact line of a net (electrification 3.0, decision 0006)', () => {
  it('is no layer of the record while it holds nothing', () => {
    expect(domainOf(record)).toEqual({ fields: {}, collections: { contactWires: [], masts: [], suspensionPoints: [], sectionInsulators: [] } })
    expect(recordOf(record, empty()).electrification).toBeUndefined()
    const wire = newWire({ voltage: '15000V' })
    const one = withStretch(empty(), wire, { trackId: 't1', from: 0, to: 600 })
    expect(recordOf(record, one).electrification).toBe(one)
    expect(recordOf(record, withoutWire(one, wire.id)).electrification).toBeUndefined()
  })

  it('lays a stretch out with masts no farther apart than the span, each holding the wire, and the wire runs between them', () => {
    const { masts, suspensionPoints } = wireAlong({ trackId: 't1', from: 600, to: 0, contactWireId: 'w', span: 60 })
    expect(masts).toHaveLength(11)
    expect(masts.map(m => m.at.station)).toEqual([0, 60, 120, 180, 240, 300, 360, 420, 480, 540, 600])
    expect(masts[0].at).toMatchObject({ anchor: 'station', trackId: 't1', side: 'right', offset: 3.2 })
    expect(suspensionPoints.every((p, i) => p.mastId === masts[i].id && p.contactWireId === 'w')).toBe(true)
    const wire = newWire({ voltage: '15000V' })
    const d = withStretch(empty(), wire, { trackId: 't1', from: 0, to: 590, span: 60 })
    expect(stretchesOf(d)).toEqual([{ contactWireId: wire.id, voltage: '15000V', trackId: 't1', startStation: 0, endStation: 590 }])
  })

  it('takes away with a wire what carries only it, with a mast its points', () => {
    const a = newWire({ voltage: '15000V' }), b = newWire({ voltage: '15000V' })
    let d = withStretch(withStretch(empty(), a, { trackId: 't1', from: 0, to: 100 }), b, { trackId: 't2', from: 0, to: 100 })
    d = put(d, 'sectionInsulators', { id: 'i', at: { anchor: 'station', trackId: 't1', station: 100 }, contactWireBeforeId: a.id, contactWireAfterId: b.id })
    const left = withoutWire(d, a.id)
    expect(left.collections.contactWires).toEqual([b])
    expect(left.collections.masts).toHaveLength(d.collections.masts.length - 3)
    expect(left.collections.sectionInsulators).toEqual([])
    const mast = d.collections.masts[0]
    expect(withoutMast(d, mast.id).collections.suspensionPoints.some(p => p.mastId === mast.id)).toBe(false)
  })

  it('finds what stands off its track, an insulator parting a wire from itself, a wire nothing carries, and two voltages over one piece of track', () => {
    const a = newWire({ voltage: '15000V' }), b = newWire({ voltage: '25000V' }), idle = newWire({ voltage: '15000V' })
    let d = withStretch(withStretch(empty(), a, { trackId: 't1', from: 0, to: 600 }), b, { trackId: 't1', from: 500, to: 1000 })
    d = withStretch(put(d, 'contactWires', idle), a, { trackId: 't2', from: 0, to: 700 })
    d = put(d, 'sectionInsulators', { id: 'i', at: { anchor: 'station', trackId: 't1', station: 550 }, contactWireBeforeId: a.id, contactWireAfterId: a.id })
    const { errors, warnings } = checkElectrification(d, tracks)
    expect(new Set(errors.map(f => f.code))).toEqual(new Set(['off_track', 'insulator_same_wire', 'voltages_meet']))
    expect(warnings.map(f => [f.code, f.id])).toEqual([['wire_unused', idle.id]])
  })

  it('says how much of the net is under wire, an overlap once', () => {
    const a = newWire({ voltage: '15000V' }), b = newWire({ voltage: '15000V' })
    const d = withStretch(withStretch(empty(), a, { trackId: 't1', from: 0, to: 600 }), b, { trackId: 't1', from: 400, to: 800 })
    const cov = coverage(d, tracks)
    expect(cov.tracks).toBeCloseTo(1500, 6)
    expect(cov.electrified).toBeCloseTo(800, 6)
    expect(cov.byVoltage['15000V']).toBeCloseTo(1000, 6)
  })

  it('keeps the planning status as a lifespan, the day open (decision 0007), the masts of a new wire new too', () => {
    const wire = withStatus(newWire({ voltage: '15000V' }), 'new')
    expect(statusOf(wire)).toBe('new')
    expect(wire.lifespan).toEqual({ from: 'open' })
    const d = withStretch(empty(), wire, { trackId: 't1', from: 0, to: 100 })
    expect(d.collections.masts.every(m => statusOf(m) === 'new')).toBe(true)
    expect(withStatus(wire, 'existing')).not.toHaveProperty('lifespan')
  })
})

describe('the net on the map', () => {
  it('draws each track from its stations and finds the station nearest a click', () => {
    const lines = netLines(tracks)
    expect([...lines.keys()]).toEqual(['t1', 't2'])
    const mid = stretch(lines, 't1', 250, 750)
    expect(mid).toHaveLength(2)
    const hit = nearest(lines, mid[0], 5)
    expect(hit.trackId).toBe('t1')
    expect(hit.station).toBeCloseTo(250, 1)
    expect(nearest(lines, [0, 0], 5)).toBeNull()
  })
})
