// The Simulation-Tool is a module of Rail-Across-Suite: the account and the
// projects are the suite's. Its tile on the start page (Fahrplan-Sim) lists
// the projects and opens a variant here by its address:
// `?project=<id>&variant=<id>` — the main variant without `variant`; it
// simulates the variant as this browser's working copy holds it, where there
// is one, else its head; `&action=view` its head alone. Nothing is written:
// the tool reads the infrastructure. Every way back leads to that tile.

export const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'
export const NOTICE = 'rail-across-notice'

/** The simulation's page on the suite's start page. */
export const projectsUrl = () => `${SUITE}#/timetable-sim`

/** Back to the projects, with a line the suite shows once. */
export function toProjects(note = null) {
  try {
    if (note) sessionStorage.setItem(NOTICE, note)
    else sessionStorage.removeItem(NOTICE)
  } catch { /* not kept: the suite shows nothing */ }
  location.assign(projectsUrl())
}

/** What the address asks: { action, project, variant }, null without a project. */
export function linkedRequest(search = globalThis.location?.search ?? '') {
  const q = new URLSearchParams(search)
  const action = q.get('action') ?? 'open'
  if (!['open', 'view'].includes(action) || !q.get('project')) return null
  return { action, project: q.get('project'), variant: q.get('variant') }
}

const RELOADED = 'ra-sign-in-reload'

/**
 * Without a session: this page loaded again, so the server shows the suite's
 * sign-in in its place, which loads this page once one is signed in — once in
 * ten seconds at the most. False where it did not.
 */
export function signInAgain() {
  let last = 0
  try { last = Number(sessionStorage.getItem(RELOADED) ?? 0) } catch { /* no storage: try once */ }
  if (Date.now() - last < 10000) return false
  try { sessionStorage.setItem(RELOADED, String(Date.now())) } catch { /* not kept */ }
  location.reload()
  return true
}
