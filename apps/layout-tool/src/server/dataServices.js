// What the project server reads for the Layout-Tool because a browser cannot
// (decision 0019 of the suite): the Länder's DGM1 — heights for points, relief
// tiles for the map — and the conversion of an Access file for the MDB import
// (server/src/routes/terrain.js).

const BASE = import.meta.env?.VITE_OLT_API ?? '/api'

/** A conversion that failed, `code` being the server's error key for the UI to translate. */
export class ServiceError extends Error {
  constructor(code, detail) {
    super(detail || code)
    this.name = 'ServiceError'
    this.code = code
    this.detail = detail
  }
}

/**
 * The URL template of the server's relief tiles (GET /api/dem/{z}/{x}/{y}.png,
 * terrain-RGB from the Länder's DGM1). Absolute, since MapLibre fetches tiles
 * from a worker; on the app's own origin the session cookie goes with them.
 */
export const demTilesUrl = () => `${new URL(`${BASE}/dem/`, window.location.href).href}{z}/{x}/{y}.png`

/**
 * Ground heights from the Länder's DGM1, which only the server can read (the
 * Länder publish tiles without a CORS header). `lngLats` are WGS84
 * [[lng, lat], ...].
 *
 * Resolves with { heights, sources } — per point the height [m] and the dataset
 * it came from, both null where no DGM1 has the point. Never throws: the
 * terrain has sources to fall back on, so a server that cannot be asked, or
 * does not answer within `timeoutMs`, answers null for every point, like one
 * that has no data there.
 */
export async function terrainOnServer(lngLats, { timeoutMs = 60000 } = {}) {
  const none = { heights: lngLats.map(() => null), sources: lngLats.map(() => null) }
  const abort = new AbortController()
  const timer = setTimeout(() => abort.abort(), timeoutMs)
  try {
    const res = await fetch(`${BASE}/terrain`, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ lnglat: lngLats }),
      signal: abort.signal,
    })
    if (!res.ok) return none
    const data = await res.json()
    if (!Array.isArray(data?.heights) || data.heights.length !== lngLats.length) return none
    return { heights: data.heights, sources: data.sources ?? none.sources }
  } catch {
    return none
  } finally {
    clearTimeout(timer)
  }
}

/**
 * Convert an Access file (MDB) into the Satzarten the MDB import reads.
 *
 * The file is sent as it is, not as a form upload — the server takes the raw
 * body, writes it to a temp file, converts and deletes it again. It is a whole
 * database and can be tens of megabytes, so the caller is expected to say so in
 * the UI before this runs: the file leaves the user's machine.
 *
 * Resolves with { points, elements, cants, tracks, nodes, counts }.
 */
export async function convertMdbOnServer(file, { signal } = {}) {
  let res
  try {
    res = await fetch(`${BASE}/mdb`, {
      method: 'POST',
      credentials: 'same-origin',
      // A header of its own, which no form of another site can send (server/src/app.js).
      headers: { 'Content-Type': 'application/octet-stream', 'X-Olt-Upload': '1' },
      body: file,
      signal,
    })
  } catch (err) {
    if (err?.name === 'AbortError') throw err
    throw new ServiceError('unavailable')
  }
  let data = null
  try {
    data = await res.json()
  } catch {
    data = null
  }
  if (!res.ok) throw new ServiceError(data?.error ?? 'unavailable', data?.message)
  if (data === null) throw new ServiceError('unavailable')
  return data
}
