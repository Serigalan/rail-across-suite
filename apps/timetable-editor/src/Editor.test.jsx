import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { stationsOf } from '@rail-across/running-times'
import { newPoint, newTrain } from '@rail-across/timetables'
import { BUILT_IN } from '@rail-across/vehicles'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
import TrainSheet from './TrainSheet.jsx'
import Graph from './Graph.jsx'
import { axisOf, insertionIndex, linesOn, trainLine } from './rows.js'
import { linkedRequest } from './suite.js'
import { net } from '../../../packages/running-times/test/net.js'

const de = (node) => renderToStaticMarkup(<I18n.Provider value={textsFor('de')}>{node}</I18n.Provider>)
const { t } = textsFor('de')
const stations = stationsOf(net)

const train = newTrain({ number: '16001', vehicle: 'osrd:emu-regional-train', departure: 7 * 3600, points: [newPoint('A-Dorf'), newPoint('B-Halt', { stop: false }), newPoint('C-Stadt')] })
const rows = [
  { key: 'station:a-dorf', name: 'A-Dorf', offset: 0, point: 0, stop: true, track: 'A1', arrival: null, departure: 25200 },
  { key: 'station:b-halt', name: 'B-Halt', offset: 1400, point: 1, stop: false, track: 'L', arrival: 25290, departure: 25290 },
  { key: 'station:c-stadt', name: 'C-Stadt', offset: 3200, point: 2, stop: true, track: 'C1', arrival: 25395, departure: null },
]
const run = {
  key: 'k', rows,
  route: { path: [{ trackId: 'A1', start: 200, end: 500 }, { trackId: 'L', start: 0, end: 2500 }, { trackId: 'C1', start: 0, end: 400 }], offsets: [0, 1400, 3200], chosen: [{ trackId: 'A1', station: 200 }, { trackId: 'L', station: 1100 }, { trackId: 'C1', station: 400 }] },
  request: { path: { length: 3200 } }, properties: { gaps: { heights: [], speed: [] } }, result: { final: { energy_consumption: 7.2e6 } },
}
const timetables = (patch = {}) => ({
  list: [{ id: 'tt', name: 'Fahrplan 2027' }], loading: false, mayAdd: true, draft: { id: 'tt', name: 'Fahrplan 2027', data: { trains: [train] }, version: 1 },
  conflict: null, ...patch,
})
const catalogue = { loading: false, entries: BUILT_IN }
const sheet = (props = {}) => de(<TrainSheet t={t} language="de" train={train} run={run} timetables={timetables()} stations={stations} catalogue={catalogue}
  record={net} net={{ lines: new Map(), bounds: null }} mayEdit update={() => {}} {...props} />)

