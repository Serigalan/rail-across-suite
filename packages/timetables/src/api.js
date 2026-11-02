/**
 * A project's timetables on the project server (decision 0016), same origin
 * as the page — the session is an HttpOnly cookie the browser sends by
 * itself. A timetable: { id, projectId, name, version, data: { trains },
 * createdBy, updatedBy, updatedAt, mayEdit }. A save names the version it
 * changed; on an older one it fails with 409, the timetable as it is now in
 * `data.timetable`.
 */

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class TimetableApiError extends Error {
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
      headers: { accept: 'application/json', ...(body !== undefined ? { 'content-type': 'application/json' } : {}) },
      ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
    })
  } catch {
    throw new TimetableApiError(0, 'offline')
  }
  if (res.status === 204) return null
  const data = await res.json().catch(() => null)
  if (!res.ok) throw new TimetableApiError(res.status, data?.error ?? 'generic', data)
  return data
}

const enc = encodeURIComponent

export const timetableApi = {
  /** { timetables, mayEdit } — whether one may add and change the project's. */
  list: (projectId) => call('GET', `/projects/${enc(projectId)}/timetables`),
  create: (projectId, name, data) => call('POST', `/projects/${enc(projectId)}/timetables`, { name, data }).then(r => r.timetable),
  save: (id, version, name, data) => call('PUT', `/timetables/${enc(id)}`, { version, name, data }).then(r => r.timetable),
  remove: (id) => call('DELETE', `/timetables/${enc(id)}`),
}
