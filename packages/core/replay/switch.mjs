// The switch crate's vectors (crates/switch): every form of the catalogue
// read the way the geometry reads it, routes and chains laid from a point.
import * as C from '../src/switch/catalogue.js'
import * as R from '../src/switch/route.js'
import * as X from '../src/switch/crossing.js'
import * as Y from '../src/switch/symbol.js'
import * as S from '../src/switch/sleepers.js'
import * as G from '../src/switch/switchGradient.js'
import { coupleSwitchCants } from '../src/switch/switchCants.js'
import * as D from '../src/switch/switchDelete.js'
import { mainTrack, branchTrack, turnout as testTurnout } from '../test/turnoutFixture.js'
import * as P from '../src/switch/switchPlacement.js'
import { switchDetail, formDetail, RAIL_OFFSET, SLEEPER_OVERHANG } from '../src/switch/detail.js'
import { switchDetailScene, connectionPartners } from '../src/switch/detailScene.js'
import { connectionPick } from '../src/commands/index.js'
import { WEICHEN_REGELWERK, weichenGruppen } from '../src/switch/weichenRegelwerk.js'
import { utmToWgs84, straightElement, arcElement, transitionElement, endPointStraightUtm } from '../src/geometry/index.js'
import { frozen, rec } from './record.mjs'
import { reuseFrom, viaRust } from '../src/native/index.js'
import * as Face from '../src/switch/index.js'
import { canonicalJson } from '../src/document/index.js'

// The solvers of the dialogs — connecting to an open end, the switch
// connection, its middle chain, the station slider — went to Rust (decision
// 0020, step 10): what is recorded of them is taken from the vectors as they
// were last recorded from the JavaScript, and what the recordings below build
// from them is built by the module.
const SOLVER_CONSTANTS = { DEFAULT_MIDS: [{ type: 'arc', length: null, radius: null }] }
const solvers = new Proxy({}, { get: (_, name) => SOLVER_CONSTANTS[name] ?? Face[name] ?? viaRust(`switch.${String(name)}`, reuseFrom) })
const K = solvers, M = solvers, Sl = solvers, Cn = solvers
const SOLVER_OPS = new Set(['connectionAnchor', 'fitConnectingElement', 'connectionRoots', 'END_CHAIN_OPEN', 'endChainDefault', 'endChainOpen',
  'solveEndChain', 'refitTrackEnd', 'refitCount', 'joinRouteIntoTrack', 'DEFAULT_MIDS', 'isDefaultMids', 'midUnknowns', 'MID_OPEN', 'solveMidChain',
  'midSignSets', 'chainNewton', 'endCurvature', 'validIntervals', 'nearestValid', 'settleStation', 'uniqueStations', 'formPair', 'orientStemToward',
  'checkMidRun', 'connectionBranchEnds', 'CONNECTION_SPEEDS', 'computeSwitchConnections', 'solveSwitchConnection', 'buildConnectionElements'].map(n => `switch.${n}`))

const EPSG = 25832
const O = { easting: 600123.25, northing: 5600456.5, zone: EPSG }

