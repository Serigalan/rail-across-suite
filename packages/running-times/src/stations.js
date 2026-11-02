import { stationRegister } from '@rail-across/core/document'
import { BOUNDARY, endMarkName, endMarkStation, trackGroupKey, trackKind, trackLength } from '@rail-across/core/track'
import { directionOf, pathLength } from './path.js'

/**
 * The stations of a net, as the layout and the register say them, each by
 * its id — the id of its point of the operational point register (R10),
 * which names it; its name is only what it is shown by. A station (Bahnhof)
 * is the station tracks of one point (trackKind 'station', a point by the
 * name or UIC number they state, as the register makes it), with the
 * platforms on them or of it; a halt (Haltepunkt) a point only platforms
 * are of — no track of its own, platforms on the line's tracks. A point of
 * the register with stopping places of the Signalling-Tool is a station too,
 * where no station tracks are of it: its tracks are those of its stopping
 * places and platforms, each only as far as they reach (`from`, `to`), a
 * train stopping in the middle of its stopping place (`at`). A buffer stop
 * and a boundary of the planning area are an end (`kind` 'end'), by the id
 * of their mark: a place at a track's end. A train begins and ends at a
 * station or an end only, on one of its tracks; it stops or passes at a
 * station or a halt.
 */

const text = (v) => (v != null ? String(v).trim() : '')
const collator = new Intl.Collator('de', { numeric: true, sensitivity: 'base' })

/** The key a name is known by — `station:halle (saale) hbf` —, what a place is found by where a point names it only (placeOf). */
export function stationKey(name) {
  const n = text(name).toLowerCase()
  return n ? `station:${n}` : null
}

/** What a track is called in a station: its number, else its name. */
export const trackLabel = (track) => text(track?.trackNumber) || text(track?.name) || String(track?.id ?? '').slice(0, 8)

const platformMiddle = (p) => (p.startStation + p.endStation) / 2

/**
 * [{ id, key, name, uic, kind: 'station' | 'halt' | 'end', pointId, tracks:
 * [{ id, label, length, platforms, from, to, at }], platforms: [{ id, code,
 * trackId, at }] }], sorted by name. `id` is what a train's point names it
 * by (`key` the same), `name` what it is shown by: the station's name, else
 * its UIC number or code; `pointId` its point of the register — the same as
 * its id but for an end. A platform's `at` is its middle on its track; a
 * track's `from`, `to` how far the station reaches on it (absent: all of
 * it), `at` where a train stops on it without a platform. An end has the one
 * track its mark is on, `at` where a train stands there, and `end` { kind:
 * 'buffer_stop' | 'boundary', markId, speed } — `speed` a boundary's [km/h],
 * or null.
 */
export function stationsOf(record) {
  const register = stationRegister(record)
  const points = new Map(register.points.map(p => [p.id, p]))
  const byId = new Map()
  const make = (id, kind) => {
    const p = points.get(id)
    const station = { id, key: id, name: text(p?.name) || text(p?.uic) || text(p?.rl100), uic: text(p?.uic) || null, kind, pointId: id, tracks: [], platforms: [] }
    byId.set(id, station)
    return station
  }
  const platforms = (record?.platforms ?? []).filter(p => p.trackId && Number.isFinite(p.startStation) && Number.isFinite(p.endStation))
  for (const track of record?.tracks ?? []) {
    if (trackKind(track) !== 'station' || !trackGroupKey(track)) continue
    const id = register.pointOf(track)
    if (!id) continue
    const station = byId.get(id) ?? make(id, 'station')
    station.tracks.push({ id: track.id, label: trackLabel(track), length: trackLength(track), platforms: [] })
  }
  const trackStation = new Map()
  for (const s of byId.values()) for (const tr of s.tracks) trackStation.set(tr.id, s)
  for (const p of platforms) {
    const id = register.pointOf(p)
    const station = trackStation.get(p.trackId) ?? (id ? byId.get(id) ?? make(id, 'halt') : null)
    if (!station) continue
    const platform = { id: p.id, code: text(p.code) || null, trackId: p.trackId, at: platformMiddle(p) }
    station.platforms.push(platform)
    station.tracks.find(tr => tr.id === p.trackId)?.platforms.push(platform)
  }
  withRegister(record, register.points, byId, make)
  withEnds(record, byId)
  for (const s of byId.values()) s.tracks.sort((a, b) => collator.compare(a.label, b.label))
  return [...byId.values()].sort((a, b) => collator.compare(a.name, b.name))
}

/**
 * The points of the register with stopping places, made stations: one the
 * station tracks are of is theirs; one only platforms are of, or nothing,
 * gets the tracks of its stopping places and platforms, as far as they
 * reach. One without either is no place a train can be.
 */
function withRegister(record, points, byId, make) {
  const lengths = new Map((record?.tracks ?? []).map(tr => [tr.id, tr]))
  const places = record?.signalling?.collections?.stoppingPlaces ?? []
  for (const point of points) {
    const known = byId.get(point.id)
    if (known?.kind === 'station') continue
    const mine = places.filter(h => h.operationalPointId === point.id && h.at)
    if (!mine.length && !known) continue
    const tracks = new Map()
    const reach = (trackId, from, to, at, platform = null) => {
      const track = lengths.get(trackId)
      if (!track) return
      const was = tracks.get(trackId)
      if (was) {
        was.from = Math.min(was.from, from); was.to = Math.max(was.to, to)
        if (platform) was.platforms.push(platform)
        return
      }
      tracks.set(trackId, { id: trackId, label: trackLabel(track), length: trackLength(track), platforms: platform ? [platform] : [], from, to, at })
    }
    for (const h of mine) reach(h.at.trackId, h.at.startStation, h.at.endStation, (h.at.startStation + h.at.endStation) / 2)
    if (!tracks.size) continue
    const station = known ?? make(point.id, 'station')
    for (const p of station.platforms) {
      const pf = (record?.platforms ?? []).find(x => x.id === p.id)
      reach(p.trackId, pf?.startStation ?? p.at, pf?.endStation ?? p.at, p.at, p)
    }
    Object.assign(station, { kind: 'station', tracks: [...tracks.values()] })
  }
}

