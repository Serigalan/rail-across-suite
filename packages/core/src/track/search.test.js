import { describe, it, expect } from 'vitest'
import { switchMatches, trackMatches } from './index.js'

const line = { id: 'a', name: '6340.12393', lineNumber: '6340', lineName: 'Halle – Bebra', trackType: 'line_track' }
const station = { id: 'b', name: 'Gl. 3', operationalPointId: 'op-k', trackNumber: '3', trackType: 'station_track' }
// Its station is the point it names (decision 0006), handed to the search.
const point = { id: 'op-k', name: 'Könnern', uic: '8010205' }

describe('search', () => {
  it('finds a track by any part of what it is called', () => {
    expect(trackMatches(line, '6340')).toBe(true)
    expect(trackMatches(line, 'bebra')).toBe(true)
    expect(trackMatches(station, 'konnern', point)).toBe(true)       // without the umlaut
    expect(trackMatches(station, 'Könnern 3', point)).toBe(true)     // every word, anywhere
    expect(trackMatches(station, 'Könnern 4', point)).toBe(false)
    expect(trackMatches(station, '8010205', point)).toBe(true)
    expect(trackMatches(station, 'konnern')).toBe(false)             // no point, no station
    expect(trackMatches(line, '')).toBe(true)
  })

  it('finds a switch by name or form', () => {
    const sw = { switchId: 'abc', name: 'W 12', label: 'EW 60-500-1:12' }
    expect(switchMatches(sw, 'w 12')).toBe(true)
    expect(switchMatches(sw, '1:12')).toBe(true)
    expect(switchMatches(sw, 'W 13')).toBe(false)
  })
})
