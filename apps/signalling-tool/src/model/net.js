import { utmToWgs84 } from '@rail-across/core/geometry'
import { pointAtStation, stationOnTrack, trackLength } from '@rail-across/core/track'

/**
 * The net as the plan draws it: every track a polyline in one local frame of
 * metres — east right, north up — whatever plane each track is stated in.
 * Points go through WGS84 and are laid flat around the net's middle, in metres
 * of the ellipsoid there; at the scale of a station that is exact to well below
 * what a drawing shows.
 */

/** A track is looked at every this much [m], and at every element's ends. */
const STEP = 8
/** WGS84: semi-major axis [m], first eccentricity squared. */
const A = 6378137
const E2 = 0.00669437999014

/** The local frame of a set of tracks: (lng, lat) → [x, y] in metres, and back. */
export function frameOf(tracks) {
  let lng = 0, lat = 0, n = 0
  for (const t of tracks) {
    const p = pointAtStation(t, 0)
    if (!p) continue
    const [x, y] = utmToWgs84(p.utm.easting, p.utm.northing, t.epsg)
    lng += x; lat += y; n++
  }
  const lng0 = n ? lng / n : 0, lat0 = n ? lat / n : 0
  // Metres per degree there: the prime vertical's radius across, the meridian's along.
  const sin = Math.sin(lat0 * Math.PI / 180)
  const w = 1 - E2 * sin * sin
  const kx = (A / Math.sqrt(w)) * Math.cos(lat0 * Math.PI / 180) * Math.PI / 180
  const ky = (A * (1 - E2) / (w * Math.sqrt(w))) * Math.PI / 180
  return {
    toLocal: ([x, y]) => [(x - lng0) * kx, (y - lat0) * ky],
    toLngLat: ([x, y]) => [x / kx + lng0, y / ky + lat0],
  }
}

/** A point of a track in the frame, with its bearing (degrees from north, clockwise), or null. */
function localPoint(frame, track, station) {
  const p = pointAtStation(track, station)
  if (!p) return null
  return { xy: frame.toLocal(utmToWgs84(p.utm.easting, p.utm.northing, track.epsg)), bearing: p.bearing }
}

/**
 * The drawing of a net: { frame, tracks: Map id → { id, track, length, line:
 * [[x, y]], stations: [s] }, bounds: [x0, y0, x1, y1] }.
 */
export function netDrawing(tracks) {
  const frame = frameOf(tracks)
  const out = new Map()
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
  for (const track of tracks) {
    const length = trackLength(track)
    if (!(length > 0)) continue
    const stations = new Set([0, length])
    let s = 0
    for (const el of track.elements ?? []) {
      const n = el.elementType === 0 ? 1 : Math.max(2, Math.ceil(el.length / STEP))
      for (let i = 0; i <= n; i++) stations.add(Math.min(length, s + el.length * i / n))
      s += el.length
    }
    const list = [...stations].sort((a, b) => a - b)
    const line = []
    const kept = []
    for (const st of list) {
      const p = localPoint(frame, track, st)
      if (!p) continue
      line.push(p.xy)
      kept.push(st)
      x0 = Math.min(x0, p.xy[0]); y0 = Math.min(y0, p.xy[1]); x1 = Math.max(x1, p.xy[0]); y1 = Math.max(y1, p.xy[1])
    }
    if (line.length >= 2) out.set(track.id, { id: track.id, track, length, line, stations: kept })
  }
  const bounds = Number.isFinite(x0) ? [x0, y0, x1, y1] : [-100, -100, 100, 100]
  return { frame, tracks: out, bounds }
}

/**
 * Where a location of the domain is drawn: { xy, angle } — `angle` the
 * direction it acts in, in degrees from north clockwise (the track's bearing,
 * turned round for `down`); for an orphan its plane point and bearing. Null
 * where it cannot be placed.
 */
export function placeOf(drawing, loc) {
  if (!loc) return null
  if (loc.anchor === 'plane') {
    const [lng, lat] = utmToWgs84(loc.easting, loc.northing, loc.epsg)
    return { xy: drawing.frame.toLocal([lng, lat]), angle: loc.bearing ?? 0, orphan: true }
  }
  const t = drawing.tracks.get(loc.trackId)
  if (!t) return null
  const p = localPoint(drawing.frame, t.track, Math.min(Math.max(loc.station, 0), t.length))
  if (!p) return null
  return { xy: p.xy, angle: loc.direction === 'down' ? (p.bearing + 180) % 360 : p.bearing, bearing: p.bearing }
}

/** The part of a track between two stations, as a polyline in the frame. */
export function stretchLine(drawing, trackId, from, to) {
  const t = drawing.tracks.get(trackId)
  if (!t) return []
  const a = Math.max(0, Math.min(from, to)), b = Math.min(t.length, Math.max(from, to))
  const pts = [localPoint(drawing.frame, t.track, a)?.xy]
  t.stations.forEach((s, i) => { if (s > a && s < b) pts.push(t.line[i]) })
  pts.push(localPoint(drawing.frame, t.track, b)?.xy)
  return pts.filter(Boolean)
}

/** Distance from point p to segment ab, and where along it (0..1). */
function toSegment(p, a, b) {
  const dx = b[0] - a[0], dy = b[1] - a[1]
  const len2 = dx * dx + dy * dy
  const t = len2 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2)) : 0
  const x = a[0] + t * dx, y = a[1] + t * dy
  return { d: Math.hypot(p[0] - x, p[1] - y), t }
}

/**
 * The location on the net nearest a point of the frame, within `reach` [m]:
 * { trackId, station, side, distance } — `side` the side of the track the
 * point lies on, seen in the direction of stationing. Null where no track is
 * that close.
 */
export function locate(drawing, xy, reach) {
  let best = null
  for (const t of drawing.tracks.values()) {
    for (let i = 0; i + 1 < t.line.length; i++) {
      const { d } = toSegment(xy, t.line[i], t.line[i + 1])
      if (d <= reach && (!best || d < best.d)) best = { t, i, d }
    }
  }
  if (!best) return null
  const { t } = best
  // The exact station on the track, through its own geometry.
  const [lng, lat] = drawing.frame.toLngLat(xy)
  const utm = toPlane(t.track, lng, lat)
  const on = utm && stationOnTrack(t.track, utm)
  const station = on ? on.station : t.stations[best.i]
  const a = t.line[best.i], b = t.line[best.i + 1]
  const cross = (b[0] - a[0]) * (xy[1] - a[1]) - (b[1] - a[1]) * (xy[0] - a[0])
  return { trackId: t.id, station: Math.round(station * 1000) / 1000, side: cross > 0 ? 'left' : 'right', distance: best.d }
}

// A WGS84 point in a track's plane: found by Newton steps on the track's own
// forward transform, so this module needs no inverse projection of its own.
function toPlane(track, lng, lat) {
  const p0 = pointAtStation(track, 0)
  if (!p0) return null
  let e = p0.utm.easting, n = p0.utm.northing
  for (let i = 0; i < 6; i++) {
    const [x, y] = utmToWgs84(e, n, track.epsg)
    const [xe, ye] = utmToWgs84(e + 1, n, track.epsg)
    const [xn, yn] = utmToWgs84(e, n + 1, track.epsg)
    const a = xe - x, b = xn - x, c = ye - y, d = yn - y
    const det = a * d - b * c
    if (!det) break
    const dl = lng - x, dt = lat - y
    e += (d * dl - b * dt) / det
    n += (-c * dl + a * dt) / det
  }
  return { easting: e, northing: n, zone: track.epsg }
}
