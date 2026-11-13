import { BOUNDARY, endMarkName, trackLength, tracksOnFrom } from '@rail-across/core/track'
import { pointOf } from '@rail-across/core/document'
import { newId } from './domain.js'
import { isMainSignal } from './itineraries.js'

/**
 * Finding the routes from a signal: the next main signals ahead of it, the
 * way it acts, over every way the net offers — at each switch it meets
 * facing, both of its routes. Each signal met first on a way is a route's
 * target; a way runs past distant and shunting signals, and ends where the
 * net does. And the track sections a route runs through, from where they
 * stand and where it runs.
 */

/**
 * How far a route is looked for ahead of its signal [m], and how many tracks
 * it may run at the most. A line of long block sections has main signals more
 * than 20 km apart; the search ends at the first main signal on every way.
 */
export const REACH = 50000
export const DEEPEST = 80

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')

/** Whether a signal acts the way a run goes on its track. */
export const actsAlong = (s, direction) => s.bothDirections || (s.at.direction ?? 'up') === direction

/**
 * The next main signals ahead of `start`, the way it acts: [{ target,
 * trackIds, length }], nearest first — one for every way that reaches one.
 * `isTarget` says which signals end a way, other than main signals; with
 * `ends`, a way that runs to where the net ends is one too, its target null;
 * with `boundaries`, one that runs to a boundary of the planning area (the
 * layout's end mark, net.endMarks), its target null and `boundary` { mark,
 * at } — the mark and the track's end, the way it runs.
 */
export const nextSignals = (net, signals, start, options) => searchWays(net, signals, start, options).ways

/**
 * nextSignals, and the ways the search gave up on: { ways, cutOff } — each of
 * `cutOff` a way that ran REACH metres or DEEPEST tracks without meeting a
 * target, { trackIds, length } as far as it got.
 */
