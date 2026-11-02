import { trackGroupKey, trackKind, trackLength } from '@rail-across/core/track'
import { directionOf, pathLength } from './path.js'

/**
 * The stations of a net, as the layout says them: a station (Bahnhof) is
 * the station tracks of one name (trackKind 'station', grouped as the
 * Layout-Tool groups them, by name, else by UIC number), with the platforms
 * on them or of its name; a halt (Haltepunkt) a name only platforms carry —
 * no track of its own, platforms on the line's tracks. A train begins and
 * ends at a station only, on one of its tracks; it stops or passes at either.
 */

const text = (v) => (v != null ? String(v).trim() : '')
const collator = new Intl.Collator('de', { numeric: true, sensitivity: 'base' })

/** The key a station is known by: `station:halle (saale) hbf`, or `station:#8010159` by its UIC number alone. */
export function stationKey(name) {
  const n = text(name).toLowerCase()
  return n ? `station:${n}` : null
}

/** What a track is called in a station: its number, else its name. */
export const trackLabel = (track) => text(track?.trackNumber) || text(track?.name) || String(track?.id ?? '').slice(0, 8)

const platformMiddle = (p) => (p.startStation + p.endStation) / 2

/**
 * [{ key, name, uic, kind: 'station' | 'halt', tracks: [{ id, label, length,
 * platforms }], platforms: [{ id, code, trackId, at }] }], sorted by name.
 * `name` is what a train's point names it by: the station's name, else its
 * UIC number. A platform's `at` is its middle on its track.
 */
export function stationsOf(record) {
  const byKey = new Map()
  const tracks = record?.tracks ?? []
  const platforms = (record?.platforms ?? []).filter(p => p.trackId && Number.isFinite(p.startStation) && Number.isFinite(p.endStation))
  for (const track of tracks) {
    if (trackKind(track) !== 'station') continue
    const groupKey = trackGroupKey(track)
    if (!groupKey) continue
    const name = text(track.stationName) || text(track.uicStation)
    const key = stationKey(name)
    let station = byKey.get(key)
    if (!station) byKey.set(key, station = { key, name, uic: text(track.uicStation) || null, kind: 'station', tracks: [], platforms: [] })
    station.uic ??= text(track.uicStation) || null
    station.tracks.push({ id: track.id, label: trackLabel(track), length: trackLength(track), platforms: [] })
  }
  const trackStation = new Map()
  for (const s of byKey.values()) for (const tr of s.tracks) trackStation.set(tr.id, s)
  for (const p of platforms) {
    const named = stationKey(p.stationName)
    let station = trackStation.get(p.trackId) ?? (named ? byKey.get(named) : null)
    if (!station && named) byKey.set(named, station = { key: named, name: text(p.stationName), uic: null, kind: 'halt', tracks: [], platforms: [] })
    if (!station) continue
    const platform = { id: p.id, code: text(p.code) || null, trackId: p.trackId, at: platformMiddle(p) }
    station.platforms.push(platform)
    station.tracks.find(tr => tr.id === p.trackId)?.platforms.push(platform)
  }
  for (const s of byKey.values()) s.tracks.sort((a, b) => collator.compare(a.label, b.label))
  return [...byKey.values()].sort((a, b) => collator.compare(a.name, b.name))
}

/** The station of that name among `stations`, or null. */
export const stationNamed = (stations, name) => stations.find(s => s.key === stationKey(name)) ?? null

/**
 * Where a train may be at a station, for its way over the net: on the track
 * asked for — at its platform's middle, else its own —, else on any track
 * with a platform, and where it only passes, or the station has no platform,
 * on any of its tracks. A halt is its platforms. Locations
 * [{ trackId, station, direction: null }].
 */
export function stationTargets(station, { trackId = null, stop = true } = {}) {
  if (!station) return []
  const onTrack = (tr) => ({ trackId: tr.id, station: tr.platforms[0]?.at ?? tr.length / 2, direction: null })
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
 * end, at }] — where it enters and leaves their tracks along the path, and
 * `at` the middle of that, where a train passing is timed. A halt is passed
 * at its platform's middle.
 */
export function stationsAlong(path, stations) {
  const ofTrack = new Map()
  for (const s of stations) for (const tr of s.tracks) ofTrack.set(tr.id, s)
  const halts = stations.filter(s => s.kind === 'halt')
  const out = []
  let offset = 0
  for (const piece of path) {
    const len = Math.abs(piece.end - piece.start)
    const s = ofTrack.get(piece.trackId)
    if (s) {
      const last = out[out.length - 1]
      if (last?.key === s.key && Math.abs(last.end - offset) < 1e-6) last.end = offset + len
      else out.push({ key: s.key, name: s.name, begin: offset, end: offset + len })
    }
    const lo = Math.min(piece.start, piece.end), hi = Math.max(piece.start, piece.end)
    for (const h of halts) {
      const p = h.platforms.find(x => x.trackId === piece.trackId && x.at >= lo - 1e-6 && x.at <= hi + 1e-6)
      if (p) {
        const at = offset + Math.abs(p.at - piece.start)
        out.push({ key: h.key, name: h.name, begin: at, end: at })
      }
    }
    offset += len
  }
  return out.sort((a, b) => a.begin - b.begin).map(s => ({ ...s, at: (s.begin + s.end) / 2 }))
}

/** The direction a path leaves a location in, for a way that goes on from it. */
export const lastDirection = (path) => (path.length ? directionOf(path[path.length - 1]) : null)

export { pathLength }
