import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { curveResistance, findPath, locationAt, offsetOnPath, pathLength, pathProperties } from './path.js'
import { routeThrough, signalledSpeeds } from './route.js'
import { marginValue, simulationRequest } from './request.js'
import { VEHICLES, rollingStockOf } from './vehicles.js'
import { runWith } from './engine.js'

// A small net in UTM 32: T1 runs east 1000 m into a turnout, whose straight
// route leads on to T2 (east, 1000 m) and whose branch to T3; T2 meets T4 end
// to end. T1 rises 10 m, T2 has a curve of 1000 m; the design speeds are
// 120 km/h on T1, 80 on T2, none on T4.
const E0 = 500000, N0 = 5600000
const straight = (x0, y0, x1, y1, extra = {}) => ({
  elementType: 0, startNode: [x0, y0], endNode: [x1, y1], length: Math.hypot(x1 - x0, y1 - y0),
  bearing: (Math.atan2(x1 - x0, y1 - y0) * 180 / Math.PI + 360) % 360, ...extra,
})
const record = {
  tracks: [
    { id: 'T1', epsg: 25832, elements: [straight(E0, N0, E0 + 1000, N0, { speed: 120 })], heights: [{ station: 0, z: 100 }, { station: 1000, z: 110 }] },
    { id: 'T2', epsg: 25832, elements: [straight(E0 + 1000, N0, E0 + 1500, N0, { speed: 80 }), { ...straight(E0 + 1500, N0, E0 + 2000, N0, { speed: 80 }), elementType: 1, radius: -1000 }] },
    { id: 'T3', epsg: 25832, elements: [straight(E0 + 1000, N0, E0 + 1600, N0 + 300)] },
    { id: 'T4', epsg: 25832, elements: [straight(E0 + 2000, N0, E0 + 3000, N0)] },
  ],
  switches: [{ switchId: 'S1', kind: 'turnout', ports: { A: { trackId: 'T1', endpoint: 'END' }, B1: { trackId: 'T2', endpoint: 'BEGIN' }, B2: { trackId: 'T3', endpoint: 'BEGIN' } } }],
  signalling: {
    collections: {
      signals: [
        { id: 'A', name: 'A', kind: 'real', at: { anchor: 'station', trackId: 'T1', station: 200, direction: 'up' }, logicalSignals: [{ system: 'ks', signallingSpeeds: [{ routeId: 'R1', speed: 60 }] }] },
        { id: 'N', name: 'N', kind: 'real', at: { anchor: 'station', trackId: 'T2', station: 900, direction: 'up' } },
      ],
      routes: [{ id: 'R1', kind: 'train', startSignalId: 'A', targetSignalId: 'N', path: { trackIds: ['T1', 'T2'] } }],
    },
  },
}

describe('a path over the net', () => {
  it('runs over the turnout and the joint, the shortest way', () => {
    const path = findPath(record, { trackId: 'T1', station: 100, direction: 'up' }, { trackId: 'T4', station: 500 })
    expect(path.map(p => [p.trackId, Math.round(p.start), Math.round(p.end)])).toEqual([['T1', 100, 1000], ['T2', 0, 1000], ['T4', 0, 500]])
    expect(pathLength(path)).toBeCloseTo(2400, 6)
  })

  it('tries both ways where the start has no direction, and none against the turnout', () => {
    const back = findPath(record, { trackId: 'T4', station: 500 }, { trackId: 'T1', station: 100 })
    expect(back.map(p => p.trackId)).toEqual(['T4', 'T2', 'T1'])
    expect(back[0].end).toBe(0)
    // from the branch onto the straight route: no way over the turnout
    expect(findPath(record, { trackId: 'T3', station: 10, direction: 'down' }, { trackId: 'T2', station: 10 })).toBeNull()
  })

  it('keeps a target ahead on the same track to that track', () => {
    expect(findPath(record, { trackId: 'T1', station: 100, direction: 'up' }, { trackId: 'T1', station: 600 })).toEqual([{ trackId: 'T1', start: 100, end: 600 }])
  })

  it('finds offsets and locations along it', () => {
    const path = findPath(record, { trackId: 'T1', station: 100, direction: 'up' }, { trackId: 'T4', station: 500 })
    expect(offsetOnPath(path, { trackId: 'T2', station: 250 })).toBeCloseTo(1150, 6)
    expect(locationAt(path, 1150)).toMatchObject({ trackId: 'T2', direction: 'up' })
    expect(locationAt(path, 1150).station).toBeCloseTo(250, 6)
  })
})

