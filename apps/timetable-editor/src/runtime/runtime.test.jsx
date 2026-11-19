import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { runtimeTexts } from './texts.js'
import { RuntimePanel } from './Runtime.jsx'
import { useRuntime } from './useRuntime.js'
import RuntimeResult from './RuntimeResult.jsx'
import RuntimeTrain from './RuntimeTrain.jsx'
import RunChart from './RunChart.jsx'
import { formatTime, pointAt } from '@rail-across/running-times'
import { linkedRequest, vehicleUrl } from '../suite.js'

const de = (node) => renderToStaticMarkup(node)
const { t } = runtimeTexts('de')

/** Computing a running time as the Fahrplan-Editor holds it, its train's panel shown; the catalogue still loading. */
function Computing({ record }) {
  const rt = useRuntime({ record, catalogue: { entries: [], loading: true }, t, language: 'de' })
  return <RuntimePanel section="train" panelProps={rt.panelProps} />
}

const record = {
  tracks: [{ id: 'T1', epsg: 25832, name: '1', trackNumber: '1', elements: [{ elementType: 'straight', startNode: [500000, 5600000], endNode: [501000, 5600000], bearing: 90, length: 1000 }] }],
  switches: [],
  platforms: [{ id: 'P1', trackId: 'T1', startStation: 400, endStation: 600, operationalPointId: 'op-h' }],
  operationalPoints: [{ id: 'op-h', name: 'Hausen' }],
  signalling: { collections: { signals: [{ id: 'S1', name: 'A', kind: 'real', at: { anchor: 'station', trackId: 'T1', station: 50, direction: 'up' } }] } },
}

const report = (scale) => ({
  positions: [0, 500, 500, 1000], times: [0, 50 * scale, 80 * scale, 130 * scale], speeds: [0, 20, 20, 0],
  scheduled_points: [{ path_offset: 500, arrival: 50 * scale, departure: 80 * scale }, { path_offset: 1000, arrival: 130 * scale, departure: 130 * scale }],
  profiles: [], energy_consumption: 3.6e6, total_time: 130 * scale,
})
const run = {
  result: { base: report(1), provisional: report(1.05), final: report(1.05), mrsp: [{ begin: 0, end: 1000, speed: 25 }], warnings: [] },
  request: { path: { length: 1000 }, schedule: [{ path_offset: 500, stop_duration: 30 }, { path_offset: 1000, stop_duration: 0 }] },
  properties: { speedLimits: [{ begin: 0, end: 1000, speed: 25 }], signalled: [{ begin: 200, end: 800, speed: 20 }] },
  offsets: [0, 500, 1000], labels: ['Signal A', 'Hausen Gl. 1', 'Ende'], key: 'k',
}

