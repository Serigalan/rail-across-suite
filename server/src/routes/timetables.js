import { randomUUID } from 'node:crypto'
import { ApiError } from '../errors.js'
import { createStore } from '../store.js'

/** Largest timetable [bytes of JSON] — some thousand trains fit. */
const TIMETABLE_LIMIT = 4 * 1024 * 1024
/** A departure lies within two days [s from midnight of the first]: a night's trains run past midnight. */
const DAY_LIMIT = 2 * 86400

const isObject = (v) => v != null && typeof v === 'object' && !Array.isArray(v)
const isText = (v, max, required = false) => typeof v === 'string' && v.length <= max && (!required || v.trim().length > 0)
const orNull = (v, test) => v == null || test(v)
const isTime = (v) => Number.isFinite(v) && v >= 0 && v < DAY_LIMIT

/**
 * A train's point — a station it starts at, stops at, passes or ends at —:
 * the place's id — absent in a point written before places had ids — and
 * its name to show it by, its track or none, whether it stops there, how
 * long, the departure it waits for there.
 */
function isPoint(p) {
  return isObject(p) && isText(p.id, 64, true) && orNull(p.place, v => isText(v, 64, true)) && isText(p.station, 200, true)
    && orNull(p.track, v => isText(v, 64, true)) && typeof p.stop === 'boolean'
    && orNull(p.dwell, v => Number.isFinite(v) && v >= 0 && v <= 86400) && orNull(p.departure, isTime)
}

/**
 * What a timetable does where trains would hold the same block
 * (@rail-across/timetables, occupancy.js): only say so, or let them wait —
 * by departure or by kind, the kinds in an order —, and the times a block is
 * held beyond a train's running in it.
 */
function isDispatch(d) {
  if (!isObject(d)) return false
  const times = d.times ?? {}
  return orNull(d.mode, v => ['show', 'wait'].includes(v)) && orNull(d.priority, v => ['departure', 'category'].includes(v))
    && orNull(d.categories, v => Array.isArray(v) && v.length <= 100 && v.every(c => isText(c, 20, true)))
    && isObject(times) && ['setup', 'approach', 'release'].every(k => orNull(times[k], v => Number.isFinite(v) && v >= 0 && v <= 10000))
}

/**
 * A timetable's data, checked for its shape: its trains, each with an id of
 * its own, its number, kind and name, its vehicle, departure, margin and
 * points; what it does where trains would hold the same block. That a train begins and ends at a station of the net is the
 * Fahrplan-Editor's to say, which knows the variant it lays it on; a train
 * not done yet is kept as it is. The key of the first fault, else null.
 */
export function timetableFault(data) {
  if (!isObject(data) || !Array.isArray(data.trains) || data.trains.length > 5000) return 'timetable_invalid'
  if (data.dispatch != null && !isDispatch(data.dispatch)) return 'dispatch_invalid'
  const ids = new Set()
  for (const t of data.trains) {
    if (!isObject(t) || !isText(t.id, 64, true) || ids.has(t.id)) return 'train_invalid'
    ids.add(t.id)
    if (!isText(t.number ?? '', 40) || !isText(t.category ?? '', 20) || !isText(t.name ?? '', 200)) return 'train_invalid'
    if (!orNull(t.vehicle, v => isText(v, 200, true)) || !orNull(t.mode, v => isText(v, 40)) || !orNull(t.comfort, v => isText(v, 40))) return 'train_invalid'
    if (!isTime(t.departure) || !isText(t.margin ?? '', 40) || !['mareco', 'linear'].includes(t.distribution ?? 'mareco')) return 'train_invalid'
    if (!Array.isArray(t.points) || t.points.length > 300 || !t.points.every(isPoint)) return 'points_invalid'
  }
  return null
}

/**
 * The timetables of a project, for the Fahrplan-Editor (decision 0016): seen
 * and changed by whoever sees the project — a template's changed by the
 * admins alone —, the project's members working on them together. A save
 * names the version it changed; on an older one it is refused with the
 * timetable as it is now (409).
 */
