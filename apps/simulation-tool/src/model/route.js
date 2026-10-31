import { directionOf, findPath, offsetOnPath, pathLength } from './path.js'

/**
 * A train's way through the points it is given — its start, the stops on
 * the way, its end —, never turning back: from a stop on, the direction it
 * arrived in. Returns { path, offsets } — the offsets of the points along
 * the path —, or { failedAt } the index of the first point it found no way to.
 */
export function routeThrough(record, points) {
  const path = []
  const offsets = [0]
  let direction = points[0]?.direction ?? null
  for (let i = 1; i < points.length; i++) {
    const from = { ...points[i - 1], direction }
    const leg = findPath(record, from, points[i])
    if (!leg) return { failedAt: i }
    for (const piece of leg) {
      const last = path[path.length - 1]
      if (last && last.trackId === piece.trackId && Math.abs(last.end - piece.start) < 1e-6 && directionOf(last) === directionOf(piece)) last.end = piece.end
      else path.push({ ...piece })
    }
    offsets.push(pathLength(path))
    if (path.length) direction = directionOf(path[path.length - 1])
  }
  return { path, offsets }
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
