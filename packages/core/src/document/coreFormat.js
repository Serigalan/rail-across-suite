import { contentHash, derivedId } from './canonical.js'
import { PayloadError } from './payloadError.js'

// The record the Layout-Tool works on, and the core 1.0 document it is
// kept as — both ways, nothing lost.
//
// The record is oltjson-1.1 (apps/layout-tool/docs/data-format.md): one
// object with the project's fields and its collections. The document holds
// the same in the shape of the data model (docs/data-model.md): two domains,
// `core` and `layout`, each with its fields and its collections; locations as
// one value each (`at`, R5); the planning status as a lifespan over the
// construction stages (R12); station names and codes as an operational point
// register (R10); named runs of tracks as `trackPaths` (TrackPath); the bulk
// point arrays as blobs named by their hash (S-R2). Turning a record into a
// document is the first step of the migration chain (oltjson-1.1 → core 1.0).
//
// Until the app holds the stages and the register itself, the document's are
// drawn from the record: two stages (existing, final) with ids drawn from the
// project's id, and one operational point per station the tracks and
// platforms name, its id drawn from the name. Which is why a document with
// other stages is refused here (`stages_unsupported`).
//
// Where the two forms say the same thing in two ways, the document says it in
// one: an explicit status `existing` is no lifespan, an empty station text no
// station, `null` in a field of a track, switch, platform or end mark an
// absent field (in an element, `null` keeps its meaning: a straight end).

/** The domains this build reads and writes, with their versions. */
export const DOMAIN_VERSIONS = { core: '1.0', layout: '1.0' }

/** The fields of the record and the domain each goes to. */
const CORE_FIELDS = ['id', 'title', 'description', 'creator', 'createdAt', 'crsSystem', 'lineCategory', 'gaugeProfile']
const LAYOUT_FIELDS = ['planHeader', 'image', 'imageHash', 'osrd']
const COLLECTIONS = ['tracks', 'switches', 'platforms', 'endMarks', 'routes', 'kmLines', 'axisSurveys', 'referenceAxes']
/** Collections a record always has, empty or not; the others only where they hold something. */
const ALWAYS = new Set(['tracks', 'switches', 'platforms'])

const byId = (a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)

const pick = (obj, keys) => Object.fromEntries(keys.filter(k => obj[k] != null).map(k => [k, obj[k]]))

/** `obj` without its members that are null or undefined. */
const withoutNull = (obj) => Object.fromEntries(Object.entries(obj).filter(([, v]) => v != null))

/** A text, or nothing where it is empty. */
const text = (v) => (v == null || String(v).trim() === '' ? undefined : v)

// ── Construction stages ─────────────────────────────────────────────────────

/** The two stages of a project that has none of its own: existing and final, ids drawn from the project's. */
export const defaultStages = (projectId) => [
  { id: derivedId(`${projectId}\u0000stage\u0000existing`) },
  { id: derivedId(`${projectId}\u0000stage\u0000final`) },
]

function lifespanOf(status, final) {
  if (status === 'new') return { from: final }
  if (status === 'removal') return { to: final }
  return undefined
}

function statusOf(lifespan, final) {
  if (!lifespan) return undefined
  const keys = Object.keys(lifespan)
  if (keys.length === 1 && lifespan.from === final) return 'new'
  if (keys.length === 1 && lifespan.to === final) return 'removal'
  throw new PayloadError('stages_unsupported')
}

// ── Operational points ──────────────────────────────────────────────────────

/**
 * The register the tracks' and platforms' station texts make, and which point
 * each of them names: { points, pointOf(kind, obj) }. One point per station
 * name, holding the codes stated for it; where two objects state different
 * codes under one name, each code its own point.
 */
