import { describe, expect, it } from 'vitest'
import { newVehicle } from '@rail-across/vehicles'
import { nearestLocation } from './places.js'
import { pathLength } from './path.js'
import { osrdFindPath, osrdHeightProfile, osrdLngLatAt, osrdNet, osrdPathGeoJSON, osrdProperties, osrdRoute, osrdSimulationRequest } from './osrd.js'

// A small RailJSON net near the equator, a degree of longitude some 111 km: a
// runs east 1000 m into a point switch, whose B1 leads on to b (east) and
// whose B2 to c; b meets d by a link, d drawn the other way (its END at the
// link). a rises 5 ‰ and is 160 km/h, b has a curve of 600 m and 100 km/h
// in either direction and 60 km/h the other way only; a and b are under
// 25 kV. The station Ost has parts on d and c.
const deg = 1000 / 111195
const line = (x0, x1) => ({ type: 'LineString', coordinates: [[x0 * deg, 0], [x1 * deg, 0]] })
const infra = {
  version: '3.5.3',
  track_sections: [
    { id: 'a', length: 1000, geo: line(0, 1000), slopes: [{ begin: 0, end: 1000, gradient: 5 }], curves: [] },
    { id: 'b', length: 1000, geo: line(1000, 2000), slopes: [], curves: [{ begin: 200, end: 400, radius: -600 }] },
    { id: 'c', length: 500, geo: { type: 'LineString', coordinates: [[1000 * deg, 0], [1400 * deg, 300 * deg]] }, slopes: [], curves: [] },
    { id: 'd', length: 1000, geo: line(3000, 2000), slopes: [], curves: [] },
  ],
  switches: [
    { id: 'w1', switch_type: 'point_switch', ports: { A: { track: 'a', endpoint: 'END' }, B1: { track: 'b', endpoint: 'BEGIN' }, B2: { track: 'c', endpoint: 'BEGIN' } } },
    { id: 'l1', switch_type: 'link', ports: { A: { track: 'b', endpoint: 'END' }, B: { track: 'd', endpoint: 'END' } } },
  ],
  speed_sections: [
    { id: 's1', speed_limit: 160 / 3.6, track_ranges: [{ track: 'a', begin: 0, end: 1000, applicable_directions: 'BOTH' }] },
    { id: 's2', speed_limit: 100 / 3.6, track_ranges: [{ track: 'b', begin: 0, end: 1000, applicable_directions: 'BOTH' }] },
    { id: 's3', speed_limit: 60 / 3.6, track_ranges: [{ track: 'b', begin: 0, end: 500, applicable_directions: 'STOP_TO_START' }] },
  ],
  electrifications: [{ id: 'e1', voltage: '25000V', track_ranges: [{ track: 'a', begin: 0, end: 1000 }, { track: 'b', begin: 0, end: 1000 }] }],
  operational_points: [
    { id: 'op1', name: 'Ost', parts: [{ track: 'd', position: 200 }, { track: 'c', position: 400 }] },
    { id: 'op2', name: 'West', parts: [{ track: 'a', position: 100 }] },
  ],
}
const net = osrdNet(infra)
const pieces = (path) => path.map(p => [p.trackId, Math.round(p.start), Math.round(p.end)])

