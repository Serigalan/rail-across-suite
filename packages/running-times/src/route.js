import { aspectOf, aspectSpeed, routeSpeedOf } from '@rail-across/core/signalling'
import { directionOf, findPath, offsetOnPath, pathAlong, pathLength } from './path.js'

const LF7 = aspectOf('Lf_7')

/** The speed an Lf 7 shows [km/h] — on a signal's plate, or as one of its logical signals' —, or null. */
const boardSpeed = (s) => aspectSpeed(LF7, [...(s.frames ?? []), ...(s.logicalSignals ?? []).flatMap(l => l.frames ?? [])]
  .flatMap(f => f.aspects ?? []).find(a => a.aspect === 'Lf_7')?.symbol)

/** A path with a leg run on: the leg's pieces after it, a piece that goes on the last one's track merged into it — the way it ran, from where it ended. */
export function joined(path, leg) {
  const out = path.map(p => ({ ...p }))
  for (const piece of leg) {
    const last = out[out.length - 1]
    if (last && last.trackId === piece.trackId && Math.abs(last.end - piece.start) < 1e-6 && directionOf(last) === directionOf(piece)) last.end = piece.end
    else out.push({ ...piece })
  }
  return out
}

/** The other way along a track. */
const flipped = (direction) => (direction === 'up' ? 'down' : direction === 'down' ? 'up' : null)

/**
 * The location `length` back along a path from its end, the way the path
 * runs there — its start, where the path is shorter: where the tail of a
 * train is whose head stands at the path's end.
 */
export function backAlong(path, length) {
  let left = length
  for (let k = path.length - 1; k >= 0; k--) {
    const p = path[k], len = Math.abs(p.end - p.start)
    if (len >= left - 1e-9) return { trackId: p.trackId, station: p.end >= p.start ? p.end - left : p.end + left, direction: directionOf(p) }
    left -= len
  }
  return path.length ? { trackId: path[0].trackId, station: path[0].start, direction: directionOf(path[0]) } : null
}

/**
 * A train's way through the points it is given — its start, the stops on
 * the way, its end —, never turning back: from a stop on, the direction it
 * arrived in. Returns { path, offsets } — the offsets of the points along
 * the path —, or { failedAt } the index of the first point it found no way to.
 * `weights` as findPath's.
 */
export function routeThrough(record, points, { weights = null } = {}) {
  const found = routeVia(record, points.map(p => [p]), { weights })
  return found.path ? { path: found.path, offsets: found.offsets } : found
}

/** How much of path `a` runs where path `b` runs too, on the same track [m]. */
export function sharedLength(a, b) {
  const onB = new Map()
  for (const p of b) {
    if (!onB.has(p.trackId)) onB.set(p.trackId, [])
    onB.get(p.trackId).push([Math.min(p.start, p.end), Math.max(p.start, p.end)])
  }
  let shared = 0
  for (const p of a) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    for (const [l, h] of onB.get(p.trackId) ?? []) shared += Math.max(0, Math.min(hi, h) - Math.max(lo, l))
  }
  return shared
}

/** How far apart the points are a corridor is drawn by [m]. */
const CORRIDOR_STEP = 10

/** The point [lng, lat] at a station along a drawn line [{ s, lngLat }]. */
function pointOn(pts, s) {
  if (s <= pts[0].s) return pts[0].lngLat
  for (let i = 1; i < pts.length; i++) {
    if (s <= pts[i].s) {
      const a = pts[i - 1], b = pts[i]
      const u = b.s > a.s ? (s - a.s) / (b.s - a.s) : 0
      return [a.lngLat[0] + u * (b.lngLat[0] - a.lngLat[0]), a.lngLat[1] + u * (b.lngLat[1] - a.lngLat[1])]
    }
  }
  return pts[pts.length - 1].lngLat
}

/** Points [lng, lat] every `step` m or less along a path drawn over `lines`, each piece's ends among them. */
function pointsAlong(lines, path, step) {
  const out = []
  for (const p of path) {
    const pts = lines.get(p.trackId)
    if (!pts) continue
    const n = Math.max(1, Math.ceil(Math.abs(p.end - p.start) / step))
    for (let i = 0; i <= n; i++) out.push(pointOn(pts, p.start + (p.end - p.start) * i / n))
  }
  return out
}

