import { trackLabel, pointAtStation } from '@rail-across/core/track'
import { aspectOf } from '@rail-across/core/signalling'

/** The name of a track as the Layout-Tool shows it. */
export const trackName = (net, id) => {
  const tr = net.tracks.find(x => x.id === id)
  return tr ? trackLabel(tr) || tr.name || id.slice(0, 8) : id?.slice(0, 8) ?? '—'
}

/**
 * How far beside its track an object stands when it is set [m]: a signal or a
 * board two metres off the axis, a balise or magnet at the rail, a detection
 * point and a derailer on the track. The plan draws it there; the user may
 * state another offset.
 */
export const OFFSET = { signals: 2, balises: 1, detectionPoints: 0, switchElements: 0 }

/**
 * A location from where the plan was hit: the track, the station, the side
 * hit and `offset` beside it. One that acts in a direction acts so that it
 * stands on the right of the track — as signals stand in Germany: hit on the
 * right of the stationing, it acts with it, on the left against it — unless
 * `direction` keeps one it had.
 */
export function locationFrom(hit, { directed = true, direction, offset = 0 } = {}) {
  const loc = { anchor: 'station', trackId: hit.trackId, station: hit.station, side: hit.side }
  if (offset) loc.offset = offset
  if (directed) loc.direction = direction ?? (hit.side === 'left' ? 'down' : 'up')
  return loc
}

/**
 * Setting an object on the plan: a mode the plan runs while the mouse is on
 * it, snapping to the nearest track and drawing `make(hit)` there before the
 * click, which hands that object to `apply`. Where `make` gives null, nothing
 * can be set there: nothing is drawn and a click does nothing.
 */
export const placing = (collection, make, apply) => ({ kind: 'place', collection, make, apply })

/**
 * Where a location stands in the plane: the point of its track at its
 * station, `offset` metres to its side, square to the track — { easting,
 * northing, epsg, bearing }. Derived, never stored: the track carries it. An
 * orphan has its plane point already. Null where its track is not there.
 */
export function planePoint(net, loc) {
  if (!loc) return null
  if (loc.anchor === 'plane') return { easting: loc.easting, northing: loc.northing, epsg: loc.epsg, bearing: loc.bearing ?? 0 }
  const track = net.tracks.find(t => t.id === loc.trackId)
  const p = track && pointAtStation(track, loc.station)
  if (!p) return null
  const b = (p.bearing * Math.PI) / 180
  const d = (loc.offset ?? 0) * (loc.side === 'left' ? -1 : loc.side === 'right' ? 1 : 0)
  // Right of the stationing is the bearing turned a quarter clockwise: (cos b, −sin b) in easting, northing.
  return { easting: p.utm.easting + Math.cos(b) * d, northing: p.utm.northing - Math.sin(b) * d, epsg: track.epsg, bearing: p.bearing }
}

/** The aspect's name as people write it, with its symbol: "Zs 3 (6)". */
export const aspectText = (a) => `${aspectOf(a.aspect)?.name ?? a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`
