import OPERATIONS from '../../schemas/operations/1.0.schema.json' with { type: 'json' }
import ENVELOPE from '../../schemas/operations.schema.json' with { type: 'json' }
import { contentHash } from './canonical.js'
import { viaNative } from './viaNative.js'

// The door for operations documents (operations-1.0; the roadmap, phase 4,
// O1): a project's timetables and their trains, the kinds of trains, the
// formations and the vehicles they run with, connections, turnarounds, delay
// scenarios and studies, in the envelope of infra-data. What comes in is
// checked here, and the timetables as the suite keeps them become a document
// and come back from one. The door is the core's Rust module (crates/doc,
// operations).

/** The file format's name and version, as the envelope states them. */
export const OPERATIONS_FORMAT = 'operations'
export const OPERATIONS_VERSION = '1.0'
/** The format's name as it is spoken of: operations-1.0. */
export const OPERATIONS_FORMAT_NAME = `${OPERATIONS_FORMAT}-${OPERATIONS_VERSION}`

/** The hash the domain's schema goes by. */
export const OPERATIONS_SCHEMA_HASH = contentHash(OPERATIONS)

/** The domain's schema ("operations"), and the envelope's ("operations-document"). */
export const operationsSchemaOf = (name) => ({ operations: OPERATIONS, 'operations-document': ENVELOPE })[name]

/** Whether a parsed file says it is an operations document — of any version. */
export const isOperationsFile = (data) => data?.format === OPERATIONS_FORMAT

/**
 * Check an operations document; returns it, or throws a PayloadError:
 * `unsupported_version` for another format or version, `invalid_payload`
 * (with its `details`) for anything else — a reference into its own
 * collections that finds nothing, a carried vehicle that is not its hash.
 */
export function checkOperations(given) {
  const checked = viaNative('operations.checkOperations', given)
  return checked.same ? given : checked.doc
}

/**
 * The operations document of a record — `{ timetables: [{ id, name, data }],
 * vehicles?: [{ key, vehicle }], trainKinds?, formations?, connections?,
 * turnarounds?, delayScenarios?, studies? }` —, checked unless `check: false`.
 * The vehicles are carried, the formations naming them pinned to their hash.
 */
export const toOperations = (record, { check = true } = {}) => viaNative('operations.toOperations', { record, check })

/**
 * The record an operations document holds, once the door has checked it: the
 * timetables as the suite keeps them (a timetable's trains by departure), the
 * carried vehicles as `[{ key, hash, vehicle }]`, the other collections as
 * they are.
 */
export const fromOperations = (doc) => viaNative('operations.fromOperations', doc)
