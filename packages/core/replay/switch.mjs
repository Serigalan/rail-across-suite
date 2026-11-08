// The switch crate's vectors (crates/switch): every form of the catalogue
// read the way the geometry reads it, routes and chains laid from a point.
import * as C from '../src/switch/catalogue.js'
import * as R from '../src/switch/route.js'
import { WEICHEN_REGELWERK, weichenGruppen } from '../src/switch/weichenRegelwerk.js'
import { utmToWgs84 } from '../src/geometry/index.js'
import { rec } from './record.mjs'

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

export default function record() {
  const calls = []
  recordCatalogueAndRoutes(calls)
  return { switch: calls }
}
