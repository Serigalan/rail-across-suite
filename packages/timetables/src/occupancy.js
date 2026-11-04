import { trackSectionExtents } from '@rail-across/core/signalling'
import { directionOf, pathLength } from '@rail-across/running-times'

/**
 * The occupancy of the net by the trains of a timetable: a train runs only
 * into a free block. A block is the stretch from one main signal along a
 * train's way to the next acting the same way — the first from its start,
 * the last to its end —, with the overlap beyond the signal that ends it
 * (Durchrutschweg). Without main signals the whole way is one block.
 *
 * A train holds a block (its blocking time, Sperrzeit, after Ril 405 and
 * OSRD's spacing requirements) from the moment it must find it free — when
 * it passes the distant signal `approach` metres before the block's signal,
 * or leaves where it stood within them, less the time to set the route
 * (`setup`) — until its tail has left the block and the block is released
 * (`release`); at its end, until it arrives. Where the variant has track
 * sections (Freimeldeabschnitte), a block is held section by section, and
 * each section is released as soon as the train's tail has left it (or the
 * block, for the sections of its overlap) — Teilfahrstraßenauflösung: a
 * route over a section another holds waits for that section alone. Two
 * trains conflict where they hold the same section, or a stretch of the
 * same track no section covers, at the same time.
 *
 * A timetable's `dispatch` says what is done about it: `mode` 'show' runs
 * every train for itself and says where they conflict, as OSRD does; 'wait'
 * runs them one after the other — by departure, or by the order of their
 * kinds (`priority` 'category', `categories` first first) — and a train
 * whose block is held waits for it: longer at its last stop before the
 * block, else before the block's signal, else at its start.
 */

/** How long a block is held beyond the train's own running in it: [s], the distant signal's distance [m]. */
export const BLOCKING = { setup: 12, approach: 1000, release: 6 }

/** The way conflicts are dealt with where a timetable says nothing. */
export const DISPATCH = { mode: 'wait', priority: 'departure', categories: ['ICE', 'EC', 'IC', 'IRE', 'RE', 'RB', 'S', 'Dz', 'Gz', 'Lz'], times: BLOCKING }

/** A timetable's way of dealing with conflicts, what it leaves out taken from DISPATCH. */
export const dispatchOf = (data) => {
  const d = data?.dispatch ?? {}
  return { ...DISPATCH, ...d, categories: d.categories ?? DISPATCH.categories, times: { ...BLOCKING, ...d.times } }
}

/** The kinds of logical signal that start or end no train route, and bound no block. */
const NO_BLOCK = new Set(['distant', 'distantRepeater', 'shunting'])

/** Whether a signal bounds blocks: a real one with a main or multi-section screen (as the Signalling-Tool's itineraries). */
export const isMainSignal = (s) => s.kind === 'real' && (s.logicalSignals ?? []).some(l => !NO_BLOCK.has(l.type))

/** The main signals along a path acting the way it runs there: [{ signal, offset }] in running order. */
export function signalsAlong(record, path) {
  const signals = (record?.signalling?.collections?.signals ?? []).filter(isMainSignal)
  const out = []
  let at = 0
  for (const p of path) {
    const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
    const dir = directionOf(p)
    for (const s of signals) {
      if (!s.bothDirections && (s.at?.direction ?? 'up') !== dir) continue
      for (const loc of [s.at, ...(s.alsoAt ?? [])]) {
        if (loc?.anchor !== 'station' || loc.trackId !== p.trackId || loc.station < lo - 1e-6 || loc.station > hi + 1e-6) continue
        const offset = at + Math.abs(loc.station - p.start)
        if (!out.some(o => o.signal.id === s.id && Math.abs(o.offset - offset) < 1)) out.push({ signal: s, offset })
      }
    }
    at += Math.abs(p.end - p.start)
  }
  return out.sort((a, b) => a.offset - b.offset)
}

