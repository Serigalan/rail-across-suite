// The suite's project server, same origin as the page: the session is an
// HttpOnly cookie the browser sends by itself, and every writing request says
// it carries JSON (the server refuses it otherwise). The start page holds what
// is the suite's — signing in and out, the password, the users and their
// rights — and lists the projects; everything else is the modules'.

const BASE = import.meta.env?.VITE_RA_API ?? '/api'

export class ApiError extends Error {
  constructor(status, code) {
    super(code)
    this.status = status
    this.code = code
  }
}

async function request(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    credentials: 'same-origin',
    headers: method === 'GET' ? { accept: 'application/json' } : { 'content-type': 'application/json', accept: 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (res.status === 204) return null
  const data = await res.json().catch(() => null)
  if (!res.ok) throw new ApiError(res.status, data?.error ?? 'generic')
  return data
}

export const api = {
  /** The user signed in, or null. */
  me: () => request('GET', '/me').then(r => r.user, err => { if (err.status === 401) return null; throw err }),
  login: (login, password) => request('POST', '/login', { login, password }).then(r => r.user),
  logout: () => request('POST', '/logout', {}),
  changePassword: (current, next) => request('POST', '/me/password', { current, next }),
  projects: () => request('GET', '/projects').then(r => r.projects ?? []),

  // The user administration (admins only).
  users: () => request('GET', '/admin/users').then(r => r.users ?? []),
  createUser: (body) => request('POST', '/admin/users', body),
  patchUser: (id, body) => request('PATCH', `/admin/users/${encodeURIComponent(id)}`, body),
  cloudAdmin: () => request('GET', '/admin/clouds'),
}

/** A picture the server keeps, by its hash. */
export const blobUrl = (hash) => (hash ? `${BASE}/blobs/${encodeURIComponent(hash)}` : null)