/**
 * The corridor `width` m either side of a path drawn over `lines` (trackId →
 * [{ s, lngLat }], as netLines and osrdNet draw a net): { along(trackId) —
 * whether most of a track lies in it, the track beside on a line of two as
 * much as the path's own —, share(path) — how much of another path runs in
 * it, 0 to 1 }.
 */
export function corridorOf(lines, path, width = 25) {
  const pts = pointsAlong(lines, path, CORRIDOR_STEP)
  const kx = 111320 * Math.cos((pts[0]?.[1] ?? 0) * Math.PI / 180), ky = 110540
  // the points in cells `width` wide, a cell's key a number: x and y of the cell
  const cells = new Map()
  const keyOf = (cx, cy) => cx * 4194304 + cy
  let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity
  for (const [lng, lat] of pts) {
    const x = lng * kx, y = lat * ky, key = keyOf(Math.floor(x / width), Math.floor(y / width))
    if (!cells.has(key)) cells.set(key, [])
    cells.get(key).push(x, y)
    x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y)
  }
  const holds = ([lng, lat]) => {
    const x = lng * kx, y = lat * ky
    if (x < x0 - width || x > x1 + width || y < y0 - width || y > y1 + width) return false
    const cx = Math.floor(x / width), cy = Math.floor(y / width)
    for (let i = -1; i <= 1; i++) {
      for (let j = -1; j <= 1; j++) {
        const xy = cells.get(keyOf(cx + i, cy + j))
        if (!xy) continue
        for (let k = 0; k < xy.length; k += 2) if ((xy[k] - x) ** 2 + (xy[k + 1] - y) ** 2 <= width * width) return true
      }
    }
    return false
  }
  const known = new Map()
  const along = (trackId) => {
    if (known.has(trackId)) return known.get(trackId)
    const line = lines.get(trackId)
    let inside = false
    if (line) {
      // points along it, the ends first: decided as soon as most of them are in, or out
      const s0 = line[0].s, len = line[line.length - 1].s - s0
      const n = Math.min(20, Math.max(2, Math.ceil(len / 100)))
      const order = [0, n, ...Array.from({ length: n - 1 }, (_, i) => i + 1)]
      const most = Math.floor((n + 1) / 2) + 1
      let k = 0, out = 0
      for (const i of order) {
        if (holds(pointOn(line, s0 + len * i / n))) k++
        else out++
        if (k >= most || out > n + 1 - most) break
      }
      inside = k >= most
    }
    known.set(trackId, inside)
    return inside
  }
  const share = (other) => {
    const q = pointsAlong(lines, other, 50)
    return q.length ? q.filter(holds).length / q.length : 1
  }
  return { along, share }
}

/** A path's tracks as a corridor: the path's own tracks along it, its share what runs on them. */
function tracksOf(path) {
  const ids = new Set(path.map(p => p.trackId))
  return { along: (trackId) => ids.has(trackId), share: (other) => sharedLength(other, path) / Math.max(pathLength(other), 1e-9) }
}

/**
 * The shortest way and up to `count - 1` others (the penalty method): the
 * tracks along every way found are made dearer by `penalty` and the way is
 * searched again; a way found so is taken where it is at most `stretch`
 * times as long as the shortest and runs at most `overlap` of its length
 * along each one taken before. Along a way is in its corridor (corridorOf)
 * where `lines` draws the net — so that the track beside it on a line of two
 * is no other way —, else on its own tracks. `routeWith(weights)` finds a
 * way with the tracks weighed so ((trackId) → factor; null the shortest): {
 * path, offsets, … } or { failedAt }; `first`, where given, is what it found
 * with none. Returns the ways in the order found, the shortest first — that
 * one alone where it has no path —, `rounds` searches after it at the most.
 */
