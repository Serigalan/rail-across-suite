import { describe, it, expect } from 'vitest'
import { straightElement, utmToWgs84 } from '@rail-across/core/geometry'
import { netDrawing, placeOf, locate, stretchLine } from './net.js'
import { findRun, runAhead, switchesOnRun } from './runs.js'
import {
  added, allLogical, baliseSettings, cascadeOf, deleted, domainOf, emptyDomain, findLogical, logicalId, newBalise, newLogical, newSignal, withKind,
  withLogical, withSignallingSpeed, withSwitchSpeeds,
} from './domain.js'
import { measuredOverlap, withMeasuredOverlap } from './overlaps.js'
import { OFFSET, foundationPoint, kmName, kmNameAt, locationFrom, mountedFrom, planePoint, signalOptions, trackName } from './places.js'
import { magnetFrequency, magnetName, magnetsDue, signalMagnet, withSignal } from './magnets.js'
import { detectionLayout, wantedPoints } from './detectionLayout.js'
import { nextSignals, routeSections, routeStretches, runStretches, wayPieces } from './routeSearch.js'
import { trackSectionExtents } from '@rail-across/core/signalling'
import { changedIds, redo, startHistory, step, stepOf, stepsOf, travel, undo } from './history.js'

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

describe('setting objects beside the track', () => {
  it('stands a signal on the right of the track it acts for, two metres off', () => {
    expect(locationFrom({ trackId: 't1', station: 50, side: 'right' }, { offset: OFFSET.signals }))
      .toEqual({ anchor: 'station', trackId: 't1', station: 50, side: 'right', offset: 2, direction: 'up' })
    expect(locationFrom({ trackId: 't1', station: 50, side: 'left' }, { offset: OFFSET.signals }).direction).toBe('down')
    expect(locationFrom({ trackId: 't1', station: 50, side: 'left' }, { direction: 'up' }).direction).toBe('up')
    expect(locationFrom({ trackId: 't1', station: 50, side: 'left' }, { directed: false })).not.toHaveProperty('direction')
  })

  it('gives the point in the plane its location stands at, square to the track', () => {
    // t1 runs east from (0, 0): right of it is south.
    const p = planePoint(net, { anchor: 'station', trackId: 't1', station: 50, side: 'right', offset: 2 })
    expect(p.easting).toBeCloseTo(500050, 6)
    expect(p.northing).toBeCloseTo(5699998, 6)
    expect(p.epsg).toBe(EPSG)
    expect(planePoint(net, { anchor: 'station', trackId: 't1', station: 50, side: 'left', offset: 2 }).northing).toBeCloseTo(5700002, 6)
    expect(planePoint(net, { anchor: 'station', trackId: 'gone', station: 1 })).toBeNull()
  })

  it('sets a gantry\'s screen and foundation where they were asked, sides seen the way it acts', () => {
    const asked = { screen: { side: null, offset: 0 }, foundation: { side: 'right', offset: 3.8 } }
    // Hit on the right, it acts up: the screen over the track, the foundation on the right of the stationing.
    expect(mountedFrom({ trackId: 't1', station: 50, side: 'right' }, asked))
      .toEqual({ at: { anchor: 'station', trackId: 't1', station: 50, direction: 'up' }, foundation: { side: 'right', offset: 3.8 } })
    // Hit on the left, it acts down: a screen on its right stands on the left of the stationing.
    const down = mountedFrom({ trackId: 't1', station: 50, side: 'left' }, { ...asked, screen: { side: 'right', offset: 1.5 } })
    expect(down.at).toEqual({ anchor: 'station', trackId: 't1', station: 50, direction: 'down', side: 'left', offset: 1.5 })
    // Its foundation on its right, seen the way it acts, is north of t1 — left of the stationing.
    const p = foundationPoint(net, { at: down.at, foundation: down.foundation })
    expect(p.northing).toBeCloseTo(5700003.8, 6)
    expect(foundationPoint(net, { at: down.at })).toBeNull()
  })
})

