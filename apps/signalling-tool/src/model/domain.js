import { stationRegister } from '@rail-across/core/document'
import { CATALOGUE_REF, baliseKindOf, protectionSystemOf, signalKindOf } from '@rail-across/core/signalling'

/**
 * The signalling domain as the tool edits it: pure functions from one state
 * of the domain to the next. Ids are new UUIDs; a deletion cascades within
 * the domain (R7) — a route goes with its start or target signal, a section
 * loses the point that bounded it — and leaves the core alone.
 *
 * A signal is a place (Standort); what it is in a signalling system is a
 * logical signal of it, which has no id of its own: the tool names it by its
 * place and its system (logicalId), as a place has each system once.
 */

export const COLLECTIONS = [
  'signals', 'detectionPoints', 'trackSections', 'switchElements', 'routes', 'overlaps', 'flankProtection', 'balises',
  'stationBoundaries', 'stoppingPlaces', 'itineraries',
]

/**
 * The collections of the record's core the tool writes as well: the points of
 * the operational point register the record keeps, and the platforms, which
 * name their station (model/stations.js). The tool's state holds them beside
 * the domain's, so an edit of a station is one step with the rest.
 */
export const CORE_PARTS = ['operationalPoints', 'platforms']

/** An empty domain, planned against the catalogue this build has. */
export const emptyDomain = () => ({
  fields: { catalogue: { ...CATALOGUE_REF } },
  collections: Object.fromEntries(COLLECTIONS.map(c => [c, []])),
})

/**
 * The state the tool edits, of a record: its domain — or an empty one —, every
 * collection there, and the parts of the core it writes (CORE_PARTS).
 */
export function domainOf(record) {
  const d = record?.signalling ?? emptyDomain()
  return {
    fields: d.fields ?? {},
    collections: Object.fromEntries([
      ...COLLECTIONS.map(c => [c, d.collections?.[c] ?? []]),
      ...CORE_PARTS.map(c => [c, record?.[c] ?? []]),
    ]),
  }
}

/** The signalling domain of the state, without the parts of the core. */
export const signallingOf = (state) => ({ fields: state.fields, collections: Object.fromEntries(COLLECTIONS.map(c => [c, state.collections[c]])) })

/**
 * The record with the state laid on it: its domain and the parts of the core
 * the tool writes. A station renamed keeps its id: the tracks of it state its
 * new name too, as its platforms do (withStation).
 */
export function recordOf(record, state) {
  const { operationalPoints, platforms } = state.collections
  const points = new Map(operationalPoints.map(p => [p.id, p]))
  const register = stationRegister(record)
  let renamed = false
  const tracks = (record?.tracks ?? []).map(tr => {
    const p = points.get(register.pointOf(tr))
    if (!p?.name || p.name === tr.stationName) return tr
    renamed = true
    return { ...tr, stationName: p.name }
  })
  return {
    ...record, ...(renamed ? { tracks } : {}), platforms,
    operationalPoints: operationalPoints.length ? operationalPoints : undefined, signalling: signallingOf(state),
  }
}

/** The state with another domain — the parts of the core as they are. */
export const withSignalling = (state, domain) => ({ fields: domain.fields, collections: { ...state.collections, ...signallingOf(domain).collections } })

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
 * by it or asked by a deleted switch element, the itineraries over a route or
 * from or to a stopping place that goes; objects that only name it lose
 * the name (a balise its signal, a section the point, a logical signal the
 * speed or parameters it states for a route). A logical signal goes alone.
 */
export function cascadeOf(domain, collection, id) {
  if (collection === 'logicalSignals') return { logicalSignals: [id] }
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
  const routeGone = new Set(gone.routes ?? []), placeGone = new Set(gone.stoppingPlaces ?? [])
  add('itineraries', c.itineraries.filter(it => it.routeIds.some(r => routeGone.has(r))
    || placeGone.has(it.startStoppingPlaceId) || placeGone.has(it.targetStoppingPlaceId)).map(it => it.id))
  return gone
}

