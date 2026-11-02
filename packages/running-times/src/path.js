import { heightAt, trackLength, tracksOnFrom } from '@rail-across/core/track'

/**
 * A train's path over the net, and what the physics needs of it: its length,
 * the grades along it — the heights' and the curves' — and the speeds the
 * layout allows. A location is { trackId, station, direction } — `direction`
 * up (with the stationing) or down, or none where either will do. A path is
 * [{ trackId, start, end }] in running order: each piece is run from `start`
 * to `end`, up where end > start.
 */

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')
const pieceLength = (p) => Math.abs(p.end - p.start)
export const directionOf = (p) => (p.end >= p.start ? 'up' : 'down')

/**
 * The shortest path from one location to another — over switches along
 * their routes, links and joints (tracksOnFrom) —, or null where none leads
 * there. Where a location has no direction, both are tried.
 */
export function findPath(record, from, to) {
  const tracks = record?.tracks ?? []
  const switches = record?.switches ?? []
  const lengths = new Map(tracks.map(t => [t.id, trackLength(t)]))
  if (!lengths.has(from?.trackId) || !lengths.has(to?.trackId)) return null
  const fits = (want, direction) => !want || want === direction
  let best = null
  for (const direction of from.direction ? [from.direction] : ['up', 'down']) {
    // Dijkstra over (track, the way it is run); the cost what is run.
    const queue = []
    const seen = new Set()
    const reach = (path, cost) => { if (!best || cost < best.cost) best = { path, cost } }
    const tryTarget = (trackId, dir, enter, path, cost) => {
      if (trackId !== to.trackId || !fits(to.direction, dir)) return
      const ahead = dir === 'up' ? to.station - enter : enter - to.station
      if (ahead >= 0) reach([...path, { trackId, start: enter, end: to.station }], cost + ahead)
    }
    tryTarget(from.trackId, direction, from.station, [], 0)
    const leaveAt = direction === 'up' ? lengths.get(from.trackId) : 0
    queue.push({ trackId: from.trackId, direction, cost: Math.abs(leaveAt - from.station), path: [{ trackId: from.trackId, start: from.station, end: leaveAt }] })
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
        if (cur.path.length > 400) continue
        queue.push({ trackId: next.trackId, direction: dir, cost: cur.cost + length, path: [...cur.path, { trackId: next.trackId, start: enter, end: length - enter }] })
      }
    }
  }
  if (!best) return null
  return best.path.filter(p => pieceLength(p) > 1e-6)
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

/** The stretches of a track and what each holds: [{ from, to, speed, radius }] along its stationing. */
function elementSpans(track) {
  const out = []
  let s = 0
  for (const el of track.elements ?? []) {
    const len = el.length ?? 0
    if (len > 0) out.push({ from: s, to: s + len, speed: el.speed > 0 ? el.speed : null, radius: el.elementType === 1 ? el.radius : null })
    s += len
  }
  return out
}

/** Merges neighbouring ranges of the same value. */
function merged(ranges, same = (a, b) => a.value === b.value) {
  const out = []
  for (const r of ranges) {
    const last = out[out.length - 1]
    if (last && same(last, r) && Math.abs(last.end - r.begin) < 1e-6) last.end = r.end
    else out.push({ ...r })
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
