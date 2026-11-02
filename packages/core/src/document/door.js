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

/**
 * Signalling 1.0 as it was written before, by the hash its schema went by, and
 * the step that brings a domain of it one schema on — the steps run until the
 * domain is of this build's (S-R10). Documents of them are in the suite's
 * stores; the door reads them as of now.
 *
 * - First: a signal's `system` and `type` become its one logical signal (a
 *   shunting signal of no system one of `sh`), a PZB element a balise of the
 *   system `pzb` with its values as settings.
 * - Second: the speed sections come, none yet.
 * - Third: the speed sections go again — a speed is signalled now, per route
 *   at its start signal; a signal's function, screen and indicators become
 *   its first logical signal's, and a route's speed that logical signal's
 *   signalling speed for it. A signal of no system keeps them.
 */
const SIGNALLING_FIRST = 'sha256:8e4fdf8b8d487f61be93b473521515fe2e5510ef3cdc21d994cd8a405aeb74a0'
const SIGNALLING_SECOND = 'sha256:72fccdfdcac3f5ce42d7f49d10fb9134e1c4ed792f7365ec9f01d997f0049724'
const SIGNALLING_THIRD = 'sha256:0194c1e7fe2a63741270025beb48929d106d74a80bf5b33ed4305965a5303487'

/** The frames a logical signal shows; the rest stay its place's. */
const OF_LOGICAL = new Set(['screen', 'indicator'])

const defined = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined))

const SIGNALLING_STEPS = {
  [SIGNALLING_FIRST]: (domain) => {
    const { signals = [], pzbElements = [], ...rest } = domain.collections ?? {}
    const logicalOf = ({ system, type }) => (system ? [type ? { system, type } : { system }] : type === 'shunting' ? [{ system: 'sh', type }] : undefined)
    return {
      ...domain,
      schema: SIGNALLING_SECOND,
      collections: {
        ...rest,
        signals: signals.map(({ system, type, ...s }) => defined({ ...s, logicalSignals: logicalOf({ system, type }) })),
        balises: pzbElements.map(({ frequency, ina, speedCheck, ...p }) => defined({
          ...p, system: 'pzb', settings: defined({ frequency, ina, ...speedCheck }),
        })),
      },
    }
  },
  [SIGNALLING_SECOND]: (domain) => ({
    ...domain, schema: SIGNALLING_THIRD, collections: { ...domain.collections, speedSections: [] },
  }),
  [SIGNALLING_THIRD]: (domain) => {
    const { speedSections: _gone, signals = [], routes = [], ...rest } = domain.collections ?? {}
    const speeds = new Map()   // start signal → [{ routeId, speed }]
    for (const r of routes) {
      if (r.speed == null) continue
      if (!speeds.has(r.startSignalId)) speeds.set(r.startSignalId, [])
      speeds.get(r.startSignalId).push({ routeId: r.id, speed: r.speed })
    }
    return {
      ...domain,
      schema: SCHEMA_HASHES.signalling,
      collections: {
        ...rest,
        signals: signals.map((s) => {
          if (!s.logicalSignals?.length) return s
          const { function: fn, frames = [], logicalSignals: [first, ...others], ...place } = s
          const own = frames.filter(f => !OF_LOGICAL.has(f.kind))
          const shown = frames.filter(f => OF_LOGICAL.has(f.kind))
          return defined({
            ...place,
            frames: own.length || s.kind === 'real' ? own : undefined,
            logicalSignals: [defined({
              ...first, function: fn, frames: shown.length ? shown : undefined,
              signallingSpeeds: speeds.get(s.id),
            }), ...others],
          })
        }),
        routes: routes.map(({ speed: _s, ...r }) => r),
      },
    }
  },
}

/**
 * Core 1.0 as it was written before the design speed took its name: an
 * element's `speed` is its `designSpeed` now (S-R10).
 */
const CORE_BEFORE_DESIGN_SPEED = 'sha256:c237af09e3ac3915398d9c7795088518df4023bfd0b954053dc9f04112fbbaf6'

function coreWithDesignSpeed(domain) {
  const tracks = (domain.collections?.tracks ?? []).map(t => (t.elements
    ? { ...t, elements: t.elements.map(el => {
      if (!Object.hasOwn(el, 'speed')) return el
      const { speed, ...rest } = el
      return { ...rest, designSpeed: speed }
    }) }
    : t))
  return { ...domain, schema: SCHEMA_HASHES.core, collections: { ...domain.collections, tracks } }
}

/** The document with every domain of an earlier schema of its version brought to this build's. */
function migrated(doc) {
  let { core, signalling } = doc.domains
  if (core?.version === '1.0' && core.schema === CORE_BEFORE_DESIGN_SPEED) core = coreWithDesignSpeed(core)
  if (signalling?.version === '1.0') while (SIGNALLING_STEPS[signalling.schema]) signalling = SIGNALLING_STEPS[signalling.schema](signalling)
  if (core === doc.domains.core && signalling === doc.domains.signalling) return doc
  return { ...doc, domains: { ...doc.domains, core, ...(signalling ? { signalling } : {}) } }
}

/** The breaches of a schema as "path: what" lines, the path prefixed with where the value lies. */
const breaches = (errors, prefix) => errors.map(e => `${prefix}${e.path === '/' ? '' : e.path}: ${e.message}`)

/**
 * Check a core 1.0 document; returns it — a domain of an earlier schema of
 * its version brought to this build's —, or throws a PayloadError —
 * the same verdicts the Python core gives (core-py, held to the shared vectors):
 * `unsupported_version` for a domain or a version this build does not read,
 * `invalid_payload` (with its `details`) for anything else that is not as the
 * schemas say.
 */
export function checkDocument(given) {
  if (given?.format !== FORMAT) throw new PayloadError('invalid_payload', ['/format: is not rail-across'])
  const shape = SCHEMAS.check('rail-across', given)
  if (shape.length) throw new PayloadError('invalid_payload', breaches(shape, ''))
  const doc = migrated(given)
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
