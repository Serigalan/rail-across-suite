import { describe, expect, it } from 'vitest'
import { renamedStation, stationRegister } from './stations.js'

const record = {
  id: 'p1',
  tracks: [
    { id: 't1', stationName: 'A-Dorf', trackNumber: '1' }, { id: 't2', stationName: 'A-Dorf', trackNumber: '2' },
    { id: 't3', lineNumber: '6000' },
  ],
  platforms: [{ id: 'p1', trackId: 't3', stationName: 'B-Halt' }],
}

describe('the stations of a record by their ids', () => {
  it('gives every station and halt an id, the same for the same project and name', () => {
    const { points, pointOf } = stationRegister(record)
    expect(points.map(p => p.name).sort()).toEqual(['A-Dorf', 'B-Halt'])
    expect(pointOf(record.tracks[0])).toBe(pointOf(record.tracks[1]))
    expect(pointOf(record.tracks[2])).toBeUndefined()
    expect(pointOf(record.tracks[0])).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
    expect(stationRegister(structuredClone(record)).points).toEqual(points)
  })

  it('keeps a station\'s id when it is renamed, its tracks and platforms stating the new name', () => {
    const before = stationRegister(record)
    const a = before.pointOf(record.tracks[0]), b = before.pointOf(record.platforms[0])
    const renamed = { ...record, ...renamedStation(record, a, { name: 'Adorf', uic: '8000001' }) }
    expect(renamed.tracks.map(tr => [tr.stationName, tr.uicStation])).toEqual([['Adorf', '8000001'], ['Adorf', '8000001'], [undefined, undefined]])
    const after = stationRegister(renamed)
    expect(after.pointOf(renamed.tracks[0])).toBe(a)
    expect(after.points.find(p => p.id === a)).toEqual({ id: a, name: 'Adorf', uic: '8000001' })
    // a halt too
    const halt = { ...renamed, ...renamedStation(renamed, b, { name: 'Bhalt' }) }
    expect(halt.platforms[0].stationName).toBe('Bhalt')
    expect(stationRegister(halt).pointOf(halt.platforms[0])).toBe(b)
    expect(stationRegister(halt).pointOf(halt.tracks[0])).toBe(a)
    expect(renamedStation(record, 'none', { name: 'X' })).toBeNull()
  })
})