/** The stretches of the tracks a path covers between two offsets along it: [{ trackId, lo, hi }]. */
export function stretchesOf(path, from, to) {
  const out = []
  let at = 0
  for (const p of path) {
    const len = Math.abs(p.end - p.start)
    const a = Math.max(from, at), b = Math.min(to, at + len)
    if (b - a > 1e-6) {
      const sign = p.end >= p.start ? 1 : -1
      const s1 = p.start + sign * (a - at), s2 = p.start + sign * (b - at)
      out.push({ trackId: p.trackId, lo: Math.min(s1, s2), hi: Math.max(s1, s2) })
    }
    at += len
  }
  return out
}

// The extents of a record's track sections, kept as long as the record is.
const extentsCache = new WeakMap()
function extentsOf(record) {
  if (!record) return new Map()
  if (!extentsCache.has(record)) {
    const sections = record.signalling?.collections?.trackSections ?? []
    extentsCache.set(record, sections.length
      ? trackSectionExtents({ core: { collections: { tracks: record.tracks ?? [], switches: record.switches ?? [] } }, signalling: record.signalling })
      : new Map())
  }
  return extentsCache.get(record)
}

/** Where along a path each track section lies: [{ id, from, to }] — from its first metre run to its last. */
export function sectionsAlong(record, path) {
  const spans = new Map()
  for (const [id, extent] of extentsOf(record)) {
    let at = 0
    for (const p of path) {
      const lo = Math.min(p.start, p.end), hi = Math.max(p.start, p.end)
      for (const part of extent.parts) {
        if (part.trackId !== p.trackId) continue
        const a = Math.max(lo, part.from), b = Math.min(hi, part.to)
        if (b - a <= 0.5) continue
        const [o1, o2] = [at + Math.abs(a - p.start), at + Math.abs(b - p.start)].sort((x, y) => x - y)
        const was = spans.get(id)
        spans.set(id, { id, from: Math.min(was?.from ?? Infinity, o1), to: Math.max(was?.to ?? -Infinity, o2) })
      }
      at += hi - lo
    }
  }
  return [...spans.values()].sort((a, b) => a.from - b.from)
}

/** The offset ranges between `from` and `to` that none of `spans` covers. */
function uncovered(spans, from, to) {
  const out = []
  let at = from
  for (const s of [...spans].sort((a, b) => a.from - b.from)) {
    if (s.to <= at || s.from >= to) continue
    if (s.from > at + 0.5) out.push([at, Math.min(s.from, to)])
    at = Math.max(at, s.to)
  }
  if (to > at + 0.5) out.push([at, to])
  return out
}

/**
 * The blocks along a path: [{ index, signalId, targetId, from, to, reach,
 * stretches, sections, free }] — `signalId` the signal it begins at (null for
 * the first, where the train starts), `targetId` the one it ends at (null for
 * the last); `from`, `to` its offsets along the path, `reach` the end of the
 * overlap beyond `to`; `stretches` what it covers of the tracks, the overlap
 * with it; `sections` the track sections it holds [{ id, to }], `to` where
 * along the path the train has run through it; `free` the stretches of it no
 * section covers.
 */
export function blocksAlong(record, path) {
  const length = pathLength(path)
  const overlaps = record?.signalling?.collections?.overlaps ?? []
  const overlapOf = (signalId) => Math.max(0, ...overlaps.filter(o => o.signalId === signalId).map(o => o.length ?? 0))
  const cuts = [{ offset: 0, signal: null }, ...signalsAlong(record, path).filter(s => s.offset > 1 && s.offset < length - 1)]
  const spans = sectionsAlong(record, path)
  return cuts.map((c, i) => {
    const next = cuts[i + 1]
    const to = next ? next.offset : length
    const reach = next ? Math.min(length, to + overlapOf(next.signal.id)) : length
    const held = spans.filter(s => Math.min(s.to, reach) - Math.max(s.from, c.offset) > 0.5)
    return {
      index: i, signalId: c.signal?.id ?? null, targetId: next?.signal.id ?? null, from: c.offset, to, reach,
      stretches: stretchesOf(path, c.offset, reach),
      sections: held.map(s => ({ id: s.id, to: Math.min(s.to, reach) })),
      free: uncovered(held, c.offset, reach).flatMap(([a, b]) => stretchesOf(path, a, b)),
    }
  })
}

