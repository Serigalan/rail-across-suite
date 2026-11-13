// track — the records a project is made of: tracks, their stationing and
// heights, switch records and their ports, links, track ends and buffer stops,
// platforms, routes, splitting and joining tracks.
//
// The module is the core's Rust crate `crates/track` (decision 0021, phase D):
// its operations are calls of the core's module — an argument that is a part
// of the project the module holds crosses as its name, not as JSON (native
// holdProject). What stays JavaScript, measured: the small queries the pages
// ask point by point — a point or a height at a station, a switch's ports, a
// label (queries.js, held to the crate's vectors) —, the constants, ids drawn
// from the browser's randomness, a heading in the interface's language.
//
// This file is the module's public face: code outside packages/core/src/track/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/track.md.

import { asMap, reuseFrom, viaRust } from '../native/index.js'
import { groupTitle } from './queries.js'

// Where a contact wire runs follows from what carries it (electrification 3.0).
export { emptyElectrification, VOLTAGES, wireStretches } from './contactWires.js'

// The small queries the pages ask point by point stay JavaScript (queries.js).
export {
  edgeOffsets, elementAtStation, elementBelongsToSwitch, elementOnSwitchRoute, elementStations, endKey, endMarkName, endOfIndex, gradientAt, groupTitle, heightAt, heightStations, isLinkSwitch, isModelledSwitch, lifespanWithStatus, newSwitchFields, platformLength, pointAtStation, pointAtStationUtm, pointGrades, pointOnElement, portEnd, portTrack, portsOf, roundHeight, stationFromClick, stationOnTrack, switchDesignation, switchElementMark, switchKindLabelKey, switchNumberOf, switchPorts, switchRouteLabelKey, switchRoutePorts, switchRoutes, switchStatus, tangentLength, trackGroupKey, trackKind, trackLength, trackSamplePoints, trackStatus, trackTypeName, turnoutDivergingPort, turnoutLinePort, turnoutLineRoute, unmarkSwitchElement, withLifespan, withPort,
} from './queries.js'

export {
  BOUNDARY, BOUNDARY_HALF_WIDTH, BUFFER_STOP, BUFFER_STOP_LENGTH, BUFFER_STOP_TYPES,
  DEFAULT_BUFFER_STOP_TYPE, DEFAULT_HEIGHT_EPSG, DEFAULT_PLATFORM_HEIGHT, DEFAULT_SWITCH_KIND,
  EMPTY_PLATFORM_FORM, FACE_HALF_WIDTH, GIVEN_MODES, HEIGHT_DATUMS, JOINT_TOL, LINE_CATEGORIES,
  LINK_KIND, STAGE_LINK_KIND, PLATFORM_BACK_OFFSET, PLATFORM_CODE_MAX, PLATFORM_FRONT_OFFSET, PLATFORM_HEIGHTS, PLATFORM_WIDTH,
  SIDE_CODES, SIDE_NAMES, STATUSES, STATUS_COLOR, SWITCH_FORM_VERSION, SWITCH_KINDS, SWITCH_PORTS, SWITCH_ROUTES,
  TRACK_USES, TYPE_CODES, TYPE_NAMES,
} from './constants.js'

const asSet = (list) => new Set(list ?? [])
// Heights a part of a track has none of are undefined, as a track without them has none.
const orUndefined = (x) => (x === null ? undefined : x)
const heightsOrNone = (out, args) => orUndefined(reuseFrom(out, args))
const partsOrNone = (out, args) => reuseFrom(out, args).map(orUndefined)
// A split names its halves from \`names\` (a Set) and takes those names there, and
// says which half lies ahead as one of the halves it answers.
const split = (at) => (out, args) => {
  const kept = reuseFrom(out, args)
  if (args[at] instanceof Set) for (const t of kept?.tracks ?? []) if (t?.name) args[at].add(t.name)
  if (kept?.ahead) kept.ahead = kept.tracks?.find(t => t.id === kept.ahead.id) ?? kept.ahead
  return kept
}
const plannedRevive = (out) => ({ names: asMap(out?.names), fallback: asSet(out?.fallback), clashes: asSet(out?.clashes) })

