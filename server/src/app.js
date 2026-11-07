import Fastify from 'fastify'
import cookie from '@fastify/cookie'
import { COOKIE, SESSION_IDLE, createAuth, ensureLocalUser, publicUser } from './auth.js'
import { ApiError } from './errors.js'
import { createStaticHandler } from './static.js'
import authRoutes from './routes/auth.js'
import adminRoutes from './routes/admin.js'
import projectRoutes from './routes/projects.js'
import userRoutes from './routes/users.js'
import cloudRoutes from './routes/clouds.js'
import runRoutes from './routes/runs.js'
import shareRoutes from './routes/shares.js'
import vehicleRoutes from './routes/vehicles.js'
import timetableRoutes from './routes/timetables.js'
import terrainRoutes from './routes/terrain.js'
import { cloudStorage } from './clouds/storage.js'

/** Largest request body [bytes] — a large MDB import fits, a runaway does not fill the disk. */
const BODY_LIMIT = 20 * 1024 * 1024

const WRITING = new Set(['POST', 'PUT', 'PATCH', 'DELETE'])

/**
 * The writing requests that carry bytes, not JSON: a piece of a point cloud
 * upload (AP 13.2) and an Access file for the MDB import. Each must say so in
 * a header of its own, which no form of another site can send without the
 * browser asking first.
 */
const UPLOADS = [
  ['PUT', /^\/api\/projects\/[^/?]+\/clouds\/[^/?]+\/raw(\?|$)/],
  ['POST', /^\/api\/mdb(\?|$)/],
]
const isUpload = (req) => UPLOADS.some(([method, path]) => req.method === method && path.test(req.url))
  && String(req.headers['content-type'] ?? '').startsWith('application/octet-stream') && req.headers['x-olt-upload'] === '1'

/**
 * Whose X-Forwarded-For is believed: only the reverse proxy's (Caddy on the
 * same host). Believing anybody's would let a client that reaches the port
 * directly name its own address, and with it step around the login brake.
 * A Caddy elsewhere (a container) is named in RA_SERVER_TRUST_PROXY.
 */
const TRUSTED_PROXIES = ['127.0.0.1', '::1']

/**
 * The API under /api (phase 10). Same origin as the app, so no CORS: the
 * session is an HttpOnly, SameSite=Strict cookie, and a writing request has to
 * say it carries JSON — a form another site posts cannot, which closes the
 * cross-site way in together with SameSite.
 *
 * `secureCookie` is off only for plain-http tests and local runs.
 * `cloudRoot` is where point clouds are kept (phase 13); without one their
 * routes answer 503.
 *
 * `local` is the local package (desktop/, Tauri): no user management — no
 * sign-in, no sessions, no admin or user routes. Every request carries the
 * one implicit local user (`ensureLocalUser`), `/api/me` answers it with 200,
 * so the modules' "signed in?" question passes without a cookie. `staticRoot`
 * serves the built suite from one directory, as Caddy does in production, so
 * the page and the API share one origin.
 */
export function buildApp({
  db, secureCookie = true, now = () => Date.now(), logger = false, routes = [], trustProxy = TRUSTED_PROXIES,
  cloudRoot = null, local = false, staticRoot = null,
} = {}) {
  const app = Fastify({ logger, bodyLimit: BODY_LIMIT, trustProxy, ...(staticRoot ? { exposeHeadRoutes: false } : {}) })
  const auth = createAuth(db, { now })
  const localUser = local ? ensureLocalUser(db, now) : null
  app.decorate('db', db)
  app.decorate('auth', auth)
  app.decorate('now', now)
  app.decorate('cloudStorage', cloudRoot ? cloudStorage(cloudRoot) : null)
  app.decorateRequest('user', null)

  app.register(cookie)

  app.setErrorHandler((err, req, reply) => {
    if (err instanceof ApiError) return reply.code(err.status).send({ error: err.error, ...err.extra })
    if (err.statusCode === 413) return reply.code(413).send({ error: 'too_large' })
    if (err.statusCode && err.statusCode < 500) return reply.code(err.statusCode).send({ error: 'bad_request', message: err.message })
    req.log.error(err)
    return reply.code(500).send({ error: 'internal' })
  })

  app.addHook('onRequest', async (req) => {
    if (WRITING.has(req.method) && !String(req.headers['content-type'] ?? '').startsWith('application/json') && !isUpload(req)) {
      throw new ApiError(415, 'json_required')
    }
    req.user = local ? localUser : auth.resolveSession(req.cookies?.[COOKIE])
  })

  app.decorate('sessionCookie', (reply, token) => reply.setCookie(COOKIE, token, {
    path: '/', httpOnly: true, sameSite: 'strict', secure: secureCookie, maxAge: SESSION_IDLE / 1000,
  }))
  app.decorate('clearSessionCookie', (reply) => reply.clearCookie(COOKIE, {
    path: '/', httpOnly: true, sameSite: 'strict', secure: secureCookie,
  }))

  /** preHandler: a signed-in user who may work (start password already changed). In the local package every request carries the local user, so this passes. */
  app.decorate('requireUser', async (req) => {
    if (!req.user) throw new ApiError(401, 'unauthenticated')
    if (req.user.must_change_password) throw new ApiError(403, 'password_change_required')
  })
  app.decorate('requireAdmin', async (req) => {
    await app.requireUser(req)
    if (req.user.role !== 'admin') throw new ApiError(403, 'admin_only')
  })

  app.register(async (api) => {
    // For the deploy script and monitoring: the process answers and the
    // database can be read. Needs no session.
    api.get('/health', async () => {
      db.prepare('select 1').get()
      return local ? { status: 'ok', local: true } : { status: 'ok' }
    })
    if (local) {
      // The local package has no user management: no sign-in, no password, no
      // admin or user routes. /me answers with the one implicit local user,
      // so the modules' "signed in?" question passes without a cookie.
      api.get('/me', async () => ({ user: { ...publicUser(localUser), local: true } }))
    } else {
      api.register(authRoutes)
      api.register(adminRoutes)
      api.register(userRoutes)
    }
    api.register(projectRoutes)
    api.register(cloudRoutes)
    api.register(runRoutes)
    api.register(shareRoutes)
    api.register(vehicleRoutes)
    api.register(timetableRoutes)
    api.register(terrainRoutes)
    for (const r of routes) api.register(r)
  }, { prefix: '/api' })

  // The built suite beside the API, one origin (the local package's window
  // loads the page from here). Registered last: /api/* wins over the wildcard.
  if (staticRoot) {
    app.route({ method: ['GET', 'HEAD'], url: '/*', handler: createStaticHandler(staticRoot) })
  }

  return app
}
