import ENVELOPE from '../../schemas/rail-across.schema.json' with { type: 'json' }
import CORE from '../../schemas/core/1.0.schema.json' with { type: 'json' }
import LAYOUT from '../../schemas/layout/1.0.schema.json' with { type: 'json' }
import SIGNALLING from '../../schemas/signalling/1.0.schema.json' with { type: 'json' }
import { contentHash } from './canonical.js'
import { DOMAIN_VERSIONS, documentToRecord, recordToDocument } from './coreFormat.js'
import { PayloadError } from './payloadError.js'
import { SchemaSet } from './schemaCheck.js'

// The one door for core 1.0 documents: what comes in is checked here — the
// file's shape, each domain's version and schema, every object against its
// domain's schema, the blobs against their hashes — and what is written is
// checked the same way before it leaves. Behind the door the model is taken
// for granted.

/** The file format's name, as the envelope states it. */
export const FORMAT = 'rail-across'

/** The schemas of this build: the envelope's, and each domain's by name. */
const SCHEMAS = new SchemaSet([ENVELOPE, CORE, LAYOUT, SIGNALLING])
const DOMAIN_SCHEMAS = { core: CORE, layout: LAYOUT, signalling: SIGNALLING }

/** The hash each domain's schema goes by — what a document names it with. */
export const SCHEMA_HASHES = Object.fromEntries(Object.entries(DOMAIN_SCHEMAS).map(([d, s]) => [d, contentHash(s)]))

/** The breaches of a schema as "path: what" lines, the path prefixed with where the value lies. */
const breaches = (errors, prefix) => errors.map(e => `${prefix}${e.path === '/' ? '' : e.path}: ${e.message}`)

/**
 * Check a core 1.0 document; returns it, or throws a PayloadError —
 * the same verdicts the Python core gives (core-py, held to the shared vectors):
 * `unsupported_version` for a domain or a version this build does not read,
 * `invalid_payload` (with its `details`) for anything else that is not as the
 * schemas say.
 */
export function checkDocument(doc) {
  if (doc?.format !== FORMAT) throw new PayloadError('invalid_payload', ['/format: is not rail-across'])
  const shape = SCHEMAS.check('rail-across', doc)
  if (shape.length) throw new PayloadError('invalid_payload', breaches(shape, ''))
  for (const [hash, schema] of Object.entries(doc.schemas ?? {})) {
    if (contentHash(schema) !== hash) throw new PayloadError('invalid_payload', [`/schemas/${hash}: holds another schema`])
  }
  for (const [hash, value] of Object.entries(doc.blobs)) {
    if (contentHash(value) !== hash) throw new PayloadError('invalid_payload', [`/blobs/${hash}: holds other content`])
  }
  const found = []
  for (const [name, domain] of Object.entries(doc.domains)) {
    if (DOMAIN_VERSIONS[name] !== domain.version) throw new PayloadError('unsupported_version', [`/domains/${name}: ${domain.version}`])
    if (domain.schema !== SCHEMA_HASHES[name]) {
      found.push(`/domains/${name}/schema: is not the schema of ${name} ${domain.version}`)
      continue
    }
    const deps = domain.dependsOn ?? []
    if (name !== 'core' && !(deps.length === 1 && deps[0] === 'core')) found.push(`/domains/${name}/dependsOn: is not ["core"]`)
    const { fields, collections } = domain
    found.push(...breaches(SCHEMAS.check(name, { fields, collections }), `/domains/${name}`))
    for (const [cname, list] of Object.entries(collections)) {
      const seen = new Set()
      for (const obj of list) {
        if (seen.has(obj.id)) found.push(`/domains/${name}/collections/${cname}: holds ${obj.id} twice`)
        seen.add(obj.id)
      }
    }
  }
  if (found.length) throw new PayloadError('invalid_payload', found)
  return doc
}

/**
 * The core 1.0 document of a record (dehydrated), checked as the door checks
 * what comes in. `carrySchemas` puts the domains' schemas into it, as a file
 * that travels carries them (R3); a device or the server that has them
 * leaves them out.
 */
export function toDocument(record, { carrySchemas = false } = {}) {
  const doc = recordToDocument(record, SCHEMA_HASHES)
  if (carrySchemas) doc.schemas = Object.fromEntries(Object.keys(doc.domains).map(d => [SCHEMA_HASHES[d], DOMAIN_SCHEMAS[d]]))
  return checkDocument(doc)
}

/** The record a core 1.0 document holds, once the door has checked it. */
export const fromDocument = (doc) => documentToRecord(checkDocument(doc))

/** The schema of a domain this build reads, by name ("core", "layout", "signalling"), and the envelope's ("rail-across"). */
export const schemaOf = (name) => (name === 'rail-across' ? ENVELOPE : DOMAIN_SCHEMAS[name])
