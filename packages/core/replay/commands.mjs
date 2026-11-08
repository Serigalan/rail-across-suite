// The commands crate's vectors (crates/commands): what the dialogs commit,
// called on tracks laid like the switch crate's — a line of straight,
// transitions and an arc, turnouts laid into tracks, a crossover.
import * as T from '../src/commands/tracks.js'
import * as Ro from '../src/commands/elementRoles.js'
import * as Tv from '../src/commands/tableView.js'
import * as Eg from '../src/commands/editGeometry.js'
import { endPointStraightUtm } from '../src/geometry/index.js'
import * as Sc from '../src/commands/sCurve.js'
import * as Sw from '../src/commands/switches.js'
import { switchStraightLength, crossingEndDistance, CROSSING_TYPES } from '../src/switch/index.js'
import { switchTypeByLabel } from '../src/switch/index.js'
import { trackOf, layTurnout } from './switch.mjs'
import { rec } from './record.mjs'

const EPSG = 25832
const A = { easting: 600000, northing: 5600000, zone: EPSG }

export default function record() {
  const calls = []
  const r = (op, fn, ...args) => calls.push(rec(`commands.${op}`, fn, ...args))
  const line = trackOf('L', A, 90, [['s', 150], ['t', 60, null, -1200, 40], ['a', 120, -1200, null, 40], ['t', 60, -1200, null, 40], ['s', 200]])
  line.heights = [{ station: 0, z: 100 }, { station: 200, z: 101, rv: 5000 }, { station: 590, z: 100.5 }]
  const other = trackOf('O', { easting: line.elements[4].endNode[0], northing: line.elements[4].endNode[1] }, line.elements[4].bearing, [['s', 50], ['a', 40, 500]])

  // tracks
  r('trackMeta', T.trackMeta, { owner: 'x', trackType: 1, lineNumber: 6340, trackNumber: 2 }, 'Gleis 2')
  r('trackOf', T.trackOf, line.elements.slice(0, 2), EPSG, { name: 'n' }, 'id1')
  r('trackOf', T.trackOf, line.elements.slice(0, 1), EPSG)
  const B = endPointStraightUtm(A, 33, 120)
  r('buildLineTrack', T.buildLineTrack, { start: A, end: B, speed: 80, meta: { name: 'l' }, id: 'l1' })
  for (const [rr, c] of [[500, 40], [-500, 40], [-500, '30'], [300, null]]) {
    r('curveCant', T.curveCant, rr, c)
    r('buildCurvedLineTrack', T.buildCurvedLineTrack, { start: A, end: B, signedR: rr, speed: 60, cant: c, meta: {}, id: `c${rr}` })
  }
  r('buildParallelLineTrack', T.buildParallelLineTrack, { start: A, end: B, signedR: 800, speed: 60, meta: {}, id: 'p1' })
  r('buildParallelLineTrack', T.buildParallelLineTrack, { start: A, end: B, speed: 60, meta: {}, id: 'p2' })
  r('buildParallelTrack', T.buildParallelTrack, { elements: line.elements, epsg: EPSG, meta: { name: 'par' }, id: 'pt' })
  const end = { easting: line.elements[4].endNode[0], northing: line.elements[4].endNode[1], zone: EPSG }
  for (const transition of [null, { length: 40, type: 'clothoid', fromRadius: 900 }, { length: 0, fromRadius: 900 }, { length: 30, type: 'bloss', fromRadius: -600 }]) {
    r('buildConnectStraight', T.buildConnectStraight, { start: end, bearing: 90, length: 75, speed: 100, transition })
    r('buildConnectCurved', T.buildConnectCurved, { start: end, bearing: 90, arcLength: 55, signedR: -700, speed: 100, cant: 60, transition })
  }
  for (const el of [null, line.elements[1], line.elements[2], { ...line.elements[1], cantEnd: undefined }, { elementType: 0 }]) r('cantAtTrackEnd', T.cantAtTrackEnd, el)
  const { sw, tracks: laid } = layTurnout('w', '300 – 1:9', A, 33.3, 'left')
  const project = { tracks: [...laid, line], switches: [sw], platforms: [{ id: 'p', trackId: laid[1].id, startStation: 0, endStation: 20, side: 'left' }] }
  r('translateTracks', T.translateTracks, project, [laid[1].id], 1.5, -2.25)
  r('translateTracks', T.translateTracks, project, ['L'], 0, 0)
  r('translateTracks', T.translateTracks, project, ['nope'], 1, 1)

  // element roles
  const withRoles = (roles) => line.elements.map((el, i) => (roles[i] ? { ...el, role: roles[i] } : el))
  for (const roles of [[], [null, null, 'intermediate'], [null, null, null, 'follow'], ['intermediate'], [null, null, 'intermediate', null, 'intermediate'], [null, null, 'intermediate', 'follow']]) {
    const els = withRoles(roles)
    r('elementRoles', Ro.elementRoles, els)
    for (let i = 0; i < els.length; i++) r('intermediateSpan', Ro.intermediateSpan, els, i)
  }
  r('elementRoles', Ro.elementRoles, [...laid[1].elements, ...line.elements].map((el, i) => (i === 3 ? { ...el, role: 'follow' } : el)))
  for (const el of [line.elements[0], line.elements[1], laid[1].elements[0]]) r('rolesFor', Ro.rolesFor, el)
  for (const [i, role] of [[2, 'intermediate'], [3, 'follow'], [2, 'fixed'], [1, 'intermediate'], [9, 'follow']]) r('withRole', Ro.withRole, withRoles([null, null, 'intermediate', null, 'follow']), i, role)

  // table view
  const route = { id: 'r', trackIds: ['L', 'O'] }
  const reversedLine = { ...line, id: 'LR', elements: [...line.elements].reverse() }
  r('tableParts', (o, t, s) => Tv.tableParts(o, t, s), { trackId: 'L' }, [line, other], [])
  r('tableParts', (o, t, s) => Tv.tableParts(o, t, s), { route }, [line, other], [])
  r('tableParts', (o, t, s) => Tv.tableParts(o, t, s), {}, [line], [])
  const parts = Tv.tableParts({ route }, [line, other], []).parts
  const turned = [{ trackId: 'O', reversed: true }, { trackId: 'L', reversed: true }]
  for (const ps of [parts, turned]) {
    r('tableRows', Tv.tableRows, ps, [line, other])
    const { rows, elements } = Tv.tableRows(ps, [line, other])
    r('tableRoles', Tv.tableRoles, rows, [withRolesTrack(line, [null, null, 'intermediate']), other])
    r('rowOf', Tv.rowOf, rows, 'L', 3); r('rowOf', Tv.rowOf, rows, 'X', 0)
    for (const key of ['radius', 'cant', 'cantStart', 'cantEnd', 'speed']) r('trackEdit', Tv.trackEdit, rows[1], key, 12.5)
    r('writeRows', Tv.writeRows, [line, other], rows, elements.map((el, i) => (i === 2 ? { ...el, speed: 60, cant: (el.cant ?? 0) + 5 } : el)))
    r('inRunElementChange', (row, t, s, ch) => Tv.inRun(row, t, s, (tt, ss, i) => Eg.planElementChange(tt, ss, row.trackId, i, ch)), rows[2], [line, other], [], { length: 130 })
  }
  r('trackEdit', Tv.trackEdit, { reversed: true }, 'radius', null)

  // geometry edits
  const tracks = [line, other, ...laid]
  for (const [id, i, change] of [['L', 2, { length: 140 }], ['L', 2, { radius: -1000 }], ['L', 0, { bearing: 91 }], ['L', 4, { length: 210 }], ['L', 1, { length: 55 }], ['L', 2, { radius: null }],
    [laid[1].id, 0, { length: 40 }], [laid[0].id, 0, { length: 45 }], ['L', 99, { length: 1 }], ['nope', 0, {}]]) {
    r('planElementChange', Eg.planElementChange, tracks, [sw], id, i, change)
  }
  r('planElementChange', Eg.planElementChange, tracks, [], 'L', 0, { length: 160 })
  for (const [z, radius] of [[2, -1500], [2, -1000], [2, null], [0, -900], [2, 0], [2, -200]]) r('planIntermediateChange', Eg.planIntermediateChange, tracks, 'L', z, radius)
  const edited = Eg.planElementChange(tracks, [], 'L', 2, { length: 140 }).tracks
  r('mergeElementEdits', Eg.mergeElementEdits, tracks, edited, { changed: ['L', 'O'], reshaped: ['L'] })
  r('mergeElementEdits', Eg.mergeElementEdits, tracks, edited, {})
  recordSCurve(r, line)
  recordSwitches(r, line)
  return { commands: calls }
}

