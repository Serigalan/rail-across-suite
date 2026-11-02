import { describe, expect, it } from 'vitest'
import { net } from '../test/net.js'
import { routeVia } from './route.js'
import { stationNamed, stationTargets, stationsAlong, stationsOf } from './stations.js'

describe('the stations of a net', () => {
  const stations = stationsOf(net)

  it('are its station tracks by name, and the halts its platforms name alone', () => {
    expect(stations.map(s => [s.name, s.kind, s.tracks.map(t => t.label)])).toEqual([
      ['A-Dorf', 'station', ['1']], ['B-Halt', 'halt', []], ['C-Stadt', 'station', ['1', '2']],
    ])
    expect(stationNamed(stations, ' c-stadt ').tracks[1].platforms.map(p => p.id)).toEqual(['pC2'])
  })

  it('are reached on the track asked for, else on any with a platform', () => {
    const c = stationNamed(stations, 'C-Stadt')
    expect(stationTargets(c)).toEqual([{ trackId: 'C1', station: 400, direction: null }, { trackId: 'C2', station: 400, direction: null }])
    expect(stationTargets(c, { trackId: 'C2' })).toEqual([{ trackId: 'C2', station: 400, direction: null }])
    expect(stationTargets(stationNamed(stations, 'B-Halt'))).toEqual([{ trackId: 'L', station: 1100, direction: null }])
  })

  it('give a train its way over the tracks it can take, and lie along it', () => {
    const route = routeVia(net, ['A-Dorf', 'B-Halt', 'C-Stadt'].map(n => stationTargets(stationNamed(stations, n))))
    expect(route.chosen.map(c => c.trackId)).toEqual(['A1', 'L', 'C1'])
    expect(route.offsets).toEqual([0, 1400, 3200])
    const along = stationsAlong(route.path, stations)
    expect(along.map(s => [s.name, s.begin, s.end])).toEqual([['A-Dorf', 0, 300], ['B-Halt', 1400, 1400], ['C-Stadt', 2800, 3200]])
    // back the other way, from track 2 of C-Stadt
    const back = routeVia(net, [stationTargets(stationNamed(stations, 'C-Stadt'), { trackId: 'C2' }), stationTargets(stationNamed(stations, 'A-Dorf'))])
    expect(back.path.map(p => p.trackId)).toEqual(['C2', 'L', 'A1'])
  })

  it('say where no way leads on', () => {
    // from A-Dorf to C-Stadt and back to B-Halt would mean turning
    const turned = routeVia(net, ['A-Dorf', 'C-Stadt', 'B-Halt'].map(n => stationTargets(stationNamed(stations, n))))
    expect(turned.failedAt).toBe(2)
    expect(routeVia(net, [[], stationTargets(stationNamed(stations, 'A-Dorf'))]).failedAt).toBe(0)
  })

  it('take the stations of the register as stations too, on the tracks of their stopping places and platforms as far as they reach', () => {
    // B-Halt made a station in the Signalling-Tool, a stopping place of it on the line; a station nothing places is none.
    const registered = {
      ...net,
      operationalPoints: [{ id: 'op-b', name: 'B-Halt', rl100: 'LBH' }, { id: 'op-x', name: 'X-Feld' }, { id: 'op-c', name: 'C-Stadt' }],
      signalling: { fields: {}, collections: { stoppingPlaces: [{ id: 'h1', name: '1', operationalPointId: 'op-b', at: { anchor: 'station', trackId: 'L', startStation: 900, endStation: 1150 } }] } },
    }
    const all = stationsOf(registered)
    expect(all.map(s => [s.name, s.kind])).toEqual([['A-Dorf', 'station'], ['B-Halt', 'station'], ['C-Stadt', 'station']])
    const b = stationNamed(all, 'B-Halt')
    expect(b).toMatchObject({ pointId: 'op-b', tracks: [{ id: 'L', from: 900, to: 1200, at: 1025 }] })
    expect(stationNamed(all, 'C-Stadt').pointId).toBe('op-c')
    // a train begins there, at its platform; passing, the station is as long as it reaches on the line
    expect(stationTargets(b)).toEqual([{ trackId: 'L', station: 1100, direction: null }])
    const route = routeVia(registered, ['B-Halt', 'C-Stadt'].map(n => stationTargets(stationNamed(all, n))))
    expect(route.chosen[0]).toMatchObject({ trackId: 'L', station: 1100 })
    const along = stationsAlong(routeVia(registered, ['A-Dorf', 'C-Stadt'].map(n => stationTargets(stationNamed(all, n)))).path, all)
    expect(along.map(s => [s.name, s.begin, s.end])).toEqual([['A-Dorf', 0, 300], ['B-Halt', 1200, 1500], ['C-Stadt', 2800, 3200]])
  })
})
