import { describe, it, expect } from 'vitest'
import { planRailJsonImport } from './railJsonImport'
import { straightFrom } from '@rail-across/core/src/geometry/elementFactory.js'
import { uuidOf as u } from '@rail-across/core/document'

const EPSG = 25832
const el = straightFrom({ easting: 500000, northing: 5700000, zone: EPSG }, 90, 100)
const track = (id, name) => ({ id, name, epsg: EPSG, elements: [el] })
let n = 0
const newId = () => `new${++n}`

describe('planning a RailJSON import', () => {
  const parsed = {
    tracks: [track('a', '6340.001'), track('b', '6340.002')],
    switches: [{ switchId: 'w1', kind: 'switch', ports: { A: { trackId: 'a' }, B1: { trackId: 'b' }, B2: { trackId: null } }}],
    endMarks: [{ id: 'm1', trackId: 'a', endpoint: 'BEGIN' }],
    infra: { routes: [1], switches: [{ id: 'f', ports: { A: { track: 'a', endpoint: 'END' } } }] },
  }

  it('takes tracks, switches and marks into an empty project, under the UUIDs drawn from the file\'s ids', () => {
    const plan = planRailJsonImport(parsed, { newId })
    expect(plan.addTracks.map(tr => [tr.id, tr.name])).toEqual([[u('a'), '6340.001'], [u('b'), '6340.002']])
    expect(plan.addTracks[0].coordinates.length).toBeGreaterThan(0)
    expect(plan.addSwitches[0]).toMatchObject({ ports: { A: { trackId: u('a') }, B1: { trackId: u('b') }, B2: { trackId: null } } })
    expect(plan.addEndMarks).toEqual([{ id: u('m1'), trackId: u('a'), endpoint: 'BEGIN' }])
    // What is carried over names the tracks as the record does now.
    expect(plan.osrd).toEqual({ routes: [1], switches: [{ id: 'f', ports: { A: { track: u('a'), endpoint: 'END' } } }] })
  })

  it('keeps an id that is a UUID already', () => {
    const id = '0f8fad5b-d9cb-469f-a165-70867728950e'
    expect(planRailJsonImport({ tracks: [track(id, 'x')] }, { newId }).addTracks[0].id).toBe(id)
  })

  it('gives taken ids and names new ones and repoints everything that named them', () => {
    const plan = planRailJsonImport(parsed, {
      existingTracks: [track(u('a'), '6340.001')], existingMarks: [{ id: u('m1') }], newId,
    })
    const [a, b] = plan.addTracks
    expect(a.id).not.toBe(u('a'))
    expect(a.name).not.toBe('6340.001')
    // b keeps its id; its name went to the renamed a, so it takes the next one.
    expect(b).toMatchObject({ id: u('b'), name: '6340.003' })
    expect(plan.addSwitches[0].ports?.A?.trackId).toBe(a.id)
    expect(plan.addEndMarks[0].trackId).toBe(a.id)
    expect(plan.addEndMarks[0].id).not.toBe(u('m1'))
    expect(plan.osrd.switches[0].ports.A.track).toBe(a.id)
    expect(plan.osrd.routes).toEqual([1])
  })
})
