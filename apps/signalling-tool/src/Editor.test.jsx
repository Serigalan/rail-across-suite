import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { straightElement } from '@rail-across/core/geometry'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
import BalisesPanel from './panels/BalisesPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'
import SignalsPanel from './panels/SignalsPanel.jsx'
import RoutesPanel from './panels/RoutesPanel.jsx'
import { logicalId } from './model/domain.js'
import { linkedRequest } from './suite.js'

const EPSG = 25832
const P = (x, y = 0) => ({ easting: 500000 + x, northing: 5700000 + y, zone: EPSG })
const track = (id, a, b) => ({ id, name: id, epsg: EPSG, elements: [straightElement(P(...a), P(...b))] })
const on = (trackId, station, direction = 'up') => ({ anchor: 'station', trackId, station, side: 'right', direction })

const record = {
  id: 'b3c4d5e6-0000-4000-8000-000000000001', title: 'Testdorf',
  tracks: [track('t1', [0, 0], [200, 0]), track('t2', [200, 0], [400, 0])],
  switches: [], platforms: [],
  signalling: {
    fields: { catalogue: { id: 'db-ril-301', version: '0.4.0' } },
    collections: {
      signals: [
        { id: 's-a', name: 'A', kind: 'real', at: on('t1', 20, 'up'),
          logicalSignals: [{ system: 'ks', type: 'multiSection', function: 'entry', frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Ks_2' }] }],
            signallingSpeeds: [{ routeId: 'r-a-n', speed: 80 }] }] },
        // A main shunting signal without Sh 1: an error of the check.
        { id: 's-n', name: 'N1', kind: 'real', at: on('t2', 80),
          logicalSignals: [{ system: 'ks', type: 'mainShunting', function: 'exit', frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }] }] },
      ],
      detectionPoints: [], trackSections: [], switchElements: [], overlaps: [], flankProtection: [], balises: [], stationBoundaries: [],
      routes: [{ id: 'r-a-n', kind: 'train', trainType: 'ZZ', startSignalId: 's-a', targetSignalId: 's-n', path: { trackIds: ['t1', 't2'] } }],
    },
  },
}

const render = (props = {}, language = 'de') => renderToStaticMarkup(
  <I18n.Provider value={{ ...textsFor(language), setLanguage: () => {} }}>
    <Editor project={{ id: record.id, title: record.title }} variant={{ id: 'v1', name: 'Bestand' }}
      head={{ revision: { id: 1, number: 1 }, record }} readOnly={false} {...props} />
  </I18n.Provider>,
)

