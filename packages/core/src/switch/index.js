// switch — turnouts and crossings: the forms of DB Ril 800.0120, their routes,
// the drawn symbol, where a turnout may lie, switch connections, connecting to
// an open track end, and what stays when a switch is deleted.
//
// The module is the core's Rust crate `crates/switch` (decision 0021, phase D:
// its JavaScript went, its vectors are frozen): every function here is a call
// of the core's module — an argument that is a part of the project the module
// holds crosses as its name, not as JSON (native holdProject). What stays
// JavaScript is the catalogue as the pickers list it and the constants.
//
// This file is the module's public face: code outside packages/core/src/switch/
// imports from here and nowhere else (tools/modules.mjs). What is not listed is
// the module's own business. See docs/modules/switch.md.

import { reuseFrom as reuseArgs, viaRust } from '../native/index.js'
import { switchTypeByLabel } from './constants.js'

export {
  ALL_SWITCH_TYPES, CONNECTION_SPEEDS, CROSSING_TYPES, DEFAULT_SWITCH_TYPE_IDX, END_CHAIN_OPEN, FIRST_SLEEPER, MID_OPEN,
  RAIL_OFFSET, SLEEPER_OVERHANG, SLEEPER_SPACING, STATION_TOL, STRAIGHT_CURVATURE, SWITCH_CONNECTION_STAGES, SWITCH_PICK_TYPES, SWITCH_TYPES, SWITCH_TYPES_SPECIAL, WEICHEN_KATALOG,
  switchTypeByLabel, WEICHEN_REGELWERK, WEICHEN_REGELWERK_ID, Z_TOL,
} from './constants.js'

/**
 * What the module answers, with what the arguments passed through untouched
 * given back as the very objects (native reuseFrom), and every form of the
 * catalogue in it — an object with its \`art\` and \`label\` — the catalogue's
 * own object, as the pickers and the tables hold them.
 */
function reuseFrom(out, args) {
  const kept = reuseArgs(out, args)
  const form = (v, depth) => {
    if (!v || typeof v !== 'object' || depth < 0) return v
    if (!Array.isArray(v) && typeof v.art === 'string' && typeof v.label === 'string') return switchTypeByLabel(v.label) ?? v
    if (Array.isArray(v)) {
      const items = v.map(x => form(x, depth - 1))
      return items.every((x, i) => x === v[i]) ? v : items
    }
    let out = null
    for (const [k, x] of Object.entries(v)) {
      const f = form(x, depth - 1)
      if (f !== x) (out ??= { ...v })[k] = f
    }
    return out ?? v
  }
  return form(kept, 3)
}

/**
 * A function that takes the forms a switch's label names — `formOf(label)`, a
 * test's own, at argument `at` or as its `formOf` option — with the forms of
 * every switch its arguments hold looked up there and handed to the module as
 * a table (`{ forms }`): a function does not cross. Without, the catalogue's.
 */
function withForms(fn, at) {
  return (...args) => {
    const given = args[at]
    const formOf = typeof given === 'function' ? given : given?.formOf
    if (typeof formOf !== 'function') return fn(...args)
    const labels = new Set()
    const look = (v, depth) => {
      if (!v || typeof v !== 'object') return
      if (typeof v.label === 'string') labels.add(v.label)
      if (depth > 0) for (const x of Array.isArray(v) ? v : Object.values(v)) look(x, depth - 1)
    }
    args.forEach(a => look(a, 2))
    const forms = Object.fromEntries([...labels].map(l => [l, formOf(l) ?? null]))
    const { formOf: _f, ...rest } = typeof given === 'function' ? {} : given
    const copy = [...args]
    copy[at] = { ...rest, forms }
    return fn(...copy)
  }
}