function recordSCurve(r, line) {
  const plain = (v) => JSON.parse(JSON.stringify(v))
  const t1 = trackOf('P1', A, 90, [['s', 120], ['s', 280]])
  const t2 = trackOf('P2', { easting: 600000, northing: 5599995.5 }, 90, [['s', 400]])
  const t3 = trackOf('P3', { easting: 600000, northing: 5599993 }, 90.4, [['s', 400]])
  const c1 = trackOf('C1', A, 90, [['s', 60], ['t', 50, null, -2000, 30], ['a', 300, -2000, null, 30]])
  const c2 = trackOf('C2', { easting: 600000, northing: 5599995 }, 90, [['s', 60], ['t', 50, null, -2004.5, 30], ['a', 300, -2004.5, null, 30]])
  const back = trackOf('PB', { easting: 600400, northing: 5599995.5 }, 270, [['s', 400]])
  for (const [t, i, a] of [[t1, 0, 30], [t1, 1, 100], [line, 2, 40], [line, 1, 10], [c1, 2, 40]]) r('connectionPick', Sc.connectionPick, t, i, a)
  for (const el of line.elements) { r('isUsableStem', Sc.isUsableStem, el) }
  r('isUsableStem', Sc.isUsableStem, { ...line.elements[1], transitionType: 'bloss' })
  for (let i = 0; i < line.elements.length; i++) for (const side of ['before', 'after']) r('runBeside', (e, k, sd) => { const x = runBesideOf(e, k, sd); return x }, line.elements, i, side)
  r('REASON_MSG', () => Sc.REASON_MSG)
  const cases = [
    [[t1, 0, 60], [t2, 0, 150], 60],
    [[t1, 1, 100], [t3, 0, 230], 80],
    [[c1, 2, 40], [c2, 2, 70], 60],
    [[t2, 0, 250], [t1, 1, 80], 40],
    [[t1, 0, 60], [back, 0, 250], 60],
  ]
  for (const [[ta, ia, aa], [tb, ib, ab], speed] of cases) {
    const picks = [Sc.connectionPick(ta, ia, aa), Sc.connectionPick(tb, ib, ab)]
    r('stems', (p) => plain(Sc.stems(p)), picks)
    r('waOffset', Sc.waOffset, picks)
    for (const shift of [0, 12.5, -30]) {
      r('solveConnection', (o) => plain(Sc.solveConnection(o)), { picks, speed, shift })
      r('settleConnection', (o) => plain(Sc.settleConnection(o)), { picks, speed, shift })
      const res = Sc.solveConnection({ picks, speed, shift })
      if (res) {
        r('connectionRemnants', Sc.connectionRemnants, picks, shift, plain(res), speed)
        r('hasStraightMid', Sc.hasStraightMid, plain(res))
      }
    }
    const bounds = Sc.computeShiftBounds({ picks, speed })
    r('computeShiftBounds', (o) => Sc.computeShiftBounds(o), { picks, speed })
    r('computeShiftBounds', (o) => Sc.computeShiftBounds(o), { picks, speed: 100, lenient: true })
    r('straightShifts', (o, b) => Sc.straightShifts(o, b), { picks, speed }, bounds)
    r('shiftSnaps', (o, b) => Sc.shiftSnaps(o, b), { picks, speed }, bounds)
    r('preferredShift', (o, b) => Sc.preferredShift(o, b), { picks, speed }, bounds)
    const shift = Sc.preferredShift({ picks, speed }, bounds)
    const settled = Sc.settleConnection({ picks, speed, shift })
    const tracks = [ta, tb, line].filter((t, k, all) => all.indexOf(t) === k)
    r('buildSCurve', (o) => plain(Sc.buildSCurve(o)), { result: plain(settled.result), picks, tracks, switches: [], speed })
    r('buildSCurve', (o) => plain(Sc.buildSCurve(o)), { result: plain(settled.result), picks, tracks, switches: [{ switchId: 'x', number: 1, name: 'switch.001', ports: {} }], speed,
      identities: [{ switchId: 'old1', name: 'W 1', number: 7 }, { switchId: 'old2', name: 'W 2', number: 8 }], connTrack: { id: 'oldConn', name: 'Verbindung', heights: [], note: 'x' } })
  }
  const mids = [{ type: 'arc', length: 25, radius: null }, { type: 'straight', length: null }]
  const picks = [Sc.connectionPick(t1, 0, 60), Sc.connectionPick(t2, 0, 150)]
  r('solveConnection', (o) => plain(Sc.solveConnection(o)), { picks, speed: 60, shift: 0, mids })
  r('solveConnection', (o) => plain(Sc.solveConnection(o)), { picks, speed: 60, shift: 0, form: switchTypeByLabel('500 – 1:12') })
  r('solveConnection', (o) => plain(Sc.solveConnection(o)), { picks: [picks[0]], speed: 60, shift: 0 })
}

