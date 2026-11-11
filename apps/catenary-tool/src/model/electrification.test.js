import { describe, expect, it } from 'vitest'
import { added, checkElectrification, coverage, domainOf, newElectrification, recordOf, removed, replaced, statusOf, withStatus } from './electrification.js'
import { nearest, netLines, stretch } from './net.js'

// Two straight tracks in UTM 32, 1000 m and 500 m, side by side.
const straight = (id, y, length) => ({
  id, name: id, epsg: 25832,
  elements: [{ elementType: 0, length, bearing: 90, startNode: [500000, 5700000 + y], endNode: [500000 + length, 5700000 + y] }],
})
const tracks = [straight('t1', 0, 1000), straight('t2', 50, 500)]
const record = { id: 'p1', tracks, switches: [], platforms: [] }

describe('the electrification of a net (decision 0025)', () => {
  it('is no layer of the record while it holds nothing', () => {
    expect(domainOf(record)).toEqual({ fields: {}, collections: { electrifications: [] } })
    expect(recordOf(record, domainOf(record)).electrification).toBeUndefined()
    const one = added(domainOf(record), newElectrification({ trackId: 't1', from: 1000, to: 0, system: '15000V' }))
    expect(one.collections.electrifications[0].at).toEqual({ anchor: 'station', trackId: 't1', startStation: 0, endStation: 1000 })
    expect(recordOf(record, one).electrification).toBe(one)
    expect(recordOf(record, removed(one, one.collections.electrifications[0].id)).electrification).toBeUndefined()
  })

  it('finds a range beyond its track, one without length, and two systems that meet', () => {
    let d = domainOf(record)
    const a = newElectrification({ trackId: 't1', from: 0, to: 600, system: '15000V' })
    const b = newElectrification({ trackId: 't1', from: 500, to: 1000, system: '25000V' })
    const c = newElectrification({ trackId: 't2', from: 0, to: 700, system: '15000V' })
    const gone = newElectrification({ trackId: 'tx', from: 0, to: 10, system: '15000V' })
    d = added(added(added(added(d, a), b), c), gone)
    const { errors, warnings } = checkElectrification(d, tracks)
    expect(errors.map(f => [f.code, f.id])).toEqual([['off_track', c.id], ['track_missing', gone.id], ['systems_meet', b.id]])
    expect(warnings).toEqual([])
    const same = replaced(d, { ...b, system: '15000V' })
    expect(checkElectrification(same, tracks).warnings.map(f => f.code)).toEqual(['overlap'])
  })

  it('says how much of the net is under wire, an overlap once', () => {
    const d = added(added(domainOf(record), newElectrification({ trackId: 't1', from: 0, to: 600, system: '15000V' })),
      newElectrification({ trackId: 't1', from: 400, to: 800, system: '15000V' }))
    const cov = coverage(d, tracks)
    expect(cov.tracks).toBeCloseTo(1500, 6)
    expect(cov.electrified).toBeCloseTo(800, 6)
    expect(cov.bySystem['15000V']).toBeCloseTo(1000, 6)
  })

  it('keeps the planning status as a lifespan over the project\'s two stages', () => {
    const e = newElectrification({ trackId: 't1', from: 0, to: 100, system: '15000V' })
    expect(statusOf(e, 'p1')).toBe('existing')
    expect(statusOf(withStatus(e, 'new', 'p1'), 'p1')).toBe('new')
    expect(statusOf(withStatus(e, 'removal', 'p1'), 'p1')).toBe('removal')
    expect(withStatus(withStatus(e, 'new', 'p1'), 'existing', 'p1')).toEqual(e)
  })
})

describe('the net on the map', () => {
  it('draws each track from its stations and finds the station nearest a click', () => {
    const lines = netLines(tracks)
    expect([...lines.keys()]).toEqual(['t1', 't2'])
    const mid = stretch(lines, 't1', 250, 750)
    expect(mid).toHaveLength(2)
    const hit = nearest(lines, mid[0], 5)
    expect(hit.trackId).toBe('t1')
    expect(hit.station).toBeCloseTo(250, 1)
    expect(nearest(lines, [0, 0], 5)).toBeNull()
  })
})
