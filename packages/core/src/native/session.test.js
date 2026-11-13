import { afterEach, describe, expect, it } from 'vitest'
import { callNative, heldProject, holdProject } from './index.js'
import { dehydrateProjects, heldKeptProject, heldProjectDocument, hydrateProjects, projectDocument } from '../project/index.js'
import { planElementChange, translateTracks } from '../commands/index.js'
import { branchTrack, mainTrack, turnout } from '../../test/turnoutFixture.js'

// The project the core's module holds (decision 0021, phase C): written by
// what changed, lent to the operations that are handed its members.

const deepFreeze = (v) => {
  if (v && typeof v === 'object' && !Object.isFrozen(v)) {
    Object.freeze(v)
    for (const x of Object.values(v)) deepFreeze(x)
  }
  return v
}
// The turnout's tracks, a the one before it among them: a document names nothing it lacks (decision 0028).
const project = () => deepFreeze(hydrateProjects([{
  id: 'p', title: 'P', tracks: [mainTrack(), branchTrack(), { ...mainTrack(), id: 'a', name: 'a' }], switches: [turnout({ formVersion: 1, name: 'W1' })], platforms: [],
}])[0])

afterEach(() => holdProject(null))

describe('the held project', () => {
  it('is what the page last handed it, written by what changed', () => {
    const first = project()
    holdProject(first)
    expect(heldProject()).toBe(first)
    expect(heldKeptProject(first)).toEqual(dehydrateProjects([first])[0])
    // A step that changes one track and adds one: the other track is named by its place.
    const moved = translateTracks([first.tracks[1]], 5, 0)[0]
    const extra = { ...first.tracks[0], id: 'x' }
    const second = deepFreeze({ ...first, title: 'Q', tracks: [first.tracks[0], moved, extra] })
    holdProject(second)
    expect(heldKeptProject(second)).toEqual(dehydrateProjects([second])[0])
    // A member taken away is gone there too.
    const { platforms: _p, ...third } = second
    holdProject(deepFreeze(third))
    expect(heldKeptProject(third)).toEqual(dehydrateProjects([third])[0])
  })

  it('makes the document a working copy keeps, as projectDocument makes it', () => {
    const p = project()
    holdProject(p)
    expect(heldProjectDocument(p, { carrySchemas: true })).toEqual(projectDocument(p, { carrySchemas: true }))
  })

  it('lends its members to an operation and has them back after it — twice in one call too', () => {
    const p = project()
    holdProject(p)
    const remapped = callNative('track.remapSwitches', [p.switches, [{ oldId: 'm', newId: 'm2' }]])
    expect(remapped).toEqual(callNative('track.remapSwitches', [structuredClone(p.switches), [{ oldId: 'm', newId: 'm2' }]]))
    expect(callNative('track.rebuildCoords', [p.tracks[0].elements])).toHaveLength(callNative('track.rebuildCoords', [structuredClone(p.tracks[0].elements)]).length)
    expect(callNative('track.switchNumbersInUse', [p.switches])).toEqual(callNative('track.switchNumbersInUse', [structuredClone(p.switches)]))
    // The same member twice in one call: lent once, copied for the second.
    const loose = structuredClone(p)
    expect(callNative('switch.coupleSwitchCants', [p.tracks, p.tracks, p.switches]))
      .toEqual(callNative('switch.coupleSwitchCants', [loose.tracks, loose.tracks, loose.switches]))
    expect(heldKeptProject(p)).toEqual(dehydrateProjects([p])[0])
  })

  it('answers a change of an element as the command does on tracks it does not hold', () => {
    const p = project()
    holdProject(p)
    const change = { length: 35 }
    const held = planElementChange(p.tracks, p.switches, 'm', 1, change)
    const copies = structuredClone({ tracks: p.tracks, switches: p.switches })
    const loose = planElementChange(copies.tracks, copies.switches, 'm', 1, change)
    expect(held).toEqual(loose)
    // What it did not touch is the very track the page has.
    for (const [i, t] of held.tracks.entries()) if (!held.touchedTrackIds.includes(t.id)) expect(t).toBe(p.tracks[i])
  })

  it('takes back what it answered by name, and whole where the answer is no longer kept', () => {
    const p = project()
    holdProject(p)
    const plan = planElementChange(p.tracks, p.switches, 'm', 1, { length: 36 })
    const next = deepFreeze({ ...p, tracks: plan.tracks })
    holdProject(next)
    expect(heldKeptProject(next)).toEqual(dehydrateProjects([next])[0])
    // An early plan written after many others: its answer is gone, the track crosses whole.
    const early = planElementChange(next.tracks, next.switches, 'm', 1, { length: 37 })
    for (let i = 0; i < 6; i++) planElementChange(next.tracks, next.switches, 'm', 1, { length: 38 + i })
    const late = deepFreeze({ ...next, tracks: early.tracks })
    holdProject(late)
    expect(heldKeptProject(late)).toEqual(dehydrateProjects([late])[0])
  })

  it('lends a single track or switch of it, and copies where the same part is named twice or with its member', () => {
    const p = project()
    holdProject(p)
    const loose = structuredClone(p)
    expect(callNative('track.rebuildCoords', [p.tracks[1].elements])).toEqual(callNative('track.rebuildCoords', [loose.tracks[1].elements]))
    // A track's elements and the track itself in one call: the elements copied, the track lent.
    expect(callNative('track.remapSwitches', [p.switches, [{ oldId: p.tracks[1].id, newId: 'z' }], p.tracks[1].elements, p.tracks[1]]))
      .toEqual(callNative('track.remapSwitches', [loose.switches, [{ oldId: 'b', newId: 'z' }], loose.tracks[1].elements, loose.tracks[1]]))
    expect(callNative('switch.coupleSwitchCants', [[p.tracks[0], p.tracks[1]], p.tracks, p.switches]))
      .toEqual(callNative('switch.coupleSwitchCants', [[loose.tracks[0], loose.tracks[1]], loose.tracks, loose.switches]))
    expect(callNative('track.remapSwitches', [[p.switches[0], p.switches[0]], []])).toEqual(callNative('track.remapSwitches', [[loose.switches[0], loose.switches[0]], []]))
    expect(heldKeptProject(p)).toEqual(dehydrateProjects([p])[0])
  })
})
