// The Fahrzeug-Editor is a module of Rail-Across-Suite: the account is the
// suite's, the vehicles a catalogue of the suite beside the projects
// (decision 0014). Its tile on the start page opens it; `?vehicle=<key>`
// opens one vehicle — the Fahrplan-Sim links a train's vehicle so. The way
// back leads to the start page.

export const SUITE = import.meta.env?.VITE_RA_SUITE ?? '../'

/** The suite's start page. */
export const suiteUrl = () => SUITE

/** The vehicle the address asks for (`?vehicle=<key>`), or null. */
export const requestedVehicle = (search = globalThis.location?.search ?? '') => new URLSearchParams(search).get('vehicle')

/** The address of a vehicle, kept in the bar so the page can be linked and reloaded. */
export function keepVehicleInAddress(key) {
  try {
    const url = new URL(location.href)
    if (key) url.searchParams.set('vehicle', key)
    else url.searchParams.delete('vehicle')
    history.replaceState(null, '', url)
  } catch { /* not kept */ }
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
