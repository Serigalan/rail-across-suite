import { heightAt, trackLength, tracksOnFrom } from '@rail-across/core/track'

/**
 * A train's path over the net, and what the physics needs of it: its length,
 * the grades along it — the heights' and the curves', transitions too — and the speeds the
 * layout allows. A location is { trackId, station, direction } — `direction`
 * up (with the stationing) or down, or none where either will do. A path is
 * [{ trackId, start, end }] in running order: each piece is run from `start`
 * to `end`, up where end > start.
 *
 * What is closed (decision 0014) a record states as `closed`, [{ trackId, lo,
 * hi }]: no path runs over a closed stretch — more than half a metre of it —,
 * a train may stop before it or start behind it.
 */

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')
const pieceLength = (p) => Math.abs(p.end - p.start)
export const directionOf = (p) => (p.end >= p.start ? 'up' : 'down')

/** How far a path may run into a closed stretch and still be open [m]. */
const CLOSED_SHARE = 0.5

/** Whether the stretch of a track from `a` to `b` runs over none of what `closed` holds. */
const openWay = (closed, trackId, a, b) => !closed.some(c => c.trackId === trackId && Math.min(Math.max(a, b), c.hi) - Math.max(Math.min(a, b), c.lo) > CLOSED_SHARE)

/** Whether a path runs over none of what a record states as closed. */
export const pathOpen = (record, path) => {
  const closed = record?.closed ?? []
  return !closed.length || path.every(p => openWay(closed, p.trackId, p.start, p.end))
}

/**
 * The shortest path from one location to another — over switches along
 * their routes, links and joints (tracksOnFrom) —, or null where none leads
 * there. Where a location has no direction, both are tried. `weights`
 * ((trackId) → a factor of 1 or more) makes a track dearer than its length:
 * the search for another way.
 */
export function findPath(record, from, to, { weights = null } = {}) {
  const tracks = record?.tracks ?? []
  const switches = record?.switches ?? []
  const lengths = new Map(tracks.map(t => [t.id, trackLength(t)]))
  if (!lengths.has(from?.trackId) || !lengths.has(to?.trackId)) return null
  const fits = (want, direction) => !want || want === direction
  const closed = record?.closed ?? []
  const open = (trackId, a, b) => !closed.length || openWay(closed, trackId, a, b)
  const factor = (trackId) => (weights ? weights(trackId) : 1)
  let best = null
  for (const direction of from.direction ? [from.direction] : ['up', 'down']) {
    // Dijkstra over (track, the way it is run); the cost what is run.
    const queue = []
    const seen = new Set()
    const reach = (path, cost) => { if (!best || cost < best.cost) best = { path, cost } }
    const tryTarget = (trackId, dir, enter, path, cost) => {
      if (trackId !== to.trackId || !fits(to.direction, dir)) return
      const ahead = dir === 'up' ? to.station - enter : enter - to.station
      if (ahead >= 0 && open(trackId, enter, to.station)) reach([...path, { trackId, start: enter, end: to.station }], cost + ahead * factor(trackId))
    }
    tryTarget(from.trackId, direction, from.station, [], 0)
    const leaveAt = direction === 'up' ? lengths.get(from.trackId) : 0
    if (open(from.trackId, from.station, leaveAt)) queue.push({ trackId: from.trackId, direction, cost: Math.abs(leaveAt - from.station) * factor(from.trackId), path: [{ trackId: from.trackId, start: from.station, end: leaveAt }] })
    while (queue.length) {
      queue.sort((a, b) => a.cost - b.cost)
      const cur = queue.shift()
      if (best && cur.cost >= best.cost) break
      const key = `${cur.trackId}:${cur.direction}`
      if (seen.has(key)) continue
      seen.add(key)
      for (const next of tracksOnFrom(cur.trackId, away(cur.direction), tracks, switches)) {
        const length = lengths.get(next.trackId)
        if (length == null) continue
        const dir = next.endpoint === 'BEGIN' ? 'up' : 'down'
        const enter = dir === 'up' ? 0 : length
        tryTarget(next.trackId, dir, enter, cur.path, cur.cost)
        if (cur.path.length > 400 || !open(next.trackId, 0, length)) continue
        queue.push({ trackId: next.trackId, direction: dir, cost: cur.cost + length * factor(next.trackId), path: [...cur.path, { trackId: next.trackId, start: enter, end: length - enter }] })
      }
    }
  }
  if (!best) return null
  return best.path.filter(p => pieceLength(p) > 1e-6)
}

