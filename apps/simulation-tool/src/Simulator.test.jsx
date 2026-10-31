import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { I18n, textsFor } from './i18n.js'
import Simulator from './Simulator.jsx'
import ResultPanel from './panels/ResultPanel.jsx'
import RunChart from './charts/RunChart.jsx'
import { formatTime, pointAt } from './format.js'
import { linkedRequest } from './suite.js'

const de = (node) => renderToStaticMarkup(<I18n.Provider value={textsFor('de')}>{node}</I18n.Provider>)
const { t } = textsFor('de')

const record = {
  tracks: [{ id: 'T1', epsg: 25832, name: '1', elements: [{ elementType: 0, startNode: [500000, 5600000], endNode: [501000, 5600000], bearing: 90, length: 1000 }] }],
  switches: [],
  platforms: [{ id: 'P1', trackId: 'T1', startStation: 400, endStation: 600, stationName: 'Hausen', code: '1' }],
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
  offsets: [0, 500, 1000], labels: ['Signal A', 'Hausen Gl. 1', 'Ende'], key: 'k',
}

describe('the simulator', () => {
  it('offers the train, its places from the signals and platforms, and the margin', () => {
    const html = de(<Simulator project={{ title: 'Hausen' }} variant={{ name: 'Bestand' }} revision={{ number: 4 }} record={record} local={false} />)
    expect(html).toContain('Regionaltriebzug (Beispiel)')
    expect(html).toContain('Signal A')
    expect(html).toContain('Hausen Gl. 1')
    expect(html).toContain('energiesparend (MARECO)')
    expect(html).toContain('liest den eingecheckten Stand')
    expect(html).toMatch(/<button[^>]*disabled[^>]*>Fahrzeit rechnen/)
  })

  it('shows the running times and the timetable of a run', () => {
    const html = de(<ResultPanel t={t} language="de" run={run} stale={false} moment={0} setMoment={() => {}} at={pointAt(run.result.final, 0)} />)
    expect(html).toContain('2:17')   // 130 s × 1.05
    expect(html).toContain('Hausen Gl. 1')
    expect(html).toContain('1 kWh')
  })

  it('draws the run over the distance', () => {
    const html = de(<RunChart run={run} at={null} t={t} />)
    expect(html).toContain('polyline')
    expect(html).toContain('zulässig')
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

  it('reads what the suite asks by the address', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ action: 'open', project: 'p', variant: 'v' })
    expect(linkedRequest('?project=p&action=view')).toEqual({ action: 'view', project: 'p', variant: null })
    expect(linkedRequest('?action=view')).toBeNull()
  })
})