/** The buffer stops and boundaries of the record, made ends: each by its mark's id, shown by its name. */
function withEnds(record, byId) {
  const tracks = new Map((record?.tracks ?? []).map(tr => [tr.id, tr]))
  for (const mark of record?.endMarks ?? []) {
    const track = tracks.get(mark.trackId)
    if (!track) continue
    const length = trackLength(track)
    const at = endMarkStation(length, mark)
    byId.set(mark.id, {
      id: mark.id, key: mark.id, name: endMarkName(mark, track), uic: null, kind: 'end', pointId: null, platforms: [],
      tracks: [{ id: track.id, label: trackLabel(track), length, platforms: [], from: at, to: at, at }],
      end: { kind: mark.kind, markId: mark.id, speed: mark.kind === BOUNDARY && mark.speed > 0 ? mark.speed : null },
    })
  }
}

/**
 * The speed a train has at a place it begins or ends at [m/s]: at a boundary
 * its speed, or Infinity — what the line allows there —; elsewhere it stands, 0.
 */
export function crossingSpeed(station) {
  if (station?.end?.kind !== BOUNDARY) return 0
  return station.end.speed ? station.end.speed / 3.6 : Infinity
}

/** The station of that name among `stations` — the first, where several are called so —, or null. */
export const stationNamed = (stations, name) => {
  const key = stationKey(name)
  return (key && stations.find(s => stationKey(s.name) === key)) ?? null
}

/** The station of that id among `stations`, or null. */
export const stationById = (stations, id) => (id ? stations.find(s => s.id === id) ?? null : null)

/**
 * The place a train's point names: by its id (`place`); a point written
 * before places had ids, by its name (`station`).
 */
export const placeOf = (stations, point) => (point?.place ? stationById(stations, point.place) : stationNamed(stations, point?.station))

/**
 * Where a train may be at a station, for its way over the net: on the track
 * asked for — at its platform's middle, else its own —, else on any track
 * with a platform, and where it only passes, or the station has no platform,
 * on any of its tracks. A halt is its platforms. Locations
 * [{ trackId, station, direction: null }].
 */
export function stationTargets(station, { trackId = null, stop = true } = {}) {
  if (!station) return []
  const onTrack = (tr) => ({ trackId: tr.id, station: tr.platforms[0]?.at ?? tr.at ?? tr.length / 2, direction: null })
  if (station.kind === 'halt') {
    const ps = trackId ? station.platforms.filter(p => p.trackId === trackId) : station.platforms
    return ps.map(p => ({ trackId: p.trackId, station: p.at, direction: null }))
  }
  if (trackId) {
    const tr = station.tracks.find(x => x.id === trackId)
    return tr ? [onTrack(tr)] : []
  }
  const withPlatform = station.tracks.filter(tr => tr.platforms.length)
  return (stop && withPlatform.length ? withPlatform : station.tracks).map(onTrack)
}

/**
 * The stations a path runs through, in running order: [{ key, name, begin,
 * end, at }] — where it enters and leaves their tracks along the path, as
 * far as a station reaches on each, and `at` the middle of that, where a
 * train passing is timed. A halt is passed at its platform's middle, an end
 * where a train stands there.
 */
export function stationsAlong(path, stations) {
  const ofTrack = new Map()
  for (const s of stations) if (s.kind === 'station') for (const tr of s.tracks) ofTrack.set(tr.id, { s, from: tr.from ?? -Infinity, to: tr.to ?? Infinity })
  // a halt at its platforms' middles, an end where a train stands there
  const halts = stations.filter(s => s.kind === 'halt' || s.kind === 'end')
    .map(s => ({ s, at: s.kind === 'halt' ? s.platforms : s.tracks.map(tr => ({ trackId: tr.id, at: tr.at })) }))
  const out = []
  let offset = 0
  for (const piece of path) {
    const len = Math.abs(piece.end - piece.start)
    const lo = Math.min(piece.start, piece.end), hi = Math.max(piece.start, piece.end)
    const on = ofTrack.get(piece.trackId)
    if (on && Math.min(hi, on.to) >= Math.max(lo, on.from) - 1e-6) {
      const { s } = on
      // where along the path the piece is within the station's reach on the track
      const along = (station) => offset + Math.abs(station - piece.start)
      const [a, b] = [along(Math.max(lo, on.from)), along(Math.min(hi, on.to))].sort((x, y) => x - y)
      const last = out[out.length - 1]
      if (last?.key === s.key && Math.abs(last.end - a) < 1e-6) last.end = b
      else out.push({ key: s.key, name: s.name, begin: a, end: b })
    }
    for (const h of halts) {
      const p = h.at.find(x => x.trackId === piece.trackId && x.at >= lo - 1e-6 && x.at <= hi + 1e-6)
      if (p) {
        const at = offset + Math.abs(p.at - piece.start)
        out.push({ key: h.s.key, name: h.s.name, begin: at, end: at })
      }
    }
    offset += len
  }
  return out.sort((a, b) => a.begin - b.begin || a.end - b.end).map(s => ({ ...s, at: (s.begin + s.end) / 2 }))
}

/** The direction a path leaves a location in, for a way that goes on from it. */
export const lastDirection = (path) => (path.length ? directionOf(path[path.length - 1]) : null)

export { pathLength }