/** The domain without the object and what goes with it (cascadeOf); names of it elsewhere dropped. */
export function deleted(domain, collection, id) {
  if (collection === 'logicalSignals') return withLogical(domain, id, null)
  const gone = cascadeOf(domain, collection, id)
  const is = (name, oid) => gone[name]?.includes(oid)
  const c = { ...domain.collections, ...Object.fromEntries(COLLECTIONS.map(name => [name, domain.collections[name].filter(o => !is(name, o.id))])) }
  c.trackSections = c.trackSections.map(t => (t.boundedBy.some(p => is('detectionPoints', p)) ? { ...t, boundedBy: t.boundedBy.filter(p => !is('detectionPoints', p)) } : t))
  c.routes = c.routes.map(r => {
    const overlaps = r.overlaps?.filter(o => !is('overlaps', o.overlapId))
    const aspects = r.aspects?.filter(a => !is('signals', a.signalId))
    return overlaps?.length !== r.overlaps?.length || aspects?.length !== r.aspects?.length ? { ...r, overlaps, aspects } : r
  })
  c.balises = c.balises.map(b => (is('signals', b.signalId) ? { ...b, signalId: undefined } : b))
  const routeGone = (id) => is('routes', id)
  const namesGone = (l) => l.conditionalParameters?.some(p => routeGone(p.routeId)) || l.signallingSpeeds?.some(v => routeGone(v.routeId))
  c.signals = c.signals.map(s => (s.logicalSignals?.some(namesGone)
    ? { ...s, logicalSignals: s.logicalSignals.map(l => withoutEmpty({
      ...l,
      conditionalParameters: l.conditionalParameters?.filter(p => !routeGone(p.routeId)),
      signallingSpeeds: l.signallingSpeeds?.filter(v => !routeGone(v.routeId)),
    })) }
    : s))
  c.switchElements = c.switchElements.map(e => (is('signals', e.clearanceMarkerId) ? { ...e, clearanceMarkerId: undefined } : e))
  c.stationBoundaries = c.stationBoundaries
    .map(b => ({ ...b, marks: b.marks.filter(m => !is('signals', m.signalId)) }))
    .filter(b => b.marks.length)
  return { ...domain, collections: c }
}

const withoutEmpty = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined && !(Array.isArray(v) && !v.length)))

/** How the tool names a logical signal: its place and its system, which a place has once. */
export const logicalId = (signalId, system) => `${signalId}|${system}`

/** The logical signal of that name: { signal, logical, index }, or empty where there is none. */
export function findLogical(domain, id) {
  const cut = String(id ?? '').lastIndexOf('|')
  if (cut < 0) return {}
  const signalId = id.slice(0, cut), system = id.slice(cut + 1)
  const signal = domain.collections.signals.find(s => s.id === signalId)
  const index = signal?.logicalSignals?.findIndex(l => l.system === system) ?? -1
  return index >= 0 ? { signal, logical: signal.logicalSignals[index], index } : {}
}

/** Every logical signal of the domain: [{ id, signal, logical, index }], place by place. */
export const allLogical = (domain) => domain.collections.signals.flatMap(signal =>
  (signal.logicalSignals ?? []).map((logical, index) => ({ id: logicalId(signal.id, logical.system), signal, logical, index })))

/** Whether a signal shows a screen: at its place, or a logical signal of it. */
export const hasScreen = (signal) => [...(signal?.frames ?? []), ...(signal?.logicalSignals ?? []).flatMap(l => l.frames ?? [])]
  .some(f => f.kind === 'screen')

/**
 * A new logical signal of a kind of a system: its screen showing what the
 * kind must show — the catalogue's `pflicht` — and its proceed aspects.
 */
export function newLogical({ system = 'ks', type = 'multiSection', fn = 'entry' } = {}) {
  const kind = signalKindOf(system, type)
  const shows = [...(kind?.schirm.pflicht ?? [])]
  for (const a of kind?.schirm.fahrt ?? []) if (!shows.includes(a)) shows.push(a)
  return withoutEmpty({ system, type, function: fn, frames: shows.length ? [{ kind: 'screen', aspects: shows.map(aspect => ({ aspect })) }] : undefined })
}