function operationalRegister(projectId, tracks, platforms) {
  const named = [
    ...tracks.map(t => ({ obj: t, name: text(t.stationName), uic: text(t.uicStation), rl100: undefined })),
    ...platforms.map(p => ({ obj: p, name: text(p.stationName), uic: undefined, rl100: text(p.code) })),
  ].filter(e => e.name !== undefined || e.uic !== undefined || e.rl100 !== undefined)
  const groups = new Map()
  for (const e of named) {
    const key = e.name !== undefined ? `name\u0000${e.name}` : `code\u0000${e.uic ?? ''}\u0000${e.rl100 ?? ''}`
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key).push(e)
  }
  const points = new Map()
  const of = new Map()
  for (const [key, entries] of groups) {
    const distinct = (k) => new Set(entries.map(e => e[k]).filter(v => v !== undefined).map(String))
    const agree = distinct('uic').size <= 1 && distinct('rl100').size <= 1
    for (const e of entries) {
      const own = agree ? key : `${key}\u0000${e.uic ?? ''}\u0000${e.rl100 ?? ''}`
      if (!points.has(own)) {
        const from = agree ? entries : [e]
        points.set(own, withoutNull({
          id: derivedId(`${projectId}\u0000operationalPoint\u0000${own}`),
          name: e.name,
          rl100: from.find(x => x.rl100 !== undefined)?.rl100,
          uic: from.find(x => x.uic !== undefined)?.uic,
        }))
      }
      of.set(e.obj, points.get(own).id)
    }
  }
  return { points: [...points.values()], pointOf: (obj) => of.get(obj) }
}

// ── Record → document ───────────────────────────────────────────────────────

/**
 * The core 1.0 document of a record (dehydrated, as persistenceUtils keeps
 * it): { format, domains: { core, layout }, blobs }, every collection sorted by
 * id. `schemaHashes` names the schema of each domain ({ core, layout }); the
 * door adds the schemas themselves where a file should carry them.
 *
 * Throws a PayloadError ('invalid_payload') for a field the format does not
 * have — a record is turned into a document whole or not at all.
 */
export function recordToDocument(record, schemaHashes) {
  const known = new Set([...CORE_FIELDS, ...LAYOUT_FIELDS, ...COLLECTIONS])
  const unknown = Object.keys(record ?? {}).filter(k => !known.has(k) && record[k] !== undefined)
  if (!record || typeof record !== 'object' || record.id == null) throw new PayloadError('invalid_payload', ['/: lacks id'])
  if (unknown.length) throw new PayloadError('invalid_payload', unknown.map(k => `/${k}: is not a field of the record`))

  const stages = defaultStages(record.id)
  const final = stages[1].id
  const tracks = record.tracks ?? []
  const platforms = record.platforms ?? []
  const register = operationalRegister(record.id, tracks, platforms)
  const blobs = {}
  const blob = (value) => {
    const hash = contentHash(value)
    blobs[hash] = value
    return hash
  }
  const located = (obj, extra) => withoutNull({
    ...extra,
    operationalPoint: register.pointOf(obj),
    lifespan: lifespanOf(obj.status, final),
  })

  const core = {
    version: DOMAIN_VERSIONS.core,
    schema: schemaHashes.core,
    fields: { ...pick(record, CORE_FIELDS), stages },
    collections: {
      tracks: tracks.map((t) => {
        const { status: _s, stationName: _n, uicStation: _u, ...rest } = t
        return located(t, withoutNull(rest))
      }),
      switches: (record.switches ?? []).map(({ switchId, status, ...sw }) => withoutNull({
        id: switchId, ...sw, lifespan: lifespanOf(status, final),
      })),
      platforms: platforms.map((p) => {
        const { trackId, startStation, endStation, side, status: _s, stationName: _n, code: _c, ...rest } = p
        return located(p, {
          at: withoutNull({ anchor: 'station', trackId, startStation, endStation, side }),
          ...withoutNull(rest),
        })
      }),
      endMarks: (record.endMarks ?? []).map((m) => {
        const { trackId, endpoint, status: _s, ...rest } = m
        return located(m, { ...withoutNull(rest), at: withoutNull({ trackId, endpoint }) })
      }),
      kmLines: [...(record.kmLines ?? [])],
      operationalPoints: register.points,
    },
  }
  const layout = {
    version: DOMAIN_VERSIONS.layout,
    schema: schemaHashes.layout,
    dependsOn: ['core'],
    fields: pick(record, LAYOUT_FIELDS),
    collections: {
      trackPaths: (record.routes ?? []).map(({ trackIds, ...r }) => withoutNull({ ...r, path: { trackIds: trackIds ?? [] } })),
      axisSurveys: (record.axisSurveys ?? []).map(s => ({ ...s, points: blob(s.points) })),
      referenceAxes: (record.referenceAxes ?? []).map(a => ({ ...a, points: blob(a.points) })),
    },
  }
  for (const domain of [core, layout]) {
    for (const name of Object.keys(domain.collections)) domain.collections[name] = [...domain.collections[name]].sort(byId)
  }
  return { format: 'rail-across', domains: { core, layout }, blobs }
}

