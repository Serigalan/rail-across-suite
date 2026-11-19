import { curveResistance, merged, pathLength } from './path.js'
import { requestFrom } from './request.js'
import { joined, routeVia } from './route.js'

/**
 * A net of OSRD's — a RailJSON infrastructure, as the server keeps them for
 * Fahrzeit rechnen — run over instead of a variant's: its track sections, a
 * location on them { trackId, station, direction } with `station` the
 * offset along the section as RailJSON counts it (its `length`), a path
 * [{ trackId, start, end }] as over a variant's (path.js); the ways from one
 * section to the next its switches' routes and its links; the grades its
 * slopes and curves, the speeds its speed sections, the wire its
 * electrifications. Signals do not signal a speed here: OSRD's own run takes
 * the speed sections alone.
 */

/** The routes through a switch of each of OSRD's builtin types: the ports they join. */
const SWITCH_ROUTES = {
  link: [['A', 'B']],
  point_switch: [['A', 'B1'], ['A', 'B2']],
  crossing: [['A1', 'B1'], ['A2', 'B2']],
  single_slip_switch: [['A1', 'B1'], ['A2', 'B2'], ['A1', 'B2']],
  double_slip_switch: [['A1', 'B1'], ['A1', 'B2'], ['A2', 'B1'], ['A2', 'B2']],
}

const EARTH = 6371008.8
const rad = (d) => d * Math.PI / 180

/** The distance between two points [lng, lat] on the sphere [m]. */
function haversine([lng1, lat1], [lng2, lat2]) {
  const a = Math.sin(rad(lat2 - lat1) / 2) ** 2 + Math.cos(rad(lat1)) * Math.cos(rad(lat2)) * Math.sin(rad(lng2 - lng1) / 2) ** 2
  return 2 * EARTH * Math.asin(Math.min(1, Math.sqrt(a)))
}

const byLabel = (a, b) => String(a.label).localeCompare(String(b.label), 'de', { numeric: true })

/** Whether a range of RailJSON applies to a train running `direction` along its section. */
const applies = (range, direction) => !range.applicable_directions || range.applicable_directions === 'BOTH'
  || range.applicable_directions === (direction === 'up' ? 'START_TO_STOP' : 'STOP_TO_START')

/**
 * A RailJSON infrastructure ready to run over: { tracks, lines, next,
 * speeds, wires, planned, places, bounds } — `lines` as netLines draws a
 * variant's (a section's points with their offsets, its drawn length taken
 * to be its `length`), `next` the sections one leads on to from an end
 * (`${trackId}:${endpoint}` → [{ trackId, endpoint }]), `places` its
 * operational points, each with the locations of its parts.
 */
