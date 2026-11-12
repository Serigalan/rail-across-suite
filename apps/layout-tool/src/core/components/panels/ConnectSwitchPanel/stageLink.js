import { JOINT_TOL, portsOf, trackEndAt } from '@rail-across/core/track'

// What a construction stage connector (decision 0027) needs of the net: the
// track ends meeting at a place, the roles their tracks' days propose, and
// what those days say against the connector.

/** The track ends held by a switch or connector already, by `trackId|endpoint`. */
function heldEnds(switches) {
  const held = new Set()
  for (const sw of switches) {
    for (const { port } of portsOf(sw)) {
      const p = sw.ports?.[port]
      if (p?.trackId) held.add(`${p.trackId}|${p.endpoint}`)
    }
  }
  return held
}

/** The free track ends that meet where `end` lies, within the joint tolerance and in its plane: [{ track, endpoint, at }]. */
export function endsMeeting(tracks, switches, end) {
  const held = heldEnds(switches)
  const out = []
  for (const track of tracks) {
    for (const endpoint of ['BEGIN', 'END']) {
      if (held.has(`${track.id}|${endpoint}`)) continue
      const at = trackEndAt(track, endpoint)
      if (!at || Number(at.epsg) !== Number(end.epsg)) continue
      const d = Math.hypot(at.easting - end.easting, at.northing - end.northing)
      if (d <= JOINT_TOL) out.push({ track, endpoint, at, d })
    }
  }
  return out.sort((a, b) => a.d - b.d)
}

/**
 * The roles the ends take at a stage connector, as their tracks' days say: B1
 * the one that goes (in service until a day, or a removal), B2 the one that
 * comes (in service from a day, or new), A the one that stays.
 */
export function proposedRoles(ends) {
  const goes = (e) => Boolean(e.track.inServiceTo) || (e.track.status === 'removal' && !e.track.inServiceFrom)
  const comes = (e) => Boolean(e.track.inServiceFrom) || e.track.status === 'new'
  const b1 = ends.find(goes)
  const b2 = ends.find(e => e !== b1 && comes(e))
  const a = ends.find(e => e !== b1 && e !== b2)
  return [a, b1, b2].every(Boolean) ? { A: a, B1: b1, B2: b2 } : { A: ends[0], B1: ends[1], B2: ends[2] }
}

/** What the roles' tracks' days say against a stage connector: the one going should end the day before the other comes. */
export function stageLinkNote(roles) {
  const until = roles.B1?.track.inServiceTo, from = roles.B2?.track.inServiceFrom
  if (!until || !from) return 'stage_link_note_days'
  const next = new Date(`${until}T00:00:00Z`)
  next.setUTCDate(next.getUTCDate() + 1)
  const day = next.toISOString().slice(0, 10)
  if (from < day) return 'stage_link_note_overlap'
  if (from > day) return 'stage_link_note_gap'
  return null
}
