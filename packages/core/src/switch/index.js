// switch — turnouts and crossings: the forms of DB Ril 800.0120, their routes,
// the drawn symbol, where a turnout may lie, switch connections, connecting to
// an open track end, and what stays when a switch is deleted.
//
// This file is the module's public face: code outside packages/core/src/switch/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/switch.md.

export {
  ALL_SWITCH_TYPES, asRadius, bauform, CROSSING_TYPES, DEFAULT_SWITCH_TYPE_IDX, SWITCH_PICK_TYPES,
  switchBranchLength, switchBranchPointAt, switchBranchSections, switchFormChain, switchStraightLength,
  switchMark, switchTypeByLabel,
} from './catalogue.js'
export {
  connectionAnchor, connectionRoots, END_CHAIN_OPEN, endChainDefault, endChainOpen, fitConnectingElement,
  joinRouteIntoTrack, refitCount, refitTrackEnd, solveEndChain,
} from './connectToTrack.js'
export {
  computeCrossingGeometryUtm, crossingAngle, crossingElements, crossingEndDistance,
  crossingLegFitsTrack, crossingLegRadius, crossingLegSignedRadius, piecesOnRadius,
} from './crossing.js'
export { coupleSwitchCants } from './switchCants.js'
export { formDetail } from './detail.js'
export { switchDetailScene } from './detailScene.js'
export { DEFAULT_MIDS, MID_OPEN, midUnknowns } from './midChain.js'
export {
  negR, switchAbreast, switchBranchChain, switchChainBauform, switchChainJoints, switchChainBearingAt, switchChainPointUtm,
  switchChainTo, switchElementRoute, switchRouteBearingAt, switchRoutePointUtm, switchRouteVaries,
  utmEndRoute,
} from './route.js'
export { nearestValid, settleStation, uniqueStations, validIntervals } from './stationSlider.js'
export {
  buildConnectionElements, computeSwitchConnections, connectionBranchEnds, CONNECTION_SPEEDS,
  orientStemToward, solveSwitchConnection, SWITCH_TYPES,
} from './switchConnectionUtils.js'
export { planSwitchDeletion, switchParts } from './switchDelete.js'
export {
  mainRouteLength, routeFrom, sleeperOffsets, sleeperThrough, switchLds, turnoutSleepers,
} from './sleepers.js'
export { clickStation, placeSwitchOnTrack, turnoutRunsOn } from './switchPlacement.js'
export {
  computeSwitchGeometryUtm, crossingBodyUtm, crossingRoutesFromTracks, lcsLine, lcsLineUtm,
  rebuildSwitchSymbol, switchFillRing, switchLabelGeometry, switchRoutePointsUtm,
  switchRoutesFromTracks,
} from './symbol.js'
export { WEICHEN_REGELWERK, weichenGruppen } from './weichenRegelwerk.js'
