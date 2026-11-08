// The vectors of the cross section and the topology diagram (crates/track)
// and of the project's validation (crates/project): the track crate's small
// project — a line parted at a turnout's toe, its branch, a track joined on,
// one that only nearly meets, a crossing — and records broken on purpose.
import * as T from '../src/track/index.js'
import * as X from '../src/crossSection/index.js'
import { elementStartStation, terrainHeightAt } from '../src/crossSection/crossSectionUtils.js'
import * as G from '../src/topology/index.js'
import { segmentHitsBox } from '../src/topology/labelPlacement.js'
import { switchTrackIds } from '../src/topology/topologyGraph.js'
import { newFindings, validateProject } from '../src/project/index.js'
import { overlappingTracks } from '../src/project/validateProject.js'
import { buildReferenceAxis, surveyFromTrace } from '../src/survey/index.js'
import { arcFrom, straightFrom, transitionElement, endPointStraightUtm } from '../src/geometry/index.js'
import { rec } from './record.mjs'

const EPSG = 25832
const chain = (start, bearing, parts, extra = {}) => {
  let at = { ...start, zone: EPSG }
  let b = bearing
  return parts.map(([kind, length, r1, r2]) => {
    const el = kind === 's' ? straightFrom(at, b, length, extra)
      : kind === 'a' ? arcFrom(at, b, length, r1, extra)
        : transitionElement(at, b, length, r1, r2, { transitionType: 'clothoid', ...extra }).element
    at = { easting: el.endNode[0], northing: el.endNode[1], zone: EPSG }
    b = el.endBearing ?? el.bearing
    return el
  })
}

function fixture() {
  const lineEls = chain({ easting: 600000, northing: 5600000 }, 80, [['s', 180], ['t', 60, null, 900], ['a', 140, 900], ['t', 60, 900, null], ['s', 220]], { speed: 120, cant: 0 })
  lineEls[2] = { ...lineEls[2], cant: 80 }
  const line = T.makeTrack({ name: '6340.001', lineNumber: 6340, trackType: 1, epsg: EPSG, status: 'existing', rails: [{ type: '60E2', from: 300, to: 400 }] }, lineEls, 'line',
    [{ station: 0, z: 100 }, { station: 250, z: 103, rv: 8000 }, { station: 660, z: 102.5 }])
  const toe = endPointStraightUtm({ easting: 600000, northing: 5600000, zone: EPSG }, 80, 100)
  const { behind, ahead } = T.splitElementAt(line, 0, toe, 80, new Set(['6340.001']))
  const branch = T.makeTrack({ name: 'Weiche 1 Zweig', trackType: 2, trackNumber: 3, epsg: EPSG },
    chain(toe, 80, [['a', 27, -300], ['s', 80]], { speed: 60, cant: 0 }), 'branch', [{ station: 0, z: 100.4 }, { station: 107, z: 100.9 }])
  const sw = { ...T.newSwitchFields('turnout'), switchId: 'sw1', name: 'switch.001', label: 'EW 60-300-1:9',
    ports: { A: { trackId: behind.id, endpoint: 'END' }, B2: { trackId: ahead.id, endpoint: 'BEGIN' }, B1: { trackId: 'branch', endpoint: 'BEGIN' } } }
  const lineEnd = ahead.elements.at(-1).endNode
  const on = T.makeTrack({ name: 'next', epsg: EPSG, lineNumber: 6340 }, chain({ easting: lineEnd[0], northing: lineEnd[1] }, ahead.elements.at(-1).endBearing ?? ahead.elements.at(-1).bearing, [['s', 90]], { speed: 120 }), 'next')
  const bEnd = branch.elements.at(-1).endNode
  const near = T.makeTrack({ name: 'near', epsg: EPSG }, chain({ easting: bEnd[0] + 0.4, northing: bEnd[1] }, branch.elements.at(-1).bearing + 0.5, [['s', 50]], { speed: 40 }), 'near')
  const cx = T.makeTrack({ name: 'cx', epsg: EPSG }, chain({ easting: 600300, northing: 5600300 }, 10, [['s', 30]], {}), 'cx')
  const cy = T.makeTrack({ name: 'cy', epsg: EPSG }, chain({ easting: 600330, northing: 5600300 }, 190, [['s', 30]], {}), 'cy')
  const crossing = { ...T.newSwitchFields('crossing'), switchId: 'cr2', name: 'crossing.002', ports: { A: { trackId: 'cx', endpoint: 'END' }, C: { trackId: 'cy', endpoint: 'BEGIN' } } }
  // A track beside the line, 4.5 m to its right, and one lying on it.
  const besideEls = chain(endPointStraightUtm({ easting: 600000, northing: 5600000, zone: EPSG }, 170, 4.5), 80, [['s', 240]], { speed: 80 })
  const beside = T.makeTrack({ name: 'beside', epsg: EPSG }, besideEls, 'beside')
  const twin = T.makeTrack({ name: 'twin', epsg: EPSG }, chain({ easting: 600020, northing: 5600003.6 }, 80, [['s', 60]], {}), 'twin')
  const tracks = [behind, ahead, branch, on, near, cx, cy, beside, twin]
  const switches = [sw, crossing]
  const endMarks = [T.newBufferStop('near', 'END', 6), T.newBoundary('next', 'END', 100)]
  const platforms = [{ id: 'p1', trackId: ahead.id, startStation: 20, endStation: 160, side: 'right', height: 550, stationName: 'Trotha', code: '1' }]
  return { tracks, switches, endMarks, platforms, behind, ahead, branch, sw }
}

