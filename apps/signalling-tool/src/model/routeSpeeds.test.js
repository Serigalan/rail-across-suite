import { describe, it, expect } from 'vitest'
import { straightElement } from '@rail-across/core/geometry'
import { signallingSpeedOf } from '@rail-across/core/signalling'
import { added, emptyDomain } from './domain.js'
import {
  elementSpeed, routeElementSpeed, routeSpeedSections, speedSignalElementSpeed, speedSignalReach, speedSignalSpeed, speedSymbol, withElementSpeeds,
} from './routeSpeeds.js'
import { whereStretch } from './places.js'
import { speedRanges } from './speedRanges.js'

const EPSG = 25832
const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
// Each track of straights from x to x, each straight with its design speed.
const track = (id, pieces) => ({ id, epsg: EPSG, elements: pieces.map(([a, b, speed]) => straightElement(P(...a), P(...b), { speed })) })
// t1 runs 300 m east into the toe of w1 (60 km/h on its branch); t2 goes on straight, t3 branches north-east.
const net = {
  tracks: [
    track('t1', [[[0, 0], [100, 0], 120], [[100, 0], [200, 0], 80], [[200, 0], [300, 0], 100]]),
    track('t2', [[[300, 0], [500, 0], 100]]),
    track('t3', [[[300, 0], [480, 40], 70]]),
  ],
  switches: [{ switchId: 'w1', kind: 'turnout', formVersion: 1, speed: 60, ports: {
    A: { trackId: 't1', endpoint: 'END' }, B1: { trackId: 't3', endpoint: 'BEGIN' }, B2: { trackId: 't2', endpoint: 'BEGIN' },
  } }],
}
const on = (trackId, station, direction = 'up') => ({ anchor: 'station', trackId, station, direction })
const main = (id, at) => ({ id, kind: 'real', name: id, at, logicalSignals: [{ system: 'ks', type: 'main' }] })
const lf7 = (id, at, symbol) => ({ id, kind: 'real', function: 'standaloneIndicator', at, frames: [{ kind: 'plate', aspects: [{ aspect: 'Lf_7', ...(symbol ? { symbol } : {}) }] }] })
const route = (id, target, trackIds) => ({ id, kind: 'train', trainType: 'ZZ', startSignalId: 'a', targetSignalId: target, path: { trackIds } })

function domainOf(signals, routes = []) {
  let d = emptyDomain()
  for (const s of signals) d = added(d, 'signals', s)
  for (const r of routes) d = added(d, 'routes', r)
  return d
}

