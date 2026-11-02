import { useCallback, useEffect, useState } from 'react'
import { BUILT_IN } from './builtIn.js'

/**
 * The suite's vehicles on the project server (decision 0014), same origin as
 * the page — the session is an HttpOnly cookie the browser sends by itself —,
 * and with OSRD's example vehicles the catalogue to choose from. An entry:
 * { key, source: 'osrd' | 'suite', vehicle, notes, id, version, mayEdit, createdBy }.
 */

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class VehicleApiError extends Error {
  constructor(status, code, data = null) {
    super(code)
    this.status = status
    this.code = code
    this.data = data
  }
}

async function call(method, path, body) {
  let res
  try {
    res = await fetch(`${BASE}${path}`, {
      method, credentials: 'same-origin',
      // Every writing request carries JSON — a DELETE an empty object —, or the server refuses it.
      headers: { accept: 'application/json', ...(method !== 'GET' ? { 'content-type': 'application/json' } : {}) },
      ...(method !== 'GET' ? { body: JSON.stringify(body ?? {}) } : {}),
    })
  } catch {
    throw new VehicleApiError(0, 'offline')
  }
  if (res.status === 204) return null
  const data = await res.json().catch(() => null)
  if (!res.ok) throw new VehicleApiError(res.status, data?.error ?? 'generic', data)
  return data
}

const enc = encodeURIComponent

/** /api/vehicles: [{ id, version, data, createdBy, mayEdit }]. A save on an older version fails with 409, the vehicle as it is now in `data.vehicle`. */
export const vehicleApi = {
  list: () => call('GET', '/vehicles').then(r => r.vehicles ?? []),
  create: (data) => call('POST', '/vehicles', { data }).then(r => r.vehicle),
  save: (id, version, data) => call('PUT', `/vehicles/${enc(id)}`, { version, data }).then(r => r.vehicle),
  remove: (id) => call('DELETE', `/vehicles/${enc(id)}`),
}

/** A vehicle of the server as an entry. */
export const suiteEntry = (v) => ({
  key: v.id, source: 'suite', vehicle: v.data, notes: [], id: v.id, version: v.version, mayEdit: v.mayEdit, createdBy: v.createdBy,
})

/** The suite's first, then OSRD's, each by name. */
export function sortedEntries(entries) {
  const name = (e) => e.vehicle.name.toLocaleLowerCase()
  return [...entries].sort((a, b) => (a.source === b.source ? name(a).localeCompare(name(b)) : a.source === 'suite' ? -1 : 1))
}

/**
 * The catalogue: { entries, loading, error, create, save, remove, reload }.
 * The suite's vehicles are loaded once, and again when the page comes back
 * into view with `refreshOnShow` — a vehicle changed in the Fahrzeug-Editor
 * meanwhile. A save on a vehicle someone else saved meanwhile fails with code
 * `stale_version`; the catalogue then holds theirs.
 */
export function useCatalogue({ refreshOnShow = false } = {}) {
  const [own, setOwn] = useState(null)
  const [error, setError] = useState(null)

  const reload = useCallback(() => vehicleApi.list().then(
    list => { setOwn(list.map(suiteEntry)); setError(null) },
    err => { setOwn(o => o ?? []); setError(err.code ?? 'generic') },
  ), [])

  useEffect(() => {
    reload()
    if (!refreshOnShow) return undefined
    const onShow = () => { if (document.visibilityState === 'visible') reload() }
    document.addEventListener('visibilitychange', onShow)
    return () => document.removeEventListener('visibilitychange', onShow)
  }, [reload, refreshOnShow])

  const create = useCallback(async (vehicle) => {
    const entry = suiteEntry(await vehicleApi.create(vehicle))
    setOwn(list => [...(list ?? []), entry])
    return entry
  }, [])

  const save = useCallback(async (entry, vehicle) => {
    try {
      const saved = suiteEntry(await vehicleApi.save(entry.id, entry.version, vehicle))
      setOwn(list => list.map(e => (e.key === saved.key ? saved : e)))
      return saved
    } catch (err) {
      if (err.status === 409 && err.data?.vehicle) {
        const theirs = suiteEntry(err.data.vehicle)
        setOwn(list => list.map(e => (e.key === theirs.key ? theirs : e)))
      }
      throw err
    }
  }, [])

  const remove = useCallback(async (entry) => {
    await vehicleApi.remove(entry.id)
    setOwn(list => list.filter(e => e.key !== entry.key))
  }, [])

  return { entries: sortedEntries([...(own ?? []), ...BUILT_IN]), loading: own == null, error, create, save, remove, reload }
}
