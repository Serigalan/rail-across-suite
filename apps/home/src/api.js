// The suite's project server, same origin as the page: the session is an
// HttpOnly cookie the browser sends by itself, and every writing request says
// it carries JSON (the server refuses it otherwise). The start page holds what
// is the suite's — signing in and out, the password, the users and their
// rights — and the projects of the infrastructure: creating, importing and
// exporting them, who works on them, their variants. What a tool does with a
// variant — working on it, its history, comparing, merging — is the tool's.
//
// Revisions travel as core 1.0 documents; a project's record goes out as one
// here (projectDocument) and comes back through the core's door
// (parseProjectsPayload), as in every module.
import { parseProjectsPayload, projectDocument } from '@rail-across/core/project'

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class ApiError extends Error {
  constructor(status, code) {
    super(code)
    this.status = status
    this.code = code
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
  if (!res.ok) throw new ApiError(res.status, data?.error ?? 'generic')
  return data
}

const enc = encodeURIComponent

/**
 * A new project's record as its document. It has no id yet — the server
 * gives it one; until then it goes under a stand-in.
 */
const withDocument = (body) => (body?.payload
  ? { ...body, payload: projectDocument({ id: 'new', ...body.payload }) }
  : body)

export const api = {
  /** The user signed in, or null. */
  me: () => request('GET', '/me').then(r => r.user, err => { if (err.status === 401) return null; throw err }),
  login: (login, password) => request('POST', '/login', { login, password }).then(r => r.user),
  logout: () => request('POST', '/logout', {}),
  changePassword: (current, next) => request('POST', '/me/password', { current, next }),

  // The projects and their variants.
  projects: () => request('GET', '/projects').then(r => r.projects ?? []),
  createProject: (body) => request('POST', '/projects', withDocument(body)),
  patchProject: (id, body) => request('PATCH', `/projects/${enc(id)}`, body),
  deleteProject: (id) => request('DELETE', `/projects/${enc(id)}`),
  branch: (projectId, body) => request('POST', `/projects/${enc(projectId)}/variants`, body),
  patchVariant: (id, body) => request('PATCH', `/variants/${enc(id)}`, body),
  /** A variant's head: { variant, revision, payload } — the payload as the record it holds. */
  head: (variantId) => request('GET', `/variants/${enc(variantId)}/head`)
    .then(r => ({ ...r, payload: parseProjectsPayload(r.payload).projects[0] })),
  members: (projectId) => request('GET', `/projects/${enc(projectId)}/members`),
  addMember: (projectId, userId) => request('POST', `/projects/${enc(projectId)}/members`, { userId }),
  removeMember: (projectId, userId) => request('DELETE', `/projects/${enc(projectId)}/members/${enc(userId)}`),
  searchUsers: (q) => request('GET', `/users?q=${enc(q)}`),
  uploadImage: (mime, data) => request('PUT', '/blobs', { mime, data }),

  // The user administration (admins only).
  users: () => request('GET', '/admin/users').then(r => r.users ?? []),
  createUser: (body) => request('POST', '/admin/users', body),
  patchUser: (id, body) => request('PATCH', `/admin/users/${enc(id)}`, body),
  cloudAdmin: () => request('GET', '/admin/clouds'),
}

/** A picture the server keeps, by its hash. */
export const blobUrl = (hash) => (hash ? `${BASE}/blobs/${enc(hash)}` : null)
