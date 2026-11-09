// track — the records a project is made of: tracks, their stationing and
// heights, switch records and their ports, links, track ends and buffer stops,
// platforms, routes, splitting and joining tracks.
//
// The module is the core's Rust crate `crates/track` (decision 0021, phase D:
// its JavaScript went, its vectors are frozen): every function here is a call
// of the core's module — an argument that is a part of the project the module
// holds crosses as its name, not as JSON (native holdProject). What stays
// JavaScript is what the page lists and labels: the constants, ids drawn from
// the browser's randomness, a heading in the interface's language.
//
// This file is the module's public face: code outside packages/core/src/track/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/track.md.

import { asMap, reuseFrom, viaRust } from '../native/index.js'

export {
  BOUNDARY, BOUNDARY_HALF_WIDTH, BUFFER_STOP, BUFFER_STOP_LENGTH, BUFFER_STOP_TYPES,
  DEFAULT_BUFFER_STOP_TYPE, DEFAULT_HEIGHT_EPSG, DEFAULT_PLATFORM_HEIGHT, DEFAULT_SWITCH_KIND,
  EMPTY_PLATFORM_FORM, FACE_HALF_WIDTH, GIVEN_MODES, HEIGHT_DATUMS, JOINT_TOL, LINE_CATEGORIES,
  LINK_KIND, PLATFORM_BACK_OFFSET, PLATFORM_CODE_MAX, PLATFORM_FRONT_OFFSET, PLATFORM_HEIGHTS, PLATFORM_WIDTH,
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
export const edgeOffsets = viaRust('track.edgeOffsets', reuseFrom)
export const elementAtStation = viaRust('track.elementAtStation', reuseFrom)
export const elementBelongsToSwitch = viaRust('track.elementBelongsToSwitch', reuseFrom)
export const elementOnSwitchRoute = viaRust('track.elementOnSwitchRoute', reuseFrom)
export const elementSpans = viaRust('track.elementSpans', reuseFrom)
export const elementStations = viaRust('track.elementStations', reuseFrom)
export const endKey = viaRust('track.endKey', reuseFrom)
export const endMarkName = viaRust('track.endMarkName', reuseFrom)
export const endMarkStation = viaRust('track.endMarkStation', reuseFrom)
export const endOfIndex = viaRust('track.endOfIndex', reuseFrom)
export const existingLinks = viaRust('track.existingLinks', reuseFrom)
export const extendRoute = viaRust('track.extendRoute', reuseFrom)
export const findTrackJoints = viaRust('track.findTrackJoints', reuseFrom)
export const flipEndMarks = viaRust('track.flipEndMarks', reuseFrom)
export const flipPlatforms = viaRust('track.flipPlatforms', reuseFrom)
export const flipSwitchEndpoints = viaRust('track.flipSwitchEndpoints', reuseFrom)
export const freeEnds = viaRust('track.freeEnds', reuseFrom)
export const gradientAt = viaRust('track.gradientAt', reuseFrom)
export const groupTitle = viaRust('track.groupTitle', reuseFrom)
export const groupTracks = viaRust('track.groupTracks', reuseFrom)
export const heightAt = viaRust('track.heightAt', reuseFrom)
export const heightDatumLabel = viaRust('track.heightDatumLabel', reuseFrom)
export const heightStations = viaRust('track.heightStations', reuseFrom)
export const inheritedSpeed = viaRust('track.inheritedSpeed', reuseFrom)
export const insertHeightPoint = viaRust('track.insertHeightPoint', reuseFrom)
export const isLinkSwitch = viaRust('track.isLinkSwitch', reuseFrom)
export const isModelledSwitch = viaRust('track.isModelledSwitch', reuseFrom)
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
export const newSwitchFields = viaRust('track.newSwitchFields', reuseFrom)
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
export const platformLength = viaRust('track.platformLength', reuseFrom)
export const platformRing = viaRust('track.platformRing', reuseFrom)
export const pointAtStation = viaRust('track.pointAtStation', reuseFrom)
export const pointAtStationUtm = viaRust('track.pointAtStationUtm', reuseFrom)
export const pointGrades = viaRust('track.pointGrades', reuseFrom)
export const pointOnElement = viaRust('track.pointOnElement', reuseFrom)
export const portEnd = viaRust('track.portEnd', reuseFrom)
export const portTrack = viaRust('track.portTrack', reuseFrom)
export const portTracks = viaRust('track.portTracks', reuseFrom)
export const portsOf = viaRust('track.portsOf', reuseFrom)
export const pruneEndMarks = viaRust('track.pruneEndMarks', reuseFrom)
export const rebuildCoords = viaRust('track.rebuildCoords', reuseFrom)
export const rebuildPlatformSymbol = viaRust('track.rebuildPlatformSymbol', reuseFrom)
export const referencesTrack = viaRust('track.referencesTrack', reuseFrom)
export const remapEndMarks = viaRust('track.remapEndMarks', reuseFrom)
export const remapSwitches = viaRust('track.remapSwitches', reuseFrom)
export const repairRoutes = viaRust('track.repairRoutes', reuseFrom)
export const resolveRoute = viaRust('track.resolveRoute', reuseFrom)
export const reverseHeights = viaRust('track.reverseHeights', reuseFrom)
export const reverseTrack = viaRust('track.reverseTrack', reuseFrom)
export const roundHeight = viaRust('track.roundHeight', reuseFrom)
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
export const stationFromClick = viaRust('track.stationFromClick', reuseFrom)
export const stationOnTrack = viaRust('track.stationOnTrack', reuseFrom)
export const stationTrackStem = viaRust('track.stationTrackStem', reuseFrom)
export const switchDesignation = viaRust('track.switchDesignation', reuseFrom)
export const switchElementMark = viaRust('track.switchElementMark', reuseFrom)
export const switchKindLabelKey = viaRust('track.switchKindLabelKey', reuseFrom)
export const switchMatches = viaRust('track.switchMatches', reuseFrom)
export const switchNodes = viaRust('track.switchNodes', reuseFrom)
export const switchNumberOf = viaRust('track.switchNumberOf', reuseFrom)
export const switchNumbersInUse = viaRust('track.switchNumbersInUse', asSet)
export const switchPorts = viaRust('track.switchPorts', reuseFrom)
export const switchRouteLabelKey = viaRust('track.switchRouteLabelKey', reuseFrom)
export const switchRoutePorts = viaRust('track.switchRoutePorts', reuseFrom)
export const switchRoutes = viaRust('track.switchRoutes', reuseFrom)
export const switchStatus = viaRust('track.switchStatus', reuseFrom)
export const tangentLength = viaRust('track.tangentLength', reuseFrom)
export const trackEndAnchor = viaRust('track.trackEndAnchor', reuseFrom)
export const trackEndAt = viaRust('track.trackEndAt', reuseFrom)
export const trackGroupKey = viaRust('track.trackGroupKey', reuseFrom)
export const trackKind = viaRust('track.trackKind', reuseFrom)
export const trackLabel = viaRust('track.trackLabel', reuseFrom)
export const trackLength = viaRust('track.trackLength', reuseFrom)
export const trackListLabel = viaRust('track.trackListLabel', reuseFrom)
export const trackMatches = viaRust('track.trackMatches', reuseFrom)
export const trackProfile = viaRust('track.trackProfile', reuseFrom)
export const trackSamplePoints = viaRust('track.trackSamplePoints', reuseFrom)
export const trackStatus = viaRust('track.trackStatus', reuseFrom)
export const trackTypeName = viaRust('track.trackTypeName', reuseFrom)
export const tracksOnFrom = viaRust('track.tracksOnFrom', reuseFrom)
export const truncateHeights = viaRust('track.truncateHeights', heightsOrNone)
export const turnoutDivergingPort = viaRust('track.turnoutDivergingPort', reuseFrom)
export const turnoutLinePort = viaRust('track.turnoutLinePort', reuseFrom)
export const turnoutLineRoute = viaRust('track.turnoutLineRoute', reuseFrom)
export const unmarkSwitchElement = viaRust('track.unmarkSwitchElement', reuseFrom)
export const verticalCurve = viaRust('track.verticalCurve', reuseFrom)
export const verticalCurveOverlaps = viaRust('track.verticalCurveOverlaps', reuseFrom)
export const verticalCurves = viaRust('track.verticalCurves', reuseFrom)
export const withBandValue = viaRust('track.withBandValue', reuseFrom)
export const withCantEnds = viaRust('track.withCantEnds', reuseFrom)
export const withPort = viaRust('track.withPort', reuseFrom)

// The module's other operations, for what asks them directly.
export const mergeableRun = viaRust('track.mergeableRun', reuseFrom)
export const offsetEdgeCoords = viaRust('track.offsetEdgeCoords', reuseFrom)

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
