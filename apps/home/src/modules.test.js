import { describe, it, expect } from 'vitest'
import { EDITORS, SIMULATIONS, hashOf, listedProjects, openUrl, routeOf, toolOf } from './modules'
import { TEXTS } from './i18n'

describe('the pages by the address', () => {
  it('reads the overview, the infrastructure editor with its tool, and a module\'s page', () => {
    expect(routeOf('')).toEqual({ page: '' })
    expect(routeOf('#/')).toEqual({ page: '' })
    expect(routeOf('#/infrastructure')).toEqual({ page: 'infrastructure', tool: 'layout' })
    expect(routeOf('#/infrastructure/catenary')).toEqual({ page: 'infrastructure', tool: 'catenary' })
    expect(routeOf('#/infrastructure/nonsense')).toEqual({ page: 'infrastructure', tool: 'layout' })
    expect(routeOf('#/capacity-sim')).toEqual({ page: 'capacity-sim' })
    expect(routeOf('#/elsewhere')).toEqual({ page: '' })
  })

  it('writes the hash every page is read back from', () => {
    for (const route of [{ page: '' }, { page: 'infrastructure', tool: 'signalling' }, { page: 'timetable' }]) {
      expect(routeOf(hashOf(route))).toEqual(route)
    }
  })
})

describe('the tiles', () => {
  it('are three editors and three simulations, each with its name and description in both languages', () => {
    expect(EDITORS.map(m => m.id)).toEqual(['infrastructure', 'vehicles', 'timetable'])
    expect(SIMULATIONS.map(m => m.id)).toEqual(['timetableSim', 'capacitySim', 'constructionSim'])
    for (const texts of Object.values(TEXTS)) {
      for (const m of [...EDITORS, ...SIMULATIONS]) {
        expect(texts[`module_${m.id}`], m.id).toBeTruthy()
        expect(texts[`module_${m.id}_desc`], m.id).toBeTruthy()
      }
      for (const tool of ['layout', 'signalling', 'catenary']) expect(texts[`tool_${tool}`]).toBeTruthy()
    }
  })

  it('the two languages have the same texts', () => {
    expect(Object.keys(TEXTS.en).sort()).toEqual(Object.keys(TEXTS.de).sort())
  })
})

describe('opening a project', () => {
  it('opens a variant in the Layout-Tool beside the start page, naming project and variant', () => {
    expect(openUrl(toolOf('layout'), 'p 1', 'v1')).toBe('layout/?project=p+1&variant=v1')
  })

  it('opens nothing in a tool that is not there yet', () => {
    expect(openUrl(toolOf('signalling'), 'p1', 'v1')).toBeNull()
  })
})

describe('the projects listed', () => {
  const v = (id, extra = {}) => ({ id, name: id, parentVariantId: null, archived: false, ...extra })
  it('leaves out templates, archived variants and projects with none left; the main variant first; by title', () => {
    const listed = listedProjects([
      { id: 'b', title: 'Zeitz', variants: [v('2030', { parentVariantId: 'x' }), v('x')] },
      { id: 't', title: 'Vorlage', template: true, variants: [v('y')] },
      { id: 'a', title: 'Ammendorf', variants: [v('z'), v('old', { archived: true })] },
      { id: 'c', title: 'Gone', variants: [v('w', { archived: true })] },
    ])
    expect(listed.map(p => [p.id, p.variants.map(x => x.id)])).toEqual([['a', ['z']], ['b', ['x', '2030']]])
  })
})
