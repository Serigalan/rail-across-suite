import { CATALOGUE_REF, signalKindOf } from '@rail-across/core/signalling'

/**
 * The signalling domain as the tool edits it: pure functions from one state
 * of the domain to the next. Ids are new UUIDs; a deletion cascades within
 * the domain (R7) — a route goes with its start or target signal, a section
 * loses the point that bounded it — and leaves the core alone.
 */

export const COLLECTIONS = [
  'signals', 'detectionPoints', 'trackSections', 'switchElements', 'routes', 'overlaps', 'flankProtection', 'pzbElements',
  'stationBoundaries',
]

/** An empty domain, planned against the catalogue this build has. */
export const emptyDomain = () => ({
  fields: { catalogue: { ...CATALOGUE_REF } },
  collections: Object.fromEntries(COLLECTIONS.map(c => [c, []])),
})

/** The domain of a record, or an empty one; every collection there. */
export function domainOf(record) {
  const d = record?.signalling
  if (!d) return emptyDomain()
  return { fields: d.fields ?? {}, collections: Object.fromEntries(COLLECTIONS.map(c => [c, d.collections?.[c] ?? []])) }
}

export const newId = () => (globalThis.crypto?.randomUUID?.() ?? `id-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`)

const withCollection = (domain, name, list) => ({ ...domain, collections: { ...domain.collections, [name]: list } })

/** The domain with `obj` added to `collection`. */
export const added = (domain, collection, obj) => withCollection(domain, collection, [...domain.collections[collection], obj])

/** The domain with the object of `obj.id` in `collection` replaced by `obj`. */
export const replaced = (domain, collection, obj) =>
  withCollection(domain, collection, domain.collections[collection].map(o => (o.id === obj.id ? obj : o)))

/**
 * What a deletion takes with it, within the domain: { collection: [ids] } —
 * routes from or to a deleted signal, overlaps at it, flank protection given
 * by it or asked by a deleted switch element; objects that only name it lose
 * the name (a PZB element its signal, a section the point).
 */
export function cascadeOf(domain, collection, id) {
  const c = domain.collections
  const gone = { [collection]: [id] }
  const add = (name, ids) => { if (ids.length) gone[name] = [...new Set([...(gone[name] ?? []), ...ids])] }
  if (collection === 'signals') {
    add('routes', c.routes.filter(r => r.startSignalId === id || r.targetSignalId === id).map(r => r.id))
    add('overlaps', c.overlaps.filter(o => o.signalId === id).map(o => o.id))
    add('flankProtection', c.flankProtection.filter(f => f.protection.signalId === id).map(f => f.id))
  }
  if (collection === 'switchElements') {
    add('flankProtection', c.flankProtection.filter(f => f.requester.switchElementId === id || f.protection.switchElementId === id).map(f => f.id))
  }
  return gone
}

/** The domain without the object and what goes with it (cascadeOf); names of it elsewhere dropped. */
export function deleted(domain, collection, id) {
  const gone = cascadeOf(domain, collection, id)
  const is = (name, oid) => gone[name]?.includes(oid)
  const c = Object.fromEntries(COLLECTIONS.map(name => [name, domain.collections[name].filter(o => !is(name, o.id))]))
  c.trackSections = c.trackSections.map(t => (t.boundedBy.some(p => is('detectionPoints', p)) ? { ...t, boundedBy: t.boundedBy.filter(p => !is('detectionPoints', p)) } : t))
  c.routes = c.routes.map(r => {
    const overlaps = r.overlaps?.filter(o => !is('overlaps', o.overlapId))
    const aspects = r.aspects?.filter(a => !is('signals', a.signalId))
    return overlaps?.length !== r.overlaps?.length || aspects?.length !== r.aspects?.length ? { ...r, overlaps, aspects } : r
  })
  c.pzbElements = c.pzbElements.map(p => (is('signals', p.signalId) ? { ...p, signalId: undefined } : p))
  c.switchElements = c.switchElements.map(e => (is('signals', e.clearanceMarkerId) ? { ...e, clearanceMarkerId: undefined } : e))
  c.stationBoundaries = c.stationBoundaries
    .map(b => ({ ...b, marks: b.marks.filter(m => !is('signals', m.signalId)) }))
    .filter(b => b.marks.length)
  return { ...domain, collections: c }
}

/**
 * A new real signal at a location: of a kind of a system, its screen showing
 * what the kind must show — the catalogue's `pflicht` and the first of its
 * proceed aspects.
 */
export function newSignal({ at, system = 'ks', type = 'multiSection', fn = 'entry', name }) {
  const kind = signalKindOf(system, type)
  const shows = [...(kind?.schirm.pflicht ?? [])]
  for (const a of kind?.schirm.fahrt ?? []) if (!shows.includes(a)) shows.push(a)
  return {
    id: newId(), name, kind: 'real', function: fn, type, system, mounting: 'mast', at,
    frames: shows.length ? [{ kind: 'screen', aspects: shows.map(aspect => ({ aspect })) }] : [],
  }
}

/** A signal of another kind: its screen keeps what the new kind may show and gains what it must. */
export function withKind(signal, system, type) {
  const kind = signalKindOf(system, type)
  if (!kind) return { ...signal, system, type }
  const frames = signal.frames ?? []
  const screen = frames.find(f => f.kind === 'screen')
  const kept = (screen?.aspects ?? []).filter(a => kind.schirm.erlaubt.includes(a.aspect))
  for (const must of kind.schirm.pflicht) if (!kept.some(a => a.aspect === must)) kept.push({ aspect: must })
  const nextScreen = { kind: 'screen', aspects: kept }
  const rest = frames.filter(f => f !== screen)
  return { ...signal, system, type, frames: [nextScreen, ...rest] }
}

/** A short label of an object for lists and findings. */
export function labelOf(collection, obj, t) {
  if (!obj) return '—'
  const name = obj.name != null && obj.name !== '' ? String(obj.name) : null
  switch (collection) {
    case 'signals': return name ?? (obj.frames?.some(f => f.kind === 'screen') ? null : boardName(obj)) ?? t('unnamed')
    case 'routes': return name ?? null
    default: return name ?? t('unnamed')
  }
}

/** What a board without a name shows: its first aspect, e.g. "Lf 7 (8)". */
function boardName(signal) {
  const a = signal.frames?.[0]?.aspects?.[0]
  if (!a) return null
  return `${a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`
}