/**
 * The path from one location to another along a given run of tracks — the
 * way a timetable prefers, an itinerary's —, or null where the run does not
 * lead from the one to the other: its first track the start's, its last the
 * target's, each meeting the next at an end (tracksOnFrom), the target ahead.
 * `direction`, where given, is the way the start must be left: never turning
 * back.
 */
export function pathAlong(record, trackIds, from, to, direction = null) {
  const tracks = record?.tracks ?? []
  const switches = record?.switches ?? []
  const lengths = new Map(tracks.map(t => [t.id, trackLength(t)]))
  if (!trackIds?.length || trackIds[0] !== from?.trackId || trackIds[trackIds.length - 1] !== to?.trackId) return null
  if (trackIds.some(id => !lengths.has(id))) return null
  if (trackIds.length === 1) {
    const dir = to.station >= from.station ? 'up' : 'down'
    if ((direction && direction !== dir) || (to.direction && to.direction !== dir)) return null
    const one = [{ trackId: from.trackId, start: from.station, end: to.station }].filter(p => pieceLength(p) > 1e-6)
    return pathOpen(record, one) ? one : null
  }
  // The way the first track is left: by the end the second meets it at.
  const leaveBy = ['END', 'BEGIN'].find(end => tracksOnFrom(trackIds[0], end, tracks, switches).some(n => n.trackId === trackIds[1]))
  if (!leaveBy) return null
  let dir = leaveBy === 'END' ? 'up' : 'down'
  if (direction && direction !== dir) return null
  const path = [{ trackId: trackIds[0], start: from.station, end: dir === 'up' ? lengths.get(trackIds[0]) : 0 }]
  for (let i = 1; i < trackIds.length; i++) {
    const next = tracksOnFrom(trackIds[i - 1], away(dir), tracks, switches).find(n => n.trackId === trackIds[i])
    if (!next) return null
    dir = next.endpoint === 'BEGIN' ? 'up' : 'down'
    const enter = dir === 'up' ? 0 : lengths.get(trackIds[i])
    const last = i === trackIds.length - 1
    path.push({ trackId: trackIds[i], start: enter, end: last ? to.station : lengths.get(trackIds[i]) - enter })
  }
  if (to.direction && to.direction !== dir) return null
  const out = path.filter(p => pieceLength(p) > 1e-6)
  return pathOpen(record, out) ? out : null
}

/** The length of a path [m]. */
export const pathLength = (path) => path.reduce((sum, p) => sum + pieceLength(p), 0)

/** Where a location of the net lies along a path [m], or null where the path does not pass it. */
export function offsetOnPath(path, loc) {
  let at = 0
  for (const p of path) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    if (p.trackId === loc.trackId && loc.station >= lo - 1e-6 && loc.station <= hi + 1e-6) {
      return at + Math.abs(loc.station - p.start)
    }
    at += pieceLength(p)
  }
  return null
}

/** The location at an offset along a path. */
export function locationAt(path, offset) {
  let at = 0
  for (const p of path) {
    const len = pieceLength(p)
    if (offset <= at + len || p === path[path.length - 1]) {
      const along = Math.min(Math.max(offset - at, 0), len)
      return { trackId: p.trackId, station: p.end >= p.start ? p.start + along : p.start - along, direction: directionOf(p) }
    }
    at += len
  }
  return null
}

/**
 * The resistance of a curve as an equivalent grade [‰], after Röckl for
 * standard gauge: 650 / (r − 55), for radii from 300 m; below that the
 * steeper 500 / (r − 30).
 */
export function curveResistance(radius) {
  const r = Math.abs(radius)
  if (!(r > 0) || !Number.isFinite(r)) return 0
  if (r >= 300) return 650 / (r - 55)
  return r > 30 ? 500 / (r - 30) : 0
}

/** The longest piece a transition is cut into for its resistance [m]. */
const TRANSITION_PIECE = 10

/**
 * The radius of a transition at `s` metres into it [m], unsigned, or null
 * where it is straight: its curvature runs from 1/r1 to 1/r2 — linearly on
 * a clothoid, as 3(s/L)² − 2(s/L)³ on a Bloss curve (geometry/clothoidUtils).
 */
export function transitionRadius(el, s) {
  const k1 = el.r1 ? 1 / el.r1 : 0, k2 = el.r2 ? 1 / el.r2 : 0
  const t = el.length > 0 ? Math.min(1, Math.max(0, s / el.length)) : 0
  const k = k1 + (k2 - k1) * (el.transitionType === 'bloss' ? 3 * t * t - 2 * t ** 3 : t)
  return Math.abs(k) > 1e-12 ? 1 / Math.abs(k) : null
}

/**
 * The stretches of a track and what each holds: [{ from, to, speed, radius }]
 * along its stationing — a transition in pieces of at most
 * TRANSITION_PIECE, each with the radius at its middle.
 */
