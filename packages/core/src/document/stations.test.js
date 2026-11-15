import { describe, expect, it } from 'vitest'
import { pointOf, renamedStation, stationPoint, stationRegister, withStationPoints } from './stations.js'

const record = {
  id: 'p1',
  operationalPoints: [{ id: 'op-a', name: 'A-Dorf' }, { id: 'op-b', name: 'B-Halt', rl100: 'BH' }],
  tracks: [
    { id: 't1', operationalPointId: 'op-a', trackNumber: '1' }, { id: 't2', operationalPointId: 'op-a', trackNumber: '2' },
    { id: 't3', lineNumber: '6000' },
  ],
  platforms: [{ id: 'p1', trackId: 't3', operationalPointId: 'op-b' }],
}

describe('the stations of a record by their ids (decision 0028)', () => {
  it('are the record\'s points, each track and platform naming its own', () => {
    const { points, pointOf: of } = stationRegister(record)
    expect(points).toEqual(record.operationalPoints)
    expect(of(record.tracks[0])).toBe('op-a')
    expect(of(record.tracks[2])).toBeUndefined()
    expect(of(record.platforms[0])).toBe('op-b')
    expect(pointOf(record, record.tracks[1])).toEqual({ id: 'op-a', name: 'A-Dorf' })
  })

  it('keep a station\'s id when it is renamed, its tracks and platforms as they are', () => {
    const renamed = { ...record, ...renamedStation(record, 'op-a', { name: 'Adorf', uic: '8000001' }) }
    expect(renamed.tracks).toBe(record.tracks)
    expect(pointOf(renamed, renamed.tracks[0])).toEqual({ id: 'op-a', name: 'Adorf', uic: '8000001' })
    const halt = { ...renamed, ...renamedStation(renamed, 'op-b', { name: 'Bhalt' }) }
    expect(pointOf(halt, halt.platforms[0])).toEqual({ id: 'op-b', name: 'Bhalt', rl100: 'BH' })
    expect(renamedStation(record, 'none', { name: 'X' })).toBeNull()
  })

  it('find the point a name names, or make one', () => {
    expect(stationPoint(record, { name: ' a-dorf ' })).toEqual({ operationalPoints: record.operationalPoints, id: 'op-a' })
    expect(stationPoint(record, { name: 'B-Halt', rl100: 'XY' }).id).not.toBe('op-b')
    const made = stationPoint(record, { name: 'C-Stadt', uic: '8000003' })
    expect(made.operationalPoints.at(-1)).toEqual({ id: made.id, name: 'C-Stadt', uic: '8000003' })
    expect(stationPoint(record, {})).toBeNull()
  })

  it('make points of the station texts an importer reads', () => {
    const read = withStationPoints({
      id: 'p2',
      tracks: [{ id: 't1', stationName: 'A-Dorf', uicStation: '8000001' }, { id: 't2', stationName: 'A-Dorf' }, { id: 't3', lineNumber: '6000' }],
      platforms: [{ id: 'p1', trackId: 't3', stationName: 'B-Halt', code: 'BH' }],
    })
    expect(read.tracks[0].operationalPointId).toBe(read.tracks[1].operationalPointId)
    expect(read.tracks[2].operationalPointId).toBeUndefined()
    expect(read.tracks.some(t => 'stationName' in t || 'uicStation' in t)).toBe(false)
    expect(pointOf(read, read.tracks[0])).toMatchObject({ name: 'A-Dorf', uic: '8000001' })
    expect(pointOf(read, read.platforms[0])).toMatchObject({ name: 'B-Halt', rl100: 'BH' })
  })
})