export function recordCatalogueAndRoutes(calls) {
  const r = (op, fn, ...args) => calls.push(rec(`switch.${op}`, fn, ...args))
  const k = (op, value) => calls.push({ op: `switch.${op}`, input: [], output: JSON.parse(JSON.stringify(value)) })
  k('SWITCH_TYPES', C.SWITCH_TYPES); k('SWITCH_TYPES_SPECIAL', C.SWITCH_TYPES_SPECIAL); k('SWITCH_PICK_TYPES', C.SWITCH_PICK_TYPES)
  k('ALL_SWITCH_TYPES', C.ALL_SWITCH_TYPES); k('CROSSING_TYPES', C.CROSSING_TYPES); k('DEFAULT_SWITCH_TYPE_IDX', C.DEFAULT_SWITCH_TYPE_IDX)
  k('SWITCH_CONNECTION_STAGES', C.SWITCH_CONNECTION_STAGES); k('WEICHEN_REGELWERK', WEICHEN_REGELWERK)
  r('weichenGruppen', weichenGruppen)
  for (const form of [...C.ALL_SWITCH_TYPES, ...C.CROSSING_TYPES]) {
    r('switchTypeByLabel', C.switchTypeByLabel, form.label)
    r('switchMark', C.switchMark, form)
    r('switchVariants', C.switchVariants, form)
    for (const rail of ['60E2', '54E4', '49E5', null]) r('switchVariantFor', C.switchVariantFor, form, rail)
    if (form.art !== 'weiche') continue
    r('switchBranchSections', C.switchBranchSections, form)
    r('hasCompoundBranch', C.hasCompoundBranch, form)
    r('switchStraightLength', C.switchStraightLength, form)
    r('switchBranchLength', C.switchBranchLength, form)
    r('switchMarkDistance', C.switchMarkDistance, form)
    if (!form.symmetric) r('switchTangents', C.switchTangents, form)
    for (const s of [0, 7.3, 1000]) r('switchBranchPointAt', C.switchBranchPointAt, form, s)
    for (const side of ['left', 'right']) r('switchFormChain', C.switchFormChain, form, side)
  }
  r('switchTypeByLabel', C.switchTypeByLabel, 'nope')
  r('switchMark', C.switchMark, null)
  r('switchBranchSections', C.switchBranchSections, null)
  for (const sw of [{ speed: 80, label: '300 – 1:9' }, { speed: 0, label: '500 – 1:12' }, { label: 'x' }, null]) r('switchBranchSpeed', C.switchBranchSpeed, sw)
  r('switchArcLength', C.switchArcLength, 300, 9)
  for (const x of [300, 0, Infinity, NaN, null, -190]) r('asRadius', C.asRadius, x)
  for (const [f, s] of [[300, null], [300, 1200], [-300, 1200], [300, -300], [-500, 500.000001]]) r('branchRadius', C.branchRadius, f, s)
  for (const [s, b] of [[null, 300], [1200, 240], [-1200, 400], [1200, null]]) r('bauform', C.bauform, s, b)

  const routes = [{ length: 33, radius: 300 }, { length: 20, radius: null }, { length: 40, r1: null, r2: -900 }, { length: 25, r1: 600, r2: 1200 },
    { length: 10, r1: 500 }, { length: 12, r1: 0, r2: Infinity }]
  for (const rt of routes) {
    r('toRoute', R.toRoute, rt)
    const route = R.toRoute(rt)
    for (const s of [0, 4.4, route.length]) {
      r('switchRouteRadiusAt', R.switchRouteRadiusAt, route, s)
      r('switchRoutePointUtm', R.switchRoutePointUtm, O, 77.5, route, s)
      r('switchRouteBearingAt', R.switchRouteBearingAt, 355.5, route, s)
    }
    r('switchRoutePointUtm', R.switchRoutePointUtm, O, 77.5, route)
    r('switchRouteBearingAt', R.switchRouteBearingAt, 355.5, route)
    r('switchRouteSlice', R.switchRouteSlice, route, 2.5, 9)
    for (const f of [300, -300, null]) r('switchBranchRoute', R.switchBranchRoute, f, route, 27.3)
    r('clothoidRoutePoints', R.clothoidRoutePoints, O, 12, route)
    const end = R.switchRoutePointUtm(O, 12, route)
    r('routeCoords', R.routeCoords, O, utmToWgs84(O.easting, O.northing, EPSG), 12, route, utmToWgs84(end.easting, end.northing, EPSG))
  }
  r('toChain', R.toChain, routes)
  r('toChain', R.toChain, routes[0])
  for (const sw of [{ mainRadius: 1200 }, { mainRadius: 1200, mainRadiusEnd: null }, { mainRadius: 900, mainRadiusEnd: 1800 }, {}]) r('switchStemRoute', R.switchStemRoute, sw, 30)
  for (const el of [{ elementType: 1, length: 30, radius: -400 }, { elementType: 0, length: 12 }, { elementType: 2, length: 50, r1: 300 }, { elementType: 2, length: 50, r1: null, r2: 700 }]) r('switchElementRoute', R.switchElementRoute, el)
  const chain = R.toChain([{ length: 10, radius: null }, { length: 14.5, r1: null, r2: 900 }, { length: 9, radius: 900 }])
  for (const L of [5, 10, 24.5, 33.4999995, 33.5, 40]) r('switchChainTo', R.switchChainTo, chain, L)
  r('switchChainSegmentsUtm', R.switchChainSegmentsUtm, O, 33, chain)
  for (const s of [null, 0, 12, 33.5, 50]) { r('switchChainPointUtm', R.switchChainPointUtm, O, 33, chain, s); r('switchChainBearingAt', R.switchChainBearingAt, 33, chain, s) }
  r('switchChainSlice', R.switchChainSlice, chain, 3, 27)
  r('switchChainJoints', R.switchChainJoints, chain)
  for (const u of [0, 10, 24.5, 30, 33.5]) r('switchAbreast', R.switchAbreast, u, [10, 24.5], 33.5, 33.41)
  for (const form of [C.switchTypeByLabel('300 – 1:9'), C.switchTypeByLabel('500 – 1:12'), ...C.ALL_SWITCH_TYPES.filter(f => f.branch).slice(0, 4)]) {
    for (const side of ['left', 'right']) {
      const fc = C.switchFormChain(form, side)
      r('switchBranchChain', R.switchBranchChain, fc, R.switchChainTo(chain, C.switchStraightLength(form)))
      r('switchBranchChain', R.switchBranchChain, fc, { length: C.switchStraightLength(form), radius: 1500 })
    }
  }
  const branch = R.switchBranchChain(C.switchFormChain(C.switchTypeByLabel('300 – 1:9'), 'right'), chain)
  r('switchChainBauform', R.switchChainBauform, chain, branch)
  r('switchChainBauform', R.switchChainBauform, chain, branch, true)
  r('switchChainBauform', R.switchChainBauform, [{ length: 30, r1: null, r2: null }], [{ length: 30, r1: 300, r2: 300 }])
  r('switchChainBauform', R.switchChainBauform, [{ length: 30, r1: -1200, r2: -1200 }], [{ length: 30, r1: 240, r2: 240 }])
  for (const [L, a, b] of [[30, null], [30, 300], [31, null, 900], [1, 5]]) r('switchRouteSegments', (...x) => R.switchRouteSegments(...x), L, a, ...(b === undefined ? [] : [b]))
  r('utmEndRoute', R.utmEndRoute, O, 45, 20, -250)
  r('utmEndRoute', R.utmEndRoute, O, 45, 20, null)
  const end = R.switchChainPointUtm(O, 33, chain)
  r('chainGeometry', R.chainGeometry, O, utmToWgs84(O.easting, O.northing, EPSG), 33, chain, utmToWgs84(end.easting, end.northing, EPSG))
}

/** Elements built from the pieces of a chain placed in the plane, marked as `route` of switch `id`. */
function elementsOf(segments, mark) {
  return segments.map(seg => {
    const extra = { ...mark, speed: 60 }
    if (seg.r1 !== seg.r2) return transitionElement(seg.startUtm, seg.bearing, seg.length, seg.r1, seg.r2, extra).element
    return seg.r1 ? arcElement(seg.startUtm, seg.endUtm, seg.r1, extra) : straightElement(seg.startUtm, seg.endUtm, extra)
  })
}

/**
 * A turnout laid facing from `start` with the tracks it joins: `a` running
 * into the toe, `m` the through route on, `b` the branch — as the dialogs
 * commit one, each track running on past the switch end.
 */
