// The Signalling-Tool is a module of Rail-Across-Suite: the account and the
// projects are the suite's. The suite asks it by its address what to do
// (apps/README.md): `?project=<id>&variant=<id>` a variant to work on — the
// main one without `variant` —, `&action=view` or `history` its head
// read-only. Comparing and merging two variants is the Layout-Tool's, for the
// whole project with its signalling: those addresses go on to it. Every way
// back leads to the suite's projects with this tool chosen.

export const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'
export const NOTICE = 'rail-across-notice'

/** The projects of the infrastructure on the suite's start page, with this tool chosen. */
export const projectsUrl = () => `${SUITE}#/infrastructure/signalling`

/** Back to the projects, with a line the suite shows once. */
export function toProjects(note = null) {
  try {
    if (note) sessionStorage.setItem(NOTICE, note)
    else sessionStorage.removeItem(NOTICE)
  } catch { /* not kept: the suite shows nothing */ }
  location.assign(projectsUrl())
}

/** What the address asks: { action, project, variant }, { forward } for the Layout-Tool's, null without a project. */
export function linkedRequest(search = globalThis.location?.search ?? '') {
  const q = new URLSearchParams(search)
  const action = q.get('action') ?? 'open'
  if (action === 'compare' || action === 'merge') return { action, forward: true }
  if (!['open', 'view', 'history'].includes(action) || !q.get('project')) return null
  return { action, project: q.get('project'), variant: q.get('variant') }
}

const RELOADED = 'ra-sign-in-reload'

/**
 * Without a session: this page loaded again, so the server shows the suite's
 * sign-in in its place, which loads this page once one is signed in. Run on
 * its own, without the suite's server in front, that would load forever —
 * once in ten seconds is the most it tries. False where it did not.
 */
export function signInAgain() {
  let last = 0
  try { last = Number(sessionStorage.getItem(RELOADED) ?? 0) } catch { /* no storage: try once */ }
  if (Date.now() - last < 10000) return false
  try { sessionStorage.setItem(RELOADED, String(Date.now())) } catch { /* not kept */ }
  location.reload()
  return true
}
