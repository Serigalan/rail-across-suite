// The geometry crate's vectors (crates/geom): the planes and their conversions
// on proj4js (7-parameter shifts; grids are held to the JavaScript in the
// core's tests where the files are there), transitions, element values, the
// factory, parallels, plane changes, chain checks and closing, and cants —
// recorded from the JavaScript on inputs drawn from a fixed seed.
import * as g from '../src/geometry/index.js'
import { moveTrackEnd, refitToNodes } from '../src/geometry/chainSnap.js'
import { tangentBreaks } from '../src/geometry/chainChecks.js'
import { transformElement } from '../src/geometry/planeTransform.js'
import {
  arcPlanePoints, computeCurvedValuesUtm, computeStraightValuesUtm, endPointCurvedUtm, endPointStraightUtm,
  projectOnArcUtm, projectOnBearingUtm, signedRadiusFrom3PointsUtm,
} from '../src/geometry/elementUtils.js'
import { sampleTransitionUtm, transitionPointAtUtm, transitionShift, cantEnds, transitionCantEnds, withCantRamp } from '../src/geometry/clothoidUtils.js'

const clone = (v) => JSON.parse(JSON.stringify(v))
function rng(seed) {
  return () => {
    seed |= 0; seed = (seed + 0x6D2B79F5) | 0
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
// A bearing or a ratio read off two converted points carries their last
// digit, amplified: held to 1e-7 [° or m] rather than 1e-9.
const DERIVED = new Set(['geometry.transformGridBearing', 'geometry.transformTrackToPlane', 'geometry.transformElement'])
const call = (op, input, fn) => ({ op, input: clone(input), output: clone(fn()), ...(DERIVED.has(op) ? { tol: 1e-7 } : {}) })

// Each plane with a point in its area, in WGS 84.
const PLANES = [
  [25832, 9.2, 50.1], [25833, 13.4, 52.5], [5682, 6.8, 50.9], [5683, 9.1, 51.2], [5684, 12.0, 51.4], [5685, 14.6, 51.8],
  [5680, 3.4, 50.0], [5677, 9.3, 49.4], [5678, 11.9, 48.3], [3397, 11.2, 50.8], [3398, 12.4, 51.1], [2398, 12.1, 53.2],
  [3068, 13.4, 52.5], [32632, 8.4, 47.4], [32733, 15.1, -12.3],
]

export default function record() {
  const r = rng(20261108)
  const planes = []
  for (const [crs, lng, lat] of PLANES) {
    for (let i = 0; i < 6; i++) {
      const lngLat = [lng + (r() - 0.5) * 1.2, lat + (r() - 0.5) * 1.0]
      planes.push(call('geometry.wgs84ToUTM', { lngLat, crs }, () => g.wgs84ToUTM(lngLat, crs)))
      const p = g.wgs84ToUTM(lngLat, crs)
      planes.push(call('geometry.utmToWgs84', { easting: p.easting, northing: p.northing, crs }, () => g.utmToWgs84(p.easting, p.northing, crs)))
      for (const [to] of PLANES.filter(([c]) => c !== crs && c !== 32733 && crs !== 32733).filter((_, k) => (k + i) % 4 === 0)) {
        planes.push(call('geometry.transformPlanePoint', { easting: p.easting, northing: p.northing, fromCrs: crs, toCrs: to },
          () => g.transformPlanePoint(p.easting, p.northing, crs, to)))
        const bearing = r() * 360
        planes.push(call('geometry.transformGridBearing', { easting: p.easting, northing: p.northing, bearing, fromCrs: crs, toCrs: to },
          () => g.transformGridBearing(p.easting, p.northing, bearing, crs, to)))
      }
    }
    planes.push(call('geometry.crsName', { crs }, () => g.crsName(crs) ?? null))
    planes.push(call('geometry.crsSystem', { crs }, () => g.crsSystem(crs)))
    planes.push(call('geometry.crsCentre', { crs }, () => g.crsCentre(crs)))
    planes.push(call('geometry.gkZone', { crs }, () => g.gkZone(crs)))
    planes.push(call('geometry.crsDatum', { crs }, () => g.crsDatum(crs)))
    planes.push(call('geometry.epsgForLngLat', [lng, lat], () => g.epsgForLngLat([lng, lat])))
  }
  for (const key of ['ETRS89_UTM', 'DBREF_GK', 'PD83_GK', 'DHDN_SOLDNER', 'WGS84_UTM_S']) {
    for (const lng of [6.1, 9.9, 13.7]) planes.push(call('geometry.systemCodeAt', { key, lng }, () => g.systemCodeAt(key, lng)))
  }

  const transitions = []
  for (let i = 0; i < 40; i++) {
    const start = { easting: 600000 + r() * 1000, northing: 5600000 + r() * 1000, zone: [25832, 5683, 5677][i % 3] }
    const bearing = r() * 360
    const length = 10 + r() * 190
    const R = (r() < 0.5 ? -1 : 1) * (180 + r() * 4000)
    const [r1, r2] = [[null, R], [R, null], [R, R * (1.5 + r())], [R, -R * 2]][i % 4]
    const type = i % 5 === 0 ? 'bloss' : 'clothoid'
    transitions.push(call('geometry.sampleTransitionUtm', { start, bearing, length, r1, r2, type, steps: 7, subdiv: 3, end: length * 0.8 },
      () => sampleTransitionUtm(start, bearing, length, r1, r2, type, { steps: 7, subdiv: 3, end: length * 0.8 })))
    transitions.push(call('geometry.computeClothoidUtm', { start, bearing, length, r1, r2, maxDeviation: 0.05, type },
      () => g.computeClothoidUtm(start, bearing, length, r1, r2, 0.05, type)))
    const s = length * r()
    transitions.push(call('geometry.transitionPointAtUtm', { start, bearing, length, r1, r2, type, s }, () => transitionPointAtUtm(start, bearing, length, r1, r2, type, s)))
    transitions.push(call('geometry.transitionBearingAtUtm', { bearing, length, r1, r2, type, s }, () => g.transitionBearingAtUtm(bearing, length, r1, r2, type, s)))
    transitions.push(call('geometry.clothoidRadiusAt', { r1, r2, length, s }, () => g.clothoidRadiusAt(r1, r2, length, s)))
    transitions.push(call('geometry.transitionShift', { L: length, R: Math.abs(R), type }, () => transitionShift(length, Math.abs(R), type)))
    const foot = transitionPointAtUtm(start, bearing, length, r1, r2, type, s)
    const point = { easting: foot.easting + (r() - 0.5) * 8, northing: foot.northing + (r() - 0.5) * 8, zone: start.zone }
    transitions.push(call('geometry.projectOnTransitionUtm', { start, point, bearing, length, r1, r2, type },
      () => g.projectOnTransitionUtm(start, point, bearing, length, r1, r2, type)))
  }

  const elements = []
  const made = []
  for (let i = 0; i < 30; i++) {
    const zone = [25832, 5683, 5677][i % 3]
    const start = { easting: 600000 + r() * 1000, northing: 5600000 + r() * 1000, zone }
    const bearing = r() * 360
    const length = 5 + r() * 300
    const radius = (r() < 0.5 ? -1 : 1) * (190 + r() * 3000)
    const end = endPointStraightUtm(start, bearing, length)
    const arcEnd = endPointCurvedUtm(start, bearing, length, radius)
    elements.push(call('geometry.endPointStraightUtm', { start, bearing, length }, () => endPointStraightUtm(start, bearing, length)))
    elements.push(call('geometry.endPointCurvedUtm', { start, bearing, length, radius }, () => endPointCurvedUtm(start, bearing, length, radius)))
    elements.push(call('geometry.computeStraightValuesUtm', { start, end }, () => computeStraightValuesUtm(start, end)))
    elements.push(call('geometry.computeCurvedValuesUtm', { start, end: arcEnd, radius }, () => computeCurvedValuesUtm(start, arcEnd, radius)))
    elements.push(call('geometry.arcPlanePoints', { start, end: arcEnd, radius, maxDeviation: 0.05, minSegments: 3 }, () => arcPlanePoints(start, arcEnd, radius, 0.05, 3)))
    const point = { easting: start.easting + (r() - 0.5) * 50, northing: start.northing + (r() - 0.5) * 50, zone }
    elements.push(call('geometry.projectOnBearingUtm', { start, point, bearing }, () => projectOnBearingUtm(start, point, bearing)))
    elements.push(call('geometry.projectOnArcUtm', { start, point, bearing, radius }, () => projectOnArcUtm(start, point, bearing, radius)))
    elements.push(call('geometry.signedRadiusFrom3PointsUtm', { p1: start, p2: point, p3: arcEnd }, () => signedRadiusFrom3PointsUtm(start, point, arcEnd)))
    elements.push(call('geometry.bearingAfterUtm', { bearing, length, radius }, () => g.bearingAfterUtm(bearing, length, radius)))
    const extra = { speed: 80, cant: i % 7 }
    const st = g.straightFrom(start, bearing, length, extra)
    const arc = g.arcFrom(start, bearing, length, radius, extra)
    const tr = g.transitionElement(start, bearing, length, null, radius, { transitionType: i % 4 ? 'clothoid' : 'bloss', ...extra })
    elements.push(call('geometry.straightFrom', { start, bearing, length, extra }, () => st))
    elements.push(call('geometry.arcFrom', { start, bearing, length, radius, extra }, () => arc))
    elements.push(call('geometry.transitionElement', { start, bearing, length, r1: null, r2: radius, transitionType: i % 4 ? 'clothoid' : 'bloss', extra },
      () => tr))
    made.push([st, zone], [arc, zone], [tr.element, zone])
  }
  for (const [el, epsg] of made) {
    elements.push(call('geometry.derivedEndBearing', el, () => g.derivedEndBearing(el)))
    elements.push(call('geometry.resolveEndBearing', { el, epsg }, () => g.resolveEndBearing(el, epsg)))
    elements.push(call('geometry.reverseElement', el, () => g.reverseElement(el)))
    elements.push(call('geometry.displayCoords', { el, epsg, spacing: 30 }, () => g.displayCoords(el, epsg, 30)))
  }

  // Tracks: straight – transition – arc – transition – straight, laid by the factory.
  const tracks = []
  const chains = []
  for (let i = 0; i < 8; i++) {
    const epsg = [25832, 5683, 5677, 25833][i % 4]
    let at = { easting: 600000 + r() * 2000, northing: 5600000 + r() * 2000, zone: epsg }
    let bearing = r() * 360
    const R = (r() < 0.5 ? -1 : 1) * (300 + r() * 2000)
    const els = []
    const push = (el) => { els.push(el); at = { easting: el.endNode[0], northing: el.endNode[1], zone: epsg }; bearing = el.endBearing ?? el.bearing }
    push(g.straightFrom(at, bearing, 100 + r() * 300, { speed: 120 }))
    push(g.transitionElement(at, bearing, 60 + r() * 60, null, R, { speed: 120, cantStart: 0, cantEnd: 60 }).element)
    push(g.arcFrom(at, bearing, 80 + r() * 200, R, { speed: 120, cant: 60 }))
    push(g.transitionElement(at, bearing, 60 + r() * 60, R, null, { speed: 120 }).element)
    push(g.straightFrom(at, bearing, 100 + r() * 300, { speed: 120 }))
    const track = { id: `t${i}`, epsg, elements: els, heights: [{ station: 0, z: 100 }, { station: 300, z: 101.5, rv: 8000 }, { station: 600, z: 99 }] }
    chains.push(track)
    tracks.push(call('geometry.reconstructElements', { elements: els, epsg }, () => g.reconstructElements(els, epsg)))
    for (const dist of [4.5, -3.8]) {
      tracks.push(call('geometry.offsetTrackElements', { elements: els, dist, epsg }, () => g.offsetTrackElements(els, dist, epsg)))
    }
    const toCrs = { 25832: 5683, 5683: 25832, 5677: 5683, 25833: 5685 }[epsg]
    tracks.push(call('geometry.transformTrackToPlane', { track, toCrs }, () => g.transformTrackToPlane(track, toCrs)))
    tracks.push(call('geometry.transformElement', { el: els[2], fromCrs: epsg, toCrs }, () => transformElement(els[2], epsg, toCrs)))
    tracks.push(call('geometry.nodeGaps', { elements: els, tol: 1e-9 }, () => g.nodeGaps(els, 1e-9)))
    tracks.push(call('geometry.tangentBreaks', { elements: els, epsg, tol: 1e-9 }, () => tangentBreaks(els, epsg, 1e-9)))
    tracks.push(call('geometry.untrueLengths', { elements: els }, () => g.untrueLengths(els)))
    tracks.push(call('geometry.epsgMismatches', { ...track, elements: [...els, { ...els[0], epsg: 1 }] }, () => g.epsgMismatches({ ...track, elements: [...els, { ...els[0], epsg: 1 }] })))
    // A chain whose joints miss: a millimetre and less closed, more left open.
    const loose = els.map((el, k) => (k ? { ...el, startNode: [el.startNode[0] + 0.0003 * k, el.startNode[1] - 0.0002] } : el))
    tracks.push(call('geometry.snapNodeGaps', { elements: loose, epsg, tol: 0.001 }, () => g.snapNodeGaps(loose, epsg, 0.001)))
    tracks.push(call('geometry.refitToNodes', { el: loose[1], epsg, lengthChange: 0.0002 }, () => refitToNodes(loose[1], epsg, 0.0002)))
    tracks.push(call('geometry.storedTrackCants', { elements: els }, () => g.storedTrackCants(els)))
    tracks.push(call('geometry.loadedTrackCants', { elements: g.storedTrackCants(els) }, () => g.loadedTrackCants(g.storedTrackCants(els))))
    for (const k of [0, 1, 2]) {
      tracks.push(call('geometry.cantEnds', { elements: els, i: k }, () => cantEnds(els, k)))
    }
    tracks.push(call('geometry.transitionCantEnds', { elements: els, i: 3 }, () => transitionCantEnds(els, 3)))
    tracks.push(call('geometry.withCantRamp', { el: els[2], start: 40, end: 70 }, () => withCantRamp(els[2], 40, 70)))
    tracks.push(call('geometry.withCantAtEnd', { elements: els, endpoint: i % 2 ? 'END' : 'BEGIN', v: 25 }, () => g.withCantAtEnd(els, i % 2 ? 'END' : 'BEGIN', 25)))
    tracks.push(call('geometry.withCantsOver', { elements: els, from: 1, to: 2, ends: [[10, 50], [50, 50]] }, () => g.withCantsOver(els, 1, 2, [[10, 50], [50, 50]])))
    tracks.push(call('geometry.withCantEdit', { elements: els, i: 2, key: 'cant', value: 45 }, () => g.withCantEdit(els, 2, 'cant', 45)))
    tracks.push(call('geometry.withCantEdit', { elements: els, i: 1, key: 'cantEnd', value: 35 }, () => g.withCantEdit(els, 1, 'cantEnd', 35)))
  }
  // Track ends that meet within a millimetre, and one that does not.
  const [a, b] = chains
  const near = (t, dx) => ({ ...t, id: `${t.id}b`, elements: [{ ...t.elements[0], startNode: [a.elements.at(-1).endNode[0] + dx, a.elements.at(-1).endNode[1]] }, ...t.elements.slice(1)] })
  const set = [a, near(b, 0.0004), near(chains[2], 0.003)]
  tracks.push(call('geometry.snapTrackJoints', { tracks: set, tol: 0.001 }, () => g.snapTrackJoints(set, 0.001)))
  tracks.push(call('geometry.moveTrackEnd', { track: a, endpoint: 'END', node: [a.elements.at(-1).endNode[0] + 0.0005, a.elements.at(-1).endNode[1]] }, () => moveTrackEnd(a, 'END', [a.elements.at(-1).endNode[0] + 0.0005, a.elements.at(-1).endNode[1]])))
  return { planes, transitions, elements, tracks }
}
