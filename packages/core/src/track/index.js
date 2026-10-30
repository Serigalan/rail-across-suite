// track — the records a project is made of: tracks, their stationing and
// heights, switch records and their ports, links, track ends and buffer stops,
// platforms, routes, splitting and joining tracks.
//
// This file is the module's public face: code outside packages/core/src/track/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/track.md.

export {
  bandFindings, bandRange, bandRuns, bandValueAt, cantBand, curvatureBand,
  elementSpans, speedBand, withBandValue, withCantEnds,
} from './alignmentBands.js'
export { bufferStopFeatures, FACE_HALF_WIDTH } from './bufferStopGeometry.js'
export { DEFAULT_HEIGHT_EPSG, HEIGHT_DATUMS, heightDatumLabel } from './heightDatums.js'
export {
  adjacentTracks, elementAtStation, endOfIndex, GIVEN_MODES, gradientAt, heightAt, heightStations,
  insertHeightPoint, jointGroup, jointHeightUpdates, neighbourStub, pointAtStationUtm, pointGrades,
  reverseHeights, roundHeight, sliceHeights, solveHeightPoint, splitHeights, tangentLength,
  trackLength, trackProfile, trackSamplePoints, truncateHeights, verticalCurveOverlaps,
  verticalCurves,
} from './heightUtils.js'
export {
  buildTypeFields, generateId, LINE_CATEGORIES, nextSwitchNumber, SIDE_CODES, SIDE_NAMES,
  switchDesignation, switchNumberOf, switchNumbersInUse, TRACK_USES, TYPE_CODES, TYPE_NAMES,
} from './identifierUtils.js'
export { STATUS_COLOR, STATUSES, switchStatus, trackStatus } from './planStatus.js'
export { EMPTY_PLATFORM_FORM, pickedStation, platformDraft, platformForm, platformFormValid } from './platformForm.js'
export {
  carryPlatforms, DEFAULT_PLATFORM_HEIGHT, edgeOffsets, elementStations, flipPlatforms, PLATFORM_CODE_MAX,
  PLATFORM_FRONT_OFFSET, PLATFORM_HEIGHTS, platformEdgeElevation, platformLength, platformRing, pointAtStation,
  pointOnElement, rebuildPlatformSymbol, stationFromClick, stationOnTrack,
} from './platformUtils.js'
export {
  closeGaps, extendRoute, nextRouteName, partStation, repairRoute, repairRoutes, resolveRoute,
  routeAt, routePointAt, routeStationOf,
} from './routes.js'
export { mergeChain } from './mergeElements.js'
export { switchMatches, trackMatches } from './search.js'
export {
  DEFAULT_SWITCH_KIND, elementBelongsToSwitch, elementOnSwitchRoute, isLinkSwitch,
  isModelledSwitch, LINK_KIND, newSwitchFields, portEnd, portsOf, portTrack, switchElementMark,
  switchKindLabelKey, switchRouteLabelKey, switchRoutePorts, turnoutDivergingPort, turnoutLinePort,
  turnoutLineRoute, unmarkSwitchElement, withPort,
} from './switchModel.js'
export { classifyTrackEnds, freeEnds, isOpenState, switchNodes, tracksOnFrom } from './topology.js'
export {
  BOUNDARY, BUFFER_STOP, BUFFER_STOP_LENGTH, BUFFER_STOP_TYPES, bufferStopFits, bufferStopStations,
  DEFAULT_BUFFER_STOP_TYPE, defaultBrakeLength, endKey, flipEndMarks, newBoundary, newBufferStop,
  pruneEndMarks, remapEndMarks,
} from './trackEndMarks.js'
export {
  assignmentFields, assignTracks, groupHeading, groupTitle, groupTracks, kmTrackName, plannedNames,
  trackGroupKey, trackKind, trackListLabel, trackTypeName,
} from './trackGroups.js'
export { existingLinks, findTrackJoints, JOINT_TOL, linkAllJoints, linksForJoints, linkSymbol } from './trackLinkUtils.js'
export {
  flipSwitchEndpoints, inheritedSpeed, joinTracks, makeTrack, nextTrackName, portTracks,
  rebuildCoords, referencesTrack, remapSwitches, reverseTrack, trackEndAnchor, trackLabel,
} from './trackModel.js'
export { carveSwitchRoute, splitElementAt, splitTrackAtJoint } from './trackSplitUtils.js'
