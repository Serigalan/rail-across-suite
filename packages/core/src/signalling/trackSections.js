import { tracksOnFrom } from '../track/index.js'

/**
 * The extent of the track detection sections, derived over the net (decision
 * 0011): a section is stored with its name, its kind and the detection points
 * it is bounded by; which part of the net it covers follows from where those
 * points stand. So a section keeps its identity when a point moves, and what
 * it covers cannot disagree with where its points are.
 *
 * A section's own points cut the net into parts — runs of track between them
 * and the track ends, joined across switches, links and joints — and it
 * covers the part bounded by all of them. Only its own: sections overlap
 * where two meet, each ending at a counting head of its own a metre or so
 * beyond the other's (P-Hausen: W1 ends at W1B, W2 begins at W2C before it).
 * Where several parts are bounded by all its points (a section reaching to a
 * track end), the one holding fewest points of other sections is it. Where
 * none is, the stored boundary and the net disagree: the section names the
 * points of its own that bound no part with the others.
 */

const lengthOf = (track) => (track.elements ?? []).reduce((sum, el) => sum + (el.length ?? 0), 0)

/** Union-find over keys. */
function unionFind() {
  const parent = new Map()
  const find = (k) => {
    if (!parent.has(k)) parent.set(k, k)
    let r = k
    while (parent.get(r) !== r) r = parent.get(r)
    while (parent.get(k) !== r) { const n = parent.get(k); parent.set(k, r); k = n }
    return r
  }
  return { find, union: (a, b) => parent.set(find(a), find(b)) }
}

/**
 * The parts the detection points cut the net into:
 * [{ parts: [{ trackId, from, to }], boundary: [detectionPointId] }].
 * `core` and `signalling` are the two domains ({ fields, collections }); a
 * point in the plane (an orphan) cuts nothing. `only` limits the cuts to
 * those points.
 */
export function netParts({ core, signalling }, only = null) {
  const tracks = core?.collections?.tracks ?? []
  const switches = core?.collections?.switches ?? []
  const cuts = new Map()   // trackId → [{ station, id }] in order
  for (const d of signalling?.collections?.detectionPoints ?? []) {
    if (d.at?.anchor !== 'station' || (only && !only.has(d.id))) continue
    if (!cuts.has(d.at.trackId)) cuts.set(d.at.trackId, [])
    cuts.get(d.at.trackId).push({ station: d.at.station, id: d.id })
  }
  for (const list of cuts.values()) list.sort((a, b) => a.station - b.station)

  // Each track falls into pieces between its cuts: piece i runs from cut i-1 to cut i.
  const pieces = new Map()   // `${trackId}:${i}` → { trackId, from, to, bounds: [ids] }
  const count = new Map()    // trackId → its number of pieces
  const { find, union } = unionFind()
  for (const t of tracks) {
    const length = lengthOf(t)
    const list = (cuts.get(t.id) ?? []).filter(c => c.station >= 0 && c.station <= length)
    const edges = [{ station: 0, id: null }, ...list, { station: length, id: null }]
    count.set(t.id, edges.length - 1)
    for (let i = 0; i + 1 < edges.length; i++) {
      const key = `${t.id}:${i}`
      pieces.set(key, {
        trackId: t.id, from: edges[i].station, to: edges[i + 1].station,
        bounds: [edges[i].id, edges[i + 1].id].filter(Boolean),
      })
      find(key)
    }
  }
  const pieceAt = (trackId, endpoint) => {
    const n = count.get(trackId)
    return n ? `${trackId}:${endpoint === 'BEGIN' ? 0 : n - 1}` : null
  }
  // Pieces meeting at a track end are one part, unless a cut stands right at that end.
  for (const t of tracks) {
    for (const endpoint of ['BEGIN', 'END']) {
      const here = pieceAt(t.id, endpoint)
      if (!here) continue
      for (const next of tracksOnFrom(t.id, endpoint, tracks, switches)) {
        const there = pieceAt(next.trackId, next.endpoint)
        if (there) union(here, there)
      }
    }
  }
  const parts = new Map()
  for (const [key, piece] of pieces) {
    const root = find(key)
    if (!parts.has(root)) parts.set(root, { parts: [], boundary: new Set() })
    const part = parts.get(root)
    if (piece.to - piece.from > 1e-6) part.parts.push({ trackId: piece.trackId, from: piece.from, to: piece.to })
    for (const b of piece.bounds) part.boundary.add(b)
  }
  return [...parts.values()]
    .filter(p => p.parts.length)
    .map(p => ({ parts: p.parts, boundary: [...p.boundary].sort() }))
}

/**
 * The extent of every stored track section: Map sectionId →
 * { parts: [{ trackId, from, to }], matches, missing } — `matches` where the
 * net has a part bounded by all of its points, else the part bounded by most
 * of them, with those of its points that bound it not (`missing`). A section
 * whose points bound nothing has no parts.
 */
export function trackSectionExtents({ core, signalling }) {
  const sections = signalling?.collections?.trackSections ?? []
  const pointsOf = new Map()   // detection point → its sections
  for (const s of sections) for (const p of s.boundedBy ?? []) pointsOf.set(p, [...(pointsOf.get(p) ?? []), s.id])
  const stations = new Map()   // trackId → [{ station, id }]
  for (const d of signalling?.collections?.detectionPoints ?? []) {
    if (d.at?.anchor !== 'station') continue
    stations.set(d.at.trackId, [...(stations.get(d.at.trackId) ?? []), { station: d.at.station, id: d.id }])
  }
  /** The points of other sections strictly inside the part. */
  const foreign = (part, own) => {
    let n = 0
    for (const p of part.parts) {
      for (const d of stations.get(p.trackId) ?? []) {
        if (!own.has(d.id) && pointsOf.has(d.id) && d.station > p.from + 1e-6 && d.station < p.to - 1e-6) n++
      }
    }
    return n
  }
  const length = (part) => part.parts.reduce((sum, p) => sum + (p.to - p.from), 0)
  const out = new Map()
  for (const s of sections) {
    const own = new Set(s.boundedBy ?? [])
    const parts = netParts({ core, signalling }, own)
    const whole = parts.filter(p => p.boundary.length === own.size && p.boundary.every(b => own.has(b)))
    if (whole.length) {
      whole.sort((a, b) => foreign(a, own) - foreign(b, own) || length(a) - length(b))
      out.set(s.id, { parts: whole[0].parts, matches: true, missing: [] })
      continue
    }
    const best = parts.filter(p => p.boundary.length).sort((a, b) => b.boundary.length - a.boundary.length)[0]
    out.set(s.id, best
      ? { parts: best.parts, matches: false, missing: [...own].filter(b => !best.boundary.includes(b)).sort() }
      : { parts: [], matches: false, missing: [...own].sort() })
  }
  return out
}
