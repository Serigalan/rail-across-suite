import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { straightElement } from '@rail-across/core/geometry'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'
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
    fields: { catalogue: { id: 'db-ril-301', version: '0.2.0' } },
    collections: {
      signals: [
        { id: 's-a', name: 'A', kind: 'real', function: 'entry', type: 'multiSection', system: 'ks', at: on('t1', 20),
          frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }, { aspect: 'Ks_2' }] }] },
        // A main shunting signal without Sh 1: an error of the check.
        { id: 's-n', name: 'N1', kind: 'real', type: 'mainShunting', system: 'ks', at: on('t2', 80),
          frames: [{ kind: 'screen', aspects: [{ aspect: 'Hp_0' }, { aspect: 'Ks_1' }] }] },
      ],
      detectionPoints: [], trackSections: [], switchElements: [], routes: [], overlaps: [], flankProtection: [], pzbElements: [], stationBoundaries: [],
    },
  },
}

const render = (props = {}) => renderToStaticMarkup(
  <I18n.Provider value={textsFor('de')}>
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
    expect(html).toContain('class="plan-track"')
    expect(html).toContain('<span class="ui-rail-count" aria-hidden="true">1</span>')
    expect(html).toContain('Einchecken')
  })

  it('offers no editing where it only looks', () => {
    const html = render({ readOnly: true })
    expect(html).toContain('Nur ansehen')
    expect(html).not.toContain('Neues Signal')
    expect(html).not.toContain('Einchecken')
  })

  it('reads what the suite asks of it from its address', () => {
    expect(linkedRequest('?project=p&variant=v')).toEqual({ action: 'open', project: 'p', variant: 'v' })
    expect(linkedRequest('?project=p&action=view')).toEqual({ action: 'view', project: 'p', variant: null })
    expect(linkedRequest('?project=p&action=merge&from=a&to=b')).toEqual({ action: 'merge', forward: true })
    expect(linkedRequest('')).toBeNull()
  })
})
