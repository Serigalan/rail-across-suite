import { portsOf, portTrack, portEnd, trackLength, tracksOnFrom } from '@rail-across/core/track'

/**
 * Runs over the net: the tracks a route takes from its start signal to its
 * target, and the run of an overlap of a given length beyond its signal.
 * A run leaves a track by the end it heads for — END for a location acting
 * up, BEGIN for one acting down — and enters the next where the two meet
 * (tracksOnFrom: across a switch along its routes, a link, a joint).
 */

const away = (direction) => (direction === 'down' ? 'BEGIN' : 'END')
const other = (end) => (end === 'BEGIN' ? 'END' : 'BEGIN')

/**
 * The shortest run from one location to another, both on the net and acting
 * the way the run goes: [trackId] in running order, or null where none
 * leads there. A target ahead on the start's own track is that track alone.
 */
export function findRun(net, from, to) {
  if (from?.anchor !== 'station' || to?.anchor !== 'station') return null
  const { tracks, switches } = net
  const lengths = new Map(tracks.map(t => [t.id, trackLength(t)]))
  if (from.trackId === to.trackId && (from.direction ?? 'up') === (to.direction ?? 'up')) {
    const ahead = (from.direction ?? 'up') === 'up' ? to.station >= from.station : to.station <= from.station
    if (ahead) return [from.trackId]
  }
  // Dijkstra over (track, the end it is left by); the cost what is run of each.
  const startLeft = (from.direction ?? 'up') === 'up' ? lengths.get(from.trackId) - from.station : from.station
  const queue = [{ trackId: from.trackId, leave: away(from.direction), cost: startLeft, path: [from.trackId] }]
  const seen = new Set()
  while (queue.length) {
    queue.sort((a, b) => a.cost - b.cost)
    const cur = queue.shift()
    const key = `${cur.trackId}:${cur.leave}`
    if (seen.has(key)) continue
    seen.add(key)
    for (const next of tracksOnFrom(cur.trackId, cur.leave, tracks, switches)) {
      const leave = other(next.endpoint)
      const direction = next.endpoint === 'BEGIN' ? 'up' : 'down'
      const path = [...cur.path, next.trackId]
      if (next.trackId === to.trackId && (to.direction ?? direction) === direction) return path
      if (path.length > 60) continue
      queue.push({ trackId: next.trackId, leave, cost: cur.cost + (lengths.get(next.trackId) ?? 0), path })
    }
  }
  return null
}

/**
 * The run between two locations, in either direction, whichever leads there
 * (a speed section holds both ways unless it says otherwise): [trackId], or null.
 */
export const runBetween = (net, from, to) =>
  findRun(net, { ...from, direction: 'up' }, { ...to, direction: undefined }) ?? findRun(net, { ...from, direction: 'down' }, { ...to, direction: undefined })

/**
 * The stretches a run covers from `start` to `end`: [{ trackId, from, to }] —
 * of its first track from the start to the end it leaves by, every track
 * between whole, of its last from the end it enters by to the end.
 */
export function runStretches(net, start, trackIds, end) {
  if (!trackIds?.length || start?.anchor !== 'station' || end?.anchor !== 'station') return []
  const length = (id) => trackLength(net.tracks.find(t => t.id === id) ?? { elements: [] })
  if (trackIds.length === 1) return [{ trackId: trackIds[0], from: start.station, to: end.station }]
  const out = []
  let entered = null
  trackIds.forEach((id, i) => {
    const next = trackIds[i + 1]
    // The end it leaves by: the one the next track meets.
    const leave = next == null ? null
      : tracksOnFrom(id, 'END', net.tracks, net.switches).some(x => x.trackId === next) ? 'END' : 'BEGIN'
    const at = (endpoint) => (endpoint === 'BEGIN' ? 0 : length(id))
    const from = i === 0 ? start.station : at(entered)
    const to = leave == null ? end.station : at(leave)
    out.push({ trackId: id, from, to })
    if (next != null) entered = tracksOnFrom(id, leave, net.tracks, net.switches).find(x => x.trackId === next)?.endpoint ?? 'BEGIN'
  })
  return out
}

/** The port of `sw` holding a track's end, or null. */
const portHolding = (sw, trackId, endpoint) => portsOf(sw).find(p => portTrack(sw, p.port) === trackId && portEnd(sw, p.port) === endpoint)?.port ?? null

/**
 * The run of `length` metres from a location in the way it acts: { trackIds,
 * end: { trackId, station } } — at a switch it takes the straight route (B2
 * of a turnout, C of a crossing) where it has the choice. Where the net ends
 * first, the run ends there.
 */
export function runAhead(net, from, length) {
  if (from?.anchor !== 'station') return null
  const { tracks, switches } = net
  const lengths = new Map(tracks.map(t => [t.id, trackLength(t)]))
  let trackId = from.trackId
  let direction = from.direction ?? 'up'
  let station = from.station
  let left = length
  const trackIds = [trackId]
  for (let guard = 0; guard < 60; guard++) {
    const len = lengths.get(trackId) ?? 0
    const room = direction === 'up' ? len - station : station
    if (room >= left) {
      const end = direction === 'up' ? station + left : station - left
      return { trackIds, end: { trackId, station: Math.round(end * 1000) / 1000 } }
    }
    left -= room
    const leave = away(direction)
    const nexts = tracksOnFrom(trackId, leave, tracks, switches)
    if (!nexts.length) return { trackIds, end: { trackId, station: direction === 'up' ? len : 0 } }
    const straight = nexts.find(n => switches.some(sw => ['B2', 'C'].includes(portHolding(sw, n.trackId, n.endpoint)))) ?? nexts[0]
    trackId = straight.trackId
    direction = straight.endpoint === 'BEGIN' ? 'up' : 'down'
    station = direction === 'up' ? 0 : lengths.get(trackId) ?? 0
    if (trackIds.includes(trackId)) break
    trackIds.push(trackId)
  }
  return { trackIds, end: { trackId, station } }
}

/** The switches a run of tracks passes: those with ports on two consecutive tracks of it. */
export function switchesOnRun(net, trackIds) {
  const out = []
  for (let i = 0; i + 1 < trackIds.length; i++) {
    for (const sw of net.switches) {
      const held = portsOf(sw).map(p => portTrack(sw, p.port))
      if (held.includes(trackIds[i]) && held.includes(trackIds[i + 1]) && !out.includes(sw.switchId)) out.push(sw.switchId)
    }
  }
  return out
}
