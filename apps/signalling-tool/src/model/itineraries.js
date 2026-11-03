import { trackLength, tracksOnFrom } from '@rail-across/core/track'
import { newId } from './domain.js'
import { switchesOnRun } from './runs.js'

/**
 * The itineraries (Fahrwege) from one stopping place to another: the ways
 * the net has between the two — leaving the first either way, never turning
 * back —, and along each the main signals that act the way it runs, in
 * running order. From signal to signal they are the train routes it takes:
 * one of each station whose signal starts it, none on the open line, where
 * the block has the signals. A way is made an itinerary with the routes it
 * lacks; the routes it has already are taken as they are.
 */

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')

/** A signal or the end of a stopping place this close is at it [m]. */
const AT = 1

/** How far beyond its target's far end a way looks for the signal that ends it [m]: past the detection point that bounds it. */
const REACH = 50

/** How many ways are looked at, and how many tracks one may run at the most. */
const BUDGET = 20000
const DEEPEST = 80

/** The kinds of logical signal that start or end no train route. */
const NO_ROUTE = new Set(['distant', 'distantRepeater', 'shunting'])

/** Whether a signal starts and ends train routes: a real one with a main or multi-section screen. */
export const isMainSignal = (s) => s.kind === 'real' && (s.logicalSignals ?? []).some(l => !NO_ROUTE.has(l.type))

/**
 * The ways from stopping place `from` to `to`: [{ pieces, trackIds, length }]
 * shortest first, `limit` at the most. A piece is { trackId, direction,
 * start, end } — the train runs it from `start` to `end`. A way leaves `from`
 * at its end in the direction it runs and ends at the far end of `to`.
 */
export function waysBetween(net, from, to, { limit = 8 } = {}) {
  const { tracks, switches } = net
  const lengths = new Map(tracks.map(tr => [tr.id, trackLength(tr)]))
  const farEnd = (h, direction) => (direction === 'up' ? h.at.endStation : h.at.startStation)
  const nearEnd = (h, direction) => (direction === 'up' ? h.at.startStation : h.at.endStation)
  const found = []
  let budget = BUDGET
  for (const direction of ['up', 'down']) {
    const start = farEnd(from, direction)
    const first = from.at.trackId
    // The target ahead on the start's own track.
    if (to.at.trackId === first && (direction === 'up' ? nearEnd(to, 'up') >= start - AT : nearEnd(to, 'down') <= start + AT)) {
      found.push([{ trackId: first, direction, start, end: farEnd(to, direction) }])
      continue
    }
    const leaveAt = direction === 'up' ? lengths.get(first) : 0
    const go = (trackId, dir, pieces, seen) => {
      if (budget-- <= 0 || pieces.length > DEEPEST) return
      for (const next of tracksOnFrom(trackId, away(dir), tracks, switches)) {
        const length = lengths.get(next.trackId)
        if (length == null) continue
        const nd = next.endpoint === 'BEGIN' ? 'up' : 'down'
        const enter = nd === 'up' ? 0 : length
        if (next.trackId === to.at.trackId) {
          found.push([...pieces, { trackId: next.trackId, direction: nd, start: enter, end: farEnd(to, nd) }])
          continue
        }
        const key = `${next.trackId}:${nd}`
        if (seen.has(key)) continue
        go(next.trackId, nd, [...pieces, { trackId: next.trackId, direction: nd, start: enter, end: length - enter }], new Set([...seen, key]))
      }
    }
    go(first, direction, [{ trackId: first, direction, start, end: leaveAt }], new Set([`${first}:${direction}`]))
  }
  const lengthOf = (pieces) => pieces.reduce((sum, p) => sum + Math.abs(p.end - p.start), 0)
  const seen = new Set()
  return found
    .map(pieces => ({ pieces, trackIds: pieces.map(p => p.trackId), length: lengthOf(pieces) }))
    .sort((a, b) => a.length - b.length)
    .filter(w => { const k = w.trackIds.join(','); if (seen.has(k)) return false; seen.add(k); return true })
    .slice(0, limit)
}

/**
 * The main signals along a way that act the way it runs, in running order:
 * [{ signal, piece }] — `piece` the index of the piece it stands on; on the
 * last, `reach` beyond its end.
 */