export function alternativeRoutes(routeWith, { first = null, count = 3, rounds = 8, penalty = 1.5, stretch = 1.6, overlap = 0.7, lines = null, width = 25 } = {}) {
  const shortest = first ?? routeWith(null)
  if (!shortest?.path) return [shortest]
  const along = (path) => (lines ? corridorOf(lines, path, width) : tracksOf(path))
  const found = [{ way: shortest, corridor: along(shortest.path) }]
  const weighed = [found[0].corridor]   // every way found, taken or not
  const most = pathLength(shortest.path) * stretch
  for (let i = 0; i < rounds && found.length < count; i++) {
    const now = [...weighed]
    const way = routeWith((trackId) => now.reduce((f, c) => (c.along(trackId) ? f * penalty : f), 1))
    if (!way?.path) break
    const corridor = along(way.path)
    weighed.push(corridor)
    if (pathLength(way.path) > most) continue
    if (found.every(f => f.corridor.share(way.path) <= overlap)) found.push({ way, corridor })
  }
  return found.map(f => f.way)
}

/**
 * A train's way through points each of which may be one of several places —
 * the tracks of a station, a stopping place left either way —, never turning
 * back. At each point the place reached the shortest way is tried first — at
 * the start, the one the first leg is shortest from —, the others where it
 * leads nowhere further on (`tries` legs at the most). Returns { path, offsets, chosen } —
 * `chosen` the place taken at each point —, or { failedAt } the furthest point
 * it found no way to (0 where the start has no place).
 *
 * `legPath(i, from, direction, to)`, where given, is asked first for the leg
 * to point `i`: a path, nothing where that leg takes the shortest way, or
 * false where it has no way there.
 *
 * At a point in `reverses` (indices) the train turns: the leg on runs the
 * other way, from where its tail stood — `length` [m] back along the way
 * (backAlong) —, so that the path jumps there: the stretch the train stood on
 * is not run again.
 *
 * `weights`, where given, weighs the tracks of the shortest way (findPath).
 */
export function routeVia(record, points, { tries = 200, legPath = null, reverses = [], length = 0, weights = null } = {}) {
  const turns = new Set(reverses)
  let failedAt = 1
  let left = tries
  const legs = new Map()
  const legOf = (i, from, direction, to) => {
    const key = `${i}|${from.trackId}:${from.station}:${direction}>${to.trackId}:${to.station}:${to.direction}`
    if (!legs.has(key)) legs.set(key, legPath?.(i, from, direction, to) ?? findPath(record, { ...from, direction }, to, { weights }))
    return legs.get(key)
  }
  const go = (i, path, offsets, chosen, direction) => {
    if (i === points.length) return { path, offsets, chosen }
    // where it turns, it leaves from its tail, the other way
    const turned = turns.has(i - 1) && path.length
    const from = turned ? backAlong(path, length) : chosen[i - 1]
    const leaving = turned ? flipped(direction) : direction
    const options = (points[i] ?? [])
      .map(to => ({ to, leg: legOf(i, from, leaving, to) }))
      .filter(o => o.leg)
      .sort((a, b) => pathLength(a.leg) - pathLength(b.leg))
    if (!options.length) { failedAt = Math.max(failedAt, i); return null }
    for (const { to, leg } of options) {
      if (left-- <= 0) return null
      const next = joined(path, leg)
      const found = go(i + 1, next, [...offsets, pathLength(next)], [...chosen, to], leg.length ? directionOf(leg[leg.length - 1]) : leaving)
      if (found) return found
    }
    return null
  }
  // the start whose first leg is the shortest first: one place left either way is two
  const first = (start) => Math.min(...(points[1] ?? []).map(to => legOf(1, start, start.direction ?? null, to)).filter(Boolean).map(pathLength))
  const starts = points.length > 1 ? (points[0] ?? []).map(s => ({ s, d: first(s) })).sort((a, b) => a.d - b.d).map(x => x.s) : points[0] ?? []
  for (const start of starts) {
    const found = go(1, [], [0], [start], start.direction ?? null)
    if (found) return found
    if (left <= 0) break
  }
  return { failedAt: points[0]?.length ? failedAt : 0 }
}

