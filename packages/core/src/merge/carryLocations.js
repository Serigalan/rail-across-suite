import { pointAtStation, repairRoute, stationOnTrack } from '../track/index.js'

/**
 * Carrying the locations of a domain across a change of the net (data model
 * R4, R7): one remap for every location, whatever domain holds it.
 *
 * A domain the core does not write — signalling, and whatever follows — says
 * where its objects stand with the core's location types only: a
 * `TrackLocation` (`{ anchor: 'station', trackId, station, side, offset,
 * direction }`) and a `TrackPath` (`{ trackIds }`). Those shapes are all this
 * module looks for, so it carries a domain it has never heard of.
 *
 * Carried by geometry, as remap.js carries switch ports and platforms: a
 * location on a track that changed is looked for, at the place it stood, on
 * the tracks that lie there now. That covers a split, a join, a track turned
 * round (side and direction swap), and a piece put in front of it (its
 * station moves). Where the track is gone and nothing lies there, the object
 * is orphaned in the same step: its location becomes the plane point it stood
 * at (R7), its id and every reference to it stay. Where the track is still
 * there but reshaped so the place is no longer on it, the station stays — a
 * station follows the track's reshaping, which is what it is for.
 */

/** A point lies on a track below this [m]. */
const ON_TRACK = 0.05

/** Stations to the millimetre; a change below that is no change. */
const round = (s) => Math.round(s * 1000) / 1000

const bearingGap = (a, b) => Math.abs(((a - b + 540) % 360) - 180)

const isLocation = (v) => v?.anchor === 'station' && typeof v.trackId === 'string' && Number.isFinite(v.station)
const isPath = (v) => Array.isArray(v?.trackIds) && Object.keys(v).length === 1

const FLIP_SIDE = { left: 'right', right: 'left' }
const FLIP_DIRECTION = { up: 'down', down: 'up' }

/**
 * Where a location on a changed track stands now: on the track that lies at
 * its place (the same one preferred), or in the plane where none does and its
 * track is gone. `ctx` is { present, oldTrack, candidates } (see carryDomain).
 * Returns the location itself when nothing changes.
 */
function carryLocation(loc, ctx) {
  const old = ctx.oldTrack(loc.trackId)
  if (!old) return loc
  const p = pointAtStation(old, loc.station)
  if (!p) return loc
  let hit = null
  for (const track of ctx.candidates(loc.trackId)) {
    if (track.epsg !== old.epsg) continue
    const on = stationOnTrack(track, p.utm)
    if (!on || on.dist > ON_TRACK) continue
    const better = !hit || (track.id === loc.trackId && hit.track.id !== loc.trackId)
      || (on.dist < hit.on.dist - 1e-6 && hit.track.id !== loc.trackId)
    if (better) hit = { track, on }
  }
  if (!hit) {
    if (ctx.present.has(loc.trackId)) return loc
    const bearing = loc.direction === 'down' ? (p.bearing + 180) % 360 : p.bearing
    return {
      anchor: 'plane', easting: round(p.utm.easting), northing: round(p.utm.northing), epsg: old.epsg,
      ...(loc.direction ? { bearing: Math.round(bearing * 1e6) / 1e6 } : {}),
    }
  }
  const reversed = bearingGap(hit.on.bearing, p.bearing) > 90
  const station = round(hit.on.station)
  if (hit.track.id === loc.trackId && !reversed && Math.abs(station - loc.station) < 1e-3) return loc
  const out = { ...loc, trackId: hit.track.id, station }
  if (reversed && loc.side && FLIP_SIDE[loc.side]) out.side = FLIP_SIDE[loc.side]
  if (reversed && loc.direction) out.direction = FLIP_DIRECTION[loc.direction]
  return out
}

/** A run of tracks with each one that went replaced by the pieces lying where it lay, in running order. */
function carryPath(path, ctx) {
  const next = repairRoute(path, ctx.tracks, ctx.switches, ctx.oldTrack, ctx.candidates)
  return next === path ? path : { trackIds: next.trackIds }
}