export function searchWays(net, signals, start, { isTarget = isMainSignal, ends = false, boundaries = false } = {}) {
  if (start?.at?.anchor !== 'station') return { ways: [], cutOff: [] }
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const targets = signals.filter(s => s.id !== start.id && isTarget(s) && s.at?.anchor === 'station')
  const out = []
  const cutOff = []
  const walk = (trackId, direction, from, trackIds, run) => {
    if (run > REACH || trackIds.length > DEEPEST) {
      cutOff.push({ trackIds: trackIds.slice(0, -1), length: run })
      return
    }
    const len = lengths.get(trackId) ?? 0
    // The nearest signal ahead on this track, acting along.
    const ahead = targets
      .filter(s => s.at.trackId === trackId && actsAlong(s, direction) && (direction === 'up' ? s.at.station > from + 0.5 : s.at.station < from - 0.5))
      .sort((a, b) => Math.abs(a.at.station - from) - Math.abs(b.at.station - from))[0]
    if (ahead) {
      out.push({ target: ahead, trackIds, length: run + Math.abs(ahead.at.station - from) })
      return
    }
    const room = direction === 'up' ? len - from : from
    const onward = tracksOnFrom(trackId, away(direction), net.tracks, net.switches)
    const mark = !onward.length && boundaries
      ? (net.endMarks ?? []).find(m => m.kind === BOUNDARY && m.trackId === trackId && m.endpoint === away(direction)) : null
    if (mark) out.push({ target: null, boundary: { mark, at: { anchor: 'station', trackId, station: direction === 'up' ? len : 0, direction } }, trackIds, length: run + room })
    else if (!onward.length && ends) out.push({ target: null, trackIds, length: run + room })
    for (const next of onward) {
      if (trackIds.includes(next.trackId)) continue
      const dir = next.endpoint === 'BEGIN' ? 'up' : 'down'
      walk(next.trackId, dir, dir === 'up' ? 0 : lengths.get(next.trackId) ?? 0, [...trackIds, next.trackId], run + room)
    }
  }
  walk(start.at.trackId, start.at.direction ?? 'up', start.at.station, [start.at.trackId], 0)
  const seen = new Set()
  const ways = out.sort((a, b) => a.length - b.length).filter(r => {
    const key = `${r.target?.id ?? r.boundary?.mark.id}|${r.trackIds.join()}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
  return { ways, cutOff }
}

/** How near a boundary a virtual signal must stand to be its route's target [m]. */
const AT_BOUNDARY = 20

/**
 * The signal a route to a boundary of the planning area ends at (a way of
 * nextSignals with `boundary`): the virtual one there acting the way the way
 * runs (Zugziel Strecke), or null where there is none yet.
 */
export const boundaryTarget = (signals, way) => signals.find(s => s.kind === 'virtual' && s.at?.anchor === 'station'
  && s.at.trackId === way.boundary.at.trackId && actsAlong(s, way.boundary.at.direction)
  && Math.abs(s.at.station - way.boundary.at.station) <= AT_BOUNDARY) ?? null

/**
 * Where trains enter the planning area: a boundary is a fictive signal both
 * ways — the target of a route out to it (boundaryTarget), and the start of
 * one in from it, run at the boundary's speed. [{ mark, signal }] — `signal`
 * the virtual one there acting inward, or, where there is none yet, the one
 * that is to be (`fresh`, a Zugstart ohne Signal named after the boundary).
 */
export function boundaryStarts(net, signals) {
  return (net.endMarks ?? []).filter(m => m.kind === BOUNDARY).flatMap(mark => {
    const track = net.tracks.find(t => t.id === mark.trackId)
    if (!track) return []
    const inward = mark.endpoint === 'END' ? 'down' : 'up'
    const at = { anchor: 'station', trackId: track.id, station: inward === 'down' ? trackLength(track) : 0, direction: inward }
    const there = signals.find(s => s.kind === 'virtual' && s.at?.anchor === 'station' && s.at.trackId === at.trackId
      && actsAlong(s, inward) && Math.abs(s.at.station - at.station) <= AT_BOUNDARY)
    return [{ mark, signal: there ?? { id: `boundary:${mark.id}`, kind: 'virtual', virtualFunction: 'trainStartWithoutSignal', name: endMarkName(mark, track, pointOf(net, track)), at, fresh: true } }]
  })
}

/** Whether a signal starts train routes: a main signal, or a virtual one where trains start — a boundary's among them. */
export const isRouteStart = (s) => isMainSignal(s) || (s.kind === 'virtual' && /^trainStart/.test(s.virtualFunction ?? ''))

/**
 * The virtual signal a route to a boundary ends at, where none is there yet:
 * a train's target on the line (PlanPro Zugziel Strecke) at the track's end,
 * named after the boundary.
 */
export function newBoundaryTarget(net, way) {
  const track = net.tracks.find(t => t.id === way.boundary.at.trackId)
  return { id: newId(), kind: 'virtual', virtualFunction: 'trainTargetLine', name: endMarkName(way.boundary.mark, track, pointOf(net, track)), at: way.boundary.at }
}

/**
 * The stretches of track a run covers from a location, the way it acts,
 * over its tracks to `end` ({ trackId, station }, else the last track's far
 * end): [{ trackId, lo, hi, direction }] in running order. Each track is
 * entered where the one before leaves it.
 */
export function runStretches(net, at, trackIds, end) {
  if (at?.anchor !== 'station' || !trackIds?.length) return []
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const out = []
  let direction = at.direction ?? 'up'
  let from = at.station
  for (let i = 0; i < trackIds.length; i++) {
    const id = trackIds[i]
    const len = lengths.get(id) ?? 0
    const last = i === trackIds.length - 1
    const to = last && end?.trackId === id ? end.station : direction === 'up' ? len : 0
    out.push({ trackId: id, lo: Math.min(from, to), hi: Math.max(from, to), direction })
    if (last) break
    const next = tracksOnFrom(id, away(direction), net.tracks, net.switches).find(n => n.trackId === trackIds[i + 1])
    if (!next) break
    direction = next.endpoint === 'BEGIN' ? 'up' : 'down'
    from = direction === 'up' ? 0 : lengths.get(next.trackId) ?? 0
  }
  return out
}

/** The stretches of track a route runs over, from its start signal to its target. */
export function routeStretches(net, route, signals) {
  const start = signals.find(s => s.id === route.startSignalId)
  const target = signals.find(s => s.id === route.targetSignalId)
  return runStretches(net, start?.at, route.path?.trackIds, target?.at)
}

/** Stretches as the pieces of a way the plan draws, each from where it is entered to where it is left. */
export const wayPieces = (stretches) => stretches.map(s => (s.direction === 'down'
  ? { trackId: s.trackId, from: s.hi, to: s.lo } : { trackId: s.trackId, from: s.lo, to: s.hi }))

/**
 * The track sections a route runs through, in the order it meets them: those
 * whose extent (`extents`, trackSectionExtents) shares more than half a metre
 * of track with its stretches.
 */
export function routeSections(net, route, domain, extents) {
  const stretches = routeStretches(net, route, domain.collections.signals)
  const first = new Map()   // section id → where along the route it is met first [m]
  let at = 0
  for (const st of stretches) {
    for (const s of domain.collections.trackSections) {
      for (const p of extents.get(s.id)?.parts ?? []) {
        if (p.trackId !== st.trackId) continue
        const lo = Math.max(p.from, st.lo), hi = Math.min(p.to, st.hi)
        if (hi - lo <= 0.5) continue
        const met = at + (st.direction === 'up' ? lo - st.lo : st.hi - hi)
        if (!first.has(s.id) || met < first.get(s.id)) first.set(s.id, met)
      }
    }
    at += st.hi - st.lo
  }
  return domain.collections.trackSections.filter(s => first.has(s.id)).sort((a, b) => first.get(a.id) - first.get(b.id))
}