/**
 * A leg over the routes it is told to take (a dispatcher's choice, where the
 * route it would run is held): from `from`, left the way `direction` says, to
 * each route's start signal the shortest way, along the route's tracks to its
 * target, and on to `to` the shortest way — or, where `to` lies on the last
 * route, along it there. `routes` [{ at, trackIds, target }] in running
 * order — `at` the start signal's location, `target` the target's, each with
 * the way it acts. A path, or null where it does not lead there.
 */
export function legThrough(record, from, direction, routes, to) {
  let path = []
  let at = { ...from, direction: direction ?? from.direction ?? null }
  for (const [i, r] of routes.entries()) {
    const head = findPath(record, at, r.at)
    if (!head) return null
    const k = r.trackIds.indexOf(to.trackId)
    if (i === routes.length - 1 && k >= 0) {
      const along = pathAlong(record, r.trackIds.slice(0, k + 1), r.at, to, r.at.direction)
      if (along) return joined(joined(path, head), along)
    }
    const run = pathAlong(record, r.trackIds, r.at, r.target, r.at.direction)
    if (!run) return null
    path = joined(joined(path, head), run)
    at = { ...r.target, direction: run.length ? directionOf(run[run.length - 1]) : r.target.direction }
  }
  const tail = findPath(record, at, to)
  return tail ? joined(path, tail) : null
}

/**
 * A path cut where the train turns (`turns`, offsets along it): [{ path,
 * offset }] — each part with the offset it begins at —, each run one way.
 */
export function turnsApart(path, turns = []) {
  const cuts = [...turns].filter(t => t > 1e-6).sort((a, b) => a - b)
  const out = [{ path: [], offset: 0 }]
  let at = 0
  for (const p of path) {
    if (cuts.length && at >= cuts[0] - 1e-6) { cuts.shift(); out.push({ path: [], offset: at }) }
    out[out.length - 1].path.push(p)
    at += Math.abs(p.end - p.start)
  }
  return out.filter(part => part.path.length)
}

/**
 * The speeds the signals signal along a path, as speed limits: for each
 * route of the signalling domain that the path runs along — its start
 * signal passed in the way it acts, its tracks those of the path —, the
 * speed its start signal states for it (a logical signal's
 * signallingSpeeds; from a planning boundary, the boundary's), from the
 * start signal to the target signal; where a
 * speed signal Lf 7 stands on it in the way it runs, the route's speed
 * holds up to it, and what the Lf 7 shows from there on.
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
    // From a planning boundary — a virtual start there — at the boundary's speed.
    const speed = routeSpeedOf(record.signalling, route, record)
    const ids = route.path?.trackIds ?? []
    const first = trackIds.indexOf(ids[0])
    if (first < 0 || !ids.every((id, i) => trackIds[first + i] === id)) continue
    const begin = offsetOnPath(path, start.at), end = offsetOnPath(path, target.at)
    if (begin == null || end == null || !(end > begin)) continue
    // the signal acts in the way the path runs there
    const piece = path.find(p => p.trackId === start.at.trackId)
    if (piece && start.at.direction && directionOf(piece) !== start.at.direction) continue
    // The Lf 7 on the route, the way it runs, cut it into sections.
    const marks = [{ at: begin, speed }]
    for (const s of signals.values()) {
      const v = s.kind === 'real' && s.at?.anchor === 'station' && ids.includes(s.at.trackId) ? boardSpeed(s) : null
      if (!(v > 0)) continue
      const at = offsetOnPath(path, s.at)
      if (at == null || at <= begin + 0.5 || at >= end - 0.5) continue
      const on = path.find(p => p.trackId === s.at.trackId)
      if (!s.bothDirections && s.at.direction && on && directionOf(on) !== s.at.direction) continue
      marks.push({ at, speed: v })
    }
    marks.sort((a, b) => a.at - b.at)
    marks.forEach((m, i) => {
      if (m.speed > 0) out.push({ begin: m.at, end: marks[i + 1]?.at ?? end, speed: m.speed / 3.6, source: 'signal' })
    })
  }
  return out
}
