// What the suite is made of, as the start page shows it: the editors, which
// write the shared description of the railway, and the simulations, which
// read it — and of the infrastructure editor its three tools. A module that
// is not there yet has its page all the same, saying when it comes (ROADMAP.md).

/**
 * The tiles of the start page, in two rows. `route` is the page a tile
 * opens; `available` whether there is anything to work with yet; `phase` the
 * phase of the roadmap that brings it.
 */
export const EDITORS = [
  { id: 'infrastructure', route: 'infrastructure', available: true, phase: 1 },
  { id: 'vehicles', route: 'vehicles', available: false, phase: 4 },
  { id: 'timetable', route: 'timetable', available: false, phase: 4 },
]

export const SIMULATIONS = [
  { id: 'timetableSim', route: 'timetable-sim', available: false, phase: 4 },
  { id: 'capacitySim', route: 'capacity-sim', available: false, phase: 4 },
  { id: 'constructionSim', route: 'construction-sim', available: false, phase: 5 },
]

export const MODULES = [...EDITORS, ...SIMULATIONS]

/**
 * The tools of the infrastructure editor — the mode a project opens in.
 * `path` is where an available tool's app lies beside the start page.
 */
export const INFRASTRUCTURE_TOOLS = [
  { id: 'layout', available: true, path: 'layout/' },
  { id: 'signalling', available: false, phase: 3 },
  { id: 'catenary', available: false, phase: 3 },
]

export const DEFAULT_TOOL = 'layout'

/** The tool of that id, or the default one. */
export const toolOf = (id) => INFRASTRUCTURE_TOOLS.find(t => t.id === id) ?? INFRASTRUCTURE_TOOLS.find(t => t.id === DEFAULT_TOOL)

/**
 * Where a variant of a project opens in a tool: the tool's app with the
 * project and the variant named, so it opens that variant at once. Null for a
 * tool that is not there yet.
 */
export function openUrl(tool, projectId, variantId) {
  if (!tool.available) return null
  const q = new URLSearchParams({ project: projectId, ...(variantId ? { variant: variantId } : {}) })
  return `${tool.path}?${q}`
}

/**
 * The page a location hash names: { page, tool } — `#/` the start, `#/infrastructure/signalling`
 * the infrastructure editor in that mode, `#/capacity-sim` a module's own page.
 */
export function routeOf(hash) {
  const [page = '', tool] = String(hash ?? '').replace(/^#\/?/, '').split('/')
  if (page === 'infrastructure') return { page, tool: toolOf(tool).id }
  const module = MODULES.find(m => m.route === page)
  return module ? { page: module.route } : { page: '' }
}

/** The hash of a page (and, for the infrastructure editor, its tool). */
export const hashOf = ({ page, tool }) => `#/${page ?? ''}${page === 'infrastructure' && tool ? `/${tool}` : ''}`

/**
 * The projects as the infrastructure editor lists them: those that are not
 * templates, each with its variants that are not archived, root variants
 * first (the main one — "Bestand" — before those branched off it).
 */
export function listedProjects(projects) {
  return (projects ?? [])
    .filter(p => !p.template)
    .map(p => ({
      ...p,
      variants: (p.variants ?? []).filter(v => !v.archived)
        .sort((a, b) => Number(Boolean(a.parentVariantId)) - Number(Boolean(b.parentVariantId))),
    }))
    .filter(p => p.variants.length)
    .sort((a, b) => String(a.title).localeCompare(String(b.title), 'de', { sensitivity: 'base' }))
}
