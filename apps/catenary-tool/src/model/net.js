import { utmToWgs84 } from '@rail-across/core/geometry'
import { pointAtStation, trackLength } from '@rail-across/core/track'

/**
 * The net as the map draws it: every track a line of [lng, lat], looked at
 * every few metres and at every element's ends, each point with its station —
 * so a range is drawn from its stations, and a click finds the track and the
 * station nearest it.
 */

/** A track is looked at every this much [m] along a curve. */
const STEP = 8
/** WGS84's semi-major axis [m]: metres per degree, near enough for picking. */
const A = 6378137

/** The drawing of the tracks: Map id → { id, track, length, points: [{ s, ll: [lng, lat] }] }. */
export function netLines(tracks) {
  const out = new Map()
  for (const track of tracks ?? []) {
    const length = trackLength(track)
    if (!(length > 0)) continue
    const stations = new Set([0, length])
    let s = 0
    for (const el of track.elements ?? []) {
      const n = el.elementType === 'straight' ? 1 : Math.max(2, Math.ceil(el.length / STEP))
      for (let i = 0; i <= n; i++) stations.add(Math.min(length, s + el.length * i / n))
      s += el.length
    }
    const points = [...stations].sort((a, b) => a - b).map(st => ({ s: st, ll: lngLatAt(track, st) })).filter(p => p.ll)
    if (points.length >= 2) out.set(track.id, { id: track.id, track, length, points })
  }
  return out
}

/** The point of a track at a station, as [lng, lat], or null. */
export function lngLatAt(track, station) {
  const p = pointAtStation(track, station)
  return p ? utmToWgs84(p.utm.easting, p.utm.northing, track.epsg) : null
}

/** The part of a track between two stations, as a line of [lng, lat]. */
export function stretch(lines, trackId, from, to) {
  const t = lines.get(trackId)
  if (!t) return []
  const a = Math.max(0, Math.min(from, to)), b = Math.min(t.length, Math.max(from, to))
  return [lngLatAt(t.track, a), ...t.points.filter(p => p.s > a && p.s < b).map(p => p.ll), lngLatAt(t.track, b)].filter(Boolean)
}

/** The bounds of the net: [[west, south], [east, north]], or null. */
export function boundsOf(lines) {
  let w = Infinity, s = Infinity, e = -Infinity, n = -Infinity
  for (const t of lines.values()) {
    for (const { ll: [x, y] } of t.points) { w = Math.min(w, x); e = Math.max(e, x); s = Math.min(s, y); n = Math.max(n, y) }
  }
  return Number.isFinite(w) ? [[w, s], [e, n]] : null
}

/**
 * The track and station nearest a point [lng, lat], within `reach` [m]: {
 * trackId, station, d } or null.
 */
export function nearest(lines, ll, reach = Infinity) {
  const kx = A * Math.cos(ll[1] * Math.PI / 180) * Math.PI / 180, ky = A * Math.PI / 180
  const xy = ([x, y]) => [(x - ll[0]) * kx, (y - ll[1]) * ky]
  let best = null
  for (const t of lines.values()) {
    for (let i = 1; i < t.points.length; i++) {
      const a = xy(t.points[i - 1].ll), b = xy(t.points[i].ll)
      const dx = b[0] - a[0], dy = b[1] - a[1]
      const len2 = dx * dx + dy * dy
      const u = len2 ? Math.max(0, Math.min(1, -(a[0] * dx + a[1] * dy) / len2)) : 0
      const d = Math.hypot(a[0] + u * dx, a[1] + u * dy)
      if (d <= reach && (!best || d < best.d)) {
        best = { trackId: t.id, station: t.points[i - 1].s + u * (t.points[i].s - t.points[i - 1].s), d }
      }
    }
  }
  return best
}
