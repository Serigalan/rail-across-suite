import { describe, expect, it } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { I18n, textsFor } from './i18n.js'
import VehicleSheet from './VehicleSheet.jsx'
import FleetPanel from './FleetPanel.jsx'
import { BUILT_IN, newVehicle } from '@rail-across/vehicles'
import { parseRows } from './fields.js'
import { requestedVehicle } from './suite.js'

const de = (node) => renderToStaticMarkup(<I18n.Provider value={textsFor('de')}>{node}</I18n.Provider>)
const { t } = textsFor('de')

describe('the vehicle editor', () => {
  const vehicle = { ...newVehicle('Triebzug 425'), metadata: { series: '425' } }
  const editor = (patch) => ({ entry: { key: 'k', source: 'suite', vehicle, mayEdit: true, createdBy: { name: 'Max' }, notes: [] }, draft: null, problems: [], ...patch })

  it('shows a vehicle with its physics and its curve', () => {
    const html = de(<VehicleSheet t={t} language="de" editor={editor()} />)
    expect(html).toContain('Triebzug 425')
    expect(html).toContain('von Max')
    expect(html).toContain('Bearbeiten')
    expect(html).toContain('Für OSRD exportieren')
    expect(html).toContain('15 kV 16,7 Hz')
    expect(html).toContain('Höchstgeschwindigkeit [km/h]')
    expect(html).toContain('>160<')
    expect(html).toContain('<svg')
  })

  it('edits it, its problems at their fields', () => {
    const draft = { vehicle: { ...vehicle, mass: 0 }, entry: null, notes: ['resistance_per_kg'] }
    const html = de(<VehicleSheet t={t} language="de" editor={editor({ draft, dirty: true, problems: [{ field: 'mass', key: 'positive' }] })} />)
    expect(html).toContain('noch nicht gespeichert')
    expect(html).toContain('Größer als null.')
    expect(html).toContain('Aus Leistung erzeugen')
    expect(html).toContain('je kg Fahrzeugmasse')
    expect(html).toMatch(/<input[^>]*type="number"/)
  })
})

describe('the catalogue', () => {
  it('lists the suite\'s and OSRD\'s vehicles under their categories', () => {
    const catalogue = { entries: BUILT_IN, loading: false, error: null }
    const html = de(<FleetPanel t={t} language="de" catalogue={catalogue} editor={{ selected: 'osrd:high-speed-train', draft: null, select: () => {}, create: () => {} }} onImport={() => {}} />)
    expect(html).toContain('Beispiele aus OSRD')
    expect(html).toContain('Hochgeschwindigkeitszug')
    expect(html).toContain('High-speed train')   // its group is open, the vehicle chosen
    expect(html).toContain('Importieren')
  })
})

describe('helpers', () => {
  it('take pasted rows of km/h and kN', () => {
    expect(parseRows('0\t158\n10\t158\n\n30;108\nv kN')).toEqual({ speeds: [0, 10 / 3.6, 30 / 3.6], max_efforts: [158000, 158000, 108000] })
    expect(parseRows('0.0,158.5\n10,5 120').speeds).toEqual([0, 10.5 / 3.6])
  })

  it('read the vehicle the address asks for', () => {
    expect(requestedVehicle('?vehicle=osrd%3Afreight-train')).toBe('osrd:freight-train')
    expect(requestedVehicle('')).toBeNull()
  })
})