export const adjacentTracks = viaRust('track.adjacentTracks', reuseFrom)
export const assignTracks = viaRust('track.assignTracks', reuseFrom)
export const assignmentFields = viaRust('track.assignmentFields', reuseFrom)
export const bandFindings = viaRust('track.bandFindings', reuseFrom)
export const bandRange = viaRust('track.bandRange', reuseFrom)
export const bandRuns = viaRust('track.bandRuns', reuseFrom)
export const bandValueAt = viaRust('track.bandValueAt', reuseFrom)
export const boundaryFeatures = viaRust('track.boundaryFeatures', reuseFrom)
export const bufferStopFeatures = viaRust('track.bufferStopFeatures', reuseFrom)
export const bufferStopFits = viaRust('track.bufferStopFits', reuseFrom)
export const bufferStopStations = viaRust('track.bufferStopStations', reuseFrom)
export const buildTypeFields = viaRust('track.buildTypeFields', reuseFrom)
export const cantBand = viaRust('track.cantBand', reuseFrom)
export const carryPlatforms = viaRust('track.carryPlatforms', reuseFrom)
export const carveSwitchRoute = viaRust('track.carveSwitchRoute', reuseFrom)
export const claimedEnds = viaRust('track.claimedEnds', asSet)
export const classifyTrackEnds = viaRust('track.classifyTrackEnds', reuseFrom)
export const closeGaps = viaRust('track.closeGaps', reuseFrom)
export const curvatureBand = viaRust('track.curvatureBand', reuseFrom)
export const elementSpans = viaRust('track.elementSpans', reuseFrom)
export const endMarkStation = viaRust('track.endMarkStation', reuseFrom)
export const existingLinks = viaRust('track.existingLinks', reuseFrom)
export const extendRoute = viaRust('track.extendRoute', reuseFrom)
export const findTrackJoints = viaRust('track.findTrackJoints', reuseFrom)
export const flipEndMarks = viaRust('track.flipEndMarks', reuseFrom)
export const flipPlatforms = viaRust('track.flipPlatforms', reuseFrom)
export const flipSwitchEndpoints = viaRust('track.flipSwitchEndpoints', reuseFrom)
export const freeEnds = viaRust('track.freeEnds', reuseFrom)
export const groupTracks = viaRust('track.groupTracks', reuseFrom)
export const heightDatumLabel = viaRust('track.heightDatumLabel', reuseFrom)
export const inheritedSpeed = viaRust('track.inheritedSpeed', reuseFrom)
export const insertHeightPoint = viaRust('track.insertHeightPoint', reuseFrom)
export const joinHeights = viaRust('track.joinHeights', heightsOrNone)
export const joinTracks = viaRust('track.joinTracks', reuseFrom)
export const jointFit = viaRust('track.jointFit', reuseFrom)
export const jointGroup = viaRust('track.jointGroup', reuseFrom)
export const jointHeightUpdates = viaRust('track.jointHeightUpdates', asMap)
export const kmForTrackPoint = viaRust('track.kmForTrackPoint', reuseFrom)
export const kmOnRuns = viaRust('track.kmOnRuns', reuseFrom)
export const kmTrackName = viaRust('track.kmTrackName', reuseFrom)
export const linkAllJoints = viaRust('track.linkAllJoints', reuseFrom)
export const linkRecord = viaRust('track.linkRecord', reuseFrom)
export const linkSymbol = viaRust('track.linkSymbol', reuseFrom)
export const linksForJoints = viaRust('track.linksForJoints', reuseFrom)
export const makeTrack = viaRust('track.makeTrack', reuseFrom)
export const mergeChain = viaRust('track.mergeChain', reuseFrom)
export const neighbourStub = viaRust('track.neighbourStub', reuseFrom)
export const newBoundary = viaRust('track.newBoundary', reuseFrom)
export const newBufferStop = viaRust('track.newBufferStop', reuseFrom)
export const nextRouteName = viaRust('track.nextRouteName', reuseFrom)
export const nextSwitchNumber = viaRust('track.nextSwitchNumber', reuseFrom)
export const nextTrackName = viaRust('track.nextTrackName', reuseFrom)
export const partStation = viaRust('track.partStation', reuseFrom)
export const pickedStation = viaRust('track.pickedStation', reuseFrom)
export const piecesAlong = viaRust('track.piecesAlong', reuseFrom)
export const planeRuns = viaRust('track.planeRuns', reuseFrom)
export const plannedNames = viaRust('track.plannedNames', plannedRevive)
export const platformDraft = viaRust('track.platformDraft', reuseFrom)
export const platformEdgeElevation = viaRust('track.platformEdgeElevation', reuseFrom)
export const platformForm = viaRust('track.platformForm', reuseFrom)
export const platformFormValid = viaRust('track.platformFormValid', reuseFrom)
export const platformRing = viaRust('track.platformRing', reuseFrom)
export const portTracks = viaRust('track.portTracks', reuseFrom)
export const pruneEndMarks = viaRust('track.pruneEndMarks', reuseFrom)
export const rebuildCoords = viaRust('track.rebuildCoords', reuseFrom)
export const rebuildPlatformSymbol = viaRust('track.rebuildPlatformSymbol', reuseFrom)
export const referencesTrack = viaRust('track.referencesTrack', reuseFrom)
export const remapEndMarks = viaRust('track.remapEndMarks', reuseFrom)
export const remapSwitches = viaRust('track.remapSwitches', reuseFrom)
export const resolveRoute = viaRust('track.resolveRoute', reuseFrom)
export const reverseHeights = viaRust('track.reverseHeights', reuseFrom)
export const reverseTrack = viaRust('track.reverseTrack', reuseFrom)
export const routeAt = viaRust('track.routeAt', reuseFrom)
export const routePath = viaRust('track.routePath', reuseFrom)
export const routePointAt = viaRust('track.routePointAt', reuseFrom)
export const routeStationOf = viaRust('track.routeStationOf', reuseFrom)
export const sliceHeights = viaRust('track.sliceHeights', heightsOrNone)
export const solveHeightPoint = viaRust('track.solveHeightPoint', reuseFrom)
export const speedBand = viaRust('track.speedBand', reuseFrom)
export const splitElementAt = viaRust('track.splitElementAt', split(4))
export const splitHeights = viaRust('track.splitHeights', partsOrNone)
export const splitTrackAtJoint = viaRust('track.splitTrackAtJoint', split(3))
export const stationTrackStem = viaRust('track.stationTrackStem', reuseFrom)
export const switchMatches = viaRust('track.switchMatches', reuseFrom)
export const switchNodes = viaRust('track.switchNodes', reuseFrom)
export const switchNumbersInUse = viaRust('track.switchNumbersInUse', asSet)
export const trackEndAnchor = viaRust('track.trackEndAnchor', reuseFrom)
export const trackEndAt = viaRust('track.trackEndAt', reuseFrom)
export const trackLabel = viaRust('track.trackLabel', reuseFrom)
export const trackListLabel = viaRust('track.trackListLabel', reuseFrom)
export const trackMatches = viaRust('track.trackMatches', reuseFrom)
export const trackProfile = viaRust('track.trackProfile', reuseFrom)
export const tracksOnFrom = viaRust('track.tracksOnFrom', reuseFrom)
export const truncateHeights = viaRust('track.truncateHeights', heightsOrNone)
export const verticalCurve = viaRust('track.verticalCurve', reuseFrom)
export const verticalCurveOverlaps = viaRust('track.verticalCurveOverlaps', reuseFrom)
export const verticalCurves = viaRust('track.verticalCurves', reuseFrom)
export const withBandValue = viaRust('track.withBandValue', reuseFrom)
export const withCantEnds = viaRust('track.withCantEnds', reuseFrom)

// The module's other operations, for what asks them directly.
export const mergeableRun = viaRust('track.mergeableRun', reuseFrom)
export const offsetEdgeCoords = viaRust('track.offsetEdgeCoords', reuseFrom)

/**
 * \`after\` with each route over a track the write took away run over what lies
 * there now (decision 249) — \`after\` itself where it has no routes.
 */
export const repairRoutes = (before, after) => (after?.routes?.length ? repairRoutesOf(before, after) : after)
const repairRoutesOf = viaRust('track.repairRoutes', reuseFrom)

/** A new id for a track, a switch or any record: drawn from the browser's randomness. */
export const generateId = () => crypto.randomUUID()

/** The brake length a buffer stop type proposes [m]: its own number. */
export const defaultBrakeLength = (type) => Number(type)

/** Whether a track end in this state is free to connect to. */
export const isOpenState = (state) => state === 'open'

/** A group's heading in a list of tracks, in the interface's language (`t`). */
export function groupHeading(t, group) {
  if (!group.kind) return t('track_group_none')
  return `${t(group.kind === 'line' ? 'track_group_line' : 'track_group_station')} ${groupTitle(group)}`
}
