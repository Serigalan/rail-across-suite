import { snapNodeGaps, snapTrackJoints } from '../geometry/index.js'

/** What two places closer than were the same place before NODE_TOL was 1 µm [m]. */
export const LEGACY_NODE_TOL = 1e-3

/**
 * A record kept while the same place was 1 mm, closed to what it is now: every
 * joint within a track and every meeting of track ends that missed by up to a
 * millimetre is one node (snapNodeGaps, snapTrackJoints). Run once over every
 * project when the tolerance changed — what was joined then stays joined.
 *
 * Returns { record, closed } — `closed` how many joints were moved.
 */
export function closeLegacyGaps(record) {
  let closed = 0
  const tracks = (record.tracks ?? []).map(track => {
    const { elements, snapped } = snapNodeGaps(track.elements, track.epsg, LEGACY_NODE_TOL)
    closed += snapped.length
    return snapped.length ? { ...track, elements } : track
  })
  const joints = snapTrackJoints(tracks, LEGACY_NODE_TOL)
  closed += joints.snapped.length
  return { record: closed ? { ...record, tracks: joints.tracks } : record, closed }
}
