import { describe, it, expect } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { I18n, textsFor } from './i18n'
import Home from './pages/Home'
import Infrastructure from './pages/Infrastructure'
import Planned from './pages/Planned'
import Simulation from './pages/Simulation'
import Admin from './pages/Admin'
import ProjectCard from './pages/projects/ProjectCard'
import { INFRASTRUCTURE_TOOLS } from './modules'
import { sizeText, startPassword } from './users'

const de = (node) => renderToStaticMarkup(<I18n.Provider value={textsFor('de')}>{node}</I18n.Provider>)
const projects = {
  list: [{ id: 'p1', title: 'PEK Halle–Könnern', variants: [{ id: 'v1', name: 'Bestand', head: { number: 3, author: { name: 'Max' }, createdAt: '2026-10-29T10:00:00Z' } }] }],
  error: false,
}

describe('the start page', () => {
  it('shows two editors and three simulations apart, the infrastructure editor with its tools', () => {
    const html = de(<Home />)
    for (const name of ['Infrastruktur-Editor', 'Fahrzeug-Editor', 'Fahrplan-Sim', 'Kapazitäts-Sim', 'Baubetriebs-Sim']) {
      expect(html).toContain(name)
    }
    expect(html.indexOf('Fahrzeug-Editor')).toBeLessThan(html.indexOf('Simulationen'))
    expect(html.indexOf('Simulationen')).toBeLessThan(html.indexOf('>Fahrplan-Sim<'))
    expect(html).toContain('href="#/infrastructure"')
    for (const tool of ['layout', 'signalling', 'catenary']) expect(html).toContain(`href="#/infrastructure/${tool}"`)
    expect(html).not.toContain('ui-badge')
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

  it('says at a variant how many changes its working copy in this browser holds', () => {
    const card = (localChanges) => de(<ProjectCard project={projects.list[0]} user={{ id: 1, role: 'user' }} tool={INFRASTRUCTURE_TOOLS[0]}
      showArchived={false} localChanges={localChanges} onDialog={() => {}} />)
    expect(card({ v1: 3 })).toContain('3 lokale Änderung(en)')
    expect(card({})).not.toContain('lokale Änderung')
  })

  it('in a tool that is not there yet lists the projects without a way to open them', () => {
    const html = de(<Infrastructure toolId="catenary" user={{ id: 1, role: 'user' }} projects={projects} />)
    expect(html).toContain('Das Catenary-Tool gibt es noch nicht')
    expect(html).not.toContain('layout/?project=')
    expect(html).toContain('disabled=""')
    expect(html).toContain('+ Variante abzweigen')
  })

})

describe('the Fahrplan-Sim', () => {
  it('lists the projects, each variant opening in the Fahrplan-Editor, timetables and running times in one', () => {
    const described = { ...projects, list: [{ ...projects.list[0], description: 'Ausbau der Strecke 6344', members: [{ name: 'Erika' }] }] }
    const html = de(<Simulation route="timetable" projects={described} />)
    // As the infrastructure editor lists them — picture, title, description, who works on it —, without managing them.
    expect(html).toContain('class="ra-project-image"')
    expect(html).toContain('Ausbau der Strecke 6344')
    expect(html).toContain('Erika')
    expect(html).toContain('class="ra-variant-meta"')
    expect(html).toContain('<h2>Vorlagen</h2>')
    expect(html).not.toContain('Variante abzweigen')
    expect(html).not.toContain('aria-label="Projektmenü"')
    expect(html).toContain('PEK Halle–Könnern')
    expect(html).toContain('Rev. 3')
    expect(html).toContain('href="timetable/?project=p1&amp;variant=v1"')
    expect(html).toContain('>Fahrpläne und Fahrzeiten</a>')
    expect(html).not.toContain('simulation/?project=')
    expect(html).toContain('Projekte öffnen im Fahrplan-Editor — Fahrpläne und Fahrzeiten')
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
