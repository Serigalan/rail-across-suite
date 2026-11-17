// topology — the diagram of the topological connections: graph, lanes along
// the lines, label placement. Computed by the core's Rust module (decision
// 0005: crates/track's topology), called with the arguments as the
// JavaScript took them; the Maps it answered with are given back as Maps.
//
// This file is the module's public face: code outside packages/core/src/topology/
// imports from here and nowhere else (tools/modules.mjs). See docs/modules/topology.md.

import { asMap, viaRust } from '../native/index.js'

/** The colours a selection's tracks are drawn in, in turn. */
export const TOPOLOGY_TRACK_COLORS = ['#ff8c00', '#1f9e3a', '#d0308f', '#0f9fb5', '#7b3fbf', '#a0662b']

/** The box a label of `text` takes: { w, h } in pixels. */
export const labelBox = viaRust('topology.labelBox')

/** Whether the segment [x1, y1, x2, y2] meets the box { x0, x1, y0, y1 }. */
export const segmentHitsBox = viaRust('topology.segmentHitsBox')

/**
 * Each label at the offset where it meets the least: `items` are
 * { key, text, x, y, offsets: [[dx, dy]] }. Returns Map(key → { x, y }), the centre of each label.
 */
export const placeLabels = viaRust('topology.placeLabels', asMap)

/**
 * The graph of the tracks' connections: { nodes: Map(id → node), edges } —
 * a node per switch or link and per free track end, an edge per track.
 */
export const buildTopologyGraph = viaRust('topology.buildTopologyGraph', ({ nodes, edges }) => ({ nodes: asMap(nodes), edges }))

/** The connected parts of a graph, largest first, and the loose tracks: { clusters, loose }. */
export const topologyClusters = viaRust('topology.topologyClusters')

/** What a selection highlights: { trackIds, switchIds, colors: { trackId → colour } }. */
export const selectionHighlight = viaRust('topology.selectionHighlight')

/** The tracks a switch's ports name. */
export const switchTrackIds = viaRust('topology.switchTrackIds')

/** A cluster laid out in lanes along its main line: { pos, routes, labels }, each a Map, in grid units. */
export const layoutClusterLanes = viaRust('topology.layoutClusterLanes', ({ pos, routes, labels }) => ({
  pos: asMap(pos), routes: asMap(routes), labels: asMap(labels),
}))