describe('a net of OSRD', () => {
  it('is drawn and picked as a variant\'s, its sections as long as RailJSON states them', () => {
    expect([...net.lines.keys()]).toEqual(['a', 'b', 'c', 'd'])
    expect(net.lines.get('c').at(-1).s).toBeCloseTo(500, 6)
    expect(nearestLocation(net, [1500 * deg, 0.00001], 20)).toEqual({ trackId: 'b', station: 500 })
    const ll = osrdLngLatAt(net, { trackId: 'd', station: 250 })
    expect(ll[0] / deg).toBeCloseTo(2750, 3)
    expect(net.places.map(p => [p.label, p.locs.length])).toEqual([['Ost', 2], ['West', 1]])
  })

  it('is run over its switches\' routes and links, the shortest way', () => {
    const path = osrdFindPath(net, { trackId: 'a', station: 100, direction: null }, { trackId: 'd', station: 200 })
    expect(pieces(path)).toEqual([['a', 100, 1000], ['b', 0, 1000], ['d', 1000, 200]])
    expect(pathLength(path)).toBeCloseTo(2700, 6)
    // from b, c is reached only by turning back: there is no route from B1 to B2
    expect(osrdFindPath(net, { trackId: 'b', station: 100, direction: 'up' }, { trackId: 'c', station: 100 })).toBeNull()
    // the way back
    expect(pieces(osrdFindPath(net, { trackId: 'd', station: 200 }, { trackId: 'a', station: 100 }))).toEqual([['d', 200, 1000], ['b', 1000, 0], ['a', 1000, 100]])
  })

  it('takes the part of a station the way leads to', () => {
    const [west, ost] = [net.places[1], net.places[0]]
    const route = osrdRoute(net, [west, ost])
    expect(route.chosen[1]).toMatchObject({ trackId: 'c', station: 400 })   // 1300 m, not 2700 to d
    expect(route.offsets).toEqual([0, 1300])
    expect(osrdPathGeoJSON(net, route.path).features[0].geometry.coordinates.length).toBeGreaterThanOrEqual(3)
    expect(osrdRoute(net, [{ loc: { trackId: 'b', station: 100, direction: 'up' } }, ost])).toMatchObject({ path: expect.any(Array) })
    expect(osrdRoute(net, [{ loc: { trackId: 'c', station: 100, direction: 'up' } }, { loc: { trackId: 'b', station: 100 } }])).toEqual({ failedAt: 1 })
  })

  it('gives its slopes, curves and speed sections the way the train runs, and the wire', () => {
    const there = osrdFindPath(net, { trackId: 'a', station: 0, direction: 'up' }, { trackId: 'd', station: 0 })
    const p = osrdProperties(net, there)
    expect(p.length).toBeCloseTo(3000, 6)
    expect(p.gradients).toEqual([{ begin: 0, end: 1000, value: 5 }, { begin: 1200, end: 1400, value: Math.round(650 / 545 * 1000) / 1000 }])
    expect(p.speedLimits.map(s => [s.begin, s.end, Math.round(s.speed * 3.6)])).toEqual([[0, 1000, 160], [1000, 2000, 100]])
    expect(p.gaps.speed).toEqual([{ begin: 2000, end: 3000, value: 0 }])
    expect(p.gaps.heights).toEqual([{ begin: 1000, end: 3000, value: 0 }])

    const back = osrdProperties(net, osrdFindPath(net, { trackId: 'd', station: 0, direction: 'up' }, { trackId: 'a', station: 0 }))
    expect(back.gradients.at(-1)).toEqual({ begin: 2000, end: 3000, value: -5 })
    // b run down: 100 km/h, and 60 km/h on its first half from where it ends
    expect(back.speedLimits.map(s => [s.begin, s.end, Math.round(s.speed * 3.6)])).toEqual([[1000, 1500, 100], [1500, 2000, 60], [2000, 3000, 160]])
    expect(osrdHeightProfile(there).every(q => q.height == null)).toBe(true)
  })

  it('makes the request the simulation takes, its mode by the wire', () => {
    const path = osrdFindPath(net, { trackId: 'a', station: 0, direction: 'up' }, { trackId: 'b', station: 1000 })
    const vehicle = newVehicle()
    const { request, properties } = osrdSimulationRequest(net, path, { vehicle, stops: [{ offset: 2000, duration: 0 }], margin: { value: '5%', distribution: 'linear' } })
    expect(request.path.length).toBeCloseTo(2000, 6)
    expect(request.path.speed_limits).toHaveLength(2)
    expect(request.schedule).toEqual([{ path_offset: 2000, stop_duration: 0 }])
    expect(request.margins).toEqual({ boundaries: [], values: ['5%'] })
    expect(properties.signalled).toEqual([])
    expect(properties.traction.unpowered).toEqual(expect.any(Array))
  })
})
