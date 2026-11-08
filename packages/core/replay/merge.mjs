// The merge's vectors (crates/commands, merge/): three states of a small
// project — a split on one side, moves and edits on the other, deletions
// against changes, km lines added on both, a signalling domain whose
// locations follow a split — merged and resolved.
import * as T from '../src/track/index.js'
import { diffProject, diffEntries, diffSize, primary, mergeProject, resolve, validationConflicts, carryAfterWrite } from '../src/merge/index.js'
import { remapKind } from '../src/merge/remap.js'
import { endPointStraightUtm } from '../src/geometry/index.js'
import { trackOf, layTurnout } from './switch.mjs'
import { rec } from './record.mjs'

const EPSG = 25832
const A = { easting: 600000, northing: 5600000, zone: EPSG }
const plain = (v) => JSON.parse(JSON.stringify(v))

export default function record() {
  const calls = []
  const r = (op, fn, ...args) => calls.push(rec(`merge.${op}`, fn, ...args))
  const line = { ...trackOf('L', A, 90, [['s', 300], ['a', 100, -900], ['s', 200]]), lineNumber: 6340, trackNumber: 1,
    heights: [{ station: 0, z: 100 }, { station: 600, z: 103 }] }
  const other = trackOf('O', { easting: 600000, northing: 5600010 }, 90, [['s', 400]])
  const { sw, tracks: laid } = layTurnout('w', '300 – 1:9', { easting: 601000, northing: 5600000, zone: EPSG }, 90, 'left')
  const signalling = { fields: {}, collections: {
    signals: [{ id: 's1', at: { anchor: 'station', trackId: 'L', station: 150, side: 'right', offset: 3, direction: 'up' } },
      { id: 's2', at: { anchor: 'station', trackId: 'L', station: 450, side: 'left', direction: 'down' } }],
    sections: [{ id: 'z1', range: { anchor: 'station', trackId: 'L', startStation: 20, endStation: 120, side: 'left' }, path: { trackIds: ['L'] } }],
  } }
  const base = {
    id: 'p', name: 'Projekt', tracks: [line, other, ...laid], switches: [sw],
    platforms: [{ id: 'pf1', trackId: 'L', startStation: 30, endStation: 170, side: 'right', stationName: 'Trotha', code: '1' }],
    endMarks: [{ id: 'm1', kind: 'buffer_stop', trackId: 'O', endpoint: 'END' }],
    kmLines: [{ lineNumber: 6340, points: [] }],
    routes: [{ id: 'r1', name: 'Route 1', trackIds: ['L'] }],
    signalling,
  }
  // Mine: L split at 200 m, the platform renamed, a project field changed.
  const p200 = endPointStraightUtm(A, 90, 200)
  const split = T.splitElementAt(line, 0, p200, 90, new Set(['L']))
  const remapsMine = [{ from: 'L', to: split.tracks.map(t => t.id) }]
  const mine = {
    ...base, name: 'Projekt (mein)',
    tracks: [...split.tracks, other, ...laid],
    platforms: [{ ...base.platforms[0], stationName: 'Halle-Trotha' }],
    kmLines: [...base.kmLines, { lineNumber: 6341, points: [1] }],
  }
  // Theirs: the platform moved, the other track's speed changed, the end mark gone, a km line added under mine's number.
  const theirs = {
    ...base, name: 'Projekt (ihr)', description: 'neu',
    tracks: [line, { ...other, elements: other.elements.map(el => ({ ...el, speed: 60 })) }, ...laid],
    platforms: [{ ...base.platforms[0], startStation: 40, endStation: 180 }, { id: 'pf2', trackId: 'O', startStation: 10, endStation: 90, side: 'left' }],
    endMarks: [],
    kmLines: [...base.kmLines, { lineNumber: 6341, points: [2] }],
    switches: [{ ...sw, name: 'W 99' }],
  }
  r('primary', primary, base)
  r('primary', primary, null)
  r('diffProject', (a, b) => plain(diffProject(a, b)), base, mine)
  r('diffProject', (a, b) => plain(diffProject(a, b)), base, theirs)
  const d = diffProject(base, theirs)
  r('diffSize', diffSize, plain(d))
  r('diffEntries', diffEntries, plain(d))
  for (const [m, t, rm, rt] of [[mine, theirs, remapsMine, []], [theirs, mine, [], remapsMine], [mine, base, remapsMine, []], [base, base, [], []]]) {
    r('mergeProject', (o) => plain(mergeProject(o)), { base, mine: m, theirs: t, remapsMine: rm, remapsTheirs: rt })
    const res = mergeProject({ base, mine: m, theirs: t, remapsMine: rm, remapsTheirs: rt })
    const choices = Object.fromEntries(res.conflicts.map((c, i) => [c.id, i % 2 ? 'theirs' : 'mine']))
    r('resolve', (mr, ch) => plain(resolve(mr, ch)), plain(res), choices)
    r('resolve', (mr, ch) => plain(resolve(mr, ch)), plain(res), {})
    r('validationConflicts', (rc, ctx) => plain(validationConflicts(rc, ctx)), plain(res.merged), plain(res.context))
  }
  // Deleted against changed, changed against deleted.
  const delMine = { ...base, tracks: base.tracks.filter(t => t.id !== 'O'), endMarks: [] }
  const chTheirs = { ...base, tracks: base.tracks.map(t => (t.id === 'O' ? { ...t, name: 'Gleis O' } : t)) }
  r('mergeProject', (o) => plain(mergeProject(o)), { base, mine: delMine, theirs: chTheirs })
  r('mergeProject', (o) => plain(mergeProject(o)), { base, mine: chTheirs, theirs: delMine })
  // A broken merge: mine moves a platform beyond its track, theirs shortens the track.
  const broken = { ...base, platforms: [{ ...base.platforms[0], endStation: 590 }] }
  const shortened = { ...base, tracks: base.tracks.map(t => (t.id === 'L' ? { ...t, elements: t.elements.slice(0, 2), heights: [{ station: 0, z: 100 }, { station: 400, z: 102 }] } : t)) }
  r('mergeProject', (o) => plain(mergeProject(o)), { base, mine: broken, theirs: shortened })
  const brokenRes = mergeProject({ base, mine: broken, theirs: shortened })
  const objectChoices = Object.fromEntries(brokenRes.conflicts.filter(c => c.kind === 'validation').map(c => [c.id, 'theirs']))
  r('resolve', (mr, ch) => plain(resolve(mr, ch)), plain(brokenRes), objectChoices)
  for (const [id, log] of [['L', remapsMine], ['L', [{ from: 'L', to: ['x'] }]], ['O', remapsMine], ['L', [{ from: 'L', to: ['L'] }]]]) r('remapKind', remapKind, id, log)
  // After one write: the signalling follows the split.
  r('carryAfterWrite', (b, a) => plain(carryAfterWrite(b, a)), base, { ...base, tracks: [...split.tracks, other, ...laid] })
  r('carryAfterWrite', (b, a) => plain(carryAfterWrite(b, a)), base, { ...base, tracks: [other, ...laid] })
  r('carryAfterWrite', (b, a) => plain(carryAfterWrite(b, a)), base, base)
  return { commands: { merge: calls } }
}