describe('what the physics needs of it', () => {
  const path = findPath(record, { trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T4', station: 1000 })
  const props = pathProperties(record, path)

  it('has the grades of the heights, rising positive, and the curve as a grade', () => {
    expect(props.length).toBeCloseTo(3000, 6)
    expect(props.gradients[0]).toMatchObject({ begin: 0, value: 10 })
    expect(props.gradients[0].end).toBeCloseTo(1000, 6)
    const curve = props.gradients.find(g => g.begin > 1000)
    expect(curve.value).toBeCloseTo(curveResistance(1000), 3)
    expect(curve.begin).toBeCloseTo(1500, 6)
  })

  it('falls the other way', () => {
    const down = pathProperties(record, findPath(record, { trackId: 'T1', station: 1000, direction: 'down' }, { trackId: 'T1', station: 0 }))
    expect(down.gradients).toEqual([{ begin: 0, end: 1000, value: -10 }])
  })

  it('takes the design speeds, and tells where heights and speeds are missing', () => {
    expect(props.speedLimits.map(s => Math.round(s.speed * 3.6))).toEqual([120, 80])
    expect(props.speedLimits[1].end).toBeCloseTo(2000, 6)
    expect(props.gaps.speed).toHaveLength(1)
    expect(props.gaps.speed[0].begin).toBeCloseTo(2000, 6)
    expect(props.gaps.heights[0].begin).toBeCloseTo(1000, 6)
  })

  it('takes the speed a signal signals for the route it runs along', () => {
    const speeds = signalledSpeeds(record, path)
    expect(speeds).toHaveLength(1)
    expect(speeds[0].begin).toBeCloseTo(200, 6)
    expect(speeds[0].end).toBeCloseTo(1900, 6)
    expect(speeds[0].speed * 3.6).toBeCloseTo(60, 6)
  })
})

describe('a train through its stops', () => {
  it('never turns back at a stop', () => {
    const route = routeThrough(record, [{ trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T2', station: 500 }, { trackId: 'T4', station: 900 }])
    expect(route.path.map(p => p.trackId)).toEqual(['T1', 'T2', 'T4'])
    expect(route.offsets.map(Math.round)).toEqual([0, 1500, 2900])
    // back from T2 to T1 would mean turning
    const turned = routeThrough(record, [{ trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T2', station: 500 }, { trackId: 'T1', station: 10 }])
    expect(turned.failedAt).toBe(2)
  })

  it('makes the simulation request', () => {
    const { path, offsets } = routeThrough(record, [{ trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T4', station: 900 }])
    const { request } = simulationRequest(record, path, { vehicle: VEHICLES[0], stops: [{ offset: offsets[1], duration: 0 }], margin: { value: '5%', distribution: 'mareco' } })
    expect(request.path.length).toBeCloseTo(2900, 6)
    expect(request.path.speed_limits.map(s => s.source)).toEqual(['design', 'design', 'signal'])
    expect(request.schedule).toEqual([{ path_offset: request.path.length, stop_duration: 0 }])
    expect(request.margins).toEqual({ boundaries: [], values: ['5%'] })
    expect(request.rolling_stock.effort_curve[0]).toEqual({ speed: 0, max_effort: 160000 })
  })

  it('reads margins as they are written', () => {
    expect(marginValue('5')).toBe('5%')
    expect(marginValue('4,5 %')).toBe('4.5%')
    expect(marginValue('3 min/100km')).toBe('3min/100km')
    expect(marginValue('')).toBe('none')
    expect(marginValue('fast')).toBeNull()
  })

  it('has effort curves that end at the top speed', () => {
    for (const v of VEHICLES) {
      const curve = rollingStockOf(v).effort_curve
      expect(curve.at(-1).speed).toBeCloseTo(v.maxSpeed, 9)
      expect(curve.every((p, i) => i === 0 || p.speed > curve[i - 1].speed)).toBe(true)
    }
  })
})

// The module itself, where it was built (npm run sim).
const WASM = new URL('../../public/sim/rail_across_sim.wasm', import.meta.url)
describe.skipIf(!existsSync(WASM))('the simulation module', () => {
  it('runs the train', async () => {
    const { instance } = await WebAssembly.instantiate(readFileSync(WASM))
    const { path, offsets } = routeThrough(record, [{ trackId: 'T1', station: 0, direction: 'up' }, { trackId: 'T2', station: 500 }, { trackId: 'T4', station: 900 }])
    const { request } = simulationRequest(record, path, {
      vehicle: VEHICLES[0],
      stops: [{ offset: offsets[1], duration: 30 }, { offset: offsets[2], duration: 0 }],
      margin: { value: '5%', distribution: 'mareco' },
    })
    const result = runWith(instance.exports, request)
    expect(result.error).toBeUndefined()
    const stop = result.final.scheduled_points[0]
    expect(stop.departure - stop.arrival).toBeCloseTo(30, 6)
    expect(result.final.positions.at(-1)).toBeCloseTo(2900, 6)
    // the signal's 60 km/h holds from A to N once the train is past A
    const at = result.base.positions.findIndex(p => p > 1000 && p < 1400)
    expect(result.base.speeds[at] * 3.6).toBeLessThanOrEqual(60 + 1e-6)
    expect(result.provisional.total_time).toBeGreaterThan(result.base.total_time)
  })
})