export function osrdNet(infra) {
  const tracks = new Map(), lines = new Map()
  let w = Infinity, so = Infinity, e = -Infinity, n = -Infinity
  for (const t of infra?.track_sections ?? []) {
    const coords = t.geo?.coordinates ?? []
    if (!t.id || coords.length < 2 || !(t.length > 0)) continue
    const along = [0]
    for (let i = 1; i < coords.length; i++) along.push(along[i - 1] + haversine(coords[i - 1], coords[i]))
    const scale = along.at(-1) > 0 ? t.length / along.at(-1) : 0
    lines.set(t.id, coords.map((c, i) => ({ s: along[i] * scale, lngLat: c })))
    for (const [lng, lat] of coords) { w = Math.min(w, lng); e = Math.max(e, lng); so = Math.min(so, lat); n = Math.max(n, lat) }
    tracks.set(t.id, { length: t.length, slopes: t.slopes ?? [], curves: t.curves ?? [] })
  }

  const next = new Map()
  const join = (a, b) => {
    if (!tracks.has(a?.track) || !tracks.has(b?.track)) return
    for (const [x, y] of [[a, b], [b, a]]) {
      const key = `${x.track}:${x.endpoint}`
      if (!next.has(key)) next.set(key, [])
      next.get(key).push({ trackId: y.track, endpoint: y.endpoint })
    }
  }
  // An extended type's routes: its groups' connections.
  const extended = new Map((infra?.extended_switch_types ?? []).map(x => [x.id, Object.values(x.groups ?? {}).flat().map(c => [c.src, c.dst])]))
  for (const sw of infra?.switches ?? []) {
    for (const [p, q] of SWITCH_ROUTES[sw.switch_type] ?? extended.get(sw.switch_type) ?? []) join(sw.ports?.[p], sw.ports?.[q])
  }
  // Before RailJSON 3, links were a collection of their own.
  for (const link of infra?.track_section_links ?? []) join(link.src, link.dst)

  const ranged = (items, value) => {
    const out = new Map()
    for (const item of items ?? []) {
      const v = value(item)
      if (v == null) continue
      for (const r of item.track_ranges ?? []) {
        if (!out.has(r.track)) out.set(r.track, [])
        out.get(r.track).push({ begin: Math.min(r.begin, r.end), end: Math.max(r.begin, r.end), applicable_directions: r.applicable_directions, value: v })
      }
    }
    return out
  }
  const speeds = ranged(infra?.speed_sections, s => (s.speed_limit > 0 ? s.speed_limit : null))
  const wires = ranged(infra?.electrifications, x => x.voltage || null)

  const places = (infra?.operational_points ?? [])
    .map(op => ({ op, locs: (op.parts ?? []).filter(p => tracks.has(p.track)).map(p => ({ trackId: p.track, station: p.position, direction: null })) }))
    .filter(({ locs }) => locs.length)
    .map(({ op, locs }) => ({ id: `osrd:${op.id}`, label: op.name || op.id, loc: locs[0], locs }))
    .sort(byLabel)

  return {
    tracks, lines, next, speeds, wires, places,
    planned: (infra?.electrifications ?? []).length > 0,
    bounds: Number.isFinite(w) ? [[w, so], [e, n]] : null,
  }
}

/** A heap of { cost } by the least cost. */
function heap() {
  const a = []
  return {
    get size() { return a.length },
    push(x) {
      a.push(x)
      for (let i = a.length - 1; i > 0;) {
        const p = (i - 1) >> 1
        if (a[p].cost <= a[i].cost) break
        ;[a[p], a[i]] = [a[i], a[p]]
        i = p
      }
    },
    pop() {
      const top = a[0], last = a.pop()
      if (a.length) {
        a[0] = last
        for (let i = 0; ;) {
          const l = 2 * i + 1, r = l + 1
          let m = i
          if (l < a.length && a[l].cost < a[m].cost) m = l
          if (r < a.length && a[r].cost < a[m].cost) m = r
          if (m === i) break
          ;[a[m], a[i]] = [a[i], a[m]]
          i = m
        }
      }
      return top
    },
  }
}

/**
 * How much of the straight line to a target a way at least runs: the
 * search's estimate of what is left (A*) stays below what is run, where a
 * section is shorter than its drawn line by a few per cent.
 */
const AHEAD_SHARE = 0.9

/**
 * The shortest way from any of the locations `froms` to any of `tos` — the
 * parts of a station either side — in one search: { path, from, to,
 * direction }, the location it left from and the one it reached, the way it
 * runs there; or null where none leads there. Where a location has no
 * direction, both are tried. `weights` ((trackId) → a factor of 1 or more)
 * makes a section dearer than its length: the search for another way.
 *
 * A* over (section, the way it is run), entered at its one end: the cost
 * what is run to there, the estimate the straight line from there to the
 * nearest target.
 */
