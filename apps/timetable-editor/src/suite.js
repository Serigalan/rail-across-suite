// The Fahrplan-Editor is a module of Rail-Across-Suite: the account and the
// projects are the suite's. Its tile on the start page lists the projects and
// opens a variant here by its address: `?project=<id>&variant=<id>` — the
// main variant without `variant`; the trains run over the variant as this
// browser's working copy holds it, where there is one, else its head;
// `&action=view` its head alone. The timetables are the project's, on the
// server; `&timetable=<id>&train=<id>` names the one shown and its train.
// Every way back leads to that tile.

export const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'
export const NOTICE = 'rail-across-notice'

/** The Fahrplan-Editor's page on the suite's start page. */
export const projectsUrl = () => `${SUITE}#/timetable`

/** Back to the projects, with a line the suite shows once. */
export function toProjects(note = null) {
  try {
    if (note) sessionStorage.setItem(NOTICE, note)
    else sessionStorage.removeItem(NOTICE)
  } catch { /* not kept: the suite shows nothing */ }
  location.assign(projectsUrl())
}

/** What the address asks: { action, project, variant, timetable, train }, null without a project. */
export function linkedRequest(search = globalThis.location?.search ?? '') {
  const q = new URLSearchParams(search)
  const action = q.get('action') ?? 'open'
  if (!['open', 'view'].includes(action) || !q.get('project')) return null
  return { action, project: q.get('project'), variant: q.get('variant'), timetable: q.get('timetable'), train: q.get('train') }
}

/** The timetable and train shown, kept in the address — a reload or a link shows them again. */
export function keepInAddress({ timetable, train }) {
  if (!globalThis.history || !globalThis.location) return
  const url = new URL(location.href)
  for (const [k, v] of Object.entries({ timetable, train })) {
    if (v) url.searchParams.set(k, v)
    else url.searchParams.delete(k)
  }
  if (url.href !== location.href) history.replaceState(history.state, '', url)
}

const KEPT_VEHICLE = 'rail-across-timetable-vehicle'

/** The vehicle chosen last in this browser, which a new train gets. */
export const keptVehicle = () => { try { return localStorage.getItem(KEPT_VEHICLE) } catch { return null } }
export const keepVehicle = (key) => { try { localStorage.setItem(KEPT_VEHICLE, key) } catch { /* not kept */ } }

/** A vehicle in the Fahrzeug-Editor, beside this tool. */
export const vehicleUrl = (key) => `${SUITE}vehicles/?${new URLSearchParams({ vehicle: key })}`

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
