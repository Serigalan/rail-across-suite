import { describe, expect, it } from 'vitest'
import { buildCurvedLineTrack, buildLineTrack, translateTracks } from './tracks.js'
import { reconstructElements } from '../geometry/index.js'
import { rebuildCoords } from '../track/index.js'

const at = (easting, northing) => ({ easting, northing, zone: 25832 })
const straight = buildLineTrack({ start: at(600000, 5700000), end: at(600100, 5700050), speed: 80, meta: { name: 'a' }, id: 'a' })
const arc = buildCurvedLineTrack({ start: at(600100, 5700050), end: at(600200, 5700150), signedR: 300, speed: 80, cant: 0, meta: { name: 'b' }, id: 'b' })
const project = {
  tracks: [straight, arc],
  switches: [{ switchId: 'w1', ports: { A: { trackId: 'b', endpoint: 'start' } } }, { switchId: 'w2', ports: { A: { trackId: 'a', endpoint: 'start' } } }],
}

describe('translateTracks', () => {
  it('moves every node of the track by the shift and keeps its design values', () => {
    const next = translateTracks(project, ['b'], 12.5, -3)
    const [before] = arc.elements
    const [after] = next.tracks[1].elements
    expect(after.startNode).toEqual([before.startNode[0] + 12.5, before.startNode[1] - 3])
    expect(after.endNode).toEqual([before.endNode[0] + 12.5, before.endNode[1] - 3])
    for (const key of ['bearing', 'endBearing', 'length', 'radius', 'speed']) expect(after[key]).toBe(before[key])
  })

  it('draws the moved track anew from its plane data', () => {
    const moved = translateTracks(project, ['b'], 100, 200).tracks[1]
    const drawn = reconstructElements(moved.elements, moved.epsg)
    expect(moved.elements.map(el => el.geometry)).toEqual(drawn.map(el => el.geometry))
    expect(moved.coordinates).toEqual(rebuildCoords(drawn))
    expect(moved.coordinates[0]).not.toEqual(arc.coordinates[0])
  })

  it('leaves the other tracks and their switches as they were', () => {
    const next = translateTracks(project, ['b'], 1, 1)
    expect(next.tracks[0]).toBe(straight)
    expect(next.switches[1]).toBe(project.switches[1])
    expect(next.switches[0]).not.toBe(project.switches[0])
  })

  it('changes nothing without a shift or a track', () => {
    expect(translateTracks(project, ['b'], 0, 0)).toBe(project)
    expect(translateTracks(project, ['x'], 1, 1)).toBe(project)
    expect(translateTracks(project, ['b'], NaN, 1)).toBe(project)
  })
})