export default async function timetableRoutes(api) {
  const opts = { preHandler: api.requireUser }
  const store = createStore(api.db, { now: api.now })
  const iso = () => new Date(api.now()).toISOString()
  const SELECT = `SELECT t.*, c.name AS created_by_name, u.name AS updated_by_name FROM timetable t
                  JOIN user c ON c.id = t.created_by JOIN user u ON u.id = t.updated_by`
  const q = {
    list: api.db.prepare(`${SELECT} WHERE t.project_id = ? AND t.deleted_at IS NULL ORDER BY t.name COLLATE NOCASE, t.created_at`),
    get: api.db.prepare(`${SELECT} WHERE t.id = ? AND t.deleted_at IS NULL`),
    insert: api.db.prepare(`INSERT INTO timetable (id, project_id, name, data, created_by, created_at, updated_by, updated_at)
                            VALUES (@id, @project, @name, @data, @user, @at, @user, @at)`),
    update: api.db.prepare(`UPDATE timetable SET name = @name, data = @data, version = version + 1, updated_by = @user, updated_at = @at
                            WHERE id = @id AND version = @version AND deleted_at IS NULL`),
    remove: api.db.prepare('UPDATE timetable SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL'),
  }

  // Whoever may not see the project is told it is not there.
  const projectOr404 = (id, user) => {
    const p = store.project(id)
    if (!p || !store.canSee(user, p)) throw new ApiError(404, 'not_found')
    return p
  }
  const mayEdit = (user, project) => !project.template || user.role === 'admin'
  const timetableOr404 = (req) => {
    const row = q.get.get(req.params.id)
    if (!row) throw new ApiError(404, 'not_found')
    return { row, project: projectOr404(row.project_id, req.user) }
  }
  const publicTimetable = (row, user, project) => ({
    id: row.id, projectId: row.project_id, name: row.name, version: row.version, data: JSON.parse(row.data),
    createdBy: { id: row.created_by, name: row.created_by_name }, createdAt: row.created_at,
    updatedBy: { id: row.updated_by, name: row.updated_by_name }, updatedAt: row.updated_at,
    mayEdit: mayEdit(user, project),
  })
  const checked = (req) => {
    const name = String(req.body?.name ?? '').trim().slice(0, 200)
    if (!name) throw new ApiError(422, 'name_required')
    const data = req.body?.data ?? { trains: [] }
    const fault = timetableFault(data)
    if (fault) throw new ApiError(422, fault)
    const json = JSON.stringify(data)
    if (json.length > TIMETABLE_LIMIT) throw new ApiError(413, 'too_large')
    return { name, data: json }
  }

  api.get('/projects/:id/timetables', opts, async (req) => {
    const project = projectOr404(req.params.id, req.user)
    return { timetables: q.list.all(project.id).map(row => publicTimetable(row, req.user, project)), mayEdit: mayEdit(req.user, project) }
  })

  api.post('/projects/:id/timetables', opts, async (req, reply) => {
    const project = projectOr404(req.params.id, req.user)
    if (!mayEdit(req.user, project)) throw new ApiError(403, 'template_admin')
    const id = randomUUID()
    q.insert.run({ id, project: project.id, ...checked(req), user: req.user.id, at: iso() })
    return reply.code(201).send({ timetable: publicTimetable(q.get.get(id), req.user, project) })
  })

  api.get('/timetables/:id', opts, async (req) => {
    const { row, project } = timetableOr404(req)
    return { timetable: publicTimetable(row, req.user, project) }
  })

  api.put('/timetables/:id', opts, async (req, reply) => {
    const { row, project } = timetableOr404(req)
    if (!mayEdit(req.user, project)) throw new ApiError(403, 'template_admin')
    const values = checked(req)
    if (!Number.isInteger(req.body?.version)) throw new ApiError(422, 'version_required')
    const done = q.update.run({ id: row.id, version: req.body.version, ...values, user: req.user.id, at: iso() })
    if (!done.changes) return reply.code(409).send({ error: 'stale_version', timetable: publicTimetable(q.get.get(row.id), req.user, project) })
    return { timetable: publicTimetable(q.get.get(row.id), req.user, project) }
  })

  api.delete('/timetables/:id', opts, async (req, reply) => {
    const { row, project } = timetableOr404(req)
    if (!mayEdit(req.user, project)) throw new ApiError(403, 'template_admin')
    q.remove.run(iso(), row.id)
    return reply.code(204).send()
  })
}