export const asRadius = viaRust('switch.asRadius', reuseFrom)
export const bauform = viaRust('switch.bauform', reuseFrom)
export const branchPlaneHeight = withForms(viaRust('switch.branchPlaneHeight', reuseFrom), 4)
export const clickStation = viaRust('switch.clickStation', reuseFrom)
export const computeCrossingGeometryFromPortA = viaRust('switch.computeCrossingGeometryFromPortA', reuseFrom)
export const computeCrossingGeometryUtm = viaRust('switch.computeCrossingGeometryUtm', reuseFrom)
export const computeSwitchConnections = viaRust('switch.computeSwitchConnections', reuseFrom)
export const computeSwitchGeometryUtm = viaRust('switch.computeSwitchGeometryUtm', reuseFrom)
export const connectionAnchor = viaRust('switch.connectionAnchor', reuseFrom)
export const connectionPartners = viaRust('switch.connectionPartners', reuseFrom)
export const coupleSwitchCants = viaRust('switch.coupleSwitchCants', reuseFrom)
export const couplingOf = withForms(viaRust('switch.couplingOf', reuseFrom), 3)
export const crossingAngle = viaRust('switch.crossingAngle', reuseFrom)
export const crossingBodyUtm = viaRust('switch.crossingBodyUtm', reuseFrom)
export const crossingElements = viaRust('switch.crossingElements', reuseFrom)
export const crossingEndDistance = viaRust('switch.crossingEndDistance', reuseFrom)
export const crossingLegFitsTrack = viaRust('switch.crossingLegFitsTrack', reuseFrom)
export const crossingLegRadius = viaRust('switch.crossingLegRadius', reuseFrom)
export const crossingLegSignedRadius = viaRust('switch.crossingLegSignedRadius', reuseFrom)
export const crossingRoutesFromTracks = viaRust('switch.crossingRoutesFromTracks', reuseFrom)
export const endChainDefault = viaRust('switch.endChainDefault', reuseFrom)
export const endChainOpen = viaRust('switch.endChainOpen', reuseFrom)
export const formDetail = viaRust('switch.formDetail', reuseFrom)
export const hasCompoundBranch = viaRust('switch.hasCompoundBranch', reuseFrom)
export const heightContext = withForms(viaRust('switch.heightContext', reuseFrom), 3)
export const keptRoutes = viaRust('switch.keptRoutes', reuseFrom)
export const lcsLine = viaRust('switch.lcsLine', reuseFrom)
export const lcsLineUtm = viaRust('switch.lcsLineUtm', reuseFrom)
export const ldsFromToe = withForms(viaRust('switch.ldsFromToe', reuseFrom), 2)
export const ldsLift = withForms(viaRust('switch.ldsLift', reuseFrom), 4)
export const mainRouteLength = viaRust('switch.mainRouteLength', reuseFrom)
export const midUnknowns = viaRust('switch.midUnknowns', reuseFrom)
export const nearestValid = viaRust('switch.nearestValid', reuseFrom)
export const piecesOnRadius = viaRust('switch.piecesOnRadius', reuseFrom)
export const placementCantAt = viaRust('switch.placementCantAt', reuseFrom)
export const planSwitchDeletion = viaRust('switch.planSwitchDeletion', reuseFrom)
export const planeFrom = viaRust('switch.planeFrom', reuseFrom)
export const rebuildSwitchSymbol = viaRust('switch.rebuildSwitchSymbol', reuseFrom)
export const roundMm = viaRust('switch.roundMm', reuseFrom)
export const settleStation = viaRust('switch.settleStation', reuseFrom)
export const sleeperNear = withForms(viaRust('switch.sleeperNear', reuseFrom), 4)
export const sleeperOffsets = withForms(viaRust('switch.sleeperOffsets', reuseFrom), 4)
export const sleeperThrough = withForms(viaRust('switch.sleeperThrough', reuseFrom), 4)
export const switchAbreast = viaRust('switch.switchAbreast', reuseFrom)
export const switchArcLength = viaRust('switch.switchArcLength', reuseFrom)
export const switchBodySpans = withForms(viaRust('switch.switchBodySpans', reuseFrom), 3)
export const switchBranchChain = viaRust('switch.switchBranchChain', reuseFrom)
export const switchBranchLength = viaRust('switch.switchBranchLength', reuseFrom)
export const switchBranchPointAt = viaRust('switch.switchBranchPointAt', reuseFrom)
export const switchBranchSections = viaRust('switch.switchBranchSections', reuseFrom)
export const switchBranchSpeed = viaRust('switch.switchBranchSpeed', reuseFrom)
export const switchChainBauform = viaRust('switch.switchChainBauform', reuseFrom)
export const switchChainBearingAt = viaRust('switch.switchChainBearingAt', reuseFrom)
export const switchChainJoints = viaRust('switch.switchChainJoints', reuseFrom)
export const switchChainPointUtm = viaRust('switch.switchChainPointUtm', reuseFrom)
export const switchChainTo = viaRust('switch.switchChainTo', reuseFrom)
export const switchCoupling = withForms(viaRust('switch.switchCoupling', reuseFrom), 2)
export const switchCouplings = withForms(viaRust('switch.switchCouplings', reuseFrom), 2)
export const switchDetail = withForms(viaRust('switch.switchDetail', reuseFrom), 2)
export const switchDetailScene = viaRust('switch.switchDetailScene', reuseFrom)
export const switchElementRoute = viaRust('switch.switchElementRoute', reuseFrom)
export const switchEndAnchorRefusal = viaRust('switch.switchEndAnchorRefusal', reuseFrom)
export const switchFillRing = viaRust('switch.switchFillRing', reuseFrom)
export const switchFormChain = viaRust('switch.switchFormChain', reuseFrom)
export const switchLabelGeometry = viaRust('switch.switchLabelGeometry', reuseFrom)
export const switchLds = withForms(viaRust('switch.switchLds', reuseFrom), 1)
export const switchMark = viaRust('switch.switchMark', reuseFrom)
export const switchParts = viaRust('switch.switchParts', reuseFrom)
export const switchRouteBearingAt = viaRust('switch.switchRouteBearingAt', reuseFrom)
export const switchRoutePointUtm = viaRust('switch.switchRoutePointUtm', reuseFrom)
export const switchRoutePointsUtm = viaRust('switch.switchRoutePointsUtm', reuseFrom)
export const switchRoutesFromTracks = viaRust('switch.switchRoutesFromTracks', reuseFrom)
export const switchStraightLength = viaRust('switch.switchStraightLength', reuseFrom)
export const switchVariants = viaRust('switch.switchVariants', reuseFrom)
export const trackHeightAt = withForms(viaRust('switch.trackHeightAt', reuseFrom), 4)
export const turnoutRunsOn = viaRust('switch.turnoutRunsOn', reuseFrom)
export const turnoutSleepers = withForms(viaRust('switch.turnoutSleepers', reuseFrom), 2)
export const utmEndRoute = viaRust('switch.utmEndRoute', reuseFrom)
export const weichenGruppen = viaRust('switch.weichenGruppen', reuseFrom)