export function layTurnout(id, label, start, bearing, side, mainR = null) {
  const form = C.switchTypeByLabel(label)
  const g = Y.computeSwitchGeometryUtm(start, bearing, form, side, false, null, mainR)
  const back = endPointStraightUtm(start, (bearing + 180) % 360, 40)
  const a = { id: `${id}a`, epsg: start.zone, name: `${id} a`, elements: [straightElement(back, start, { speed: 60 })] }
  const mEls = elementsOf(g.stemSegments, { switchBranch: true, switchRoute: 'main', switchId: id })
  const mEnd = g.straightUtm
  mEls.push(straightElement(mEnd, endPointStraightUtm(mEnd, g.mainEndBearing, 30), { speed: 60 }))
  const bEls = elementsOf(g.branchSegments, { switchBranch: true, switchRoute: 'branch', switchId: id })
  bEls.push(straightElement(g.curvedUtm, endPointStraightUtm(g.curvedUtm, g.branchEndBearing, 25), { speed: 40 }))
  const m = { id: `${id}m`, epsg: start.zone, name: `${id} m`, elements: mEls }
  const b = { id: `${id}b`, epsg: start.zone, name: `${id} b`, elements: bEls }
  const sw = { switchId: id, kind: 'turnout', name: `W ${id}`, label, speed: form.speed,
    ports: { A: { trackId: a.id, endpoint: 'END' }, B1: { trackId: b.id, endpoint: 'BEGIN' }, B2: { trackId: m.id, endpoint: 'BEGIN' } } }
  return { sw, tracks: [a, m, b], g }
}

/** A crossing kind laid at `centre`, its four legs tracks of their own. */
export function layCrossing(id, label, centre, bearing, angle) {
  const form = C.switchTypeByLabel(label)
  const g = X.computeCrossingGeometryUtm(centre, bearing, form, angle)
  const els = X.crossingElements(g, { switchId: id })
  const t = (k, el) => ({ id: `${id}${k}`, epsg: centre.zone, elements: [el] })
  const tracks = [t('A', els.A), t('B', els.B), t('C', els.C), t('D', els.D)]
  const ports = { A: { trackId: `${id}A`, endpoint: 'END' }, B: { trackId: `${id}B`, endpoint: 'END' }, C: { trackId: `${id}C`, endpoint: 'BEGIN' }, D: { trackId: `${id}D`, endpoint: 'BEGIN' } }
  return { sw: { switchId: id, kind: form.kind, name: `K ${id}`, label, ports }, tracks, g, els }
}

function recordCrossingAndSymbol(calls) {
  const r = (op, fn, ...args) => calls.push(rec(`switch.${op}`, fn, ...args))
  const byId = (tracks) => Object.fromEntries(tracks.map(t => [t.id, t]))
  for (const form of C.CROSSING_TYPES) {
    r('crossingAngle', X.crossingAngle, form); r('crossingLegRadius', X.crossingLegRadius, form)
    r('crossingLegSignedRadius', X.crossingLegSignedRadius, form, 6.3); r('crossingLegSignedRadius', X.crossingLegSignedRadius, form, -6.3)
    r('crossingEndDistance', X.crossingEndDistance, form)
    const deg = Math.atan(1 / form.ratio) * 180 / Math.PI
    for (const angle of [deg, -deg]) {
      r('computeCrossingGeometryUtm', X.computeCrossingGeometryUtm, O, 63.5, form, angle)
      const g = X.computeCrossingGeometryUtm(O, 63.5, form, angle)
      r('crossingElements', X.crossingElements, g, { switchId: 'k1' })
    }
    r('computeCrossingGeometryFromPortA', X.computeCrossingGeometryFromPortA, O, utmToWgs84(O.easting, O.northing, EPSG), 200, form, deg)
    const { sw, tracks } = layCrossing('k', form.label, O, 63.5, deg)
    r('crossingRoutesFromTracks', Y.crossingRoutesFromTracks, sw, byId(tracks))
    r('crossingBodyUtm', (s, b) => Y.crossingBodyUtm(Y.crossingRoutesFromTracks(s, b)), sw, byId(tracks))
    r('rebuildSwitchSymbol', Y.rebuildSwitchSymbol, sw, byId(tracks))
    // Without the legs behind the point: mirrored.
    r('crossingRoutesFromTracks', Y.crossingRoutesFromTracks, sw, byId(tracks.filter(t => !t.id.endsWith('A') && !t.id.endsWith('B'))))
  }
  r('computeCrossingGeometryUtm', X.computeCrossingGeometryUtm, O, 10, C.switchTypeByLabel(C.CROSSING_TYPES[0].label), 6, [8.5, 50.5], [8.6, 50.6], EPSG)
  r('crossingWedgeRing', X.crossingWedgeRing, [[0, 0], [1, 1], [2, 2]], [[0, 0], [1, -1]])
  const straightPieces = [{ length: 3, r1: null, r2: null }]
  r('piecesOnRadius', X.piecesOnRadius, straightPieces, null)
  r('piecesOnRadius', X.piecesOnRadius, [{ length: 3, r1: 500, r2: 500.00001 }], 500)
  r('piecesOnRadius', X.piecesOnRadius, [{ length: 3, r1: 500, r2: null }], 500)
  r('crossingLegFitsTrack', X.crossingLegFitsTrack, straightPieces, 300)

  const stems = [null, 1200, -1200, -300, { r1: null, r2: 900 }, { r1: 2000, r2: 700 },
    [{ length: 12, radius: null }, { length: 18, r1: null, r2: -1500 }, { length: 40, radius: -1500 }]]
  for (const form of C.ALL_SWITCH_TYPES) {
    for (const side of ['left', 'right']) {
      for (const trailing of [false, true]) {
        for (const mainR of (form.label === '300 – 1:9' || form.label === '500 – 1:12' ? stems : [null, 900])) {
          r('computeSwitchGeometryUtm', Y.computeSwitchGeometryUtm, O, 77.25, form, side, trailing, null, mainR)
        }
      }
    }
  }
  r('computeSwitchGeometryUtm', Y.computeSwitchGeometryUtm, O, 77.25, C.switchTypeByLabel('300 – 1:9'), 'right', false, [8.9, 50.1], null)
  let k = 0
  for (const [label, side, mainR] of [['300 – 1:9', 'left', null], ['500 – 1:12', 'right', 1100], ['190 – 1:9', 'right', null], ['300 – 1:9', 'right', -300],
    ['500 – 1:12', 'left', { r1: null, r2: -800 }], ['300 – 1:9', 'left', stems[6]], ...C.ALL_SWITCH_TYPES.filter(f => f.branch).slice(0, 3).map(f => [f.label, 'left', null])]) {
    const { sw, tracks } = layTurnout(`s${++k}`, label, O, 33.3, side, mainR)
    r('switchRoutesFromTracks', Y.switchRoutesFromTracks, sw, byId(tracks))
    r('rebuildSwitchSymbol', Y.rebuildSwitchSymbol, sw, byId(tracks))
    // A record of another label, one without marks on its stem, one without its branch.
    r('rebuildSwitchSymbol', Y.rebuildSwitchSymbol, { ...sw, label: 'gone' }, byId(tracks))
    const bare = tracks.map(t => (t.id.endsWith('m') ? { ...t, elements: t.elements.map(({ switchBranch, switchRoute, switchId, ...el }) => el) } : t))
    r('rebuildSwitchSymbol', Y.rebuildSwitchSymbol, { ...sw, mainRadius: 1300 }, byId(bare))
    r('rebuildSwitchSymbol', Y.rebuildSwitchSymbol, sw, byId(tracks.slice(0, 2)))
  }
  const chain = [{ length: 10, radius: null }, { length: 14.5, r1: null, r2: 900 }, { length: 9, radius: 900 }]
  r('switchRoutePointsUtm', Y.switchRoutePointsUtm, O, 12, chain)
  r('switchRoutePointsUtm', Y.switchRoutePointsUtm, O, 12, { length: 30, radius: 300 })
  r('switchLabelGeometry', Y.switchLabelGeometry, O, 12, chain, [{ length: 33.5, radius: 260 }], EPSG)
  r('switchFillRing', Y.switchFillRing, [[0, 0], [1, 0]], [[0, 0], [1, 1]])
  for (const d of [3.9, 0, -1, null]) r('lcsLineUtm', Y.lcsLineUtm, [0, 0], [30, 2], [30, 0], d)
  r('lcsLineUtm', Y.lcsLineUtm, [0, 0], [30, 0], [30, 0], 3)
  r('lcsLine', Y.lcsLine, [600000, 5600000], [600030, 5600002], [600030, 5600000], 3.9, EPSG)
}

