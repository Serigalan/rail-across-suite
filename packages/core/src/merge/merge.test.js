import { describe, expect, it } from 'vitest'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { hasPek, loadPek } from '../../test/pekFixture.js'
import { splitTrackAtJoint } from '../track/index.js'
import { joinTracks } from '../track/index.js'
import { diffProject, diffSize, mergeProject, resolve } from './index.js'
import { primary } from './index.js'
import { portTrack, portEnd, withPort } from '../track/index.js'
import { straightElement } from '../geometry/elementFactory.js'
import { rebuildCoords } from '../track/index.js'

const SWITCH_1 = '00000000-0000-4000-8000-000000000001'
const A = 'a9f56232-878c-4eef-ab79-09131c7c1212'     // 6050.001, switch.001's toe at its END

const trackOf = (p, id) => p.tracks.find(t => t.id === id)
const switchOf = (p, id) => p.switches.find(s => s.switchId === id)
const len = (t) => t.elements.reduce((s, e) => s + e.length, 0)

/** Split track `id` of `p` at joint `j`, the way storage does it: pieces in, ports and marks repointed. */
function split(p, id, j) {
  const old = trackOf(p, id)
  const { tracks: [b, c] } = splitTrackAtJoint(old, j, old.elements[j].bearing, new Set())
  p.tracks = p.tracks.flatMap(t => (t.id === id ? [primary({ tracks: [b] }).tracks[0], primary({ tracks: [c] }).tracks[0]] : [t]))
  p.switches = p.switches.map(sw => {
    let out = sw
    for (const k of ['A', 'B1', 'B2']) {
      if (portTrack(sw, k) !== id) continue
      out = withPort(out, k, { trackId: portEnd(sw, k) === 'BEGIN' ? b.id : c.id })
    }
    return out
  })
  return { b: trackOf(p, b.id), c: trackOf(p, c.id), log: [{ from: id, to: [b.id, c.id] }] }
}

