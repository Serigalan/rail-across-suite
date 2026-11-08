// The commands crate's vectors (crates/commands): what the dialogs commit,
// called on tracks laid like the switch crate's — a line of straight,
// transitions and an arc, turnouts laid into tracks, a crossover.
import * as T from '../src/commands/tracks.js'
import * as Ro from '../src/commands/elementRoles.js'
import * as Tv from '../src/commands/tableView.js'
import * as Eg from '../src/commands/editGeometry.js'
import { endPointStraightUtm } from '../src/geometry/index.js'
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
  return { commands: calls }
}

function withRolesTrack(track, roles) {
  return { ...track, elements: track.elements.map((el, i) => (roles[i] ? { ...el, role: roles[i] } : el)) }
}
