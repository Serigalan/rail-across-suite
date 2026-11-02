import { newId } from './domain.js'

/**
 * The stations, as the tool edits them: points of the core's operational
 * point register (R10), which the tool writes as well as the signalling —
 * the points the record keeps (`operationalPoints`) and which station each
 * platform is of. The register itself is the core's: the points the tracks'
 * and platforms' station names make and those the record keeps
 * (projectDocument); the tool reads it from there (`register`, the core
 * domain's `operationalPoints`) and writes what makes it.
 *
 * A platform is of a station by its name, and states the station's code
 * (`stationName`, `code`); a track the same by its name (`stationName`), which
 * the Layout-Tool sets. A signal is of a station where it names it
 * (`operationalPointId`), else of the open line; what bounds a station towards
 * the open line is its boundary (`stationBoundaries`): its entry signals or
 * boards Ne 1, else its entry switches (EBO §4).
 */

/** The functions of a signal that leads into a station. */
const ENTRY = new Set(['entry', 'entryExit', 'entryBlock'])

/** Whether a signal leads into its station: an entry signal, or a board Ne 1. */
export function marksEntry(signal) {
  const fns = [signal.function, ...(signal.logicalSignals ?? []).map(l => l.function)]
  return fns.some(fn => ENTRY.has(fn)) || (signal.frames ?? []).some(f => f.aspects.some(a => a.aspect === 'Ne_1'))
}

/**
 * What the core says of each station: id → { point, tracks, platforms } — the
 * tracks and platforms of it by id.
 */
export function stationsOf(core) {
  const c = core?.collections ?? {}
  const out = new Map((c.operationalPoints ?? []).map(p => [p.id, { point: p, tracks: [], platforms: [] }]))
  for (const tr of c.tracks ?? []) if (tr.operationalPoint) out.get(tr.operationalPoint)?.tracks.push(tr.id)
  for (const pf of c.platforms ?? []) if (pf.operationalPoint) out.get(pf.operationalPoint)?.platforms.push(pf.id)
  return out
}

/** A station's name as a list shows it. */
export const stationName = (point) => point?.name ?? point?.rl100 ?? point?.uic ?? '—'

/** Whether a station of that name is in the register already — but `except`. */
export const nameTaken = (register, name, except = null) => {
  const n = String(name ?? '').trim().toLocaleLowerCase('de')
  return register.some(p => p.id !== except && String(p.name ?? '').trim().toLocaleLowerCase('de') === n)
}

const withCollections = (state, patch) => ({ ...state, collections: { ...state.collections, ...patch } })

/** A new station of the register: its name and, where there is one, its RL100 code. */
export const newStation = ({ name, rl100 }) => withoutEmpty({ id: newId(), name: name.trim(), rl100: rl100?.trim() })

/** The state with a station kept as `point` says: a new one, renamed or with other codes — its platforms naming it so. */
export function withStation(state, point, platformIds = []) {
  const kept = state.collections.operationalPoints
  const next = withoutEmpty({ id: point.id, name: point.name?.trim(), rl100: point.rl100?.trim(), uic: point.uic })
  const mine = new Set(platformIds)
  return withCollections(state, {
    operationalPoints: kept.some(p => p.id === point.id) ? kept.map(p => (p.id === point.id ? next : p)) : [...kept, next],
    platforms: state.collections.platforms.map(pf => (mine.has(pf.id) ? platformOf(pf, next) : pf)),
  })
}

/** A platform of a station — or, `point` null, of none: its name, and its code. */
function platformOf(platform, point) {
  const { stationName: _n, code: _c, ...rest } = platform
  if (!point) return rest
  return withoutEmpty({ ...rest, stationName: point.name, code: point.rl100 ?? platform.code })
}

/** The state with the platforms of `ids` of the station `point`, those of `before` not of it any more. */
export function withPlatforms(state, point, ids, before) {
  const now = new Set(ids)
  const gone = new Set(before.filter(id => !now.has(id)))
  const added = new Set(ids.filter(id => !before.includes(id)))
  return withCollections(state, {
    platforms: state.collections.platforms.map(pf => (added.has(pf.id) ? platformOf(pf, point) : gone.has(pf.id) ? platformOf(pf, null) : pf)),
  })
}

/** The state with the signals of `ids` of the station `pointId`; those of it before and not now of the open line. */
export function withSignals(state, pointId, ids) {
  const now = new Set(ids)
  return withCollections(state, {
    signals: state.collections.signals.map((s) => {
      if (now.has(s.id)) return s.operationalPointId === pointId ? s : { ...s, operationalPointId: pointId }
      if (s.operationalPointId !== pointId) return s
      const { operationalPointId: _o, ...rest } = s
      return rest
    }),
  })
}

/** A station's boundary, or none. */
export const boundaryOf = (state, pointId) => state.collections.stationBoundaries.find(b => b.operationalPointId === pointId) ?? null

/** The state with the boundary of a station marked by `marks` — none where there are none. */
export function withBoundary(state, pointId, marks) {
  const list = state.collections.stationBoundaries
  const was = boundaryOf(state, pointId)
  if (!marks.length) return withCollections(state, { stationBoundaries: list.filter(b => b !== was) })
  if (!was) return withCollections(state, { stationBoundaries: [...list, { id: newId(), operationalPointId: pointId, marks }] })
  return withCollections(state, { stationBoundaries: list.map(b => (b === was ? { ...b, marks } : b)) })
}

/** The marks of a station's boundary as the rule gives them: its signals that lead into it — entry signals and boards Ne 1. */
export function entryMarks(state, pointId) {
  return state.collections.signals.filter(s => s.operationalPointId === pointId && marksEntry(s)).map(s => ({ signalId: s.id }))
}

/**
 * The state without a station that nothing but the tool keeps: its kept
 * point, its platforms of no station, its signals of the open line, its
 * boundary gone.
 */
export function withoutStation(state, pointId, platformIds = []) {
  const gone = withCollections(withPlatforms(withSignals(state, pointId, []), null, [], platformIds), {
    operationalPoints: state.collections.operationalPoints.filter(p => p.id !== pointId),
  })
  return withBoundary(gone, pointId, [])
}

const withoutEmpty = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined && v !== null && v !== ''))
