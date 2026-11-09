// The track module's small queries — a point at a station, a height, a
// switch's ports, a label — kept in JavaScript beside its crate (decision
// 0021, phase D): the pages ask them point by point, per signal and per frame,
// and across to the module each would cost at least 12 µs against well under
// one here (a coordinate conversion: 2 µs here, 12.7 µs there). They are held
// to the crate's answers by its frozen vectors (src/track/queries.test.js).

import {
  bearingAfterUtm, clamp, endPointCurvedUtm, endPointStraightUtm, nodeUtm, projectOnArcUtm,
  transitionBearingAtUtm, transitionPointAtUtm, utmToWgs84,
} from '../geometry/index.js'
import {
  BOUNDARY, DEFAULT_SWITCH_KIND, HEIGHT_POINT_SPACING, HEIGHT_SPLIT_MIN, LINK_KIND, PLATFORM_FRONT_OFFSET,
  PLATFORM_WIDTH, STATUSES, SWITCH_FORM_VERSION, TYPE_CODES,
} from './constants.js'

// A height point this close to a track end sits on its joint [m].
const HEIGHT_JOINT_TOL = 0.01

/**
 * The routes each kind has, as its elements mark themselves, with the ports each
 * route runs between.
 *
 * A turnout's two routes share the toe: they part there, which is what makes it
 * a turnout. The crossing kinds instead have four ports and routes that cross
 * without sharing an end — `main` from A to C, `cross` from B to D — and a slip
 * adds the connecting curves between them, one for the single slip and two for
 * the double. That the routes of a crossing share no port is why both of them
 * can outlive it as ordinary track, which the turnout's pair never can (see
 * switchDelete).
 */
const ROUTE_PORTS = {
  turnout:     { main: ['A', 'B2'], branch: ['A', 'B1'] },
  crossing:    { main: ['A', 'C'],  cross: ['B', 'D'] },
  single_slip: { main: ['A', 'C'],  cross: ['B', 'D'], slip1: ['A', 'D'] },
  double_slip: { main: ['A', 'C'],  cross: ['B', 'D'], slip1: ['A', 'D'], slip2: ['B', 'C'] },
  // A link has no route because it has no length: both of its ports are the
  // one node. Everything that walks routes therefore walks nothing here, which
  // is what makes a link own no elements and take none with it when it goes.
  link: {},
}

/** The ports each route of this kind runs between, as { route: [port, port] }. */
export const switchRoutePorts = (kind) => ROUTE_PORTS[kind] ?? ROUTE_PORTS[DEFAULT_SWITCH_KIND]

/** The routes of a kind, in the order a shared port is claimed (see switchDelete). */
export const switchRoutes = (kind) => Object.keys(switchRoutePorts(kind))

/**
 * The port of a turnout the line it lies on runs on through: B2, the through
 * route — unless the turnout is `swapped`. An outer-bent turnout whose stem
 * radius is its form's own has a straight branch; that is no bent turnout but
 * the plain form with its routes swapped, and it is built as one: the curved
 * line it was laid into is its branch (B1), the straight leaving it its
 * through route (B2). The line then runs on over B1, and only the record can
 * say so — at a joint the tracks give no sign which of the two it is.
 */
export const turnoutLinePort = (sw) => (sw?.swapped ? 'B1' : 'B2')

/** …and the port the turnout leads off that line at. */
export const turnoutDivergingPort = (sw) => (sw?.swapped ? 'B2' : 'B1')

/** The route of a turnout that its line runs over, as its elements are marked. */
export const turnoutLineRoute = (sw) => (sw?.swapped ? 'branch' : 'main')

/**
 * What a kind and its routes are called, as locale keys — a turnout is a
 * Weiche, a crossing a Kreuzung, both slips a Kreuzungsweiche, and each route
 * carries the name the dialogs give it. The element table reads them so a
 * turnout's elements say what they are instead of reading as plain running
 * line; an unknown kind falls back to the turnout's names, the same way
 * switchRoutePorts does.
 */
const KIND_LABEL_KEY = {
  turnout:     'table_type_switch',
  crossing:    'table_type_crossing',
  single_slip: 'table_type_crossing_switch',
  double_slip: 'table_type_crossing_switch',
  link:        'table_type_link',
}

