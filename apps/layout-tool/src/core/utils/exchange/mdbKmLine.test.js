import { describe, it, expect, vi, afterEach } from 'vitest'
import { mdbKmRuns, mdbKmLines } from './mdbKmLine'
import { kmLineRuns } from '@rail-across/core/track'
import { parseMdbPayload } from './mdbImport'
import { ALL_LINES, runMdbImport } from './mdbPipeline'
import { ensureKmLines } from '../km'
import { fitShippedKmLine } from './importFit'
import fixture from '../../test/fixtures/mdb_str1244.json'
import { kmLineId } from '../km'

/**
 * The kilometrage line of the file (Satzart 24), on the delivered
 * STR_1244_KM_112-132: line 1244 from km 12.944 686 on, and a piece of line
 * 6100 around km 273 with a kilometrage jump of 7.770 m in it (type 6).
 */

const payload = parseMdbPayload(fixture)

describe('mdbKmRuns', () => {
  const runs = mdbKmRuns(payload)

  it('lays out the line of each number, its kilometrage in metres on every vertex', () => {
    expect([...runs.keys()].sort()).toEqual(['1244', '6100'])
    const [run] = runs.get('1244')
    expect(run[0][2]).toBeCloseTo(12944.686, 3)
    expect(run[run.length - 1][2]).toBeCloseTo(13718.423, 3)
    // Rising along the run, in WGS84 near Hamburg.
    for (let i = 1; i < run.length; i++) expect(run[i][2]).toBeGreaterThanOrEqual(run[i - 1][2])
    expect(run[0][0]).toBeGreaterThan(9)
    expect(run[0][0]).toBeLessThan(11)
    expect(run[0][1]).toBeGreaterThan(53)
    expect(run[0][1]).toBeLessThan(54)
  })

  it('starts a new run at a kilometrage jump', () => {
    const lines = runs.get('6100')
    expect(lines).toHaveLength(2)
    expect(lines[0].at(-1)[2]).toBeCloseTo(273203.349, 3)
    expect(lines[1][0][2]).toBeCloseTo(273211.119, 3)
  })

  it('keeps the kilometrage of a vertex true to the length along the element', () => {
    // The first element of line 1244 is a transition of 69.944 m from km 12.944 686.
    const [run] = runs.get('1244')
    const k = Math.cos(run[0][1] * Math.PI / 180)
    const d = Math.hypot((run[1][0] - run[0][0]) * 111320 * k, (run[1][1] - run[0][1]) * 110574)
    expect(run[1][2] - run[0][2]).toBeCloseTo(d, 0)
  })

  it('takes only the lines it is asked for', () => {
    expect([...mdbKmRuns(payload, new Set(['1244'])).keys()]).toEqual(['1244'])
  })
})

describe('mdbKmLines', () => {
  it('gives each line as the document keeps a km line, in its plane with its elements, marked as from the file', () => {
    const [line] = mdbKmLines(payload, ['1244'], null)
    expect(line).toMatchObject({ id: kmLineId(1244), lineNumber: 1244, epsg: expect.any(Number), source: 'mdb' })
    expect(line.runs).toHaveLength(1)
    expect(line.runs[0]).toMatchObject({ kmStart: expect.any(Number), counting: 'up' })
    expect(line.runs[0].elements[0]).toMatchObject({ elementType: 'transition', length: 69.94415, startNode: expect.any(Array) })
    // Drawn from its elements as the line it was.
    const [drawn] = kmLineRuns(line)
    const [run] = mdbKmRuns(payload).get('1244')
    expect(drawn[0][0]).toBeCloseTo(run[0][0], 6)
    expect(drawn[0][2]).toBeCloseTo(run[0][2], 3)
  })

  it('keeps the runs the box touches, whole — elements are not cut — and none the box misses', () => {
    const [run] = mdbKmRuns(payload).get('1244')
    const [lon, lat] = run[0]
    const [line] = mdbKmLines(payload, ['1244'], [lon - 0.001, lat - 0.001, lon + 0.001, lat + 0.001])
    expect(line.runs).toHaveLength(1)
    expect(line.runs[0].elements.length).toBe(mdbKmLines(payload, ['1244'], null)[0].runs[0].elements.length)
    expect(mdbKmLines(payload, ['1244'], [0, 0, 0.001, 0.001])).toEqual([])
  })
})

