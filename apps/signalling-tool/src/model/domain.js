import { CATALOGUE_REF, baliseKindOf, protectionSystemOf, screenOf } from '@rail-across/core/signalling'

/**
 * The signalling domain as the tool edits it: pure functions from one state
 * of the domain to the next. Ids are new UUIDs; a deletion cascades within
 * the domain (R7) — a route goes with its start or target signal, a section
 * loses the point that bounded it — and leaves the core alone.
 */

export const COLLECTIONS = [
  'signals', 'detectionPoints', 'trackSections', 'switchElements', 'routes', 'overlaps', 'flankProtection', 'balises',
  'speedSections', 'stationBoundaries',
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
 * the name (a balise its signal, a section the point, a speed section or a
 * signal's parameters a route).
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
  c.balises = c.balises.map(b => (is('signals', b.signalId) ? { ...b, signalId: undefined } : b))
  const routeGone = (id) => is('routes', id)
  c.speedSections = c.speedSections.map(v => (v.onRoutes?.some(routeGone) ? { ...v, onRoutes: v.onRoutes.filter(r => !routeGone(r)) } : v))
  c.signals = c.signals.map(s => (s.logicalSignals?.some(l => l.conditionalParameters?.some(p => routeGone(p.routeId)))
    ? { ...s, logicalSignals: s.logicalSignals.map(l => ({ ...l, conditionalParameters: l.conditionalParameters?.filter(p => !routeGone(p.routeId)) })) }
    : s))
  c.switchElements = c.switchElements.map(e => (is('signals', e.clearanceMarkerId) ? { ...e, clearanceMarkerId: undefined } : e))
  c.stationBoundaries = c.stationBoundaries
    .map(b => ({ ...b, marks: b.marks.filter(m => !is('signals', m.signalId)) }))
    .filter(b => b.marks.length)
  return { ...domain, collections: c }
}

/**
 * A new real signal at a location: one logical signal of a kind of a system,
 * its screen showing what the kind must show — the catalogue's `pflicht` and
 * its proceed aspects.
 */
export function newSignal({ at, system = 'ks', type = 'multiSection', fn = 'entry', name }) {
  const logicalSignals = [{ system, type }]
  const kind = screenOf(logicalSignals)
  const shows = [...(kind?.pflicht ?? [])]
  for (const a of kind?.fahrt.flat() ?? []) if (!shows.includes(a)) shows.push(a)
  return {
    id: newId(), name, kind: 'real', function: fn, logicalSignals, mounting: 'mast', at,
    frames: shows.length ? [{ kind: 'screen', aspects: shows.map(aspect => ({ aspect })) }] : [],
  }
}

/**
 * A signal with other logical signals: its screen keeps what they may show
 * together and gains what they must; with none of a known kind it stays.
 */
export function withLogicalSignals(signal, logicalSignals) {
  const next = { ...signal, logicalSignals: logicalSignals.length ? logicalSignals : undefined }
  const kind = screenOf(logicalSignals)
  if (!kind) return next
  const frames = signal.frames ?? []
  const screen = frames.find(f => f.kind === 'screen')
  const kept = (screen?.aspects ?? []).filter(a => kind.erlaubt.includes(a.aspect))
  for (const must of kind.pflicht) if (!kept.some(a => a.aspect === must)) kept.push({ aspect: must })
  return { ...next, frames: [{ kind: 'screen', aspects: kept }, ...frames.filter(f => f !== screen)] }
}

/**
 * The settings a balise of that system and kind starts with, or keeps when
 * it becomes one: what it had that the kind allows, and for what the kind
 * asks the catalogue's `vorgabe`, else the first choice of a setting that is one.
 */
export function baliseSettings(system, kind, settings = {}) {
  const declared = protectionSystemOf(system)?.einstellungen ?? {}
  const art = baliseKindOf(system, kind)
  if (!art) return settings
  const out = Object.fromEntries(Object.entries(settings).filter(([name]) => art.erlaubt.includes(name) && declared[name]))
  for (const name of art.pflicht) {
    const setting = declared[name]
    if (out[name] === undefined && setting) out[name] = setting.vorgabe ?? (setting.typ === 'auswahl' ? Object.keys(setting.werte)[0] : undefined)
    if (out[name] === undefined) delete out[name]
  }
  return out
}

/** A new balise or magnet at a location: of a system and its first kind. */
export function newBalise({ at, system = 'pzb' }) {
  const kind = Object.keys(protectionSystemOf(system).arten)[0]
  const settings = baliseSettings(system, kind)
  return {
    id: newId(), at, system, kind, effect: system === 'pzb' ? 'switchedBySignal' : 'permanent',
    ...(Object.keys(settings).length ? { settings } : {}),
  }
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