describe('the Fahrplan-Editor', () => {
  it('loads the project\'s timetables first', () => {
    const html = de(<Editor project={{ id: 'p', title: 'Strecke' }} variant={{ name: 'Bestand' }} revision={{ number: 3 }} record={net} local={false} />)
    expect(html).toContain('Fahrplan-Editor')
    expect(html).toContain('Wird geladen')
    expect(html).toContain('liest den eingecheckten Stand')
  })

  it('shows a train: what it is, its way from a station to a station, its times with the stations it passes', () => {
    const html = sheet()
    expect(html).toContain('RB 16001')
    expect(html).toContain('Beginn (Bahnhof)')
    // begin and end offer stations only; on the way a halt as well
    const begin = html.slice(html.indexOf('Beginn (Bahnhof)'), html.indexOf('Unterwegs'))
    expect(begin).toContain('A-Dorf')
    expect(begin).not.toContain('B-Halt')
    expect(html).toContain('B-Halt (Haltepunkt)')
    expect(html).toContain('07:00<small>00</small>')
    expect(html).toContain('07:01<small>30</small>')
    expect(html).toContain('Hier halten')
    expect(html).toContain('59 km/h')
  })

  it('offers the itineraries of the signalling as the way a train prefers to the next point, and says where it ran another', () => {
    const range = (trackId, startStation, endStation) => ({ anchor: 'station', trackId, startStation, endStation })
    const signalled = { ...net, signalling: { fields: {}, collections: {
      stoppingPlaces: [{ id: 'hA', name: '1', at: range('A1', 100, 300) }, { id: 'hB', name: 'B', platformId: 'pB', at: range('L', 1000, 1200) }],
      itineraries: [{ id: 'f1', name: 'A-Dorf 1 → B-Halt', startStoppingPlaceId: 'hA', targetStoppingPlaceId: 'hB', routeIds: [], path: { trackIds: ['A1', 'L'] } }],
    } } }
    const html = sheet({ record: signalled })
    expect(html).toContain('Bevorzugter Fahrweg bis B-Halt')
    expect(html).toContain('<option value="" selected="">kürzester Weg</option><option value="f1">A-Dorf 1 → B-Halt</option>')
    expect(html).not.toContain('Bevorzugter Fahrweg bis C-Stadt')
    const preferring = { ...train, points: train.points.map((p, i) => (i === 0 ? { ...p, itinerary: 'f1' } : p)) }
    const other = sheet({ record: signalled, train: preferring, run: { ...run, route: { ...run.route, preferred: ['other', null] } } })
    expect(other).toContain('Der bevorzugte Fahrweg führt nicht dorthin')
  })

  it('says what keeps a train from running', () => {
    const lonely = { ...train, points: [newPoint('A-Dorf'), newPoint('B-Halt')] }
    const html = sheet({ train: lonely, run: { key: 'k', problems: [{ key: 'end_not_station', point: 1 }] } })
    expect(html).toContain('Ein Zug endet nur in einem Bahnhof')
    expect(html).toContain('1 offen')
  })

  it('is read only for a template that is not one\'s own', () => {
    const html = sheet({ mayEdit: false })
    expect(html).not.toContain('Hier halten')
    expect(html).not.toContain('Kopieren')
    expect(html).toMatch(/<select[^>]*disabled/)
  })

  it('offers a first timetable where the project has none', () => {
    const html = sheet({ train: null, timetables: timetables({ list: [], draft: null }) })
    expect(html).toContain('Das Projekt hat noch keinen Fahrplan')
    expect(html).toContain('Neuer Fahrplan')
  })

  it('draws the graphical timetable along a train\'s way', () => {
    const back = { ...train, id: 'b', number: '16002', category: 'RE' }
    const backRows = [...rows].reverse().map((r, i) => ({ ...r, arrival: i ? 26000 + i * 100 : null, departure: i < 2 ? 26000 + i * 100 : null }))
    const runs = new Map([[train.id, run], ['b', { rows: backRows }]])
    const html = de(<Graph t={t} trains={[train, back]} runs={runs} axisTrain={train} view={{ from: null, to: null, scale: 4 }} selected={train.id} onSelect={() => {}} />)
    expect(html).toContain('A-Dorf')
    expect(html).toContain('16001')
    expect(html).toContain('16002')
    expect(html).toContain('07:00')
    expect((html.match(/class="tt-train-line"/g) ?? []).length).toBe(2)
  })
})

describe('what it shows of runs', () => {
  it('lays trains on the axis of one, broken where they leave it', () => {
    const axis = axisOf(rows)
    expect(axis.map(s => [s.name, s.at])).toEqual([['A-Dorf', 0], ['B-Halt', 1400], ['C-Stadt', 3200]])
    expect(linesOn(axis, rows)).toEqual([[{ i: 0, t: 25200 }, { i: 1, t: 25290 }, { i: 2, t: 25395 }]])
    const elsewhere = [rows[0], { ...rows[1], key: 'station:x' }, rows[2]]
    expect(linesOn(axis, elsewhere)).toEqual([])
  })

  it('puts a stop at a station passed among the points, and says a train in a line', () => {
    expect(insertionIndex(run, 1400)).toBe(2)
    expect(insertionIndex(run, 700)).toBe(1)
    expect(trainLine(train, run)).toBe('A-Dorf 07:00 → C-Stadt 07:03')
    expect(trainLine(train, null)).toBe('A-Dorf 07:00 → C-Stadt')
  })

  it('reads its address', () => {
    expect(linkedRequest('?project=p&variant=v&timetable=tt&train=x')).toEqual({ action: 'open', project: 'p', variant: 'v', timetable: 'tt', train: 'x' })
    expect(linkedRequest('?variant=v')).toBeNull()
  })
})
