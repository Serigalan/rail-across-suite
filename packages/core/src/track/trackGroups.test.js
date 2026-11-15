import { describe, it, expect } from 'vitest'
import {
  trackKind, trackTypeName, trackGroupKey, groupTracks, groupTitle, assignTracks, plannedNames, stationTrackStem,
} from './index.js'
import { groupHeading, kmTrackName } from './index.js'
import { buildTypeFields } from './index.js'

const line = (id, lineNumber, extra = {}) => ({ id, name: id, trackType: 'line', lineNumber, ...extra })
// A station track names its operational point (decision 0028); the points of these tests by their ids.
const station = (id, operationalPointId, extra = {}) => ({ id, name: id, trackType: 'station', operationalPointId, ...extra })
const POINTS = [
  { id: 'op-k', name: 'Könnern', uic: '8010205' }, { id: 'op-h', name: 'Halle (Saale) Hbf' }, { id: 'op-w', name: 'Wallwitz' },
]

describe('what a track belongs to', () => {
  it('is what its type says', () => {
    expect(trackKind({ trackType: 'line' })).toBe('line')
    expect(trackKind({ trackType: 'station', lineNumber: 6340 })).toBe('station')
  })

  it('is read off its fields where it states no type', () => {
    expect(trackKind({ lineNumber: '6340' })).toBe('line')
    // A station track keeps the line number its kilometrage is read off.
    expect(trackKind({ lineNumber: 6340, operationalPointId: 'op-k' })).toBe('station')
    expect(trackKind({ name: 'track 1' })).toBe(null)
  })

  it('opens the forms on the type it has', () => {
    expect(trackTypeName({ operationalPointId: 'op-k' }, 'line_track')).toBe('station_track')
    expect(trackTypeName({}, 'line_track')).toBe('line_track')
  })

  it('is written by the forms', () => {
    expect(trackKind(buildTypeFields({ type: 'station_track', stationName: '' }))).toBe('station')
    expect(trackKind(buildTypeFields({ type: 'line_track', lineNumber: '' }))).toBe('line')
  })

  it('is grouped by line number, and by its operational point', () => {
    expect(trackGroupKey(line('a', 6340))).toBe('line:6340')
    expect(trackGroupKey(station('a', 'op-h'))).toBe('station:op-h')
    // A station track that names no point names no station.
    expect(trackGroupKey(station('a', undefined))).toBe(null)
    // A line track without a number names no line.
    expect(trackGroupKey(line('a', null))).toBe(null)
  })
})

describe('the tracks in groups', () => {
  const tracks = [
    station('s2', 'op-k'),
    { id: 'x', name: 'loose' },
    line('l10', 6340, { name: '6340.10', lineName: 'Halle–Könnern' }),
    line('l2', '6340', { name: '6340.2' }),
    line('m', 6343),
    station('s1', 'op-k'),
    station('h', 'op-h'),
  ]
  const groups = groupTracks(tracks, POINTS)

  it('are lines by number, then stations by name, then the unassigned', () => {
    expect(groups.map(g => g.key)).toEqual([
      'line:6340', 'line:6343', 'station:op-h', 'station:op-k', 'none',
    ])
    expect(groups[3]).toMatchObject({ operationalPointId: 'op-k' })
  })

  it('hold their tracks sorted by name, numbers counted', () => {
    expect(groups[0].tracks.map(tr => tr.id)).toEqual(['l2', 'l10'])
  })

  it('are named by the first track that names them, a station by its point', () => {
    expect(groupTitle(groups[0])).toBe('6340 · Halle–Könnern')
    expect(groupTitle(groups[3])).toBe('Könnern')
    expect(groups[3].uicStation).toBe('8010205')
  })

  it('are headed in the list by their kind and title — through the module\'s face, as the panels ask', () => {
    const t = (key) => ({ track_group_line: 'Strecke', track_group_station: 'Bahnhof', track_group_none: 'Ohne' })[key]
    expect(groupHeading(t, groups[0])).toBe('Strecke 6340 · Halle–Könnern')
    expect(groupHeading(t, groups[3])).toBe('Bahnhof Könnern')
    expect(groupHeading(t, groups[4])).toBe('Ohne')
  })

  it('leave out the unassigned group when every track belongs somewhere', () => {
    expect(groupTracks([line('a', 6340)], POINTS).map(g => g.key)).toEqual(['line:6340'])
    expect(groupTracks([], POINTS)).toEqual([])
  })
})

describe('assigning tracks', () => {
  const tracks = [
    line('a', 6340, { lineName: 'Halle–Könnern', side: 'regular' }),
    station('b', 'op-k', { lineNumber: 6340, trackNumber: 3 }),
    { id: 'c' },
  ]

  it('puts them on a line, leaving the others as they are', () => {
    const out = assignTracks(tracks, ['b', 'c'], { kind: 'line', lineNumber: ' 6343 ', lineName: '' })
    expect(out[0]).toBe(tracks[0])
    expect(trackGroupKey(out[1])).toBe('line:6343')
    expect(out[2]).toMatchObject({ trackType: 'line', lineNumber: 6343, lineName: null })
    // What only the other kind states stays: it may be wanted back.
    expect(out[1].trackNumber).toBe(3)
  })

  it('puts them into a station, keeping the line their kilometrage is read off', () => {
    const [a] = assignTracks(tracks, ['a'], { kind: 'station', operationalPointId: 'op-h', stationName: 'Halle (Saale) Hbf' })
    expect(trackGroupKey(a)).toBe('station:op-h')
    expect(a).not.toHaveProperty('stationName')
    expect(a.lineNumber).toBe(6340)
    // a station typed anew stays as typed, for the record to make its point of
    const [b] = assignTracks(tracks, ['a'], { kind: 'station', stationName: 'Wallwitz (Saalkr)', uicStation: '' })
    expect(b).toMatchObject({ trackType: 'station', stationName: 'Wallwitz (Saalkr)', uicStation: null })
  })

  it('takes them out of what they were in', () => {
    const out = assignTracks(tracks, ['a', 'b'], { kind: null })
    expect(out.slice(0, 2).map(trackGroupKey)).toEqual([null, null])
    expect(out[1].operationalPointId).toBeNull()
    expect(groupTracks(out, POINTS).map(g => g.key)).toEqual(['none'])
  })
})

