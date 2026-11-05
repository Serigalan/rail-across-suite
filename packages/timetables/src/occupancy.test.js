import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { net, WASM } from '../../running-times/test/net.js'
import { newPoint, newTrain, parseClock, runTrain } from './timetable.js'
import { blockingTimes, blocksAlong, byPriority, conflictsOf, DISPATCH, dispatchOf, occupancy } from './occupancy.js'

// The test net with main signals the way A-Dorf → C-Stadt runs: A-Dorf's exit, a block signal on the line, C-Stadt's entry.
const ks = [{ system: 'ks', type: 'multiSection', function: 'exit', frames: [] }]
const signal = (id, trackId, station) => ({ id, name: id, kind: 'real', at: { anchor: 'station', trackId, station, side: 'right', direction: 'up' }, logicalSignals: ks })
const signalled = {
  ...net,
  signalling: { fields: {}, collections: { signals: [signal('N1', 'A1', 450), signal('B', 'L', 1000), signal('A', 'L', 2400)], overlaps: [{ id: 'd', signalId: 'B', length: 100 }] } },
}
const stations = stationsOf(signalled)
const train = (number, clock, patch = {}) => newTrain({ number, vehicle: 'v', departure: parseClock(clock), points: [newPoint('A-Dorf'), newPoint('C-Stadt')], ...patch })
const vehicle = newVehicle('Triebzug')

describe('the blocks of a way', () => {
  it('runs from main signal to main signal acting its way, the overlap beyond with it', () => {
    const path = [{ trackId: 'A1', start: 200, end: 500 }, { trackId: 'L', start: 0, end: 2500 }, { trackId: 'C1', start: 0, end: 400 }]
    const blocks = blocksAlong(signalled, path)
    expect(blocks.map(b => [b.signalId, b.targetId, b.from, b.to, b.reach])).toEqual([[null, 'N1', 0, 250, 250], ['N1', 'B', 250, 1300, 1400], ['B', 'A', 1300, 2700, 2700], ['A', null, 2700, 3200, 3200]])
    expect(blocks[1].stretches).toEqual([{ trackId: 'A1', lo: 450, hi: 500 }, { trackId: 'L', lo: 0, hi: 1100 }])
    // The other way none of them acts: the whole way is one block.
    expect(blocksAlong(signalled, [{ trackId: 'L', start: 2500, end: 0 }]).map(b => [b.from, b.to])).toEqual([[0, 2500]])
  })

  it('holds its track sections each until the train has left it, those of the overlap with the block', () => {
    const point = (id, station) => ({ id, kind: 'axleCounter', at: { anchor: 'station', trackId: 'L', station } })
    const sectioned = {
      ...signalled,
      signalling: {
        ...signalled.signalling,
        collections: { ...signalled.signalling.collections, detectionPoints: [point('p1', 300), point('p2', 700)], trackSections: [{ id: 'S1', kind: 'axleCounting', boundedBy: ['p1', 'p2'] }] },
      },
    }
    const path = [{ trackId: 'A1', start: 200, end: 500 }, { trackId: 'L', start: 0, end: 2500 }, { trackId: 'C1', start: 0, end: 400 }]
    const blocks = blocksAlong(sectioned, path)
    // L 300…700 lies 600…1000 along the way, in the block from N1 to B.
    expect(blocks[1].sections).toEqual([{ id: 'S1', from: 600, to: 1000 }])
    expect(blocks[1].free).toEqual([{ trackId: 'A1', lo: 450, hi: 500 }, { trackId: 'L', lo: 0, hi: 300 }, { trackId: 'L', lo: 700, hi: 1100 }])
    expect(blocks[0].sections).toEqual([])
    // At 10 m/s, 100 m long: S1 is free once the tail has left it at 1100 m, the block once it has passed B at 1300 m.
    const clock = { at0: 0, first: (x) => x / 10, standstills: [], length: 3200, end: 400 }
    const [, timed] = blockingTimes(blocks, clock, 100, { setup: 12, approach: 1000, release: 6 })
    expect(timed.end).toBe(146)
    expect(timed.sections[0].end).toBe(116)
    // Another train's block over S1 alone meets this one there only, and is let go when S1 is.
    const other = { index: 0, begin: 100, end: 300, stretches: [{ trackId: 'L', lo: 300, hi: 700 }], sections: [{ id: 'S1', to: 0, end: 300 }], free: [] }
    expect(conflictsOf([timed], [{ id: 'x', blocks: [other], begin: 100, end: 300 }])).toEqual([{ block: 1, other: 'x', otherBlock: 0, section: 'S1', begin: 100, end: 116, until: 300 }])
    expect(conflictsOf([timed], [{ id: 'x', blocks: [{ ...other, begin: 120 }], begin: 120, end: 300 }])).toEqual([])
  })

  it('gives the net to trains by departure, or by their kinds', () => {
    const [a, b] = [train('1', '7:00', { category: 'RB' }), train('2', '7:01', { category: 'ICE' })]
    expect(byPriority([b, a], DISPATCH).map(t => t.number)).toEqual(['1', '2'])
    expect(byPriority([a, b], { ...DISPATCH, priority: 'category' }).map(t => t.number)).toEqual(['2', '1'])
    expect(dispatchOf({ dispatch: { mode: 'show', times: { setup: 20 } } })).toMatchObject({ mode: 'show', priority: 'departure', times: { setup: 20, approach: 1000, release: 6 } })
  })
})

