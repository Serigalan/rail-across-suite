// The track crate's vectors (crates/track): a small project laid by the
// factory — a line parted at a turnout's toe, its branch, a track joined end
// to end and one that only nearly meets, a crossing, a buffer stop, a
// boundary, platforms, a route, heights with vertical curves, a kilometrage
// line — and every function of the track module called on it.
import * as T from '../src/track/index.js'
import { arcFrom, straightFrom, transitionElement, utmToWgs84, endPointStraightUtm } from '../src/geometry/index.js'
import { offsetEdgeCoords } from '../src/track/platformUtils.js'
import { claimedEnds, jointFit, linkRecord, trackEndAt } from '../src/track/trackLinkUtils.js'
import { mergeableRun } from '../src/track/mergeElements.js'
import { piecesAlong, routePath } from '../src/track/routes.js'
import { tangentLength } from '../src/track/heightUtils.js'
import { checkTrack } from '../src/rules/index.js'
import { rec, rng } from './record.mjs'

const EPSG = 25832
const chain = (start, bearing, parts, extra = {}) => {
  let at = { ...start, zone: EPSG }
  let b = bearing
  return parts.map(([kind, length, r1, r2]) => {
    const el = kind === 's' ? straightFrom(at, b, length, extra)
      : kind === 'a' ? arcFrom(at, b, length, r1, extra)
        : transitionElement(at, b, length, r1, r2, { transitionType: r2 === 'bloss' ? 'bloss' : 'clothoid', ...extra }).element
    at = { easting: el.endNode[0], northing: el.endNode[1], zone: EPSG }
    b = el.endBearing ?? el.bearing
    return el
  })
}