describe('the speeds the elements give', () => {
  it('is the lowest design speed over a stretch, and a turnout passed on its branch', () => {
    expect(elementSpeed(net, [{ trackId: 't1', lo: 20, hi: 90, direction: 'up' }])).toMatchObject({ speed: 120 })
    expect(elementSpeed(net, [{ trackId: 't1', lo: 20, hi: 150, direction: 'up' }])).toMatchObject({ speed: 80, where: { kind: 'track', trackId: 't1', from: 100, to: 150 } })
    expect(elementSpeed(net, [{ trackId: 't1', lo: 250, hi: 300, direction: 'up' }, { trackId: 't3', lo: 0, hi: 100, direction: 'up' }]))
      .toEqual({ speed: 60, where: { kind: 'switch', switchId: 'w1' } })
    expect(elementSpeed({ ...net, tracks: [track('t1', [[[0, 0], [100, 0]]])] }, [{ trackId: 't1', lo: 0, hi: 50, direction: 'up' }])).toBeNull()
  })

  it('gives a route the lowest from its start signal to its target', () => {
    const d = domainOf([main('a', on('t1', 20)), main('n', on('t2', 80)), main('m', on('t3', 100))])
    const signals = d.collections.signals
    expect(routeElementSpeed(net, route('r1', 'n', ['t1', 't2']), signals)?.speed).toBe(80)
    expect(routeElementSpeed(net, route('r2', 'm', ['t1', 't3']), signals)?.speed).toBe(60)
    // Started past the slow element, it is run faster.
    const late = domainOf([main('a', on('t1', 210)), main('n', on('t2', 80))])
    expect(routeElementSpeed(net, route('r1', 'n', ['t1', 't2']), late.collections.signals)?.speed).toBe(100)
  })

  it('cuts a route at the Lf 7 on it acting its way, each showing the lowest ahead of it, rounded down to tens', () => {
    const d = domainOf([main('a', on('t1', 20)), lf7('v', on('t1', 210)), lf7('back', on('t1', 150, 'down')), main('n', on('t2', 80)), main('m', on('t3', 100))])
    const signals = d.collections.signals
    const sections = routeSpeedSections(net, route('r1', 'n', ['t1', 't2']), signals)
    expect(sections.map(s => [s.signal.id, Math.round(s.from), Math.round(s.to), s.elements.speed])).toEqual([['a', 0, 190, 80], ['v', 190, 360, 100]])
    expect(routeElementSpeed(net, route('r1', 'n', ['t1', 't2']), signals)?.speed).toBe(80)
    // The board is the same for every route past it: the branch's turnout and track rule it.
    expect(speedSignalElementSpeed(net, signals, signals[1])?.speed).toBe(60)
    expect(speedSymbol(75)).toBe('7')
    expect(speedSymbol(5)).toBe('1')
    expect(speedSymbol(null)).toBeNull()
  })

  it('sets every train route and every Lf 7 as the elements give them', () => {
    const d = domainOf([main('a', on('t1', 20)), lf7('v', on('t1', 210), '12'), main('n', on('t2', 80)), main('m', on('t3', 100))],
      [route('r1', 'n', ['t1', 't2']), route('r2', 'm', ['t1', 't3']), { ...route('s', 'n', ['t1', 't2']), kind: 'shunting', trainType: undefined }])
    const { domain: next, routes, boards } = withElementSpeeds(d, net)
    expect([routes, boards]).toEqual([2, 1])
    expect(speedSignalSpeed(next.collections.signals[1])).toBe(60)
    expect(next.collections.routes.map(r => signallingSpeedOf(next, r))).toEqual([80, 80, null])
    // Once set, again changes nothing.
    expect(withElementSpeeds(next, net)).toMatchObject({ routes: 0, boards: 0 })
  })

  it('holds an Lf 7 up to the next Zs 3 or Lf 7 on every way, and shows the element that sets it', () => {
    const zs3 = { ...main('n', on('t2', 80)), logicalSignals: [{ system: 'ks', type: 'main', frames: [{ kind: 'indicator', aspects: [{ aspect: 'Zs_3', symbol: '8' }] }] }] }
    const d = domainOf([lf7('v', on('t1', 210)), zs3, main('m', on('t3', 100))])
    const reach = speedSignalReach(net, d.collections.signals, d.collections.signals[0])
    // Straight on to n's Zs 3; on the branch past m, which shows none, to the end of the track.
    expect(reach.ways.map(w => [w.target?.id ?? null, w.trackIds])).toEqual([['n', ['t1', 't2']], [null, ['t1', 't3']]])
    expect(reach.ways.map(w => w.elements.speed)).toEqual([100, 60])
    expect(reach.elements).toEqual({ speed: 60, where: { kind: 'switch', switchId: 'w1' } })
    // The turnout is shown by the start of its branch.
    expect(whereStretch(net, reach.elements.where)).toEqual({ trackId: 't3', from: 0, to: 20 })
    expect(whereStretch(net, { kind: 'track', trackId: 't1', from: 100, to: 200 })).toEqual({ trackId: 't1', from: 100, to: 200 })
  })

  it('joins where each signal signals a speed into a range, coloured apart from the one it leads to', () => {
    const d = domainOf([main('a', on('t1', 20)), lf7('v', on('t1', 210), '12'), main('n', on('t2', 80)), main('m', on('t3', 100))],
      [route('r1', 'n', ['t1', 't2']), route('r2', 'm', ['t1', 't3'])])
    const ranges = speedRanges(withElementSpeeds(d, net).domain, net)
    const round = (p) => [p.trackId, Math.round(p.from), Math.round(p.to)]
    expect(ranges.map(r => [r.id, r.board, r.routeIds, r.parts.map(round)])).toEqual([
      ['a|80', false, ['r1', 'r2'], [['t1', 20, 210]]],
      ['v|60', true, ['r1', 'r2'], [['t1', 210, 300], ['t2', 0, 80], ['t3', 0, 100]]],
    ])
    expect(ranges[0].color).not.toBe(ranges[1].color)
    // Without a speed signalled there is no range.
    expect(speedRanges(d, net)).toEqual([expect.objectContaining({ id: 'v|120' })])
  })
})