/**
 * The domain `{ fields, collections }` with every location on a changed track
 * carried. `ctx`:
 *   present     Set of the ids of the tracks the net has now
 *   changed     (trackId) → whether that track went or changed
 *   oldTrack    (trackId) → the geometry it had, or null
 *   candidates  (trackId) → the tracks it may have become (with itself, if it is still there)
 *   tracks, switches  the net as it is now, to put the pieces of a run in order
 *
 * Returns { domain, carried }: the domain (the input itself where nothing
 * moved) and what moved, [{ collection, id, field, from, to }], `to` a track
 * id or 'plane'.
 */
export function carryDomain(domain, ctx) {
  const carried = []
  const collections = domain?.collections
  if (!collections) return { domain, carried }

  const walk = (value, field, note) => {
    if (Array.isArray(value)) {
      let changed = false
      const next = value.map((v, i) => {
        const w = walk(v, field ? `${field}.${i}` : String(i), note)
        if (w !== v) changed = true
        return w
      })
      return changed ? next : value
    }
    if (!value || typeof value !== 'object') return value
    if (isLocation(value)) {
      if (!ctx.changed(value.trackId)) return value
      const next = carryLocation(value, ctx)
      if (next !== value) note(field, value.trackId, next.anchor === 'plane' ? 'plane' : next.trackId)
      return next
    }
    if (isPath(value)) {
      if (!value.trackIds.some(id => ctx.changed(id))) return value
      const next = carryPath(value, ctx)
      if (next !== value) note(field, value.trackIds.filter(id => !ctx.present.has(id)).join(','), next.trackIds.join(','))
      return next
    }
    let changed = false
    const next = {}
    for (const [k, v] of Object.entries(value)) {
      const w = walk(v, field ? `${field}.${k}` : k, note)
      if (w !== v) changed = true
      next[k] = w
    }
    return changed ? next : value
  }

  let any = false
  const out = {}
  for (const [name, list] of Object.entries(collections)) {
    if (!Array.isArray(list)) { out[name] = list; continue }
    let changed = false
    const next = list.map(obj => {
      const note = (field, from, to) => carried.push({ collection: name, id: obj?.id ?? null, field, from, to })
      const w = walk(obj, '', note)
      if (w !== obj) changed = true
      return w
    })
    out[name] = changed ? next : list
    if (changed) any = true
  }
  return { domain: any ? { ...domain, collections: out } : domain, carried }
}

/** The domains a record carries for others, by the name it holds them under (coreFormat's CARRIED). */
export const CARRIED_DOMAINS = ['signalling']

/**
 * A record after one write of the net (the Layout-Tool's storage.js, every
 * write comes by here): the domains it carries for others with their
 * locations carried onto the tracks `after` has. A track counts as changed
 * where `after` holds another object than `before` did, or none; it may have
 * become any track the write brought or changed, as repairRoutes reads it.
 * The record itself where nothing moved.
 */
export function carryAfterWrite(before, after) {
  if (!after || before?.tracks === after.tracks) return after
  const domains = CARRIED_DOMAINS.filter(name => after[name]?.collections)
  if (!domains.length) return after
  const old = new Map((before?.tracks ?? []).map(t => [t.id, t]))
  const now = new Map((after.tracks ?? []).map(t => [t.id, t]))
  const touched = (after.tracks ?? []).filter(t => old.get(t.id) !== t)
  const ctx = {
    present: new Set(now.keys()),
    changed: (id) => old.has(id) && old.get(id) !== now.get(id),
    oldTrack: (id) => old.get(id) ?? null,
    candidates: () => touched,
    tracks: after.tracks ?? [],
    switches: after.switches ?? [],
  }
  let out = after
  for (const name of domains) {
    const { domain } = carryDomain(after[name], ctx)
    if (domain !== after[name]) out = { ...out, [name]: domain }
  }
  return out
}