export default function record() {
  const rand = rng(4040)
  const calls = []
  const r = (op, fn, ...args) => calls.push(rec(`track.${op}`, fn, ...args))

  // The line: straight, transition, arc, transition, straight; heights along it.
  const lineEls = chain({ easting: 600000, northing: 5600000 }, 80, [['s', 180], ['t', 60, null, 900], ['a', 140, 900], ['t', 60, 900, null], ['s', 220]], { speed: 120, cant: 0 })
  const line = T.makeTrack({ name: '6340.001', lineNumber: 6340, trackType: 1, epsg: EPSG, status: 'existing' }, lineEls, 'line',
    [{ station: 0, z: 100 }, { station: 250, z: 103, rv: 8000 }, { station: 450, z: 101, rv: 6000 }, { station: 660, z: 102.5 }])
  // Parted inside its first straight at 100 m — where a turnout's toe goes.
  const toe = endPointStraightUtm({ easting: 600000, northing: 5600000, zone: EPSG }, 80, 100)
  const names = new Set(['6340.001'])
  r('splitElementAt', (t, i, j, b, n) => T.splitElementAt(t, i, j, b, new Set(n)), line, 0, toe, 80, [...names])
  const split = T.splitElementAt(line, 0, toe, 80, names)
  const [behind, ahead] = [split.behind, split.ahead]
  r('splitTrackAtJoint', (t, j, b, n) => T.splitTrackAtJoint(t, j, b, new Set(n)), line, 2, 80, ['6340.001'])
  // The branch leaves the toe to the left.
  const branch = T.makeTrack({ name: 'Weiche 1 Zweig', trackType: 2, stationName: 'Halle-Trotha', uicStation: '8010159', trackNumber: 3, epsg: EPSG },
    chain(toe, 80, [['a', 27, -300], ['s', 80]], { speed: 60, cant: 0 }), 'branch', [{ station: 0, z: 100.4 }, { station: 107, z: 100.9 }])
  const sw = { ...T.newSwitchFields('turnout'), name: 'switch.001', number: 1, label: 'EW 60-300-1:9', speed: 60,
    ports: { A: { trackId: behind.id, endpoint: 'END' }, B2: { trackId: ahead.id, endpoint: 'BEGIN' }, B1: { trackId: 'branch', endpoint: 'BEGIN' } } }
  // A track joined end to end to the line's end, and one that only nearly meets the branch's.
  const lineEnd = ahead.elements.at(-1).endNode
  const on = T.makeTrack({ name: 'next', epsg: EPSG, lineNumber: 6340 }, chain({ easting: lineEnd[0], northing: lineEnd[1] }, ahead.elements.at(-1).endBearing ?? ahead.elements.at(-1).bearing, [['s', 90]], { speed: 120 }), 'next')
  const bEnd = branch.elements.at(-1).endNode
  const near = T.makeTrack({ name: 'near', epsg: EPSG }, chain({ easting: bEnd[0] + 0.4, northing: bEnd[1] }, branch.elements.at(-1).bearing + 0.5, [['s', 50]], { speed: 40 }), 'near')
  // A crossing of two short tracks, and a link record.
  const cx = T.makeTrack({ name: 'cx', epsg: EPSG }, chain({ easting: 600300, northing: 5600300 }, 10, [['s', 30]], {}), 'cx')
  const cy = T.makeTrack({ name: 'cy', epsg: EPSG }, chain({ easting: 600330, northing: 5600300 }, 190, [['s', 30]], {}), 'cy')
  const crossing = { ...T.newSwitchFields('crossing'), name: 'crossing.002', ports: { A: { trackId: 'cx', endpoint: 'END' }, C: { trackId: 'cy', endpoint: 'BEGIN' } } }
  const tracks = [behind, ahead, branch, on, near, cx, cy]
  const switches = [sw, crossing]
  const marks = [T.newBufferStop('near', 'END', 6), T.newBoundary('next', 'END', 100), T.newBufferStop('branch', 'END'), { id: 'stale', kind: 'buffer_stop', trackId: 'gone', endpoint: 'BEGIN' }]
  const platforms = [
    { id: 'p1', trackId: ahead.id, startStation: 20, endStation: 160, side: 'right', height: 550 },
    { id: 'p2', trackId: 'branch', startStation: 30, endStation: 90, side: 'left', height: 760, frontOffset: 1.7 },
  ]
  const byId = Object.fromEntries(tracks.map(t => [t.id, t]))

  for (const kind of ['turnout', 'crossing', 'single_slip', 'double_slip', 'link', 'nonsense', null]) {
    r('switchRoutePorts', T.switchRoutePorts, kind)
    r('switchRoutes', (k) => Object.keys(T.switchRoutePorts(k)), kind)
    r('switchKindLabelKey', T.switchKindLabelKey, kind)
    for (const route of ['main', 'branch', 'cross', 'slip1', 'slip2', 'x']) r('switchRouteLabelKey', T.switchRouteLabelKey, kind, route)
    r('portsOf', T.portsOf, { kind })
  }
  for (const s of [sw, { ...sw, swapped: true }, crossing]) {
    r('turnoutLinePort', T.turnoutLinePort, s); r('turnoutDivergingPort', T.turnoutDivergingPort, s); r('turnoutLineRoute', T.turnoutLineRoute, s)
    r('isLinkSwitch', T.isLinkSwitch, s); r('isModelledSwitch', T.isModelledSwitch, s); r('switchElementMark', T.switchElementMark, s, 'branch')
    for (const p of ['A', 'B1', 'B2', 'C', 'X']) { r('portTrack', T.portTrack, s, p); r('portEnd', T.portEnd, s, p) }
    r('withPort', T.withPort, s, 'A', { endpoint: 'BEGIN' })
    r('switchStatus', (x, ts) => T.switchStatus(x, new Map(ts.map(t => [t.id, t]))), s, [{ ...branch, status: 'new' }, behind, ahead])
    r('portTracks', T.portTracks, s); r('referencesTrack', T.referencesTrack, s, 'branch')
    r('switchMatches', T.switchMatches, s, 'switch 001'); r('switchMatches', T.switchMatches, s, 'EW 1:9')
  }
  r('newSwitchFields', T.newSwitchFields, 'link')
  const marked = { ...branch.elements[0], ...T.switchElementMark(sw, 'branch'), cantException: 'x' }
  r('elementBelongsToSwitch', T.elementBelongsToSwitch, marked, sw)
  r('elementOnSwitchRoute', T.elementOnSwitchRoute, marked, sw, 'branch'); r('elementOnSwitchRoute', T.elementOnSwitchRoute, marked, sw, 'main')
  r('unmarkSwitchElement', T.unmarkSwitchElement, marked)

  for (const t of tracks) {
    r('trackLength', T.trackLength, t); r('trackStatus', T.trackStatus, t); r('trackLabel', T.trackLabel, t)
    r('trackKind', T.trackKind, t); r('trackGroupKey', T.trackGroupKey, t); r('trackListLabel', T.trackListLabel, t)
    r('trackTypeName', T.trackTypeName, t, 'track'); r('trackMatches', T.trackMatches, t, 'halle 3'); r('trackMatches', T.trackMatches, t, '6340')
    r('trackEndAnchor', T.trackEndAnchor, t); r('reverseTrack', T.reverseTrack, t); r('trackProfile', T.trackProfile, t)
    r('inheritedSpeed', T.inheritedSpeed, t, switches)
    for (const end of ['BEGIN', 'END']) {
      r('trackEndAt', trackEndAt, t, end); r('tracksOnFrom', T.tracksOnFrom, t.id, end, tracks, switches)
      r('adjacentTracks', T.adjacentTracks, tracks, switches, t, end)
    }
    r('elementStations', T.elementStations, t)
    const total = T.trackLength(t)
    for (const s of [0, total * 0.31, total * 0.77, total]) {
      r('pointAtStation', T.pointAtStation, t, s)
      r('elementAtStation', T.elementAtStation, t.elements, s)
      const hit = T.elementAtStation(t.elements, s)
      r('pointAtStationUtm', T.pointAtStationUtm, hit.el, hit.s, t.epsg)
      const p = T.pointAtStation(t, s)
      if (p) r('stationOnTrack', T.stationOnTrack, t, { easting: p.utm.easting + 1.3, northing: p.utm.northing - 0.7, zone: EPSG })
    }
    r('trackSamplePoints', T.trackSamplePoints, t, [0, total / 3, total])
    r('offsetEdgeCoords', offsetEdgeCoords, t, total * 0.1, total * 0.9, 2.5)
    r('mergeChain', T.mergeChain, t.elements, t.epsg)
    r('mergeChain', T.mergeChain, t.elements, t.epsg, { equal: true })
    r('curvatureBand', T.curvatureBand, t.elements); r('cantBand', T.cantBand, t.elements); r('speedBand', T.speedBand, t.elements)
    r('elementSpans', T.elementSpans, t.elements)
    const band = T.curvatureBand(t.elements)
    r('bandValueAt', T.bandValueAt, band, total / 2); r('bandRuns', T.bandRuns, band); r('bandRange', T.bandRange, band)
    r('bandFindings', T.bandFindings, checkTrack(t.elements))
  }
  r('mergeableRun', mergeableRun, [behind.elements[0], ahead.elements[0]], EPSG)
  // A straight cut in two, merged again.
  const halves = T.splitElementAt({ ...line, elements: [lineEls[0]] }, 0, endPointStraightUtm({ easting: 600000, northing: 5600000, zone: EPSG }, 80, 60), 80, new Set()).tracks
  r('mergeChain', T.mergeChain, [...halves[0].elements, ...halves[1].elements], EPSG)
  r('withBandValue', T.withBandValue, lineEls, [0, 2, 4], 'cant', 45)
  r('withBandValue', T.withBandValue, lineEls, [1, 2], 'speed', 100)
  r('withBandValue', T.withBandValue, lineEls, [2], 'speed', null)
  r('withCantEnds', T.withCantEnds, lineEls, [1, 3], { start: 10, end: 55 })
  r('withCantEnds', T.withCantEnds, lineEls, [1], { start: null })

  r('remapSwitches', T.remapSwitches, switches, [{ oldId: behind.id, newId: ['x1', 'x2'] }, { oldId: 'branch', newId: 'b2', flip: true }])
  r('flipSwitchEndpoints', T.flipSwitchEndpoints, switches, 'branch')
  r('rebuildCoords', T.rebuildCoords, lineEls)
  r('nextTrackName', T.nextTrackName, '6340', ['6340.001', '6340.002'])
  r('joinTracks', T.joinTracks, behind, ahead)
  r('makeTrack', T.makeTrack, { name: 'x', epsg: EPSG, id: 'old', coordinates: [] }, lineEls.slice(0, 2), undefined, [{ station: 0, z: 1 }])

  r('switchNodes', T.switchNodes, tracks, switches)
  r('classifyTrackEnds', T.classifyTrackEnds, tracks, switches, marks)
  r('freeEnds', T.freeEnds, T.classifyTrackEnds(tracks, switches, marks))
  r('findTrackJoints', T.findTrackJoints, tracks, switches, {})
  r('findTrackJoints', T.findTrackJoints, tracks, [], { tol: 3 })
  r('existingLinks', T.existingLinks, [{ ...T.newSwitchFields('link'), ports: { A: { trackId: 'branch', endpoint: 'END' }, B: { trackId: 'near', endpoint: 'BEGIN' } } }], tracks)
  r('linkAllJoints', T.linkAllJoints, tracks, switches, ['link.001'])
  r('linkSymbol', T.linkSymbol, linkRecord({ a: trackEndAt(branch, 'END'), b: trackEndAt(near, 'BEGIN') }, 'link.009'), byId)
  r('jointFit', jointFit, trackEndAt(branch, 'END'), trackEndAt(near, 'BEGIN'))
  r('claimedEnds', (s) => [...claimedEnds(s)].sort(), switches)

  r('endKey', T.endKey, 'branch', 'END')
  r('newBufferStop', T.newBufferStop, 'near', 'BEGIN', 8)
  r('newBoundary', T.newBoundary, 'next', 'BEGIN')
  for (const m of marks) {
    r('endMarkName', T.endMarkName, m, byId[m.trackId] ?? null)
    r('bufferStopStations', T.bufferStopStations, 50, m); r('endMarkStation', T.endMarkStation, 50, m)
  }
  r('endMarkName', T.endMarkName, { kind: 'buffer_stop', endpoint: 'END', name: '  Pb 3 ' }, branch)
  r('bufferStopFits', T.bufferStopFits, 10, 6); r('bufferStopFits', T.bufferStopFits, 8, 6)
  r('remapEndMarks', T.remapEndMarks, marks, [{ oldId: 'near', newId: ['n1', 'n2'] }, { oldId: 'next', newId: 'nx', flip: true }], [{ trackId: 'branch', endpoint: 'END' }])
  r('flipEndMarks', T.flipEndMarks, marks, 'near')
  r('pruneEndMarks', T.pruneEndMarks, marks, tracks, switches)
  r('bufferStopFeatures', T.bufferStopFeatures, tracks, marks)
  r('boundaryFeatures', T.boundaryFeatures, tracks, marks)

  for (const f of [{ type: 'line_track', lineNumber: ' 6340 ', lineName: 'Halle–Könnern', side: 'sorting', heightEpsg: '5783', trackUse: 'siding' },
    { type: 'station_track', stationName: ' Halle ', uicStation: '', trackNumber: '3', heightEpsg: '' }, { type: 'line_track', side: 'x' }]) {
    r('buildTypeFields', T.buildTypeFields, f)
  }
  for (const n of [8, 12, 345]) { r('switchDesignation', T.switchDesignation, n, 'turnout'); r('switchDesignation', T.switchDesignation, n, 'crossing') }
  r('switchNumberOf', T.switchNumberOf, { name: 'crossing.017' }); r('switchNumberOf', T.switchNumberOf, { number: 4, name: 'switch.009' })
  r('switchNumbersInUse', (s) => [...T.switchNumbersInUse(s)], [...switches, { name: 'switch.003' }])
  r('nextSwitchNumber', T.nextSwitchNumber, [...switches, { name: 'switch.003' }], [2])
  r('heightDatumLabel', T.heightDatumLabel, 5783); r('heightDatumLabel', T.heightDatumLabel, 4979); r('heightDatumLabel', T.heightDatumLabel)

  const hs = ahead.heights ?? behind.heights ?? line.heights
  const H = line.heights
  r('heightStations', T.heightStations, 150); r('heightStations', T.heightStations, 640)
  for (const s of [0, 120, 250, 300, 460, 700]) { r('heightAt', T.heightAt, H, s); r('gradientAt', T.gradientAt, H, s) }
  for (let i = 0; i < H.length; i++) { r('pointGrades', T.pointGrades, H, i); r('tangentLength', tangentLength, H, i) }
  r('roundHeight', T.roundHeight, 101.234567)
  for (const given of [{ s: 300, z: 102 }, { s: 300, gb: 0.01 }, { z: 102, ga: -0.004 }, { gb: 0.012, ga: -0.009 }, { gb: 0.01, ga: 0.01 }, {}]) {
    r('solveHeightPoint', T.solveHeightPoint, H, 2, given, { length: 660 })
  }
  r('insertHeightPoint', T.insertHeightPoint, H, 333.3)
  for (const sj of [-5, 0, 100, 240, 250, 450.3, 660, 700]) r('splitHeights', T.splitHeights, H, sj)
  r('truncateHeights', T.truncateHeights, H, 300); r('sliceHeights', T.sliceHeights, H, 100, 500); r('reverseHeights', T.reverseHeights, H, 660)
  r('verticalCurves', T.verticalCurves, H, 8); r('verticalCurveOverlaps', T.verticalCurveOverlaps, [...H.slice(0, 2), { station: 290, z: 102, rv: 20000 }, ...H.slice(2)])
  r('neighbourStub', T.neighbourStub, { track: line, endpoint: 'BEGIN' }); r('neighbourStub', T.neighbourStub, { track: line, endpoint: 'END' })
  for (const t of [behind, ahead, branch]) for (let i = 0; i < (t.heights?.length ?? 0); i++) r('endOfIndex', T.endOfIndex, t, i)
  r('jointGroup', T.jointGroup, tracks, switches, { trackId: behind.id, index: behind.heights.length - 1 })
  r('jointHeightUpdates', T.jointHeightUpdates, tracks, switches, [{ trackId: behind.id, index: behind.heights.length - 1, z: 100.5, rv: null, locked: true }])

  for (const p of platforms) {
    r('edgeOffsets', T.edgeOffsets, p); r('platformRing', T.platformRing, p, byId[p.trackId]); r('platformLength', T.platformLength, p)
    r('platformEdgeElevation', T.platformEdgeElevation, p, byId[p.trackId], 50)
    r('rebuildPlatformSymbol', T.rebuildPlatformSymbol, p, byId)
    r('platformForm', T.platformForm, p)
  }
  r('carryPlatforms', T.carryPlatforms, platforms, [{ oldId: ahead.id, newId: 'z', from: 0, to: 200, offset: 10 }, { oldId: 'branch', newId: 'zb', from: 0, to: 107, offset: 107, flip: true }])
  r('flipPlatforms', T.flipPlatforms, platforms, 'branch', 107)
  r('stationFromClick', T.stationFromClick, ahead, 2, { easting: 600200, northing: 5600050, zone: EPSG })
  r('pickedStation', T.pickedStation, 12.34567)
  const form = T.platformForm(platforms[0])
  r('platformDraft', T.platformDraft, form)
  r('platformFormValid', T.platformFormValid, form, ahead); r('platformFormValid', T.platformFormValid, { ...form, end: '9999' }, ahead)

  const route = { id: 'r', name: 'Route 1', trackIds: [behind.id, ahead.id, 'next'] }
  r('resolveRoute', T.resolveRoute, route, tracks, switches)
  r('resolveRoute', T.resolveRoute, { trackIds: ['next', ahead.id, 'gone', 'branch'] }, tracks, switches)
  const resolved = T.resolveRoute(route, tracks, switches)
  for (const s of [0, 80, 300, 9999]) { r('routeAt', T.routeAt, resolved, s); r('routePointAt', T.routePointAt, resolved, s) }
  r('routeStationOf', T.routeStationOf, resolved, ahead.id, 50)
  r('routePath', routePath, tracks, switches, behind.id, ['END'], 'next')
  r('extendRoute', T.extendRoute, [behind.id], 'next', tracks, switches)
  r('closeGaps', T.closeGaps, [behind.id, 'next'], tracks, switches)
  r('piecesAlong', piecesAlong, line, [behind, ahead])
  r('repairRoutes', T.repairRoutes, { tracks: [line, branch], routes: [{ id: 'q', trackIds: ['line'] }] }, { tracks, switches, routes: [{ id: 'q', trackIds: ['line'] }] })
  r('nextRouteName', T.nextRouteName, [route, { name: 'Route 2' }])

  r('kmTrackName', T.kmTrackName, 6340, 1234.5, ['6340.00123']); r('kmTrackName', T.kmTrackName, 6340, -46)
  r('groupTracks', T.groupTracks, [...tracks, { id: 'zz', name: 'Gleis 10', stationName: 'Halle' }, { id: 'zy', name: 'Gleis 9', stationName: 'halle' }])
  for (const g of T.groupTracks(tracks)) r('groupTitle', T.groupTitle, g)
  for (const target of [{ kind: 'line', lineNumber: ' 6345 ', lineName: 'X' }, { kind: 'station', stationName: 'Halle (Saale) Hbf', uicStation: '' }, { kind: null }]) {
    r('assignmentFields', T.assignmentFields, target)
    r('assignTracks', (ts, ids, t, o) => T.assignTracks(ts, ids, t, { names: new Map(Object.entries(o.names)), trackNumbers: o.trackNumbers }), tracks, ['near', 'cx'], target, { names: { near: 'renamed' }, trackNumbers: { cx: ' 7 ', near: 'a' } })
    r('plannedNames', T.plannedNames, tracks, ['near', 'cx', 'cy'], target, { kmById: { near: 1203, cx: 999 }, numberById: { near: '1', cx: '1' } })
  }
  for (const n of ['Halle (Saale) Hbf', 'Köln-Ehrenfeld [Gbf]', 'Ærø', '', null]) r('stationTrackStem', T.stationTrackStem, n)

  // A kilometrage line along the line's first track, km rising with the station.
  const runs = [behind.elements.concat(ahead.elements).flatMap((el, i) => [el.startNode, el.endNode]).map((n, i) => [...utmToWgs84(n[0], n[1], EPSG), 12000 + i * 50])]
  const kmLine = { lineNumber: 6340, runs }
  r('planeRuns', T.planeRuns, kmLine, EPSG)
  r('kmOnRuns', T.kmOnRuns, T.planeRuns(kmLine, EPSG), [600100, 5600030])
  r('kmForTrackPoint', T.kmForTrackPoint, [kmLine, { lineNumber: 1, runs }], ahead, [600150, 5600010])

  const cut = endPointStraightUtm({ easting: toe.easting, northing: toe.northing, zone: EPSG }, 80, 27)
  r('carveSwitchRoute', T.carveSwitchRoute, ahead, 'BEGIN', cut, T.switchElementMark(sw, 'main'), 27)
  r('carveSwitchRoute', T.carveSwitchRoute, ahead, 'BEGIN', cut, T.switchElementMark(sw, 'main'))
  void rand
  return { track: calls }
}
