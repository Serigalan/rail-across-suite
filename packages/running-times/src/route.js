import { directionOf, findPath, offsetOnPath, pathLength } from './path.js'

/** A path with a leg run on: the leg's pieces after it, a piece that goes on the last one's track merged into it. */
function joined(path, leg) {
  const out = path.map(p => ({ ...p }))
  for (const piece of leg) {
    const last = out[out.length - 1]
    if (last && last.trackId === piece.trackId && Math.abs(last.end - piece.start) < 1e-6 && directionOf(last) === directionOf(piece)) last.end = piece.end
    else out.push({ ...piece })
  }
  return out
}

/**
 * A train's way through the points it is given — its start, the stops on
 * the way, its end —, never turning back: from a stop on, the direction it
 * arrived in. Returns { path, offsets } — the offsets of the points along
 * the path —, or { failedAt } the index of the first point it found no way to.
 */
export function routeThrough(record, points) {
  const found = routeVia(record, points.map(p => [p]))
  return found.path ? { path: found.path, offsets: found.offsets } : found
}

/**
 * A train's way through points each of which may be one of several places —
 * the tracks of a station —, never turning back. At each point the place
 * reached the shortest way is tried first, the others where it leads nowhere
 * further on (`tries` legs at the most). Returns { path, offsets, chosen } —
 * `chosen` the place taken at each point —, or { failedAt } the furthest point
 * it found no way to (0 where the start has no place).
 */
export function routeVia(record, points, { tries = 200 } = {}) {
  let failedAt = 1
  let left = tries
  const legs = new Map()
  const legOf = (from, direction, to) => {
    const key = `${from.trackId}:${from.station}:${direction}>${to.trackId}:${to.station}:${to.direction}`
    if (!legs.has(key)) legs.set(key, findPath(record, { ...from, direction }, to))
    return legs.get(key)
  }
  const go = (i, path, offsets, chosen, direction) => {
    if (i === points.length) return { path, offsets, chosen }
    const options = (points[i] ?? [])
      .map(to => ({ to, leg: legOf(chosen[i - 1], direction, to) }))
      .filter(o => o.leg)
      .sort((a, b) => pathLength(a.leg) - pathLength(b.leg))
    if (!options.length) { failedAt = Math.max(failedAt, i); return null }
    for (const { to, leg } of options) {
      if (left-- <= 0) return null
      const next = joined(path, leg)
      const found = go(i + 1, next, [...offsets, pathLength(next)], [...chosen, to], next.length ? directionOf(next[next.length - 1]) : direction)
      if (found) return found
    }
    return null
  }
  for (const start of points[0] ?? []) {
    const found = go(1, [], [0], [start], start.direction ?? null)
    if (found) return found
    if (left <= 0) break
  }
  return { failedAt: points[0]?.length ? failedAt : 0 }
}

/**
 * The speeds the signals signal along a path, as speed limits: for each
 * route of the signalling domain that the path runs along — its start
 * signal passed in the way it acts, its tracks those of the path —, the
 * speed its start signal states for it (a logical signal's
 * signallingSpeeds), from the start signal to the target signal.
 */
export function signalledSpeeds(record, path) {
  const c = record?.signalling?.collections
  if (!c?.routes?.length) return []
  const signals = new Map((c.signals ?? []).map(s => [s.id, s]))
  const trackIds = path.map(p => p.trackId)
  const out = []
  for (const route of c.routes) {
    if (route.kind !== 'train') continue
    const start = signals.get(route.startSignalId), target = signals.get(route.targetSignalId)
    if (start?.at?.anchor !== 'station' || target?.at?.anchor !== 'station') continue
    const speed = (start.logicalSignals ?? []).flatMap(l => l.signallingSpeeds ?? []).find(s => s.routeId === route.id)?.speed
    if (!(speed > 0)) continue
    const ids = route.path?.trackIds ?? []
    const first = trackIds.indexOf(ids[0])
    if (first < 0 || !ids.every((id, i) => trackIds[first + i] === id)) continue
    const begin = offsetOnPath(path, start.at), end = offsetOnPath(path, target.at)
    if (begin == null || end == null || !(end > begin)) continue
    // the signal acts in the way the path runs there
    const piece = path.find(p => p.trackId === start.at.trackId)
    if (piece && start.at.direction && directionOf(piece) !== start.at.direction) continue
    out.push({ begin, end, speed: speed / 3.6, source: 'signal' })
  }
  return out
}