export function signalsAlong(domain, way, reach = AT) {
  const out = []
  const last = way.pieces.length - 1
  way.pieces.forEach((p, i) => {
    const ahead = i === last ? reach : AT
    const lo = Math.min(p.start, p.end) - (p.direction === 'down' && i === last ? ahead : AT)
    const hi = Math.max(p.start, p.end) + (p.direction === 'up' && i === last ? ahead : AT)
    const here = domain.collections.signals
      .filter(s => isMainSignal(s) && s.at?.anchor === 'station' && s.at.trackId === p.trackId && s.at.station >= lo && s.at.station <= hi)
      .filter(s => s.bothDirections || (s.at.direction ?? 'up') === p.direction)
      .sort((a, b) => (p.direction === 'up' ? a.at.station - b.at.station : b.at.station - a.at.station))
    for (const signal of here) if (!out.some(o => o.signal.id === signal.id)) out.push({ signal, piece: i })
  })
  return out
}

/**
 * What a way is as an itinerary: { routes, problems } — `routes` from each
 * signal along it that is of a station to the next ({ startSignalId,
 * targetSignalId, trackIds, stationId, existingId }), `problems` what keeps it
 * from being one: no signal at the target's far end (`no_target_signal`), no
 * route at all (`no_routes`).
 */
export function routesOfWay(domain, way, target) {
  const lastPiece = way.pieces[way.pieces.length - 1]
  // The signal that ends it: the first one past the target's near end — one there is passed —, within reach of its far end.
  const near = lastPiece.direction === 'up' ? target.at.startStation : target.at.endStation
  const past = (s) => (lastPiece.direction === 'up' ? s.at.station > near + AT : s.at.station < near - AT)
  const all = signalsAlong(domain, way, REACH)
  const end = all.findIndex(a => a.piece === way.pieces.length - 1 && past(a.signal))
  const along = end >= 0 ? all.slice(0, end + 1) : all.filter(a => a.piece < way.pieces.length - 1 || !past(a.signal))
  const problems = []
  if (end < 0) problems.push('no_target_signal')
  const routes = []
  for (let i = 0; i + 1 < along.length; i++) {
    const a = along[i], b = along[i + 1]
    if (!a.signal.operationalPointId) continue
    const trackIds = way.trackIds.slice(a.piece, b.piece + 1)
    const existing = domain.collections.routes.find(r => r.kind === 'train' && r.startSignalId === a.signal.id && r.targetSignalId === b.signal.id
      && (r.path?.trackIds ?? []).join(',') === trackIds.join(','))
    routes.push({ startSignalId: a.signal.id, targetSignalId: b.signal.id, trackIds, stationId: a.signal.operationalPointId, existingId: existing?.id ?? null })
  }
  if (!routes.length) problems.push('no_routes')
  return { routes, problems }
}

/**
 * The itineraries from one stopping place to another the net offers:
 * [{ way, routes, problems, switches, existingId }] — `switches` those the way
 * runs over, `existingId` the itinerary there is of it already.
 */
export function searchItineraries(net, domain, from, to, options) {
  return waysBetween(net, from, to, options).map(way => {
    const { routes, problems } = routesOfWay(domain, way, to)
    const existing = domain.collections.itineraries.find(it => it.startStoppingPlaceId === from.id && it.targetStoppingPlaceId === to.id
      && it.path.trackIds.join(',') === way.trackIds.join(','))
    return { way, routes, problems, switches: switchesOnRun(net, way.trackIds), existingId: existing?.id ?? null }
  })
}

/**
 * The state with a found way made an itinerary: the train routes it lacks
 * created (ZZ, regular), and the itinerary over them. Returns { state, id }.
 */
export function withItinerary(state, found, from, to, name) {
  const made = found.routes.filter(r => !r.existingId).map(r => ({
    id: newId(), kind: 'train', trainType: 'ZZ', startSignalId: r.startSignalId, targetSignalId: r.targetSignalId, path: { trackIds: r.trackIds },
  }))
  let k = 0
  const routeIds = found.routes.map(r => r.existingId ?? made[k++].id)
  const itinerary = Object.fromEntries(Object.entries({
    id: newId(), name, startStoppingPlaceId: from.id, targetStoppingPlaceId: to.id, routeIds, path: { trackIds: found.way.trackIds },
  }).filter(([, v]) => v !== undefined && v !== ''))
  const c = state.collections
  return { state: { ...state, collections: { ...c, routes: [...c.routes, ...made], itineraries: [...c.itineraries, itinerary] } }, id: itinerary.id }
}