/** A new place at a location, with what it is in one system (a Ks signal unless said otherwise), or bare; on a gantry or bracket its foundation. */
export function newSignal({ at, system = 'ks', type = 'multiSection', fn = 'entry', name, mounting = 'mast', foundation, bare = false }) {
  return withoutEmpty({
    id: newId(), name, kind: 'real', mounting, at, foundation,
    logicalSignals: bare ? undefined : [newLogical({ system, type, fn })],
  })
}

/**
 * A logical signal of another system or kind: its screen keeps what the new
 * kind may show and gains what it must, its indicators those the kind
 * carries; the speeds it signals stay.
 */
export function withKind(logical, system, type) {
  const kind = signalKindOf(system, type)
  if (!kind) return withoutEmpty({ ...logical, system, type })
  const frames = logical.frames ?? []
  const screen = frames.find(f => f.kind === 'screen')
  const kept = (screen?.aspects ?? []).filter(a => kind.schirm.erlaubt.includes(a.aspect))
  for (const must of kind.schirm.pflicht) if (!kept.some(a => a.aspect === must)) kept.push({ aspect: must })
  const indicators = frames.filter(f => f !== screen && (f.kind !== 'indicator' || f.aspects.every(a => kind.zusatz.includes(a.aspect))))
  return withoutEmpty({ ...logical, system, type, frames: [{ kind: 'screen', aspects: kept }, ...indicators] })
}

/** The domain with a logical signal of a place replaced — or, `next` null, taken away. */
export function withLogical(domain, id, next) {
  const { signal, index } = findLogical(domain, id)
  if (!signal) return domain
  const list = signal.logicalSignals.map((l, i) => (i === index ? next : l)).filter(Boolean)
  return replaced(domain, 'signals', withoutEmpty({ ...signal, logicalSignals: list }))
}

/** A logical signal with the speed it signals for a route set — or, `speed` null, taken away. */
export function withSignallingSpeed(logical, routeId, speed) {
  const rest = (logical.signallingSpeeds ?? []).filter(v => v.routeId !== routeId)
  return withoutEmpty({ ...logical, signallingSpeeds: speed == null ? rest : [...rest, { routeId, speed }] })
}

/** A speed as a signal shows it: in tens, rounded down — Zs 3 and Lf 7 show no other; null for none. */
export const shownSpeed = (speed) => (speed > 0 ? Math.max(10, Math.floor(speed / 10) * 10) : null)

/** The highest speed a Zs 3 shows [km/h]. */
const ZS3_TOP = 160

/** A logical signal's frames with a Zs 3 of that symbol among its indicators, or, `drop`, without it. */
function withZs3(frames = [], symbol, drop = false) {
  const has = (f) => f.kind === 'indicator' && f.aspects.some(a => a.aspect === 'Zs_3' && a.symbol === symbol)
  if (drop) {
    return frames.map(f => (has(f) ? { ...f, aspects: f.aspects.filter(a => !(a.aspect === 'Zs_3' && a.symbol === symbol)) } : f))
      .filter(f => f.kind !== 'indicator' || f.aspects.length)
  }
  if (frames.some(has)) return frames
  const at = frames.findIndex(f => f.kind === 'indicator' && f.aspects.some(a => a.aspect === 'Zs_3'))
  const zs3 = { aspect: 'Zs_3', symbol }
  if (at < 0) return [...frames, { kind: 'indicator', aspects: [zs3] }]
  return frames.map((f, i) => (i === at ? { ...f, aspects: [...f.aspects, zs3].sort((x, y) => Number(x.symbol ?? 0) - Number(y.symbol ?? 0)) } : f))
}

/**
 * The domain with the speed a route's start signal signals for it set, in
 * tens: at the logical signal that signals it already, else the first of
 * the place — unchanged where the place has none. A speed is shown by a
 * Zs 3 at the same signal: the logical signal gets one of that figure among
 * its indicators, the route shows it; the one it showed before goes where
 * no other route shows it. A speed above what a Zs 3 shows has none.
 * `system` names the logical signal that is to signal it, where one is asked.
 */