// ── Document → record ───────────────────────────────────────────────────────

/**
 * The record a core 1.0 document holds — the inverse of recordToDocument. The
 * document has been through the door (shape, schemas and blob hashes
 * checked); what is checked here is what this build can hold: two stages
 * only, every station a point of the register, every blob an object names
 * there.
 */
export function documentToRecord(doc) {
  const core = doc.domains.core
  const layout = doc.domains.layout ?? { fields: {}, collections: {} }
  const stages = core.fields.stages
  if (stages.length !== 2) throw new PayloadError('stages_unsupported')
  const final = stages[1].id
  const points = new Map((core.collections.operationalPoints ?? []).map(p => [p.id, p]))
  const pointOf = (obj) => {
    if (obj.operationalPoint == null) return {}
    const p = points.get(obj.operationalPoint)
    if (!p) throw new PayloadError('invalid_payload', [`/operationalPoints: lacks ${obj.operationalPoint}`])
    return p
  }
  const blob = (hash) => {
    if (!Object.hasOwn(doc.blobs ?? {}, hash)) throw new PayloadError('invalid_payload', [`/blobs: lacks ${hash}`])
    return doc.blobs[hash]
  }
  const status = (obj) => statusOf(obj.lifespan, final)

  const c = core.collections
  const l = layout.collections ?? {}
  const collections = {
    tracks: (c.tracks ?? []).map(({ lifespan: _l, operationalPoint: _o, ...t }, i, all) => {
      const p = pointOf(all[i])
      return withoutNull({ ...t, stationName: p.name, uicStation: p.uic, status: status(all[i]) })
    }),
    switches: (c.switches ?? []).map(({ id, lifespan: _l, ...sw }, i, all) => withoutNull({
      switchId: id, ...sw, status: status(all[i]),
    })),
    platforms: (c.platforms ?? []).map(({ at, lifespan: _l, operationalPoint: _o, ...rest }, i, all) => {
      const p = pointOf(all[i])
      const { anchor: _a, ...loc } = at
      return withoutNull({ ...rest, ...loc, stationName: p.name, code: p.rl100, status: status(all[i]) })
    }),
    endMarks: (c.endMarks ?? []).map(({ at, lifespan: _l, ...rest }, i, all) => withoutNull({
      ...rest, ...at, status: status(all[i]),
    })),
    routes: (l.trackPaths ?? []).map(({ path, ...r }) => ({ ...r, trackIds: path.trackIds })),
    kmLines: c.kmLines ?? [],
    axisSurveys: (l.axisSurveys ?? []).map(s => ({ ...s, points: blob(s.points) })),
    referenceAxes: (l.referenceAxes ?? []).map(a => ({ ...a, points: blob(a.points) })),
  }
  const { stages: _st, ...coreFields } = core.fields
  const record = { ...coreFields, ...(layout.fields ?? {}) }
  for (const name of COLLECTIONS) {
    if (ALWAYS.has(name) || collections[name].length) record[name] = collections[name]
  }
  return record
}
