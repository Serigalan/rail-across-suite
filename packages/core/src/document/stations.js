import { operationalRegister } from './coreFormat.js'

/**
 * The stations of a record by their ids. A station or halt is a point of the
 * operational point register (R10), and its id is what names it; its name is
 * only what it is shown by. The register is the one the record's document
 * keeps (coreFormat): the points the tracks' and platforms' station texts
 * make — a new one's id drawn from the project and its name, the same on
 * every variant —, and those the record keeps, among them every station
 * renamed: it keeps its id under its new name.
 */

const text = (v) => (v == null || String(v).trim() === '' ? undefined : String(v).trim())
const withoutEmpty = (obj) => Object.fromEntries(Object.entries(obj).filter(([, v]) => v !== undefined))

/**
 * The register of a record: { points, pointOf(obj) } — every point with its
 * id, name and codes, and the id of the point a track or platform of the
 * record is of (undefined where it names none).
 */
export function stationRegister(record) {
  return operationalRegister(record?.id, record?.tracks ?? [], record?.platforms ?? [], record?.operationalPoints ?? [])
}

/**
 * The record with the station `id` renamed — `name`, and its `uic` number
 * where given —, keeping its id: the register keeps the point under its new
 * name, and its tracks and platforms state it. Returns { tracks, platforms,
 * operationalPoints }, the parts it changed, or null where the record has no
 * station of that id.
 */
export function renamedStation(record, id, { name, uic } = {}) {
  const register = stationRegister(record)
  const point = register.points.find(p => p.id === id)
  if (!point) return null
  const next = withoutEmpty({ id, name: text(name), rl100: point.rl100, uic: uic === undefined ? point.uic : text(uic) })
  const kept = record?.operationalPoints ?? []
  const of = (obj) => register.pointOf(obj) === id
  return {
    tracks: (record?.tracks ?? []).map(tr => (of(tr) ? withoutEmpty({ ...tr, stationName: next.name, uicStation: next.uic }) : tr)),
    platforms: (record?.platforms ?? []).map(pf => (of(pf) ? withoutEmpty({ ...pf, stationName: next.name }) : pf)),
    operationalPoints: kept.some(p => p.id === id) ? kept.map(p => (p.id === id ? next : p)) : [...kept, next],
  }
}