const ROUTE_LABEL_KEY = {
  turnout:  { main: 'table_route_stem',    branch: 'table_route_branch' },
  crossing: { main: 'table_route_through', cross:  'table_route_cross',
    slip1: 'table_route_slip', slip2: 'table_route_slip' },
}

/** Locale key naming this kind of switch. */
export const switchKindLabelKey = (kind) =>
  KIND_LABEL_KEY[kind] ?? KIND_LABEL_KEY[DEFAULT_SWITCH_KIND]

/**
 * Locale key naming one route of this kind — null for a route it does not have,
 * and therefore null throughout for a link, which has none. The crossing kinds
 * share their route names: a slip is a crossing with the connecting curves
 * added, so its two crossing roads are called the same.
 */
export const switchRouteLabelKey = (kind, route) => {
  // Against the kind's own routes, not against the name table: `main` is a
  // route of a crossing and of a turnout, and of a link it is neither.
  if (ROUTE_PORTS[kind] && !ROUTE_PORTS[kind][route]) return null
  const names = kind !== DEFAULT_SWITCH_KIND && ROUTE_PORTS[kind]
    ? ROUTE_LABEL_KEY.crossing
    : ROUTE_LABEL_KEY.turnout
  return names[route] ?? null
}

/** A node between two track ends rather than a shape of track (see SWITCH_KINDS). */
export const isLinkSwitch = (sw) => sw?.kind === LINK_KIND

const newSwitchId = () => crypto.randomUUID()

/**
 * What an element of a switch's route carries so it can be found again: the
 * switch's id and which of its routes this is. Its name and form label are the
 * switch's own and are read there (decision 333) — an element
 * keeps no copy of them.
 */
export function switchElementMark(sw, route) {
  return {
    switchBranch: true,
    switchRoute:  route,
    ...(sw.switchId ? { switchId: sw.switchId } : {}),
  }
}

/**
 * Does this element belong to that switch? The id decides, and only the id —
 * every record and every marked element carries one, and a payload that does
 * not is refused at the door (persistenceUtils.parseProjectsPayload). The
 * missing id is checked for explicitly: two of them absent must not read as a
 * match and join elements of unrelated turnouts.
 */
export function elementBelongsToSwitch(el, sw) {
  return el.switchId != null && el.switchId === sw.switchId
}

/** The fields a record needs to be one of the current model, for a new record. */
export function newSwitchFields(kind = DEFAULT_SWITCH_KIND) {
  return { switchId: newSwitchId(), kind, formVersion: SWITCH_FORM_VERSION }
}

/** Does this record carry what the model requires of it? */
export function isModelledSwitch(sw) {
  return Boolean(sw?.switchId) && Boolean(sw?.kind) && sw?.formVersion != null
}

/**
 * The ports of each kind: which of the switch's routes carries the elements
 * marked there. What a port joins stands on the record under `ports` (decision
 * 333): `ports.A = { trackId, endpoint }` — the track it names
 * and which end of that track ('BEGIN' | 'END') the switch sits at. portTrack and
 * portEnd read it, withPort writes it.
 *
 * A turnout has three, and port A is its toe: the switch owns no elements there
 * — the toe is a node, not a stretch — so `route` says which route *would* run
 * into it. The crossing kinds have four, one per end of the two crossing
 * routes, and every one of them carries elements.
 *
 * Everything that walks a record's ports walks the list for its kind, so a kind
 * with more of them is a further entry here and nothing else.
 */
const PORTS_BY_KIND = {
  turnout: [
    { port: 'A', route: 'main' },
    { port: 'B1', route: 'branch' },
    { port: 'B2', route: 'main' },
  ],
  crossing: [
    { port: 'A', route: 'main' },
    { port: 'B', route: 'cross' },
    { port: 'C', route: 'main' },
    { port: 'D', route: 'cross' },
  ],
}
PORTS_BY_KIND.single_slip = PORTS_BY_KIND.crossing
PORTS_BY_KIND.double_slip = PORTS_BY_KIND.crossing
// A link has two, and neither carries elements: both are the same node, met
// from opposite sides. `route: null` is not a gap in the table — it says there
// is no route to look for, which is how switchParts finds none.
PORTS_BY_KIND.link = [
  { port: 'A', route: null },
  { port: 'B', route: null },
]

