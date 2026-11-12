import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { net, WASM } from '../../running-times/test/net.js'
import { crossing } from '../test/crossing.js'
import { newPoint, newTrain, parseClock, runTrain } from './timetable.js'
import { blockingTimes, blocksAlong, byPriority, conflictsOf, DISPATCH, dispatchOf, occupancy, withHeadways } from './occupancy.js'

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

  it('takes the times to set and release a route and an overlap\'s release from the signalling, the timetable\'s where it says none', () => {
    const path = [{ trackId: 'A1', start: 200, end: 500 }, { trackId: 'L', start: 0, end: 2500 }, { trackId: 'C1', start: 0, end: 400 }]
    const timed = {
      ...signalled,
      signalling: {
        fields: { routeTimes: { setting: 8 } },
        collections: {
          ...signalled.signalling.collections,
          routes: [{ id: 'r', kind: 'train', startSignalId: 'N1', targetSignalId: 'B', settingTime: 20, releaseTime: 3 }],
          overlaps: [{ id: 'd', signalId: 'B', length: 100, releaseDelay: 30 }],
        },
      },
    }
    const blocks = blocksAlong(timed, path)
    expect(blocks.map(b => [b.setting, b.release, b.overlapRelease])).toEqual([[null, null, null], [20, 3, 30], [8, null, null], [8, null, null]])
    // At 10 m/s, 100 m long: the block from N1 is asked for 1000 m before it — at once —, less its own 20 s.
    const clock = { at0: 0, first: (x) => x / 10, standstills: [], length: 3200, end: 400 }
    const [first, own, next] = blockingTimes(blocks, clock, 100, { setup: 12, approach: 1000, release: 6 })
    expect(first.begin).toBe(-12)
    expect(own.begin).toBe(-20)
    expect(own.end).toBe(140 + 3)
    // B is asked for where N1, a multi-section signal, announces it: 250 m along.
    expect(next.begin).toBe(25 - 8)
    expect(next.end).toBe(280 + 6)
  })

  it('asks for a block where its signal is announced: within sight of the distant signal, or of the main signal before showing Ks 2', () => {
    const path = [{ trackId: 'A1', start: 200, end: 500 }, { trackId: 'L', start: 0, end: 2500 }, { trackId: 'C1', start: 0, end: 400 }]
    const distant = { ...signal('vA', 'L', 1600), sight: { required: 150 }, logicalSignals: [{ system: 'ks', type: 'distant', function: 'distant', frames: [] }] }
    const plain = { ...signal('B', 'L', 1000), logicalSignals: [{ system: 'ks', type: 'main', function: 'block', frames: [] }] }
    const announced = {
      ...signalled,
      signalling: { ...signalled.signalling, collections: { ...signalled.signalling.collections, signals: [signal('N1', 'A1', 450), plain, signal('A', 'L', 2400), distant] } },
    }
    const blocks = blocksAlong(announced, path)
    // B is announced by N1, a multi-section signal, 250 m along; A — after a plain main signal — by its distant signal, 1900 m along, seen 150 m before.
    expect(blocks.map(b => b.approachFrom)).toEqual([null, null, 250, 1750])
    const clock = { at0: 0, first: (x) => x / 10, standstills: [], length: 3200, end: 400 }
    const timed = blockingTimes(blocks, clock, 100, { setup: 12, approach: 1000, release: 6 })
    expect(timed[2].begin).toBe(25 - 12)
    expect(timed[3].begin).toBe(175 - 12)
  })

  it('says of a train entering at a boundary sooner after the one before than its least headway', () => {
    const record = { endMarks: [{ id: 'g', kind: 'boundary', trackId: 'L', endpoint: 'BEGIN', headway: 180 }] }
    const at = (id, place) => ({ id, points: [{ place }] })
    const trains = [at('1', 'g'), at('2', 'g'), at('3', 'g'), at('4', 'A-Dorf')]
    const runs = new Map([['1', { clock: { at0: 600 } }], ['2', { clock: { at0: 720 } }], ['3', { clock: { at0: 1000 } }], ['4', { clock: { at0: 610 } }]])
    const out = withHeadways(record, trains, runs)
    expect(out.get('2').headway).toEqual({ place: 'g', after: '1', gap: 120, least: 180 })
    expect(out.get('1').headway).toBeUndefined()
    expect(out.get('3').headway).toBeUndefined()
    expect(out.get('4').headway).toBeUndefined()
    expect(withHeadways({ endMarks: [] }, trains, runs)).toBe(runs)
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
