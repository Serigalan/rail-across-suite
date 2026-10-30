import { describe, it, expect } from 'vitest'
import { straightElement } from '@rail-across/core/geometry'
import { netDrawing, placeOf, locate, stretchLine } from './net.js'
import { findRun, runAhead, switchesOnRun } from './runs.js'
import { added, cascadeOf, deleted, domainOf, emptyDomain, newSignal, withKind } from './domain.js'
import { redo, startHistory, step, stepOf, stepsOf, travel, undo } from './history.js'

const EPSG = 25832
const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
const track = (id, a, b) => ({ id, epsg: EPSG, elements: [straightElement(P(...a), P(...b))] })
// t1 runs 200 m east into the toe of w1; t2 goes on straight, t3 branches north-east.
const net = {
  tracks: [track('t1', [0, 0], [200, 0]), track('t2', [200, 0], [400, 0]), track('t3', [200, 0], [380, 40])],
  switches: [{ switchId: 'w1', kind: 'turnout', formVersion: 1, ports: {
    A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't3', endpoint: 'BEGIN' }, B2: { trackId: 't2', endpoint: 'BEGIN' },
  } }],
}
const on = (trackId, station, direction = 'up') => ({ anchor: 'station', trackId, station, direction })

describe('the net as the plan draws it', () => {
  const drawing = netDrawing(net.tracks)
  it('lays every track flat in metres, east right and north up', () => {
    const t1 = drawing.tracks.get('t1')
    expect(t1.line[1][0] - t1.line[0][0]).toBeCloseTo(200, 0)
    expect(Math.abs(t1.line[1][1] - t1.line[0][1])).toBeLessThan(0.5)
    const p = placeOf(drawing, on('t1', 50, 'down'))
    expect(p.angle).toBeCloseTo(270, 3)
  })

  it('finds the location nearest a point, and which side it lies on', () => {
    const p = placeOf(drawing, on('t2', 120))
    const hit = locate(drawing, [p.xy[0], p.xy[1] - 3], 10)
    expect(hit).toMatchObject({ trackId: 't2', side: 'right' })
    expect(hit.station).toBeCloseTo(120, 1)
    expect(locate(drawing, [p.xy[0], p.xy[1] + 500], 10)).toBeNull()
    expect(stretchLine(drawing, 't1', 20, 80)).toHaveLength(2)
  })
})

describe('runs over the net', () => {
  it('finds the run from a signal to another, through the switch the way they act', () => {
    expect(findRun(net, on('t1', 20), on('t3', 100))).toEqual(['t1', 't3'])
    expect(findRun(net, on('t1', 20), on('t1', 150))).toEqual(['t1'])
    expect(findRun(net, on('t1', 20), on('t1', 10))).toBeNull()
    expect(findRun(net, on('t3', 100, 'down'), on('t1', 20, 'down'))).toEqual(['t3', 't1'])
    expect(switchesOnRun(net, ['t1', 't3'])).toEqual(['w1'])
  })

  it('runs an overlap ahead of its signal, straight on at a switch', () => {
    expect(runAhead(net, on('t1', 150), 100)).toEqual({ trackIds: ['t1', 't2'], end: { trackId: 't2', station: 50 } })
    expect(runAhead(net, on('t1', 150, 'down'), 100)).toEqual({ trackIds: ['t1'], end: { trackId: 't1', station: 50 } })
  })
})

describe('editing the domain', () => {
  it('makes a signal of a kind with what its screen must show, and changes its kind', () => {
    const s = newSignal({ at: on('t1', 20), type: 'mainShunting' })
    expect(s.frames[0].aspects.map(a => a.aspect)).toEqual(['Hp_0', 'Sh_1', 'Ks_1'])
    const d = withKind(s, 'ks', 'distant')
    expect(d.frames[0].aspects.map(a => a.aspect)).toEqual(['Ks_1', 'Ks_2'])
  })

  it('cascades a deletion within the domain', () => {
    let d = emptyDomain()
    d = added(d, 'signals', { id: 'a', kind: 'real', at: on('t1', 20) })
    d = added(d, 'signals', { id: 'n', kind: 'real', at: on('t2', 80) })
    d = added(d, 'routes', { id: 'r', kind: 'train', startSignalId: 'a', targetSignalId: 'n', path: { trackIds: ['t1', 't2'] }, aspects: [{ signalId: 'a', aspect: 'Ks_1' }] })
    d = added(d, 'pzbElements', { id: 'p', at: on('t1', 20), frequency: '1000', kind: 'magnet', effect: 'switchedBySignal', signalId: 'n' })
    expect(cascadeOf(d, 'signals', 'n')).toEqual({ signals: ['n'], routes: ['r'] })
    const out = deleted(d, 'signals', 'n')
    expect(out.collections.routes).toEqual([])
    expect(out.collections.pzbElements[0].signalId).toBeUndefined()
    expect(domainOf({ signalling: { fields: {}, collections: { signals: [] } } }).collections.routes).toEqual([])
  })

  it('keeps its steps for undo and redo', () => {
    const a = emptyDomain(), b = added(a, 'signals', { id: 'x' })
    let h = step(startHistory(a), b)
    expect(step(h, b)).toBe(h)
    h = undo(h)
    expect(h.now).toBe(a)
    expect(redo(h).now).toBe(b)
  })

  it('says what each step did and goes several at once', () => {
    let d = emptyDomain()
    d = added(d, 'signals', { id: 'a', kind: 'real', name: 'A', at: on('t1', 20) })
    d = added(d, 'signals', { id: 'n', kind: 'real', name: 'N1', at: on('t2', 80) })
    d = added(d, 'routes', { id: 'r', kind: 'train', startSignalId: 'a', targetSignalId: 'n', path: { trackIds: ['t1', 't2'] } })
    const states = [emptyDomain(), d, deleted(d, 'signals', 'n')]
    let h = states.slice(1).reduce(step, startHistory(states[0]))
    expect(stepOf(states[0], d)).toEqual({ kind: 'many', n: 3 })
    expect(stepOf(d, states[2])).toMatchObject({ kind: 'removed', collection: 'signals', obj: { id: 'n' }, more: 1 })
    const renamed = { ...d, collections: { ...d.collections, signals: [{ ...d.collections.signals[0], name: 'B' }, d.collections.signals[1]] } }
    expect(stepOf(d, renamed)).toMatchObject({ kind: 'changed', collection: 'signals', obj: { name: 'B' } })
    expect(stepOf(d, d)).toEqual({ kind: 'fields', n: 0 })

    h = travel(h, -2)
    expect(h.now).toBe(states[0])
    expect(stepsOf(h)).toMatchObject({ past: [], future: [{ kind: 'many' }, { kind: 'removed' }] })
    h = travel(h, 1)
    expect(h.now).toBe(d)
    expect(travel(h, 5).now).toBe(states[2])
  })
})