/** The ports of a kind. */
export const switchPorts = (kind) => PORTS_BY_KIND[kind] ?? PORTS_BY_KIND[DEFAULT_SWITCH_KIND]

/** The ports of this record, by its own kind. */
export const portsOf = (sw) => switchPorts(sw?.kind)

/** The track `port` of `sw` joins, or null. */
export const portTrack = (sw, port) => sw?.ports?.[port]?.trackId ?? null

/** Which end of its track `port` of `sw` sits at ('BEGIN' | 'END'), or null. */
export const portEnd = (sw, port) => sw?.ports?.[port]?.endpoint ?? null

/** `sw` with `patch` ({ trackId?, endpoint? }) written over its `port`. */
export const withPort = (sw, port, patch) => ({
  ...sw, ports: { ...sw.ports, [port]: { ...sw.ports?.[port], ...patch } },
})

/** Does this element carry `sw`'s mark for `route`? */
export const elementOnSwitchRoute = (el, sw, route) => Boolean(el?.switchBranch)
  && elementBelongsToSwitch(el, sw)
  && (!el.switchRoute || el.switchRoute === route)

/**
 * The element without any of the marks that tied it to a switch — what a route
 * becomes when the switch over it is deleted and the geometry stays as ordinary
 * track. The cant justification goes with them: it was written for a switch
 * route's 100 mm limit, and on a line element (limit 170) it would claim an
 * exception nothing needs any more.
 */
export function unmarkSwitchElement(el) {
  const {
    switchBranch: _b, switchRoute: _r, switchId: _i,
    cantException: _c, ...rest
  } = el
  return rest
}

const STATION_TOL = 1e-6   // m — two stations this close are the same point

export const trackLength = (track) => (track.elements ?? []).reduce((s, el) => s + (el.length ?? 0), 0)

/**
 * Stations a track's height points sit at when they are read from the terrain:
 * begin and end, and for a track longer than HEIGHT_SPLIT_MIN evenly spaced
 * points at most HEIGHT_POINT_SPACING apart in between.
 */
export function heightStations(length) {
  if (!(length > 0)) return []
  if (length <= HEIGHT_SPLIT_MIN) return [0, length]
  const n = Math.ceil(length / HEIGHT_POINT_SPACING)
  return Array.from({ length: n + 1 }, (_, i) => (i === n ? length : length * i / n))
}

/** Point on an element at station `s` along it, in the track's plane. */
export function pointAtStationUtm(el, s, epsg) {
  const coords = el.geometry?.coordinates ?? []
  if (s >= el.length && el.endNode) return nodeUtm(el.endNode, coords[coords.length - 1], epsg)
  const start = nodeUtm(el.startNode, coords[0], epsg)
  if (s <= 0) return start
  if (el.elementType === 2 && el.r1 !== undefined) {
    return transitionPointAtUtm(start, el.bearing, el.length, el.r1, el.r2 ?? null, el.transitionType, s)
  }
  if (el.radius != null) return endPointCurvedUtm(start, el.bearing, s, el.radius)
  return endPointStraightUtm(start, el.bearing, s)
}

/** The element a track station falls in, with the station along that element. */
export function elementAtStation(elements, station) {
  let start = 0
  for (const [i, el] of (elements ?? []).entries()) {
    const length = el.length ?? 0
    if (station <= start + length + STATION_TOL) return { el, elIdx: i, s: Math.max(0, station - start) }
    start += length
  }
  const last = (elements ?? []).length - 1
  return last >= 0 ? { el: elements[last], elIdx: last, s: elements[last].length ?? 0 } : null
}

/** Where a track's height points sit on the ground, for sampling the terrain. */
export function trackSamplePoints(track, stations) {
  const epsg = track.epsg
  return stations.map(station => {
    const hit = elementAtStation(track.elements, station)
    const p = hit ? pointAtStationUtm(hit.el, hit.s, epsg) : null
    return { station, lngLat: p ? utmToWgs84(p.easting, p.northing, epsg) : null }
  })
}

