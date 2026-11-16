import { utmToWgs84 } from '@rail-across/core/geometry'
import { pointAtStation, trackEndAt, trackLength } from '@rail-across/core/track'

/**
 * The net as the map draws it: every track a line of [lng, lat], looked at
 * every few metres and at every element's ends.
 */

/** A track is looked at every this much [m] along a curve. */
const STEP = 8

/** The drawing of the tracks: Map id → [[lng, lat], …]. */
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
    const line = [...stations].sort((a, b) => a - b).map(st => {
      const p = pointAtStation(track, st)
      return p ? utmToWgs84(p.utm.easting, p.utm.northing, track.epsg) : null
    }).filter(Boolean)
    if (line.length >= 2) out.set(track.id, line)
  }
  return out
}

/** Where a switch or connector lies: the end of the track at its port A, as [lng, lat], or null. */
export function switchPoint(sw, tracks) {
  const port = sw.ports?.A
  const track = port && tracks.find(t => t.id === port.trackId)
  const end = track && trackEndAt(track, port.endpoint)
  return end ? utmToWgs84(end.easting, end.northing, end.epsg) : null
}

/** The bounds of the net: [[west, south], [east, north]], or null. */
export function boundsOf(lines) {
  let w = Infinity, s = Infinity, e = -Infinity, n = -Infinity
  for (const line of lines.values()) {
    for (const [x, y] of line) { w = Math.min(w, x); e = Math.max(e, x); s = Math.min(s, y); n = Math.max(n, y) }
  }
  return Number.isFinite(w) ? [[w, s], [e, n]] : null
}
