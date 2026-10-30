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

const TOOL_ACTIONS = ['view', 'history', 'compare', 'merge', 'storage']

/**
 * What the suite asks a tool to do with a project, by the tool's address —
 * the same for every tool of the infrastructure (apps/README.md):
 *
 *   ?project=<id>&variant=<id>                  work on the variant (the main one without `variant`)
 *   ?project=<id>&variant=<id>&action=view      look at its head, read-only
 *   ?project=<id>&variant=<id>&action=history   its revisions
 *   ?project=<id>&action=compare&from=<a>&to=<b>  two variants compared
 *   ?project=<id>&action=merge&from=<a>&to=<b>    a's changes merged into b
 *   ?action=storage                             what the tool keeps in this browser
 *
 * The projects themselves — creating, importing, exporting, who works on
 * them, branching, renaming and archiving variants — are the suite's. Null
 * for a tool that is not there yet.
 */
export function toolUrl(tool, { project, variant, action, from, to } = {}) {
  if (!tool.available) return null
  if (action && !TOOL_ACTIONS.includes(action)) throw new Error(`unknown tool action: ${action}`)
  const q = new URLSearchParams()
  if (project) q.set('project', project)
  if (variant) q.set('variant', variant)
  if (action) q.set('action', action)
  if (from) q.set('from', from)
  if (to) q.set('to', to)
  const query = String(q)
  return query ? `${tool.path}?${query}` : tool.path
}

/** Where a variant of a project opens in a tool to be worked on. */
export const openUrl = (tool, projectId, variantId) => toolUrl(tool, { project: projectId, variant: variantId })

/**
 * A note a tool leaves for the start page when it sends one back — that a
 * merge is done, that a link named a project that is not there: kept for
 * this tab (sessionStorage, the same site) and shown once.
 */
const NOTICE = 'rail-across-notice'

export function takeNotice() {
  try {
    const text = sessionStorage.getItem(NOTICE)
    sessionStorage.removeItem(NOTICE)
    return text || null
  } catch {
    return null
  }
}

/**
 * The suite's own pages: the password (every module sends its users here to
 * change it, and back), and the user administration. Signing in is the
 * sign-in page's (sign-in/), which the server shows before anything else.
 */
export const ACCOUNT_PAGES = ['password']
export const SUITE_PAGES = [...ACCOUNT_PAGES, 'admin']

/**
 * Where to go after changing the password, from `next`: a path
 * on this site (a module, `/layout/?project=…`), never another site.
 */
export function safeNext(next) {
  if (typeof next !== 'string' || !next) return null
  return next.startsWith('/') && !next.startsWith('//') && !next.includes('\\') ? next : null
}

/**
 * The page a location hash names: { page, tool, next } — `#/` the start,
 * `#/infrastructure/signalling` the infrastructure editor in that mode,
 * `#/capacity-sim` a module's own page, `#/password?next=/layout/` changing
 * the password and back to where one came from, `#/admin` the users.
 */
export function routeOf(hash) {
  const [path, query = ''] = String(hash ?? '').replace(/^#\/?/, '').split('?')
  const [page = '', tool] = path.split('/')
  if (page === 'infrastructure') return { page, tool: toolOf(tool).id }
  if (ACCOUNT_PAGES.includes(page)) return { page, next: safeNext(new URLSearchParams(query).get('next')) }
  if (page === 'admin') return { page }
  const module = MODULES.find(m => m.route === page)
  return module ? { page: module.route } : { page: '' }
}

/** The hash of a page (for the infrastructure editor with its tool, for an account page with where it goes back to). */
export function hashOf({ page, tool, next }) {
  if (page === 'infrastructure' && tool) return `#/${page}/${tool}`
  if (ACCOUNT_PAGES.includes(page) && next) return `#/${page}?${new URLSearchParams({ next })}`
  return `#/${page ?? ''}`
}
