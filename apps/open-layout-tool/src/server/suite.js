// The Open-Layout-Tool is a module of Rail-Across-Suite. The account is the
// suite's: signing in, changing the password and signing out happen on the
// suite's start page, which lies beside this app (it at /, the tool at
// /layout/), and send one back here.

const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'

/** The address of a page of the suite's start page (`sign-in`, `password`, or the overview), with where to come back to. */
export function suiteUrl(page = '', next = null) {
  const q = next ? `?${new URLSearchParams({ next })}` : ''
  return `${SUITE}#/${page}${q}`
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