describe('names', () => {
  const named = { tracks: [{ ...track('t9', [0, 0], [10, 0]), name: 'koennern.1', lineNumber: '6344', trackNumber: '1' }, track('t8', [0, 0], [10, 0])] }
  it('names a track as the Layout-Tool does, else by its line and number', () => {
    expect(trackName(named, 't9')).toBe('koennern.1')
    expect(trackName({ tracks: [{ ...named.tracks[0], name: undefined }] }, 't9')).toBe('6344 / 1')
  })

  it('leads a signal\'s name with its area\'s number, and offers it with its track', () => {
    const t = (k) => k
    const signals = [
      { id: 'b', name: 'B', areaNumber: '21', kind: 'real', at: on('t9', 5) },
      { id: 'a', name: 'A', kind: 'real', at: { anchor: 'plane', easting: 0, northing: 0, epsg: EPSG } },
    ]
    expect(signalOptions(named, signals, t)).toEqual([{ value: 'b', label: '21-B · koennern.1' }, { value: 'a', label: 'A' }])
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

describe('the speed of a route over a turnout\'s branch', () => {
  const switches = [{ ...net.switches[0], speed: 60 }]
  const route = (id, trackIds, kind = 'train') => ({ id, kind, startSignalId: 'a', targetSignalId: 'b', path: { trackIds } })
  const domain = (speeds) => ({
    ...emptyDomain(),
    collections: {
      ...emptyDomain().collections,
      signals: [{ id: 'a', kind: 'real', at: on('t1', 20), logicalSignals: [{ system: 'ks', type: 'multiSection', signallingSpeeds: speeds }] },
        { id: 'b', kind: 'real', at: on('t3', 150) }],
      routes: [route('branch', ['t1', 't3']), route('straight', ['t1', 't2']), route('shunt', ['t1', 't3'], 'shunting')],
    },
  })
  const signalled = (d) => d.collections.signals[0].logicalSignals[0].signallingSpeeds

  it('is the turnout\'s where the route signals none or a higher one', () => {
    const { domain: d, changed } = withSwitchSpeeds(domain(undefined), switches)
    expect(changed).toEqual(['branch'])
    expect(signalled(d)).toEqual([{ routeId: 'branch', speed: 60 }])
    expect(signalled(withSwitchSpeeds(domain([{ routeId: 'branch', speed: 100 }]), switches).domain)).toEqual([{ routeId: 'branch', speed: 60 }])
  })

  it('keeps a lower one, and leaves the routes it is not asked for', () => {
    expect(withSwitchSpeeds(domain([{ routeId: 'branch', speed: 40 }]), switches).changed).toEqual([])
    expect(withSwitchSpeeds(domain(undefined), switches, ['straight']).changed).toEqual([])
  })
})

describe('editing the domain', () => {
  it('makes a place with a logical signal showing what its kind must, and changes its kind', () => {
    const s = newSignal({ at: on('t1', 20), type: 'mainShunting', fn: 'exit' })
    expect(s.frames).toBeUndefined()
    expect(s.logicalSignals[0]).toMatchObject({ system: 'ks', type: 'mainShunting', function: 'exit' })
    expect(s.logicalSignals[0].frames[0].aspects.map(a => a.aspect)).toEqual(['Hp_0', 'Sh_1', 'Ks_1'])
    expect(newSignal({ at: on('t1', 20), bare: true }).logicalSignals).toBeUndefined()
    const d = withKind({ ...s.logicalSignals[0], frames: [...s.logicalSignals[0].frames, { kind: 'indicator', aspects: [{ aspect: 'Zs_1' }] }] }, 'ks', 'distant')
    expect(d.frames[0].aspects.map(a => a.aspect)).toEqual(['Ks_1', 'Ks_2'])
    expect(d.frames).toHaveLength(1)   // a distant signal carries no Zs 1
    expect(newLogical({ system: 'sh', type: 'shunting' }).frames[0].aspects.map(a => a.aspect)).toEqual(['Sh_0', 'Sh_1'])
  })

  it('names a logical signal by its place and system, and edits it there', () => {
    let d = emptyDomain()
    d = added(d, 'signals', newSignal({ at: on('t1', 20) }))
    const sid = d.collections.signals[0].id
    const id = logicalId(sid, 'ks')
    expect(findLogical(d, id).logical.type).toBe('multiSection')
    d = withLogical(d, id, withSignallingSpeed(findLogical(d, id).logical, 'r1', 60))
    expect(findLogical(d, id).logical.signallingSpeeds).toEqual([{ routeId: 'r1', speed: 60 }])
    expect(allLogical(d)).toHaveLength(1)
    d = deleted(d, 'logicalSignals', id)
    expect(d.collections.signals[0].logicalSignals).toBeUndefined()
    expect(findLogical(d, id)).toEqual({})
  })

  it('measures an overlap from the speed its route\'s start signal signals, rising with a fall', () => {
    const tracks = net.tracks.map(tr => ({ ...tr, heights: [{ station: 0, z: 100 }, { station: 200, z: 100 - (tr.id === 't2' ? 6 : 0) }] }))
    const hilly = { ...net, tracks }
    const a = { id: 'a', kind: 'real', at: on('t1', 150), logicalSignals: [{ system: 'ks', type: 'main', signallingSpeeds: [{ routeId: 'r', speed: 50 }] }] }
    expect(measuredOverlap(net, a, 50)).toMatchObject({ length: 100, path: { trackIds: ['t1', 't2'] }, end: { trackId: 't2', station: 50 }, speed: 50 })
    expect(measuredOverlap(hilly, a, 50).length).toBeGreaterThan(100)
    let d = emptyDomain()
    d = added(d, 'signals', { ...a, at: on('t1', 20) })
    d = added(d, 'signals', a)
    d.collections.signals[0] = { ...d.collections.signals[0], id: 's' }
    d.collections.signals[0].logicalSignals = [{ system: 'ks', type: 'main', signallingSpeeds: [{ routeId: 'r', speed: 50 }] }]
    d.collections.signals[1].logicalSignals = [{ system: 'ks', type: 'main' }]
    d = added(d, 'routes', { id: 'r', kind: 'train', startSignalId: 's', targetSignalId: 'a', path: { trackIds: ['t1'] } })
    const { domain: next } = withMeasuredOverlap(d, net, d.collections.routes[0])
    expect(next.collections.overlaps).toHaveLength(1)
    expect(next.collections.overlaps[0]).toMatchObject({ signalId: 'a', length: 100, speed: 50 })
    expect(next.collections.routes[0].overlaps).toEqual([{ overlapId: next.collections.overlaps[0].id, preferred: true }])
    expect(withMeasuredOverlap(d, net, { ...d.collections.routes[0], id: 'other' }).problem).toBe('speed')
  })

  it('makes a balise with the settings its kind asks for, and keeps what a new kind allows', () => {
    const m = newBalise({ at: on('t1', 20) })
    expect(m).toMatchObject({ system: 'pzb', kind: 'magnet', settings: { frequency: '1000' }, effect: 'switchedBySignal' })
    expect(baliseSettings('pzb', 'magnet', { frequency: '2000', testSpeed: 40, ina: true })).toEqual({ frequency: '2000', ina: true })
    expect(newBalise({ at: on('t1', 20), system: 'etcs' })).toMatchObject({ system: 'etcs', kind: 'fixed', effect: 'permanent' })
  })

  it('cascades a deletion within the domain', () => {
    let d = emptyDomain()
    d = added(d, 'signals', { id: 'a', kind: 'real', at: on('t1', 20) })
    d = added(d, 'signals', { id: 'n', kind: 'real', at: on('t2', 80) })
    d = added(d, 'routes', { id: 'r', kind: 'train', startSignalId: 'a', targetSignalId: 'n', path: { trackIds: ['t1', 't2'] }, aspects: [{ signalId: 'a', aspect: 'Ks_1' }] })
    d = added(d, 'balises', { id: 'p', at: on('t1', 20), system: 'pzb', kind: 'magnet', settings: { frequency: '1000' }, effect: 'switchedBySignal', signalId: 'n' })
    expect(cascadeOf(d, 'signals', 'n')).toEqual({ signals: ['n'], routes: ['r'] })
    const out = deleted(d, 'signals', 'n')
    expect(out.collections.routes).toEqual([])
    expect(out.collections.balises[0].signalId).toBeUndefined()
    expect(domainOf({ signalling: { fields: {}, collections: { signals: [] } } }).collections.routes).toEqual([])
  })

  it('names a place by its kilometrage, the next metre free where that one is taken', () => {
    expect(kmName(6344, 5231)).toBe('6344-5+231')
    expect(kmName(6344, 5031.4)).toBe('6344-5+031')
    expect(kmName(6344, -1687)).toBe('6344--1+687')
    // Line 6344 along t1, km 5.000 at its begin, rising eastwards.
    const lonLat = (x) => utmToWgs84(500000 + x, 5700000, EPSG)
    const kmLines = [{ id: 'km-6344', lineNumber: '6344', runs: [[[...lonLat(-50), 4950], [...lonLat(250), 5250]]] }]
    expect(kmNameAt(net, kmLines, on('t1', 31))).toBe('6344-5+031')
    expect(kmNameAt(net, kmLines, on('t1', 31), new Set(['6344-5+031', '6344-5+032']))).toBe('6344-5+033')
    expect(kmNameAt(net, [], on('t1', 31))).toBeNull()
  })

  it('equips each signal with the magnet its kind asks for, at its place, once', () => {
    const ks = (id, type, at = on('t1', 50)) => ({ id, kind: 'real', at, logicalSignals: [{ system: 'ks', type }] })
    expect(magnetFrequency(ks('a', 'main'))).toBe('2000')
    expect(magnetFrequency(ks('a', 'distant'))).toBe('1000')
    expect(magnetFrequency(ks('a', 'multiSection'))).toBe('1000+2000')
    expect(magnetFrequency({ ...ks('a', 'main'), logicalSignals: [{ type: 'main' }, { type: 'distant' }] })).toBe('1000+2000')
    expect(magnetFrequency(ks('a', 'shunting'))).toBeNull()
    expect(magnetFrequency({ id: 'b', kind: 'real', at: on('t1', 5), frames: [{ kind: 'plate', aspects: [{ aspect: 'Ne_1' }] }] })).toBeNull()
    const down = ks('n', 'main', { ...on('t2', 80, 'down'), side: 'left', offset: 2 })
    expect(signalMagnet(down)).toMatchObject({
      system: 'pzb', kind: 'magnet', effect: 'switchedBySignal', signalId: 'n', settings: { frequency: '2000' },
      at: { anchor: 'station', trackId: 't2', station: 80, side: 'left', offset: OFFSET.balises, direction: 'down' },
    })
    let d = emptyDomain()
    for (const s of [ks('a', 'main'), down, ks('v', 'distant'), ks('r', 'shunting')]) d = added(d, 'signals', s)
    d = added(d, 'balises', signalMagnet(d.collections.signals[0]))
    const { due, equipped } = magnetsDue(d)
    expect(equipped).toBe(1)
    expect(due.map(x => [x.signal.id, x.frequency])).toEqual([['n', '2000'], ['v', '1000']])
  })

  it('names a magnet after its signal, and anew when it is given another', () => {
    const a = { id: 'a', kind: 'real', name: 'A', areaNumber: '21', at: on('t1', 50), logicalSignals: [{ system: 'ks', type: 'main' }] }
    const n = { id: 'n', kind: 'real', name: 'N 1', at: on('t2', 80), logicalSignals: [{ system: 'ks', type: 'main' }] }
    expect(magnetName(a)).toBe('M21A')
    expect(magnetName(n)).toBe('MN1')
    expect(magnetName({ id: 'x', kind: 'real' })).toBeUndefined()
    const m = signalMagnet(a)
    expect(m.name).toBe('M21A')
    expect(withSignal(m, n, a)).toMatchObject({ signalId: 'n', name: 'MN1' })
    expect(withSignal({ ...m, name: 'Eigener' }, n, a)).toMatchObject({ signalId: 'n', name: 'Eigener' })
    expect(withSignal(m, undefined, a)).not.toHaveProperty('name')
    expect(withSignal(m, undefined, a)).not.toHaveProperty('signalId')
    expect(withSignal({ ...m, system: 'etcs', name: undefined }, n, a).name).toBeUndefined()
  })

  it('lays out detection at the signals, ahead of a switch and past its clearance point, near ones made one', () => {
    const main = (id, at) => ({ id, kind: 'real', name: id, at, logicalSignals: [{ system: 'ks', type: 'main' }] })
    const distant = { id: 'v', kind: 'real', at: on('t1', 60), logicalSignals: [{ system: 'ks', type: 'distant' }] }
    const wanted = wantedPoints(net, [main('a', on('t1', 150)), distant])
    const at = (why) => wanted.filter(w => w.why === why).map(w => [w.trackId, Math.round(w.station)])
    expect(at('signal')).toEqual([['t1', 150]])
    expect(at('switch')).toEqual([['t1', 190]])
    // t3 leaves at about 12.5°: 3.5 m apart some 16 m on, the point 10 m past that.
    expect(at('clearance').sort()).toEqual([['t2', 26], ['t3', 26]])

    let d = emptyDomain()
    d = added(d, 'signals', main('a', on('t1', 175)))
    const laid = detectionLayout(net, d, { t: (k) => k })
    expect(laid.merged).toBe(1)
    expect(laid.points.map(p => [p.at.trackId, Math.round(p.at.station * 10) / 10]).sort()).toEqual([['t1', 182.5], ['t2', 26], ['t3', 26]])
    expect(laid.sections).toHaveLength(4)
    const atSwitch = laid.sections.find(x => x.boundedBy.length === 3)
    expect(atSwitch.name).toBe('W')
    expect(laid.sections.filter(x => x.name.startsWith('t')).map(x => x.name).sort()).toEqual(['t1', 't2', 't3'])

    // Again over what it laid: a point there takes the new ones near it, a section with the same bounds stays.
    const again = { ...d, collections: { ...d.collections, detectionPoints: laid.points, trackSections: laid.sections } }
    const second = detectionLayout(net, again, { t: (k) => k })
    expect(second.points).toEqual([])
    expect(second.sections).toEqual([])
    expect(second.kept).toHaveLength(4)
    expect(second.removed).toEqual([])
  })

  it('finds the routes from a signal to the next main signals on every way, and the sections they run through', () => {
    const main = (id, at, type = 'main') => ({ id, kind: 'real', name: id, at, logicalSignals: [{ system: 'ks', type }] })
    let d = emptyDomain()
    for (const s of [main('a', on('t1', 20)), main('sh', on('t1', 100), 'shunting'), main('n', on('t2', 80)), main('m', on('t3', 100)), main('back', on('t2', 150, 'down'))]) d = added(d, 'signals', s)
    const found = nextSignals(net, d.collections.signals, d.collections.signals[0])
    expect(found.map(f => [f.target.id, f.trackIds, Math.round(f.length)])).toEqual([['n', ['t1', 't2'], 260], ['m', ['t1', 't3'], 280]])
    expect(nextSignals(net, d.collections.signals, d.collections.signals[2])).toEqual([])

    const route = { startSignalId: 'a', targetSignalId: 'n', path: { trackIds: ['t1', 't2'] } }
    expect(routeStretches(net, route, d.collections.signals)).toEqual([{ trackId: 't1', lo: 20, hi: 200, direction: 'up' }, { trackId: 't2', lo: 0, hi: 80, direction: 'up' }])
    // Drawn from signal to signal, each piece the way it is run: the other way too.
    expect(wayPieces(routeStretches(net, route, d.collections.signals))).toEqual([{ trackId: 't1', from: 20, to: 200 }, { trackId: 't2', from: 0, to: 80 }])
    expect(wayPieces(runStretches(net, on('t2', 80, 'down'), ['t2', 't1'], on('t1', 20, 'down')))).toEqual([{ trackId: 't2', from: 80, to: 0 }, { trackId: 't1', from: 200, to: 20 }])
    const laid = detectionLayout(net, d, { t: (k) => k })
    d = { ...d, collections: { ...d.collections, detectionPoints: laid.points, trackSections: laid.sections } }
    const extents = trackSectionExtents({ core: { collections: net }, signalling: d })
    // The shunting signal on t1 has a point too: the route runs through two sections of t1, the switch's and one of t2.
    expect(routeSections(net, route, d, extents).map(x => x.name)).toEqual(['t1-2', 't1-3', 'W', 't2-1'])
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

  it('says what a draft holds new or changed, for the plan to draw as a preview', () => {
    let d = emptyDomain()
    d = added(d, 'signals', { id: 'a', kind: 'real', name: 'A', at: on('t1', 20) })
    d = added(d, 'signals', { id: 'n', kind: 'real', name: 'N1', at: on('t2', 80) })
    const draft = added({ ...d, collections: { ...d.collections, signals: [{ ...d.collections.signals[0], name: 'B' }, d.collections.signals[1]] } },
      'balises', { id: 'p', at: on('t1', 20), system: 'pzb', kind: 'magnet' })
    expect([...changedIds(d, draft)]).toEqual(['a', 'p'])
    expect(changedIds(d, deleted(d, 'signals', 'n')).size).toBe(0)
  })
})
