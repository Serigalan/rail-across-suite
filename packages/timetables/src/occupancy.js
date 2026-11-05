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
 * An overlap is released with its block, or, where the train comes to a
 * stand before the block's end signal, once it has stood there a while
 * (`overlap`): the train behind the signal cannot run into it any more.
 *
 * A timetable's `dispatch` says what is done about it: `mode` 'show' runs
 * every train for itself and says where they conflict, as OSRD does; 'wait'
 * sets the trains' routes one after the other in time, as an interlocking
 * does — the train that asks first first; at the same moment the one first
 * by departure, or by the order of their kinds (`priority` 'category',
 * `categories` first first). A train asks for its next block as it nears the
 * block's signal, and gets it where none of its track sections is held then
 * by another train — by the blocks the other has been given, as far as it
 * has got. Where one is held, the train takes another route from that
 * signal (one of the signalling's) that is free and still leads to where it
 * runs — into the other track of a station to cross a train there —, else
 * it waits for the route that is freed first: longer at its last stop
 * before the block, else before the block's signal, else at its start.
 */

/**
 * How long a block is held beyond the train's own running in it: [s], the
 * distant signal's distance [m]; how long a train stands before a signal
 * before its overlap is released [s].
 */
export const BLOCKING = { setup: 12, approach: 1000, release: 6, overlap: 60 }

/** How far before a block's end signal a train stands for its overlap to be released [m]. */
const BEFORE_SIGNAL = 200

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
 * with it; `sections` the track sections it holds [{ id, from, to }], where
 * along the path the train runs into it and has run through it; `free` the stretches of it no
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
      sections: held.map(s => ({ id: s.id, from: Math.max(s.from, c.offset), to: Math.min(s.to, reach) })),
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
    // Where it comes to a stand before the block's end signal, its overlap is released a while after.
    const stand = b.targetId == null ? null : clock.standstills.find(s => s.offset <= b.to + 1e-6 && s.offset >= b.to - BEFORE_SIGNAL)
    const overlapEnd = stand ? Math.min(end, clock.first(stand.offset) + (times.overlap ?? BLOCKING.overlap)) : end
    // A section is released once the train has left it; one of the overlap, with the block or once it stands.
    const sections = (b.sections ?? []).map(s => ({ ...s, end: s.from >= b.to - 0.5 ? overlapEnd : Math.min(end, leftAt(Math.min(s.to, b.to))) }))
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

/** A train's class in giving the net: the place of its kind among `categories` where they come first, else all one. */
function classOf(tr, { priority, categories }) {
  const i = categories.indexOf(String(tr.category ?? '').trim())
  return priority === 'category' ? (i < 0 ? categories.length : i) : 0
}

/** The trains in the order they are given the net: by departure, or by the order of their kinds, then by departure. */
export function byPriority(trains, dispatch) {
  const rank = (tr) => classOf(tr, dispatch)
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

/** How many decisions the dispatcher makes for each train at the most, before it lets the rest run as they would. */
const STEPS_PER_TRAIN = 80

/** How often a train asks again for a block another may yet hold longer, before it waits for it as it stands. */
const ASKS = 20

/**
 * The runs of a timetable's trains over the net of `record`, with their
 * blocks: Map id → run with { blocks, conflicts, waits, detours, delay } —
 * `waits` [{ offset, point, signalId, wait, because }] where it waits,
 * before which signal's block, for whom; `detours` [{ signalId, routeId,
 * because }] the routes it was sent over as the one it would run was held,
 * by whom; `delay` how much later it arrives than run for itself — for
 * every train `runOf(train, holds, routes)` (async) can run. In mode 'wait'
 * the routes are set as the trains ask for them (dispatch); what cannot be
 * waited away is a conflict all the same.
 */
export async function occupancy(record, trains, runOf, dispatch = DISPATCH) {
  const times = dispatch.times ?? BLOCKING
  const timed = (run) => {
    if (!run?.rows) return run
    const blocks = blockingTimes(blocksAlong(record, run.route.path), run.clock, run.request.rolling_stock.length, times)
    return { ...run, blocks }
  }
  const out = new Map()
  const placed = []   // [{ id, blocks, begin, end }]
  const place = (id, blocks) => placed.push({ id, blocks, begin: Math.min(...blocks.map(b => b.begin)), end: Math.max(...blocks.map(b => b.end)) })

  if (dispatch.mode !== 'wait') {
    for (const train of trains) {
      const run = timed(await runOf(train, []))
      out.set(train.id, run)
      if (run?.blocks) place(train.id, run.blocks)
    }
    for (const train of trains) {
      const run = out.get(train.id)
      if (run?.blocks) out.set(train.id, { ...run, conflicts: conflictsOf(run.blocks, placed.filter(p => p.id !== train.id)), waits: [], delay: 0 })
    }
    return out
  }

  // Kind by kind where the kinds come first, the trains of each as they ask, around those of the kinds before.
  let states = []
  const ordered = byPriority(trains, dispatch)
  for (const c of [...new Set(ordered.map(tr => classOf(tr, dispatch)))]) {
    states = await dispatched(record, ordered.filter(tr => classOf(tr, dispatch) === c), runOf, timed, times, states)
  }
  for (const st of states) if (st.run?.blocks) place(st.train.id, st.run.blocks)
  for (const st of states) {
    if (!st.run?.blocks) { out.set(st.train.id, st.run); continue }
    out.set(st.train.id, {
      ...st.run,
      conflicts: conflictsOf(st.run.blocks, placed.filter(p => p.id !== st.train.id)),
      waits: st.holds.map(h => ({ ...h, because: st.because.get(h.offset) ?? [] })),
      detours: st.routes.map(r => ({ signalId: r.signalId, routeId: r.routeId, because: r.because ?? [] })),
      delay: st.run.clock.end - st.free,
    })
  }
  return out
}

/**
 * Setting the trains' routes as they ask for them (occupancy, mode 'wait'):
 * [{ train, run, holds, routes, because, free }] for each train, those dealt
 * with `before` first — their routes set, as they hold them —, then
 * `trains`, in the order they come first at the same moment. Each train asks for its
 * blocks one by one; the one that asks first is dealt with first. A block is
 * given where none of what it holds is held by the blocks given to the
 * others; where another may yet hold its last block longer — it has not yet
 * asked for its next, and may come to a stand before it —, the train asks
 * again once that one has. Else another route from the block's signal is
 * taken that is free, or the train waits for the one freed first.
 */
async function dispatched(record, trains, runOf, timed, times, before = []) {
  const collections = record?.signalling?.collections ?? {}
  const signals = new Map((collections.signals ?? []).map(s => [s.id, s]))
  const routesFrom = new Map()
  for (const r of collections.routes ?? []) if (r.kind === 'train') routesFrom.set(r.startSignalId, [...(routesFrom.get(r.startSignalId) ?? []), r])
  const states = [...before]
  for (const [rank, train] of trains.entries()) {
    const run = timed(await runOf(train, [], []))
    states.push({ train, rank, run, holds: [], routes: [], because: new Map(), free: run?.clock?.end, granted: 0, again: -Infinity, asks: 0 })
  }
  const live = states.filter(st => st.run?.blocks?.length)
  // When a train asks for its next block: as it nears its signal, or again later.
  const askAt = (st) => Math.max(st.run.blocks[st.granted].begin, st.again)
  const asking = (st) => st.granted < st.run.blocks.length
  // Waiting to ask again: for another's next step, not for its block.
  const waiting = (st) => st.again > st.run.blocks[st.granted].begin
  // What a train holds as others see it at `now`: the blocks it has been given. Of its last one, while it has not
  // got its next, what it would not have left standing before that block's signal is held until it goes on — its
  // overlap only until a while after it would stand there; a block without track sections as a whole.
  const heldBy = (st, now) => {
    const given = st.run.blocks.slice(0, st.granted)
    if (!given.length || !asking(st)) return given
    const at = askAt(st)
    if (at <= now && !waiting(st)) return given
    const last = given[given.length - 1]
    const stand = holdBefore(st.run, st.run.blocks[st.granted], st.run.blocks, times.approach).offset
    const length = st.run.request.rolling_stock.length
    const stood = st.run.clock.first(stand) + (times.overlap ?? BLOCKING.overlap)
    const sections = (last.sections ?? []).map(x => {
      if (x.from >= last.to - 0.5) return { ...x, end: Math.min(x.end, stood) }
      return x.to + length > stand - 1e-6 ? { ...x, end: Infinity } : x
    })
    return [...given.slice(0, -1), { ...last, end: last.end > at ? Infinity : last.end, sections }]
  }
  /** Whether a block is free for a train at `now`: { free, until, again, by } — held until `until`, or to be asked for again at `again`. */
  const verdict = (st, block, now) => {
    let until = -Infinity, again = null
    const by = new Set()
    for (const o of live) {
      if (o === st) continue
      for (const ob of heldBy(o, now)) {
        for (const c of shared(block, ob)) {
          by.add(o.train.id)
          if (c.until === Infinity) again = Math.min(again ?? Infinity, askAt(o))
          else until = Math.max(until, c.until)
        }
      }
    }
    return { free: !by.size, until, again, by: [...by] }
  }
  /** The other routes from a block's signal a train may take there: [{ run, routes, route }] — those that still lead to where it runs. */
  const otherRoutes = async (st, block) => {
    const signal = signals.get(block.signalId)
    if (signal?.at?.anchor !== 'station') return []
    const leg = st.run.route.offsets.findLastIndex(o => o <= block.from + 1e-6)
    const runs = []
    for (const route of routesFrom.get(signal.id) ?? []) {
      const target = signals.get(route.targetSignalId)
      if (target?.at?.anchor !== 'station' || !route.path?.trackIds?.length) continue
      const sent = { leg, signalId: signal.id, routeId: route.id, at: signal.at, trackIds: route.path.trackIds, target: target.at }
      const routes = [...st.routes.filter(r => r.signalId !== signal.id), sent]
      const run = timed(await runOf(st.train, st.holds, routes.map(({ because: _b, ...r }) => r)))
      const b = run?.blocks?.[block.index]
      if (!b || b.signalId !== signal.id || Math.abs(b.from - block.from) > 1) continue
      // the way it runs already
      if (sameWay(b.stretches, block.stretches)) continue
      runs.push({ run, routes, route: sent })
    }
    return runs
  }
  const rerun = async (st) => {
    st.run = timed(await runOf(st.train, st.holds, st.routes.map(({ because: _b, ...r }) => r)))
    return st.run?.blocks
  }

  let steps = STEPS_PER_TRAIN * trains.length + 50
  while (steps-- > 0) {
    const open = live.filter(asking)
    if (!open.length) break
    const now = Math.min(...open.map(askAt))
    const st = open.filter(x => askAt(x) === now).sort((a, b) => a.rank - b.rank)[0]
    const block = st.run.blocks[st.granted]
    const mine = verdict(st, block, now)
    if (mine.free) { st.granted++; st.again = -Infinity; st.asks = 0; continue }
    // Another route from the same signal, free now.
    const options = [{ run: st.run, routes: st.routes, v: mine }]
    if (block.signalId) {
      for (const o of await otherRoutes(st, block)) options.push({ ...o, v: verdict(st, o.run.blocks[block.index], now) })
    }
    const free = options.find(o => o.v.free)
    if (free) {
      if (free.route) {
        free.route.because = mine.by
        st.run = free.run
        st.routes = free.routes
      }
      st.granted++; st.again = -Infinity; st.asks = 0
      continue
    }
    // Held only as long as another may yet hold it: ask again once it has gone on.
    const later = options.map(o => o.v.again).filter(a => a != null)
    if (later.length && st.asks++ < ASKS) { st.again = Math.min(...later) + 1e-3; continue }
    // Held: wait for the route freed first.
    const until = (o) => (o.v.again != null ? Math.max(o.v.until, o.v.again) : o.v.until)
    const best = options.reduce((a, b) => (until(b) < until(a) ? b : a))
    if (best.route) {
      best.route.because = mine.by
      st.run = best.run
      st.routes = best.routes
    }
    const held = st.run.blocks[block.index]
    const hold = holdBefore(st.run, held, st.run.blocks, times.approach)
    const leaves = hold.point === 0 ? st.run.clock.at0 : st.run.clock.last(hold.offset)
    const more = Math.max(1, Math.ceil(until(best) + times.setup - leaves))
    const same = st.holds.find(h => Math.abs(h.offset - hold.offset) < 1e-6 && h.point === hold.point)
    // a wait further back makes those after it start anew
    st.holds = [...st.holds.filter(h => h.offset < hold.offset - 1e-6), { ...hold, wait: (same?.wait ?? 0) + more }]
    st.because.set(hold.offset, [...new Set([...(st.because.get(hold.offset) ?? []), ...best.v.by])])
    st.again = -Infinity
    st.asks = 0
    // a train that cannot be run so any more drops out
    if (!await rerun(st)) live.splice(live.indexOf(st), 1)
  }
  return states
}

/** Whether two lists of stretches cover the same track. */
const sameWay = (xs, ys) => xs.length === ys.length && xs.every((x, i) => x.trackId === ys[i].trackId && Math.abs(x.lo - ys[i].lo) < 1 && Math.abs(x.hi - ys[i].hi) < 1)
