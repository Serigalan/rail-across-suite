import { trackLabel } from '@rail-across/core/track'
import { aspectOf } from '@rail-across/core/signalling'

/** The name of a track as the Layout-Tool shows it. */
export const trackName = (net, id) => {
  const tr = net.tracks.find(x => x.id === id)
  return tr ? trackLabel(tr) || tr.name || id.slice(0, 8) : id?.slice(0, 8) ?? '—'
}

/** A location from where the plan was clicked: the track, station and side; acting `direction`. */
export const locationFrom = (hit, { direction = 'up', side = true } = {}) => ({
  anchor: 'station', trackId: hit.trackId, station: hit.station, ...(side ? { side: hit.side } : {}), ...(direction ? { direction } : {}),
})

/** The aspect's name as people write it, with its symbol: "Zs 3 (6)". */
export const aspectText = (a) => `${aspectOf(a.aspect)?.name ?? a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`
