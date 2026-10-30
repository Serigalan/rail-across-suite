import {
  PayloadError, dehydrateProjects, parseProjectsPayload, projectDocument, projectsPayload, newFindings,
  validateProject,
} from '@rail-across/core/project'
import { checkSignalling } from '@rail-across/core/signalling'
import { ApiError } from './errors.js'

/**
 * What a payload is as the door takes it: a core 1.0 document as it is, a
 * bare record — oltjson-1.1 without its envelope, what clients sent before
 * core 1.0 — in its envelope, so the door migrates it. `id` stands in for a
 * record's missing id (a new project's, which the store gives it).
 */
function asPayload(payload, id) {
  if (payload?.format) return payload
  return projectsPayload([{ ...payload, ...(payload.id == null && id != null ? { id } : {}) }])
}

/**
 * A payload coming in, taken through the browser's own door (decision 91):
 * parseProjectsPayload refuses what is not a core 1.0 document (or an
 * oltjson-1.1 record it can migrate into one), dehydrateProjects strips
 * whatever derived geometry rode along, validateProject says what holds. The
 * server trusts the browser in none of it.
 *
 * Returns { record, errors, warnings }.
 */
export function checkRecord(payload, { id } = {}) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) throw new ApiError(422, 'invalid_payload')
  let parsed
  try {
    parsed = parseProjectsPayload(asPayload(payload, id))
  } catch (err) {
    throw new ApiError(422, err instanceof PayloadError ? err.code : 'invalid_payload')
  }
  const [record] = dehydrateProjects(parsed.projects)
  const { errors, warnings } = validateProject(record)
  const signalling = signallingFindings(record)
  return { record, errors: [...errors, ...signalling.errors], warnings: [...warnings, ...signalling.warnings] }
}

/**
 * The findings of the record's signalling (decision 0011), where it has
 * some: checked on the core it stands on, keyed apart from the record's own
 * so a finding of one never stands for one of the other.
 */
function signallingFindings(record) {
  if (!record.signalling) return { errors: [], warnings: [] }
  const { domains } = projectDocument(record)
  const { errors, warnings } = checkSignalling({ core: domains.core, signalling: domains.signalling })
  const keyed = (f) => ({ ...f, domain: 'signalling', key: `signalling:${f.key}` })
  return { errors: errors.map(keyed), warnings: warnings.map(keyed) }
}

/**
 * The record a stored payload holds — a core 1.0 document, or a record of the
 * old layout — as checkRecord would hand it on, not validated: so a part of it
 * compares with a record coming in.
 */
export function normalizedRecord(payload) {
  try {
    return dehydrateProjects(parseProjectsPayload(asPayload(payload)).projects)[0]
  } catch {
    return payload
  }
}

/** The errors of `errors` a revision with `knownKeys` did not have. */
export const freshErrors = (errors, knownKeys) => newFindings(errors, knownKeys.map(key => ({ key })))

/** JSON with the keys of every object sorted — two records' parts compared as values. */
const canonical = (v) => JSON.stringify(v, (_, x) => (x && typeof x === 'object' && !Array.isArray(x)
  ? Object.fromEntries(Object.keys(x).sort().map(k => [k, x[k]]))
  : x))

/**
 * Whether the measured axes of `record` are other than its parents have them
 * (decision 216): each one — and each one gone — must be as the base or the
 * merge parent has it. Saving or deleting a measured axis is changing point
 * clouds; a merge only carries over what one side already had.
 */
export function axisSurveysChanged(record, base, mergeParent = null) {
  const byId = (r) => new Map((r?.axisSurveys ?? []).map(s => [s?.id, canonical(s)]))
  const now = byId(record), parents = [byId(base), ...(mergeParent ? [byId(mergeParent)] : [])]
  const ids = new Set([...now.keys(), ...parents.flatMap(p => [...p.keys()])])
  return [...ids].some(id => !parents.some(p => p.get(id) === now.get(id)))
}
