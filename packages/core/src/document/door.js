import ENVELOPE from '../../schemas/infra-data.schema.json' with { type: 'json' }
import CORE from '../../schemas/core/1.0.schema.json' with { type: 'json' }
import LAYOUT from '../../schemas/layout/1.0.schema.json' with { type: 'json' }
import SIGNALLING from '../../schemas/signalling/1.0.schema.json' with { type: 'json' }
import { contentHash } from './canonical.js'
import { viaNative } from './viaNative.js'

// The one door for infra-data-1.0 documents (decision 0021: the one format of
// a project, what was "the core 1.0 document"): what comes in is checked here — the
// file's shape, each domain's version and schema, every object against its
// domain's schema, the blobs against their hashes, a domain of an earlier
// schema of its version brought to this build's — and what is written is
// checked the same way before it leaves. Behind the door the model is taken
// for granted. The door is the core's Rust module (crates/doc; decision 0020,
// step 10 took its JavaScript out).

/** The file format's name and version, as the envelope states them. */
export const FORMAT = 'infra-data'
export const FORMAT_VERSION = '1.0'
/** The format's name as it is spoken of: infra-data-1.0. */
export const FORMAT_NAME = `${FORMAT}-${FORMAT_VERSION}`

const DOMAIN_SCHEMAS = { core: CORE, layout: LAYOUT, signalling: SIGNALLING }

/** The hash each domain's schema goes by — what a document names it with. */
export const SCHEMA_HASHES = Object.fromEntries(Object.entries(DOMAIN_SCHEMAS).map(([d, s]) => [d, contentHash(s)]))

/**
 * Check an infra-data-1.0 document; returns it — the very object where it was as
 * this build reads it, else a domain of an earlier schema of its version
 * brought to this build's —, or throws a PayloadError:
 * `unsupported_version` for another format (an oltjson file among them), a
 * domain or a version this build does not read,
 * `invalid_payload` (with its `details`) for anything else that is not as the
 * schemas say.
 */
export function checkDocument(given) {
  const checked = viaNative('document.checkDocument', given)
  return checked.same ? given : checked.doc
}

/**
 * The infra-data-1.0 document of a record (dehydrated), checked as the door checks
 * what comes in. `carrySchemas` puts the domains' schemas into it, as a file
 * that travels carries them (R3); a device or the server that has them
 * leaves them out. `check: false` leaves the check out — for a file the user
 * takes although the schemas refuse it, which the door will refuse in turn.
 */
export const toDocument = (record, { carrySchemas = false, check = true } = {}) =>
  viaNative('document.toDocument', { record, carrySchemas, check })

/** The record an infra-data-1.0 document holds, once the door has checked it. */
export const fromDocument = (doc) => viaNative('document.fromDocument', doc)

/** The schema of a domain this build reads, by name ("core", "layout", "signalling"), and the envelope's ("infra-data"). */
export const schemaOf = (name) => (name === FORMAT ? ENVELOPE : DOMAIN_SCHEMAS[name])
