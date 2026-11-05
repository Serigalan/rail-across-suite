import { describe, it, expect } from 'vitest'
import { straightElement } from '@rail-across/core/geometry'
import { checkSignalling } from '@rail-across/core/signalling'
import { added, emptyDomain, shownSpeed, signalsSpeed, withRouteSpeed, withSpeedsShown } from './domain.js'
import { clearanceMarkersDue, isClearanceMarker, withClearanceMarkers } from './clearanceMarkers.js'
import { detectionLayout } from './detectionLayout.js'

const EPSG = 25832
const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
const track = (id, a, b) => ({ id, epsg: EPSG, elements: [straightElement(P(...a), P(...b))] })
const turnout = (switchId, A, B1, B2) => ({ switchId, kind: 'turnout', formVersion: 1, ports: { A, B1, B2 } })
const on = (trackId, station, direction = 'up') => ({ anchor: 'station', trackId, station, direction })

// t1 runs 200 m east into the toe of w1; t2 goes on straight, t3 branches north-east.
const net = {
  tracks: [track('t1', [0, 0], [200, 0]), track('t2', [200, 0], [400, 0]), track('t3', [200, 0], [380, 40])],
  switches: [turnout('w1', { trackId: 't1', endpoint: 'END' }, { trackId: 't3', endpoint: 'BEGIN' }, { trackId: 't2', endpoint: 'BEGIN' })],
}

// A crossover: a and b 4.5 m apart, c leading over from w1 on a to w2 on b, whose toe faces w1.
const crossover = {
  tracks: [
    track('a1', [0, 0], [100, 0]), track('a2', [100, 0], [400, 0]), track('c', [100, 0], [160, 4.5]),
    track('b1', [0, 4.5], [160, 4.5]), track('b2', [160, 4.5], [400, 4.5]),
  ],
  switches: [
    turnout('w1', { trackId: 'a1', endpoint: 'END' }, { trackId: 'c', endpoint: 'BEGIN' }, { trackId: 'a2', endpoint: 'BEGIN' }),
    turnout('w2', { trackId: 'b2', endpoint: 'BEGIN' }, { trackId: 'c', endpoint: 'END' }, { trackId: 'b1', endpoint: 'END' }),
  ],
}

describe('the clearance markers', () => {
  it('sets one where the branches of a switch lie 3.5 m apart, half way between them, facing the switch', () => {
    const [due, ...more] = clearanceMarkersDue(net, [])
    expect(more).toEqual([])
    expect(due.switchId).toBe('w1')
    expect(due.at).toMatchObject({ trackId: 't3', side: 'right', offset: 1.75, direction: 'down' })
    expect(due.at.station).toBeCloseTo(16, 0)

    const { domain, added: n } = withClearanceMarkers(emptyDomain(), net)
    expect(n).toBe(1)
    const marker = domain.collections.signals[0]
    expect(isClearanceMarker(marker)).toBe(true)
    // It is a body Ra 12 the check takes, and set once.
    const findings = checkSignalling({ core: { collections: net }, signalling: domain })
    expect(findings.errors.filter(f => f.id === marker.id)).toEqual([])
    expect(withClearanceMarkers(domain, net).added).toBe(0)
  })

  it('makes the points of two switches whose sections would overlap one, in the middle', () => {
    expect(clearanceMarkersDue(crossover, [])).toHaveLength(2)
    const laid = detectionLayout(crossover, emptyDomain(), { t: (k) => k })
    // On c, w1's point lies some 47 m from it and w2's some 47 m from w2: past one another, 34 m apart.
    const onC = laid.points.filter(p => p.at.trackId === 'c')
    expect(onC).toHaveLength(1)
    expect(onC[0].at.station).toBeCloseTo(30, 0)
  })
})

describe('the speed a route signals', () => {
  const main = (id, at) => ({ id, kind: 'real', name: id, at, logicalSignals: [{ system: 'ks', type: 'main' }] })
  const route = (id, target) => ({ id, kind: 'train', startSignalId: 'a', targetSignalId: target, path: { trackIds: ['t1', 't2'] } })
  let d = emptyDomain()
  for (const s of [main('a', on('t1', 20)), main('n', on('t2', 80)), main('m', on('t3', 100))]) d = added(d, 'signals', s)
  d = added(added(d, 'routes', route('r1', 'n')), 'routes', route('r2', 'm'))
  const zs3 = (dom) => dom.collections.signals[0].logicalSignals[0].frames?.filter(f => f.kind === 'indicator').flatMap(f => f.aspects.map(a => a.symbol)) ?? []

  it('is shown in tens by a Zs 3 at its start signal', () => {
    const one = withRouteSpeed(d, d.collections.routes[0], 75)
    expect(shownSpeed(75)).toBe(70)
    expect(one.collections.signals[0].logicalSignals[0].signallingSpeeds).toEqual([{ routeId: 'r1', speed: 70 }])
    expect(zs3(one)).toEqual(['7'])
    expect(one.collections.routes[0].aspects).toEqual([{ signalId: 'a', aspect: 'Zs_3', symbol: '7' }])
    expect(signalsSpeed(one, one.collections.routes[0], 70)).toBe(true)
    expect(signalsSpeed(d, d.collections.routes[0], 70)).toBe(false)
  })

  it('takes the figure away where no other route shows it, and keeps it where one does', () => {
    let two = withRouteSpeed(d, d.collections.routes[0], 70)
    two = withRouteSpeed(two, two.collections.routes[1], 70)
    expect(zs3(two)).toEqual(['7'])
    two = withRouteSpeed(two, two.collections.routes[0], 100)
    expect(zs3(two)).toEqual(['7', '10'])
    two = withRouteSpeed(two, two.collections.routes[1], null)
    expect(zs3(two)).toEqual(['10'])
    expect(two.collections.routes[1].aspects).toBeUndefined()
  })

  it('gives a speed signalled before without its Zs 3 one, in tens', () => {
    const before = { ...d, collections: { ...d.collections, signals: d.collections.signals.map((s, i) => (i ? s : { ...s, logicalSignals: [{ ...s.logicalSignals[0], signallingSpeeds: [{ routeId: 'r1', speed: 65 }] }] })) } }
    const { domain: shown, changed } = withSpeedsShown(before)
    expect(changed).toBe(1)
    expect(zs3(shown)).toEqual(['6'])
    expect(shown.collections.signals[0].logicalSignals[0].signallingSpeeds).toEqual([{ routeId: 'r1', speed: 60 }])
    expect(withSpeedsShown(shown).changed).toBe(0)
  })
})
