import { trackLabel, pointAtStation } from '@rail-across/core/track'
import { aspectOf } from '@rail-across/core/signalling'
import { labelOf } from './domain.js'

/** The name of a track as the Layout-Tool names it (koennern.1); one without a name by its line and number. */
export const trackName = (net, id) => {
  const tr = net.tracks.find(x => x.id === id)
  return tr ? tr.name || trackLabel(tr) : id?.slice(0, 8) ?? '—'
}

/**
 * Signals to choose from in a list: each by its designation and the track it
 * stands at (21-B · koennern.1), as names repeat from track to track; sorted.
 */
export const signalOptions = (net, signals, t) => signals
  .map(s => ({ value: s.id, label: [labelOf('signals', s, t), s.at?.anchor === 'station' ? trackName(net, s.at.trackId) : null].filter(Boolean).join(' · ') }))
  .sort((a, b) => a.label.localeCompare(b.label, 'de', { numeric: true }))

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

/** The mountings that stand on a foundation beside the track and carry the screen out over it: a gantry, a bracket. */
export const ON_FOUNDATION = new Set(['gantry', 'bracket'])

/** How far from its track a foundation is set until it is told [m]. */
export const FOUNDATION_OFFSET = 3.5

const OTHER_SIDE = { left: 'right', right: 'left' }

/** A side seen in the direction a location acts, as seen in the stationing's — and back, the same turn. */
export const sideAlong = (loc, side) => (side && loc?.direction === 'down' ? OTHER_SIDE[side] : side)

/**
 * A signal's location on a gantry or bracket from where the plan was hit: it
 * acts as a signal set there would (locationFrom), its screen and its
 * foundation stand where they are asked — each a side seen in the direction
 * it acts, the screen's null over the track, and an offset from the axis.
 * Returns { at, foundation }.
 */
export function mountedFrom(hit, { screen, foundation }) {
  const { side: _hit, ...loc } = locationFrom(hit)
  const at = screen.side ? { ...loc, side: sideAlong(loc, screen.side), offset: screen.offset ?? 0 } : loc
  return { at, foundation: { side: foundation.side, offset: foundation.offset ?? FOUNDATION_OFFSET } }
}

/**
 * Setting an object on the plan: a mode the plan runs while the mouse is on
 * it, snapping to the nearest track and drawing `make(hit)` there before the
 * click, which hands that object to `apply`. Where `make` gives null, nothing
 * can be set there: nothing is drawn and a click does nothing.
 */
export const placing = (collection, make, apply) => ({ kind: 'place', collection, make, apply })

/** Picking a signal on the plan, one of `signals`: a click on it hands its id to `apply`. */
export const pickingSignal = (signals, t, apply) => {
  const ids = new Set(signals.map(s => s.id))
  return { kind: 'pick-signal', hint: t('pick_signal_hint'), accepts: (id) => ids.has(id), apply }
}

/** Picking a section's points on the plan: a click on a point adds it, or takes it away again. */
export const pickingPoints = (ops, sectionId) => ({
  kind: 'pick-points',
  toggle: (id) => ops.modify('trackSections', sectionId, (now) => ({
    ...now, boundedBy: now.boundedBy.includes(id) ? now.boundedBy.filter(x => x !== id) : [...now.boundedBy, id],
  })),
})

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

/** Where a signal's foundation stands in the plane, as planePoint gives it; null for one without, or an orphan. */
export const foundationPoint = (net, signal) => (signal?.foundation && signal.at?.anchor === 'station'
  ? planePoint(net, { ...signal.at, side: sideAlong(signal.at, signal.foundation.side), offset: signal.foundation.offset })
  : null)

/** The aspect's name as people write it, with its symbol: "Zs 3 (6)". */
export const aspectText = (a) => `${aspectOf(a.aspect)?.name ?? a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`
