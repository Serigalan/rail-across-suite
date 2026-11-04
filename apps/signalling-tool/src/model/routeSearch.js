import { trackLength, tracksOnFrom } from '@rail-across/core/track'
import { isMainSignal } from './itineraries.js'

/**
 * Finding the routes from a signal: the next main signals ahead of it, the
 * way it acts, over every way the net offers — at each switch it meets
 * facing, both of its routes. Each signal met first on a way is a route's
 * target; a way runs past distant and shunting signals, and ends where the
 * net does. And the track sections a route runs through, from where they
 * stand and where it runs.
 */

/** How far a route is looked for ahead of its signal [m], and how many tracks it may run at the most. */
const REACH = 20000
const DEEPEST = 80

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')

/** Whether a signal acts the way a run goes on its track. */
const actsAlong = (s, direction) => s.bothDirections || (s.at.direction ?? 'up') === direction

/**
 * The next main signals ahead of `start`, the way it acts: [{ target,
 * trackIds, length }], nearest first — one for every way that reaches one.
 */
export function nextSignals(net, signals, start) {
  if (start?.at?.anchor !== 'station') return []
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const targets = signals.filter(s => s.id !== start.id && isMainSignal(s) && s.at?.anchor === 'station')
  const out = []
  const walk = (trackId, direction, from, trackIds, run) => {
    if (run > REACH || trackIds.length > DEEPEST) return
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
    for (const next of tracksOnFrom(trackId, away(direction), net.tracks, net.switches)) {
      if (trackIds.includes(next.trackId)) continue
      const dir = next.endpoint === 'BEGIN' ? 'up' : 'down'
      walk(next.trackId, dir, dir === 'up' ? 0 : lengths.get(next.trackId) ?? 0, [...trackIds, next.trackId], run + room)
    }
  }
  walk(start.at.trackId, start.at.direction ?? 'up', start.at.station, [start.at.trackId], 0)
  const seen = new Set()
  return out.sort((a, b) => a.length - b.length).filter(r => {
    const key = `${r.target.id}|${r.trackIds.join()}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
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