/** Height at a track station, linearly between the points. */
export function heightAt(heights, station) {
  if (!heights?.length) return null
  if (station <= heights[0].station) return heights[0].z
  for (let i = 1; i < heights.length; i++) {
    const a = heights[i - 1], b = heights[i]
    if (station <= b.station) {
      const t = b.station === a.station ? 0 : (station - a.station) / (b.station - a.station)
      return a.z + (b.z - a.z) * t
    }
  }
  return heights[heights.length - 1].z
}

/** The gradients into and out of height point `i` [m/m] — null where there is no point before or after. */
export function pointGrades(heights, i) {
  const p = heights?.[i], a = heights?.[i - 1], b = heights?.[i + 1]
  const grade = (u, v) => (u && v && v.station > u.station ? (v.z - u.z) / (v.station - u.station) : null)
  return { before: p ? grade(a, p) : null, after: p ? grade(p, b) : null }
}

/** Heights are kept to a tenth of a millimetre: a gradient between two of
 * them is then good to the hundredth of a per mille over a few metres. */
export const roundHeight = (z) => Math.round(z * 1e4) / 1e4

/**
 * The index of a track's height point at one of its ends, or null without
 * heights there — also where the heights stop short of that end.
 */
const endIndex = (track, end) => {
  const h = track?.heights ?? []
  if (!h.length) return null
  if (end === 'BEGIN') return h[0].station <= HEIGHT_JOINT_TOL ? 0 : null
  return trackLength(track) - h[h.length - 1].station <= HEIGHT_JOINT_TOL ? h.length - 1 : null
}

/** Which end of a track a height point index is at, or null for one in between. */
export function endOfIndex(track, index) {
  if (index === 0 && endIndex(track, 'BEGIN') === 0) return 'BEGIN'
  if (index > 0 && index === endIndex(track, 'END')) return 'END'
  return null
}

/** The gradients into and out of point `i`, or null where one of them is not there. */
function gradients(points, i) {
  const p = points[i], a = points[i - 1], b = points[i + 1]
  if (!p || !a || !b) return null
  const before = (p.z - a.z) / (p.station - a.station)
  const after  = (b.z - p.z) / (b.station - p.station)
  return Number.isFinite(before) && Number.isFinite(after) ? { before, after } : null
}

/**
 * Tangent length of the vertical curve at point `i` of `points` ([{ station,
 * z, rv }]): T = R·|Δi|/2 with Δi the change of gradient there. Null at the
 * ends, where there is no gradient change, and where the point has no curve.
 */
export function tangentLength(points, i) {
  const g = points[i]?.rv ? gradients(points, i) : null
  return g ? Math.abs(points[i].rv) * Math.abs(g.after - g.before) / 2 : null
}

/**
 * Height of the rounded gradient at a station [m]: the tangent polygon of
 * `heightAt`, and inside a vertical curve the parabola `verticalCurve` draws.
 * This is the height the track is built at — the polygon is only where its
 * gradients meet. Null for a track without heights.
 */
export function gradientAt(heights, station) {
  const z = heightAt(heights, station)
  if (z == null) return null
  for (let i = 1; i < heights.length - 1; i++) {
    const t = tangentLength(heights, i)
    const p = heights[i]
    const x = station - p.station
    if (!t || Math.abs(x) >= t) continue
    const { before, after } = gradients(heights, i)
    return p.z + before * x + (x + t) ** 2 * (after - before) / (4 * t)
  }
  return z
}


/** The elements of a track with the station each of them starts and ends at. */
export function elementStations(track) {
  let station = 0
  return (track?.elements ?? []).map((el, index) => {
    const start = station
    station += el.length ?? 0
    return { el, index, start, end: station }
  })
}

/** Point and tangent bearing at station `s` within one element, in the plane. */
export function pointOnElement(el, epsg, s) {
  const startUtm = nodeUtm(el.startNode, el.geometry?.coordinates?.[0], epsg)
  if (el.elementType === 2) {
    return {
      utm:     transitionPointAtUtm(startUtm, el.bearing, el.length, el.r1 ?? null, el.r2 ?? null, el.transitionType, s),
      bearing: transitionBearingAtUtm(el.bearing, el.length, el.r1 ?? null, el.r2 ?? null, el.transitionType, s),
    }
  }
  if (el.radius != null) {
    return {
      utm:     endPointCurvedUtm(startUtm, el.bearing, s, el.radius),
      bearing: bearingAfterUtm(el.bearing, s, el.radius),
    }
  }
  return { utm: endPointStraightUtm(startUtm, el.bearing, s), bearing: el.bearing }
}

