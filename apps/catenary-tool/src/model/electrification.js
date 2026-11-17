import { lifespanWithStatus, trackLength, trackStatus, VOLTAGES, wireStretches, withLifespan } from '@rail-across/core/track'

/**
 * The layer `electrification` as the tool edits it (electrification 3.0,
 * decision 0006): pure functions from one state of the domain to the
 * next. Masts stand beside a track; the suspension points on them hold a
 * contact wire over a track; section insulators part two wires, two circuits,
 * on a track. A wire carries a voltage — named as OSRD names a traction mode,
 * so a train runs electrically where its vehicle has the mode — and runs, on a
 * track, from its first to its last carrying point there (`wireStretches`).
 */

export { VOLTAGES }

/** The colour each voltage is drawn in on the plan. */
export const VOLTAGE_COLORS = {
  '15000V': '#d6336c', '25000V': '#7048e8', '3000V': '#1c7ed6', '1500V': '#0ca678', '750V': '#f59f00', '600V': '#e8590c',
}

/** The kinds of masts and of section insulators. */
export const MAST_KINDS = ['cantilever', 'portal', 'headspan', 'other']
export const INSULATOR_KINDS = ['insulator', 'neutralSection']

/** The span a stretch is laid out with: the most there is between two masts [m]. */
export const SPAN = 60
/** Where a new mast stands and what its point holds, as is usual on a DB line: beside the track, the wire 5.5 m up. */
export const MAST_OFFSET = 3.2
export const WIRE_HEIGHT = 5.5

const COLLECTIONS = ['contactWires', 'masts', 'suspensionPoints', 'sectionInsulators']

/** How far apart two stations may be and still be one [m]. */
const TOL = 0.01

/** The domain of a record — an empty one where it has none. */
export function domainOf(record) {
  const d = record?.electrification
  return { fields: d?.fields ?? {}, collections: Object.fromEntries(COLLECTIONS.map(c => [c, d?.collections?.[c] ?? []])) }
}

/** The record with the domain laid on it; one that holds nothing is no layer of it. */
export function recordOf(record, domain) {
  const empty = COLLECTIONS.every(c => !domain.collections[c].length) && !Object.keys(domain.fields).length
  return { ...record, electrification: empty ? undefined : domain }
}

// A UUID, as every id of the record (decision 0006) — drawn by hand where the browser draws none.
export const newId = () => (globalThis.crypto?.randomUUID?.() ?? '10000000-1000-4000-8000-100000000000'.replace(/[018]/g, c => (c ^ (Math.random() * 16) >> (c / 4)).toString(16)))

const station = (trackId, at, extra = {}) => ({ anchor: 'station', trackId, station: at, ...extra })

/** A new contact wire of a voltage. */
export const newWire = ({ voltage, name }) => ({ id: newId(), ...(name ? { name } : {}), voltage })

/**
 * The masts that carry a wire along a stretch of a track, from `from` to `to`
 * in either order, as many as keep every span within `span`, equally apart —
 * each with a suspension point that holds the wire over the track:
 * { masts, suspensionPoints }.
 */
export function wireAlong({ trackId, from, to, contactWireId, span = SPAN, side = 'right', offset = MAST_OFFSET, height = WIRE_HEIGHT }) {
  const [a, b] = [Math.min(from, to), Math.max(from, to)]
  const n = Math.max(1, Math.ceil((b - a) / span - 1e-9))
  const masts = [], suspensionPoints = []
  for (let i = 0; i <= n; i++) {
    const at = Math.round((a + (b - a) * i / n) * 1000) / 1000
    const mast = { id: newId(), kind: 'cantilever', at: station(trackId, at, { side, offset }) }
    masts.push(mast)
    suspensionPoints.push({ id: newId(), mastId: mast.id, at: station(trackId, at), contactWireId, height })
  }
  return { masts, suspensionPoints }
}

const list = (domain, name) => domain.collections[name]
const withList = (domain, name, items) => ({ ...domain, collections: { ...domain.collections, [name]: items } })

/** The domain with `obj` in collection `name`: added, or in the place of the one of its id. */
export function put(domain, name, obj) {
  const items = list(domain, name)
  return withList(domain, name, items.some(o => o.id === obj.id) ? items.map(o => (o.id === obj.id ? obj : o)) : [...items, obj])
}

/** The domain with a stretch of a wire laid out (`wireAlong`'s masts and points), the wire added where it is new. */
export function withStretch(domain, wire, stretch) {
  let d = list(domain, 'contactWires').some(w => w.id === wire.id) ? domain : put(domain, 'contactWires', wire)
  const { masts, suspensionPoints } = wireAlong({ ...stretch, contactWireId: wire.id })
  d = withList(d, 'masts', [...list(d, 'masts'), ...masts.map(m => withLifespan(m, wire.lifespan))])
  return withList(d, 'suspensionPoints', [...list(d, 'suspensionPoints'), ...suspensionPoints.map(p => withLifespan(p, wire.lifespan))])
}