function search(net, froms, tos, weights = null) {
  const lengthOf = (id) => net.tracks.get(id)?.length
  const factor = (id) => (weights ? weights(id) : 1)
  const starts = froms.filter(f => lengthOf(f?.trackId) != null)
  const targets = tos.filter(t => lengthOf(t?.trackId) != null)
  if (!starts.length || !targets.length) return null
  const byTrack = new Map()
  for (const t of targets) {
    if (!byTrack.has(t.trackId)) byTrack.set(t.trackId, [])
    byTrack.get(t.trackId).push(t)
  }
  const aims = targets.map(t => osrdLngLatAt(net, t)).filter(Boolean)
  const estimate = (trackId, dir) => {
    const pts = net.lines.get(trackId)
    const at = dir === 'up' ? pts[0].lngLat : pts[pts.length - 1].lngLat
    let near = Infinity
    for (const aim of aims) near = Math.min(near, haversine(at, aim))
    return near * AHEAD_SHARE
  }
  const fits = (to, dir) => !to.direction || to.direction === dir

  let best = null
  const reach = (cost, found) => { if (!best || cost < best.cost) best = { cost, ...found } }
  const queue = heap()
  const done = new Map()   // `${trackId}:${dir}` → the least cost it was entered at
  const enter = (trackId, endpoint, cost, prev) => {
    const dir = endpoint === 'BEGIN' ? 'up' : 'down'
    const key = `${trackId}:${dir}`
    if (done.has(key) && done.get(key) <= cost) return
    done.set(key, cost)
    queue.push({ trackId, dir, cost: cost + estimate(trackId, dir), run: cost, prev })
  }
  for (const from of starts) {
    for (const direction of from.direction ? [from.direction] : ['up', 'down']) {
      const length = lengthOf(from.trackId)
      for (const to of byTrack.get(from.trackId) ?? []) {
        const ahead = direction === 'up' ? to.station - from.station : from.station - to.station
        if (ahead >= 0 && fits(to, direction)) reach(ahead * factor(from.trackId), { from, to, direction, pieces: [{ trackId: from.trackId, start: from.station, end: to.station }] })
      }
      const leaveAt = direction === 'up' ? length : 0
      const first = { from, piece: { trackId: from.trackId, start: from.station, end: leaveAt } }
      for (const nx of net.next.get(`${from.trackId}:${direction === 'up' ? 'END' : 'BEGIN'}`) ?? []) enter(nx.trackId, nx.endpoint, Math.abs(leaveAt - from.station) * factor(from.trackId), first)
    }
  }
  const piecesTo = (cur) => {
    const out = []
    let p = cur
    for (; p.trackId != null; p = p.prev) out.unshift({ trackId: p.trackId, start: p.dir === 'up' ? 0 : lengthOf(p.trackId), end: p.dir === 'up' ? lengthOf(p.trackId) : 0 })
    return { from: p.from, pieces: [p.piece, ...out] }
  }
  while (queue.size) {
    const cur = queue.pop()
    if (best && cur.cost >= best.cost) break
    if (done.get(`${cur.trackId}:${cur.dir}`) < cur.run) continue
    const length = lengthOf(cur.trackId)
    const entry = cur.dir === 'up' ? 0 : length
    for (const to of byTrack.get(cur.trackId) ?? []) {
      if (!fits(to, cur.dir)) continue
      const cost = cur.run + Math.abs(to.station - entry) * factor(cur.trackId)
      if (best && cost >= best.cost) continue
      const { from, pieces } = piecesTo(cur.prev)
      reach(cost, { from, to, direction: cur.dir, pieces: [...pieces, { trackId: cur.trackId, start: entry, end: to.station }] })
    }
    const on = cur.run + length * factor(cur.trackId)
    for (const nx of net.next.get(`${cur.trackId}:${cur.dir === 'up' ? 'END' : 'BEGIN'}`) ?? []) enter(nx.trackId, nx.endpoint, on, cur)
  }
  if (!best) return null
  return { path: best.pieces.filter(p => Math.abs(p.end - p.start) > 1e-6), from: best.from, to: best.to, direction: best.direction }
}

/**
 * The shortest path over an OSRD net from one location to another, or null
 * where none leads there; as findPath over a variant's: where a location
 * has no direction, both are tried. `weights` as search's.
 */
export function osrdFindPath(net, from, to, { weights = null } = {}) {
  return search(net, [from], [to], weights)?.path ?? null
}