function elementSpans(track) {
  const out = []
  let s = 0
  for (const el of track.elements ?? []) {
    const len = el.length ?? 0
    const speed = el.speed > 0 ? el.speed : null
    if (len > 0 && el.elementType === 'transition') {
      const n = Math.ceil(len / TRANSITION_PIECE)
      for (let i = 0; i < n; i++) {
        out.push({ from: s + len * i / n, to: s + len * (i + 1) / n, speed, radius: transitionRadius(el, len * (i + 0.5) / n) })
      }
    } else if (len > 0) out.push({ from: s, to: s + len, speed, radius: el.elementType === 'arc' ? el.radius : null })
    s += len
  }
  return out
}

/** Merges neighbouring ranges of the same value. */
export function merged(ranges, same = (a, b) => a.value === b.value) {
  const out = []
  for (const r of ranges) {
    const last = out[out.length - 1]
    if (last && same(last, r) && Math.abs(last.end - r.begin) < 1e-6) last.end = r.end
    else out.push({ ...r })
  }
  return out
}

/**
 * The heights along a path, for its profile: [{ position, height }] in
 * running order at the ends of its pieces and the height points between —
 * the heights between them straight, as the grades are taken. A track
 * without heights gives null there: a gap the profile leaves open.
 */
export function heightProfile(record, path) {
  const tracks = new Map((record?.tracks ?? []).map(t => [t.id, t]))
  const out = []
  let at = 0
  for (const p of path ?? []) {
    const track = tracks.get(p.trackId)
    const heights = track?.heights?.length >= 2 ? track.heights : null
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const stations = [lo, ...(heights ?? []).map(h => h.station).filter(s => s > lo && s < hi), hi]
    if (p.end < p.start) stations.reverse()
    for (const s of stations) out.push({ position: at + Math.abs(s - p.start), height: heights ? heightAt(heights, s) : null })
    at += pieceLength(p)
  }
  return out
}

/**
 * What the physics needs of a path: { length, gradients, speedLimits, gaps }
 * — `gradients` [{ begin, end, value }] in ‰ along the path, rising
 * positive, the curves' resistance counted in; `speedLimits` [{ begin, end,
 * speed (m/s), source }] where the layout states a design speed; `gaps` the
 * stretches without heights or without a speed, for the user to know.
 */
export function pathProperties(record, path) {
  const tracks = new Map((record?.tracks ?? []).map(t => [t.id, t]))
  const gradients = [], speeds = [], noHeights = [], noSpeed = []
  let at = 0
  for (const p of path) {
    const track = tracks.get(p.trackId)
    const len = pieceLength(p)
    const up = p.end >= p.start
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    // along the path from a station of the track
    const along = (station) => at + Math.abs(station - p.start)
    const heights = track?.heights?.length >= 2 ? track.heights : null
    if (!heights) noHeights.push({ begin: at, end: at + len })
    // the cuts of this piece: height points and element ends
    const cuts = new Set([lo, hi])
    for (const h of heights ?? []) if (h.station > lo && h.station < hi) cuts.add(h.station)
    const spans = track ? elementSpans(track) : []
    for (const sp of spans) for (const s of [sp.from, sp.to]) if (s > lo && s < hi) cuts.add(s)
    const stations = [...cuts].sort((a, b) => a - b)
    for (let i = 0; i + 1 < stations.length; i++) {
      const a = stations[i], b = stations[i + 1]
      if (b - a < 1e-6) continue
      const mid = (a + b) / 2
      const span = spans.find(sp => mid >= sp.from && mid <= sp.to)
      let grade = 0
      if (heights) {
        const rise = (heightAt(heights, b) - heightAt(heights, a)) / (b - a) * 1000
        grade = up ? rise : -rise
      }
      grade += curveResistance(span?.radius)
      const [begin, end] = up ? [along(a), along(b)] : [along(b), along(a)]
      gradients.push({ begin, end, value: Math.round(grade * 1000) / 1000 })
      if (span?.speed) speeds.push({ begin, end, value: span.speed })
      else noSpeed.push({ begin, end })
    }
    at += len
  }
  const byBegin = (a, b) => a.begin - b.begin
  return {
    length: at,
    gradients: merged(gradients.sort(byBegin)).filter(g => g.value !== 0),
    speedLimits: merged(speeds.sort(byBegin)).map(s => ({ begin: s.begin, end: s.end, speed: s.value / 3.6, source: 'design' })),
    gaps: { heights: merged(noHeights.map(r => ({ ...r, value: 0 }))), speed: merged(noSpeed.sort(byBegin).map(r => ({ ...r, value: 0 }))) },
  }
}
