import { extentsOf } from '@rail-across/timetables'

/**
 * The stretches of a timetable and what they hold: a stretch runs from one
 * place where trains overtake, cross, begin or end — a station of more than
 * one track, or where a train begins or ends — to the next along the trains'
 * ways, the same for either direction. A train on it brings its blocks there
 * as the capacity crate takes them.
 */

/** Whether a place is where trains may overtake or cross: a station of more than one track. */
const isNode = (station) => station?.kind === 'station' && (station.tracks?.length ?? 0) >= 2

/** The rows of a run at the places that bound stretches, in running order. */
function nodeRows(run, stationsByKey) {
  const rows = run?.rows ?? []
  return rows.filter((r, i) => i === 0 || i === rows.length - 1 || isNode(stationsByKey.get(r.key)))
}

/**
 * The stretches the trains run: [{ id, from, to, fromName, toName, trains }] —
 * `from` and `to` the places' keys in the order first run, `trains` how many
 * run it —, in the order they are first met.
 */
export function proposedStretches(stations, trains, runs) {
  const byKey = new Map((stations ?? []).map(s => [s.key, s]))
  const out = new Map()
  for (const train of trains) {
    const rows = nodeRows(runs.get(train.id), byKey)
    for (let i = 0; i + 1 < rows.length; i++) {
      const [a, b] = [rows[i], rows[i + 1]]
      if (a.key === b.key) continue
      const id = [a.key, b.key].sort().join('|')
      if (!out.has(id)) out.set(id, { id, from: a.key, to: b.key, fromName: a.name, toName: b.name, trains: 0 })
      out.get(id).trains++
    }
  }
  return [...out.values()]
}

/** A block's resources as the capacity crate takes them: its track sections and the stretches no section covers, else it whole. */
function resourcesOf(block) {
  if (Array.isArray(block.free)) {
    return [
      ...(block.sections ?? []).map(s => ({ section: s.id, end: s.end })),
      ...block.free.map(s => ({ track: s.trackId, lo: s.lo, hi: s.hi })),
    ]
  }
  return (block.stretches ?? []).map(s => ({ track: s.trackId, lo: s.lo, hi: s.hi }))
}

/** Where a block lies on a stretch from `lo` to `hi` along the train's way, as fractions of it from its first place. */
const spanOf = (b, lo, hi, reversed) => {
  const f = (x) => Math.min(1, Math.max(0, (x - lo) / (hi - lo || 1)))
  const [a, c] = [f(b.from), f(b.reach ?? b.to)]
  return reversed ? [1 - c, 1 - a] : [a, c]
}

const asTrain = (train, at, blocks, place = null) => ({
  id: train.id, kind: String(train.category ?? '').trim(), at,
  ...(place ? { reversed: place.reversed } : {}),
  blocks: blocks.map(b => ({
    begin: b.begin, end: b.end, resources: resourcesOf(b), signalId: b.signalId ?? null, targetId: b.targetId ?? null,
    ...(place ? { span: spanOf(b, place.lo, place.hi, place.reversed) } : {}),
  })),
})

/**
 * The trains on a stretch, as the capacity crate takes them: each that runs
 * from one of its places to the other, `at` the time it leaves the first, its
 * blocks that reach into the stretch.
 */
export function trainsOnStretch(stretch, trains, runs) {
  const out = []
  for (const train of trains) {
    const run = runs.get(train.id)
    if (!run?.blocks || !run.rows) continue
    const a = run.rows.find(r => r.key === stretch.from)
    const b = run.rows.find(r => r.key === stretch.to)
    if (!a || !b) continue
    const [first, second] = a.offset <= b.offset ? [a, b] : [b, a]
    const blocks = run.blocks.filter(bl => Math.min(bl.reach ?? bl.to, second.offset) - Math.max(bl.from, first.offset) > 1)
    if (!blocks.length) continue
    // `span`, `reversed`: where each block lies on the stretch, from its `from` place — for drawing it
    out.push(asTrain(train, first.departure ?? first.arrival, blocks, { lo: first.offset, hi: second.offset, reversed: a.offset > b.offset }))
  }
  return out
}

/**
 * The trains at a station, as the capacity crate takes them: each block that
 * holds one of its tracks, with what it holds there — its stretches on the
 * station's tracks and the track sections that lie on them.
 */
export function trainsAtStation(station, trains, runs, record) {
  const tracks = new Set((station?.tracks ?? []).map(t => t.id))
  const extents = extentsOf(record)
  const onStation = (sectionId) => (extents.get(sectionId)?.parts ?? []).some(p => tracks.has(p.trackId))
  const out = []
  for (const train of trains) {
    const run = runs.get(train.id)
    if (!run?.blocks) continue
    const blocks = []
    for (const b of run.blocks) {
      if (!(b.stretches ?? []).some(s => tracks.has(s.trackId))) continue
      const resources = resourcesOf(b).filter(r => (r.section ? onStation(r.section) || !extents.size : tracks.has(r.track)))
      if (resources.length) blocks.push({ begin: b.begin, end: b.end, resources })
    }
    if (blocks.length) out.push({ id: train.id, kind: String(train.category ?? '').trim(), at: blocks[0].begin, blocks })
  }
  return out
}
