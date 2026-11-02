import { trackLength } from '@rail-across/core/track'
import { hasScreen, labelOf, newId } from './domain.js'
import { trackName } from './places.js'
import { stationName } from './stations.js'

/**
 * The stopping places (Halteplätze): where trains stop, a stretch of one
 * track — in a station on one of its tracks or at a platform, on the open
 * line at a platform. The signals and detection points bound them: what a
 * track or platform offers are the stretches between them, of which the
 * user takes one; the check says where one stands within a stopping place.
 */

/** Shorter stretches are no place to stop [m]. */
const SHORTEST = 5

/** What bounds a stopping place on a track: its real signals with a screen and its detection points, by station. */
export function boundsOn(domain, trackId, t) {
  const { signals, detectionPoints } = domain.collections
  const out = []
  for (const s of signals) {
    if (s.kind !== 'real' || !hasScreen(s)) continue
    for (const loc of [s.at, ...(s.alsoAt ?? [])]) {
      if (loc?.anchor === 'station' && loc.trackId === trackId) out.push({ station: loc.station, label: `${s.areaNumber ?? ''}${labelOf('signals', s, t)}`, signal: true })
    }
  }
  for (const d of detectionPoints) {
    if (d.at?.anchor === 'station' && d.at.trackId === trackId) out.push({ station: d.at.station, label: d.name ? labelOf('detectionPoints', d, t) : t('object_detectionPoints') })
  }
  // A signal first where it stands with a detection point at one place: it names the bound.
  return out.sort((a, b) => a.station - b.station || (a.signal ? -1 : 0) - (b.signal ? -1 : 0))
}

/**
 * The stretches a track offers between `from` and `to` — its own length, or
 * a platform's —, cut by what bounds them: [{ from, to, below, above }], the
 * labels of what bounds each at its two ends (null at the track's or the
 * platform's end), the short ones left out.
 */
export function stoppingStretches(net, domain, trackId, t, [from, to] = []) {
  const track = net.tracks.find(tr => tr.id === trackId)
  if (!track) return []
  const lo = from ?? 0, hi = to ?? trackLength(track)
  const cuts = boundsOn(domain, trackId, t).filter(b => b.station > lo + 0.5 && b.station < hi - 0.5)
  // One cut per place, named by the signal there where there is one.
  const placesCut = cuts.filter((b, i) => !cuts.slice(0, i).some(o => Math.abs(o.station - b.station) <= 0.5))
    .map(b => ({ station: b.station, label: boundAt(domain, trackId, b.station, t) }))
  const ends = [{ station: lo, label: boundAt(domain, trackId, lo, t) }, ...placesCut, { station: hi, label: boundAt(domain, trackId, hi, t) }]
  const out = []
  for (let i = 0; i + 1 < ends.length; i++) {
    const [a, b] = [ends[i], ends[i + 1]]
    if (b.station - a.station >= SHORTEST) out.push({ from: round(a.station), to: round(b.station), below: a.label, above: b.label })
  }
  return out
}

/** What stands at a station of a track, within half a metre — what bounds a stretch there —, or null. */
function boundAt(domain, trackId, station, t) {
  const here = boundsOn(domain, trackId, t).filter(b => Math.abs(b.station - station) <= 0.5)
  return (here.find(b => b.signal) ?? here[0])?.label ?? null
}

const round = (s) => Math.round(s * 1000) / 1000

/** A new stopping place on a stretch of a track: of a station, at a platform, or both. */
export const newStoppingPlace = ({ name, operationalPointId, platformId, trackId, from, to }) => Object.fromEntries(Object.entries({
  id: newId(), name, operationalPointId, platformId, at: { anchor: 'station', trackId, startStation: from, endStation: to },
}).filter(([, v]) => v !== undefined && v !== null && v !== ''))

/** The stopping places of a station — or, `pointId` null, those of the open line. */
export const stoppingPlacesOf = (domain, pointId) => domain.collections.stoppingPlaces.filter(h => (h.operationalPointId ?? null) === pointId)

/** Where a stopping place lies, as a list says it: its track and its stretch. */
export const placeWhere = (net, h) => `${trackName(net, h.at.trackId)} · ${Math.round(h.at.startStation)}–${Math.round(h.at.endStation)} m`

/** A stopping place as a choice names it: its station — or the open line —, then its own name. */
export function stoppingPlaceLabel(core, h, t) {
  const point = (core.collections.operationalPoints ?? []).find(p => p.id === h.operationalPointId)
  return `${point ? stationName(point) : t('open_line')} · ${h.name ?? labelOf('stoppingPlaces', h, t)}`
}

/** How long a stopping place is [m]. */
export const placeLength = (h) => Math.round(h.at.endStation - h.at.startStation)