describe.skipIf(!existsSync(WASM))('trains on one track', () => {
  const simulate = async (request) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    return runWith(instance.exports, request)
  }
  const runOf = (t, holds) => runTrain(signalled, stations, t, vehicle, simulate, holds)
  const first = train('1', '7:00', { category: 'RB' })
  const second = train('2', '7:01', { category: 'ICE' })

  it('says where they conflict, each run for itself', async () => {
    const runs = await occupancy(signalled, [first, second], runOf, { ...DISPATCH, mode: 'show' })
    const [a, b] = [runs.get(first.id), runs.get(second.id)]
    expect(a.conflicts.length).toBeGreaterThan(0)
    expect(b.conflicts.map(c => c.other)).toContain(first.id)
    expect(b.waits).toEqual([])
    // Run for itself, the second leaves a minute after the first.
    expect(b.rows[0].departure - a.rows[0].departure).toBe(60)
  })

  it('lets a train run into a free block only, the one after waiting for it', async () => {
    const runs = await occupancy(signalled, [second, first], runOf, DISPATCH)
    const [a, b] = [runs.get(first.id), runs.get(second.id)]
    expect(a.waits).toEqual([])
    expect(b.conflicts).toEqual([])
    expect(b.waits.length).toBeGreaterThan(0)
    expect(b.waits.every(w => w.because.includes(first.id))).toBe(true)
    // No block of the one is held while the other holds it.
    const shared = (x, y) => x.stretches.some(s => y.stretches.some(z => z.trackId === s.trackId && Math.min(s.hi, z.hi) - Math.max(s.lo, z.lo) > 0.5))
    for (const x of a.blocks) for (const y of b.blocks) if (shared(x, y)) expect(x.end <= y.begin || y.end <= x.begin).toBe(true)
    expect(b.rows.at(-1).arrival).toBeGreaterThan(a.rows.at(-1).arrival)
  })

  it('gives way to the kind set first', async () => {
    const runs = await occupancy(signalled, [first, second], runOf, { ...DISPATCH, priority: 'category' })
    expect(runs.get(second.id).waits).toEqual([])
    expect(runs.get(first.id).waits.length).toBeGreaterThan(0)
    expect(runs.get(first.id).conflicts).toEqual([])
  })
})

// A single line with a crossing station: H-Stadt — line 1 — W-Dorf (track 1 straight, track 2 a little longer) — line 2 — K-Stadt.
const E0 = 500000, N0 = 5600000
const straight = (x0, y0, x1, y1) => ({
  elementType: 0, startNode: [E0 + x0, N0 + y0], endNode: [E0 + x1, N0 + y1], length: Math.hypot(x1 - x0, y1 - y0),
  bearing: (Math.atan2(x1 - x0, y1 - y0) * 180 / Math.PI + 360) % 360, speed: 100,
})
const inStation = (name, number) => ({ trackType: 2, stationName: name, trackNumber: number })
const at = (trackId, station, direction) => ({ anchor: 'station', trackId, station, side: 'right', direction })
const main = (id, trackId, station, direction) => ({ id, name: id, kind: 'real', at: at(trackId, station, direction), logicalSignals: ks })
const crossing = {
  tracks: [
    { id: 'H1', epsg: 25832, ...inStation('H-Stadt', '1'), elements: [straight(0, 0, 500, 0)] },
    { id: 'L1', epsg: 25832, trackType: 1, lineNumber: '6000', elements: [straight(500, 0, 3000, 0)] },
    { id: 'W1', epsg: 25832, ...inStation('W-Dorf', '1'), elements: [straight(3000, 0, 3600, 0)] },
    { id: 'W2', epsg: 25832, ...inStation('W-Dorf', '2'), elements: [straight(3000, 0, 3300, 60), straight(3300, 60, 3600, 0)] },
    { id: 'L2', epsg: 25832, trackType: 1, lineNumber: '6000', elements: [straight(3600, 0, 6000, 0)] },
    { id: 'K1', epsg: 25832, ...inStation('K-Stadt', '1'), elements: [straight(6000, 0, 6500, 0)] },
  ],
  switches: [
    { switchId: 'Wa', kind: 'turnout', ports: { A: { trackId: 'L1', endpoint: 'END' }, B1: { trackId: 'W2', endpoint: 'BEGIN' }, B2: { trackId: 'W1', endpoint: 'BEGIN' } } },
    { switchId: 'Wb', kind: 'turnout', ports: { A: { trackId: 'L2', endpoint: 'BEGIN' }, B1: { trackId: 'W2', endpoint: 'END' }, B2: { trackId: 'W1', endpoint: 'END' } } },
  ],
  platforms: [
    { id: 'pH', trackId: 'H1', startStation: 100, endStation: 300, stationName: 'H-Stadt', code: '1' },
    { id: 'pK', trackId: 'K1', startStation: 200, endStation: 400, stationName: 'K-Stadt', code: '1' },
  ],
  signalling: {
    fields: {},
    collections: {
      signals: [
        main('HN', 'H1', 450, 'up'), main('WF', 'L1', 2300, 'up'), main('WN1', 'W1', 550, 'up'), main('WN2', 'W2', 560, 'up'), main('KF', 'L2', 2200, 'up'),
        main('KP', 'K1', 50, 'down'), main('WA', 'L2', 200, 'down'), main('WP1', 'W1', 50, 'down'), main('WP2', 'W2', 50, 'down'), main('HA', 'L1', 300, 'down'),
      ],
      routes: [
        { id: 'WF-WN1', kind: 'train', startSignalId: 'WF', targetSignalId: 'WN1', path: { trackIds: ['L1', 'W1'] } },
        { id: 'WF-WN2', kind: 'train', startSignalId: 'WF', targetSignalId: 'WN2', path: { trackIds: ['L1', 'W2'] } },
        { id: 'WA-WP1', kind: 'train', startSignalId: 'WA', targetSignalId: 'WP1', path: { trackIds: ['L2', 'W1'] } },
        { id: 'WA-WP2', kind: 'train', startSignalId: 'WA', targetSignalId: 'WP2', path: { trackIds: ['L2', 'W2'] } },
      ],
      overlaps: [],
      // Axle counters at the line's ends, by the switches and the signals: the switch areas, the station tracks, the lines each a section of their own.
      detectionPoints: [
        ['a', 'L1', 2350], ['b', 'W1', 20], ['c', 'W2', 20], ['d', 'W1', 580], ['e', 'W2', 590], ['f', 'L2', 150],
      ].map(([id, trackId, station]) => ({ id, kind: 'axleCounter', at: at(trackId, station) })),
      trackSections: [
        { id: 'line1', kind: 'axleCounting', boundedBy: ['a'] },
        { id: 'switchA', kind: 'axleCounting', boundedBy: ['a', 'b', 'c'] },
        { id: 'track1', kind: 'axleCounting', boundedBy: ['b', 'd'] },
        { id: 'track2', kind: 'axleCounting', boundedBy: ['c', 'e'] },
        { id: 'switchB', kind: 'axleCounting', boundedBy: ['d', 'e', 'f'] },
        { id: 'line2', kind: 'axleCounting', boundedBy: ['f'] },
      ],
    },
  },
}