/**
 * The blocking times of a train's blocks, as it runs (`clock`, runClock) with
 * its length [m]: each block with { begin, end } in seconds from midnight.
 */
export function blockingTimes(blocks, clock, trainLength, times = BLOCKING) {
  return blocks.map(b => {
    const approach = Math.max(0, b.from - times.approach)
    let begin = b.from <= 1e-6 ? clock.at0 : clock.first(approach)
    // where it stands within the approach, the block is asked for as it leaves
    for (const s of clock.standstills) if (s.offset >= approach - 1e-6 && s.offset < b.from - 1e-6) begin = Math.max(begin, s.leave)
    // Until the train has run through `at` with its whole length: the block's end, else that moment.
    const leftAt = (at) => {
      const clear = at + trainLength
      return (b.targetId == null || clear >= clock.length - 1e-6 ? clock.end : clock.first(clear)) + times.release
    }
    const end = leftAt(b.to)
    // A section is released once the train has left it; one of the overlap, with the block.
    const sections = (b.sections ?? []).map(s => ({ ...s, end: Math.min(end, leftAt(Math.min(s.to, b.to))) }))
    return { ...b, begin: begin - times.setup, end, sections }
  })
}

/** Whether two lists of stretches share more than half a metre of a track. */
const shareTrack = (xs, ys) => xs.some(x => ys.some(y => x.trackId === y.trackId && Math.min(x.hi, y.hi) - Math.max(x.lo, y.lo) > 0.5))

/**
 * What two blocks hold at the same time: [{ section, begin, end, until }] —
 * `section` the track section both hold (null for a stretch no section
 * covers), `begin`, `end` the time both hold it, `until` when the other
 * releases it. A block without sections is held as a whole.
 */
function shared(a, b) {
  const out = []
  const both = (end, otherEnd, section) => {
    const begin = Math.max(a.begin, b.begin)
    const stop = Math.min(end, otherEnd)
    if (begin < stop) out.push({ section, begin, end: stop, until: otherEnd })
  }
  if (a.free && b.free) {
    for (const x of a.sections ?? []) for (const y of b.sections ?? []) if (x.id === y.id) both(x.end, y.end, x.id)
    if (shareTrack(a.free, b.free)) both(a.end, b.end, null)
  } else if (shareTrack(a.stretches, b.stretches)) both(a.end, b.end, null)
  return out
}

/**
 * Where a train's blocks conflict with those of others: [{ block, other,
 * otherBlock, section, begin, end, until }] — `other` the other train's id,
 * `section` the track section both hold (null for a stretch of track),
 * `begin`, `end` the time both hold it, `until` when the other lets it go —,
 * in the order of the train's blocks.
 */
export function conflictsOf(blocks, others) {
  const out = []
  const lo = Math.min(...blocks.map(b => b.begin)), hi = Math.max(...blocks.map(b => b.end))
  for (const o of others) {
    if (!o.blocks.length || o.end <= lo || o.begin >= hi) continue
    for (const b of blocks) {
      for (const ob of o.blocks) {
        for (const c of shared(b, ob)) out.push({ block: b.index, other: o.id, otherBlock: ob.index, ...c })
      }
    }
  }
  return out.sort((a, b) => a.block - b.block || a.begin - b.begin)
}

/** The trains in the order they are given the net: by departure, or by the order of their kinds, then by departure. */
export function byPriority(trains, { priority, categories }) {
  const rank = (tr) => {
    const i = categories.indexOf(String(tr.category ?? '').trim())
    return priority === 'category' ? (i < 0 ? categories.length : i) : 0
  }
  return [...trains].sort((a, b) => rank(a) - rank(b) || a.departure - b.departure || String(a.number).localeCompare(String(b.number), 'de', { numeric: true }))
}

/**
 * Where a train waits for a block: at its start for the first; else at the
 * last point it stops at within the block's approach — between the signal
 * before and the block's, its start among them —, longer; else a few metres
 * before the block's signal. { offset, point } — `point` the index of the train's point, or null.
 */
