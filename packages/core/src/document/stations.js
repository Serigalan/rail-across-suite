import { viaNative } from './viaNative.js'

/**
 * The stations of a record by their ids. A station or halt is a point of the
 * operational point register (R10), and its id is what names it; its name is
 * only what it is shown by. The register is the one the record's document
 * keeps: the points the tracks' and platforms' station texts make — a new
 * one's id drawn from the project and its name, the same on every variant —,
 * and those the record keeps, among them every station renamed. The core's
 * module works it out (crates/doc; decision 0021, phase D).
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
 * where given —, keeping its id: the register keeps the point under its new
 * name, and its tracks and platforms state it. Returns { tracks, platforms,
 * operationalPoints }, the parts it changed, or null where the record has no
 * station of that id.
 */
export const renamedStation = (record, id, { name, uic } = {}) =>
  viaNative('document.renamedStation', { record, id, name, ...(uic !== undefined ? { uic } : {}) })

/** The two stages of a project that has none of its own: existing and final, ids drawn from the project's. */
export const defaultStages = (projectId) => viaNative('document.defaultStages', projectId)

/**
 * The record with the stopping places at the platform `platformId` as the
 * Civil-Tool sets them where it puts, moves or deletes a platform (decision
 * 0025): those at it passenger stops it names, one over it called `name` where
 * none is at it yet, those no longer at it neither — never one deleted, they
 * are for every train (crates/doc, layers).
 */
export const withPlatformStops = (record, platformId, { name } = {}) =>
  viaNative('document.withPlatformStops', { record, platformId, ...(name ? { name } : {}) })
