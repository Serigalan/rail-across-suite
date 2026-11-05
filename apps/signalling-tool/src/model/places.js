import { kmForTrackPoint, trackLabel, pointAtStation } from '@rail-across/core/track'
import { aspectOf } from '@rail-across/core/signalling'
import { labelOf } from './domain.js'

/** The name of a track as the Layout-Tool names it (koennern.1); one without a name by its line and number. */
export const trackName = (net, id) => {
  const tr = net.tracks.find(x => x.id === id)
  return tr ? tr.name || trackLabel(tr) : id?.slice(0, 8) ?? '—'
}

/**
 * The boards a plate may show here, in this order: the speed indicator and
 * speed signals, the station limit, the slow-speed boards, the shunting
 * limit, the clearance marker, the ETCS stop board — no other are needed.
 */
export const BOARD_ASPECTS = ['Zs_3', 'Ne_1', 'Lf_1', 'Lf_2', 'Lf_3', 'Lf_6', 'Lf_7', 'Ra_10', 'Ra_12', 'Ne_14']

/** A turnout as people name it: W3, else its name or form. */
export function switchName(net, id) {
  const sw = net.switches.find(x => x.switchId === id)
  return Number.isFinite(sw?.number) ? `W${sw.number}` : sw?.name ?? sw?.label ?? id?.slice(0, 8) ?? '—'
}

/** What sets a speed the elements give (routeSpeeds' `where`): "W3", "koennern.1 100–150 m". */
export const speedWhereText = (net, where) => (where?.kind === 'switch'
  ? switchName(net, where.switchId)
  : where ? `${trackName(net, where.trackId)} ${Math.round(where.from)}–${Math.round(where.to)} m` : '')

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

/**
 * The mountings that stand on a foundation beside the track and carry the
 * screen out: a bracket — its mast on one side, the screen on the other —
 * and a gantry, a mast on either side and the screen hung between them.
 */
export const ON_FOUNDATION = new Set(['gantry', 'bracket'])

/** Where a bracket's screen stands from the axis until it is told [m]. */
export const BRACKET_OFFSET = 2

/** How far from its track a foundation is set until it is told [m]. */
export const FOUNDATION_OFFSET = 3.5

const OTHER_SIDE = { left: 'right', right: 'left' }

/** A side seen in the direction a location acts, as seen in the stationing's — and back, the same turn. */
export const sideAlong = (loc, side) => (side && loc?.direction === 'down' ? OTHER_SIDE[side] : side)

/** The other side of the track. */
export const otherSide = (side) => OTHER_SIDE[side] ?? null

/** A bracket's screen: on the other side of the track than its mast, at its offset. */
export const bracketScreen = (foundation, offset = BRACKET_OFFSET) => ({ side: offset ? otherSide(foundation.side ?? 'right') : null, offset })

/**
 * Where a gantry's second mast stands: across from its foundation, as far
 * from the screen the other way — the gantry spans the track symmetrically
 * about its signal. As an offset from the axis, signed: right of the
 * stationing positive.
 */
export function gantryFarOffset(signal) {
  const signed = (side, offset) => (side === 'left' ? -1 : side === 'right' ? 1 : 0) * (offset ?? 0)
  const screen = signed(signal.at?.side, signal.at?.offset)
  const near = signed(sideAlong(signal.at, signal.foundation?.side ?? 'right'), signal.foundation?.offset)
  return 2 * screen - near
}

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

/** Where a gantry's second mast stands in the plane (gantryFarOffset); null for a signal on no gantry. */
export function farMastPoint(net, signal) {
  if (signal?.mounting !== 'gantry' || !signal.foundation || signal.at?.anchor !== 'station') return null
  const far = gantryFarOffset(signal)
  return planePoint(net, { ...signal.at, side: far < 0 ? 'left' : 'right', offset: Math.abs(far) })
}

/** The aspect's name as people write it, with its symbol: "Zs 3 (6)". */
export const aspectText = (a) => `${aspectOf(a.aspect)?.name ?? a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`

/** A kilometrage as a name: the line, the kilometre and the metre — 6344-5+231. */
export function kmName(lineNumber, metres) {
  const whole = Math.round(metres)
  const total = Math.abs(whole)
  return `${lineNumber}-${whole < 0 ? '-' : ''}${Math.floor(total / 1000)}+${String(total % 1000).padStart(3, '0')}`
}

/** How far from its line a point may lie and still be stationed against it [m]. */
const KM_REACH = 500

/**
 * The name a location takes from its kilometrage, read off the project's
 * kilometrage lines (`kmLines`): 6344-5+231 — the next metre free where
 * `taken` has that name already. Null where no line runs near.
 */
export function kmNameAt(net, kmLines, loc, taken = new Set()) {
  const track = net.tracks.find(tr => tr.id === loc?.trackId)
  const p = track && planePoint(net, { ...loc, offset: 0 })
  const hit = p && kmForTrackPoint(kmLines, track, [p.easting, p.northing])
  if (!hit || hit.offset > KM_REACH) return null
  let metres = Math.round(hit.km)
  while (taken.has(kmName(hit.lineNumber, metres))) metres++
  return kmName(hit.lineNumber, metres)
}
