import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { I18n, textsFor } from './i18n.js'
import Editor from './Editor.jsx'

const straight = (id, y, length) => ({
  id, name: id, epsg: 25832,
  elements: [{ elementType: 0, length, bearing: 90, startNode: [500000, 5700000 + y], endNode: [500000 + length, 5700000 + y] }],
})
const record = {
  id: 'p1', title: 'P', tracks: [straight('t1', 0, 1000)], switches: [], platforms: [],
  electrification: { fields: {}, collections: {
    contactWires: [{ id: 'cw1', name: 'Oberleitung Nord', voltage: '15000V' }],
    masts: [0, 600].map(station => ({ id: `m${station}`, kind: 'cantilever', at: { anchor: 'station', trackId: 't1', station } })),
    suspensionPoints: [0, 600].map(station => ({ id: `p${station}`, mastId: `m${station}`, at: { anchor: 'station', trackId: 't1', station }, contactWireId: 'cw1' })),
    sectionInsulators: [],
  } },
}
const head = { revision: { id: 1, number: 3 }, record }

describe('the Catenary-Tool', () => {
  it('lists the contact wires with their voltages, a form for a new stretch, and how much is electrified', () => {
    const html = renderToStaticMarkup(
      <I18n.Provider value={textsFor('de')}>
        <Editor project={{ title: 'P' }} variant={{ id: 'v1', name: 'Hauptvariante' }} head={head} copy={null} readOnly={false} />
      </I18n.Provider>,
    )
    expect(html).toContain('Catenary-Tool')
    expect(html).toContain('Oberleitung Nord · 15 kV 16,7 Hz')
    expect(html).toContain('Abschnitt anlegen')
    expect(html).toContain('Masten')
    expect(html).toContain('0.60 von 1.00 km Gleis elektrifiziert')
    expect(html).toContain('Revision 3')
  })
})
