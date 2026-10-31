import { describe, it, expect } from 'vitest'
import { EDITORS, SIMULATIONS, hashOf, openUrl, routeOf, safeNext, simulationUrl, toolOf, toolUrl } from './modules'
import { pairDefaults, splitProjects, variantTree } from './projects'
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
    for (const route of [
      { page: '' }, { page: 'infrastructure', tool: 'signalling' }, { page: 'timetable' },
      { page: 'password', next: '/layout/?project=p1&variant=v1' }, { page: 'password', next: null }, { page: 'admin' },
    ]) {
      expect(routeOf(hashOf(route))).toEqual(route)
    }
  })

  it('changes the password for a module and sends one back to it — on this site only', () => {
    expect(routeOf('#/password?next=%2Flayout%2F%3Fproject%3Dp1')).toEqual({ page: 'password', next: '/layout/?project=p1' })
    expect(routeOf('#/sign-in')).toEqual({ page: '' })
    for (const elsewhere of ['https://evil.example/', '//evil.example/', '/\\evil.example', 'layout/', '', null]) {
      expect(safeNext(elsewhere), String(elsewhere)).toBeNull()
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
    expect(openUrl(toolOf('catenary'), 'p1', 'v1')).toBeNull()
  })

  it('opens a variant in the Signalling-Tool, which keeps nothing in the browser', () => {
    expect(openUrl(toolOf('signalling'), 'p1', 'v1')).toBe('signalling/?project=p1&variant=v1')
    expect(toolUrl(toolOf('signalling'), { action: 'storage' })).toBeNull()
    expect(toolUrl(toolOf('layout'), { action: 'storage' })).toBe('layout/?action=storage')
  })
})

describe('asking a tool', () => {
  it('names the action and its variants in the address', () => {
    const layout = toolOf('layout')
    expect(toolUrl(layout, { project: 'p1', variant: 'v1', action: 'history' })).toBe('layout/?project=p1&variant=v1&action=history')
    expect(toolUrl(layout, { project: 'p1', action: 'merge', from: 'a', to: 'b' })).toBe('layout/?project=p1&action=merge&from=a&to=b')
    expect(toolUrl(layout, { action: 'storage' })).toBe('layout/?action=storage')
    expect(toolUrl(toolOf('catenary'), { project: 'p1', action: 'view' })).toBeNull()
  })
})

describe('the projects listed', () => {
  const v = (id, extra = {}) => ({ id, name: id, parentVariantId: null, archived: false, ...extra })
  it('keeps the templates apart, each by title', () => {
    const { regular, templates } = splitProjects([
      { id: 'b', title: 'Zeitz' }, { id: 't', title: 'Vorlage', template: true }, { id: 'a', title: 'Ammendorf' },
    ])
    expect(regular.map(p => p.id)).toEqual(['a', 'b'])
    expect(templates.map(p => p.id)).toEqual(['t'])
  })

  it('shows the variants as a tree, and pairs a variant with its parent', () => {
    const variants = [v('2030', { parentVariantId: 'x' }), v('x'), v('2035', { parentVariantId: '2030' })]
    expect(variantTree(variants).map(r => [r.variant.id, r.depth])).toEqual([['x', 0], ['2030', 1], ['2035', 2]])
    expect(pairDefaults(variants)).toEqual({ from: 'x', to: '2030' })
  })
})

describe('a simulation that is there', () => {
  it('opens a variant by its address, and one still to come nowhere', () => {
    expect(simulationUrl(SIMULATIONS[0], 'p1', 'v1')).toBe('simulation/?project=p1&variant=v1')
    expect(simulationUrl(SIMULATIONS[1], 'p1', 'v1')).toBeNull()
  })
})
