import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest'
import { utmToWgs84 } from '@rail-across/core/geometry'
import { identifyTrackLine, lineNameOf } from './lineLookup'

// The line files are fetched next to the app; here they answer from a table.
const FILES = {
  '6344': { lineNumber: 6344, lineName: 'Halle - Vienenburg', chainage: [] },
  '6345': { lineNumber: 6345, chainage: [] },
  '_index': { cell: [0.1, 0.1], cells: {} },
}

// A track running 1 km east in UTM 32, and a line 5 m north of it whose
// kilometrage rises from 12.000 km at its western end.
const EPSG = 25832
const E0 = 600000, N0 = 5700000
const track = { epsg: EPSG, path: [[E0, N0], [E0 + 1000, N0]] }
const localLine = (lineNumber, extra = {}) => ({
  id: `km-${lineNumber}`, lineNumber, crs: 'EPSG:4326',
  runs: [[0, 1200].map((d) => [...utmToWgs84(E0 - 100 + d, N0 + 5, EPSG), 11900 + d])],
  ...extra,
})
// The shipped line 6346 runs along the same track, stationed from 50.000 km.
FILES['6346'] = { lineNumber: 6346, lineName: 'Shipped', chainage: localLine(6346).runs.map((run) => run.map(([lon, lat, km]) => [lon, lat, km + 38100])) }

describe('the line lookup', () => {
  const fetchMock = vi.fn(async (url) => {
    const file = FILES[decodeURIComponent(String(url).split('/').pop().replace(/\.json$/, ''))]
    return file
      ? { ok: true, status: 200, json: async () => file }
      : { ok: false, status: 404, json: async () => null }
  })
  beforeAll(() => {
    vi.stubGlobal('window', { location: { href: 'http://localhost/' } })
    vi.stubGlobal('fetch', fetchMock)
  })
  afterAll(() => vi.unstubAllGlobals())

  it('gives the name the line file carries', async () => {
    expect(await lineNameOf('6344')).toBe('Halle - Vienenburg')
    expect(await lineNameOf(' 6344 ')).toBe('Halle - Vienenburg')
    expect(await lineNameOf(6344)).toBe('Halle - Vienenburg')
  })

  it('gives null for a line without a name or without a file', async () => {
    expect(await lineNameOf('6345')).toBeNull()
    expect(await lineNameOf('9999')).toBeNull()
  })

  it('names a track after a line of the project where no file has one', async () => {
    const hit = await identifyTrackLine(track, null, { localLines: [localLine(6399, { source: 'vermesn' })] })
    expect(hit.lineNumber).toBe('6399')
    expect(hit.km).toBeCloseTo(12500, 3)
    expect(hit.offset).toBeCloseTo(5, 1)
  })

  it('reads the kilometrage off an imported line before the shipped one of its number', async () => {
    expect((await identifyTrackLine(track, '6344')).km).toBeNull()
    const hit = await identifyTrackLine(track, '6344', { localLines: [localLine(6344, { source: 'mdb' })] })
    expect(hit).toMatchObject({ lineNumber: '6344', lineName: 'Halle - Vienenburg' })
    expect(hit.km).toBeCloseTo(12500, 3)
  })

  it('keeps the shipped line where the project holds only a stretch of it', async () => {
    expect((await identifyTrackLine(track, '6346')).km).toBeCloseTo(50600, 3)
    expect((await identifyTrackLine(track, '6346', { localLines: [localLine(6346)] })).km).toBeCloseTo(50600, 3)
    expect((await identifyTrackLine(track, '6346', { localLines: [localLine(6346, { source: 'mdb' })] })).km).toBeCloseTo(12500, 3)
  })

  it('finds nothing without a line near', async () => {
    expect(await identifyTrackLine(track)).toBeNull()
  })

  it('does not look up what is no number', async () => {
    fetchMock.mockClear()
    expect(await lineNameOf('')).toBeNull()
    expect(await lineNameOf(null)).toBeNull()
    expect(await lineNameOf('63a4')).toBeNull()
    expect(fetchMock).not.toHaveBeenCalled()
  })
})