export default function record() {
  const { tracks, switches, endMarks, platforms, behind, ahead, branch, sw } = fixture()

  // ── cross section ──
  const xs = []
  const x = (op, fn, ...args) => xs.push(rec(`crossSection.${op}`, fn, ...args))
  xs.push({ op: 'crossSection.RAILS', input: [], output: X.RAILS }, { op: 'crossSection.SLEEPERS', input: [], output: X.SLEEPERS },
    { op: 'crossSection.LICHTRAUM_SOURCE', input: [], output: X.LICHTRAUM_SOURCE }, { op: 'crossSection.GAUGE_PROFILES', input: [], output: X.GAUGE_PROFILES },
    { op: 'crossSection.PLANUM_EDGE', input: [], output: X.PLANUM_EDGE })
  for (const key of ['hauptgleis', 'en15273_gc', null]) x('gaugeProfile', X.gaugeProfile, key)
  x('gaugeProfileLabelKey', X.gaugeProfileLabelKey, 'nebengleis')
  for (const p of Object.values(X.GAUGE_PROFILES)) { x('gaugeProfileRing', X.gaugeProfileRing, p.points); x('gaugeProfileAreas', X.gaugeProfileAreas, p.einragungen) }
  x('gaugeProfileRing', X.gaugeProfileRing, [[0, 0], [100, 0], [100, 50], [0, 60]])
  x('gaugeProfileAreas', X.gaugeProfileAreas, [[[0, 1], [0, 2]], [], [[5, 5], [6, 5], [6, 6], [5, 5]]])
  for (const i of [0, 2, 9]) x('elementStartStation', elementStartStation, ahead, i)
  for (const s of [0, 210, 350, 1000]) x('superstructureAt', X.superstructureAt, ahead, s)
  x('superstructureAt', X.superstructureAt, { ...ahead, sleepers: [{ type: 'B90', from: 0 }, { type: 'SWITCH', to: 20 }] }, 10)
  for (const c of [0, 80, -120, 160, 2000, null]) { x('cantAngle', X.cantAngle, c); x('cantPivot', X.cantPivot, c) }
  x('rotatePoint', X.rotatePoint, [750, 12], -0.05)
  x('fitSection', X.fitSection, [[-2000, -300], [3000, 4800], [0, 0]], { w: 640, h: 480 }, 20)
  x('fitSection', X.fitSection, [[5, 5]], { w: 100, h: 50 })
  for (const s of [0, 30, 80, 135, 160, 200, 300, 395, 420, 1e4]) x('sectionAtStation', X.sectionAtStation, ahead, s)
  const ring = X.gaugeProfileRing(X.gaugeProfile('hauptgleis').points)
  for (const o of [{ cant: 0, gaugeRing: ring, rail: '54E4', sleeper: 'B70' }, { cant: 120, gaugeRing: ring, gaugeAreas: X.gaugeProfileAreas(X.gaugeProfile('hauptgleis').einragungen), rail: '60E2', sleeper: 'B90' },
    { cant: -45, rail: 'nope', sleeper: 'B70' }, {}]) x('crossSection', X.crossSection, o)
  x('placeSection', X.placeSection, [[1, 2], [-3, 4]], { offset: 4500, dz: -20, mirrored: true })
  x('placeSection', X.placeSection, [[1, 2]])
  for (const s of [10, 60, 150]) x('sectionNeighbours', X.sectionNeighbours, ahead, s, tracks, 8)
  x('sectionNeighbours', X.sectionNeighbours, behind, 90, tracks, 30)
  const terrain = [{ y: -5000, z: 99 }, { y: 0, z: 99.5 }, { y: 2000, z: null }, { y: 6000, z: 100.2 }]
  for (const y of [-6000, -2500, 0, 1000, 4000, 6000]) x('terrainHeightAt', terrainHeightAt, terrain, y)
  x('sectionLevels', X.sectionLevels, [{ z: 101.2, offset: 0 }, { z: null, offset: -3000 }, { z: 100.9, offset: 4500 }], terrain)
  x('sectionLevels', X.sectionLevels, [{ z: null, offset: 0 }, { z: null, offset: 5000 }, { z: 102, offset: -4500 }], terrain)
  x('sectionLevels', X.sectionLevels, [{ z: null, offset: 3000 }], [])
  for (const s of [-5, 0, 77.7, 1e5]) x('sectionOrigin', X.sectionOrigin, ahead, s)
  x('sectionLinePoints', X.sectionLinePoints, ahead, 50, -12, 9, 2.5)
  x('sectionLinePoints', X.sectionLinePoints, ahead, 50, 3, 1)
  x('platformSection', X.platformSection, platforms[0])
  x('platformSection', X.platformSection, { front: 1.7, width: 4, height: 760 }, { rail: '49E5', sleeper: 'SWITCH' })

  // ── topology ──
  const tp = []
  const t = (op, fn, ...args) => tp.push(rec(`topology.${op}`, fn, ...args))
  for (const s of ['W1', 'Gleis 12', '']) t('labelBox', G.labelBox, s)
  const box = { x0: 0, x1: 10, y0: 0, y1: 10 }
  for (const sg of [[-5, 5, 15, 5], [-5, -5, -1, 20], [5, 5, 5, 5], [20, 20, 30, 30], [0, 10, 10, 0]]) t('segmentHitsBox', segmentHitsBox, sg, box)
  t('placeLabels', (items, segs, nb) => G.placeLabels(items, segs, nb),
    [{ key: 'a', text: 'Weiche 1', x: 0, y: 0, offsets: [[0, -8], [0, 8], [20, 0]] }, { key: 'b', text: 'W2', x: 2, y: -6, offsets: [[0, -8], [0, 8]] }, { key: 'c', text: 'x', x: 50, y: 0, offsets: [[0, 0]] }],
    [[-30, 0, 30, 0], [0, -30, 0, -10]], [{ x0: -3, x1: 3, y0: -3, y1: 3 }])
  const graph = G.buildTopologyGraph(tracks, switches, endMarks)
  t('buildTopologyGraph', G.buildTopologyGraph, tracks, switches, endMarks)
  t('buildTopologyGraph', G.buildTopologyGraph, tracks, [{ ...sw, swapped: true }], [])
  t('topologyClusters', G.topologyClusters, graph)
  const { clusters } = G.topologyClusters(graph)
  for (const c of clusters) t('layoutClusterLanes', G.layoutClusterLanes, c)
  // A larger network: a line with three turnouts, a crossover and a loop.
  const big = (() => {
    const els = chain({ easting: 601000, northing: 5601000 }, 90, [['s', 900]], { speed: 100 })
    let main = T.makeTrack({ name: 'main', epsg: EPSG }, els, 'm')
    const parts = []
    const sws = []
    let rest = main
    for (const [k, at] of [[1, 150], [2, 400], [3, 650]]) {
      const p = endPointStraightUtm({ easting: 601000, northing: 5601000, zone: EPSG }, 90, at)
      const { behind: b, ahead: a } = T.splitElementAt(rest, 0, p, at - (900 - T.trackLength(rest)), new Set())
      parts.push(b)
      rest = a
      const side = k === 2 ? 1 : -1
      const br = T.makeTrack({ name: `b${k}`, epsg: EPSG }, chain(p, 90, [['a', 30, side * 300], ['s', 120]], {}), `b${k}`)
      parts.push(br)
      sws.push({ ...T.newSwitchFields('turnout'), switchId: `w${k}`, name: `W${k}`, ports: { A: { trackId: b.id, endpoint: 'END' }, B2: { trackId: a.id, endpoint: 'BEGIN' }, B1: { trackId: br.id, endpoint: 'BEGIN' } } })
    }
    parts.push(rest)
    return { tracks: parts, switches: sws }
  })()
  const bigGraph = G.buildTopologyGraph(big.tracks, big.switches, [])
  t('buildTopologyGraph', G.buildTopologyGraph, big.tracks, big.switches, [])
  for (const c of G.topologyClusters(bigGraph).clusters) t('layoutClusterLanes', G.layoutClusterLanes, c)
  for (const sel of [null, { kind: 'switch', id: 'sw1' }, { kind: 'switch', id: 'gone' }, { kind: 'track', id: 'branch' }]) t('selectionHighlight', G.selectionHighlight, sel, switches)
  t('switchTrackIds', switchTrackIds, sw)

  // ── validation ──
  const pj = []
  const v = (op, fn, ...args) => pj.push(rec(`project.${op}`, fn, ...args))
  const good = { id: 'p', tracks: tracks.filter(tr => tr.id !== 'twin'), switches, endMarks, platforms }
  v('validateProject', validateProject, good)
  const survey = surveyFromTrace({ id: 's1', name: 'S' }, { points: [{ station: 0, easting: 600001, northing: 5600001, zLeft: 100, zRight: 100, gauge: 1.435, quality: 'good' }], gaps: [], epsg: EPSG })
  const axis = buildReferenceAxis({ name: 'R', epsg: EPSG, elements: behind.elements, from: 0, to: 50, tra: 't', importedAt: 'x', id: 'r1' })
  const gap = { ...branch, elements: [branch.elements[0], { ...branch.elements[1], startNode: [branch.elements[1].startNode[0] + 0.03, branch.elements[1].startNode[1]], length: branch.elements[1].length + 3, epsg: 31468 }],
    heights: [{ station: 0, z: 1 }, { station: 0, z: 2 }, { station: 900, z: 3 }] }
  const broken = {
    ...good,
    tracks: [...good.tracks.map(tr => (tr.id === 'branch' ? gap : tr)), twin(tracks), { id: 'x9', name: 'odd', epsg: EPSG, elements: [{ ...behind.elements[0], switchId: 'ghost' }] }],
    switches: [...switches, { ...T.newSwitchFields('turnout'), switchId: 'sw3', name: 'W3', ports: { A: { trackId: behind.id, endpoint: 'END' }, B1: { trackId: 'gone', endpoint: 'BEGIN' }, B2: { trackId: 'cx', endpoint: 'MIDDLE' } } }],
    platforms: [...platforms, { id: 'p2', trackId: 'gone' }, { id: 'p3', trackId: 'cx', startStation: -3, endStation: 10 }],
    endMarks: [...endMarks, { id: 'm1', trackId: behind.id, endpoint: 'END' }, { id: 'm2', trackId: 'near', endpoint: 'END' }, { id: 'm3', trackId: 'gone', endpoint: 'BEGIN' }],
    axisSurveys: [survey, { ...survey, id: 's2', points: { ...survey.points, dn: [] } }],
    referenceAxes: [axis, { ...axis, id: 'r2', points: { ...axis.points, step: 0 } }],
    routes: [{ id: 'rt1', name: 'R1', trackIds: [behind.id, ahead.id, 'gone', 'cx'] }, { id: 'rt2', trackIds: [] }],
  }
  v('validateProject', validateProject, broken)
  v('validateProject', validateProject, {})
  const a1 = validateProject(good), a2 = validateProject(broken)
  v('newFindings', newFindings, a2.errors, a1.errors)
  v('newFindings', newFindings, a2.warnings, null)
  v('overlappingTracks', overlappingTracks, [...tracks], switches)
  v('overlappingTracks', overlappingTracks, [ahead, { ...ahead, id: 'copy' }], [])
  return { track: { cross_section: xs, topology: tp }, project: { validate: pj } }
}

function twin(tracks) { return tracks.find(tr => tr.id === 'twin') }
