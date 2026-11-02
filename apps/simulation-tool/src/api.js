// The suite's project server, same origin as the page: the session is an
// HttpOnly cookie the browser sends by itself. The tool only reads: the user,
// the projects, a variant's head — a core 1.0 document, through the core's
// door (parseProjectsPayload) as in every module.
import { parseProjectsPayload } from '@rail-across/core/project'

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class ApiError extends Error {
  constructor(status, code) {
    super(code)
    this.status = status
    this.code = code
  }
}

async function get(path) {
  let res
  try {
    res = await fetch(`${BASE}${path}`, { credentials: 'same-origin', headers: { accept: 'application/json' } })
  } catch {
    throw new ApiError(0, 'offline')
  }
  const data = await res.json().catch(() => null)
  if (!res.ok) throw new ApiError(res.status, data?.error ?? 'generic')
  return data
}

const enc = encodeURIComponent

export const api = {
  /** The user signed in, or null. */
  me: () => get('/me').then(r => r.user, err => { if (err.status === 401) return null; throw err }),
  projects: () => get('/projects').then(r => r.projects ?? []),
  /** A variant's head: { variant, revision, record }. */
  head: (variantId) => get(`/variants/${enc(variantId)}/head`).then(r => ({ ...r, record: parseProjectsPayload(r.payload).projects[0] })),
}
