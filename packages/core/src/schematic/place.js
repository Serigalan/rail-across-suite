import { trackLength } from '../track/index.js'

/**
 * Where a station of a track lies on the strip: every track is the polyline
 * its lines draw — its straight, the steps and diagonals it takes. A station
 * goes onto it at its x along the reference, so an object keeps its place
 * along the line, held within what the strip draws of its track. A crossover
 * the strip spreads to 45° is drawn off its place; a station on it goes by
 * its share of the way from one end to the other.
 */

/** A connection of several tracks, cut into their shares by their lengths. */
function shares(pts, trackIds, byId) {
  const lengths = trackIds.map(id => trackLength(byId.get(id)) || 1)
  const total = lengths.reduce((a, b) => a + b, 0)
  const [[x0, y0], [x1, y1]] = pts
  let t = 0
  return trackIds.map((id, i) => {
    const t0 = t
    t += lengths[i] / total
    return [id, [[x0 + (x1 - x0) * t0, y0 + (y1 - y0) * t0], [x0 + (x1 - x0) * t, y0 + (y1 - y0) * t]]]
  })
}

/** The y of a polyline sorted by x at an x. */
function yAt(line, x) {
  if (x <= line[0][0]) return line[0][1]
  for (let i = 1; i < line.length; i++) {
    const [ax, ay] = line[i - 1]
    const [bx, by] = line[i]
    if (x <= bx) return bx > ax ? ay + (by - ay) * (x - ax) / (bx - ax) : by
  }
  return line[line.length - 1][1]
}

/**
 * The placer of a strip: `at(trackId, station)` → { x, y, dir } — dir 1 where
 * the track's stationing runs along x, -1 where it runs against it — or null
 * for a track the strip does not draw; `stretch(trackId, from, to)` → the
 * points of the track between two stations.
 *
 * @param {object} layout  schematicLayout's
 * @param {object} strip   schematicStrip's
 */
export function schematicPlacer(layout, strip) {
  const pieces = new Map()
  const put = (id, pts) => { if (!pieces.has(id)) pieces.set(id, []); pieces.get(id).push(...pts) }
  for (const it of strip.items) {
    if (it.kind !== 'line') continue
    if (it.trackIds.length === 1) put(it.trackIds[0], it.pts)
    else for (const [id, pts] of shares(it.pts, it.trackIds, layout.trackById)) put(id, pts)
  }
  const tracks = new Map()
  for (const [id, pts] of pieces) {
    const line = [...pts].sort((a, b) => a[0] - b[0])
    const track = layout.trackById.get(id)
    const length = trackLength(track)
    const [r0, r1] = [layout.stationX(id, 0), layout.stationX(id, length)]
    const spread = Boolean(layout.sectionOfTrack.get(id)?.strand.connector)
    tracks.set(id, { line, length, r0, r1, dir: r1 >= r0 ? 1 : -1, spread })
  }

  const at = (trackId, station) => {
    const tr = tracks.get(trackId)
    if (!tr) return null
    const s = Math.min(Math.max(station, 0), tr.length)
    const raw = layout.stationX(trackId, s)
    const lo = tr.line[0][0]
    const hi = tr.line[tr.line.length - 1][0]
    let x = Math.min(Math.max(raw, lo), hi)
    if (tr.spread) {
      // The share of the way along the track, as the reference measures it.
      const f = Math.abs(tr.r1 - tr.r0) > 1e-6 ? (raw - tr.r0) / (tr.r1 - tr.r0) : s / (tr.length || 1)
      const share = Math.min(Math.max(f, 0), 1)
      x = tr.dir > 0 ? lo + (hi - lo) * share : hi - (hi - lo) * share
    }
    return { x, y: yAt(tr.line, x), dir: tr.dir }
  }

  const stretch = (trackId, from, to, steps = 24) => {
    const tr = tracks.get(trackId)
    if (!tr) return []
    const out = []
    for (let i = 0; i <= steps; i++) {
      const p = at(trackId, from + (to - from) * i / steps)
      if (p) out.push([p.x, p.y])
    }
    return out
  }

  return { at, stretch, line: (trackId) => tracks.get(trackId)?.line ?? null }
}
