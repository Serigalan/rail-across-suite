import { netParts } from '@rail-across/core/signalling'
import { pointAtStation, portsOf, portEnd, portTrack, trackLength, tracksOnFrom } from '@rail-across/core/track'
import { newId } from './domain.js'
import { kmNameAt, trackName } from './places.js'

/**
 * Laying out train detection over the net: detection points at every main
 * signal, `before` metres ahead of every switch on its toe's side and
 * `behind` metres past its clearance point on each of its branches — the
 * clearance point (Grenzzeichen) where the two tracks lie `clearance` apart —;
 * points nearer to one another than `merge` metres made one, in the middle
 * between them (two switches close together share the point between them),
 * and a point already there taking those near it. The points cut the net into
 * the track sections, each bounded by the points around it: a section the
 * net already has with those bounds stays, the others are made anew.
 */

/** The distances it lays out by, unless told others [m]. */
export const LAYOUT = { before: 10, behind: 10, clearance: 3.5, merge: 20 }

/** The kinds of logical signal that bound no detection: distant signals and their repeaters. */
const NO_POINT = new Set(['distant', 'distantRepeater'])

/** Whether a signal gets a detection point: a real one on a track with a screen that is no distant signal's. */
export const takesPoint = (s) => s.kind === 'real' && s.at?.anchor === 'station' && (s.logicalSignals ?? []).some(l => !NO_POINT.has(l.type))

const STEP = 0.5
const REACH = 250
const otherEnd = (end) => (end === 'BEGIN' ? 'END' : 'BEGIN')

/**
 * Walking `distance` metres into the net from a track's end, away from it:
 * every place reached [{ trackId, station }] — more than one where a switch
 * on the way leads into several tracks; none where the net ends first.
 */
export function walkFrom(net, lengths, trackId, endpoint, distance, depth = 0) {
  const len = lengths.get(trackId) ?? 0
  if (distance <= len) return [{ trackId, station: endpoint === 'BEGIN' ? distance : len - distance }]
  if (depth > 20) return []
  return tracksOnFrom(trackId, otherEnd(endpoint), net.tracks, net.switches)
    .flatMap(n => walkFrom(net, lengths, n.trackId, n.endpoint, distance - len, depth + 1))
}

/** The point in the plane `s` metres into the net from a track's end, along the first way it leads; null past its end. */
function armPoint(net, lengths, byId, arm, s) {
  const [at] = walkFrom(net, lengths, arm.trackId, arm.endpoint, s)
  const p = at && pointAtStation(byId.get(at.trackId), at.station)
  return p ? [p.utm.easting, p.utm.northing] : null
}

/**
 * How far from a switch two of its arms run before they lie `apart` metres
 * from one another — the clearance point; null where they never do within
 * reach, or diverge at once (they leave the switch to opposite sides).
 */
function clearanceOf(net, lengths, byId, a, b, apart) {
  let before = 0
  for (let s = STEP; s <= REACH; s += STEP) {
    const p = armPoint(net, lengths, byId, a, s), q = armPoint(net, lengths, byId, b, s)
    if (!p || !q) return null
    const d = Math.hypot(p[0] - q[0], p[1] - q[1])
    // Within the step, straight between the two distances measured.
    if (d >= apart) return s - STEP + (STEP * (apart - before)) / Math.max(d - before, 1e-9)
    before = d
  }
  return null
}

/**
 * The places a detection point is asked for: [{ trackId, station, why }] —
 * at the signals, ahead of the switches, past their clearance points.
 */
export function wantedPoints(net, signals, layout = LAYOUT) {
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const byId = new Map(net.tracks.map(t => [t.id, t]))
  const out = []
  for (const s of signals.filter(takesPoint)) {
    if (lengths.has(s.at.trackId)) out.push({ trackId: s.at.trackId, station: s.at.station, why: 'signal' })
  }
  for (const sw of net.switches) {
    const arms = portsOf(sw).map(({ port }) => ({ port, trackId: portTrack(sw, port), endpoint: portEnd(sw, port) }))
      .filter(a => lengths.has(a.trackId) && a.endpoint)
    if (arms.length < 3) continue   // a link, or a switch not laid
    for (const arm of arms) {
      // Its neighbour: the arm that leaves the switch beside it, to the same side.
      const near = arms.filter(o => o !== arm).map(o => {
        const p = armPoint(net, lengths, byId, arm, 5), q = armPoint(net, lengths, byId, o, 5)
        return { o, d: p && q ? Math.hypot(p[0] - q[0], p[1] - q[1]) : Infinity }
      }).filter(x => x.d < layout.clearance).sort((x, y) => x.d - y.d)[0]
      const clear = near ? clearanceOf(net, lengths, byId, arm, near.o, layout.clearance) : 0
      if (clear == null) continue
      const distance = near ? clear + layout.behind : layout.before
      for (const at of walkFrom(net, lengths, arm.trackId, arm.endpoint, distance)) out.push({ ...at, why: near ? 'clearance' : 'switch' })
    }
  }
  return out
}

/** Union-find over indices. */
function groups(n, joined) {
  const parent = Array.from({ length: n }, (_, i) => i)
  const find = (i) => { while (parent[i] !== i) { parent[i] = parent[parent[i]]; i = parent[i] } return i }
  for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) if (joined(i, j)) parent[find(i)] = find(j)
  const out = new Map()
  for (let i = 0; i < n; i++) out.set(find(i), [...(out.get(find(i)) ?? []), i])
  return [...out.values()]
}

