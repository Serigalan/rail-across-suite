import { lifespanWithStatus, trackLength, trackStatus, withLifespan } from '@rail-across/core/track'

/**
 * The layer `electrification` as the tool edits it (decision 0025): pure
 * functions from one state of the domain to the next. A range of a track is
 * under contact wire of one system — the voltage as OSRD names a traction
 * mode, so a train runs electrically where its vehicle has the mode.
 */

/** The systems, as `electrification` 1.0 names them: 15 kV 16.7 Hz first, DB's. */
export const SYSTEMS = ['15000V', '25000V', '3000V', '1500V', '750V', '600V']

/** The colour each system is drawn in on the plan. */
export const SYSTEM_COLORS = {
  '15000V': '#d6336c', '25000V': '#7048e8', '3000V': '#1c7ed6', '1500V': '#0ca678', '750V': '#f59f00', '600V': '#e8590c',
}

/** How far apart two stations may be and still be one [m]. */
const TOL = 0.01

/** The domain of a record — an empty one where it has none. */
export function domainOf(record) {
  const d = record?.electrification
  return { fields: d?.fields ?? {}, collections: { electrifications: d?.collections?.electrifications ?? [] } }
}

/** The record with the domain laid on it; one that holds nothing is no layer of it. */
export function recordOf(record, domain) {
  const empty = !domain.collections.electrifications.length && !Object.keys(domain.fields).length
  return { ...record, electrification: empty ? undefined : domain }
}

export const newId = () => (globalThis.crypto?.randomUUID?.() ?? `id-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`)

/** A new range: `from` and `to` in either order, on `trackId`. */
export function newElectrification({ trackId, from, to, system, name }) {
  const range = { anchor: 'station', trackId, startStation: Math.min(from, to), endStation: Math.max(from, to) }
  return { id: newId(), ...(name ? { name } : {}), at: range, system }
}

const withList = (domain, list) => ({ ...domain, collections: { ...domain.collections, electrifications: list } })

export const added = (domain, obj) => withList(domain, [...domain.collections.electrifications, obj])
export const replaced = (domain, obj) => withList(domain, domain.collections.electrifications.map(e => (e.id === obj.id ? obj : e)))
export const removed = (domain, id) => withList(domain, domain.collections.electrifications.filter(e => e.id !== id))

/** The planning status of a range, from its lifespan (decision 0028). */
export const statusOf = (obj) => trackStatus(obj)

/** The range with that planning status: new or removal with the day open where it states none, the days it states kept. */
export const withStatus = (obj, status) => withLifespan(obj, lifespanWithStatus(obj.lifespan, status))

/** A range's length [m]. */
export const lengthOf = (obj) => Math.max(0, obj.at.endStation - obj.at.startStation)

/**
 * What holds the layer together on the net: { errors, warnings }, each finding
 * { code, id, params } — a range on a track that is not there, one without
 * length or beyond its track's ends, two that overlap: of one system they are
 * one, of two the systems meet without a section that parts them.
 */
export function checkElectrification(domain, tracks) {
  const errors = [], warnings = []
  const byId = new Map((tracks ?? []).map(t => [t.id, t]))
  const list = domain.collections.electrifications
  for (const e of list) {
    const track = byId.get(e.at.trackId)
    if (!track) { errors.push({ code: 'track_missing', id: e.id, params: {} }); continue }
    if (!(e.at.endStation - e.at.startStation > TOL)) errors.push({ code: 'empty', id: e.id, params: {} })
    const length = trackLength(track)
    if (e.at.startStation < -TOL || e.at.endStation > length + TOL) errors.push({ code: 'off_track', id: e.id, params: { length: Math.round(length) } })
  }
  for (let i = 0; i < list.length; i++) {
    for (let j = i + 1; j < list.length; j++) {
      const a = list[i], b = list[j]
      if (a.at.trackId !== b.at.trackId) continue
      const overlap = Math.min(a.at.endStation, b.at.endStation) - Math.max(a.at.startStation, b.at.startStation)
      if (overlap <= TOL) continue
      if (a.system === b.system) warnings.push({ code: 'overlap', id: b.id, params: { other: a.id } })
      else errors.push({ code: 'systems_meet', id: b.id, params: { other: a.id, a: a.system, b: b.system } })
    }
  }
  return { errors, warnings }
}

/**
 * How much of the net is electrified: { bySystem: { system: metres }, tracks:
 * metres of every track, electrified: metres under some wire }, every
 * overlap counted once.
 */
export function coverage(domain, tracks) {
  const bySystem = {}
  let total = 0, electrified = 0
  for (const t of tracks ?? []) {
    const length = trackLength(t)
    total += length
    const mine = domain.collections.electrifications.filter(e => e.at.trackId === t.id)
    for (const e of mine) bySystem[e.system] = (bySystem[e.system] ?? 0) + lengthOf(e)
    // Under some wire: the ranges' union, on the track.
    const spans = mine.map(e => [Math.max(0, e.at.startStation), Math.min(length, e.at.endStation)]).filter(([a, b]) => b > a).sort((x, y) => x[0] - y[0])
    let end = -Infinity
    for (const [a, b] of spans) {
      if (a > end) { electrified += b - a; end = b } else if (b > end) { electrified += b - end; end = b }
    }
  }
  return { bySystem, tracks: total, electrified }
}
