// switch — turnouts and crossings: the forms of DB Ril 800.0120, their routes,
// the drawn symbol, where a turnout may lie, switch connections, connecting to
// an open track end, and what stays when a switch is deleted.
//
// The solvers of the dialogs — connecting a turnout to an open end, the
// switch connection and its middle chain, the station slider — are the core's
// Rust module (decision 0020, step 10: crates/switch), called on an input of
// the dialog's. The forms, the routes, the symbol and the rest the page draws
// with stay JavaScript: it asks them for every switch it draws.
//
// This file is the module's public face: code outside packages/core/src/switch/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/switch.md.

import { viaRust } from '../native/index.js'
import { SWITCH_CONNECTION_STAGES } from './catalogue.js'

export {
  ALL_SWITCH_TYPES, asRadius, bauform, CROSSING_TYPES, DEFAULT_SWITCH_TYPE_IDX, SWITCH_PICK_TYPES,
  switchBranchLength, switchBranchPointAt, switchBranchSections, switchFormChain, switchStraightLength,
  switchBranchSpeed, switchMark, switchTypeByLabel,
} from './catalogue.js'
export {
  computeCrossingGeometryUtm, crossingAngle, crossingElements, crossingEndDistance,
  crossingLegFitsTrack, crossingLegRadius, crossingLegSignedRadius, piecesOnRadius,
} from './crossing.js'
export { coupleSwitchCants } from './switchCants.js'
export { formDetail } from './detail.js'
export { switchDetailScene } from './detailScene.js'
export {
  negR, switchAbreast, switchBranchChain, switchChainBauform, switchChainJoints, switchChainBearingAt, switchChainPointUtm,
  switchChainTo, switchElementRoute, switchRouteBearingAt, switchRoutePointUtm, switchRouteVaries,
  utmEndRoute,
} from './route.js'
export { planSwitchDeletion, switchParts } from './switchDelete.js'
export {
  mainRouteLength, routeFrom, sleeperOffsets, sleeperThrough, switchLds, turnoutSleepers,
} from './sleepers.js'
export { clickStation, placeSwitchOnTrack, turnoutRunsOn } from './switchPlacement.js'
export {
  couplingOf, heightContext, ldsLift, planeFrom, roundMm, STATION_TOL, switchBodySpans, switchCouplings,
  trackHeightAt, Z_TOL,
} from './switchGradient.js'
export {
  computeSwitchGeometryUtm, crossingBodyUtm, crossingRoutesFromTracks, lcsLine, lcsLineUtm,
  rebuildSwitchSymbol, switchFillRing, switchLabelGeometry, switchRoutePointsUtm,
  switchRoutesFromTracks,
} from './symbol.js'
export { WEICHEN_REGELWERK, weichenGruppen } from './weichenRegelwerk.js'

// ── connecting a turnout to an open track end ──

/** Where a turnout connects to an open end of `track`: its anchor, `{ tracks, switches, endMarks, count }` around it. */
export const connectionAnchor = viaRust('switch.connectionAnchor')
/** How many elements the connecting chain may have, at the most. */
export const END_CHAIN_OPEN = 3
/** The chain a connection starts with at an anchor. */
export const endChainDefault = viaRust('switch.endChainDefault')
/** How many of a chain's values are left open to solve for. */
export const endChainOpen = viaRust('switch.endChainOpen')

// ── the switch connection ──

/** How many of the middle chain's values may be left open. */
export const MID_OPEN = 2
/** Which of the middle chain's values are open. */
export const midUnknowns = viaRust('switch.midUnknowns')
/** The forms of a connection's two turnouts: one form for both, or a pair. */
export const formPair = (form) => (Array.isArray(form) ? form : [form, form])
/** The speeds a switch connection is laid for, those of its first stage's forms. */
export const CONNECTION_SPEEDS = [...new Set(SWITCH_CONNECTION_STAGES[0].map(type => type.speed))]
/** Which speeds the two tracks can be connected with at a shift. */
export const computeSwitchConnections = viaRust('switch.computeSwitchConnections')
export { SWITCH_TYPES } from './catalogue.js'

// ── the station slider ──

/** The station nearest `s` within the slider's valid intervals. */
export const nearestValid = viaRust('switch.nearestValid')
/** Where the slider settles from `s`: { intervals, snaps, tol }. */
export const settleStation = viaRust('switch.settleStation')