describe('the Signalling-Tool', () => {
  it('shows the rail, the places and the plan of a variant, the check counting its findings', () => {
    const html = render()
    expect(html).toContain('aria-label="Standorte"')
    expect(html).toContain('<span class="ui-list-title">A</span>')
    expect(html).toContain('<span class="ui-list-title">N1</span>')
    expect(html).toContain('class="plan-map"')
    expect(html).toContain('aria-label="Kartenebenen"')
    expect(html).toContain('<span class="ui-rail-count" aria-hidden="true">')
    expect(html).toContain('Einchecken')
  })

  it('names its rail as the Layout-Tool does: the signal and the tool, the history, the way to the infrastructure editor', () => {
    const html = render()
    expect(html).toContain('<span class="ui-rail-brand-name">Signalling-Tool</span>')
    expect(html).toContain('<span class="ui-rail-label">Signale</span>')
    expect(html).toContain('aria-label="Rückgängig"')
    expect(html).toContain('aria-label="Verlauf"')
    expect(html).toContain('aria-label="Wiederholen"')
    expect(html).toContain('<span class="ui-rail-label">Infrastruktur-Editor</span>')
    expect(html).not.toContain('Startseite')
  })

  it('offers no editing where it only looks', () => {
    const html = render({ readOnly: true })
    expect(html).toContain('Nur ansehen')
    expect(html).not.toContain('Neues Signal')
    expect(html).not.toContain('Einchecken')
  })

  it('says everything in the language set, the catalogue\'s titles too, and offers the other one', () => {
    const html = render({}, 'en')
    expect(html).toContain('<span class="ui-rail-label">Signals</span>')
    expect(html).toContain('<span class="ui-rail-label">Balises/magnets</span>')
    expect(html).toContain('Ks multi-section signal')
    expect(html).toContain('aria-label="Settings"')
    expect(html).not.toContain('Mehrabschnittssignal')
  })

  it('orders its rail as the Layout-Tool does: the map layers first, the settings at the foot', () => {
    const html = render()
    expect(html.indexOf('aria-label="Kartenebenen"')).toBeLessThan(html.indexOf('aria-label="Signale"'))
    expect(html.indexOf('aria-label="Infrastruktur-Editor"')).toBeLessThan(html.indexOf('aria-label="Einstellungen"'))
    const settings = renderToStaticMarkup(<SettingsPanel {...textsFor('en')} setLanguage={() => {}} />)
    expect(settings).toContain('<option value="de">Deutsch</option>')
    expect(settings).toContain('<option value="en" selected="">English</option>')
  })

  it('edits a balise or magnet by the settings its system declares, in the language set', () => {
    const balise = { id: 'b1', at: on('t1', 20), system: 'pzb', kind: 'speedCheck', settings: { frequency: '2000', testSpeed: 50, measuringLength: 21 }, effect: 'switchedBySignal', signalId: 's-a' }
    const domain = { ...record.signalling, collections: { ...record.signalling.collections, balises: [balise] } }
    const panel = (language) => renderToStaticMarkup(
      <I18n.Provider value={textsFor(language)}>
        <BalisesPanel domain={domain} net={{ tracks: record.tracks, switches: [] }} findings={{ errors: [], warnings: [] }}
          selection={{ collection: 'balises', id: 'b1' }} select={() => {}} ops={{}} setMode={() => {}} readOnly={false} {...textsFor(language)} />
      </I18n.Provider>,
    )
    const en = panel('en')
    expect(en).toContain('Speed check · 2000 Hz')
    expect(en).toContain('Test speed [km/h]')
    expect(en).toContain('Measuring length [m]')
    expect(en).toContain('INA (platform)')
    expect(panel('de')).toContain('Geschwindigkeitsprüfung · 2000 Hz')
  })

  it('has the six sections in the order of the work, the switches with their flank protection', () => {
    const html = render()
    const at = (label) => html.indexOf(`<span class="ui-rail-label">${label}</span>`)
    const order = ['Standorte', 'Signale', 'Gleisfreimeldung', 'Balisen/Magnete', 'Weichen', 'Fahrstraßen'].map(at)
    expect(order.every(i => i > 0)).toBe(true)
    expect([...order].sort((a, b) => a - b)).toEqual(order)
    expect(html).not.toContain('Geschwindigkeiten')
    expect(html).not.toContain('>Flankenschutz<')
  })

  it('says how many changes the working copy holds and offers to update, discard and check in', () => {
    const base = structuredClone(record)
    const copy = { base: { id: 1, number: 1 }, basePayload: base, idLog: [], project: { ...record, signalling: { ...record.signalling, collections: { ...record.signalling.collections, signals: record.signalling.collections.signals.slice(0, 1) } } } }
    const html = render({ copy })
    expect(html).toContain('1 lokale Änderung(en)')
    expect(html).toContain('Aktualisieren')
    expect(html).toContain('Verwerfen')
    expect(render()).toContain('alles eingecheckt')
  })

  it('edits a logical signal: its place, kind, function, screen and the speed it signals per route', () => {
    const panel = (language) => renderToStaticMarkup(
      <I18n.Provider value={textsFor(language)}>
        <SignalsPanel domain={record.signalling} net={{ tracks: record.tracks, switches: [] }} findings={{ errors: [], warnings: [] }}
          selection={{ collection: 'logicalSignals', id: logicalId('s-a', 'ks') }} select={() => {}} ops={{}} setMode={() => {}} readOnly={false} {...textsFor(language)} />
      </I18n.Provider>,
    )
    const de = panel('de')
    expect(de).toContain('A · Ks-Mehrabschnittssignal')
    expect(de).toContain('Signalisierte Geschwindigkeit je Fahrstraße')
    expect(de).toContain('value="80"')
    expect(de).toContain('Einfahrsignal')
    expect(panel('en')).toContain('Signalled speed per route')
  })

  it('shows a route\'s signalled speed and offers to determine its overlap from it', () => {
    const html = renderToStaticMarkup(
      <I18n.Provider value={textsFor('de')}>
        <RoutesPanel domain={record.signalling} net={{ tracks: record.tracks, switches: [] }} findings={{ errors: [], warnings: [] }} extents={new Map()}
          selection={{ collection: 'routes', id: 'r-a-n' }} select={() => {}} ops={{}} setMode={() => {}} readOnly={false} {...textsFor('de')} />
      </I18n.Provider>,
    )
    expect(html).toContain('80 km/h')
    expect(html).toContain('Durchrutschweg ermitteln')
    expect(html).toContain('Am Signal einstellen')
  })

  it('reads what the suite asks of it from its address', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ action: 'open', project: 'p', variant: 'v' })
    expect(linkedRequest('?project=p&action=view')).toEqual({ action: 'view', project: 'p', variant: null })
    expect(linkedRequest('?project=p&action=merge&from=a&to=b')).toEqual({ action: 'merge', forward: true })
    expect(linkedRequest('')).toBeNull()
  })
})