describe.skipIf(!hasPek)('mergeProject — PEK Halle–Könnern', () => {
  it('a merge without changes gives the starting state', () => {
    const base = loadPek()
    const { merged, conflicts, applied } = mergeProject({ base, mine: loadPek(), theirs: loadPek() })
    expect(conflicts).toEqual([])
    expect(applied).toEqual([])
    expect(diffSize(diffProject(base, merged))).toBe(0)
  })

  it('diffProject names what was added, removed and changed, field by field', () => {
    const base = loadPek()
    const other = loadPek()
    other.title = 'Neu'
    trackOf(other, A).name = 'x'
    other.platforms.pop()
    other.switches[0].ports.A.endpoint = 'BEGIN'
    const d = diffProject(base, other)
    expect(d.project.changed).toEqual(['title'])
    expect(d.tracks.changed).toEqual([expect.objectContaining({ id: A, fields: ['name'] })])
    expect(d.platforms.removed).toHaveLength(1)
    expect(d.switches.changed[0].fields).toEqual(['ports'])
  })

  it('rule 2: different fields of one track are both taken', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    trackOf(mine, A).name = 'meins'
    trackOf(theirs, A).speed = 80
    const { merged, conflicts, applied } = mergeProject({ base, mine, theirs })
    expect(conflicts).toEqual([])
    expect(trackOf(merged, A)).toMatchObject({ name: 'meins', speed: 80 })
    expect(applied).toEqual([expect.objectContaining({ collection: 'tracks', id: A, kind: 'changed', fields: ['speed'] })])
  })

  it('rule 2: the same field changed alike is no conflict, differently it is one', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    trackOf(mine, A).name = 'gleich'
    trackOf(theirs, A).name = 'gleich'
    expect(mergeProject({ base, mine, theirs }).conflicts).toEqual([])

    trackOf(theirs, A).name = 'anders'
    const { conflicts, merged } = mergeProject({ base, mine, theirs })
    expect(conflicts).toEqual([expect.objectContaining({
      collection: 'tracks', objectId: A, field: 'name', kind: 'field',
      base: '6050.001', mine: 'gleich', theirs: 'anders', label: '6050-1',
    })])
    expect(trackOf(merged, A).name).toBe('gleich')
  })

  it('the geometry of both sides is one field: any two edits of it conflict', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    trackOf(mine, A).elements[0].speed = 70
    trackOf(theirs, A).elements[40].speed = 90
    const { conflicts } = mergeProject({ base, mine, theirs })
    expect(conflicts.map(c => c.field)).toEqual(['elements'])
  })

  it('the ports of a switch are merged as one field', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    switchOf(mine, SWITCH_1).ports.A.endpoint = 'BEGIN'
    switchOf(theirs, SWITCH_1).ports.B1.endpoint = 'END'
    const { conflicts } = mergeProject({ base, mine, theirs })
    expect(conflicts.map(c => c.id)).toEqual([`switches:${SWITCH_1}:ports`])
  })

  it('rule 3: deleted against changed is a conflict, deleted against deleted is not', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const pf = base.platforms[0].id
    mine.platforms = mine.platforms.filter(p => p.id !== pf)
    theirs.platforms.find(p => p.id === pf).stationName = 'Trotha'
    const r = mergeProject({ base, mine, theirs })
    expect(r.conflicts).toEqual([expect.objectContaining({ kind: 'deleted_changed', objectId: pf, field: null })])
    expect(r.merged.platforms.find(p => p.id === pf)).toBeUndefined()
    expect(resolve(r, { [r.conflicts[0].id]: 'theirs' }).platforms.find(p => p.id === pf).stationName).toBe('Trotha')

    theirs.platforms = theirs.platforms.filter(p => p.id !== pf)
    const both = mergeProject({ base, mine, theirs })
    expect(both.conflicts).toEqual([])
    expect(both.merged.platforms.find(p => p.id === pf)).toBeUndefined()
  })

  it('an unchanged object the other side deleted goes, and that is listed', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const pf = base.platforms[1].id
    theirs.platforms = theirs.platforms.filter(p => p.id !== pf)
    const { merged, applied } = mergeProject({ base, mine, theirs })
    expect(merged.platforms.find(p => p.id === pf)).toBeUndefined()
    expect(applied).toEqual([expect.objectContaining({ kind: 'removed', id: pf })])
  })

  it('rule 4: a km line new on both sides conflicts where they differ', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    // a km line goes by its id, which the km sources draw from its number
    mine.kmLines = [{ id: 'km-6050', lineNumber: 6050, runs: [[[11.9, 51.5, 0]]] }]
    theirs.kmLines = [{ id: 'km-6050', lineNumber: 6050, runs: [[[11.9, 51.5, 0]]] }]
    expect(mergeProject({ base, mine, theirs }).conflicts).toEqual([])
    theirs.kmLines = [{ id: 'km-6050', lineNumber: 6050, runs: [[[11.9, 51.5, 1]]] }]
    const { conflicts } = mergeProject({ base, mine, theirs })
    expect(conflicts).toEqual([expect.objectContaining({ kind: 'both_added', collection: 'kmLines', objectId: 'km-6050' })])
  })

  it('rule 5: a switch and a platform the other side put on a split track follow it to the right piece', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const { b, c, log } = split(mine, A, 30)
    expect(switchOf(mine, SWITCH_1).ports?.A?.trackId).toBe(c.id)

    const tA = trackOf(theirs, A)
    const L = len(tA), lb = len(b)
    theirs.platforms.push({ id: 'pf-new', trackId: A, startStation: L - 300, endStation: L - 150, side: 'left' })
    theirs.platforms.push({ id: 'pf-front', trackId: A, startStation: 100, endStation: 200, side: 'right' })
    theirs.endMarks = [{ id: 'm1', kind: 'bufferStop', trackId: A, endpoint: 'BEGIN' }]
    switchOf(theirs, SWITCH_1).speed = 40

    const { merged, conflicts, applied } = mergeProject({ base, mine, theirs, remapsMine: log })
    expect(conflicts).toEqual([])
    const pf = merged.platforms.find(p => p.id === 'pf-new')
    expect(pf.trackId).toBe(c.id)
    expect(pf.startStation).toBeCloseTo(L - 300 - lb, 2)
    expect(pf.endStation).toBeCloseTo(L - 150 - lb, 2)
    expect(merged.platforms.find(p => p.id === 'pf-front')).toMatchObject({ trackId: b.id, startStation: 100, endStation: 200 })
    expect(merged.endMarks).toEqual([expect.objectContaining({ trackId: b.id, endpoint: 'BEGIN' })])
    expect(switchOf(merged, SWITCH_1)).toMatchObject({ ports: { A: { trackId: c.id, endpoint: 'END' } }, speed: 40 })
    // The platform the data set already had on A is carried too: the split
    // left it naming A on mine as well.
    const existing = base.platforms.find(p => p.trackId === A).id
    expect(applied.filter(a => a.kind === 'carried').map(a => a.id).sort()).toEqual([existing, 'm1', 'pf-front', 'pf-new'].sort())
  })

  it('rule 5: a switch the other side hung on a split track goes to the piece with that end', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const { b, log } = split(mine, A, 30)
    // Theirs repoints switch.001's toe from A.END to A.BEGIN.
    switchOf(theirs, SWITCH_1).ports.A.endpoint = 'BEGIN'
    // Mine left that switch alone apart from the split's own repoint: carried
    // first, that is no change (data model, R6), and theirs' repoint is taken —
    // onto the piece that holds A's beginning.
    const { conflicts, merged } = mergeProject({ base, mine, theirs, remapsMine: log })
    expect(conflicts).toEqual([])
    expect(switchOf(merged, SWITCH_1).ports.A).toMatchObject({ trackId: b.id, endpoint: 'BEGIN' })
    // A new switch theirs added at A.BEGIN goes to b.BEGIN.
    const t2 = loadPek()
    t2.switches.push({ ...withPort(switchOf(t2, SWITCH_1), 'A', { trackId: A, endpoint: 'BEGIN' }), switchId: 'sw-new' })
    const r = mergeProject({ base, mine, theirs: t2, remapsMine: log })
    expect(switchOf(r.merged, 'sw-new')).toMatchObject({ ports: { A: { trackId: b.id, endpoint: 'BEGIN' } } })
  })

  it('rule 5: a platform across the cut cannot be carried and keeps its old track', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const { b, log } = split(mine, A, 30)
    const lb = len(b)
    theirs.platforms.push({ id: 'pf-cut', trackId: A, startStation: lb - 50, endStation: lb + 50, side: 'left' })
    const { merged } = mergeProject({ base, mine, theirs, remapsMine: log })
    expect(merged.platforms.find(p => p.id === 'pf-cut').trackId).toBe(A)
  })

  it('rule 5: the other side changed the split track itself — deleted against changed, with the hint', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const { log } = split(mine, A, 30)
    trackOf(theirs, A).speed = 100
    const { conflicts } = mergeProject({ base, mine, theirs, remapsMine: log })
    expect(conflicts).toEqual([expect.objectContaining({ kind: 'deleted_changed', objectId: A, hint: 'split' })])
  })

  it('rule 5 the other way round: references on a track that was joined move along it', () => {
    const base = loadPek()
    const { b, c } = split(base, A, 30)
    const mine = structuredClone(base), theirs = structuredClone(base)
    const joined = primary({ tracks: [joinTracks(trackOf(mine, b.id), trackOf(mine, c.id))] }).tracks[0]
    mine.tracks = mine.tracks.filter(t => t.id !== c.id).map(t => (t.id === b.id ? joined : t))
    switchOf(mine, SWITCH_1).ports.A.trackId = b.id
    const log = [{ from: c.id, to: [b.id] }]

    const lb = len(b)
    theirs.platforms.push({ id: 'pf-c', trackId: c.id, startStation: 10, endStation: 60, side: 'left' })
    theirs.endMarks = [{ id: 'm-c', kind: 'boundary', trackId: c.id, endpoint: 'BEGIN' }]
    const { merged } = mergeProject({ base, mine, theirs, remapsMine: log })
    const pf = merged.platforms.find(p => p.id === 'pf-c')
    expect(pf.trackId).toBe(b.id)
    expect(pf.startStation).toBeCloseTo(lb + 10, 2)
    // c.BEGIN is the joint now, no end any more: the mark cannot follow.
    expect(merged.endMarks[0].trackId).toBe(c.id)
  })
})

