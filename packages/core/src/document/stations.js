import { viaNative } from './viaNative.js'

/**
 * The stations of a record by their ids (decision 0006). A station or halt is
 * a point of the record's operational point register (`operationalPoints`,
 * R10), an object with an id of its own; a track or platform names its point
 * by `operationalPointId`. Its name and codes are the point's, stated once. The
 * core's module answers (crates/doc, stations).
 */

/**
 * The register of a record: { points, pointOf(obj) } — every point with its
 * id, name and codes, and the id of the point a track or platform of the
 * record is of (undefined where it names none).
 */
export function stationRegister(record) {
  const r = viaNative('document.stationRegister', record ?? {})
  const of = new Map()
  ;(record?.tracks ?? []).forEach((t, i) => of.set(t, r.tracks[i] ?? undefined))
  ;(record?.platforms ?? []).forEach((p, i) => of.set(p, r.platforms[i] ?? undefined))
  return { points: r.points, pointOf: (obj) => of.get(obj) }
}

/**
 * The record with the station `id` renamed — `name`, and its `uic` number
 * where given —, keeping its id. Returns { operationalPoints }, the part it
 * changed, or null where the record has no station of that id; its tracks and
 * platforms name the point by its id and stay as they are.
 */
export const renamedStation = (record, id, { name, uic } = {}) =>
  viaNative('document.renamedStation', { record, id, name, ...(uic !== undefined ? { uic } : {}) })

/**
 * The point called `name` (and its codes `uic`, `rl100` where known): the
 * record's where it has one of that name whose codes do not say otherwise,
 * else a new one. Returns { operationalPoints, id } — the register with it,
 * and its id for a track or platform to name — or null where nothing is given.
 */
export const stationPoint = (record, { name, uic, rl100 } = {}) =>
  viaNative('document.stationPoint', { record, name, uic, rl100 })

/** The point of the record's register a track or platform names, where it names one. */
export const pointOf = (record, obj) =>
  (obj?.operationalPointId ? (record?.operationalPoints ?? []).find(p => p.id === obj.operationalPointId) : undefined)

/**
 * A record whose tracks and platforms state their station as text
 * (`stationName`, `uicStation`, a platform's `code`) — what an importer reads
 * from a file that knows no operational points — with the points those texts
 * make: `operationalPoints`, each track and platform its `operationalPointId` —
 * one stating its station as no text names none. The record's points are
 * kept, and a text names the one of its name and codes. The record itself
 * where nothing states a station as text.
 */
/**
 * The record with its tracks and platforms stating their station as text as
 * well — `stationName`, `uicStation`, a platform's `code`, from the point they
 * name —: for a file or a drawing that names stations by text, never kept.
 */
export function withStationTexts(record) {
  const points = new Map((record?.operationalPoints ?? []).map(p => [p.id, p]))
  const stated = (o, texts) => {
    const p = points.get(o?.operationalPointId)
    if (!p) return o
    return { ...o, ...Object.fromEntries(Object.entries(texts(p)).filter(([, v]) => v != null)) }
  }
  return {
    ...record,
    tracks: (record?.tracks ?? []).map(t => stated(t, p => ({ stationName: p.name, uicStation: p.uic }))),
    platforms: (record?.platforms ?? []).map(pf => stated(pf, p => ({ stationName: p.name, code: p.rl100 }))),
  }
}

export function withStationPoints(record) {
  const texts = (list, keys) => (list ?? []).some(o => keys.some(k => o && k in o))
  if (!texts(record?.tracks, ['stationName', 'uicStation']) && !texts(record?.platforms, ['stationName', 'code'])) return record
  return viaNative('document.withStationPoints', record)
}

/**
 * The record with the stopping places at the platform `platformId` as the
 * Civil-Tool sets them where it puts, moves or deletes a platform (decision
 * 0008): those at it passenger stops it names, one over it called `name` where
 * none is at it yet, those no longer at it neither — never one deleted, they
 * are for every train (crates/doc, layers).
 */
export const withPlatformStops = (record, platformId, { name } = {}) =>
  viaNative('document.withPlatformStops', { record, platformId, ...(name ? { name } : {}) })
