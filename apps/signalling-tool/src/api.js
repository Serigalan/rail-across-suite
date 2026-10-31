// The suite's project server, same origin as the page: the session is an
// HttpOnly cookie the browser sends by itself, and every writing request says
// it carries JSON. Revisions travel as core 1.0 documents; a record goes out
// as one (projectDocument) and comes back through the core's door
// (parseProjectsPayload), as in every module.
import { parseProjectsPayload, projectDocument } from '@rail-across/core/project'

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class ApiError extends Error {
  constructor(status, code, body = null) {
    super(code)
    this.status = status
    this.code = code
    this.body = body
  }
}

async function request(method, path, body) {
  let res
  try {
    res = await fetch(`${BASE}${path}`, {
      method,
      credentials: 'same-origin',
      headers: method === 'GET' ? { accept: 'application/json' } : { 'content-type': 'application/json', accept: 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch {
    throw new ApiError(0, 'offline')
  }
  if (res.status === 204) return null
  const data = await res.json().catch(() => null)
  if (!res.ok) throw new ApiError(res.status, data?.error ?? 'generic', data)
  return data
}

const enc = encodeURIComponent
const recordOf = (payload) => parseProjectsPayload(payload).projects[0]

export const api = {
  /** The user signed in, or null. */
  me: () => request('GET', '/me').then(r => r.user, err => { if (err.status === 401) return null; throw err }),
  projects: () => request('GET', '/projects').then(r => r.projects ?? []),
  /** A variant's head: { variant, revision, record }. */
  head: (variantId) => request('GET', `/variants/${enc(variantId)}/head`).then(r => ({ ...r, record: recordOf(r.payload) })),
  revision: (id) => request('GET', `/revisions/${enc(id)}`).then(r => ({ ...r, record: recordOf(r.payload) })),
  /** The id logs `head` has that `base` has not. */
  remaps: (head, base) => request('GET', `/revisions/${enc(head)}/remaps?base=${enc(base)}`).then(r => r.remaps ?? []),
  /**
   * Check `record` in on `base`, with the splits and joins since (`remaps`, the
   * working copy's id log): { revision, warnings } — or { stale: head } when
   * someone else checked in first, or { refused: errors } when it brings in
   * errors.
   */
  async checkIn(variantId, { base, message, record, remaps = [] }) {
    try {
      return await request('POST', `/variants/${enc(variantId)}/revisions`, { base, message, payload: projectDocument(record), remaps })
    } catch (err) {
      if (err.status === 409 && err.code === 'stale_base') return { stale: err.body.head }
      if (err.status === 422 && err.code === 'invalid_record') return { refused: err.body.errors ?? [] }
      throw err
    }
  },
}
