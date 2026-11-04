// The Simulation-Tool was the Fahrplan-Sim's computing of one train's running
// time; it is now a section of the Fahrplan-Editor (../timetable/, Fahrzeit
// rechnen). This page stays for the addresses that name it, and sends them on.

export const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'

/**
 * Where an address of the Simulation-Tool leads now: the Fahrplan-Editor's
 * computing of a running time, on the same project and variant — or its head
 * alone (`action=view`) —; the Fahrplan-Sim's page on the start page without
 * a project.
 */
export function forwardUrl(search = globalThis.location?.search ?? '') {
  const q = new URLSearchParams(search)
  if (!q.get('project')) return `${SUITE}#/timetable`
  const to = new URLSearchParams({ project: q.get('project'), section: 'runtime' })
  if (q.get('variant')) to.set('variant', q.get('variant'))
  if (q.get('action') === 'view') to.set('action', 'view')
  return `${SUITE}timetable/?${to}`
}