/** Point and tangent bearing at a station of the whole track, in its plane. */
export function pointAtStation(track, station) {
  const rows = elementStations(track)
  if (!rows.length || !track?.epsg) return null
  const total = rows[rows.length - 1].end
  const s     = clamp(station, 0, total)
  const row   = rows.find(r => s <= r.end + 1e-9) ?? rows[rows.length - 1]
  return pointOnElement(row.el, track.epsg, s - row.start)
}

/** Nearest station of a plane point on a track, with its distance. */
export function stationOnTrack(track, utm) {
  let best = null
  for (const row of elementStations(track)) {
    const s = stationFromClick(track, row.index, utm)
    if (s == null) continue
    const p = pointAtStation(track, s)
    if (!p) continue
    const d = Math.hypot(p.utm.easting - utm.easting, p.utm.northing - utm.northing)
    if (!best || d < best.dist) best = { station: s, dist: d, bearing: p.bearing }
  }
  return best
}

/** Signed offsets of the two edges: the side decides which way they go. */
/**
 * Station of a point clicked on element `elIdx` of a track. A straight or arc
 * is projected onto analytically; a transition has no closed form, so the
 * nearest point of a dense sampling of its own curve stands in — a click is
 * only ever a starting value the form lets the user correct.
 */
export function stationFromClick(track, elIdx, clickUtm) {
  const row = elementStations(track)[elIdx]
  if (!row || !track?.epsg) return null
  const { el } = row
  let along
  if (el.elementType === 2) {
    const distAt = (s) => {
      const { utm } = pointOnElement(el, track.epsg, s)
      return Math.hypot(utm.easting - clickUtm.easting, utm.northing - clickUtm.northing)
    }
    // Coarse scan for the bracket, then a ternary search inside it — the
    // distance falls and rises again around the single nearest point.
    const n = 64
    let best = Infinity
    for (let i = 0; i <= n; i++) {
      const s = el.length * i / n
      const d = distAt(s)
      if (d < best) { best = d; along = s }
    }
    let lo = Math.max(0, along - el.length / n)
    let hi = Math.min(el.length, along + el.length / n)
    for (let i = 0; i < 40 && hi - lo > 1e-4; i++) {
      const m1 = lo + (hi - lo) / 3
      const m2 = hi - (hi - lo) / 3
      if (distAt(m1) < distAt(m2)) hi = m2; else lo = m1
    }
    along = (lo + hi) / 2
  } else {
    const startUtm = nodeUtm(el.startNode, el.geometry?.coordinates?.[0], track.epsg)
    along = projectOnArcUtm(startUtm, clickUtm, el.bearing, el.radius ?? null).along
  }
  return row.start + clamp(along, 0, el.length)
}

/** Length of the platform along the track [m]. */

export function edgeOffsets(platform) {
  const sign  = platform.side === 'left' ? -1 : 1
  const front = platform.frontOffset ?? PLATFORM_FRONT_OFFSET
  // The back edge follows the front one: moving the edge shifts the platform,
  // it does not make it narrower.
  const back  = platform.backOffset ?? (front + PLATFORM_WIDTH)
  return { front: sign * front, back: sign * back }
}

/** Length of the platform along the track [m]. */
export const platformLength = (platform) =>
  Math.abs((platform?.endStation ?? 0) - (platform?.startStation ?? 0))

const valid = (s) => (STATUSES.includes(s) ? s : null)

export const trackStatus = (track) => valid(track?.status) ?? 'existing'

/**
 * A switch's own status, or — where it states none — the one its tracks give
 * it: a turnout is built or taken out with its branch, a crossing with any of
 * its roads. New wins over removal, which is what a renewal on an otherwise
 * standing track looks like.
 */
