import { moveTrackEnd, refitToNodes, snapNodeGaps, snapTrackJoints } from '@rail-across/core/geometry'

/**
 * How far apart two nodes of imported track may be and still be closed [m]:
 * 2 mm, ProVI's limit for an element ending beside its end point. The files
 * place every element from its own point, so their joints miss by the source's
 * quantisation; inside the model the same place is 1 µm (NODE_TOL), so the
 * import closes them. A wider gap stays open, and is reported where the import
 * reports one.
 */
const IMPORT_SNAP_TOL = 0.002

/** The elements of an imported chain, closed where they miss by no more than IMPORT_SNAP_TOL. */
export const snapImportedChain = (elements, epsg) => snapNodeGaps(elements, epsg, IMPORT_SNAP_TOL).elements

/** Imported tracks whose ends meet within IMPORT_SNAP_TOL, meeting at one node. */
export const snapImportedJoints = (tracks) => snapTrackJoints(tracks, IMPORT_SNAP_TOL).tracks

/**
 * An imported track beginning or ending at `point` (a plane point) where its
 * end lies within IMPORT_SNAP_TOL of it — a turnout's branch at the toe the
 * placement found on the stem.
 */
export function snapImportedEnd(track, endpoint, point) {
  const els = track.elements ?? []
  const node = endpoint === 'BEGIN' ? els[0]?.startNode : els[els.length - 1]?.endNode
  if (!Array.isArray(node) || !point) return track
  const gap = Math.hypot(node[0] - point.easting, node[1] - point.northing)
  return gap > 0 && gap <= IMPORT_SNAP_TOL ? moveTrackEnd(track, endpoint, [point.easting, point.northing]) : track
}

/**
 * Imported elements whose last one ends at `point` ([easting, northing]) where
 * it lies within IMPORT_SNAP_TOL of it: the end point the file states, which a
 * length the file rounds misses by its quantisation.
 */
export function snapImportedLastEnd(elements, epsg, point) {
  const last = elements[elements.length - 1]
  if (!Array.isArray(last?.endNode) || !Array.isArray(point)) return elements
  const gap = Math.hypot(last.endNode[0] - point[0], last.endNode[1] - point[1])
  if (!(gap > 0 && gap <= IMPORT_SNAP_TOL)) return elements
  return [...elements.slice(0, -1), refitToNodes({ ...last, endNode: [point[0], point[1]] }, epsg)]
}
