import { reconstructElements } from './elementReconstruct.js'
import { computeCurvedValuesUtm, computeStraightValuesUtm } from './elementUtils.js'

/**
 * An element whose node was moved, its values taken from its nodes again: a
 * straight and an arc their length and bearings, as splitting one does (see
 * untrueLengths); a transition, whose nodes do not state its design values,
 * keeps them and changes its length by `lengthChange` [m] — the distance its
 * node was moved along the track, where the caller knows it.
 */
export function refitToNodes(el, epsg, lengthChange = 0) {
  const startUtm = { easting: el.startNode[0], northing: el.startNode[1], zone: epsg }
  const endUtm = { easting: el.endNode[0], northing: el.endNode[1], zone: epsg }
  let out = el
  if (el.radius != null) {
    const { length, bearing, endBearing } = computeCurvedValuesUtm(startUtm, endUtm, el.radius)
    if (length > 0) out = { ...el, length, bearing, ...(el.endBearing != null ? { endBearing } : {}) }
  } else if (el.elementType !== 2) {
    const { length, bearing } = computeStraightValuesUtm(startUtm, endUtm)
    out = { ...el, length, bearing }
  } else if (lengthChange) {
    out = { ...el, length: el.length + lengthChange }
  }
  return epsg ? reconstructElements([out], epsg)[0] : out
}

/**
 * A chain closed where its nodes miss each other by no more than `tol` [m]:
 * each such element ends where the next one begins (refitToNodes). The start
 * is what a file states as a point; the end is computed from a length the file
 * rounds, so the end is the one that moves. What foreign data brings along,
 * and what records kept to 1 µm bring along, is closed this way: at the import
 * and once for every project kept before NODE_TOL was 1 µm.
 *
 * Returns { elements, snapped: [{ index, gap }] } — `index` the element that
 * now ends where the next one begins, as nodeGaps counts.
 */
export function snapNodeGaps(elements, epsg, tol) {
  const out = [...(elements ?? [])]
  const snapped = []
  for (let i = 0; i + 1 < out.length; i++) {
    const end = out[i].endNode, start = out[i + 1].startNode
    if (!Array.isArray(end) || !Array.isArray(start) || !Array.isArray(out[i].startNode)) continue
    const gap = Math.hypot(end[0] - start[0], end[1] - start[1])
    if (!(gap > 0 && gap <= tol)) continue
    out[i] = refitToNodes({ ...out[i], endNode: [start[0], start[1]] }, epsg)
    snapped.push({ index: i, gap })
  }
  return { elements: out, snapped }
}

/** `track` with the node at `endpoint` moved to `node`, the element there refitted to it. */
export function moveTrackEnd(track, endpoint, node) {
  const elements = [...track.elements]
  const k = endpoint === 'BEGIN' ? 0 : elements.length - 1
  const key = endpoint === 'BEGIN' ? 'startNode' : 'endNode'
  elements[k] = refitToNodes({ ...elements[k], [key]: [node[0], node[1]] }, track.epsg)
  return { ...track, elements }
}

/**
 * Tracks whose ends meet within `tol` [m] meeting at one node: where a track
 * begins stays, a track that ends there ends at it (as snapNodeGaps within a
 * track); of two ends alike the earlier track's stays. Ends in different
 * planes are never one node.
 *
 * Returns { tracks, snapped: [{ trackId, endpoint, gap }] }.
 */
export function snapTrackJoints(tracks, tol) {
  const out = [...(tracks ?? [])]
  const snapped = []
  const nodeOf = (i, endpoint) => {
    const els = out[i].elements ?? []
    return endpoint === 'BEGIN' ? els[0]?.startNode : els[els.length - 1]?.endNode
  }
  const anchors = []
  const ends = ['BEGIN', 'END'].flatMap(endpoint => out.map((_, i) => ({ i, endpoint })))
  for (const { i, endpoint } of ends) {
    const node = nodeOf(i, endpoint)
    if (!Array.isArray(node)) continue
    const epsg = Number(out[i].epsg)
    const anchor = anchors.find(a => a.i !== i && a.epsg === epsg
      && Math.hypot(a.node[0] - node[0], a.node[1] - node[1]) <= tol)
    if (!anchor) { anchors.push({ i, epsg, node: [node[0], node[1]] }); continue }
    const gap = Math.hypot(anchor.node[0] - node[0], anchor.node[1] - node[1])
    if (!(gap > 0)) continue
    out[i] = moveTrackEnd(out[i], endpoint, anchor.node)
    snapped.push({ trackId: out[i].id, endpoint, gap })
  }
  return { tracks: out, snapped }
}