export function withRouteSpeed(domain, route, speed, system = null) {
  const start = domain.collections.signals.find(s => s.id === route.startSignalId)
  const setter = system ? start?.logicalSignals?.find(l => l.system === system)
    : start?.logicalSignals?.find(l => l.signallingSpeeds?.some(v => v.routeId === route.id)) ?? start?.logicalSignals?.[0]
  if (!setter) return domain
  const shown = shownSpeed(speed)
  const symbol = shown != null && shown <= ZS3_TOP ? String(shown / 10) : null
  const current = domain.collections.routes.find(r => r.id === route.id) ?? route
  const isZs3 = (a) => a.signalId === start.id && a.aspect === 'Zs_3'
  const before = (current.aspects ?? []).find(isZs3)?.symbol
  // The figure shown before stays where another route from the signal shows it.
  const othersShow = before != null && domain.collections.routes.some(r => r.id !== route.id && (r.aspects ?? []).some(a => isZs3(a) && a.symbol === before))
  let frames = setter.frames
  if (before != null && before !== symbol && !othersShow) frames = withZs3(frames, before, true)
  if (symbol) frames = withZs3(frames, symbol)
  const logical = withSignallingSpeed({ ...setter, frames: frames?.length ? frames : undefined }, route.id, shown)
  const aspects = [...(current.aspects ?? []).filter(a => !isZs3(a)), ...(symbol ? [{ signalId: start.id, aspect: 'Zs_3', symbol }] : [])]
  const next = withLogical(domain, logicalId(start.id, setter.system), logical)
  return replaced(next, 'routes', withoutEmpty({ ...current, aspects }))
}

/** Whether a route's speed is signalled as asked: that speed in tens, and its Zs 3 at its start signal. */
export function signalsSpeed(domain, route, speed) {
  const shown = shownSpeed(speed)
  const start = domain.collections.signals.find(s => s.id === route.startSignalId)
  const signalled = (start?.logicalSignals ?? []).flatMap(l => l.signallingSpeeds ?? []).find(v => v.routeId === route.id)?.speed ?? null
  if (signalled !== shown) return false
  if (shown == null || shown > ZS3_TOP) return true
  const symbol = String(shown / 10)
  const current = domain.collections.routes.find(r => r.id === route.id) ?? route
  return (current.aspects ?? []).some(a => a.signalId === start.id && a.aspect === 'Zs_3' && a.symbol === symbol)
    && (start.logicalSignals ?? []).some(l => (l.frames ?? []).some(f => f.kind === 'indicator' && f.aspects.some(a => a.aspect === 'Zs_3' && a.symbol === symbol)))
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

/** A new balise or magnet at a location: of a system and a kind (its first unless said), how it acts, the signal it belongs to. */
export function newBalise({ at, system = 'pzb', kind = Object.keys(protectionSystemOf(system).arten)[0], effect, signalId }) {
  const settings = baliseSettings(system, kind)
  return {
    id: newId(), at, system, kind, effect: effect ?? (system === 'pzb' ? 'switchedBySignal' : 'permanent'),
    ...(signalId ? { signalId } : {}),
    ...(Object.keys(settings).length ? { settings } : {}),
  }
}

/** A signal's full designation, its area's number leading (60-A): names repeat from area to area. */
export const designationOf = (s, t) => labelOf('signals', s, t)

/** A short label of an object for lists and findings; a signal's with its area's number (designationOf). */
export function labelOf(collection, obj, t) {
  if (!obj) return '—'
  const name = obj.name != null && obj.name !== '' ? String(obj.name) : null
  switch (collection) {
    case 'signals': return (name && obj.areaNumber ? `${obj.areaNumber}-${name}` : name) ?? (hasScreen(obj) ? null : boardName(obj)) ?? t('unnamed')
    case 'routes': return name ?? null
    case 'operationalPoints': return name ?? obj.rl100 ?? t('unnamed')
    case 'platforms': return [obj.stationName, obj.code].filter(Boolean).join(' ') || t('unnamed')
    default: return name ?? t('unnamed')
  }
}

/** What a board without a name shows: its first aspect, e.g. "Lf 7 (8)". */
function boardName(signal) {
  const a = signal.frames?.[0]?.aspects?.[0]
  if (!a) return null
  return `${a.aspect.replace(/_/g, ' ')}${a.symbol ? ` (${a.symbol})` : ''}`
}