// Case F2 of the data model: Alice splits track T at 100 m, which re-stations
// platform P (120–260 m) onto the second piece at 20–160 m; Bob, on the old
// state, moves P to 135–275 m on T. Carried first and then compared, Bob's move
// lands on the piece at 35–175 m — no conflict, whichever side merges which.
describe('a platform moved on a track the other side split (data model, case F2)', () => {
  const EPSG = 25832
  const P = (x) => ({ easting: 500000 + x, northing: 5700000, zone: EPSG })
  const track = (id, x0, x1) => {
    const elements = [straightElement(P(x0), P(x1), { speed: 80 })]
    return { id, epsg: EPSG, elements, coordinates: rebuildCoords(elements) }
  }
  const platform = (trackId, startStation, endStation, extra = {}) =>
    ({ id: 'P', trackId, startStation, endStation, side: 'left', height: 550, ...extra })
  const state = (tracks, pl) => primary({ id: 'f2', title: 'F2', tracks, switches: [], platforms: [pl] })

  const base = state([track('T', 0, 400)], platform('T', 120, 260))
  const alice = state([track('T1', 0, 100), track('T2', 100, 400)], platform('T2', 20, 160))
  const aliceLog = [{ from: 'T', to: ['T1', 'T2'] }]
  const bob = state([track('T', 0, 400)], platform('T', 135, 275))

  it('takes Bob\'s move onto Alice\'s piece, Bob merging Alice\'s state into his', () => {
    const { merged, conflicts } = mergeProject({ base, mine: bob, theirs: alice, remapsTheirs: aliceLog })
    expect(conflicts).toEqual([])
    expect(merged.tracks.map(t => t.id).sort()).toEqual(['T1', 'T2'])
    expect(merged.platforms).toEqual([platform('T2', 35, 175)])
  })

  it('does the same the other way round, Alice merging Bob\'s state into hers', () => {
    const { merged, conflicts } = mergeProject({ base, mine: alice, theirs: bob, remapsMine: aliceLog })
    expect(conflicts).toEqual([])
    expect(merged.platforms).toEqual([platform('T2', 35, 175)])
  })

  it('reports nothing taken over where the only change was the carrying', () => {
    const { merged, conflicts, applied } = mergeProject({ base, mine: base, theirs: alice, remapsTheirs: aliceLog })
    expect(conflicts).toEqual([])
    expect(merged.platforms).toEqual([platform('T2', 20, 160)])
    expect(applied.filter(a => a.collection === 'platforms').map(a => a.kind)).toEqual(['carried'])
  })

  it('merges track, stations and side as one value: two different moves are one conflict, never a mix', () => {
    const turned = state([track('T1', 0, 100), track('T2', 100, 400)], platform('T2', 20, 160, { side: 'right' }))
    const { merged, conflicts } = mergeProject({ base, mine: bob, theirs: turned, remapsTheirs: aliceLog })
    expect(conflicts).toEqual([expect.objectContaining({ collection: 'platforms', objectId: 'P', field: 'at' })])
    // Mine stands until decided — carried onto the piece, all of it from one side.
    expect(merged.platforms).toEqual([platform('T2', 35, 175)])
    const taken = resolve(mergeProject({ base, mine: bob, theirs: turned, remapsTheirs: aliceLog }),
      { 'platforms:P:at': 'theirs' })
    expect(taken.platforms).toEqual([platform('T2', 20, 160, { side: 'right' })])
  })
})

