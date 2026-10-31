import { utmToWgs84 } from '@rail-across/core/geometry'
import { pointAtStation, trackLength } from '@rail-across/core/track'

/**
 * The places a train can start, stop and end at — the signals of the
 * signalling domain, in the way they act, and the platforms of the layout
 * at their middle —, and the net in WGS84 for the map, with a way to find
 * the location nearest a point of it.
 */

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

/** { signals, platforms }: [{ id, label, loc }], each sorted by its label. */
export function placesOf(record, t) {
  const signals = (record?.signalling?.collections?.signals ?? [])
    .filter(s => s.kind !== 'virtual' && s.at?.anchor === 'station' && s.name)
    .map(s => ({ id: `signal:${s.id}`, label: `${t('signal')} ${s.name}`, loc: { trackId: s.at.trackId, station: s.at.station, direction: s.at.direction ?? null } }))
    .sort(byLabel)
  const platforms = (record?.platforms ?? [])
    .filter(p => p.trackId && Number.isFinite(p.startStation) && Number.isFinite(p.endStation))
    .map((p, i) => ({
      id: `platform:${p.id ?? i}`,
      label: [p.stationName, p.code ? `${t('platform_track')} ${p.code}` : null].filter(Boolean).join(' ') || `${t('platform')} ${i + 1}`,
      loc: { trackId: p.trackId, station: (p.startStation + p.endStation) / 2, direction: null },
    }))
    .sort(byLabel)
  return { signals, platforms }
}

/** A track is looked at every this much [m] for the map. */
const STEP = 10

const lngLatOf = (track, station) => {
  const p = pointAtStation(track, station)
  return p ? utmToWgs84(p.utm.easting, p.utm.northing, track.epsg) : null
}

/** The stations a track is drawn at: every STEP and every element's ends. */
function stationsOf(track) {
  const length = trackLength(track)
  const set = new Set([0, length])
  let s = 0
  for (const el of track.elements ?? []) {
    const n = el.elementType === 0 ? 1 : Math.max(1, Math.ceil(el.length / STEP))
    for (let i = 0; i <= n; i++) set.add(Math.min(length, s + el.length * i / n))
    s += el.length
  }
  return [...set].sort((a, b) => a - b)
}

/** The net drawn: { lines: Map trackId → [{ s, lngLat }], bounds: [[w, s], [e, n]] | null }. */
export function netLines(record) {
  const lines = new Map()
  let w = Infinity, so = Infinity, e = -Infinity, n = -Infinity
  for (const track of record?.tracks ?? []) {
    const pts = []
    for (const s of stationsOf(track)) {
      const ll = lngLatOf(track, s)
      if (!ll) continue
      pts.push({ s, lngLat: ll })
      w = Math.min(w, ll[0]); e = Math.max(e, ll[0]); so = Math.min(so, ll[1]); n = Math.max(n, ll[1])
    }
    if (pts.length >= 2) lines.set(track.id, pts)
  }
  return { lines, bounds: Number.isFinite(w) ? [[w, so], [e, n]] : null }
}

export function netGeoJSON(net) {
  return {
    type: 'FeatureCollection',
    features: [...net.lines].map(([id, pts]) => ({ type: 'Feature', properties: { id }, geometry: { type: 'LineString', coordinates: pts.map(p => p.lngLat) } })),
  }
}

/** The part of a track's line between two stations. */
function lineBetween(net, record, trackId, a, b) {
  const pts = net.lines.get(trackId)
  const track = record.tracks.find(t => t.id === trackId)
  if (!pts || !track) return []
  const lo = Math.min(a, b), hi = Math.max(a, b)
  const inner = pts.filter(p => p.s > lo && p.s < hi).map(p => p.lngLat)
  const line = [lngLatOf(track, lo), ...inner, lngLatOf(track, hi)].filter(Boolean)
  return b < a ? line.reverse() : line
}

/** A path as one line on the map. */
export function pathGeoJSON(net, record, path) {
  const coordinates = (path ?? []).flatMap(p => lineBetween(net, record, p.trackId, p.start, p.end))
  return { type: 'FeatureCollection', features: coordinates.length >= 2 ? [{ type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates } }] : [] }
}

/** A point of the net on the map, or null. */
export function lngLatAt(record, loc) {
  const track = record?.tracks?.find(t => t.id === loc?.trackId)
  return track ? lngLatOf(track, Math.min(Math.max(loc.station, 0), trackLength(track))) : null
}

/**
 * The location of the net nearest a point, within `reach` [m], or null:
 * { trackId, station } — the station found on the segment of the drawn line
 * nearest it.
 */
export function nearestLocation(net, [lng, lat], reach) {
  const kx = 111320 * Math.cos(lat * Math.PI / 180), ky = 110540
  const xy = (ll) => [(ll[0] - lng) * kx, (ll[1] - lat) * ky]
  let best = null
  for (const [trackId, pts] of net.lines) {
    for (let i = 0; i + 1 < pts.length; i++) {
      const a = xy(pts[i].lngLat), b = xy(pts[i + 1].lngLat)
      const dx = b[0] - a[0], dy = b[1] - a[1]
      const len2 = dx * dx + dy * dy
      const u = len2 ? Math.max(0, Math.min(1, -(a[0] * dx + a[1] * dy) / len2)) : 0
      const d = Math.hypot(a[0] + u * dx, a[1] + u * dy)
      if (d <= reach && (!best || d < best.d)) best = { d, trackId, station: pts[i].s + u * (pts[i + 1].s - pts[i].s) }
    }
  }
  return best && { trackId: best.trackId, station: Math.round(best.station * 10) / 10 }
}
