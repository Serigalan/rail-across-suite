import { describe, it, expect } from 'vitest'
import { domainOf, deleted } from './domain.js'
import { searchItineraries, waysBetween, withItinerary } from './itineraries.js'
import { newStoppingPlace, snappedStation, stoppingStretches, stretchAt } from './stoppingPlaces.js'

const t = (k) => k
// A station X with a passing loop between two turnouts, a line to the next station Y:
//   a (0–200) ─ w1 ─┬─ m (0–400) ─┬─ w2 ─ b (0–300)
//                   └─ l (0–420) ─┘
const el = (length) => ({ elementType: 0, startNode: [500000, 5700000], endNode: [500000 + length, 5700000], bearing: 90, length })
const track = (id, length) => ({ id, name: id, epsg: 25832, elements: [el(length)] })
const port = (trackId, endpoint) => ({ trackId, endpoint })
const net = {
  tracks: [track('a', 200), track('m', 400), track('l', 420), track('b', 300)],
  switches: [
    { switchId: 'w1', kind: 'turnout', ports: { A: port('a', 'END'), B1: port('l', 'BEGIN'), B2: port('m', 'BEGIN') } },
    { switchId: 'w2', kind: 'turnout', ports: { A: port('b', 'BEGIN'), B1: port('l', 'END'), B2: port('m', 'END') } },
  ],
}
const ks = [{ system: 'ks', type: 'multiSection', function: 'exit', frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }] }]
const signal = (id, trackId, station, station0) => ({
  id, name: id, kind: 'real', at: { anchor: 'station', trackId, station, side: 'right', direction: 'up' }, logicalSignals: ks,
  ...(station0 ? { operationalPointId: station0 } : {}),
})
const record = {
  platforms: [],
  signalling: {
    fields: {},
    collections: {
      signals: [signal('A', 'a', 150, 'X'), signal('N1', 'm', 380, 'X'), signal('N2', 'l', 400, 'X'), signal('F', 'b', 200, 'Y'), signal('S', 'b', 100)],
      detectionPoints: [{ id: 'd1', name: '1', kind: 'axleCounter', at: { anchor: 'station', trackId: 'm', station: 20 } }],
    },
  },
}
const place = (id, trackId, from, to, op) => ({ ...newStoppingPlace({ name: id, operationalPointId: op, trackId, from, to }), id })

describe('stopping places and itineraries', () => {
  it('offers the stretches of a track the signals and detection points cut it into', () => {
    const domain = domainOf(record)
    expect(stoppingStretches(net, domain, 'm', t)).toEqual([
      { from: 0, to: 20, below: null, above: '1' }, { from: 20, to: 380, below: '1', above: 'N1' }, { from: 380, to: 400, below: 'N1', above: null },
    ])
    // A platform's stretch is cut the same way, within it.
    expect(stoppingStretches(net, domain, 'm', t, [100, 390])).toEqual([{ from: 100, to: 380, below: null, above: 'N1' }, { from: 380, to: 390, below: 'N1', above: null }])
  })

  it('offers the stretch a click on the plan meets, at a platform before on a track', () => {
    const domain = domainOf(record)
    const where = { tracks: ['m'], platforms: [{ id: 'p1', trackId: 'l', startStation: 50, endStation: 350 }] }
    expect(stretchAt(net, domain, where, { trackId: 'm', station: 200 }, t)).toMatchObject({ trackId: 'm', from: 20, to: 380, platformId: undefined })
    expect(stretchAt(net, domain, where, { trackId: 'l', station: 100 }, t)).toMatchObject({ trackId: 'l', from: 50, to: 350, platformId: 'p1' })
    // Beside the platform, on a track that is not the station's, there is none.
    expect(stretchAt(net, domain, where, { trackId: 'l', station: 380 }, t)).toBeNull()
    expect(stretchAt(net, domain, where, { trackId: 'b', station: 50 }, t)).toBeNull()
    // An end picked near a signal or detection point is taken at it.
    expect(snappedStation(domain, 'm', 377.2, 5, t)).toBe(380)
    expect(snappedStation(domain, 'm', 300.04, 5, t)).toBe(300)
  })

  it('finds the ways from one stopping place to another, never turning back, the shortest first', () => {
    const from = place('h-a', 'a', 0, 150, 'X'), to = place('h-b', 'b', 100, 200, 'Y')
    const ways = waysBetween(net, from, to)
    expect(ways.map(w => w.trackIds)).toEqual([['a', 'm', 'b'], ['a', 'l', 'b']])
    expect(ways[0].pieces[0]).toEqual({ trackId: 'a', direction: 'up', start: 150, end: 200 })
    expect(ways[0].pieces.at(-1)).toEqual({ trackId: 'b', direction: 'up', start: 0, end: 200 })
  })

  it('takes the routes of the stations on a way, none from a signal of the open line, and makes it an itinerary', () => {
    const from = place('h-a', 'a', 0, 150, 'X'), to = place('h-b', 'b', 100, 200, 'Y')
    let state = domainOf(record)
    state = { ...state, collections: { ...state.collections, stoppingPlaces: [from, to] } }
    const [first, second] = searchItineraries(net, state, from, to)
    // A → N1 is X's, N1 → S too; S → F starts on the open line and is the block's.
    expect(first.routes.map(r => [r.startSignalId, r.targetSignalId, r.trackIds, r.stationId])).toEqual([
      ['A', 'N1', ['a', 'm'], 'X'], ['N1', 'S', ['m', 'b'], 'X'],
    ])
    expect(first.problems).toEqual([])
    expect(first.switches).toEqual(['w1', 'w2'])
    expect(second.routes.map(r => r.startSignalId)).toEqual(['A', 'N2'])
    const made = withItinerary(state, first, from, to, 'X → Y')
    state = made.state
    expect(state.collections.routes).toHaveLength(2)
    expect(state.collections.itineraries[0]).toMatchObject({ id: made.id, startStoppingPlaceId: 'h-a', targetStoppingPlaceId: 'h-b', path: { trackIds: ['a', 'm', 'b'] } })
    // Found again, it is there with its routes; the other way's routes are not.
    const again = searchItineraries(net, state, from, to)
    expect(again[0].existingId).toBe(made.id)
    expect(again[0].routes.map(r => r.existingId)).toEqual(state.collections.routes.map(r => r.id))
    expect(again[1].routes.some(r => r.existingId)).toBe(false)
    // Deleting a route takes the itinerary over it along; so does deleting its stopping place.
    expect(deleted(state, 'routes', state.collections.routes[1].id).collections.itineraries).toEqual([])
    expect(deleted(state, 'stoppingPlaces', 'h-b').collections.itineraries).toEqual([])
  })

  it('says where no signal ends the way at its target', () => {
    const from = place('h-a', 'a', 0, 150, 'X'), to = place('h-m', 'm', 20, 380, 'X')
    const state = { ...domainOf(record), collections: { ...domainOf(record).collections, signals: record.signalling.collections.signals.filter(s => s.id !== 'N1') } }
    const [way] = searchItineraries(net, state, from, to)
    expect(way.problems).toEqual(['no_target_signal', 'no_routes'])
  })
})