/**
 * A train's way over an OSRD net through the places it is given — its
 * start, the stops on the way, its end, each with its locations (`locs`:
 * the parts of an operational point; a point picked on the map its one) —
 * as routeVia finds it over a variant's: { path, offsets, chosen }, or {
 * failedAt }. Each leg is one search from where the last one arrived, the
 * way it arrived, to any part of the next place — the first from any part
 * of the start. Where a stop reached so leads nowhere further on, routeVia
 * tries its other parts, a search for each. `weights` as search's.
 */
export function osrdRoute(net, places, { weights = null } = {}) {
  const points = places.map(p => p.locs ?? [p.loc])
  if (!points[0]?.length) return { failedAt: 0 }
  let path = []
  const offsets = [0], chosen = []
  let froms = points[0]
  for (let i = 1; i < points.length; i++) {
    const found = search(net, froms, points[i], weights)
    if (!found) {
      if (i === 1) return { failedAt: 1 }
      return routeVia(null, points, { legPath: (_i, from, direction, to) => search(net, [{ ...from, direction }], [to], weights)?.path ?? false })
    }
    if (i === 1) chosen.push(found.from)
    path = joined(path, found.path)
    offsets.push(pathLength(path))
    chosen.push(found.to)
    froms = [{ ...found.to, direction: found.direction }]
  }
  return { path, offsets, chosen }
}

/**
 * The operational point each of several paths over an OSRD net passes and
 * none of the others does — the one with the most parts, the nearest to the
 * middle of its path among equals —: a name to tell them apart by, or null
 * where a path passes none of its own.
 */
export function osrdViaPlaces(net, paths) {
  const passed = paths.map(path => {
    const pieces = new Map()
    let at = 0
    for (const p of path ?? []) {
      if (!pieces.has(p.trackId)) pieces.set(p.trackId, [])
      pieces.get(p.trackId).push({ lo: Math.min(p.start, p.end), hi: Math.max(p.start, p.end), at, start: p.start })
      at += Math.abs(p.end - p.start)
    }
    const out = new Map()   // place id → its offset along the path
    for (const place of net.places) {
      for (const loc of place.locs) {
        const on = pieces.get(loc.trackId)?.find(q => loc.station >= q.lo - 1e-6 && loc.station <= q.hi + 1e-6)
        if (on) { out.set(place.id, on.at + Math.abs(loc.station - on.start)); break }
      }
    }
    return { out, length: at }
  })
  const byId = new Map(net.places.map(p => [p.id, p]))
  return passed.map(({ out, length }, i) => {
    let pick = null
    for (const [id, offset] of out) {
      if (passed.some((o, j) => j !== i && o.out.has(id))) continue
      const parts = byId.get(id).locs.length, off = Math.abs(offset - length / 2)
      if (!pick || parts > pick.parts || (parts === pick.parts && off < pick.off)) pick = { id, parts, off }
    }
    return pick ? byId.get(pick.id).label : null
  })
}

/** The point [lng, lat] at an offset along a section's drawn line. */
function lngLatOn(pts, s) {
  if (s <= pts[0].s) return pts[0].lngLat
  for (let i = 1; i < pts.length; i++) {
    if (s <= pts[i].s) {
      const a = pts[i - 1], b = pts[i]
      const u = b.s > a.s ? (s - a.s) / (b.s - a.s) : 0
      return [a.lngLat[0] + u * (b.lngLat[0] - a.lngLat[0]), a.lngLat[1] + u * (b.lngLat[1] - a.lngLat[1])]
    }
  }
  return pts.at(-1).lngLat
}

/** A location of an OSRD net on the map [lng, lat], or null. */
export function osrdLngLatAt(net, loc) {
  const pts = net.lines.get(loc?.trackId)
  return pts ? lngLatOn(pts, loc.station) : null
}

/** A path over an OSRD net as one line on the map. */
export function osrdPathGeoJSON(net, path) {
  const coordinates = (path ?? []).flatMap(p => {
    const pts = net.lines.get(p.trackId)
    if (!pts) return []
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const line = [lngLatOn(pts, lo), ...pts.filter(q => q.s > lo && q.s < hi).map(q => q.lngLat), lngLatOn(pts, hi)]
    return p.end < p.start ? line.reverse() : line
  })
  return { type: 'FeatureCollection', features: coordinates.length >= 2 ? [{ type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates } }] : [] }
}