/** The masts left holding nothing once `points` went, of those that held them. */
const emptied = (domain, points) => {
  const held = new Set(points.map(p => p.mastId))
  const still = new Set(list(domain, 'suspensionPoints').map(p => p.mastId))
  return new Set([...held].filter(id => !still.has(id)))
}

/** The domain without a wire: its suspension points with it, the masts they leave holding nothing, the insulators that part it. */
export function withoutWire(domain, id) {
  const points = list(domain, 'suspensionPoints').filter(p => p.contactWireId === id)
  let d = withList(domain, 'contactWires', list(domain, 'contactWires').filter(w => w.id !== id))
  d = withList(d, 'suspensionPoints', list(d, 'suspensionPoints').filter(p => p.contactWireId !== id))
  d = withList(d, 'sectionInsulators', list(d, 'sectionInsulators').filter(i => i.contactWireBeforeId !== id && i.contactWireAfterId !== id))
  const gone = emptied(d, points)
  return withList(d, 'masts', list(d, 'masts').filter(m => !gone.has(m.id)))
}

/** The domain without a mast and the suspension points on it. */
export function withoutMast(domain, id) {
  const d = withList(domain, 'masts', list(domain, 'masts').filter(m => m.id !== id))
  return withList(d, 'suspensionPoints', list(d, 'suspensionPoints').filter(p => p.mastId !== id))
}

/** The domain without one object of a collection, nothing else. */
export const without = (domain, name, id) => withList(domain, name, list(domain, name).filter(o => o.id !== id))

/** The planning status of an object, from its lifespan (decision 0007). */
export const statusOf = (obj) => trackStatus(obj)

/** The object with that planning status: new or removal with the day open where it states none, the days it states kept. */
export const withStatus = (obj, status) => withLifespan(obj, lifespanWithStatus(obj.lifespan, status))

/** Where the wires run: [{ contactWireId, voltage, trackId, startStation, endStation }]. */
export const stretchesOf = (domain) => wireStretches(domain)

/** A stretch's length [m]. */
export const lengthOf = (s) => Math.max(0, s.endStation - s.startStation)

/**
 * What holds the layer together on the net: { errors, warnings }, each finding
 * { code, collection, id, params } — something on a track that is not there or
 * beyond its ends, an insulator that parts a wire from itself, a wire nothing
 * carries, and two wires of two voltages over the same piece of track.
 */
export function checkElectrification(domain, tracks) {
  const errors = [], warnings = []
  const byId = new Map((tracks ?? []).map(t => [t.id, t]))
  for (const collection of ['masts', 'suspensionPoints', 'sectionInsulators']) {
    for (const o of list(domain, collection)) {
      if (o.at?.anchor !== 'station') continue
      const track = byId.get(o.at.trackId)
      if (!track) { errors.push({ code: 'track_missing', collection, id: o.id, params: {} }); continue }
      const length = trackLength(track)
      if (o.at.station < -TOL || o.at.station > length + TOL) errors.push({ code: 'off_track', collection, id: o.id, params: { length: Math.round(length) } })
    }
  }
  for (const i of list(domain, 'sectionInsulators')) {
    if (i.contactWireBeforeId === i.contactWireAfterId) errors.push({ code: 'insulator_same_wire', collection: 'sectionInsulators', id: i.id, params: {} })
  }
  const stretches = stretchesOf(domain)
  const carried = new Set(stretches.map(s => s.contactWireId))
  for (const w of list(domain, 'contactWires')) {
    if (!carried.has(w.id)) warnings.push({ code: 'wire_unused', collection: 'contactWires', id: w.id, params: {} })
  }
  for (let i = 0; i < stretches.length; i++) {
    for (let j = i + 1; j < stretches.length; j++) {
      const a = stretches[i], b = stretches[j]
      if (a.trackId !== b.trackId || a.voltage === b.voltage) continue
      const overlap = Math.min(a.endStation, b.endStation) - Math.max(a.startStation, b.startStation)
      if (overlap > TOL) errors.push({ code: 'voltages_meet', collection: 'contactWires', id: b.contactWireId, params: { other: a.contactWireId, a: a.voltage, b: b.voltage } })
    }
  }
  return { errors, warnings }
}

/**
 * How much of the net is under wire: { byVoltage: { voltage: metres }, tracks:
 * metres of every track, electrified: metres under some wire }, every overlap
 * counted once.
 */
export function coverage(domain, tracks) {
  const byVoltage = {}
  let total = 0, electrified = 0
  const stretches = stretchesOf(domain)
  for (const t of tracks ?? []) {
    const length = trackLength(t)
    total += length
    const mine = stretches.filter(s => s.trackId === t.id)
    for (const s of mine) byVoltage[s.voltage] = (byVoltage[s.voltage] ?? 0) + lengthOf(s)
    const spans = mine.map(s => [Math.max(0, s.startStation), Math.min(length, s.endStation)]).filter(([a, b]) => b > a).sort((x, y) => x[0] - y[0])
    let end = -Infinity
    for (const [a, b] of spans) {
      if (a > end) { electrified += b - a; end = b } else if (b > end) { electrified += b - end; end = b }
    }
  }
  return { byVoltage, tracks: total, electrified }
}
