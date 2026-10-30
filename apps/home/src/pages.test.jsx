import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { I18n, textsFor } from './i18n'
import Home from './pages/Home'
import Infrastructure from './pages/Infrastructure'
import Planned from './pages/Planned'
import Admin from './pages/Admin'
import { sizeText, startPassword } from './users'

const de = (node) => renderToStaticMarkup(<I18n.Provider value={textsFor('de')}>{node}</I18n.Provider>)
const projects = {
  list: [{ id: 'p1', title: 'PEK Halle–Könnern', variants: [{ id: 'v1', name: 'Bestand', head: { number: 3, author: { name: 'Max' }, createdAt: '2026-10-29T10:00:00Z' } }] }],
  error: false,
}

describe('the start page', () => {
  it('shows the six modules, the editors and the simulations apart', () => {
    const html = de(<Home />)
    for (const name of ['Infrastruktur-Editor', 'Fahrzeug-Editor', 'Fahrplan-Editor', 'Fahrplan-Sim', 'Kapazitäts-Sim', 'Baubetriebs-Sim']) {
      expect(html).toContain(name)
    }
    expect(html.indexOf('Fahrplan-Editor')).toBeLessThan(html.indexOf('<hr'))
    expect(html.indexOf('<hr')).toBeLessThan(html.indexOf('Fahrplan-Sim'))
    expect(html).toContain('href="#/infrastructure"')
  })
})

describe('the infrastructure editor', () => {
  it('shows its three tools as tabs, the chosen one selected, and the projects opening in it', () => {
    const html = de(<Infrastructure toolId="layout" user={{ id: 1, role: 'user' }} projects={projects} />)
    expect(html.match(/role="tab"/g)).toHaveLength(3)
    expect(html).toMatch(/aria-selected="true"[^>]*href="#\/infrastructure\/layout"/)
    expect(html).toContain('Projekte öffnen im Layout-Tool')
    expect(html).toContain('href="layout/?project=p1&amp;variant=v1"')
    expect(html).toContain('href="layout/?project=p1&amp;variant=v1&amp;action=history"')
    expect(html).toContain('+ Neu')
  })

  it('in a tool that is not there yet lists the projects without a way to open them', () => {
    const html = de(<Infrastructure toolId="catenary" user={{ id: 1, role: 'user' }} projects={projects} />)
    expect(html).toContain('Das Catenary-Tool gibt es noch nicht')
    expect(html).not.toContain('layout/?project=')
    expect(html).toContain('disabled=""')
    expect(html).toContain('+ Variante abzweigen')
  })

})

describe('a module still to come', () => {
  it('says what it will be and in which phase', () => {
    const html = de(<Planned route="construction-sim" />)
    expect(html).toContain('Baubetriebs-Sim')
    expect(html).toContain('Roadmap, Phase 5')
  })
})

describe('the users of the suite', () => {
  it('opens for an admin with the way to add a user and the rights of the modules', () => {
    const html = de(<Admin me={{ id: 1, role: 'admin' }} />)
    expect(html).toContain('Nutzerverwaltung')
    expect(html).toContain('Nutzer anlegen')
    expect(html).toContain('Ein Konto für die ganze Suite')
  })

  it('makes start passwords of 16 characters a person can read out', () => {
    for (let i = 0; i < 20; i++) expect(startPassword()).toMatch(/^[a-km-zA-HJ-NP-Z2-9]{16}$/)
  })

  it('says sizes as people read them', () => {
    expect([0, 999, 1500, 25e9].map(sizeText)).toEqual(['0 B', '999 B', '1.5 kB', '25.0 GB'])
  })
})
