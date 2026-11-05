import { pointAtStation, trackLength } from '@rail-across/core/track'
import { LAYOUT, switchArms, walkFrom } from './detectionLayout.js'
import { added, newId } from './domain.js'

/**
 * The clearance markers (Grenzzeichen, Ra 12): every switch has one between
 * each two of its arms that leave it side by side, where the two tracks lie
 * 3.5 m apart — on the one of the two the switch names first, half way to
 * the other, facing the switch. One that stands within a few metres of that
 * place already counts; the others are set.
 */

/** The aspect of the clearance marker. */
export const CLEARANCE_MARKER = 'Ra_12'

/** How near a marker there is must stand to where one is due, to count [m]. */
const NEAR = 5

/** Whether a signal is a clearance marker: one that shows Ra 12. */
export const isClearanceMarker = (s) => s?.kind === 'real' && (s.frames ?? []).some(f => (f.aspects ?? []).some(a => a.aspect === CLEARANCE_MARKER))

/** The side of its track a point lies on, seen along the stationing: 'left' or 'right'. */
function sideOf(track, station, point) {
  const len = trackLength(track)
  const a = pointAtStation(track, Math.max(0, station - 1)), b = pointAtStation(track, Math.min(len, station + 1))
  const here = pointAtStation(track, station)
  if (!a || !b || !here) return 'right'
  const dx = b.utm.easting - a.utm.easting, dy = b.utm.northing - a.utm.northing
  const px = point.easting - here.utm.easting, py = point.northing - here.utm.northing
  return dx * py - dy * px > 0 ? 'left' : 'right'
}

/** The clearance markers the net asks for and the signals lack: [{ switchId, at }], `at` where each is set. */
export function clearanceMarkersDue(net, signals, apart = LAYOUT.clearance) {
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const byId = new Map(net.tracks.map(t => [t.id, t]))
  const there = signals.filter(s => isClearanceMarker(s) && s.at?.anchor === 'station')
  const out = []
  for (const { switchId, arms } of switchArms(net, apart)) {
    const done = new Set()
    for (const arm of arms) {
      if (!arm.near || arm.clear == null) continue
      const [first, second] = [arm, arm.near].sort((a, b) => a.port.localeCompare(b.port))
      const pair = `${first.port}|${second.port}`
      if (done.has(pair)) continue
      done.add(pair)
      const [here] = walkFrom(net, lengths, first.trackId, first.endpoint, arm.clear)
      const [other] = walkFrom(net, lengths, second.trackId, second.endpoint, arm.clear)
      if (!here || !other) continue
      if (there.some(m => [here, other].some(p => m.at.trackId === p.trackId && Math.abs(m.at.station - p.station) < NEAR))) continue
      const across = pointAtStation(byId.get(other.trackId), other.station)
      if (!across) continue
      out.push({
        switchId,
        at: {
          anchor: 'station', trackId: here.trackId, station: Math.round(here.station * 1000) / 1000,
          side: sideOf(byId.get(here.trackId), here.station, across.utm), offset: apart / 2,
          direction: here.away === 'up' ? 'down' : 'up',
        },
      })
    }
  }
  return out
}

/** A clearance marker at a place: a body Ra 12 of no system. */
export const newClearanceMarker = (at) => ({ id: newId(), kind: 'real', function: 'other', at, frames: [{ kind: 'body', aspects: [{ aspect: CLEARANCE_MARKER }] }] })

/** The domain with every clearance marker it lacks set: { domain, added } — how many. */
export function withClearanceMarkers(domain, net) {
  const due = clearanceMarkersDue(net, domain.collections.signals)
  return { domain: due.reduce((d, m) => added(d, 'signals', newClearanceMarker(m.at)), domain), added: due.length }
}