/**
 * Where a turnout of straight length `length` lies on `track` with its toe at
 * `station` (`reversed` the other way along it): the placement, with
 * `cantAt(x, options)` the cant the track has under it at `x` — asked of the
 * module, as the placement's own function was.
 */
export function placeSwitchOnTrack(track, station, reversed, length) {
  const placement = placeSwitchOnTrackRust(track, station, reversed, length)
  if (!placement || placement.error) return placement
  return { ...placement, cantAt: (x, options) => placementCantAt(track, station, reversed, length, x, options) }
}
const placeSwitchOnTrackRust = viaRust('switch.placeSwitchOnTrack', reuseFrom)

// The module's other operations, for what asks them directly.
export const branchRadius = viaRust('switch.branchRadius', reuseFrom)
export const buildConnectionElements = viaRust('switch.buildConnectionElements', reuseFrom)
export const chainGeometry = viaRust('switch.chainGeometry', reuseFrom)
export const checkMidRun = viaRust('switch.checkMidRun', reuseFrom)
export const clothoidRoutePoints = viaRust('switch.clothoidRoutePoints', reuseFrom)
export const connectionBranchEnds = viaRust('switch.connectionBranchEnds', reuseFrom)
export const crossingWedgeRing = viaRust('switch.crossingWedgeRing', reuseFrom)
export const endCurvature = viaRust('switch.endCurvature', reuseFrom)
export const fitConnectingElement = viaRust('switch.fitConnectingElement', reuseFrom)
export const isDefaultMids = viaRust('switch.isDefaultMids', reuseFrom)
export const joinRouteIntoTrack = viaRust('switch.joinRouteIntoTrack', reuseFrom)
export const midSignSets = viaRust('switch.midSignSets', reuseFrom)
export const orientStemToward = viaRust('switch.orientStemToward', reuseFrom)
export const refitCount = viaRust('switch.refitCount', reuseFrom)
export const refitTrackEnd = viaRust('switch.refitTrackEnd', reuseFrom)
export const routeCoords = viaRust('switch.routeCoords', reuseFrom)
export const sleeperGrid = viaRust('switch.sleeperGrid', reuseFrom)
export const sleepersWeToLds = viaRust('switch.sleepersWeToLds', reuseFrom)
export const solveSwitchConnection = viaRust('switch.solveSwitchConnection', reuseFrom)
export const switchBranchRoute = viaRust('switch.switchBranchRoute', reuseFrom)
export const switchChainSegmentsUtm = viaRust('switch.switchChainSegmentsUtm', reuseFrom)
export const switchChainSlice = viaRust('switch.switchChainSlice', reuseFrom)
export const switchMarkDistance = viaRust('switch.switchMarkDistance', reuseFrom)
export const switchRouteRadiusAt = viaRust('switch.switchRouteRadiusAt', reuseFrom)
export const switchRouteSegments = viaRust('switch.switchRouteSegments', reuseFrom)
export const switchRouteSlice = viaRust('switch.switchRouteSlice', reuseFrom)
export const switchStemRoute = viaRust('switch.switchStemRoute', reuseFrom)
export const switchTangents = viaRust('switch.switchTangents', reuseFrom)
export const switchVariantFor = viaRust('switch.switchVariantFor', reuseFrom)
export const tableLds = viaRust('switch.tableLds', reuseFrom)
export const toChain = viaRust('switch.toChain', reuseFrom)
export const toRoute = viaRust('switch.toRoute', reuseFrom)
export const uniqueStations = viaRust('switch.uniqueStations', reuseFrom)

/** A radius the other way round; null stays null. */
export const negR = (r) => (r == null ? null : -r)

/** Does the curvature change along a route — is it a clothoid? */
export const switchRouteVaries = (route) => route.r1 !== route.r2

/** The forms of a connection's two turnouts: one form for both, or a pair. */
export const formPair = (form) => (Array.isArray(form) ? form : [form, form])