/**
 * How far apart two places are along the net: on one track the difference
 * of their stations, across a joint where two tracks are laid end to end the
 * way over it — never over a switch, whose sides a point must keep apart.
 */
function apartAlong(net, lengths, a, b) {
  if (a.trackId === b.trackId) return Math.abs(a.station - b.station)
  let best = Infinity
  for (const end of ['BEGIN', 'END']) {
    const held = net.switches.some(sw => portsOf(sw).some(({ port }) => portTrack(sw, port) === a.trackId && portEnd(sw, port) === end))
    if (held) continue
    for (const n of tracksOnFrom(a.trackId, end, net.tracks, net.switches)) {
      if (n.trackId !== b.trackId) continue
      const toEnd = end === 'BEGIN' ? a.station : (lengths.get(a.trackId) ?? 0) - a.station
      const fromEnd = n.endpoint === 'BEGIN' ? b.station : (lengths.get(b.trackId) ?? 0) - b.station
      best = Math.min(best, toEnd + fromEnd)
    }
  }
  return best
}

/**
 * The detection points the net asks for, made one where they come nearer
 * than `merge` metres, and the sections they cut it into: { points, merged,
 * sections, kept, removed } — `points` the new points, `merged` how many asked
 * for went into another, `sections` the new sections, `kept` the sections
 * there are that stay, `removed` the ids of those whose bounds no longer hold.
 */
export function detectionLayout(net, domain, { kmLines, layout = LAYOUT, t } = {}) {
  const lengths = new Map(net.tracks.map(t => [t.id, trackLength(t)]))
  const { detectionPoints, trackSections, signals } = domain.collections
  const there = detectionPoints.filter(d => d.at?.anchor === 'station').map(d => ({ trackId: d.at.trackId, station: d.at.station, id: d.id }))
  const wanted = wantedPoints(net, signals, layout)
  const all = [...there, ...wanted]
  const clusters = groups(all.length, (i, j) => apartAlong(net, lengths, all[i], all[j]) < layout.merge)
  const taken = new Set(detectionPoints.map(d => d.name).filter(Boolean))
  const points = []
  let merged = 0
  for (const members of clusters) {
    const asked = members.filter(i => !all[i].id)
    if (!asked.length) continue
    if (members.some(i => all[i].id)) { merged += asked.length; continue }
    merged += asked.length - 1
    const places = asked.map(i => all[i])
    // The middle: on one track between the outermost, else the place nearest to all the others.
    const at = places.every(p => p.trackId === places[0].trackId)
      ? { trackId: places[0].trackId, station: (Math.min(...places.map(p => p.station)) + Math.max(...places.map(p => p.station))) / 2 }
      : places.map(p => ({ p, worst: Math.max(...places.map(q => apartAlong(net, lengths, p, q))) })).sort((x, y) => x.worst - y.worst)[0].p
    const loc = { anchor: 'station', trackId: at.trackId, station: Math.round(at.station * 1000) / 1000 }
    const name = kmNameAt(net, kmLines, loc, taken)
    if (name) taken.add(name)
    points.push({ id: newId(), ...(name ? { name } : {}), kind: 'axleCounter', at: loc })
  }

  const parts = netParts({ core: { collections: net }, signalling: { collections: { detectionPoints: [...detectionPoints, ...points] } } })
    .filter(p => p.boundary.length)
  const key = (ids) => [...ids].sort().join('|')
  const byBounds = new Map(trackSections.map(s => [key(s.boundedBy), s]))
  const kept = [], sections = []
  for (const part of parts) {
    const same = byBounds.get(key(part.boundary))
    if (same) kept.push(same)
    else sections.push({ id: newId(), name: sectionName(net, part, t), kind: 'axleCounting', boundedBy: part.boundary })
  }
  numberAlike(sections, new Set(kept.map(s => s.name)))
  const keptIds = new Set(kept.map(s => s.id))
  return { points, merged, sections, kept, removed: trackSections.filter(s => !keptIds.has(s.id)).map(s => s.id) }
}

/** A section's name: the switches it holds (W2/W3), else the track it lies on most of. */
function sectionName(net, part, t) {
  const holds = (trackId, endpoint) => part.parts.some(p => p.trackId === trackId
    && (endpoint === 'BEGIN' ? p.from <= 1e-6 : p.to >= trackLength(net.tracks.find(tr => tr.id === trackId)) - 1e-6))
  const switches = net.switches.filter(sw => portsOf(sw).length >= 3 && portsOf(sw).every(({ port }) => holds(portTrack(sw, port), portEnd(sw, port))))
  if (switches.length) {
    return switches.map(sw => (Number.isFinite(sw.number) ? `W${sw.number}` : sw.name ?? sw.label ?? 'W')).sort((a, b) => a.localeCompare(b, 'de', { numeric: true })).join('/')
  }
  const longest = [...part.parts].sort((a, b) => (b.to - b.from) - (a.to - a.from))[0]
  return longest ? trackName(net, longest.trackId) : t?.('unnamed')
}

/** Names that repeat numbered: koennern.1-1, koennern.1-2; one a kept section has already numbered on. */
function numberAlike(sections, taken) {
  const count = new Map()
  for (const s of sections) count.set(s.name, (count.get(s.name) ?? 0) + 1)
  const next = new Map()
  for (const s of sections) {
    if (count.get(s.name) < 2 && !taken.has(s.name)) continue
    let n = next.get(s.name) ?? 1
    while (taken.has(`${s.name}-${n}`)) n++
    next.set(s.name, n + 1)
    taken.add(`${s.name}-${n}`)
    s.name = `${s.name}-${n}`
  }
}
