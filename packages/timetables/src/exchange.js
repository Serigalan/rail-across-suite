import { checkOperations, CONTAINER_EXTENSION, contentHash, fileParts, fromOperations, isOperationsFile, isProjectFile, projectFile, toOperations } from '@rail-across/core/document'

/**
 * A project's timetables as a file of their own, beside the project's backup
 * file: the infra-data document holds the infrastructure, and a timetable is
 * the project's, not a variant's record (decision 0012). Such a file goes
 * out with a project's export, or alone from the Fahrplan-Sim; it comes in
 * with a backup file — into the project or template made of it — or alone,
 * into a project or template there is.
 *
 * It is an **operations-1.0** document (`.operations.json`, decision 0012):
 * the project's operations in the envelope of infra-data, the vehicles its
 * trains run with carried, so it runs on another server with the same times.
 * The file of before is still read, and turned on its way in:
 *
 *   { format: 'rail-across-timetables', version: '1.0', project, timetables: [{ name, data }] }
 *
 * `project` the title it was exported from, only to say what it is; each
 * timetable its name and its data as the server keeps it ({ trains,
 * dispatch }). Whether a train is as the server takes it, the server says.
 */
export const TIMETABLES_FORMAT = 'rail-across-timetables'
export const TIMETABLES_VERSION = '1.0'

/** Most timetables a file brings — the server takes no more at once. */
export const TIMETABLES_PER_FILE = 200

export class TimetablesFileError extends Error {
  constructor(code) {
    super(code)
    this.code = code
  }
}

/** The file of `timetables` (as the server lists them, or { name, data }) of the project titled `project`. */
export const timetablesFile = (timetables, { project = '' } = {}) => ({
  format: TIMETABLES_FORMAT,
  version: TIMETABLES_VERSION,
  project,
  timetables: timetables.map(({ name, data }) => ({ name, data })),
})

/** Whether a parsed file says it is one of timetables — of any version. */
export const isTimetablesFile = (data) => data?.format === TIMETABLES_FORMAT

const isObject = (v) => v != null && typeof v === 'object' && !Array.isArray(v)

/**
 * The timetables of a parsed file, [{ name, data }]. Throws a
 * TimetablesFileError: `timetables_version` for a file of another version,
 * `timetables_invalid` for one that is not of this shape.
 */
export function parseTimetablesFile(data) {
  if (!isTimetablesFile(data)) throw new TimetablesFileError('timetables_invalid')
  if (data.version !== TIMETABLES_VERSION) throw new TimetablesFileError('timetables_version')
  const list = data.timetables
  if (!Array.isArray(list) || list.length > TIMETABLES_PER_FILE) throw new TimetablesFileError('timetables_invalid')
  if (!list.every(t => isObject(t) && typeof t.name === 'string' && t.name.trim() && isObject(t.data) && Array.isArray(t.data.trains))) {
    throw new TimetablesFileError('timetables_invalid')
  }
  return list.map(({ name, data: d }) => ({ name: name.trim(), data: d }))
}

/** The timetables of a file's text, as parseTimetablesFile reads it; `timetables_invalid` where it is no JSON. */
export function timetablesOfText(text) {
  return readTimetables(text).timetables
}

/** The name a project's timetables are saved under. */
export const timetablesFileName = (title) => `${title} – Fahrpläne.json`

// ── operations-1.0 ──────────────────────────────────────────────────────────

/** The keys of the vehicles the trains of `timetables` run with. */
export function vehicleKeys(timetables) {
  const keys = new Set()
  for (const t of timetables) for (const train of t.data?.trains ?? []) if (train.vehicle) keys.add(train.vehicle)
  return [...keys]
}

/**
 * The operations file of `timetables` (as the server lists them, or { id?,
 * name, data }), carrying the vehicles of `entries` (the catalogue's, { key,
 * vehicle }) their trains run with.
 */
export function operationsFile(timetables, entries = []) {
  const keys = new Set(vehicleKeys(timetables))
  const record = {
    timetables: timetables.map(({ id, name, data }, i) => ({ id: id ?? `timetable-${i + 1}`, name, data })),
    vehicles: entries.filter(e => keys.has(e.key)).map(e => ({ key: e.key, vehicle: e.vehicle })),
  }
  return toOperations(record)
}

/**
 * The file rail-across-1.0 of a project's operations alone (decision 0006): `timetables` and the vehicles of `entries` as operationsFile has
 * them, the envelope naming the project (`projectId`) and when it was saved.
 */
export const operationsProjectFile = (timetables, entries = [], meta = {}) =>
  projectFile({ operations: operationsFile(timetables, entries) }, { savedAt: new Date().toISOString(), ...meta })

/** The name a project's operations are saved under, alone. */
export const operationsFileName = (title) => `${title} – Fahrpläne${CONTAINER_EXTENSION}`

/**
 * What a parsed file of timetables brings — an operations file, or one of
 * before —: `{ timetables: [{ name, data }], vehicles: [{ key, hash, vehicle }] }`.
 * Throws a TimetablesFileError: `timetables_version` for a file of another
 * version, `timetables_invalid` for one that is not one to read.
 */
export function timetablesOf(data) {
  if (isOperationsFile(data) || isProjectFile(data)) {
    let record
    try {
      const doc = isProjectFile(data) ? fileParts(data).operations : data
      record = doc ? fromOperations(checkOperations(doc)) : { timetables: [], vehicles: [] }
    } catch (err) {
      throw new TimetablesFileError(err.code === 'unsupported_version' ? 'timetables_version' : 'timetables_invalid')
    }
    if (record.timetables.length > TIMETABLES_PER_FILE) throw new TimetablesFileError('timetables_invalid')
    return { timetables: record.timetables.map(({ name, data: d }) => ({ name, data: d })), vehicles: record.vehicles }
  }
  return { timetables: parseTimetablesFile(data), vehicles: [] }
}

/** What a file's text brings, as timetablesOf reads it; `timetables_invalid` where it is no JSON. */
export function readTimetables(text) {
  let data
  try { data = JSON.parse(text) } catch { throw new TimetablesFileError('timetables_invalid') }
  return timetablesOf(data)
}

/** Whether a parsed file is one of timetables — an operations file or one of before. */
export const isTimetablesData = (data) => isTimetablesFile(data) || isOperationsFile(data) || (isProjectFile(data) && !data.infraData)

/**
 * The timetables with the vehicles a file carried taken into the catalogue:
 * one the catalogue has under its key as it is (`entries`, { key, vehicle })
 * is taken as it is; one it lacks, or has with another content, is made anew
 * by `create(vehicle)` — beside the catalogue's, never over it — and the
 * trains that run with it name the new one. Returns { timetables, added:
 * [{ key, id, name }] }, what was made, to be said.
 */
export async function withCarriedVehicles(timetables, vehicles, { entries, create }) {
  const byKey = new Map(entries.map(e => [e.key, e]))
  const renamed = new Map()
  const added = []
  for (const v of vehicles) {
    const there = byKey.get(v.key)
    if (there && contentHash(there.vehicle) === v.hash) continue
    const made = await create(v.vehicle)
    renamed.set(v.key, made.id)
    added.push({ key: v.key, id: made.id, name: v.vehicle?.name ?? v.key })
  }
  if (!renamed.size) return { timetables, added }
  const trains = (list) => list.map(train => (renamed.has(train.vehicle) ? { ...train, vehicle: renamed.get(train.vehicle) } : train))
  return { timetables: timetables.map(t => ({ ...t, data: { ...t.data, trains: trains(t.data?.trains ?? []) } })), added }
}
