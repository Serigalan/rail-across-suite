import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { runWith, stationsOf } from '@rail-across/running-times'
import { net, WASM } from '../../running-times/test/net.js'
import { newPoint, newTrain, parseClock, runTrain } from './timetable.js'
import { blocksAlong, byPriority, DISPATCH, dispatchOf, occupancy } from './occupancy.js'

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
