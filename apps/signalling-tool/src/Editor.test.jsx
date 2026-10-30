import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { straightElement } from '@rail-across/core/geometry'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
import BalisesPanel from './panels/BalisesPanel.jsx'
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
    fields: { catalogue: { id: 'db-ril-301', version: '0.3.0' } },
    collections: {
      signals: [
        { id: 's-a', name: 'A', kind: 'real', function: 'entry', logicalSignals: [{ system: 'ks', type: 'multiSection' }], at: on('t1', 20),
          frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Ks_2' }] }] },
        // A main shunting signal without Sh 1: an error of the check.
        { id: 's-n', name: 'N1', kind: 'real', logicalSignals: [{ system: 'ks', type: 'mainShunting' }], at: on('t2', 80),
          frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }] },
      ],
      detectionPoints: [], trackSections: [], switchElements: [], routes: [], overlaps: [], flankProtection: [], balises: [], stationBoundaries: [],
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
  it('shows the rail, the signals and the plan of a variant, the check counting its findings', () => {
    const html = render()
    expect(html).toContain('aria-label="Signale"')
    expect(html).toContain('<span class="ui-list-title">A</span>')
    expect(html).toContain('<span class="ui-list-title">N1</span>')
    expect(html).toContain('class="plan-map"')
    expect(html).toContain('aria-label="Kartenebenen"')
    expect(html).toContain('<span class="ui-rail-count" aria-hidden="true">1</span>')
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
    expect(html).toContain('aria-label="Deutsch"')
    expect(html).not.toContain('Mehrabschnittssignal')
    expect(render()).toContain('aria-label="English"')
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

  it('reads what the suite asks of it from its address', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ action: 'open', project: 'p', variant: 'v' })
    expect(linkedRequest('?project=p&action=view')).toEqual({ action: 'view', project: 'p', variant: null })
    expect(linkedRequest('?project=p&action=merge&from=a&to=b')).toEqual({ action: 'merge', forward: true })
    expect(linkedRequest('')).toBeNull()
  })
})
