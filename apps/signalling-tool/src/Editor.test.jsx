import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { straightElement } from '@rail-across/core/geometry'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
import EditPanel from './panels/EditPanel.jsx'
import SettingsPanel from './panels/SettingsPanel.jsx'
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

/** The editing panel, with `selection` chosen. */
const edit = (selection, language = 'de', domain = record.signalling) => renderToStaticMarkup(
  <I18n.Provider value={textsFor(language)}>
    <EditPanel domain={domain} net={{ tracks: record.tracks, switches: [] }} findings={{ errors: [], warnings: [] }} extents={new Map()}
      selection={selection} select={() => {}} ops={{}} setMode={() => {}} readOnly={false} {...textsFor(language)} />
  </I18n.Provider>,
)

const render = (props = {}, language = 'de') => renderToStaticMarkup(
  <I18n.Provider value={{ ...textsFor(language), setLanguage: () => {} }}>
    <Editor project={{ id: record.id, title: record.title }} variant={{ id: 'v1', name: 'Bestand' }}
      head={{ revision: { id: 1, number: 1 }, record }} readOnly={false} {...props} />
  </I18n.Provider>,
)

describe('the Signalling-Tool', () => {
  it('opens at creating signals, with the plan of a variant and the check counting its findings', () => {
    const html = render()
    expect(html).toContain('aria-label="Signale"')
    expect(html).toContain('Signalmast erstellen')
    expect(html).toContain('Tafel erstellen')
    expect(html).toContain('Logisches Signal erstellen')
    expect(html).toContain('Fiktives Signal erstellen')
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

  it('offers no creating and no editing where it only looks, and opens at the overview', () => {
    const html = render({ readOnly: true })
    expect(html).toContain('Nur ansehen')
    expect(html).not.toContain('Signalmast erstellen')
    expect(html).not.toContain('<span class="ui-rail-label">Gleisfreimeldung</span>')
    expect(html).not.toContain('<span class="ui-rail-label">Datenaustausch</span>')
    expect(html).toContain('<h2>Bearbeiten</h2>')
    expect(html).toContain('<span class="ui-group-title">Hauptsignale</span><span class="ui-group-count">2</span>')
    expect(html).not.toContain('Einchecken')
  })

  it('says everything in the language set, the catalogue\'s titles too, and offers the other one', () => {
    const html = render({}, 'en')
    expect(html).toContain('<span class="ui-rail-label">Signals</span>')
    expect(html).toContain('<span class="ui-rail-label">Balises/magnets</span>')
    expect(html).toContain('Create signal mast')
    expect(html).toContain('aria-label="Settings"')
    const logical = edit({ collection: 'logicalSignals', id: logicalId('s-a', 'ks') }, 'en')
    expect(logical).toContain('Ks multi-section signal')
    expect(logical).not.toContain('Mehrabschnittssignal')
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
    const panel = (language) => edit({ collection: 'balises', id: 'b1' }, language, domain)
    const en = panel('en')
    expect(en).toContain('Speed check · 2000 Hz')
    expect(en).toContain('Test speed [km/h]')
    expect(en).toContain('Measuring length [m]')
    expect(en).toContain('INA (platform)')
    expect(panel('de')).toContain('Geschwindigkeitsprüfung · 2000 Hz')
  })

  it('has its sections in the order of the work, in groups the gray line parts', () => {
    const html = render()
    const rail = html.slice(html.indexOf('<nav class="ui-rail"'), html.indexOf('</nav>'))
    const at = (label) => rail.indexOf(`<span class="ui-rail-label">${label}</span>`)
    const order = ['Kartenebenen', 'Signale', 'Gleisfreimeldung', 'Balisen/Magnete', 'Bearbeiten', 'Weichen', 'Fahrstraßen', 'Prüfung', 'Datenaustausch', 'Infrastruktur-Editor', 'Einstellungen'].map(at)
    expect(order.every(i => i > 0)).toBe(true)
    expect([...order].sort((a, b) => a - b)).toEqual(order)
    // A line after the map layers, after balises, after editing, after routes, and above the foot.
    const lines = [...rail.matchAll(/ui-rail-divider/g)].map(m => m.index)
    for (const [before, after] of [[0, 1], [3, 4], [4, 5], [6, 7]]) expect(lines.some(i => i > order[before] && i < order[after])).toBe(true)
    expect(lines.some(i => i > order[1] && i < order[3])).toBe(false)
    expect(rail).not.toContain('Standorte')
    expect(rail).not.toContain('>Flankenschutz<')
  })

  it('edits signals, train detection and balises in one overview under their kinds', () => {
    const html = edit(null)
    expect(html).toContain('<h2>Bearbeiten</h2>')
    expect(html).toContain('<span class="ui-options-heading">Signale</span>')
    expect(html).toContain('aria-label="Suchen: Name, Gleis, Art …"')
    expect(html).toContain('<span class="ui-group-title">Hauptsignale</span><span class="ui-group-count">2</span>')
    // A signal mast opens in its editor, the way back leading to the overview; its logical signals open from it.
    const mast = edit({ collection: 'signals', id: 's-a' })
    expect(mast).toContain('<h2>Signalmast</h2>')
    expect(mast).toContain('Zurück</button>')
    expect(mast).toContain('Ks-Mehrabschnittssignal')
    const logical = edit({ collection: 'logicalSignals', id: logicalId('s-a', 'ks') })
    expect(logical).toContain('<h2>Logisches Signal</h2>')
    expect(logical).toMatch(/class="ui-back"[^>]*>.*A<\/button>/)
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

  it('edits a logical signal: its mast, kind, function, screen and the speed it signals per route', () => {
    const panel = (language) => edit({ collection: 'logicalSignals', id: logicalId('s-a', 'ks') }, language)
    const de = panel('de')
    expect(de).toContain('A · Ks-Mehrabschnittssignal')
    expect(de).toContain('Signalisierte Geschwindigkeit je Fahrstraße')
    expect(de).toContain('value="80"')
    expect(de).toContain('Einfahrsignal')
    expect(panel('en')).toContain('Signalled speed per route')
  })

  it('lists the routes under a menu, and shows a route\'s signalled speed and offers to determine its overlap from it', () => {
    const menu = renderToStaticMarkup(
      <I18n.Provider value={textsFor('de')}>
        <RoutesPanel domain={record.signalling} net={{ tracks: record.tracks, switches: [] }} findings={{ errors: [], warnings: [] }} extents={new Map()}
          selection={null} select={() => {}} ops={{}} setMode={() => {}} readOnly={false} {...textsFor('de')} />
      </I18n.Provider>,
    )
    expect(menu).toContain('Fahrstraße erstellen')
    expect(menu).toContain('Durchrutschweg erstellen')
    expect(menu).toContain('<span class="ui-grow">Fahrstraßen</span><span class="ui-option-end">1</span>')
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
