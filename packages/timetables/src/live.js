import { sectionsAlong } from './occupancy.js'

/**
 * The timetable played: where each train is at a moment of the day, as it
 * runs (its clock, runClock) — not yet left, running, standing, arrived —,
 * and the track sections it occupies then: those its length covers, and,
 * where asked, those its routes hold for it (its blocks, blockingTimes)
 * ahead of it and not yet released behind it.
 */

// The track sections along each way, kept as long as the way is.
const along = new WeakMap()
const sectionsOn = (record, path) => {
  if (!along.has(path)) along.set(path, sectionsAlong(record, path))
  return along.get(path)
}

/** The colours the trains are told apart by, in the order of their departures. */
export const TRAIN_COLORS = ['#1f77b4', '#d62728', '#2ca02c', '#ff7f0e', '#9467bd', '#17becf', '#8c564b', '#e377c2', '#bcbd22', '#0a1f8f', '#7f7f7f', '#b8860b']

/** How far along its way a train has got at `time` [m]: the furthest offset it has reached. */
export function offsetAt(clock, time) {
  if (time <= clock.at0) return 0
  if (time >= clock.end) return clock.length
  let lo = 0, hi = clock.length
  for (let i = 0; i < 40 && hi - lo > 0.05; i++) {
    const mid = (lo + hi) / 2
    if (clock.first(mid) <= time) lo = mid
    else hi = mid
  }
  return lo
}

/** A value of the simulation's report at an offset, straight between its points. */
function speedAlong(report, offset) {
  const xs = report?.positions ?? [], vs = report?.speeds ?? []
  if (!xs.length) return 0
  let i = 1
  while (i < xs.length - 1 && xs[i] < offset) i++
  const span = xs[i] - xs[i - 1]
  return span > 0 ? vs[i - 1] + (vs[i] - vs[i - 1]) * (offset - xs[i - 1]) / span : vs[i]
}

/**
 * Where a train is at `time`: { state, offset, speed } — `state` 'before'
 * (not yet left), 'running', 'standing' or 'arrived'; `offset` along its way
 * [m], `speed` [m/s]. Null for a train without a run.
 */
export function trainAt(run, time) {
  const clock = run?.clock
  if (!clock) return null
  if (time < clock.at0) return { state: 'before', offset: 0, speed: 0 }
  if (time >= clock.end) return { state: 'arrived', offset: clock.length, speed: 0 }
  const offset = offsetAt(clock, time)
  if (clock.last(offset) - clock.first(offset) > 1 && time <= clock.last(offset)) return { state: 'standing', offset, speed: 0 }
  return { state: 'running', offset, speed: Math.max(0, speedAlong(run.result?.final, offset)) }
}

/**
 * The track sections a train occupies at `time`: { occupied, reserved } —
 * ids, `occupied` those between its head and its tail, `reserved` those its
 * blocks hold for it then and it does not occupy (with `reserved` asked
 * for). Empty before it leaves and once it has arrived.
 */
export function sectionsAt(record, run, time, { reserved: withReserved = false } = {}) {
  const at = trainAt(run, time)
  if (!at || at.state === 'before' || at.state === 'arrived') return { occupied: [], reserved: [] }
  const length = run.request?.rolling_stock?.length ?? 0
  const tail = Math.max(0, at.offset - length)
  const occupied = sectionsOn(record, run.route.path).filter(s => s.to > tail + 0.5 && s.from < at.offset - 0.5).map(s => s.id)
  if (!withReserved) return { occupied, reserved: [] }
  const mine = new Set(occupied)
  const reserved = new Set()
  for (const b of run.blocks ?? []) {
    if (b.begin > time) continue
    for (const s of b.sections ?? []) if (s.end > time && !mine.has(s.id)) reserved.add(s.id)
  }
  return { occupied, reserved: [...reserved] }
}

/** The day the trains run within: { from, to } — the first's departure and the last's arrival [s from midnight], or null. */
export function playSpan(runs) {
  const clocks = [...runs.values()].map(r => r?.clock).filter(Boolean)
  if (!clocks.length) return null
  return { from: Math.min(...clocks.map(c => c.at0)), to: Math.max(...clocks.map(c => c.end)) }
}