describe('computing a running time', () => {
  it('offers the train, its places from the signals and platforms, and the margin', () => {
    const html = de(<Computing record={record} />)
    expect(html).toContain('<h2>Fahrzeit rechnen</h2>')
    expect(html).toContain('Wird geladen')   // the suite's vehicles, loading at first
    expect(html).toContain('Signal A')
    expect(html).toContain('Hausen Gl. 1')
    expect(html).toContain('energiesparend (MARECO)')
    expect(html).toMatch(/<button[^>]*disabled[^>]*>Fahrzeit rechnen/)
  })

  it('offers the server\'s OSRD nets, and on one its stations as the places', () => {
    const props = {
      t, language: 'de', nets: [{ id: 'morocco.railjson', name: 'morocco' }], netId: 'morocco.railjson', chooseNet: () => {}, netLoading: false, netError: null,
      places: { signals: [], platforms: [], stations: [{ id: 'osrd:1', label: 'Casa Voyageurs', loc: { trackId: 'a', station: 0 } }] },
      catalogue: { entries: [], loading: true }, start: null, end: null, stops: [], margin: '5', marginText: '5%', distribution: 'mareco', picking: null, route: null, run: null,
    }
    const html = de(<RuntimeTrain {...props} />)
    expect(html).toContain('<option value="">Netz der Variante</option>')
    expect(html).toMatch(/<option value="morocco.railjson" selected="">OSRD: morocco<\/option>/)
    expect(html).toContain('Casa Voyageurs')
    expect(html).toContain('ein OSRD-Netz des Servers')
    // without nets on the server, the variant's net alone and no choice
    expect(de(<RuntimeTrain {...props} nets={[]} netId={null} places={{ signals: [], platforms: [] }} />)).not.toContain('>Netz der Variante</option>')
  })

  it('shows the running times and the timetable of a run', () => {
    const html = de(<RuntimeResult t={t} language="de" run={run} stale={false} moment={0} setMoment={() => {}} at={pointAt(run.result.final, 0)} />)
    expect(html).toContain('2:17')   // 130 s × 1.05
    expect(html).toContain('Hausen Gl. 1')
    expect(html).toContain('1 kWh')
  })

  it('lists the routes computed, the shortest and the fastest marked, the one shown chosen', () => {
    const slower = { ...run, result: { ...run.result, final: report(1.3) } }
    const routes = [
      { ...slower, length: 1000, via: null },
      { ...run, length: 1200, via: 'Nord' },
      { length: 1500, via: 'Süd', error: { code: 'internal', message: 'x' } },
    ]
    const html = de(<RuntimeResult t={t} language="de" run={{ ...routes[1], key: 'k' }} routes={routes} choice={1} choose={() => {}} fastest={1} shortest={0}
      stale={false} moment={0} setMoment={() => {}} at={pointAt(run.result.final, 0)} />)
    expect(html).toContain('<span class="ui-grow">Routen</span>')
    expect(html).toContain('Route A')
    expect(html).toContain('Route B über Nord')
    expect(html).toContain('Route C über Süd')
    expect(html).toMatch(/Route A.*kürzeste/)
    expect(html).toMatch(/Route B über Nord.*schnellste/)
    expect(html).toContain('2:49')   // 130 s × 1.3, route A
    expect(html).toContain('Fahrzeit nicht gerechnet')
    expect(html).toMatch(/aria-selected="true"[^>]*>(?:(?!aria-selected).)*Route B/)
    // one route alone: none other found
    expect(de(<RuntimeResult t={t} language="de" run={{ ...routes[0], key: 'k' }} routes={routes.slice(0, 1)} choice={0} choose={() => {}} fastest={0} shortest={0}
      stale={false} moment={0} setMoment={() => {}} at={null} />)).toContain('Keine Alternativroute gefunden')
  })

  it('draws the run over the distance, and under it the height along the same stretch', () => {
    const profile = [{ position: 0, height: 100 }, { position: 600, height: 106 }, { position: 600, height: null }, { position: 1000, height: null }]
    const chart = (r) => de(<RunChart run={r} profile={profile} length={1000} offsets={run.offsets} labels={run.labels} at={null} t={t} language="de" />)
    const html = chart(run)
    expect(html).toContain('polyline')
    // The elements' speed dashed and light, the signalled speed solid over it.
    expect(html).toContain('Elementgeschwindigkeit')
    expect(html).toMatch(/<path d="M[^"]*" class="sim-line-design"/)
    expect(html).toMatch(/<path d="M[^"]*" class="sim-line-signalled"/)
    expect(html).toContain('aria-label="Höhe"')
    expect(html).toContain('class="sim-profile-line"')
    expect(html).toContain('class="sim-profile-gap"')
    // Before the run is simulated, the height alone.
    const before = chart(null)
    expect(before).not.toContain('Elementgeschwindigkeit')
    expect(before).toContain('<span class="sim-chart-title">Höhe</span>')
  })
})

describe('helpers', () => {
  it('formats times and finds the train at a moment', () => {
    expect(formatTime(75)).toBe('1:15')
    expect(formatTime(3725)).toBe('1:02:05')
    const r = report(1)
    expect(pointAt(r, 65)).toMatchObject({ position: 500, speed: 20 })
    expect(pointAt(r, 25).position).toBeGreaterThan(0)
    expect(pointAt(r, 25).position).toBeLessThan(250)   // still accelerating: less than half the way
  })

  it('opens computing a running time by the address, as the Simulation-Tool\'s address leads on to it', () => {
    expect(linkedRequest('?project=p&variant=v&section=runtime')).toMatchObject({ action: 'open', project: 'p', variant: 'v', section: 'runtime' })
    expect(vehicleUrl('osrd:freight-train')).toBe('../vehicles/?vehicle=osrd%3Afreight-train')
  })
})
