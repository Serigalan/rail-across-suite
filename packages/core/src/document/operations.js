import VEHICLES from '../../schemas/vehicles/1.0.schema.json' with { type: 'json' }
import TIMETABLES from '../../schemas/timetables/1.0.schema.json' with { type: 'json' }
import ENVELOPE from '../../schemas/operations.schema.json' with { type: 'json' }
import { contentHash } from './canonical.js'
import { viaNative } from './viaNative.js'

// The door for operations documents (operations-1.0; the roadmap, phase 4,
// O1; decision 0029, phase 4): the domains vehicles — the vehicles a
// project's trains run with — and timetables — its timetables and their
// trains, the kinds of trains, the formations, connections, turnarounds,
// delay scenarios and studies. What comes in is
// checked here, and the timetables as the suite keeps them become a document
// and come back from one. The door is the core's Rust module (crates/doc,
// operations).

/** The file format's name and version, as the envelope states them. */
export const OPERATIONS_FORMAT = 'operations'
export const OPERATIONS_VERSION = '1.0'
/** The format's name as it is spoken of: operations-1.0. */
export const OPERATIONS_FORMAT_NAME = `${OPERATIONS_FORMAT}-${OPERATIONS_VERSION}`

/** The hash each domain's schema goes by. */
export const OPERATIONS_SCHEMA_HASHES = { vehicles: contentHash(VEHICLES), timetables: contentHash(TIMETABLES) }

/** A domain's schema ("vehicles", "timetables"), and the envelope's ("operations-document"). */
export const operationsSchemaOf = (name) => ({ vehicles: VEHICLES, timetables: TIMETABLES, 'operations-document': ENVELOPE })[name]

/** Whether a parsed file says it is an operations document — of any version. */
export const isOperationsFile = (data) => data?.format === OPERATIONS_FORMAT

/**
 * Check an operations document; returns it, or throws a PayloadError:
 * `unsupported_version` for another format or version, `invalid_payload`
 * (with its `details`) for anything else — a reference into its own
 * domains that finds nothing; a document of before (one domain operations)
 * comes back in the two domains.
 */
export function checkOperations(given) {
  const checked = viaNative('operations.checkOperations', given)
  return checked.same ? given : checked.doc
}

/**
 * The operations document of a record — `{ timetables: [{ id, name, data }],
 * vehicles?: [{ key, vehicle }], trainKinds?, formations?, connections?,
 * turnarounds?, delayScenarios?, studies? }` —, checked unless `check: false`.
 * The vehicles are carried with their rolling stock; one a train names by its
 * key and the record does not carry is the catalogue's, named by its key.
 */
export const toOperations = (record, { check = true } = {}) => viaNative('operations.toOperations', { record, check })

/**
 * The record an operations document holds, once the door has checked it: the
 * timetables as the suite keeps them (a timetable's trains by departure), the
 * carried vehicles as `[{ key, hash, vehicle }]`, the other collections as
 * they are.
 */
export const fromOperations = (doc) => viaNative('operations.fromOperations', doc)