export function switchStatus(sw, trackById) {
  const own = valid(sw?.status)
  if (own) return own
  const turnout = (sw.kind ?? DEFAULT_SWITCH_KIND) === 'turnout'
  const ports = portsOf(sw).filter(p => !turnout || p.port === 'B1')
  const found = ports.map(p => trackStatus(trackById.get(portTrack(sw, p.port))))
  if (found.includes('new')) return 'new'
  if (found.includes('removal')) return 'removal'
  return 'existing'
}

/**
 * Designations are composed from the number: 8 → `switch.008`. A crossing is
 * no switch — its kinds carry the crossing prefix, so the plan and the OSRD
 * label name it for what it is.
 */
const SWITCH_PREFIX   = 'switch'

const CROSSING_PREFIX = 'crossing'

/** The prefix a kind's designation carries. */
const switchPrefix = (kind) =>
  (kind && kind !== 'turnout' ? CROSSING_PREFIX : SWITCH_PREFIX)

export const switchDesignation = (number, kind = null) =>
  `${switchPrefix(kind)}.${String(number).padStart(3, '0')}`

/**
 * The number a switch carries. Records written before numbering existed have
 * only their designation, so the number is read back out of that.
 */
export function switchNumberOf(sw) {
  if (Number.isFinite(sw?.number)) return sw.number
  const match = /^(?:switch|crossing)\.(\d+)$/.exec(sw?.name ?? '')
  return match ? Number(match[1]) : null
}

const LINE    = Number(TYPE_CODES.line_track)

const STATION = Number(TYPE_CODES.station_track)

const text = (v) => (v != null ? String(v).trim() : '')

/**
 * 'line', 'station' or null. The type a track states decides; tracks written
 * before the type was stored say it only through the fields they carry — a
 * station name makes a station track, a line number a line track.
 */
export function trackKind(track) {
  if (track?.trackType === LINE) return 'line'
  if (track?.trackType === STATION) return 'station'
  if (text(track?.stationName) || text(track?.uicStation)) return 'station'
  if (text(track?.lineNumber)) return 'line'
  return null
}

/**
 * The type a form shows for a track, `line_track` or `station_track` — as it
 * states it or its fields tell (see trackKind), else `fallback`.
 */
export function trackTypeName(track, fallback) {
  const kind = trackKind(track)
  return kind ? `${kind}_track` : fallback
}

/**
 * The group a track falls in: `line:6340`, `station:halle (saale) hbf`, or
 * null where its kind names nothing to group it under. A station is known by
 * its name rather than its UIC number — the number is often missing on some
 * of its tracks, the name never is on a station track that says anything.
 */
export function trackGroupKey(track) {
  const kind = trackKind(track)
  if (kind === 'line') {
    const number = text(track.lineNumber)
    return number ? `line:${number}` : null
  }
  if (kind === 'station') {
    const name = text(track.stationName).toLowerCase()
    const uic = text(track.uicStation)
    if (name) return `station:${name}`
    return uic ? `station:#${uic}` : null
  }
  return null
}

/**
 * What a group is called: `6340 · Halle–Könnern`, `Halle (Saale) Hbf`. The
 * caller puts the word for the kind in front, or the word for none.
 */
export function groupTitle(group) {
  if (group.kind === 'line') return [group.lineNumber, group.lineName].map(text).filter(Boolean).join(' · ')
  if (group.kind === 'station') return text(group.stationName) || text(group.uicStation)
  return ''
}

export const endKey = (trackId, endpoint) => `${trackId}|${endpoint}`

/**
 * What a timetable calls the place of a mark — a train may begin and end at
 * a buffer stop and at a boundary: its own name, else one made of its kind
 * and its track's name — or station and number — and end, in the words of
 * the data (German, as the stations').
 */
export function endMarkName(mark, track) {
  const own = String(mark?.name ?? '').trim()
  if (own) return own
  const station = [track?.stationName, track?.trackNumber && `Gl. ${track.trackNumber}`].filter(Boolean).join(' ')
  const where = String(track?.name ?? '').trim() || station || String(mark?.trackId ?? '').slice(0, 8)
  return `${mark?.kind === BOUNDARY ? 'Grenze' : 'Prellbock'} ${where} ${mark?.endpoint === 'BEGIN' ? 'Anfang' : 'Ende'}`
}