describe.skipIf(!existsSync(WASM))('trains meeting on a single line', () => {
  const simulate = async (request) => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    return runWith(instance.exports, request)
  }
  const places = stationsOf(crossing)
  const runOf = (t, holds, routes) => runTrain(crossing, places, t, vehicle, simulate, holds, routes)
  const there = train('1', '7:00', { points: [newPoint('H-Stadt'), newPoint('K-Stadt')] })
  const back = train('2', '7:00', { points: [newPoint('K-Stadt'), newPoint('H-Stadt')] })

  it('cross in the station between, the one taking its other track, route by route', async () => {
    const runs = await occupancy(crossing, [there, back], runOf, DISPATCH)
    const [a, b] = [runs.get(there.id), runs.get(back.id)]
    expect(a.conflicts).toEqual([])
    expect(b.conflicts).toEqual([])
    // The one coming second finds track 1 held and is sent into track 2.
    const detoured = [a, b].find(r => r.detours.length)
    expect(detoured.detours[0]).toMatchObject({ routeId: expect.stringMatching(/-WN2$|-WP2$/) })
    expect(detoured.route.path.map(p => p.trackId)).toContain('W2')
    // Neither waits at its start for the other to have left the line: both leave at 7:00.
    expect(a.rows[0].departure).toBe(parseClock('7:00'))
    expect(b.rows[0].departure).toBe(parseClock('7:00'))
    // One waits in W-Dorf for the other, not for its whole run.
    const waits = [...a.waits, ...b.waits]
    expect(waits.length).toBeGreaterThan(0)
    expect(waits.every(w => w.signalId)).toBe(true)
    expect(Math.max(a.delay, b.delay)).toBeLessThan(b.rows.at(-1).arrival - b.rows[0].departure - 60)
  })

  it('send the one coming second into the other track where the first stands in track 1, whichever it is', async () => {
    for (const [one, two] of [['7:00', '7:00:40'], ['7:00', '7:01'], ['7:00:40', '7:00'], ['7:01', '7:00']]) {
      const [x, y] = [{ ...there, departure: parseClock(one) }, { ...back, departure: parseClock(two) }]
      const runs = await occupancy(crossing, [x, y], runOf, DISPATCH)
      const [a, b] = [runs.get(x.id), runs.get(y.id)]
      expect([a.conflicts, b.conflicts]).toEqual([[], []])
      // Each leaves when it is to: they cross in W-Dorf, one of them on track 2.
      expect([a.rows[0].departure, b.rows[0].departure]).toEqual([x.departure, y.departure])
      expect([a, b].filter(r => r.route.path.some(p => p.trackId === 'W2'))).toHaveLength(1)
    }
  })
})