describe('the merge module under plain Node', () => {
  it('loads and merges without a DOM, the bundler or a module hook, on the core\'s module', () => {
    const root = fileURLToPath(new URL('../..', import.meta.url))
    const script = `
      const { loadNativeFile } = await import('./src/native/node.js')
      loadNativeFile()
      const { mergeProject } = await import('./src/merge/index.js')
      const p = { id: 'p', title: 'a', tracks: [], switches: [] }
      const r = mergeProject({ base: p, mine: { ...p, title: 'b' }, theirs: p })
      console.log(JSON.stringify([typeof window, r.merged.title, r.conflicts.length]))`
    const out = spawnSync(process.execPath, ['--input-type=module', '-e', script],
      { cwd: root, encoding: 'utf8' })
    expect(out.stderr).toBe('')
    expect(JSON.parse(out.stdout.trim())).toEqual(['undefined', 'b', 0])
  })
})

describe.skipIf(!hasPek)('mergeProject — validation (rules 6 and 7)', () => {
  it('an error the merge brings in is a conflict on the object, decided by taking it from one side', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const victim = '29223a76'
    const id = base.tracks.find(t => t.id.startsWith(victim)).id
    // Mine deletes a short track; theirs puts a platform on it.
    mine.tracks = mine.tracks.filter(t => t.id !== id)
    theirs.platforms.push({ id: 'pf-v', trackId: id, startStation: 1, endStation: 10, side: 'left' })
    const r = mergeProject({ base, mine, theirs })
    const v = r.conflicts.filter(c => c.kind === 'validation')
    // The switches naming the deleted track were already broken in mine: not
    // the merge's doing. The platform is.
    expect(v).toEqual([expect.objectContaining({
      id: 'object:platforms:pf-v', collection: 'platforms',
      findings: [expect.objectContaining({ code: 'platform_track_missing' })],
    })])
    expect(v[0].mine).toBeUndefined()   // mine has no such platform
    const resolved = resolve(r, { 'object:platforms:pf-v': 'mine' })
    expect(resolved.platforms.find(p => p.id === 'pf-v')).toBeUndefined()
  })

  it('warnings the merge brings in are listed: the same track drawn on both sides', () => {
    const base = loadPek(), mine = loadPek(), theirs = loadPek()
    const copy = (p, id) => ({ ...structuredClone(trackOf(p, A)), id })
    mine.tracks.push(copy(mine, 'drawn-by-me'))
    theirs.tracks.push(copy(theirs, 'drawn-by-them'))
    const r = mergeProject({ base, mine, theirs })
    expect(r.conflicts).toEqual([])
    expect(r.warnings.filter(w => w.code === 'tracks_overlap').map(w => [w.id, w.params.otherId].sort()))
      .toContainEqual(['drawn-by-me', 'drawn-by-them'])
  })
})