describe('the name of a track on a line', () => {
  it('is the line number and the kilometre, five digits wide', () => {
    expect(kmTrackName(6344, 29120)).toBe('6344.02912')
    expect(kmTrackName(6340, 123930)).toBe('6340.12393')
    expect(kmTrackName(6344, 29120, ['6344.02912'])).toBe('6344.02913')
    expect(kmTrackName(6344, -40)).toBe('6344.-00004')
  })
})

describe('renaming tracks as they are assigned', () => {
  const tracks = [
    { id: 'a', name: 'track 1' },
    { id: 'b', name: 'track 2', trackNumber: 3 },
    { id: 'c', name: 'track 3' },
    { id: 'o', name: '6344.02912' },
    { id: 'k', name: 'koennern.4' },
  ]

  it('names them by line and kilometre, moving on past names taken', () => {
    const { names, fallback, clashes } = plannedNames(tracks, ['a', 'b', 'c'],
      { kind: 'line', lineNumber: '6344' }, { kmById: { a: 29120, b: 29118, c: null } })
    // b lies before a, takes the next ten metres past the other track's name,
    // and a the ten after that.
    expect(names.get('b')).toBe('6344.02913')
    expect(names.get('a')).toBe('6344.02914')
    expect(names.get('c')).toBe('6344.001')
    expect([...fallback]).toEqual(['c'])
    expect(clashes.size).toBe(0)
  })

  it('names them by station and track number, numbering on those without one', () => {
    const { names, fallback, clashes } = plannedNames(tracks, ['a', 'b', 'c'],
      { kind: 'station', stationName: 'Könnern' }, { numberById: { a: '1', b: ' 3 ', c: '' } })
    expect(names.get('a')).toBe('koennern.1')
    expect(names.get('b')).toBe('koennern.3')
    expect(names.get('c')).toBe('koennern.001')
    expect([...fallback]).toEqual(['c'])
    expect(clashes.size).toBe(0)
  })

  it('builds a station track\'s name from the station\'s name as an identifier', () => {
    expect(stationTrackStem('Könnern')).toBe('koennern')
    expect(stationTrackStem('Wallwitz (Saalkreis)')).toBe('wallwitz')
    expect(stationTrackStem('Wallwitz (Saalkr)')).toBe('wallwitz')
    expect(stationTrackStem('Halle (Saale) Hbf')).toBe('halle-hbf')
    expect(stationTrackStem('Halle-Trotha')).toBe('halle-trotha')
    expect(stationTrackStem('Großkorbetha')).toBe('grosskorbetha')
    expect(stationTrackStem('Dürrenberg / Süd')).toBe('duerrenberg-sued')
    expect(stationTrackStem('Žilina')).toBe('zilina')
    expect(stationTrackStem('8010159')).toBe('8010159')
    expect(stationTrackStem(' (–) ')).toBe('track')
    expect(plannedNames([], ['x'], { kind: 'station', stationName: 'Wallwitz (Saalkreis)' }, { numberById: { x: '' } }).names.get('x')).toBe('wallwitz.001')
  })

  it('says which station names clash, with another track or among themselves', () => {
    const { clashes } = plannedNames(tracks, ['a', 'b', 'c'],
      { kind: 'station', stationName: 'Könnern' }, { numberById: { a: '4', b: '2', c: '2' } })
    expect([...clashes].sort()).toEqual(['a', 'b', 'c'])
  })

  it('lets a renamed track give up its own name', () => {
    const { names } = plannedNames(tracks, ['k'], { kind: 'station', stationName: 'Könnern' }, { numberById: { k: '4' } })
    expect(names.get('k')).toBe('koennern.4')
  })

  it('writes the names and the track numbers with the assignment', () => {
    const target = { kind: 'station', stationName: 'Könnern' }
    const numberById = { a: '1', b: '' }
    const withSuffix = assignTracks(tracks, ['c'], target, { trackNumbers: { c: '3a' } })
    expect(withSuffix[2].trackNumber).toBe('3a')
    const { names } = plannedNames(tracks, ['a', 'b'], target, { numberById })
    const out = assignTracks(tracks, ['a', 'b'], target, { names, trackNumbers: numberById })
    expect(out[0]).toMatchObject({ name: 'koennern.1', trackNumber: 1, stationName: 'Könnern' })
    expect(out[1]).toMatchObject({ name: 'koennern.001', trackNumber: null })
    expect(out[2]).toBe(tracks[2])
  })
})