/**
 * What the physics needs of a path over an OSRD net, as pathProperties
 * gives it of a variant's: { length, gradients, speedLimits, gaps } — the
 * grades its sections' slopes, rising positive the way the train runs, with
 * the resistance of their curves (Röckl); the speeds the lowest of its speed
 * sections that apply the way it runs; `gaps.heights` where a section states
 * no slope, `gaps.speed` where no speed section speaks.
 */
export function osrdProperties(net, path) {
  const gradients = [], speeds = [], noSlopes = [], noSpeed = []
  let at = 0
  for (const p of path) {
    const track = net.tracks.get(p.trackId)
    const len = Math.abs(p.end - p.start)
    const up = p.end >= p.start
    const direction = up ? 'up' : 'down'
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const along = (station) => at + Math.abs(station - p.start)
    const slopes = track?.slopes ?? [], curves = track?.curves ?? []
    const limits = (net.speeds.get(p.trackId) ?? []).filter(r => applies(r, direction))
    if (!slopes.length) noSlopes.push({ begin: at, end: at + len, value: 0 })
    const cuts = new Set([lo, hi])
    for (const r of [...slopes, ...curves, ...limits]) for (const s of [r.begin, r.end]) if (s > lo && s < hi) cuts.add(s)
    const stations = [...cuts].sort((a, b) => a - b)
    for (let i = 0; i + 1 < stations.length; i++) {
      const a = stations[i], b = stations[i + 1]
      if (b - a < 1e-6) continue
      const mid = (a + b) / 2
      const inside = (r) => mid >= Math.min(r.begin, r.end) && mid <= Math.max(r.begin, r.end)
      const slope = slopes.find(inside)?.gradient ?? 0
      const grade = (up ? slope : -slope) + curveResistance(curves.find(inside)?.radius)
      const [begin, end] = up ? [along(a), along(b)] : [along(b), along(a)]
      gradients.push({ begin, end, value: Math.round(grade * 1000) / 1000 })
      const speed = Math.min(...limits.filter(inside).map(r => r.value))
      if (Number.isFinite(speed)) speeds.push({ begin, end, value: speed })
      else noSpeed.push({ begin, end, value: 0 })
    }
    at += len
  }
  const byBegin = (a, b) => a.begin - b.begin
  return {
    length: at,
    gradients: merged(gradients.sort(byBegin)).filter(g => g.value !== 0),
    speedLimits: merged(speeds.sort(byBegin)).map(s => ({ begin: s.begin, end: s.end, speed: s.value, source: 'osrd' })),
    gaps: { heights: merged(noSlopes), speed: merged(noSpeed.sort(byBegin)) },
  }
}

/** The ranges under wire along a path over an OSRD net, as electrifiedAlong gives them: [{ begin, end, system }]. */
export function osrdElectrified(net, path) {
  const out = []
  let at = 0
  for (const p of path ?? []) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const along = (station) => at + Math.abs(station - p.start)
    for (const r of net.wires.get(p.trackId) ?? []) {
      const a = Math.max(lo, r.begin), b = Math.min(hi, r.end)
      if (b - a <= 1e-6) continue
      const [begin, end] = [along(a), along(b)].sort((x, y) => x - y)
      out.push({ begin, end, system: r.value })
    }
    at += hi - lo
  }
  return out.sort((x, y) => x.begin - y.begin)
}

/** The request the simulation takes for a train's way over an OSRD net, as simulationRequest's over a variant's. */
export function osrdSimulationRequest(net, path, opts) {
  return requestFrom(osrdProperties(net, path), { ...opts, electrified: osrdElectrified(net, path), planned: net.planned })
}

/** The heights along a path over an OSRD net, for its profile: none — RailJSON states slopes, not heights. */
export function osrdHeightProfile(path) {
  const out = [{ position: 0, height: null }]
  let at = 0
  for (const p of path ?? []) {
    at += Math.abs(p.end - p.start)
    out.push({ position: at, height: null })
  }
  return out
}