describe('what the records hold, object by object (decision 0006)', () => {
  const P = (x) => ({ easting: 500000 + x, northing: 5600000, zone: 25832 })
  const pieces = (...xs) => xs.slice(1).map((x, i) => straightElement(P(xs[i]), P(x)))
  const straight = (id, x0, x1) => ({ id, name: id, epsg: 25832, elements: pieces(x0, x1) })
  const base = () => ({
    id: 'p', title: 'P', tracks: [straight('t1', 0, 1000)], switches: [], platforms: [],
    signalling: { fields: { catalogue: { id: 'db-ril-301', version: '0.4.0' } }, collections: {
      signals: [{ id: 's1', name: 'A', kind: 'virtual', virtualFunction: 'trainTargetLine' }, { id: 's2', name: 'B', kind: 'virtual', virtualFunction: 'trainTargetLine' }],
    } },
    bridges: [{ id: 'b1', name: 'Saalebrücke', at: { anchor: 'station', trackId: 't1', startStation: 100, endStation: 300 } }],
  })
  const signal = (p, id) => p.signalling.collections.signals.find(s => s.id === id)
  const withSignal = (p, id, change) => ({ ...p, signalling: { ...p.signalling, collections: { ...p.signalling.collections, signals: p.signalling.collections.signals.map(s => (s.id === id ? { ...s, ...change } : s)) } } })

  it('takes two people\'s changes to different signals both, no longer the signalling as one value', () => {
    const { merged, conflicts, applied } = mergeProject({ base: base(), mine: withSignal(base(), 's1', { name: 'A1' }), theirs: withSignal(base(), 's2', { name: 'B1' }) })
    expect(conflicts).toEqual([])
    expect([signal(merged, 's1').name, signal(merged, 's2').name]).toEqual(['A1', 'B1'])
    expect(applied).toEqual([expect.objectContaining({ collection: 'signalling.signals', id: 's2', kind: 'changed', fields: ['name'] })])
  })

  it('makes one signal changed both ways a conflict on that signal, which a choice decides', () => {
    const result = mergeProject({ base: base(), mine: withSignal(base(), 's1', { name: 'X' }), theirs: withSignal(base(), 's1', { name: 'Y' }) })
    expect(result.conflicts).toEqual([expect.objectContaining({ collection: 'signalling.signals', objectId: 's1', field: 'name', kind: 'field' })])
    expect(signal(resolve(result, { [result.conflicts[0].id]: 'theirs' }), 's1').name).toBe('Y')
    expect(signal(resolve(result, {}), 's1').name).toBe('X')
  })

  it('merges the civil structures object by object, and lists them in the diff', () => {
    const mine = { ...base(), bridges: [...base().bridges, { id: 'b2', name: 'Flutbrücke', at: { anchor: 'station', trackId: 't1', startStation: 500, endStation: 600 } }] }
    const theirs = { ...base(), bridges: [{ ...base().bridges[0], name: 'Saalebrücke Trotha' }] }
    const { merged, conflicts } = mergeProject({ base: base(), mine, theirs })
    expect(conflicts).toEqual([])
    expect(merged.bridges.map(b => [b.id, b.name])).toEqual([['b1', 'Saalebrücke Trotha'], ['b2', 'Flutbrücke']])
    expect(diffProject(base(), mine).bridges.added).toEqual([{ id: 'b2', label: 'Flutbrücke' }])
  })

  it('carries a civil structure onto the piece of a track the other side split', () => {
    const t1 = { ...straight('t1', 0, 1000), elements: pieces(0, 500, 1000) }
    const { tracks: [left, right] } = splitTrackAtJoint(t1, 1, 90, new Set())
    const before = { ...base(), tracks: [t1] }
    const tracks = [primary({ tracks: [left] }).tracks[0], primary({ tracks: [right] }).tracks[0]]
    const { merged } = mergeProject({ base: before, mine: { ...before, title: 'Q' }, theirs: { ...before, tracks }, remapsTheirs: [{ from: 't1', to: [left.id, right.id] }] })
    expect(merged.bridges[0].at).toMatchObject({ trackId: left.id, startStation: 100, endStation: 300 })
  })
})