export function holdBefore(run, block, blocks, approach) {
  if (block.from <= 1e-6) return { offset: 0, point: 0, signalId: null }
  const before = blocks[block.index - 1]
  const from = Math.max(before ? before.from : 0, block.from - approach)
  // its start is where it stands first: it leaves later rather than wait at the signal ahead
  const stands = [{ offset: 0, point: 0 }, ...run.schedule.filter(s => s.point != null)]
  const stop = stands.reverse().find(s => s.offset >= from - 1e-6 && s.offset < block.from - 1e-6)
  if (stop) return { offset: stop.offset, point: stop.point, signalId: block.signalId }
  return { offset: Math.max(block.from - 10, (before?.from ?? 0) + 1), point: null, signalId: block.signalId }
}

const MAX_STEPS = 40

/**
 * The runs of a timetable's trains over the net of `record`, with their blocks: Map id → run with {
 * blocks, conflicts, waits, delay } — `waits` [{ offset, point, signalId,
 * wait, because }] where it waits, before which signal's block, for whom;
 * `delay` how much later it arrives than run for itself — for every train
 * `runOf(train, holds)` (async)
 * can run. In mode 'wait' a train waits where a train before it holds a
 * block; what it cannot wait away is a conflict all the same.
 */
export async function occupancy(record, trains, runOf, dispatch = DISPATCH) {
  const times = dispatch.times ?? BLOCKING
  const timed = (train, run) => {
    if (!run?.rows) return run
    const blocks = blockingTimes(blocksAlong(record, run.route.path), run.clock, run.request.rolling_stock.length, times)
    return { ...run, blocks }
  }
  const out = new Map()
  const placed = []   // [{ id, blocks, begin, end }]
  const place = (id, blocks) => placed.push({ id, blocks, begin: Math.min(...blocks.map(b => b.begin)), end: Math.max(...blocks.map(b => b.end)) })

  if (dispatch.mode !== 'wait') {
    for (const train of trains) {
      const run = timed(train, await runOf(train, []))
      out.set(train.id, run)
      if (run?.blocks) place(train.id, run.blocks)
    }
    for (const train of trains) {
      const run = out.get(train.id)
      if (run?.blocks) out.set(train.id, { ...run, conflicts: conflictsOf(run.blocks, placed.filter(p => p.id !== train.id)), waits: [], delay: 0 })
    }
    return out
  }

  for (const train of byPriority(trains, dispatch)) {
    let holds = []
    let run = timed(train, await runOf(train, holds))
    const free = run?.clock?.end
    const because = new Map()   // hold offset → ids of the trains it waits for
    for (let step = 0; run?.blocks && step < MAX_STEPS; step++) {
      const found = conflictsOf(run.blocks, placed)
      if (!found.length) break
      // The first block held: wait until every train holding it then has left it.
      const block = run.blocks[found[0].block]
      const holding = found.filter(c => c.block === block.index)
      const until = Math.max(...holding.map(c => c.until))
      const hold = holdBefore(run, block, run.blocks, times.approach)
      const leaves = hold.point === 0 ? run.clock.at0 : run.clock.last(hold.offset)
      const more = Math.max(1, Math.ceil(until + times.setup - leaves))
      const same = holds.find(h => Math.abs(h.offset - hold.offset) < 1e-6 && h.point === hold.point)
      // a wait further back makes those after it start anew
      holds = [...holds.filter(h => h.offset < hold.offset - 1e-6), { ...hold, wait: (same?.wait ?? 0) + more }]
      because.set(hold.offset, [...new Set([...(because.get(hold.offset) ?? []), ...holding.map(c => c.other)])])
      run = timed(train, await runOf(train, holds))
    }
    if (run?.blocks) {
      out.set(train.id, {
        ...run, conflicts: conflictsOf(run.blocks, placed), waits: holds.map(h => ({ ...h, because: because.get(h.offset) ?? [] })), delay: run.clock.end - free,
      })
      place(train.id, run.blocks)
    } else out.set(train.id, run)
  }
  return out
}
