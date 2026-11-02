import { randomUUID } from 'node:crypto'
import { ApiError } from '../errors.js'

/** Largest vehicle [bytes of JSON] — a rolling stock with many curves fits. */
const VEHICLE_LIMIT = 512 * 1024

const isObject = (v) => v != null && typeof v === 'object' && !Array.isArray(v)
const positive = (v) => Number.isFinite(v) && v > 0

/** A curve: speeds and efforts, as many of each, at least two, the speeds rising. */
function isCurve(c) {
  if (!isObject(c) || !Array.isArray(c.speeds) || !Array.isArray(c.max_efforts)) return false
  const { speeds, max_efforts: efforts } = c
  return speeds.length >= 2 && speeds.length === efforts.length
    && speeds.every((s, i) => Number.isFinite(s) && s >= 0 && (i === 0 || s > speeds[i - 1]))
    && efforts.every(e => Number.isFinite(e) && e >= 0)
}

/**
 * A vehicle, checked for what the simulation needs of it: a name, its mass,
 * length, top speed and braking, its rolling resistance, and tractive effort
 * curves — one mode at least, the default one among them. Whatever else OSRD's
 * format holds is kept as it comes. The key of the first fault, else null.
 */
export function vehicleFault(v) {
  if (!isObject(v)) return 'vehicle_invalid'
  if (typeof v.name !== 'string' || !v.name.trim() || v.name.length > 200) return 'name_required'
  for (const key of ['mass', 'length', 'max_speed', 'const_gamma']) if (!positive(v[key])) return `${key}_invalid`
  if (!(Number.isFinite(v.inertia_coefficient) && v.inertia_coefficient >= 1)) return 'inertia_coefficient_invalid'
  const rr = v.rolling_resistance
  if (!isObject(rr) || !['A', 'B', 'C'].every(k => Number.isFinite(rr[k]) && rr[k] >= 0)) return 'rolling_resistance_invalid'
  const ec = v.effort_curves
  if (!isObject(ec) || !isObject(ec.modes) || !Object.keys(ec.modes).length || !isObject(ec.modes[ec.default_mode])) return 'effort_curves_invalid'
  for (const mode of Object.values(ec.modes)) {
    if (!isObject(mode) || !isCurve(mode.default_curve) || !Array.isArray(mode.curves ?? [])) return 'effort_curves_invalid'
    if (!(mode.curves ?? []).every(c => isObject(c) && isObject(c.cond) && isCurve(c.curve))) return 'effort_curves_invalid'
  }
  return null
}

/**
 * The suite's vehicles for the Fahrplan-Sim (decision 0014): one catalogue for
 * everyone signed in, kept as OSRD's RailJSON rolling stock. Everyone sees them
 * all and adds their own; changing and deleting one is its creator's and the
 * admins' — the others copy it. A save names the version it changed; on an
 * older one it is refused with the vehicle as it is now (409).
 */
export default async function vehicleRoutes(api) {
  const opts = { preHandler: api.requireUser }
  const iso = () => new Date(api.now()).toISOString()
  const q = {
    list: api.db.prepare(`SELECT v.*, c.name AS created_by_name FROM vehicle v JOIN user c ON c.id = v.created_by
                          WHERE v.deleted_at IS NULL ORDER BY v.name COLLATE NOCASE, v.id`),
    get: api.db.prepare(`SELECT v.*, c.name AS created_by_name FROM vehicle v JOIN user c ON c.id = v.created_by
                         WHERE v.id = ? AND v.deleted_at IS NULL`),
    insert: api.db.prepare(`INSERT INTO vehicle (id, name, data, created_by, created_at, updated_by, updated_at)
                            VALUES (@id, @name, @data, @user, @at, @user, @at)`),
    update: api.db.prepare(`UPDATE vehicle SET name = @name, data = @data, version = version + 1, updated_by = @user, updated_at = @at
                            WHERE id = @id AND version = @version AND deleted_at IS NULL`),
    remove: api.db.prepare('UPDATE vehicle SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL'),
  }

  const publicVehicle = (row, user) => ({
    id: row.id, version: row.version, data: JSON.parse(row.data),
    createdBy: { id: row.created_by, name: row.created_by_name }, createdAt: row.created_at, updatedAt: row.updated_at,
    mayEdit: user.role === 'admin' || row.created_by === user.id,
  })
  const vehicleOr404 = (req) => {
    const row = q.get.get(req.params.id)
    if (!row) throw new ApiError(404, 'not_found')
    return row
  }
  const checked = (req) => {
    const data = req.body?.data
    const fault = vehicleFault(data)
    if (fault) throw new ApiError(422, fault)
    const json = JSON.stringify(data)
    if (json.length > VEHICLE_LIMIT) throw new ApiError(413, 'too_large')
    return { name: data.name.trim().slice(0, 200), data: json }
  }

  api.get('/vehicles', opts, async (req) => ({ vehicles: q.list.all().map(row => publicVehicle(row, req.user)) }))

  api.get('/vehicles/:id', opts, async (req) => ({ vehicle: publicVehicle(vehicleOr404(req), req.user) }))

  api.post('/vehicles', opts, async (req, reply) => {
    const id = randomUUID()
    q.insert.run({ id, ...checked(req), user: req.user.id, at: iso() })
    return reply.code(201).send({ vehicle: publicVehicle(q.get.get(id), req.user) })
  })

  api.put('/vehicles/:id', opts, async (req, reply) => {
    const row = vehicleOr404(req)
    if (!publicVehicle(row, req.user).mayEdit) throw new ApiError(403, 'not_allowed')
    const values = checked(req)
    if (!Number.isInteger(req.body?.version)) throw new ApiError(422, 'version_required')
    const done = q.update.run({ id: row.id, version: req.body.version, ...values, user: req.user.id, at: iso() })
    if (!done.changes) return reply.code(409).send({ error: 'stale_version', vehicle: publicVehicle(q.get.get(row.id), req.user) })
    return { vehicle: publicVehicle(q.get.get(row.id), req.user) }
  })

  api.delete('/vehicles/:id', opts, async (req, reply) => {
    const row = vehicleOr404(req)
    if (!publicVehicle(row, req.user).mayEdit) throw new ApiError(403, 'not_allowed')
    q.remove.run(iso(), row.id)
    return reply.code(204).send()
  })
}