/** A form lookup from a table, as the options of a call carry it. */
const formsOf = (forms) => (forms ? (label) => forms[label] ?? null : undefined)
const withForms = (o = {}) => { const { forms, ...rest } = o; return { ...rest, ...(forms ? { formOf: formsOf(forms) } : {}) } }
const stripFrame = (f) => f && ({
  main: { trackId: f.main.track.id, length: f.main.length, sense: f.main.sense },
  branch: { trackId: f.branch.track.id, length: f.branch.length, sense: f.branch.sense },
  variant: f.variant, we: f.we, lds: f.lds, ldsMain: f.ldsMain, ldsBranch: f.ldsBranch, ldsBranchDistance: f.ldsBranchDistance,
  weBisector: f.weBisector, samples: f.samples, sleepers: f.sleepers, grid: f.grid,
  ...(f.ldsSleeper ? { sw: f.sw, ldsSleeper: f.ldsSleeper, offset: f.offset, cant: f.cant, reach: f.reach, end: f.end } : {}),
})

function recordBodies(calls) {
  const r = (op, fn, ...args) => calls.push(rec(`switch.${op}`, fn, ...args))
  const TEST = { TEST: { lds: 10 } }
  const heights = (z0) => [{ station: 0, z: z0 }, { station: 30, z: z0 + 0.12, rv: 4000 }, { station: 60, z: z0 + 0.05 }]
  const testTracks = (cant) => [{ id: 'a', epsg: 5684, elements: [] }, mainTrack(cant, heights(100)), branchTrack(heights(100.02))]
  const mainSpec = [[testTracks(0), testTurnout()], [testTracks(60), testTurnout()], [testTracks(-45), testTurnout()],
    [testTracks(0), testTurnout({ swapped: true, ports: { B1: { trackId: 'm' }, B2: { trackId: 'b' } } })]]
  for (const [tracks, sw] of mainSpec) {
    r('switchLds', (s, o) => S.switchLds(s, formsOf(o.forms)), sw, { forms: TEST })
    r('mainRouteLength', S.mainRouteLength, tracks[1], sw)
    r('turnoutSleepers', (t, s, o) => stripFrame(S.turnoutSleepers(t, s, withForms(o))), tracks, sw, { forms: TEST })
    for (const [side, st] of [['main', 12.3], ['branch', 7], ['branch', 39.5], ['main', 0.001], ['main', 55]]) {
      const frame = S.turnoutSleepers(tracks, sw, { formOf: formsOf(TEST) })
      if (!frame) continue
      r('sleeperNear', (t, s, sd, x, o) => S.sleeperNear(S.turnoutSleepers(t, s, withForms(o)), sd, x), tracks, sw, side, st, { forms: TEST })
      r('sleeperThrough', (t, s, sd, x, o) => S.sleeperThrough(S.turnoutSleepers(t, s, withForms(o)), sd, x), tracks, sw, side, st, { forms: TEST })
      if (frame && S.sleeperThrough(frame, side, st)) {
        r('sleeperOffsets', (t, s, sd, x, o) => { const f = S.turnoutSleepers(t, s, withForms(o)); return S.sleeperOffsets(f, S.sleeperThrough(f, sd, x)) }, tracks, sw, side, st, { forms: TEST })
      }
    }
    r('ldsFromToe', (t, s, o) => G.ldsFromToe(t, s, withForms(o)), tracks, sw, { forms: TEST })
    r('switchCouplings', (t, ss, o) => G.switchCouplings(t, ss, withForms(o)).map(stripFrame), tracks, [sw], { forms: TEST })
    r('couplingOf', (t, ss, s, o) => stripFrame(G.couplingOf(t, ss, s, withForms(o))), tracks, [sw], sw, { forms: TEST })
    r('ldsLift', (t, ss, s, h, o) => { const c = G.couplingOf(t, ss, s, withForms(o)); return c ? G.ldsLift(c, h) : null }, tracks, [sw], sw, heights(100), { forms: TEST })
    for (const st of [0, 5, 17.25, 29.99, 41, 59]) {
      r('branchPlaneHeight', (t, ss, s, x, o) => { const c = G.couplingOf(t, ss, s, withForms(o)); return c ? G.branchPlaneHeight(c, x) : null }, tracks, [sw], sw, st, { forms: TEST })
      r('trackHeightAt', (t, ss, tr, x, o) => G.trackHeightAt(t, ss, tr, x, withForms(o)), tracks, [sw], tracks[2], st, { forms: TEST })
    }
    r('heightContext', (t, ss, tr, o) => G.heightContext(t, ss, tr, withForms(o)), tracks, [sw], tracks[2], { forms: TEST })
    r('heightContext', (t, ss, tr, o) => G.heightContext(t, ss, tr, withForms(o)), tracks, [sw], tracks[1], { forms: TEST })
    r('switchBodySpans', (t, ss, tr, o) => G.switchBodySpans(t, ss, tr, withForms(o)), tracks, [sw], tracks[1], { forms: TEST })
    r('switchBodySpans', (t, ss, tr, o) => G.switchBodySpans(t, ss, tr, withForms(o)), tracks, [sw], tracks[2], {})
  }
  r('planeFrom', G.planeFrom, mainTrack(60), 20, 101, branchTrack(), 20)
  r('planeFrom', G.planeFrom, mainTrack(60), 20, null, branchTrack(), 20)
  for (const x of [1.23456, -0.0004, 2.0005]) r('roundMm', G.roundMm, x)

  // Turnouts of the catalogue laid with heights, a crossover of two of them.
  let k = 0
  for (const [label, side, mainR] of [['300 – 1:9', 'left', null], ['500 – 1:12', 'right', 1100], ['190 – 1:9', 'right', null], ['300 – 1:9', 'right', -900],
    ['760 – 1:15', 'left', null], ...C.ALL_SWITCH_TYPES.filter(f => f.branch).slice(0, 2).map(f => [f.label, 'left', null])]) {
    const { sw, tracks } = layTurnout(`t${++k}`, label, O, 33.3, side, mainR)
    const withH = tracks.map((t, i) => ({ ...t, heights: [{ station: 0, z: 100 + i }, { station: 20, z: 100.2 + i, rv: 2000 }, { station: 60, z: 100.1 + i }],
      elements: t.elements.map(el => ({ ...el, cant: i === 1 ? -40 : 0 })) }))
    r('turnoutSleepers', (t, s, o) => stripFrame(S.turnoutSleepers(t, s, withForms(o))), withH, sw, {})
    r('turnoutSleepers', (t, s, o) => stripFrame(S.turnoutSleepers(t, s, withForms(o))), withH, sw, { rail: '49E5' })
    r('switchCouplings', (t, ss, o) => G.switchCouplings(t, ss, withForms(o)).map(stripFrame), withH, [sw], {})
    for (const st of [3, 15, 31]) r('trackHeightAt', (t, ss, tr, x, o) => G.trackHeightAt(t, ss, tr, x, withForms(o)), withH, [sw], withH[2], st, {})
    r('switchBodySpans', (t, ss, tr, o) => G.switchBodySpans(t, ss, tr, withForms(o)), withH, [sw], withH[1], {})
    // Cants: the main route's cant changed by a write; the branch follows.
    const after = withH.map((t, i) => (i === 1 ? { ...t, elements: t.elements.map((el, j) => (j === 0 ? { ...el, cant: -70 } : el)) } : t))
    r('coupleSwitchCants', coupleSwitchCants, withH, after, [sw])
    const branchWrite = withH.map((t, i) => (i === 2 ? { ...t, elements: t.elements.map((el, j) => (j === 0 ? { ...el, cant: 25 } : el)) } : t))
    r('coupleSwitchCants', coupleSwitchCants, withH, branchWrite, [sw])
    const approachWrite = withH.map((t, i) => (i === 0 ? { ...t, elements: t.elements.map(el => ({ ...el, cant: -15 })) } : t))
    r('coupleSwitchCants', coupleSwitchCants, withH, approachWrite, [sw])
    // Deletion: all three ports occupied, the branch track alone, no track behind the toe.
    r('switchParts', (s, t) => JSON.parse(JSON.stringify(D.switchParts(s, t))), sw, withH)
    r('keptRoutes', (s, t) => D.keptRoutes(s, D.switchParts(s, t).byPort), sw, withH)
    r('planSwitchDeletion', D.planSwitchDeletion, sw, withH)
    r('planSwitchDeletion', D.planSwitchDeletion, sw, withH.slice(1))
    r('planSwitchDeletion', D.planSwitchDeletion, { ...sw, swapped: true }, withH)
    r('planSwitchDeletion', D.planSwitchDeletion, sw, [withH[0], withH[2]])
  }
  r('coupleSwitchCants', coupleSwitchCants, [], [], [])
  r('planSwitchDeletion', D.planSwitchDeletion, { switchId: 'l', kind: 'link', ports: {} }, [])
  r('planSwitchDeletion', D.planSwitchDeletion, {}, [])
  for (const form of C.CROSSING_TYPES) {
    const { sw, tracks, els } = layCrossing('x', form.label, O, 63.5, 6)
    const more = tracks.map(t => ({ ...t, elements: t.id.endsWith('A') || t.id.endsWith('B') ? [straightElement(endPointStraightUtm(
      { easting: t.elements[0].startNode[0], northing: t.elements[0].startNode[1], zone: EPSG }, (t.elements[0].bearing + 180) % 360, 20), { easting: t.elements[0].startNode[0], northing: t.elements[0].startNode[1], zone: EPSG }), ...t.elements]
      : [...t.elements, straightElement({ easting: t.elements.at(-1).endNode[0], northing: t.elements.at(-1).endNode[1], zone: EPSG }, endPointStraightUtm({ easting: t.elements.at(-1).endNode[0], northing: t.elements.at(-1).endNode[1], zone: EPSG }, t.elements.at(-1).bearing, 20))] }))
    const slips = [els.slip1 && { id: 'x1', epsg: EPSG, name: 'slip 1', elements: [els.slip1] }, els.slip2 && { id: 'x2', epsg: EPSG, elements: [els.slip2] }].filter(Boolean)
    r('planSwitchDeletion', D.planSwitchDeletion, sw, [...tracks, ...slips])
    r('planSwitchDeletion', D.planSwitchDeletion, sw, [...more, ...slips])
    r('planSwitchDeletion', D.planSwitchDeletion, sw, [...more.slice(0, 2), ...tracks.slice(2), ...slips])
  }

  for (const [sWe, sEnd, count] of [[20, 30, null], [20, 30, 33], [0.9, 5, null], [27.14, 36, 46]]) r('sleeperGrid', S.sleeperGrid, sWe, sEnd, count)
  for (const lds of [0, -0.244, 3.3, 10.25]) r('sleepersWeToLds', S.sleepersWeToLds, lds)
  r('tableLds', S.tableLds, { masse: { lds: 7.5 } }, 6); r('tableLds', S.tableLds, null, 6)

  const iv = [[0, 10], [15, 20.5], [30, 30]]
  for (const s of [-3, 5, 12, 13, 25, 40]) {
    r('nearestValid', Sl.nearestValid, s, iv)
    r('settleStation', Sl.settleStation, s, { intervals: iv, snaps: [10.4, 14.8, 30], tol: 1 })
  }
  r('nearestValid', Sl.nearestValid, 3, [])
  r('uniqueStations', Sl.uniqueStations, [3.00001, 3, 2.99999, NaN, 1.5, 12.34567])
  for (const mids of [M.DEFAULT_MIDS, [], [{ type: 'straight', length: null }, { type: 'arc', length: 20, radius: null }],
    [{ type: 'transition', length: 15 }, { type: 'arc', length: null, radius: 500 }, { type: 'transition', length: null }],
    [{ type: 'arc', length: null, radius: 300, side: -1 }, { type: 'arc', length: null, radius: 500 }]]) {
    r('isDefaultMids', M.isDefaultMids, mids); r('midUnknowns', M.midUnknowns, mids)
    r('midSignSets', M.midSignSets, mids, 1); r('midSignSets', M.midSignSets, mids, -1)
  }
  r('endCurvature', M.endCurvature, [{ length: 3, r1: null, r2: -250 }])
  r('endCurvature', M.endCurvature, [])
}