describe('in the import', () => {
  it('comes along for the lines the imported tracks run on', async () => {
    let n = 0
    const { commit, counts, notes } = await runMdbImport({
      payload, strecke: ALL_LINES, fill: (key, params) => `${key} ${JSON.stringify(params)}`, newId: () => `id${++n}`,
    })
    expect(counts.kmLines).toBe(2)
    expect(commit.addKmLines.map(l => l.lineNumber).sort()).toEqual([1244, 6100])
    expect(notes.some(t => t.startsWith('data_exchange_mdb_km_lines'))).toBe(true)
  })
})

describe('ensureKmLines', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('fetches nothing over a line from the file', async () => {
    const fetch = vi.fn()
    vi.stubGlobal('fetch', fetch)
    const result = await ensureKmLines({
      tracks: [{ lineNumber: 1244, coordinates: [[10, 53.5], [10.01, 53.51]] }],
      kmLines: [{ id: 'km-1244', lineNumber: 1244, clip: [0, 0, 1, 1], runs: [], source: 'mdb' }],
    })
    expect(fetch).not.toHaveBeenCalled()
    expect(result).toMatchObject({ save: [], remove: [] })
  })
  const file = { lineNumber: 6050, chainage: [[[11.95, 51.55, 1200], [11.951, 51.55, 1269.4], [11.952, 51.55, 1338.8]]] }
  const tracks = [{ lineNumber: 6050, coordinates: [[11.9505, 51.55], [11.9515, 51.55]] }]

  it('keeps a fetched line in the plane of its first point, its runs as points, fitted where it is asked to', async () => {
    vi.stubGlobal('window', { location: { href: 'http://localhost/layout/' } })
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, status: 200, json: async () => file })))
    const { save: [line] } = await ensureKmLines({ tracks, kmLines: [] })
    expect(line).toMatchObject({ id: kmLineId(6050), lineNumber: 6050, epsg: 25832 })
    expect(line.runs[0]).toMatchObject({ kmStart: 1200, counting: 'up' })
    expect(line.runs[0].points).toHaveLength(3)
    expect(line).not.toHaveProperty('crs')
    const fit = vi.fn(async (l) => ({ ...l, fitted: true }))
    const { save: [fitted] } = await ensureKmLines({ tracks, kmLines: [] }, { fit })
    expect(fit).toHaveBeenCalledOnce()
    expect(fitted.fitted).toBe(true)
    // Held now, and covering the tracks: nothing is fetched again.
    const again = vi.fn()
    vi.stubGlobal('fetch', again)
    expect((await ensureKmLines({ tracks, kmLines: [line] })).save).toEqual([])
    expect(again).not.toHaveBeenCalled()
  })
})

describe('fitShippedKmLine', () => {
  const line = { lineNumber: 6050, epsg: 25832, runs: [{ kmStart: 100, counting: 'up', points: [[0, 0, 100], [50, 0, 150], [100, 0, 200]] }] }
  const straight = { elementType: 'straight', startNode: [0, 0], endNode: [100, 0], bearing: 90, length: 100, speed: 0 }

  it('takes the elements of a fit within a centimetre whose stations are the points\' kilometrage', async () => {
    const run = async () => ({ elements: [straight], max: 0.004, stations: [0, 50, 100] })
    const fitted = await fitShippedKmLine(line, { run })
    expect(fitted.runs[0]).toEqual({ kmStart: 100, counting: 'up', elements: [{ elementType: 'straight', startNode: [0, 0], endNode: [100, 0], bearing: 90, length: 100 }] })
  })

  it('keeps the points where the fit misses them, or their kilometrage is not the station', async () => {
    expect((await fitShippedKmLine(line, { run: async () => ({ elements: [straight], max: 0.02, stations: [0, 50, 100] }) })).runs[0].points).toHaveLength(3)
    expect((await fitShippedKmLine(line, { run: async () => ({ elements: [straight], max: 0.004, stations: [0, 52, 100] }) })).runs[0].points).toHaveLength(3)
    expect((await fitShippedKmLine(line, { run: async () => { throw new Error('missing') } })).runs[0].points).toHaveLength(3)
  })
})