import { runBeside as runBesideOf } from '../src/rules/index.js'

function recordSwitches(r, line) {
  const plain = (v) => JSON.parse(JSON.stringify(v))
  const t1 = trackOf('P1', A, 90, [['s', 120], ['s', 280]])
  const c1 = trackOf('C1', A, 90, [['s', 60], ['t', 50, null, -2000, 30], ['a', 300, -2000, null, 30]])
  r('SLIDER_REACH', () => Sw.SLIDER_REACH)
  const forms = ['300 – 1:9', '500 – 1:12', '190 – 1:9'].map(switchTypeByLabel)
  for (const [track, toe, reversed, side, sw] of [[t1, 50, false, 'left', forms[0]], [t1, 120, true, 'right', forms[1]], [line, 170, false, 'left', forms[0]], [line, 260, false, 'right', forms[0]],
    [line, 380, true, 'left', forms[1]], [c1, 150, false, 'right', forms[2]], [line, 0.2, false, 'left', forms[0]], [line, 9999, false, 'left', forms[0]], [line, 585, false, 'left', forms[0]]]) {
    const straightLen = switchStraightLength(sw)
    const args = { track, reversed, sw, side, straightLen, speed: 60 }
    r('switchOnTrackPlacement', (o) => plain(Sw.switchOnTrackPlacement(o)), { ...args, toeStation: toe })
    r('switchOnTrackRouteEnd', (o, x) => plain(Sw.switchOnTrackRouteEnd(o, x)), args, toe)
    r('switchOnTrackRemnant', Sw.switchOnTrackRemnant, track, toe, reversed, 60)
    r('switchOnTrackToeValid', (o, x) => Sw.switchOnTrackToeValid(o, x), args, toe)
    const p = Sw.switchOnTrackPlacement({ ...args, toeStation: toe })
    if (p.geom) {
      const o = { track, tracks: [track, line], g: p.geom, plain: p.plain, sw, straightLen, cant: 20, cantReason: 'Bestand', speed: 60, switchName: 'switch.009', switchNumber: 9, name: 'Zweig', fields: { owner: 'DB', trackType: 2 } }
      r('buildSwitchOnTrack', (oo) => plain(Sw.buildSwitchOnTrack({ ...oo, place: Sw.switchOnTrackPlacement({ track: oo.track, toeStation: oo.toeStation, reversed: oo.reversed, sw: oo.sw, side, straightLen }).place })), { ...o, toeStation: toe, reversed })
    }
  }
  r('switchOnTrackSlider', (o, a) => plain(Sw.switchOnTrackSlider(o, a)), { track: line, reversed: false, sw: forms[0], side: 'left', straightLen: switchStraightLength(forms[0]), speed: 60 }, 180)
  r('switchOnTrackSlider', (o, a) => plain(Sw.switchOnTrackSlider(o, a)), { track: t1, reversed: true, sw: forms[1], side: 'right', straightLen: switchStraightLength(forms[1]), speed: 100 }, null)
  r('switchOnTrackSlider', (o, a) => plain(Sw.switchOnTrackSlider(o, a)), { track: { elements: [] }, reversed: false, sw: forms[0], side: 'left', straightLen: 30 }, null)
  const end = { easting: line.elements[4].endNode[0], northing: line.elements[4].endNode[1], zone: EPSG }
  for (const [sw, side, trailing, stemSigned] of [[forms[0], 'left', false, null], [forms[0], 'right', true, null], [forms[1], 'left', false, 1100], [forms[0], 'right', false, -300], [forms[0], 'right', true, -300], [forms[2], 'left', true, 900]]) {
    r('buildSwitchAtTrackEnd', (o) => plain(Sw.buildSwitchAtTrackEnd(o)), { start: end, bearing: 90, startWgs: null, sourceTrackId: 'L', sw, side, trailing, stemSigned, cant: 40, cantReason: 'x',
      speed: 80, switchName: 'switch.003', switchNumber: 3, name: 'b', fields: { owner: 'o' }, mainName: 'm', mainFields: { trackType: 1 } })
  }
  for (const form of CROSSING_TYPES.slice(0, 6)) {
    const endDist = crossingEndDistance(form)
    const crossAngle = Math.atan(1 / form.ratio) * 180 / Math.PI
    for (const [track, point] of [[t1, 100], [line, 180], [t1, 5]]) {
      const ask = { track, endDist, form, crossAngle }
      r('crossingOnTrackPlacement', (o) => plain(Sw.crossingOnTrackPlacement(o)), { ...ask, pointStation: point })
      for (const port of ['B', 'D']) r('crossingOnTrackPort', (o, pp) => plain(Sw.crossingOnTrackPort(o, pp)), { ...ask, pointStation: point }, port)
      const pl = Sw.crossingOnTrackPlacement({ ...ask, pointStation: point })
      if (pl.geom) {
        r('buildCrossingOnTrack', (o) => plain(Sw.buildCrossingOnTrack({ ...o, ahead: Sw.crossingOnTrackPlacement({ track: o.track, pointStation: o.pointStation, endDist: o.endDist, form: o.form, crossAngle }).ahead })),
          { g: pl.geom, track, tracks: [track], form, endDist, switchName: 'crossing.004', switchNumber: 4, name: 'Kreuz', fields: {}, pointStation: point })
      }
    }
    r('crossingOnTrackSlider', (o, a) => plain(Sw.crossingOnTrackSlider(o, a)), { track: t1, endDist, form, crossAngle }, 100)
  }
}

function withRolesTrack(track, roles) {
  return { ...track, elements: track.elements.map((el, i) => (roles[i] ? { ...el, role: roles[i] } : el)) }
}
