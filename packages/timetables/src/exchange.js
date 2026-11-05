/**
 * A project's timetables as a file of their own, beside the project's backup
 * file: the core 1.0 document holds the infrastructure, and a timetable is
 * the project's, not a variant's record (decision 0016). Such a file goes
 * out with a project's export, or alone from the Fahrplan-Sim; it comes in
 * with a backup file — into the project or template made of it — or alone,
 * into a project or template there is.
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
  let data
  try { data = JSON.parse(text) } catch { throw new TimetablesFileError('timetables_invalid') }
  return parseTimetablesFile(data)
}

/** The name a project's timetables are saved under. */
export const timetablesFileName = (title) => `${title} – Fahrpläne.json`