/** A track of straights, arcs and clothoids from `start` (elements built as the dialogs build them). */
export function trackOf(id, start, bearing, parts, extra = {}) {
  let at = { ...start, zone: EPSG }
  let b = bearing
  const elements = parts.map(([kind, length, r1, r2, cant = 0]) => {
    const fields = { speed: 100, cant, ...extra }
    const el = kind === 's' ? straightElement(at, endPointStraightUtm(at, b, length), fields)
      : kind === 'a' ? arcElement(at, R.switchRoutePointUtm(at, b, { length, r1, r2: r1 }), r1, fields)
        : transitionElement(at, b, length, r1, r2, { ...fields, cant: undefined, cantStart: cant, cantEnd: r2 === 'x' ? 0 : cant }).element
    at = { easting: el.endNode[0], northing: el.endNode[1], zone: EPSG }
    b = el.endBearing ?? el.bearing
    return el
  })
  return { id, epsg: EPSG, name: id, elements }
}

function recordConnections(calls) {
  const r = (op, fn, ...args) => calls.push(rec(`switch.${op}`, fn, ...args))
  const k = (op, value) => calls.push({ op: `switch.${op}`, input: [], output: JSON.parse(JSON.stringify(value)) })
  const A = { easting: 600000, northing: 5600000 }
  const line = trackOf('L', A, 90, [['s', 150], ['t', 60, null, -1200, 40], ['a', 120, -1200, null, 40], ['t', 60, -1200, null, 40], ['s', 200]])
  const kinked = { ...line, id: 'K', elements: line.elements.map((el, i) => (i === 0 ? { ...el, endBearing: el.bearing + 0.5 } : el)) }
  const bloss = { ...line, id: 'B', elements: line.elements.map((el, i) => (i === 1 ? { ...el, transitionType: 'bloss' } : el)) }

  // Placement.
  for (const [st, rev, len] of [[20, false, 33.23], [140, false, 33.23], [150, false, 27], [170, false, 45], [300, true, 33.23], [210, false, 30], [580, false, 33], [585, true, 40], [0, false, 10], [5, true, 30], [9999, false, 10]]) {
    r('placeSwitchOnTrack', (t, s, rv, l) => { const p = P.placeSwitchOnTrack(t, s, rv, l); const { cantAt: _c, ...rest } = p; return rest }, line, st, rev, len)
    const p = P.placeSwitchOnTrack(line, st, rev, len)
    if (!p.error) {
      for (const [x, on] of [[0, null], [len / 3, null], [len, null], [len / 2, { s0: 1, length: 3 }]]) {
        r('placementCantAt', (t, s, rv, l, xx, o) => P.placeSwitchOnTrack(t, s, rv, l).cantAt(xx, o), line, st, rev, len, x, on)
      }
    }
  }
  r('placeSwitchOnTrack', (t, s, rv, l) => { const p = P.placeSwitchOnTrack(t, s, rv, l); const { cantAt: _c, ...rest } = p; return rest }, kinked, 100, false, 60)
  r('placeSwitchOnTrack', (t, s, rv, l) => { const p = P.placeSwitchOnTrack(t, s, rv, l); const { cantAt: _c, ...rest } = p; return rest }, bloss, 170, false, 30)
  for (let i = 0; i < line.elements.length; i++) for (const side of ['before', 'after']) r('turnoutRunsOn', P.turnoutRunsOn, line.elements, i, side, EPSG)
  r('turnoutRunsOn', P.turnoutRunsOn, kinked.elements, 1, 'before', EPSG)
  for (const [i, dx, dy] of [[0, 30, 2], [2, 10, -3], [1, 25, 1], [4, -5, 0]]) {
    const el = line.elements[i]
    r('clickStation', P.clickStation, line, i, { easting: el.startNode[0] + dx, northing: el.startNode[1] + dy, zone: EPSG })
  }
  for (const i of [0, 4, 9]) r('switchEndAnchorRefusal', P.switchEndAnchorRefusal, line, i)

  // Connecting to an open end.
  const spur = trackOf('S', { easting: 600100, northing: 5600030 }, 95, [['s', 40], ['a', 25, 400], ['s', 12]])
  const spurT = trackOf('T', { easting: 600100, northing: 5600030 }, 95, [['s', 40], ['t', 20, null, 300]])
  const spurBack = { ...spur, id: 'SB', elements: [...spur.elements].reverse().map(el => reverseEl(el)) }
  for (const [t, end, count] of [[spur, 'END', 1], [spur, 'END', 2], [spur, 'BEGIN', 1], [spurT, 'END', 1], [spurBack, 'BEGIN', 1], [spur, 'END', 9], [{ id: 'E', epsg: EPSG, elements: [] }, 'END', 1]]) {
    r('connectionAnchor', (tr, e, o) => K.connectionAnchor(tr, e, o), t, end, { tracks: [t], switches: [], endMarks: [], count })
  }
  r('connectionAnchor', (tr, e, o) => K.connectionAnchor(tr, e, o), spur, 'END', { tracks: [spur], switches: [], endMarks: [{ id: 'bs', kind: 'buffer_stop', trackId: 'S', endpoint: 'END' }] })
  for (const [t, end, count] of [[spur, 'END', 1], [spur, 'END', 2], [spurT, 'END', 1], [spur, 'BEGIN', 1]]) {
    const { anchor } = K.connectionAnchor(t, end, { tracks: [t], switches: [], endMarks: [], count })
    if (!anchor) continue
    r('endChainDefault', K.endChainDefault, anchor)
    const last = t.elements[end === 'END' ? t.elements.length - 1 : 0]
    const node = end === 'END' ? last.endNode : last.startNode
    for (const [dx, dy, bin] of [[1.5, 0.8, 110], [0.3, -0.4, 100], [-60, 0, 90], [4, 2, 120]]) {
      const port = { easting: node[0] + dx, northing: node[1] + dy, zone: EPSG }
      for (const lead of [0, 15]) {
        r('fitConnectingElement', K.fitConnectingElement, anchor, port, bin, lead)
        const fit = K.fitConnectingElement(anchor, port, bin, lead)
        if (fit) {
          r('refitTrackEnd', K.refitTrackEnd, { ...t, heights: [{ station: 0, z: 10 }, { station: 30, z: 11 }, { station: 70, z: 12 }] }, anchor, fit, port, { speedLimit: 60 })
          r('refitCount', K.refitCount, fit)
        }
      }
    }
    const route = trackOf('R', { easting: node[0], northing: node[1] }, 30, [['s', 20]])
    r('joinRouteIntoTrack', K.joinRouteIntoTrack, t, anchor, route)
  }
  k('END_CHAIN_OPEN', K.END_CHAIN_OPEN)
  for (const mids of [[{ type: 'arc', length: null, radius: 300 }, { type: 'straight', length: null }], [{ type: 'transition', length: 10 }]]) r('endChainOpen', K.endChainOpen, mids)

  // Switch connections between two parallel tracks, two converging ones, one in a curve.
  const t1 = trackOf('P1', A, 90, [['s', 400]])
  const t2 = trackOf('P2', { easting: 600000, northing: 5599995.5 }, 90, [['s', 400]])
  const t3 = trackOf('P3', { easting: 600000, northing: 5599993 }, 90.4, [['s', 400]])
  const c1 = trackOf('C1', A, 90, [['s', 60], ['t', 50, null, -2000, 30], ['a', 300, -2000, null, 30]])
  const c2 = trackOf('C2', { easting: 600000, northing: 5599995 }, 90, [['s', 60], ['t', 50, null, -2004.5, 30], ['a', 300, -2004.5, null, 30]])
  k('CONNECTION_SPEEDS', Cn.CONNECTION_SPEEDS)
  r('formPair', Cn.formPair, C.switchTypeByLabel('300 – 1:9'))
  r('formPair', Cn.formPair, [C.switchTypeByLabel('300 – 1:9'), C.switchTypeByLabel('500 – 1:12')])
  const pick = (t, i, a) => connectionPick(t, i, a)
  const strip = (res) => res && JSON.parse(JSON.stringify(res))
  for (const [ta, ia, aa, tb, ib, ab] of [[t1, 0, 100, t2, 0, 150], [t1, 0, 100, t3, 0, 160], [t2, 0, 250, t1, 0, 200], [c1, 2, 40, c2, 2, 70], [c1, 0, 20, c2, 1, 30]]) {
    const g1raw = pick(ta, ia, aa), g2 = pick(tb, ib, ab)
    r('orientStemToward', (a, b) => strip(Cn.orientStemToward(a, b)), g1raw, g2)
    const g1 = Cn.orientStemToward(g1raw, g2)
    r('computeSwitchConnections', (a, b, s) => Cn.computeSwitchConnections(a, b, s), g1, g2, 0)
    for (const speed of [40, 60, 80, 100]) {
      for (const sh of [0, 7.5]) {
        r('solveSwitchConnection', (a, b, v, s) => strip(Cn.solveSwitchConnection(a, b, v, s)), g1, g2, speed, sh)
        const res = Cn.solveSwitchConnection(g1, g2, speed, sh)
        if (res?.valid || res?.overridable) r('buildConnectionElements', (x, v) => strip(Cn.buildConnectionElements(x, v)), strip(res), speed)
      }
    }
    const mids = [{ type: 'arc', length: 25, radius: null }, { type: 'straight', length: null }]
    r('solveSwitchConnection', (a, b, v, s, m) => strip(Cn.solveSwitchConnection(a, b, v, s, m)), g1, g2, 60, 0, mids)
    const res3 = Cn.solveSwitchConnection(g1, g2, 60, 0, [{ type: 'straight', length: null }, { type: 'arc', length: null, radius: 600 }, { type: 'straight', length: 8 }])
    r('solveSwitchConnection', (a, b, v, s, m) => strip(Cn.solveSwitchConnection(a, b, v, s, m)), g1, g2, 60, 0, [{ type: 'straight', length: null }, { type: 'arc', length: null, radius: 600 }, { type: 'straight', length: 8 }])
    if (res3?.valid || res3?.overridable) r('buildConnectionElements', (x, v) => strip(Cn.buildConnectionElements(x, v)), strip(res3), 60)
    r('solveSwitchConnection', (a, b, v, s, m, sd, f) => strip(Cn.solveSwitchConnection(a, b, v, s, m, sd, f)), g1, g2, 60, 0, null, null, [C.switchTypeByLabel('300 – 1:9'), C.switchTypeByLabel('500 – 1:12')])
    r('connectionBranchEnds', (a, b, v, o) => strip(Cn.connectionBranchEnds(a, b, v, o)), g1, g2, 60, { step: 5, reach: 60 })
  }
  r('checkMidRun', Cn.checkMidRun, [{ length: 20, r1: 300, r2: 300 }, { length: 6, r1: null, r2: null }], [{ length: 5, r1: null, r2: null }], [{ length: 6, r1: null, r2: null }, { length: 20, r1: -300, r2: -300 }], 80)
  r('checkMidRun', Cn.checkMidRun, [{ length: 20, r1: 300, r2: 300 }], [{ length: 30, r1: -200, r2: -200 }, { length: 0.0001, r1: null, r2: null }], [{ length: 20, r1: -300, r2: -300 }], 100)

  // Details.
  k('RAIL_OFFSET', RAIL_OFFSET); k('SLEEPER_OVERHANG', SLEEPER_OVERHANG)
  for (const form of C.ALL_SWITCH_TYPES) {
    r('formDetail', formDetail, form, { side: 'left' })
    if (form.symmetric || form.branch) r('formDetail', formDetail, form, { side: 'right', rail: '49E5' })
  }
  r('formDetail', formDetail, C.CROSSING_TYPES[0], {})
  const laid = [layTurnout('d1', '300 – 1:9', O, 33.3, 'left'), layTurnout('d2', '500 – 1:12', O, 33.3, 'right', 1100)]
  for (const { sw, tracks } of laid) {
    r('switchDetail', (t, s, o) => switchDetail(t, s, withForms(o)), tracks, sw, {})
    r('switchDetail', (t, s, o) => switchDetail(t, s, withForms(o)), tracks, sw, { rail: '49E5' })
    r('switchDetailScene', switchDetailScene, tracks, [sw], sw, { margin: 10 })
  }
  // A crossover: two turnouts joined by their branches, and a third close by.
  const g1 = Cn.orientStemToward(pick(t1, 0, 100), pick(t2, 0, 150)), g2 = pick(t2, 0, 150)
  const res = Cn.solveSwitchConnection(g1, g2, 60, 0)
  if (res?.valid) {
    const els = Cn.buildConnectionElements(res, 60)
    const mark = (list, id, route) => list.map(el => ({ ...el, switchBranch: true, switchRoute: route, switchId: id }))
    const conn = { id: 'X', epsg: EPSG, elements: [...mark(els.branch1, 'w1', 'branch'), ...els.midEls, ...mark(els.branch2, 'w2', 'branch')] }
    const cut = (t, s0, id) => {
      const p = P.placeSwitchOnTrack(t, s0, false, C.switchStraightLength(res.switchType))
      return p
    }
    const s1 = res.s1 + 100, s2 = 150 + res.s2
    const split = (t, s, len, id) => {
      const before = { ...t, id: `${t.id}a`, elements: [straightElement({ easting: t.elements[0].startNode[0], northing: t.elements[0].startNode[1], zone: EPSG }, endPointStraightUtm({ easting: t.elements[0].startNode[0], northing: t.elements[0].startNode[1], zone: EPSG }, 90, s))] }
      const toe = endPointStraightUtm({ easting: t.elements[0].startNode[0], northing: t.elements[0].startNode[1], zone: EPSG }, 90, s)
      const we = endPointStraightUtm(toe, 90, len)
      const after = { ...t, id: `${t.id}b`, elements: [straightElement(toe, we, { switchBranch: true, switchRoute: 'main', switchId: id }), straightElement(we, endPointStraightUtm(we, 90, 400 - s - len))] }
      return [before, after]
    }
    const len = C.switchStraightLength(res.switchType)
    const [a1, b1] = split(t1, s1, len, 'w1')
    const [a2, b2] = split(t2, s2 - len, len, 'w2')
    const w1 = { switchId: 'w1', kind: 'turnout', label: res.switchType.label, ports: { A: { trackId: a1.id, endpoint: 'END' }, B2: { trackId: b1.id, endpoint: 'BEGIN' }, B1: { trackId: 'X', endpoint: 'BEGIN' } } }
    const w2 = { switchId: 'w2', kind: 'turnout', label: res.switchType.label, ports: { A: { trackId: b2.id, endpoint: 'BEGIN' }, B2: { trackId: a2.id, endpoint: 'END' }, B1: { trackId: 'X', endpoint: 'END' } } }
    const tracks = [a1, b1, a2, b2, conn]
    r('connectionPartners', connectionPartners, tracks, [w1, w2], w1)
    r('connectionPartners', connectionPartners, tracks, [w1, w2], w2)
    r('switchDetailScene', switchDetailScene, tracks, [w1, w2], w1, {})
  }
}

function reverseEl(el) {
  const out = { ...el, startNode: el.endNode, endNode: el.startNode, bearing: ((el.endBearing ?? el.bearing) + 180) % 360,
    ...(el.endBearing != null ? { endBearing: (el.bearing + 180) % 360 } : {}),
    geometry: { ...el.geometry, coordinates: [...el.geometry.coordinates].reverse() } }
  if (el.radius != null) out.radius = -el.radius
  delete out.renderCoords
  return out
}

export default function record() {
  const calls = []
  recordCatalogueAndRoutes(calls)
  recordCrossingAndSymbol(calls)
  recordBodies(calls)
  recordConnections(calls)
  // Matched by their input whatever the order of its members: what the module built comes in its own order.
  const keyOf = (c) => `${c.op}\u0000${canonicalJson(c.input)}`
  const kept = new Map(frozen('switch', 'switch').filter(c => SOLVER_OPS.has(c.op)).map(c => [keyOf(c), c]))
  return { switch: calls.map(c => (SOLVER_OPS.has(c.op) ? kept.get(keyOf(c)) ?? c : c)) }
}
