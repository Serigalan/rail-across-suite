// The Layout-Tool is a module of Rail-Across-Suite. The account is the
// suite's: nothing of the suite opens without a session — the server shows
// the suite's sign-in page in its place (deploy/templates/Caddyfile) — and the
// password, the users and signing out are the suite's start page's, which lies
// beside this app (it at /, the tool at /layout/) and sends one back here.
// So are the projects: the suite's infrastructure editor makes, imports and
// manages them for every tool, and asks this one by its address to work on a
// variant, look at it, show its history, compare or merge two (apps/README.md).

const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'

/** The address of a page of the suite's start page (`sign-in`, `password`, or the overview), with where to come back to. */
function suiteUrl(page = '', next = null) {
  const q = next ? `?${new URLSearchParams({ next })}` : ''
  return `${SUITE}#/${page}${q}`
}

/** The projects of the infrastructure on the suite's start page, with this tool chosen. */
export const projectsUrl = () => suiteUrl('infrastructure/layout')

const NOTICE = 'rail-across-notice'

/**
 * Back to the projects on the suite's start page — with `note`, a line it
 * shows once (a merge done, a link to a project that is not there), kept for
 * this tab on the same site.
 */
export function toProjects(note = null, { replace = false } = {}) {
  try {
    if (note) sessionStorage.setItem(NOTICE, note)
    else sessionStorage.removeItem(NOTICE)
  } catch { /* not kept: the suite shows nothing */ }
  if (replace) location.replace(projectsUrl())
  else location.assign(projectsUrl())
}

/** This page's address on this site — where the suite sends one back to. */
const here = () => location.pathname + location.search + location.hash

/**
 * Whether the suite's start page is somewhere else than this app. Run on its
 * own (`npm run dev`), the tool is where the suite would be, and sending one
 * there would send one round in a circle.
 */
export const suiteBeside = () => new URL(SUITE, location.href).pathname !== location.pathname

/** To a page of the suite, and back here afterwards. */
export const toSuite = (page, { back = true } = {}) => location.assign(suiteUrl(page, back ? here() : null))

const RELOADED = 'ra-sign-in-reload'

/**
 * Without a session (it ended, or was ended elsewhere): this page loaded
 * again, so the server shows the sign-in page in its place, which loads this
 * page once one is signed in. Run on its own, without the suite's server in
 * front, that would load forever — once in ten seconds is the most it tries.
 * False where it did not.
 */
export function signInAgain() {
  let last = 0
  try { last = Number(sessionStorage.getItem(RELOADED) ?? 0) } catch { /* no storage: try once */ }
  if (Date.now() - last < 10000) return false
  try { sessionStorage.setItem(RELOADED, String(Date.now())) } catch { /* not kept */ }
  location.reload()
  return true
}
